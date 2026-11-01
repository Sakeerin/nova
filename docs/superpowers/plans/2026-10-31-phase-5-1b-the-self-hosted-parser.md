# Phase 5.1b, "The self-hosted parser", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** The second of Phase 5.1's two parts: N's parser. `selfhost/`
gains the AST and a parser that follows `nova-parser` function for
function, and `selfhost ast FILE…` prints the same AST dump as S0's hidden
`nova parse --dump-ast FILE…`: the tree as an indented S-expression with
spans, its newline splits, then every lex error, parse error and note, in
order. The differential (`crates/nova-cli/tests/selfhost.rs`) compares the
two dumps beside the token dumps on the corpus, its Windows-endings copy,
the fixed and error cases, the edge inputs, the mutations, one soup seed
and the new deep inputs, on Linux, macOS and Windows. One language change:
both parsers count how deeply source nests, and past 128 levels report
**P0002**, "this is nested too deeply (more than 128 levels)", and stop.
S0 runs every command's pipeline on a 64 MB thread, so the limit, not the
platform's stack, decides what deeply nested source meets (a decimal
literal of 210,000 digits or more still aborts it: ADR 0051's per-digit
recursion is no level). ADR 0052 records the
dump's grammar, the idioms, the limit, the thread and the speed.

**Architecture:**
- **S0, the depth limit:** `crates/nova-parser/src/grammar.rs`
  (`Parser::deeper`, `nested`, the binary and postfix chains' counts,
  `parse_type_here`, `parse_pattern_here`, `parse_view_children_here`,
  `parse_postfix_chain`, the stop at the limit, `roll_back`), `error.rs`
  (`ParseError::TooDeep`, `ParseError::code()`), `lib.rs` (`MAX_DEPTH`);
  every caller attaches `code()`: `nova-cli`'s `parse.rs`, the driver's
  loader, the resolver's std loader, `nova-fmt`, and the language server's
  organize-imports.
- **S0, the thread:** `nova_driver::PIPELINE_STACK`; `nova-cli`'s `main`
  runs each command on a thread of that stack named `main`, and `nova run`
  hands its compiled program back to the main thread
  (`cmd::run::Compiled`); the language server's checker and `nova dev`'s
  rebuilds get the same stack.
- **S0, the dump:** `crates/nova-ast/src/dump.rs` (`dump_file`, `quote`,
  `escape_message`, `KINDS`), `crates/nova-parser/src/dump.rs`
  (`dump_ast`, the errors); `nova parse --dump-ast`, hidden, beside
  5.1a's `--dump-tokens`.
- **N:** `selfhost/src/`: `ast.nova`, `parser.nova`, `parse_item.nova`,
  `parse_type.nova`, `parse_stmt.nova`, `parse_expr.nova`,
  `parse_view.nova`, `parse_pattern.nova`, `ast_dump.nova` and
  `parser_tests.nova` (N's own `@test`s); `token.nova`'s `Spanned` made
  generic; `main.nova`'s `selfhost ast`.
- **The differential:** `selfhost.rs`'s AST pass beside the token pass,
  its every-kind test, N's stack at 128 levels, S0 at the limit, the
  thread's tests; `tests/selfhost/fuzz.rs`'s deep generator and new edge
  inputs; `tests/selfhost/compare.rs`'s `AST`; 266 new files under
  `tests/selfhost/cases/`.
- **Records:** ADR 0052; `CHANGELOG.md`; dated notes in
  `docs/phase-5-plan.md`, `nova-spec/00-MASTER-SPEC.md` and
  `nova-spec/11-PARSER.md`; P0002, and rows for P0001 and L0001, in
  `nova-spec/12-TYPESYSTEM.md`'s catalog.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §6.1).
- N is built by `nova build` through Cranelift: no `clang` for the
  differential. `clang-18` (Ubuntu's 18.1.3, CI's pin) for the golden
  suite, and Node 22 for the WASM tests, as before.

**Spec:**
`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`,
approved by the user on 2026-10-30, on `main` since 5.1a.
- Read its header, §4 (5.1b), §§1, 2 and 5 to 10 before Task 1: it is the
  authority this plan argues from.
- Its header holds the user's decisions of 2026-10-30 (decisions 1 to 7
  here) and its "not asked" recommendations (decisions 9 to 11; floats by
  their bits was 5.1a's). The user's eighth, the ruling of 2026-10-31 that
  postfix chains count toward the same 128 levels, asked after the
  prototype's step 0, is decision 8.

**Checked on 2026-10-31, before execution:**
- **A prototype built 5.1b whole** on a local branch from `4f32f13d`
  (`main` after 5.1a), in this tree, in two parts: S0's side, `b2818ebe`
  (P0002), `4883c74a` (the dump and the flag), `5c8d6771` (the thread),
  `8d34863f` (the error cases), `ea7345a0` (the deep generator) and
  `42e17bc8` (ADR 0052's first half); then N's, `d49b3361` (128 nested
  constructs fit), `2710463d` (N's AST and parser), `4c8b96c0` (the
  comparison and N's depth) and `087ea566` (ADR 0052's second half); then
  step 0, `0c80283a`, "5.1b prototype: before the split" (decisions
  28, 37 to 44 and 49), amended after the mutants' first run; then
  `9be3dd8e`, "5.1b prototype: the first fact-check's fixes" (decision
  53); then `5a68e7b9`, "5.1b prototype: the second
  fact-check's fixes" (decision 54). No difference between
  the two parsers was ever found: N's first build agreed with S0 on all
  585 files it then had (588 now, with step 0's case and the second
  fact-check's two), the fixed seeds and 52,000 inputs of local searches,
  and step 0's probes and both fact-checks' agreed too. The one code
  fault found, by the second fact-check, was in both parsers alike.
- **Step 0, before the split** (decisions 28, 37 to 44 and 49):
  - **postfix chains are counted** **(differs)**: uncounted, a chain of
    about 5,000 method calls overflowed `nova check` on the 64 MB thread;
    asked, the user ruled that each postfix operator after a chain's first
    counts toward the same limit (decision 8), a method call one, in both
    parsers, with tests and a deep kind (decision 28);
  - the thread, probed against the base binary command by command, and a
    test that `nova run`'s program keeps the main thread's stack
    (decision 37);
  - P0002 probed on 48 shapes at 127 to 129 levels, by both parsers,
    `nova check` and `nova fmt` (decision 38);
  - the dump probed on both sides: ADR 0052's 72 examples regenerated
    byte for byte, escapes, floats, integers, empty and error-only files,
    the errors' order (decision 39);
  - `--dump-ast`'s argument shapes, and `--help` of every command
    (decision 40);
  - the harness made to fail when the AST pass is skipped, and
    `every_other_node.nova` made to name, with the base's corpus, every
    kind, so S0's every-kind test stands in Task 3 (decisions
    41 and 42);
  - CI's time estimated, and the records made true: ADR 0052 accepted
    with the branch, the dated notes naming it, the `Send` argument, the
    new figures (decisions 43 and 44);
  - after the first run of the mutants, their three gaps closed: a fixed
    case and `depth.rs` put an operand 100 deep after a chain of each
    precedence, after a postfix chain and 128 casts deep, and `nova-ast`'s
    dump test pins the lowercase hex (decision 49).
- **The first fact-check's fixes** (decision 53). A fact-check
  of the prototype and this plan found no code fault: 0
  differences on 586 files, about 33,000 mutations of its own, about 90
  hand files aimed at every grammar branch and over 60 depth shapes; the
  thread matched the base on all 176 runtime programs, the GC stress,
  signals, stdin and `nova test`; the patches, figures and examples held.
  It found, and `9be3dd8e` fixed:
  - the postfix ruling not recorded as the user's: now decision 8, cited
    by decision 28, ADR 0052's Status and its postfix bullet, the
    CHANGELOG and `docs/phase-5-plan.md`'s note;
  - the records saying the thread stops every overflow: a decimal
    literal of 210,000 digits or more still aborts debug S0's 64 MB
    thread, so `11-PARSER.md` §5, the ADR and the CHANGELOG now say so
    (decision 20);
  - the every-kind check blind to a missing kind or value: `KINDS` now
    lists each `bool` field's two values and `children=`'s, after their
    node's kind, 167 forms; `every_declared_form_is_listed` derives the
    forms from `nova-ast`'s declarations by a small scanner, so a new
    variant or `bool` field fails until listed; `every_other_node.nova`
    gained the three values no file held (a record pattern without `..`,
    `*Int`, a trait's `async` signature); each fails with its fix taken
    out (decisions 17 and 42);
  - nits: the dump's two departures from the spec's sketch marked
    (decision 15); the base's crash figures measured again and
    stated alike everywhere; 586 files; the MSRV step's scope (Task 8,
    Step 6).
- **The second fact-check's fixes** (decision 54). A second
  fact-check of the prototype and this plan found one code fault, in both
  parsers alike, and six faults in the records and tests; `5a68e7b9`
  fixed each:
  - an abandoned attempt decided the depth: the generic arguments'
    rollback restored the position but not the P0002 state, so `x as Int
    < (((a)))` with 127 parentheses, which the base parses, was P0002,
    the abandoned type a level deeper than the comparison. Restoring the
    state alone would make a parameter's `Vec<Vec<…>>` past the limit,
    whose lists roll back one by one, a run of P0001s; so the crossing
    waits, and stands only if reading the tokens the other way fails
    before the token where it was made. The trait body's lookahead, the
    other rollback, cannot cross unseen. A `depth.rs` test and two cases
    hold it (decision 19);
  - `KINDS` has 167 forms, not 168 (decision 17);
  - the chain figures: a call's arguments are a level deeper than the
    call, so after a name 129 calls without arguments parse but 128 with,
    and a router built from `Router::new()` parses 127 routes, measured on
    both parsers (decisions 8 and 28);
  - the completeness scanner passed over declaration shapes silently: it
    now reads generic, `where`, unit and private declarations, fails on
    any `struct` or `enum` it cannot read, and skips `->`; its test holds
    each shape (decision 17);
  - the long literal's threshold stated as measured: 210,000 digits abort
    at once; shorter literals do not overflow but lex in quadratic time
    (decision 20);
  - rows for P0001 and L0001 in the catalog (decision 9);
  - N's stack margin at the limit measured on Linux alone, CI's Windows
    run the check (decision 22).
- **A replay split the prototype into this plan's tasks,** on the local
  branch `split/5-1b`, by a script, and ran each task's steps as written:
  - from `4f32f13d`, a tests commit and a code commit for each of Tasks 1
    to 6, then Task 7's records, each the patch of its task below. The
    last commit's tree is the prototype head's
    (`git diff --stat proto/5-1b-whole split/5-1b` prints nothing). Each
    patch applies with plain `git apply`, in order, on `4f32f13d`, with no
    warning;
  - each tests commit failed as its step says before its code, and passed
    after it; the whole suite, but the IPv6 test, `cargo fmt`, clippy with
    `-D warnings` and `nova fmt --check` passed after every task:
    `2824 1 10` after Task 1, `2828 1 10` after Task 2,
    `2836 1 10` after Task 3, `2838 1 10` after Tasks 4 and 5 and
    `2839 1 10` after Task 6; Task 7 changes no code;
  - every "Expected:" below is the replay's, from a fresh `NOVA_HOME`, but
    Task 0's suite figures, which are 5.1a's last full run on `4f32f13d`.
- **The final checks ran on the split's last commit:**
  - the full suite gave `2839 1 10` (6 min 5 s): the base's `2816 1 10`
    and 23 tests more, which a comparison by binary finds as
    23 added and 0 gone (Task 8, Step 1); the one failure is the IPv6
    test;
  - CI's Linux leg, the whole suite under
    `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 … --all-features`:
    `2839 1 10`, the golden suite among them (Task 8, Step 2);
  - the golden suite, in 36 s of wall time: `release golden: 147 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`;
  - `nova parse --check` and `nova parse --ast` over every `.nova` file
    `4f32f13d` tracks, by the base's `nova` and the tree's: `parse: 624 runs over 312 files, 0 differ; --check exits nonzero on 14 at the base, 14 now`;
    `nova check` over the same files: `check: 274 checks, 0 outside std differ, 0 std files differ` (Task 8, Step 4);
  - `--help` of `nova` and of each of its 21 commands byte for byte the
    base's; the second file's error under five first arguments, as in
    5.1a; and `--dump-ast`'s errors `--dump-tokens`'s under the same five
    (Task 8, Step 5);
  - Rust 1.78's check was clean, in 42.28 s, and `Cargo.lock` did
    not change (Task 8, Step 6);
  - `selfhost.rs` in three runs: 24.7 s, 25.3 s and 24.4 s; N's `nova test`:
    `test result: ok. 17 passed; 0 failed; 0 trapped; 17 total` (Task 8, Step 7).
- **The mutants.** 92 breakages of what 5.1b adds, each a text
  replaced in one file, run one at a time at the split's last commit,
  under the process limit (Task 8, Step 8): S0's depth count, each kind's
  level, the limit both ways, the stop, the errors dropped, the cast's
  level, the postfix count, `code()` and each caller, the rollback's
  crossing (29); the thread,
  its stack, name, panic path and `nova run`'s program, the language
  server's and `nova dev`'s stacks (7); S0's dump, its spans, indent,
  names, floats, escapes, fields, splits, error order and notes, and the
  flag (13); the harness, its AST pass, the every-kind check, a kind or
  a `bool` value missing from `KINDS`, the scanner that checks it, its
  loud failure, generic parameters and `->`, a case that no longer holds
  a value, a `bool` read without its node's kind, N's depth test and the
  report (15); and N, its depth count and its rollback's crossing, each
  recovery point, the rollback's errors, `>>`, the newline rule's split and halt, the trailing
  expression, `component` and `view`, the record literal's restriction,
  a comparison chain, an operator, a context string, the arm note and its
  dump (28). 88 of 92 were caught, in
  45 min 38 s; the other four pass by design, std's loader's and
  organize-imports' use of P0002, `nova dev`'s stack and an equivalent
  guard (decision 49).
- **What the checks left standing,** each recorded or handed on:
  - macOS and Windows are reasoned from the code, not run here: N's stack
    there, the thread, line endings (decision 36);
  - each system's time for `selfhost.rs`, estimated here at about
    35 to 50 s on the slowest, is CI's to give; past a minute, ask the
    user (decision 43);
  - a decimal literal of 210,000 digits or more still aborts debug S0's
    64 MB thread, and a shorter one lexes in quadratic time: `logos`'s
    per-digit recursion is no level of the limit (decision 20);
  - where the base's own abandoned attempt reported an error past the
    limit, or the other reading fails before the crossing's token, the
    result is not the base's: no error for `x as Int < (((1)))` at 127
    parentheses, P0002 for P0001 for `x as Int < (*mut (((a))))` at 126
    (decision 19);
  - P0002 drops the item in progress and everything after it in the file
    (decision 19);
  - N is about 6× slower than release S0 with a release runtime, 30× with
    the debug one CI uses (decision 26);
  - the mutants that pass by design (decision 49).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` under `RUSTUP_TOOLCHAIN=1.78.0`
  with `RUSTFLAGS=-D warnings`. No `std::sync::LazyLock` (1.80), no
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79). 5.1b's Rust
  uses `std::thread::Builder::stack_size` and `std::panic::resume_unwind`
  (1.0), `Option::is_some_and` (1.70), binding `@` on a parenthesised
  or-pattern (1.53), `str::split_once` (1.52) and slices' `strip_prefix`
  (1.51), nothing newer.
- **No new crate, and `Cargo.lock` does not change** (spec §6.1). Any
  change to the lock stops the task: ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`. 5.1b's Rust outside tests has none. N's
  `unwrap()`s are on values its own code has checked.
- **One `unsafe` block, and its argument** (Task 2): `nova run`'s compiled
  program crosses from the pipeline's thread to the main thread in
  `cmd::run::Compiled`, `unsafe impl Send`, whose comment and ADR 0052 §3
  give the argument (decision 21). No other `unsafe` is added.
- **Tests never touch the internet,** bind no port, and need no `clang`
  (spec §6.1). The differential runs `nova`, the N it builds, and `sh` for
  `ulimit -s` on Unix (N's own stack, S0's 1 MB main thread, `nova run`'s
  8 MB one), nothing else.
- **Temporary names are fixed** (spec §6.1): under
  `std::env::temp_dir()`, `nova-selfhost-<name>`, one per use; 5.1b adds
  `ast-headers`, `deep-201`, `deep-202`, `deep-n`, `deep-s0`, `panic`,
  `long-number` and `run-stack` to 5.1a's, and `nova-cli`'s `fmt.rs` and
  `lsp.rs` add `too-deep` in their own fixed directories. Each is removed
  and made afresh by the test that uses it. A test that needs `$NOVA_HOME`
  sets a fresh one.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and the golden suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the suite here with `NOVA_REQUIRE_NODE=1`. No test of 5.1b needs Node.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and
  passes. 5.1b adds no runtime program, so it stays at 147.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary, which only a build
  of the workspace writes. `nova build` of `selfhost/` links against that
  runtime. Every run step starts with `cargo build --locked --workspace`
  (Conventions).
- **The process limit.** The differential starts two processes a batch,
  and the suite thousands; every test, mutant and suite runs under the
  limit of the Conventions, so that a mistake that forks without end fails
  to fork rather than filling the machine.
- **The stack.** S0's commands now run on a 64 MB thread (Task 2), and
  `depth.rs` parses on one of its own, since a debug parser 128 levels
  deep needs more than a test thread's 2 MB. Never lower
  `PIPELINE_STACK` or `MAX_DEPTH`, and never run a deep input on a test
  thread of the default size.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other. Never build in another worktree.
- **Keep `df -h /` above 6G.** Older duplicate test executables in
  `target/debug/deps` may be removed between commands, never while one
  runs. Each run of the whole suite leaves about 0.3 GB under
  `/tmp/nova-todo-*`, which is never cleaned (below): this plan's eight
  runs take about 2.4 GB, so begin with room for them; the first replay
  fell to 5.0G at its last golden suite for want of it, and the
  replays after the fact-checks, the last begun with 11G free,
  kept 11G. Every `cargo` command here selects the whole workspace
  (Conventions), so cargo keeps one copy of each test executable. Task
  8's Rust 1.78 check uses its own target directory, in `/dev/shm`,
  removed after.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.1b's
  patches holds one. Apply the patches with `patch_block`, from the
  committed plan.
- **No patch adds a CR, a NUL or trailing whitespace.** The inputs that
  need them are written by the tests at run time (`fuzz::edges`, the
  Windows-endings corpus), not kept as files: the six error cases with a
  control character among them (decision 24).
- **Nothing writes into the repository but the patches.** `nova build -o`
  writes N under the system's temporary directory and `nova test` writes
  nothing into the package; `git status --short` is empty after every run.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.1b.
- **This machine is Linux.** macOS and Windows results come from CI; what
  differs there is reasoned from the code (decision 36).

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-51b.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-51b-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-51b-<task>.txt"`. A task with a tests
  patch and a code patch makes one commit, after both.
- **The process limit.** Every shell that runs a test or a mutant first
  caps the tasks it may start at 600, threads as well as processes, and
  checks that the cap holds (Task 0, Step 2).
  - **As root, a pids control group.** Linux never applies `ulimit -u` to
    root, so the shell joins a group with that cap. On cgroup v1, as the
    replay ran:

    ```sh
    CG=/sys/fs/cgroup/pids/nova-limit
    mkdir -p "$CG" && echo 600 > "$CG/pids.max" && echo $$ > "$CG/cgroup.procs"
    grep nova-limit /proc/self/cgroup     # N:pids:/nova-limit
    ```

    On cgroup v2 (`stat -fc %T /sys/fs/cgroup` prints `cgroup2fs`), the
    root's `cgroup.subtree_control` must hold `pids` first, and the group
    is a child of the root:

    ```sh
    grep -qw pids /sys/fs/cgroup/cgroup.subtree_control || echo +pids > /sys/fs/cgroup/cgroup.subtree_control
    CG=/sys/fs/cgroup/nova-limit
    mkdir -p "$CG" && echo 600 > "$CG/pids.max" && echo $$ > "$CG/cgroup.procs"
    cat /proc/self/cgroup                 # 0::/nova-limit
    ```

  - **As any other user,** a systemd scope with the cap,
    `systemd-run --user --scope -p TasksMax=600 bash`, whose shell then
    runs everything; or, without systemd, `ulimit -Su 600`, which counts
    all the user's tasks, so a test may fail to start a thread sooner.
    Neither shell is in a group named `nova-limit`, so each checks its
    own cap (Runs, below), not the root's group.

  600 tasks was ample here: the replay's suites, builds, golden suite and
  mutants ran under it on 4 CPUs, and peaked at 65 tasks. Test threads
  scale with the cores, so a machine with many more may need a higher cap:
  if a test fails to start a thread, read the group's `pids.peak`, raise
  `pids.max`, and record it. `pids.peak` counts from the group's creation.
- **Runs.** The replay ran every command in the group and with these
  variables set. Each shell puts itself under the cap and sets them too,
  with three shorthands, and runs every command from the repository's
  root; a shell that is not under the cap stops at once:

  ```sh
  # The process limit (above), then the check that this shell is under
  # it, by one of these four, the first as the replay ran (root, cgroup v1):
  CG=/sys/fs/cgroup/pids/nova-limit
  mkdir -p "$CG" && echo 600 > "$CG/pids.max" && echo $$ > "$CG/cgroup.procs"
  grep -q nova-limit /proc/self/cgroup || exit 1
  # Root on cgroup v2: the v2 lines above, then
  #   grep -qx '0::/nova-limit' /proc/self/cgroup || exit 1
  # Another user, in the shell systemd-run started (cgroup v2), without
  # the lines above:
  #   [ "$(cat "/sys/fs/cgroup$(sed -n 's/^0:://p' /proc/self/cgroup)/pids.max")" = 600 ] || exit 1
  # Another user, after `ulimit -Su 600`, without the lines above:
  #   [ "$(ulimit -Su)" = 600 ] || exit 1
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-51b"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG
  mkdir -p "$NOVA_HOME"
  SH() { cargo test --locked --workspace --test selfhost -- "$@"; }
  S() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test run_tests -- release_golden_suite --nocapture "$@"; }
  ```

  So `SH` runs `selfhost.rs`, the differential, the flags' and the
  thread's tests, `S` the whole suite, and `G` the golden suite through
  LLVM as CI's Linux leg runs it. `NOVA_HOME` keeps a run by hand off `~/.nova`; every test that
  needs one sets its own.
- **Every `cargo` command selects the whole workspace,** `--workspace`
  with `--test NAME` or `--lib` to narrow it, never `-p`. Cargo unifies
  features over the packages a command selects, so each `-p` selection
  builds and links its own copy of every test executable it needs, which
  the disk does not have room for; a `--workspace` command reuses one
  copy. `S` gives each code step the whole suite, about six
  minutes; its one failure is the IPv6 test (Task 0).
- **The build.** Each "Run" step below starts with

  ```sh
  cargo build --locked --workspace
  ```

  which ends with `Finished`, so that the `nova` the steps run by hand and
  the runtime library beside it are the tree's (Global Constraints). The
  replay began every step with it.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the
  repository's root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-31-phase-5-1b-the-self-hosted-parser.md   # this file, committed
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
  nothing: no 5.1b patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. Task 4's tests patch
  adds 263 files and Task 5's code patch touches thirteen: each is one
  block all the same. The non-ASCII
  characters the patches add are `§`, `…`, `→`, `×`, `–` and `—` in prose and comments, and `é` in
  the tests' sources. A block that does not apply
  means the tree differs from the plan's: stop, and record why in the
  ledger.
- **Each task** but Task 7 is test first: its tests' patch, their run and
  the expected failure, then its code's patch, its run and the expected
  pass. Task 7's patch is the records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy,
    `cargo clippy --locked --all-targets --all-features -- -D warnings`, as
    CI runs it, which ends with `Finished` and no warning;
  - `target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost`,
    `selfhost` included, as CI runs it since 5.1a, with the `nova` the
    step's build made, which prints nothing.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost
  if wrong.

## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
check in the task that owns the code:

1. **N's parser against S0's** (`selfhost/src/parser.nova` and
   `parse_*.nova` against `crates/nova-parser/src/grammar.rs`): every
   message, context string and note word for word; the recovery points
   (`sync_to_item_boundary`, `sync_to_stmt_boundary`, the trait body's
   truncation); the generic arguments' rollback that keeps its errors
   and holds back a crossing;
   `>>` split through `pending_gt`; the newline rule's three splits and
   the halt after a bare `return` or `break`; the trailing expression with
   or without `;`; the `component` and `view` lookahead; the record
   literal's restriction in a condition; the precedence ladder and the
   chained comparison; the arm note (decisions 13 and
   14) → the differential's AST pass on the corpus, its Windows
   endings, the 278 cases, the edge inputs, soup seed 1, the mutations and
   the deep inputs, and N's own `@test`s (Tasks 5 and 6) (mutants
   65 to 92). A difference no generator
   makes is the risk the spec names first (spec §9 risk 1): the reviewer
   reads the recovery functions, `parse_generic_args_opt`,
   `bare_keyword_ends`, `parse_block_inner` and `component_at` against
   the Rust.
2. **The depth count, on both sides** (`Parser::deeper`, `nested`, the
   binary functions, `parse_postfix_chain`, `parse_type_here`, the stop
   in `peek` and `parse_file`; N's the same, entry for entry): each kind
   of nesting at 128 parses and at 129 is one P0002, the file's last line,
   at the token after the one that crossed the limit; the depth restored
   after every operand, chain and sub-pattern, so siblings never add up;
   a cast's type at its operator's level; a postfix chain's first
   operator and a method call's `(…)` free; nothing after P0002, the
   items before it kept; an abandoned attempt's crossing held back until
   the other reading passes its token or fails (decisions 18,
   19 and 28) → `depth.rs`'s seven tests (Task 1), the
   rollback's two cases (Task 4), the deep inputs
   through both differentials and `n_parses_128_levels_…`,
   `s0_meets_the_depth_limit_…` (Tasks 4 and 6) (mutants
   1 to 29,
   65 to 74). The reviewer reads where
   S0 and N enter and leave a level, side by side.
3. **The thread** (`nova-cli`'s `main.rs`, `cmd/run.rs`, `cmd/dev.rs`,
   `nova-lsp`'s `checker.rs`, `PIPELINE_STACK`): every command's exit
   code, streams and signals as before; a panic printed once, as
   `thread 'main'`, exit 101; `std::process::exit` from the thread; `nova
   run`'s program on the main thread, by `unsafe impl Send` on
   `Compiled`, whose argument the reviewer checks against
   `nova-codegen-cranelift`'s `CompiledProgram` and the runtime's
   thread-locals (decisions 20, 21 and
   37) → the LSP's P0002 test (Task 2), the thread's three
   tests in `selfhost.rs`, `s0_meets_…` (mutants 30 to
   36; 36 passes by design).
4. **Windows and macOS** (`selfhost.rs`, N's stack): N on Windows' own
   1 MB main thread at 128 levels, on Unix under `ulimit -s` (the least
   that passed is 256 KB); S0's 64 MB thread on every system, so the
   4,000-digit numbers no longer overflow; `ulimit` tests run on Unix
   only; paths and line endings as 5.1a's (decisions 36 and
   22) → `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`,
   `a_long_decimal_number_lexes_with_a_1_mb_main_thread`, the
   Windows-endings corpus's AST pass. CI's three systems are the only run
   of the other two.
5. **The dump, on both sides** (`nova-ast`'s `dump.rs`,
   `nova-parser`'s `dump.rs`, `ast_dump.nova`): ADR 0052's grammar, its
   table and its 72 examples; a node's span only where the tree holds
   one; names quoted with their spans; floats by their bits; strings'
   and messages' escapes; `children=`; the splits closing the tree; `ERR`
   lines first, then `PARSE` and `NOTE` in push order (decisions
   15 and 39) → `nova-ast`'s and `nova-parser`'s
   dump tests, the flag's test with its exact output (Task 3), the
   every-kind test on both sides (Tasks 3 and 6), the differential
   (mutants 37 to 46,
   89 to 92).
6. **The harness agreeing falsely** (`agree`, `agree_on_inputs`, the
   every-kind test, `compare.rs`'s `AST`): every input set through both
   dumps, the tests failing if a set skips the AST pass; each side's
   kinds and `bool` values checked on its own, and `KINDS` against
   `nova-ast`'s declarations, read by a scanner that fails on any it cannot
   read; the report naming the source line of an AST
   line (decisions 17, 23 and 41) →
   `every_declared_form_is_listed` (Task 3), the corpus, soup, mutations'
   and deep inputs' assertions, the mismatch test (Task 6) (mutants 50 to 64).
7. **The hidden flag** (`parse.rs`): absent from `--help`, every command's
   help the base's; conflicting with `--ast`, `--check` and
   `--dump-tokens`; the further files, `IOERR` and exit code 2 as
   `--dump-tokens`'s; errors under each first argument as
   `--dump-tokens`'s (decisions 16 and 40) →
   `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`
   (Task 3), Task 8's Step 5 (mutants 47 to
   49).
8. **The records:** ADR 0052 against the spec's §4 and §8 and this plan's
   decisions, its examples against S0; the CHANGELOG's Added and Changed;
   the dated notes, each naming the branch
   `claude/loving-archimedes-frtf5q-51b`; the catalog's P0002, P0001 and
   L0001 → Task 7,
   Step 2.

## Decisions

Made while writing the spec (its header and §4), prototyping, probing,
splitting and replaying, each as built and each read against the code.
Where one departs from the spec, it says so and why, and is marked
**(differs)**; one that fills in what the spec left to the plan says so,
unmarked. ADR 0052 records 5.1b's departures and the points it settles
(spec §8).

**The user's decisions, as built**

1. **5.1b is the second pull request** (the user's decision 1): the AST,
   the parser, the AST dump on both sides, parse errors compared, the
   depth limit and the speed, on `main` after 5.1a.
2. **A depth limit in both parsers** (the user's decision 2): S0 and N
   count nesting the same way and, past 128 levels, report P0002 at the
   same token in place of a stack overflow; a language change, in ADR
   0052 and the CHANGELOG's Changed (decisions 18, 19 and 28).
3. **The AST dump is an indented S-expression with spans** (the user's
   decision 3): one node per line, two spaces a depth, `(Kind
   start..end …`; floats by their bits, as 5.1a's token dump (5.1a's
   decision 4; decision 15).
4. **Parse errors are compared completely** (the user's decision 4): the
   dump ends with every lex error, then every parse error with its notes,
   in push order, spans and messages, and the tree ends with the newline
   rule's splits, from which E0103 is computed (decision 15).
5. **The fuzzer gains its third generator, with fixed seeds** (the user's
   decision 5): deep nesting, seeds 201 and 202, in `cargo test`, no new
   crate (decision 25).
6. **Speed is measured and recorded, with no threshold** (the user's
   decision 6): N is 6.1× release S0 on std concatenated, with a release
   runtime, under the 10× past which the user is asked (decision 26).
7. **The differential runs in `cargo test` on all three systems** (the
   user's decision 7): N built by `nova build`, Cranelift, no `clang`
   (decision 36).
8. **Postfix chains count toward the same 128 levels** (the user's ruling
   of 2026-10-31, asked after step 0): each postfix operator after a
   chain's first is one level, a method call `.f(…)` one, so after a name
   129 calls or field accesses parse and 130 give P0002, a call's
   arguments being a level deeper than the call (128 calls with arguments
   parse, decision 28); a departure from the spec's
   list of levels, recorded in ADR 0052 and the CHANGELOG (decision 28).

**The spec's recommendations, not asked, taken**

9. **The limit is 128 levels, and the code is P0002** (spec §4.4, §5):
   `nova_parser::MAX_DEPTH`, N's `MAX_DEPTH`; "this is nested too deeply
   (more than 128 levels)"; P0002 in `12-TYPESYSTEM.md`'s catalog beside
   P0001 and L0001, which gain rows of their own there, codes every lexer
   and parser error has had since Phase 1 (decision 54).
10. **S0 runs its pipeline on a thread with a 64 MB stack** (spec §4.4):
    `nova_driver::PIPELINE_STACK` (decision 20).
11. **The parser's and lexer's tests' inline sources are the error
    corpus** (spec §4.6; decision 24).

**The spec's design (§4), as built**

12. **N's AST** (spec §4.1): `ast.nova`, a record per struct and a sum per
    enum, the same fields in the same order, a variant with named fields
    holding them positionally; spans as `token.nova`'s `Span`, two `Int`s,
    no `FileId`; recursive sums with no box, so `Option<Box<…>>` is
    `Option<…>`; names `String`s. 245 code lines for the Rust's 477.
13. **N's parser** (spec §4.2): `parser.nova` (state, token access,
    recovery, docs, the `component` lookahead, paths, `error.rs`,
    `lib.rs`), `parse_item.nova`, `parse_type.nova`, `parse_stmt.nova`,
    `parse_expr.nova`, `parse_view.nova` and `parse_pattern.nova`: each
    Rust function a Nova function of the same name and shape, in the same
    order, with its comments; one added, `value_at`. Every message,
    context string and note word for word, the rollback that keeps its
    errors and holds back a crossing (decision 19), `>>`, the statement
    recovery, the trailing expression, the
    lookaheads, the splits and the halt, the depth count entry for entry.
    Not ported: dropped items' names, the language server's alone. 2,485
    code lines for the Rust's 2,948 (`grammar.rs`, `error.rs`, `lib.rs`;
    2,836 at the base), counted as 5.1a counted (no blank or comment
    line, no `#[cfg(test)]` item).
14. **Each Rust idiom has one translation** (spec §4.2), ADR 0052 §6's
    table: `discriminant` as `Token.kind()`, a number per variant; a match
    on the token `peek` returns; the 21 `in_*_ctx` closures as
    `enter_stmt_ctx`/`enter_no_struct_ctx` and a restore, the `?` after
    it; `nested` as the depth saved, `deeper()`, the call, the restore,
    and S0's closures of more than one call as functions both sides share
    (`parse_type_here`, `parse_pattern_here`, `parse_view_children_here`,
    and step 0's `parse_postfix_chain`); `HALT` as the field `eof`; the
    tokens as a field; `halt` as an `Int`, -1 for none; the rest in the
    table.
15. **The AST dump** (spec §4.3): `nova_ast::dump::dump_file` writes the
    tree, `nova_parser::dump_ast(source)` lexes, parses and adds the
    errors, `ast_dump.nova` writes the same bytes. ADR 0052 §1 writes the
    grammar out, a table of every node, and an example of every kind,
    regenerated from S0 (decision 39). A struct's kind is its name, an
    enum value's `Enum.Variant`; `Doc`, `Attr` and `Split` as the spec
    names them, but printed otherwise than its sketch **(differs)**: a
    split's kind as the field `kind=Call`, not a bare `Call`, the uniform
    `key=value` of every field-less enum, and an attribute's name and
    arguments as names with their spans, `"test"@1..5
    "should_panic"@6..18`, not a bare name and quoted strings, since each
    is a `Spanned<String>` and every name prints its span; a node's span where the tree holds one; scalar fields
    `key=value` in the struct's order, then children in order; absent
    options and empty lists print nothing; a name `"text"@s..e`, a path
    names joined by `::`; literals `Int:`, `Float:` and 16 hex digits,
    `Str:`, `Char:` and the code point, `Bool:`; a string's bytes below
    0x20, 0x7F, `"` and `\` as `\xHH`, a message's below 0x20 and 0x7F
    alone; `ViewNode.Element`'s `children=Some|None`, since `None` and an
    empty list would print alike; then `ERR`, `PARSE`, `NOTE` lines, the
    `PARSE` line with no code, P0002's message naming it
    (`a_tree_with_every_kind_of_field`, `the_errors_follow_the_tree`;
    mutants 37 to 46).
16. **S0's hidden flag** (spec §4.3): `nova parse --dump-ast FILE…`,
    `hide = true`, conflicting with `--ast`, `--check` and
    `--dump-tokens`, sharing 5.1a's hidden `more`, its `== FILE` headers,
    `IOERR` and exit code 2 through one `dump_each`; `--help` byte for
    byte the base's
    (`the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`;
    mutants 47 to 49).
17. **Every kind appears in a dump, on both sides** (spec §4.3):
    `nova_ast::dump::KINDS` lists the 167 forms the dump can print (24
    structs and `Doc`, the 18 enums' 119 variants as node kinds, `key=Word`
    values and literal prefixes, and the 11 `bool` fields' two values and
    `children=`'s two after their node's kind, `(Type.Ptr is_mut=false`,
    but `Pattern.Ident`'s `is_mut=true`, which no parser builds).
    `the_kinds_are_well_formed_and_distinct` pins the list;
    `every_declared_form_is_listed` reads the `struct` and `enum`
    declarations of `nova-ast`'s sources, whatever their visibility, with
    a small scanner that fails on any it cannot read, and checks that
    `KINDS` is exactly what they imply, so a new variant or `bool` field
    fails until it is listed, and `the_scanner_reads_the_declarations`
    checks the scanner on known forms and on each shape: generic, `where`,
    `pub(crate)`, private and unit declarations and a field of a function
    type read, a tuple struct and a discriminant refused; `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`
    reads S0's dumps (Task 3) and N's (Task 6) of the corpus and the
    cases, each on its own, and `kinds_on` takes a `bool`'s or
    `children=`'s value with its node's kind (mutants
    52 to 62).
18. **The depth count** (spec §4.4): one counter, `depth`, raised by
    `Parser::deeper`, which refuses past `MAX_DEPTH + 1`; the outermost
    expression, type, pattern or nested item is level 0, so `n` nested
    constructs of one kind are `n` levels. A level is an expression, a
    type or a pattern inside another; a prefix operator's operand; each
    `else if`; each binary operator, assignment, `..` and `as` included,
    for the rest of its chain, the type after `as` at its operator's
    level; each postfix operator after a chain's first (decision 28); an
    item inside a block; a view's or an element's children. Every entry
    but the chains' is `nested`, which restores the depth however the
    call ends; a binary function restores it when its chain ends,
    `parse_postfix_from` after the chain however it ends. The corpus
    nests at most 25 (a router chain), then 18 and 14; its longest binary
    chain is 9 and `else if` chain 14 (`depth.rs`; mutants
    1 to 16).
19. **Past the limit, one P0002, and the parser stops** (spec §4.4): the
    first crossing records the errors so far and the span of the token at
    the cursor; `peek` then reads the end of the file, so every function
    unwinds; at the file's end the errors pushed since are dropped and
    `ParseError::TooDeep` pushed, the file's last error. `code()` gives
    "P0002" for it and "P0001" for every other, and each caller attaches
    it: `nova parse`, the driver's loader, the resolver's std loader,
    `nova fmt`; organize-imports counts P0002 as a syntax error. The
    language server and `nova fmt` give one diagnostic on such a file
    and go on (spec §6.2; `a_file_nested_too_deeply_gets_one_p0002`,
    `a_file_nested_too_deeply_is_one_p0002_and_left_untouched`; mutants
    17 to 25). An abandoned attempt
    does not decide alone (decision 54): the generic
    arguments the parser tries after a type's `<` and rolls back can be
    deeper than the reading that follows. `roll_back` restores the P0002
    state with the position, drops the errors pushed after the crossing,
    and keeps the crossing waiting in `rolled_back` with its token; it
    stands, as if never rolled back, if an error comes before the parser,
    reading the tokens the other way, reaches that token, and goes once it
    does. So `x as Int < (((a)))` parses at 127 parentheses (126 after two
    casts) and is P0002 at its own deepest token at 128, and a parameter's
    `Vec<Vec<…>>` past the limit is still one P0002 at its leaf. Where the
    base's own attempt reported an error past the limit
    (`x as Int < (((1)))` at 127), that error goes with it, and where the
    other reading fails first (`(*mut (((a))))` at `mut`) P0002 stands
    where the base gave P0001: inputs in error either way. The trait
    body's lookahead re-reads at the same depth, and only after a `{` a
    crossing would hide; the `component` and `view` lookaheads read no
    nesting
    (`a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`,
    `abandoned_attempts_past_the_limit.nova`,
    `a_rolled_back_crossing_that_stands.nova`; mutants
    26 to 29 and
    67 to 69).
20. **The thread** (spec §4.4): `nova`'s `main` parses its arguments,
    sets up logging and the embedded runtime, then runs the command on a
    thread named `main` with `PIPELINE_STACK`, 64 MB. A result returns to
    the main thread and is returned as before; a panic prints on that
    thread as `thread 'main' … panicked at`, then `resume_unwind` carries
    it out of `main` unprinted, exit 101; `std::process::exit` ends the
    process from either thread; a stack overflow aborts it from either.
    `nova run` compiles on the thread and runs the program on the main
    thread (decision 21), so a program's stack under `nova run` is the
    one it has when built. The language server's checker thread and
    `nova dev`'s rebuild thread get the same stack; `nova test` runs built
    binaries, as before. The thread bounds the parser's recursion, which
    the limit bounds, but not every overflow: `logos`'s per-digit
    recursion (ADR 0051) is no level, so a decimal literal of 210,000
    digits or more aborts debug S0's 64 MB thread at once, in its first
    scan of the literal, one frame a digit (210,000, 250,000 and 300,000
    abort in about 50 ms under `nova parse --dump-tokens` and
    `--dump-ast`). A shorter one does not overflow, every later scan being
    shallower, but lexes in time quadratic in its length (20,000 digits in
    7.6 s, 40,000 in 42 s), so 207,500 had not aborted after 60 s, which
    is all "pass" meant there (decision 54). No generator
    makes one (the thread's three tests, the LSP's P0002 test,
    `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`; mutants
    30 to 36).
21. **One `unsafe impl Send`, argued** (Task 2): `cmd::run::Compiled`
    wraps the JIT's `CompiledProgram`, which Rust cannot prove `Send`
    because its module holds raw pointers. It is sound because that
    module owns the code and data the pointers name, mapped for the
    process; the compiling thread has ended when the main thread runs it,
    so one thread uses it at a time; and the runtime's per-thread state
    (heap, executor, file and socket tables) is made on first use by the
    thread that runs the program, compiling touching nothing of the
    runtime but its symbol table, and `set_args` being a process-wide
    `OnceLock`. The comment above the impl and ADR 0052 §3 say so
    (`nova_run_runs_the_program_on_the_main_thread`; mutant
    34).
22. **N's own stack at the limit** (spec §4.4): N, built by `nova build`,
    passes 128 and 129 levels of every deep kind on an 8 MB and a 1 MB
    main thread; the least stack that passed at 128 was 256 KB
    (parentheses, blocks, `if`, `match`, `while`, `for`, calls), 128 KB
    (items) and 64 KB (the rest, method calls among them). The spec's stop
    and question to the user did not arise: the limit stays 128 and the
    link is unchanged. The margin was measured on Linux: Windows x64
    frames may be 1.5 to 2 times larger (the shadow space, saved `xmm`
    registers), so CI's Windows run of the test, on that system's real
    1 MB main thread, is the check, as ADR 0052 §8 and `11-PARSER.md` §5
    say.
    `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread` runs it on
    8 MB and 1 MB under `ulimit -s` on Unix and on Windows' own 1 MB main
    thread (mutant 63).
23. **The comparison** (spec §4.5, §4.8): `agree` runs every batch
    through the token dumps and then the AST dumps, compared byte for
    byte with their exit codes: the corpus (every `.nova` file outside
    `selfhost/`, then N's sources) and its Windows-endings copy; the 278
    cases (5.1a's 12, the 262 recorded, `every_other_node.nova`,
    `chains_and_casts_at_the_limit.nova`, decision 49, and the rollback's
    two, decision 19); the
    edge inputs; soup seed 1's 2,000 inputs (decision 33); the 750
    mutated files of seeds 101 to 103; the 180 deep inputs of seeds 201
    and 202. A mismatch report names an AST line's source line by its
    node's start or its error's; both sides refuse no file for either
    dump (`the_mismatch_report_names_the_file_and_line`,
    `both_sides_refuse_no_file`; mutant 64).
24. **The error corpus** (spec §4.6): 262 files, recorded rather than
    copied: the five test binaries of `nova-lexer` and `nova-parser` ran
    with `lex` and `lex_with_comments` recording each source and the test
    that lexed it; property tests' random inputs and the whole-tree test
    skipped; sources equal to a tracked file (the six fixtures) dropped;
    duplicates kept once under the first test in sorted order; the 32
    keywords one file. Each is `<test file>__<test>[_n].nova`. The six
    with a `\r` or another control character, which a checkout cannot
    keep, and three decimal numbers of 4,000 characters (ADR 0051's known
    bug) are `fuzz::edges` instead. Both lexers agreed on all.
25. **The deep generator** (spec §3.5 generator 3): `fuzz::deep(rng,
    kind, depth)`, a program whose deepest level is `depth`, nested as
    one of 14 kinds (parentheses, blocks, `if`, `else if` chains, `match`,
    `while`, `for`, calls, prefix operators, binary chains, method calls,
    types, patterns, items in blocks) or mixed, eight wrappers of one
    level in a random order; well typed up to the limit but for nested
    items, E0900. `fuzz::deeps(seed, 20)` gives each kind at 126 to 130
    and 20 mixed: 90 inputs a seed, through both differentials, and seed
    201's through S0's `parse --check`, and `check` at 128 and 129, with
    no overflow and one P0002 past the limit.
26. **The speed** (spec §4.7), measured once, locally, best of three,
    peak memory by `wait4`: N (`selfhost ast`, built with a release
    `libnova_runtime.a`) lexes, parses and dumps all of std (25 files,
    697 KB) concatenated in 0.244 s and 46 MB, against 0.040 s and 18 MB
    for release S0's parser and dump: **6.1×**; 5.5× on the 25 files in
    one process; 30× and 33× with the debug runtime CI uses. ADR 0052 §9
    holds the table; CI checks no time (decision 32).
27. **The grammar changes in both compilers, and the records** (spec
    §8): from 5.1b on a change to what the parser accepts, builds or
    reports changes `nova-parser`/`nova-ast` and `selfhost/` in one pull
    request, the dump on both sides with them (ADR 0052 §10); ADR 0052,
    the CHANGELOG's Added and Changed, dated notes in `11-PARSER.md` (§4
    the dump, §5 P0002), `00-MASTER-SPEC.md` and `docs/phase-5-plan.md`,
    each naming `claude/loving-archimedes-frtf5q-51b`, and P0002, with
    rows for P0001 and L0001, in `12-TYPESYSTEM.md`'s catalog (Task 7).

**Where the prototype departs from the spec, or settles what it left open**

28. **Postfix chains are counted, by the user's ruling.** **(differs)**
    The spec's list of levels has no postfix operators. Step 0 found that,
    uncounted, a chain of about 5,000 method calls overflowed `nova
    check`, `run` and `build` even on the 64 MB thread (4,000 passed; the
    base binary on its 8 MB thread passed 500 and aborted at 600), below
    the 10,000 past which a count was to be added. Asked, the user ruled
    (decision 8) that each postfix operator after a chain's first (a call,
    an index, a field, `?`, `.await`) is a level for the rest of the
    chain, and a method call's `(…)` counts with its `.name`: after a
    name a chain of 129 field accesses or 129 method calls without
    arguments parses, 130 is P0002, at the token after the operator, as
    after a binary operator; a call's arguments are a level deeper than
    the call, so 128 calls with arguments parse and the 129th's arguments
    are P0002; nested calls and
    method calls count one each, as before. Both parsers count alike
    (`parse_postfix_chain`), `depth.rs` has two kinds more and the deep
    generator one; a chain of 100,000 is one P0002. Its cost: a router
    built as one chain from `Router::new()`, whose call is the chain's
    first operator, one `.get("/", h)` a route, parses 127 routes and is
    P0002 at the 128th (each figure measured on both parsers); the
    repository's
    longest chain is 21 calls (`examples/07-fullstack-blog/src/server.nova`),
    whose file now nests 25 levels, the deepest (mutants
    13 to 16,
    70, 71).
29. **A block is not a level; its statements are.** **(differs)** The spec
    lists "a block" as a level. Counting a block and its statements'
    expressions both would make `{ x }` two levels and 64 nested blocks
    the limit; counting the statements alone keeps `n` nested blocks `n`
    levels, as `n` parentheses.
30. **P0002 drops the item in progress.** **(differs)** The spec says the
    tree holds what came before; it holds the items finished before, and
    the item being parsed, however far it got, is dropped with every
    error and split after the crossing: one clean error, and both parsers
    stop at the same point without carrying a half-built tree.
31. **The dump is split between two crates.** **(differs)** The spec's §7
    puts a `dump` module in `nova-ast`; the tree's dump is there, and the
    errors' part, which needs the lexer's and parser's error types, is
    `nova_parser::dump_ast`, next to the parser it describes.
32. **S0's release speed was measured as a library.** **(differs)** The
    spec times S0 "built in release mode": a release `nova` would have
    taken the disk below its floor, so release S0's lexer, parser and dump
    were built into a temporary example of `nova-parser`, not kept, that
    reads the files and prints the same dump on a 64 MB thread; debug
    `nova parse --dump-ast` was timed too (ADR 0052 §9).
33. **One soup seed of three goes through the AST pass.** The spec's gate
    names the corpus, the cases, the mutated and the deep inputs; soup is
    made for the lexer and is mostly one parse error after another, so
    seed 1's 2,000 inputs go through the AST pass and seeds 2 and 3 the
    token pass alone, keeping CI's time for the mutations, which reach far
    into the parser (decision 43).
34. **N's names.** A variant is named `EnumVariant` in N (`ExprIf`,
    `PatternLit`, `BinOpAdd`): a variant's name is a value of its module,
    one module may hold only one of a name and may not import two
    (E0002), and the AST's Rust names repeat across its enums and with
    the tokens'. `token.nova`'s `Spanned` is generic, `Spanned<T>`, as
    nova-diagnostics' is, its 17 uses in the lexer `Spanned<Token>`. The
    dump's `Enum.Variant` is the name with a dot.
35. **N's own `@test`s are in `selfhost/src/parser_tests.nova`.**
    **(differs)** As 5.1a's decision 13, for the same reason: a module in
    `selfhost/tests/` cannot reach the parser. Six tests, each expected
    dump S0's: the rollback and `>>`, statement recovery, the trailing
    expression, `component` and `view`, a bare `return` and the arm note,
    128 against 129 parentheses; `nova test` in `selfhost/` runs 17.
36. **Windows and macOS, reasoned from the code.** N runs on the
    platform's main thread: on Windows it runs as built, 1 MB, and the
    test asks it to pass 128 levels there, where it needs a quarter of
    that. S0's thread is 64 MB everywhere, so the 4,000-digit numbers
    pass on Windows' 1 MB main thread, as `a_long_decimal_number_…` shows
    under `ulimit -s 1024` on Unix. The `ulimit` tests and the panic test
    (`/dev/full`) run on Unix or Linux alone; `nova run`'s main-thread
    test on Unix. Line endings and paths are 5.1a's, and the
    Windows-endings corpus runs through the AST pass on every system.

**Step 0's decisions, before the split**

37. **The thread, probed against the base binary** (`$P/nova-base-51b`),
    both run as `nova`: `parse`, `check`, `fmt`, `run`, `build`,
    `bundle`, `test`, `doc`, `version`, `lsp`, `fetch`, `update`,
    `package`, `publish`, `login`, `add`, `remove`, an unknown command and
    none, on success and on failure: both streams and every exit code the
    same. `nova run` of a program that reads stdin, exits 3 after an
    unterminated `print`, panics (134) and takes its arguments, empty
    included: the same; one that recurses past its stack overflows both
    between 520,000 and 550,000 frames on 8 MB, so the program's stack is
    unchanged. `nova test` with a passing, failing, panicking, exiting and
    overflowing test: the same report. `nova dev` under SIGINT and
    SIGTERM, `nova run` under SIGINT: killed by the signal, as before. A
    new test holds the main thread at 8 MB and checks that `nova run`'s
    recursion of 2,000,000 overflows where 100,000 runs, which a program
    on the 64 MB thread would not
    (`nova_run_runs_the_program_on_the_main_thread`; mutant
    34). The `Send` argument went into the comment
    and the ADR (decision 21).
38. **P0002, probed at its edges** by both parsers at 127, 128 and 129
    levels, the dumps compared: parentheses, blocks, `if`, `else if`,
    match arms, closures' bodies, interpolation holes nested and holding
    parentheses, calls, indexes, arrays, tuples, prefix operators, `!`,
    binary chains, assignments, ranges, casts, a cast's generic type,
    generic arguments in a parameter's and a `let`'s type, tuple, function
    and optional types, tuple-struct, tuple, record, match and
    or-patterns, items in blocks, `while`, `for`, `return`, record
    literals, views, mixes, and, after decision 28, chains of fields,
    indexes, `?`, `.await`, tuple fields, calls and method calls, nested
    method calls: S0 and N agree on every one, none is P0002 at 128, and
    each is one P0002 at 129, the dump's last line. `nova check` at 128 of
    each ran every stage without an overflow, and `nova fmt` at 129
    reported the one P0002 and left each file untouched. Wide inputs need
    no count: 20,000 `let`s, statements, array or tuple elements,
    interpolations, match arms or items pass `nova check`.
39. **The dump, probed on both sides.** ADR 0052's examples, regenerated
    from S0 by the prototype's script, are byte for byte the ADR's; S0
    and N print the same bytes on all of them and on 30 edge files:
    escapes in strings, raw strings, docs, `extern`'s ABI and chars;
    non-ASCII names; floats `0.0`, `-0.0` (a negation), the least
    subnormal, the least normal, an overflow to infinity, the largest,
    `0.1` and an underflow to zero; `Int`'s largest in decimal and hex,
    other radixes, `_`, a leading zero; an overflowing literal; tuple
    fields refused; holes; an unterminated string; the three splits; a
    bare `return` before a value; elements with and without children;
    the arm note; an empty file, one of comments alone, one of errors
    alone, a byte order mark, a NUL; lex and parse errors interleaved in
    the source, the `ERR` lines first, as the callers render them.
40. **`--dump-ast`'s arguments.** No file, an empty first file and an
    empty further one, `--`, a value given to the flag, the flag twice,
    each conflict in either order, a directory, a file that is not UTF-8,
    a missing one: each as `--dump-tokens` gives it, with the flag's name.
    `usagecmp.sh` gains the check under 5.1a's five first arguments, and
    `helpcmp.sh` compares `--help` of `nova` and each of its 21 commands
    with the base's (Task 8, Step 5).
41. **The harness fails when an input set skips the AST pass.** `agree`
    returned S0's token dump alone, so a harness that skipped the AST
    pass would pass every test. It now returns the token dump and then
    the AST dump; the corpus test counts one tree per file, the soup's
    and the mutations' find trees, and the deep inputs' find P0002
    (mutants 50 and 51).
42. **`every_other_node.nova` names, with the base's corpus, every
    kind.** It held the twelve kinds no file did; it now holds the twenty
    the base's corpus lacks (a module, an item in a block, an array
    pattern, a type alias and the splits before `[`, `-` and a value
    besides), so S0's every-kind test stands in Task 3, before the error
    cases. The fact-check's fixes add the three values no file held: a
    record pattern without `..`, a pointer type without `mut`, and a
    trait's `async` signature (decision 53).
43. **CI's time, estimated; no count changed.** `selfhost.rs` takes
    24.7 s, 25.3 s and 24.4 s here, wall, 55 to 60 s of CPU on 4 cores (ADR 0052 §7
    gives step 0's three runs, 21.5 to 23.4 s by the test's own clock,
    the plan's the replay's, by `time`). In 5.1a, the
    same measure gave 11.7 to 12.6 s here (the replay) or 7.5 to 8.5 s
    (the execution), and CI took 16.5 s on macOS, 10.7 s on Windows and
    6.8 s on Linux: so about 35 to 50 s on macOS, 20 to 30 s on Windows
    and 13 to 20 s on Linux, under a minute. The soup's AST pass, the one
    cut the spec allows, costs about 1 s of CPU, so no count was lowered.
    The spec asks the user past a minute; if CI shows more, Task 8 stops
    and asks (Step 11).
44. **The records made true.** ADR 0052 is accepted, names the branch, and
    gains decision 28, the probes, the `Send` argument and the figures
    (the corpus's 25, the chain figures, N's 2,485 lines); each dated note
    names `claude/loving-archimedes-frtf5q-51b`; the CHANGELOG's Changed
    names postfix chains; `docs/phase-5-plan.md`'s note records decision
    28.

**The split's decisions**

45. **Seven tasks, six test first, then the records:** P0002 and the depth
    count with `code()` and its callers; the 64 MB thread; S0's dump and
    the flag with S0's every-kind test; the error cases and the deep
    generator through the token differential; N's AST, dump and parser,
    with `selfhost ast` and N's `@test`s; the AST comparison and N's
    depth test; ADR 0052 and the records. Each stands alone: Task 1's
    tests parse on a 64 MB test thread and need no pipeline thread;
    Task 3's every-kind test reads the corpus and `every_other_node.nova`
    (decision 42); Task 5's parser is whole before Task 6's harness
    compares it, so no tree holds a half-ported parser the harness
    compares. N's AST and dump are not a task of their own: a dump of
    trees N cannot yet parse would need tests written for that alone.
46. **`selfhost.rs` is cut by task:** Task 2's tests add the section's
    head and the thread's three tests; Task 3's the flag's test,
    `kinds_on` and the every-kind test reading S0 alone; Task 4's the deep
    seeds, `the_deep_inputs_agree` through `agree_on_inputs(name, inputs)`
    and S0 at the limit; Task 6 the module doc, the uses, the counts, the
    AST pass and its assertions, N's depth test, the every-kind test's N
    side and the AST parts of the older tests.
47. **Tests that cannot stand first, and tests that pass first.** Task
    1's `depth.rs` names `MAX_DEPTH`, Task 3's tests `KINDS`, Task 4's
    `fuzz::deeps`, Task 6's `AST`, so each fails to compile before its
    code; Task 1's `nova fmt` test overflows, Task 2's LSP test overflows
    the checker's thread and its long-number test S0's 1 MB main thread;
    Task 5's `the_selfhost_tests_pass` counts 17 `@test`s and runs 11.
    Task 2's panic test and `nova run` test pass before the thread, as
    they must: they pin behaviour kept.
48. **Every commit builds correct code and passes the suite.** Each code
    step ran the whole suite, `cargo fmt`, clippy and `nova fmt --check`
    (`2824 1 10` after Task 1, `2828 1 10` after Task 2,
    `2836 1 10` after Task 3, `2838 1 10` after Tasks 4 and 5 and
    `2839 1 10` after Task 6; Task 7 changes no code).

**The mutants**

49. **The mutants: three gaps closed, four accepted.** The first run, at
    the split of step 0's first commit, caught 66 of 73. Three gaps, each
    closed in step 0 before this split:
    - a binary chain or a postfix chain that kept its raised depth (mutants
      11 and 16), and N's cast
      type counted one level deeper (mutant 74): no input
      put an operand deep enough after a chain, and no deep kind was a
      cast. `chains_and_casts_at_the_limit.nova`, a fixed case, holds a
      chain of each precedence and a postfix chain followed by an operator
      of the next lower one whose operand is 100 deep, a view whose first
      child is a postfix chain, and 128 casts; `depth.rs`'s siblings test
      holds the same for S0 alone. A binary function's restore at its end
      also covers a postfix chain under it, so only a view child, which
      reads a postfix expression outside any binary function, shows the
      postfix restore missing. N's two restores are mutants too
      (72, 73);
    - beside them, `nova-ast`'s every-field test now dumps `0.1`, whose
      bits have letters, so S0's lowercase hex is pinned on its own, as
      5.1a's decision 41 pinned the token dump's.

    Four pass by design, each a mutant no test can reach without what the
    tests may not do, and each read by the review (Task 8, Step 10):
    - 24, std's loader attaching P0001: std nests at
      most 13 levels, so the loader never meets P0002;
    - 25, organize-imports not counting P0002: a
      P0002 in the file being organized has its label there, which refuses
      already; only one in another module of the project needs the code,
      and no test builds such a project;
    - 36, `nova dev`'s rebuilds on a default stack: `nova
      dev` serves on a port, which no test may bind;
    - 19, P0002 recorded at each crossing: equivalent,
      since after the first every token reads as the end of the file and
      the parser enters no level again; the guard keeps the first should a
      path ever cross twice.

    The fact-check's fixes (decision 53) added seven, one for
    each way its new checks could go blind (mutants 53 to
    62): a kind and a `bool` value missing from `KINDS`,
    its count kept; the scanner reading no `bool` field, no variant's
    fields, or one file fewer; `every_other_node.nova` without its record
    pattern that lacks `..`; and `kinds_on` taking a `bool` without its
    node's kind.

    The second fact-check's fixes (decision 54) added ten:
    in S0, the rollback keeping the crossing, dropping it never to stand,
    keeping the errors pushed after it, and the crossing waiting past its
    token (mutants 26 to 29);
    in N, the first, second and fourth (67 to
    69); and the scanner passing over a declaration
    it cannot read, closing a bracket at `->`, and reading no generic
    parameters (58 to 60).

    The whole set, 92, ran again at the split's last commit:
    88 caught, the four accepted passed, none left unrun. Every
    mutant of N's parser failed the differential's tests, not N's own
    `@test`s alone.
50. **The process limit** (Conventions): 600 tasks, set by a pids control
    group for root on cgroup v1 or v2, and by a systemd scope's `TasksMax`
    or a soft `ulimit -Su` for any other user; every shell checks that it
    is under the cap and stops if not. The replay ran as root on cgroup v1
    and 4 CPUs, every step in the group `nova-limit`.
51. **The mutants leave the tree built:** Task 8, Step 8 ends with
    `cargo build --locked --workspace`, so `target/debug` holds the last
    commit's binaries for the steps after it.

**What the replay met**

52. **Nothing unexpected in the code; three things in the first replay.**
    Every step gave its "Expected:", and no test failed but the IPv6 one,
    in eight whole suites and CI's leg; the suite took 5 min 29 s to 6 min
    43 s a run, and the group's peak was 65 tasks of 600, on 4 CPUs.
    - `parsecmp.sh` first ran the tree's `nova` on the tree's text, as
      5.1a's did, and so showed `--ast` differing on the four N sources
      5.1b changes: it runs both binaries on the base's text now, as Task
      8, Step 4 has it, and the 624 runs agree.
    - Free disk fell to 5.0G during the last golden suite, under the 6G
      floor: each whole-suite run leaves about 0.3 GB under
      `/tmp/nova-todo-*`, which the rules never clean (Global
      Constraints).
    - The mutants' first run found three gaps, closed before this split
      (decision 49).

    After the fact-check's fixes (decision 53) the split was made again:
    Tasks 1 and 2's four commits have the trees the first replay ran, so
    their figures stand; Task 3 onwards and every final check ran again,
    each as expected, with 13G free at the start and 12G at the end.

    After the second fact-check's fixes (decision 54) the split was made
    again, and every step and final check ran again from Task 1, each as
    expected, with 11G free at the start and at the end.

**The first fact-check**

53. **Its findings, fixed in one prototype commit,** `9be3dd8e`, before the
    split and the replay were redone. It found no code fault: 0
    differences on 586 files, about 33,000 mutations of its own, about 90
    hand files and over 60 depth shapes; the thread as the base on all
    176 runtime programs, the GC stress, signals, stdin and `nova test`.
    - The postfix ruling is the user's (decision 8), and ADR 0052's Status
      and postfix bullet, the CHANGELOG and `docs/phase-5-plan.md`'s note
      say so; decision 28 cites it.
    - The thread does not stop every overflow: a decimal literal of
      210,000 digits or more aborts debug S0's 64 MB thread, first
      measured by halving between 200,000 and 240,000 under `timeout`,
      restated by the second fact-check (decision 54); `11-PARSER.md` §5,
      the ADR and the CHANGELOG are qualified (decision 20).
    - The every-kind check now sees a missing kind or value (decision 17):
      a new variant with its dump arm but no `KINDS` entry fails
      `every_declared_form_is_listed` (`declared but not in KINDS:
      ["(Type.Never"]`), and so does a new `bool` field; without the three
      values in `every_other_node.nova` the every-kind test fails, `no
      file's dump by S0 holds ["(FunctionSig is_async=true",
      "(Pattern.Record rest=false", "(Type.Ptr is_mut=false"]`; mutants
      53 to 62 hold it.
    - The base's figures, measured again with its binary under `nova
      check`: parentheses pass at 230 and abort at 240, `if`s pass at 190
      and abort at 200, operators pass at 750 and abort at 800, method
      calls pass at 500 and abort at 600; stated so in the ADR and the
      CHANGELOG. The dump's two departures from the spec's sketch are
      marked (decision 15); the corpus is 586 files; Task 8, Step 6 says
      that `cargo check` compiles no test target.

**The second fact-check**

54. **Its findings, fixed in one prototype commit,** `5a68e7b9`, before
    the split, the replay and the mutants were redone. It found one code
    fault, in both parsers alike, and six faults in the records and tests:
    - **An abandoned attempt decided the depth** (low): the generic
      arguments' rollback restored `pos` and `pending_gt` but not
      `too_deep`, so the abandoned type attempt in `x as Int < (((a)))`,
      a level deeper than the comparison, made 127 parentheses P0002
      where the base parses them. Restoring the state alone, as first
      asked, broke `depth.rs`: a parameter's type past the limit,
      `Vec<Vec<…>>`, rolls its lists back one by one and became P0001s.
      So the crossing waits and stands only if the other reading fails
      first (decision 19), in `grammar.rs` and N's `parser.nova` and
      `parse_type.nova` alike. The other speculative reads were checked:
      the trait body's lookahead rolls back only before a `{`, which a
      crossing hides, and re-reads at the same depth; the `component`,
      `view` and item-start lookaheads read tokens alone. With S0's fix
      taken out,
      `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`
      fails (`parentheses: [TooDeep …]`, left `["P0002"]`, right `[]`);
      with either side's, `the_fixed_cases_agree` fails (`the dumps
      differ in …/abandoned_attempts_past_the_limit.nova, at line 1 of
      its dump`); seven mutants hold each part on each side.
    - `KINDS` has 167 forms (decision 17); ADR 0052, this plan and the
      first fix's report said 168.
    - The chain figures counted no call's arguments: decisions 8 and 28,
      the ADR's Consequences and the CHANGELOG now give the rule and the
      router's 127, each measured on both parsers.
    - The scanner passed over shapes silently: `pub struct G<T>`,
      `pub enum E<T>`, `where`, tuple structs, `pub(crate)`, private
      declarations, and a `bool` field after a `fn(Int) -> Int` field,
      whose `>` closed the body. It now fails on any `struct` or `enum` it
      does not read, and its test holds each shape: with the old scanner
      all nine fail (`read as ``, not `G{a:bool}``, …, `read as
      `F{f:fn(Int)-}``, `read as `N{A,=,1}``, not `fails`); three mutants
      hold the new parts.
    - The long literal's "207,500 pass" meant "had not aborted before the
      timeout": decision 20, the ADR, `11-PARSER.md` and the CHANGELOG
      state the threshold as measured.
    - P0001 and L0001 gained rows in the catalog (decision 9).
    - N's stack margin is Linux's; CI's Windows run is the check (decision
      22).

---
### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-51b`: from `4f32f13d`,
`main` after 5.1a, then this plan's commit, which adds this file alone, so
the patches apply to it as to `4f32f13d`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-51b`.
  - `git log --format='%h %s' -1 HEAD~1` prints
    `4f32f13d 5.1a: the review's fixes`.
  - `git diff --stat 4f32f13d HEAD` lists only
    `docs/superpowers/plans/2026-10-31-phase-5-1b-the-self-hosted-parser.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 8's review reads from it.
  - `df -h /` shows at least 6G free; if not, remove older duplicate test
    executables in `target/debug/deps` first. The replay began with
    11G free.
- [ ] **Step 2: The tools and the process limit.**
  - `clang-18 --version | head -n 1` prints
    `Ubuntu clang version 18.1.3 (1ubuntu1)`: CI's pin, for the golden
    suite.
  - `node --version` prints `v22.22.0`, for the WASM tests.
  - `RUSTUP_TOOLCHAIN=1.78.0 cargo --version` prints `cargo 1.78.0`, for
    Task 8.
  - The process limit, as the conventions set it: `id -u` says whether a
    pids control group is needed (`0`, root). Then the shell's membership:
    on cgroup v1, `grep nova-limit /proc/self/cgroup` prints
    `N:pids:/nova-limit` and `cat /sys/fs/cgroup/pids/nova-limit/pids.max`
    prints `600`; on cgroup v2, `cat /proc/self/cgroup` prints
    `0::/nova-limit` (or the systemd scope's path), and
    `cat "/sys/fs/cgroup$(sed -n 's/^0:://p' /proc/self/cgroup)/pids.max"`
    prints `600`; with `ulimit -Su 600` alone, `ulimit -Su` prints `600`.
    Record which way, and its output, in the ledger.
- [ ] **Step 3: The suite.** In a shell set up as the conventions say:

  ```sh
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-51b.txt" 2>&1
  grep '^test result' "$P/base-51b.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-51b.txt"
  cp target/debug/nova "$P/nova-base-51b"
  cargo test --locked --workspace -- --list > "$P/base-list-51b.txt" 2>&1
  grep -c ': test$' "$P/base-list-51b.txt"
  ```

  Expected, from 5.1a's last full run on `4f32f13d` and not the replay's:
  `2816 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. The replay built `4f32f13d` and listed its tests, and
  found `2827` names, the run's 2816 + 1 + 10. The copy of `nova`
  is the base's, for Task 8's comparisons; the list is the base's names.
  Task 8 expects `2839 1 10`: 5.1b's tests make 23 more, 23 added
  and 0 gone.
- [ ] **Step 4: The ledger.** Start `$P/ledger-51b.md`, headed with the
  base commit, the baseline's figures and the process limit's form.

### Task 1: P0002 and the depth count in S0

Spec §4.4, §5; decisions 2, 9, 18, 19 and 28 to 30.

**Files:**
- Modify: `crates/nova-parser/src/grammar.rs` (`depth`, `too_deep`,
  `deeper`, `nested`, the levels, the stop), `error.rs` (`TooDeep`,
  `code()`), `lib.rs` (`MAX_DEPTH`)
- Modify: `crates/nova-cli/src/cmd/parse.rs`,
  `crates/nova-driver/src/program.rs`, `crates/nova-resolver/src/lib.rs`,
  `crates/nova-fmt/src/source.rs` (each attaches `e.code()`),
  `crates/nova-lsp/src/organize.rs` (P0002 a syntax error)
- Create: `crates/nova-parser/tests/depth.rs`
- Test: `depth.rs` (seven), `crates/nova-cli/tests/fmt.rs` (one)

- [ ] **Step 1: The tests.** `patch_block t1-tests`. The patch holds:
  - `depth.rs`: 24 kinds of nesting, each a function from a depth to a
    source whose deepest level begins at the word `leaf`, parsed on a
    64 MB thread; `every_kind_of_nesting_parses_at_the_limit` (128, no
    error); `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`
    (129 between an item before and broken text after: one error,
    `TooDeep`, code P0002, its message, at the leaf, only the item before
    kept); `view_children_are_a_level`;
    `errors_before_the_limit_stay_and_there_is_one_p0002`;
    `the_depth_goes_back_down_after_each_sibling` (300 arguments nested
    100 deep, 300 chains of 100 operators, 300 postfix chains, an operand
    100 deep after a chain of 99 operators of each higher precedence, 50
    types and patterns 100 deep, two `else if` chains);
    `far_past_the_limit_it_stops_without_overflowing` (100,000 of eight
    kinds); `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`
    (a cast's type and `<` before 127 parentheses, a tuple of them and
    126 after two casts parse, an error later stays P0001; at 128 each is
    P0002 at its leaf; a generic cast type 200 deep and a `*mut` that no
    expression has are P0002);
  - `fmt.rs`: `a_file_nested_too_deeply_is_one_p0002_and_left_untouched`
    (300 parentheses, then a broken item: one error, P0002, the file
    unchanged).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which
  ends with `Finished`.
  - `cargo test --locked --workspace --test depth`. Expected: it does not
    compile, ``error[E0432]: unresolved import `nova_parser::MAX_DEPTH` ``, six
    `error[E0599]`s (no `TooDeep`, no `code()`) and
    ``error: could not compile `nova-parser` (test "depth") due to 7 previous errors`` (decision 47).
  - `cargo test --locked --workspace --test fmt -- a_file_nested_too_deeply`.
    Expected: `0 passed; 1 failed`: `nova fmt` aborts with
    `thread 'main' … has overflowed its stack` on 300 parentheses.
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`grammar.rs`:** `Parser::deeper`, its doc the definition of a
    level; `nested`; `parse_expr`, `parse_type`, `parse_pattern`, the
    prefix operands, `else if`, a nested item and a view's children
    through `nested`, the last three with a `…_here` function each; each
    binary function's depth saved, raised after each operator, restored
    at the chain's end; the cast's type by `parse_type_here`;
    `parse_postfix_from` saving and restoring the depth around
    `parse_postfix_chain`, which raises it after each operator but the
    first and a method call's `(`; `peek` reading the end of the file
    once past the limit; `parse_file` dropping the errors pushed since
    and pushing `TooDeep`; `roll_back`, the generic arguments' rollback,
    holding back a crossing in `rolled_back` until `advance` reads its
    token or `crossing_stands` finds an error first (decision 19).
  - **`error.rs`:** `ParseError::TooDeep { span }`, its message naming
    `MAX_DEPTH`, its span, and `code()`, through `Noted` to the error it
    holds.
  - **`lib.rs`:** `pub const MAX_DEPTH: usize = 128`.
  - **The callers:** `Diagnostic::error(e.code(), …)` in place of
    `"P0001"`, four places; organize-imports' syntax codes gain `P0002`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --test depth`. Expected:
    `7 passed; 0 failed`.
  - `cargo test --locked --workspace --test fmt -- a_file_nested_too_deeply`.
    Expected: `1 passed; 0 failed`.
  - `S` (6 min 16 s). Expected: `2824 1 10`: the base's figures and
    Task 1's 8 tests, the one failure the IPv6 test.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `parser: the depth limit, P0002`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/fmt.rs b/crates/nova-cli/tests/fmt.rs
index dadb6a66f8dd80f3d976bba1b4b701a73c3d3f6c..44cd8ba0f0b438ed3ee5057e4850bf19333f070e 100644
--- a/crates/nova-cli/tests/fmt.rs
+++ b/crates/nova-cli/tests/fmt.rs
@@ -165,6 +165,29 @@ fn a_file_with_a_syntax_error_is_left_untouched_and_the_rest_are_formatted() {
     nova().arg("fmt").arg("--check").arg(&dir).assert().code(2);
 }
 
+/// A file nested past the parser's limit is one error, P0002, and left as
+/// it is (spec
+/// `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+/// §6.2): no crash, and no error for what follows it.
+#[test]
+fn a_file_nested_too_deeply_is_one_p0002_and_left_untouched() {
+    let dir = fresh_dir("too-deep");
+    let deep = format!(
+        "fn main() {{\n    let x = {}1{}\n}}\nfn after( {{\n",
+        "(".repeat(300),
+        ")".repeat(300)
+    );
+    let file = write(&dir, "a.nova", &deep);
+    let out = nova().arg("fmt").arg(&file).assert().code(2);
+    let err = stderr(&out);
+    assert_eq!(err.matches("error[").count(), 1, "{err}");
+    assert!(
+        err.contains("error[P0002]: this is nested too deeply (more than 128 levels)"),
+        "{err}"
+    );
+    assert_eq!(read(&file), deep);
+}
+
 #[test]
 fn an_already_formatted_file_is_not_rewritten() {
     let dir = fresh_dir("unchanged");
diff --git a/crates/nova-parser/tests/depth.rs b/crates/nova-parser/tests/depth.rs
new file mode 100644
index 0000000000000000000000000000000000000000..fe4ca65ae9aba37bdba32bf639882f0e8b4ab0f7
--- /dev/null
+++ b/crates/nova-parser/tests/depth.rs
@@ -0,0 +1,415 @@
+//! The depth limit (spec
+//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+//! §4.4, ADR 0052): each kind of nesting parses at 128 levels, and at 129
+//! gives one P0002 at the token where the limit was crossed, after which
+//! the parser stops: no further error, and the items before it kept.
+//!
+//! Each input puts the token where its deepest level begins at the word
+//! `leaf` (`Leaf` for a type); the file's outermost expression, type or
+//! pattern is level 0 (`Parser::deeper`), so an input `d` levels deep is
+//! `d` nested parentheses, blocks, calls or other constructs of its kind. The parses run on a thread with
+//! a 64 MB stack, as S0's commands do, since a debug parser 128 levels
+//! deep needs more than a test thread's 2 MB.
+
+use nova_diagnostics::FileId;
+use nova_lexer::lex;
+use nova_parser::{parse, ParseError, MAX_DEPTH};
+
+/// `f` on a thread with a 64 MB stack.
+fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
+    std::thread::Builder::new()
+        .stack_size(64 << 20)
+        .spawn(f)
+        .expect("start a thread")
+        .join()
+        .expect("the thread ends")
+}
+
+/// `src`'s top-level item names and parse errors.
+fn parse_src(src: String) -> (Vec<String>, Vec<ParseError>) {
+    big_stack(move || {
+        let (tokens, lex_errors) = lex(&src, FileId::DUMMY);
+        assert!(lex_errors.is_empty(), "{lex_errors:?}");
+        let (file, errors) = parse(&tokens, FileId::DUMMY);
+        let names = file
+            .expect("a file")
+            .items
+            .iter()
+            .map(|i| match &i.value {
+                nova_ast::Item::Function(f) => f.name.value.clone(),
+                other => format!("{other:?}"),
+            })
+            .collect();
+        (names, errors)
+    })
+}
+
+/// `n` copies of `s`.
+fn rep(s: &str, n: usize) -> String {
+    s.repeat(n)
+}
+
+/// `fn f() { body }`.
+fn in_body(body: String) -> String {
+    format!("fn f() {{\n    {body}\n}}\n")
+}
+
+/// A kind of nesting: its name, and a function from the depth `d` (at
+/// least 2) to a file whose deepest level is `d`: `d` constructs of the
+/// kind, each inside the one before.
+type Kind = (&'static str, fn(usize) -> String);
+
+/// The kinds of nesting.
+fn kinds() -> Vec<Kind> {
+    vec![
+        ("parentheses", |d| {
+            in_body(format!("{}leaf{}", rep("(", d), rep(")", d)))
+        }),
+        ("blocks", |d| {
+            in_body(format!("{}leaf{}", rep("{ ", d), rep(" }", d)))
+        }),
+        ("calls", |d| {
+            in_body(format!("{}leaf{}", rep("g(", d), rep(")", d)))
+        }),
+        ("indexes", |d| {
+            in_body(format!("{}leaf{}", rep("a[", d), rep("]", d)))
+        }),
+        ("arrays", |d| {
+            in_body(format!("{}leaf{}", rep("[", d), rep("]", d)))
+        }),
+        ("record literals", |d| {
+            in_body(format!("{}leaf{}", rep("R { f: ", d), rep(" }", d)))
+        }),
+        ("interpolation holes", |d| {
+            in_body(format!("{}leaf{}", rep("\"${", d), rep("}\"", d)))
+        }),
+        ("closures", |d| in_body(format!("{}leaf", rep("|| ", d)))),
+        ("returns", |d| in_body(format!("{}leaf", rep("return ", d)))),
+        ("prefix operators", |d| {
+            // `&` takes a postfix expression, not another prefix one, so
+            // it comes last.
+            let ops = ["! ", "- ", "~ ", "* "];
+            let prefix: String = (0..d - 1).map(|i| ops[i % ops.len()]).collect();
+            in_body(format!("{prefix}&mut leaf"))
+        }),
+        ("a binary chain", |d| {
+            in_body(format!("{}leaf", rep("1 + ", d)))
+        }),
+        ("a postfix chain", |d| {
+            // The chain's first operator is not a level, and each after it
+            // one more: the `k`th field is `k - 1` levels deep.
+            in_body(format!("a{}.leaf", rep(".x", d)))
+        }),
+        ("method calls", |d| {
+            // A method call's `(…)` counts with its `.name`.
+            in_body(format!("a{}.leaf()", rep(".f()", d)))
+        }),
+        ("assignments", |d| {
+            in_body(format!("{}leaf", rep("a = ", d)))
+        }),
+        ("casts", |d| {
+            // The `k`th `as`, and its type, are level `k`.
+            in_body(format!("0{} as Leaf", rep(" as Int", d - 1)))
+        }),
+        ("ifs", |d| {
+            in_body(format!(
+                "{}if leaf {{ 0 }} else {{ 0 }}{}",
+                rep("if c { ", d - 1),
+                rep(" } else { 0 }", d - 1)
+            ))
+        }),
+        ("an else-if chain", |d| {
+            in_body(format!(
+                "if c {{ 0 }}{} else if leaf {{ 0 }} else {{ 0 }}",
+                rep(" else if c { 0 }", d - 2)
+            ))
+        }),
+        ("matches", |d| {
+            in_body(format!(
+                "{}match leaf {{ _ => 0 }}{}",
+                rep("match c { _ => ", d - 1),
+                rep(" }", d - 1)
+            ))
+        }),
+        ("whiles", |d| {
+            in_body(format!(
+                "{}while leaf {{ 0 }}{}",
+                rep("while c { ", d - 1),
+                rep(" }", d - 1)
+            ))
+        }),
+        ("fors", |d| {
+            in_body(format!(
+                "{}for leaf in c {{ 0 }}{}",
+                rep("for x in c { ", d - 1),
+                rep(" }", d - 1)
+            ))
+        }),
+        ("items in blocks", |d| {
+            in_body(format!("{}leaf{}", rep("fn g() { ", d), rep(" }", d)))
+        }),
+        ("types", |d| {
+            // Each wrapper one level: generic arguments (with `>>` glued),
+            // an array, a tuple, a reference, a pointer, a function type.
+            let wrappers: [(&str, &str); 6] = [
+                ("Vec<", ">"),
+                ("[", "]"),
+                ("(", ", Int)"),
+                ("&", ""),
+                ("*mut ", ""),
+                ("fn(", ") -> Int"),
+            ];
+            let (mut open, mut close) = (String::new(), String::new());
+            for i in 0..d {
+                let (o, c) = wrappers[i % wrappers.len()];
+                open.push_str(o);
+                close.insert_str(0, c);
+            }
+            format!("fn f(x: {open}Leaf{close}) {{}}\n")
+        }),
+        ("patterns", |d| {
+            let wrappers: [(&str, &str); 6] = [
+                ("S(", ")"),
+                ("(", ", 0)"),
+                ("[", "]"),
+                ("R { f: ", " }"),
+                ("m::S(", ")"),
+                ("a @ ", ""),
+            ];
+            let (mut open, mut close) = (String::new(), String::new());
+            for i in 0..d {
+                let (o, c) = wrappers[i % wrappers.len()];
+                open.push_str(o);
+                close.insert_str(0, c);
+            }
+            in_body(format!("let {open}leaf{close} = 0"))
+        }),
+        ("mixed expressions", |d| {
+            // Every wrapper is one level. A prefix operator binds tighter
+            // than `+`, so `1 + ` never comes straight inside one.
+            let wrappers: [(&str, &str); 11] = [
+                ("(", ")"),
+                ("{ ", " }"),
+                ("g(", ")"),
+                ("a[", "]"),
+                ("[", "]"),
+                ("!", ""),
+                ("1 + ", ""),
+                ("|| ", ""),
+                ("if c { ", " } else { 0 }"),
+                ("match c { _ => ", " }"),
+                ("R { f: ", " }"),
+            ];
+            let (mut open, mut close) = (String::new(), String::new());
+            let mut state = 7usize;
+            let mut last = 0;
+            for _ in 0..d - 1 {
+                state = state
+                    .wrapping_mul(6364136223846793005)
+                    .wrapping_add(1442695040888963407);
+                let mut w = (state >> 33) % wrappers.len();
+                if last == 5 && w == 6 {
+                    w = 0;
+                }
+                last = w;
+                let (o, c) = wrappers[w];
+                open.push_str(o);
+                close.insert_str(0, c);
+            }
+            in_body(format!("{open}(leaf){close}"))
+        }),
+    ]
+}
+
+#[test]
+fn every_kind_of_nesting_parses_at_the_limit() {
+    for (kind, src) in kinds() {
+        let src = src(MAX_DEPTH);
+        let (names, errors) = parse_src(src.clone());
+        assert!(
+            errors.is_empty(),
+            "{kind} at {MAX_DEPTH}: {errors:?}\n{src}"
+        );
+        assert_eq!(names, ["f"], "{kind}");
+    }
+}
+
+#[test]
+fn every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token() {
+    for (kind, src) in kinds() {
+        let deep = src(MAX_DEPTH + 1);
+        let leaf = deep
+            .find("leaf")
+            .or_else(|| deep.find("Leaf"))
+            .expect("a leaf") as u32;
+        // The items before it are kept; nothing after it is read, an error
+        // included.
+        let src = format!("fn before() {{}}\n{deep}fn after( {{\n) ]\n");
+        let leaf = leaf + "fn before() {}\n".len() as u32;
+        let (names, errors) = parse_src(src.clone());
+        assert_eq!(errors.len(), 1, "{kind}: {errors:?}");
+        let error = &errors[0];
+        assert!(
+            matches!(error, ParseError::TooDeep { .. }),
+            "{kind}: {error:?}"
+        );
+        assert_eq!(error.code(), "P0002");
+        assert_eq!(
+            error.to_string(),
+            "this is nested too deeply (more than 128 levels)"
+        );
+        assert_eq!(
+            (error.span().start, error.span().end),
+            (leaf, leaf + 4),
+            "{kind}: {}",
+            &src[error.span().start as usize..]
+        );
+        assert_eq!(names, ["before"], "{kind}");
+    }
+}
+
+/// An element's or a view's children are a level, and its child
+/// expressions are not: past the limit, the `{` that opens the children.
+#[test]
+fn view_children_are_a_level() {
+    let view = |d: usize| {
+        // The view's children are level 1, each element's one deeper.
+        in_body(format!(
+            "view {{ {}x{} }}",
+            rep("div { ", d - 1),
+            rep(" }", d - 1)
+        ))
+    };
+    let (_, errors) = parse_src(view(MAX_DEPTH));
+    assert!(errors.is_empty(), "{errors:?}");
+    let src = view(MAX_DEPTH + 1);
+    let last = src.rfind("div {").expect("a div") + "div ".len();
+    let (_, errors) = parse_src(src);
+    assert_eq!(errors.len(), 1, "{errors:?}");
+    assert_eq!(errors[0].code(), "P0002");
+    assert_eq!(errors[0].span().start as usize, last);
+}
+
+/// An error before the limit is crossed stays, before the P0002; a file
+/// nested past it twice has one P0002, at the first.
+#[test]
+fn errors_before_the_limit_stay_and_there_is_one_p0002() {
+    let deep = format!("{}1{}", rep("(", MAX_DEPTH + 1), rep(")", MAX_DEPTH + 1));
+    let src = format!("fn a() {{ let = 1; }}\nfn b() {{ {deep} }}\nfn c() {{ {deep} }}\n");
+    let (names, errors) = parse_src(src);
+    let codes: Vec<&str> = errors.iter().map(|e| e.code()).collect();
+    assert_eq!(codes, ["P0001", "P0002"], "{errors:?}");
+    assert_eq!(names, ["a"]);
+}
+
+/// The depth goes back down after each operand, operator chain, postfix
+/// chain and sub-pattern: many siblings, each nested deep, are fine, and so
+/// is a chain of operators in each of many parentheses, and an operand 100
+/// deep after a chain of 99 operators of a higher precedence.
+#[test]
+fn the_depth_goes_back_down_after_each_sibling() {
+    let nest = format!("{}1{}", rep("(", 100), rep(")", 100));
+    let args = vec![nest; 300].join(", ");
+    let chain = format!("({});", rep("1 + ", 100) + "1");
+    let chains = vec![chain; 300].join(" ");
+    let calls = vec![format!("a{}", rep(".f(x)[0]?", 40)); 300].join("\n    ");
+    // A chain of each binary precedence, and a postfix chain, then an
+    // operator of the next lower one whose operand is 100 deep; and a view
+    // whose first child is a postfix chain, its second's argument 100 deep.
+    let deep = format!("{}1{}", rep("(", 100), rep(")", 100));
+    let lowers = [
+        (" as Int", " * "),
+        (" * 1", " + "),
+        (" + 1", " << "),
+        (" << 1", " & "),
+        (" & 1", " ^ "),
+        (" ^ 1", " | "),
+        (" | 1", " < "),
+        (" && x", " || "),
+        (" || x", " .. "),
+        (".x", " * "),
+    ]
+    .map(|(op, lower)| format!("x{}{lower}{deep}", rep(op, 99)))
+    .join("\n    ")
+        + &format!("\n    view {{ a{} g({deep}) }}", rep(".x", 99));
+    let types = vec![format!("{}Int{}", rep("Vec<", 100), rep(">", 100)); 50].join(", ");
+    let pats = vec![format!("{}x{}", rep("S(", 100), rep(")", 100)); 50].join(" | ");
+    let src = format!(
+        "fn f(t: ({types})) {{\n    g({args})\n    {chains}\n    {calls}\n    {lowers}\n    match t {{ {pats} => 0 }}\n    \
+         if a {{ 1 }} else if b {{ 2 }} else {{ 3 }}\n    if a {{ 1 }} else if b {{ 2 }} else {{ 3 }}\n}}\n"
+    );
+    let (names, errors) = parse_src(src);
+    assert!(errors.is_empty(), "{errors:?}");
+    assert_eq!(names, ["f"]);
+}
+
+/// An abandoned attempt does not decide the depth alone (ADR 0052). The
+/// type after a cast and `<` is first tried as generic arguments, a level
+/// deeper than the expression the tokens then are, and after two casts two
+/// levels: 127 and 126 parentheses take that attempt past the limit, and
+/// its rollback takes the crossing back, with any error pushed after it
+/// (the tuple type's missing `)`), so the expression parses, as the parser
+/// before the limit read it, and an error later in the file is its own. At
+/// 128 the expression itself is too deep: P0002 at its own deepest token.
+/// The crossing stands if the tokens, read the other way, fail before its
+/// token: a cast's type past the limit, read again as comparisons, and a
+/// type `*mut`, which no expression has.
+#[test]
+fn a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way() {
+    let paren = |n: usize| format!("x as Int < {}leaf{}", rep("(", n), rep(")", n));
+    let tuple = |n: usize| format!("x as Int < (b, {}leaf{})", rep("(", n - 1), rep(")", n - 1));
+    let two = |n: usize| format!("x as A as B < {}leaf{}", rep("(", n), rep(")", n));
+    let later = format!(
+        "{}\n}}\nfn g() {{ let = 1 }}\nfn h() {{",
+        paren(MAX_DEPTH - 1)
+    );
+    let generic = format!("x as {}Leaf{}", rep("V<", 200), rep(">", 200));
+    let ptr = format!("x as Int < (*mut {}leaf{})", rep("(", 126), rep(")", 126));
+    for (shape, body, codes) in [
+        ("parentheses", paren(MAX_DEPTH - 1), vec![]),
+        ("a tuple", tuple(MAX_DEPTH - 1), vec![]),
+        ("two casts", two(MAX_DEPTH - 2), vec![]),
+        ("an error after", later, vec!["P0001"]),
+        ("parentheses", paren(MAX_DEPTH), vec!["P0002"]),
+        ("a tuple", tuple(MAX_DEPTH), vec!["P0002"]),
+        ("two casts", two(MAX_DEPTH), vec!["P0002"]),
+        ("a generic cast type", generic, vec!["P0002"]),
+        ("a pointer type", ptr, vec!["P0002"]),
+    ] {
+        let src = in_body(body);
+        let (_, errors) = parse_src(src.clone());
+        let found: Vec<&str> = errors.iter().map(|e| e.code()).collect();
+        assert_eq!(found, codes, "{shape}: {errors:?}");
+        if codes == ["P0002"] {
+            let leaf = src.find("leaf").or(src.find("Leaf")).expect("a leaf");
+            let at = errors[0].span().start as usize;
+            assert!(
+                at <= leaf,
+                "{shape}: P0002 at {at}, past the leaf at {leaf}"
+            );
+            if shape == "parentheses" || shape == "a tuple" {
+                assert_eq!(at, leaf, "{shape}");
+            }
+        }
+    }
+}
+
+/// Far past the limit, as a fuzzer could write, the parser stops at 129
+/// and never overflows its stack.
+#[test]
+fn far_past_the_limit_it_stops_without_overflowing() {
+    for src in [
+        in_body(format!("{}1{}", rep("(", 100_000), rep(")", 100_000))),
+        in_body(rep("{ ", 100_000)),
+        in_body(rep("!", 100_000) + "x"),
+        in_body(rep("1 + ", 100_000) + "1"),
+        in_body(format!("a{}", rep(".f()", 100_000))),
+        in_body(rep("if c { ", 50_000)),
+        format!("fn f(x: {}Int) {{}}\n", rep("Vec<", 100_000)),
+        in_body(format!("let {}x = 0", rep("S(", 100_000))),
+    ] {
+        let (_, errors) = parse_src(src);
+        let codes: Vec<&str> = errors.iter().map(|e| e.code()).collect();
+        assert_eq!(codes, ["P0002"]);
+    }
+}
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-cli/src/cmd/parse.rs b/crates/nova-cli/src/cmd/parse.rs
index cc31f8175e7649f1522fc791480d300c94fd3cc6..2119bd14ab0ce24ad8bb0779b4421e984b78ac61 100644
--- a/crates/nova-cli/src/cmd/parse.rs
+++ b/crates/nova-cli/src/cmd/parse.rs
@@ -102,7 +102,7 @@ pub fn run(cmd: ParseCmd) -> Result<()> {
         .iter()
         .map(|e| {
             e.notes().into_iter().fold(
-                nova_diagnostics::Diagnostic::error("P0001", e.to_string())
+                nova_diagnostics::Diagnostic::error(e.code(), e.to_string())
                     .with_primary_label(e.span(), "here"),
                 |d, n| d.with_note(n),
             )
diff --git a/crates/nova-driver/src/program.rs b/crates/nova-driver/src/program.rs
index 5e52df32ae421a0321df93b13b989772ea2335af..308fffe63173bd5d3b3efc7ef4e24faf5b827753 100644
--- a/crates/nova-driver/src/program.rs
+++ b/crates/nova-driver/src/program.rs
@@ -431,7 +431,7 @@ pub(crate) fn load_program(
         let parsed = nova_parser::parse_recovering(&tokens, file);
         load.diagnostics.extend(parsed.errors.iter().map(|e| {
             e.notes().into_iter().fold(
-                Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here"),
+                Diagnostic::error(e.code(), e.to_string()).with_primary_label(e.span(), "here"),
                 |d, n| d.with_note(n),
             )
         }));
diff --git a/crates/nova-fmt/src/source.rs b/crates/nova-fmt/src/source.rs
index d3879a17a9d4762ad610c931db27e28161a895ae..52901b6015e7a4696aeb1a058ba36741fddf0bdb 100644
--- a/crates/nova-fmt/src/source.rs
+++ b/crates/nova-fmt/src/source.rs
@@ -33,7 +33,7 @@ impl<'t> Source<'t> {
             .collect();
         diagnostics.extend(parse_errors.iter().map(|e| {
             e.notes().into_iter().fold(
-                Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here"),
+                Diagnostic::error(e.code(), e.to_string()).with_primary_label(e.span(), "here"),
                 |d, n| d.with_note(n),
             )
         }));
diff --git a/crates/nova-lsp/src/organize.rs b/crates/nova-lsp/src/organize.rs
index a5afec1895c0fc0d72ec4f3eb5992845af563bfe..4fb5a35d84dcc6f45cdcefd490557e32823dcb43 100644
--- a/crates/nova-lsp/src/organize.rs
+++ b/crates/nova-lsp/src/organize.rs
@@ -58,7 +58,7 @@ pub fn action(answer: &Answer, uri_of: &dyn Fn(&Path) -> String) -> Option<lsp::
 /// E0001s a dropped item causes.
 fn may_remove(a: &Analysis, file: FileId) -> bool {
     !a.diagnostics.iter().any(|d| {
-        matches!(d.code.as_str(), "L0001" | "P0001")
+        matches!(d.code.as_str(), "L0001" | "P0001" | "P0002")
             || (d.severity == Severity::Error && d.labels.iter().any(|l| l.span.file == file))
     })
 }
diff --git a/crates/nova-parser/src/error.rs b/crates/nova-parser/src/error.rs
index 71150de482d350b2c13b8b70c1c591ab641bf697..8159f6598e0e451024621cd1d301bfcb033f7651 100644
--- a/crates/nova-parser/src/error.rs
+++ b/crates/nova-parser/src/error.rs
@@ -22,6 +22,13 @@ pub enum ParseError {
     #[error("{message}")]
     Custom { message: String, span: Span },
 
+    /// More than [`MAX_DEPTH`](crate::MAX_DEPTH) levels of nesting, at the
+    /// token where the limit was crossed: P0002 (spec
+    /// `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+    /// §4.4). The parser stops there, so it is the file's last error.
+    #[error("this is nested too deeply (more than {} levels)", crate::MAX_DEPTH)]
+    TooDeep { span: Span },
+
     /// `error`, with a note that says why it may have come about.
     #[error("{error}")]
     Noted {
@@ -37,10 +44,21 @@ impl ParseError {
             ParseError::UnexpectedEof { span } => *span,
             ParseError::ChainedComparison { span } => *span,
             ParseError::Custom { span, .. } => *span,
+            ParseError::TooDeep { span } => *span,
             ParseError::Noted { error, .. } => error.span(),
         }
     }
 
+    /// The code a caller gives the error's diagnostic: P0002 for
+    /// [`ParseError::TooDeep`], P0001 for every other.
+    pub fn code(&self) -> &'static str {
+        match self {
+            ParseError::TooDeep { .. } => "P0002",
+            ParseError::Noted { error, .. } => error.code(),
+            _ => "P0001",
+        }
+    }
+
     /// The notes that go with the error's diagnostic.
     pub fn notes(&self) -> Vec<String> {
         match self {
diff --git a/crates/nova-parser/src/grammar.rs b/crates/nova-parser/src/grammar.rs
index fecc344831f0e9ed34567633d01810072ca3d531..526fc99a3eb3776d4c4190fee5eeacd21d378fe2 100644
--- a/crates/nova-parser/src/grammar.rs
+++ b/crates/nova-parser/src/grammar.rs
@@ -20,7 +20,7 @@ use nova_ast::{
 use nova_diagnostics::{FileId, Span, Spanned};
 use nova_lexer::{Token, Tokens};
 
-use crate::ParseError;
+use crate::{ParseError, MAX_DEPTH};
 
 /// The error for a `///` that documents nothing (spec
 /// `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md` §4).
@@ -69,6 +69,21 @@ struct Parser<'a> {
     /// The split that ended the last match arm's body: a P0001 in the head
     /// of the arm that begins there gets a note naming the rule.
     arm_end_split: Option<NewlineSplit>,
+    /// How many levels have been entered at the cursor, the outermost
+    /// included: one more than the cursor's level ([`Parser::deeper`]).
+    depth: usize,
+    /// Where the depth first passed [`MAX_DEPTH`]: how many errors there
+    /// were then, and the span of the token at the cursor. From then on
+    /// the parser reads the end of the file (`peek`), and when the file
+    /// ends, P0002 at that span replaces every error pushed since.
+    too_deep: Option<(usize, Span)>,
+    /// A crossing that an abandoned attempt made ([`Parser::roll_back`]):
+    /// how many errors there were then, its span, and the token where it
+    /// was made. The parser reads those tokens again another way, and the
+    /// crossing stands, as `too_deep` ([`Parser::crossing_stands`]), if
+    /// an error comes before that token is read; once it is read, the
+    /// crossing goes.
+    rolled_back: Option<(usize, Span, usize)>,
 }
 
 impl<'a> Parser<'a> {
@@ -85,9 +100,68 @@ impl<'a> Parser<'a> {
             splits: Vec::new(),
             halt: None,
             arm_end_split: None,
+            depth: 0,
+            too_deep: None,
+            rolled_back: None,
         }
     }
 
+    /// One level deeper (spec
+    /// `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+    /// §4.4, ADR 0052). Whether the parser may go on: past [`MAX_DEPTH`]
+    /// levels it may not, and the first time, it records P0002 at the
+    /// token at the cursor (`too_deep`). A level is:
+    ///
+    /// - an expression ([`Parser::parse_expr`]), a type
+    ///   ([`Parser::parse_type`]) or a pattern ([`Parser::parse_pattern`]),
+    ///   so each operand, type argument or sub-pattern inside another;
+    /// - the operand of a prefix operator, and each `else if`;
+    /// - each binary operator, `as` and assignment included, for the rest
+    ///   of its chain: the right operand of a chain's `k`th operator is `k`
+    ///   levels deeper than the chain, and so is the type after its `k`th
+    ///   `as`;
+    /// - an item inside a block, and the children of a view or an element.
+    ///
+    /// The file's outermost expression, type, pattern or nested item is
+    /// level 0, so `n` nested parentheses, blocks, calls, `if`s or types
+    /// are `n` levels, and 128 of any one kind parse: `depth`, which counts
+    /// every entry, the outermost's included, is one more than the level.
+    /// A block is not a level of its own: its statements' expressions are.
+    /// Each of these is entered through [`Parser::nested`], which restores
+    /// the depth after it, but for the binary operators, whose function
+    /// restores it when the chain ends.
+    fn deeper(&mut self) -> bool {
+        self.depth += 1;
+        if self.depth <= MAX_DEPTH + 1 {
+            return true;
+        }
+        if self.too_deep.is_none() {
+            let here = (self.errors.len(), self.peek_span());
+            self.too_deep = Some(self.crossing_stands().unwrap_or(here));
+        }
+        false
+    }
+
+    /// The crossing an abandoned attempt made (`rolled_back`), if an error
+    /// has come since: the tokens read again the other way failed before
+    /// the token where it was made, so it stands, as the parser's own
+    /// crossing does, and the parser reads the end of the file (`peek`).
+    fn crossing_stands(&self) -> Option<(usize, Span)> {
+        match self.rolled_back {
+            Some((errors, span, _)) if self.errors.len() > errors => Some((errors, span)),
+            _ => None,
+        }
+    }
+
+    /// `f`, one level deeper ([`Parser::deeper`]), with the depth restored
+    /// after it however it ends; `None`, without `f`, past the limit.
+    fn nested<R>(&mut self, f: impl FnOnce(&mut Self) -> Option<R>) -> Option<R> {
+        let depth = self.depth;
+        let result = if self.deeper() { f(self) } else { None };
+        self.depth = depth;
+        result
+    }
+
     /// Run `f` with `no_struct_literal = true`, then restore the previous value.
     fn in_no_struct_ctx<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
         let prev = self.no_struct_literal;
@@ -221,7 +295,8 @@ impl<'a> Parser<'a> {
     // --- Token access ---
 
     fn peek(&self) -> &Token {
-        if self.halted() {
+        // Past the depth limit, the file has ended.
+        if self.halted() || self.too_deep.is_some() || self.crossing_stands().is_some() {
             return &HALT;
         }
         self.tokens
@@ -238,6 +313,16 @@ impl<'a> Parser<'a> {
     }
 
     fn advance(&mut self) -> Spanned<Token> {
+        // A crossing an abandoned attempt made stands if an error came
+        // first, and goes once its token is read.
+        if let Some((_, _, at)) = self.rolled_back {
+            if let Some(crossing) = self.crossing_stands() {
+                self.too_deep.get_or_insert(crossing);
+                self.rolled_back = None;
+            } else if self.pos >= at {
+                self.rolled_back = None;
+            }
+        }
         let tok = self
             .tokens
             .get(self.pos)
@@ -520,6 +605,12 @@ impl<'a> Parser<'a> {
                 }
             }
         }
+        // Past the depth limit: one error, P0002, for all that came after
+        // (spec §4.4).
+        if let Some((errors, span)) = self.too_deep.or(self.crossing_stands()) {
+            self.errors.truncate(errors);
+            self.errors.push(ParseError::TooDeep { span });
+        }
         let mut newline_splits = std::mem::take(&mut self.splits);
         newline_splits.sort_by_key(|s| s.at.start);
         File {
@@ -1337,6 +1428,13 @@ impl<'a> Parser<'a> {
 
 impl<'a> Parser<'a> {
     fn parse_type(&mut self, ctx: &str) -> Option<Spanned<Type>> {
+        self.nested(|p| p.parse_type_here(ctx))
+    }
+
+    /// A type at the cursor's level, which is not one more: the type after
+    /// `as`, which its operator's level counts, as a binary operator's
+    /// counts its right operand ([`Parser::deeper`]).
+    fn parse_type_here(&mut self, ctx: &str) -> Option<Spanned<Type>> {
         let start = self.peek_span();
         let mut ty = self.parse_type_atom(ctx)?;
 
@@ -1508,6 +1606,23 @@ impl<'a> Parser<'a> {
         }
     }
 
+    /// Undo a speculative parse that began at `pos`, with `pending_gt`
+    /// pending `>`s and the P0002 state `too_deep`. The errors it pushed
+    /// stand, as the parser before the depth limit left them, but for
+    /// those after it crossed the limit, which it read at the end of the
+    /// file. Its crossing does not decide alone (ADR 0052): it waits in
+    /// `rolled_back`, at the token at the cursor, where it was made, for
+    /// the tokens to be read again the other way.
+    fn roll_back(&mut self, pos: usize, pending_gt: usize, too_deep: Option<(usize, Span)>) {
+        if let (None, Some((errors, span))) = (too_deep, self.too_deep) {
+            self.errors.truncate(errors);
+            self.rolled_back = Some((errors, span, self.pos));
+        }
+        self.pos = pos;
+        self.pending_gt = pending_gt;
+        self.too_deep = too_deep;
+    }
+
     fn parse_generic_args_opt(&mut self, ctx: &str) -> Vec<Spanned<Type>> {
         if !self.check(&Token::Lt) {
             return Vec::new();
@@ -1516,6 +1631,7 @@ impl<'a> Parser<'a> {
         // Otherwise this `<` might be a comparison operator.
         let saved = self.pos;
         let saved_pending = self.pending_gt;
+        let saved_deep = self.too_deep;
         self.advance(); // consume `<`
         let mut args = Vec::new();
         while !self.at_generic_close() && !self.is_at_end() {
@@ -1523,8 +1639,7 @@ impl<'a> Parser<'a> {
                 args.push(t);
             } else {
                 // Wasn't a type — roll back
-                self.pos = saved;
-                self.pending_gt = saved_pending;
+                self.roll_back(saved, saved_pending, saved_deep);
                 return Vec::new();
             }
             // A `>` split from a `>>` now closes this list; a following comma (if
@@ -1538,8 +1653,7 @@ impl<'a> Parser<'a> {
         }
         if !self.eat_generic_close() {
             // Didn't close — roll back
-            self.pos = saved;
-            self.pending_gt = saved_pending;
+            self.roll_back(saved, saved_pending, saved_deep);
             return Vec::new();
         }
         args
@@ -1621,7 +1735,7 @@ impl<'a> Parser<'a> {
 
         // Nested items, documented or not (spec §4)
         if self.is_item_start_at(self.docs_end()) {
-            let item = self.in_stmt_ctx(false, |p| p.try_parse_item())?;
+            let item = self.nested(|p| p.in_stmt_ctx(false, |p| p.try_parse_item()))?;
             let span = item.span;
             return Some(Spanned::new(Stmt::Item(Box::new(item.value)), span));
         }
@@ -1679,7 +1793,7 @@ impl<'a> Parser<'a> {
 
 impl<'a> Parser<'a> {
     fn parse_expr(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let expr = self.parse_assign(ctx);
+        let expr = self.nested(|p| p.parse_assign(ctx));
         self.end_halt();
         expr
     }
@@ -1693,7 +1807,7 @@ impl<'a> Parser<'a> {
     }
 
     fn parse_assign(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let lhs = self.parse_range(ctx)?;
 
         let op = match self.peek() {
@@ -1713,7 +1827,11 @@ impl<'a> Parser<'a> {
 
         if let Some(op) = op {
             self.advance();
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_assign(ctx)?; // right-associative
+            self.depth = depth;
             let span = start.merge(rhs.span);
             Some(Spanned::new(
                 Expr::Assign {
@@ -1731,7 +1849,7 @@ impl<'a> Parser<'a> {
     /// Range expressions `lo..hi` / `lo..=hi`, just above assignment in
     /// precedence and non-chainable.
     fn parse_range(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let lo = self.parse_or(ctx)?;
         let inclusive = if self.eat(&Token::DotDotEq).is_some() {
             true
@@ -1740,7 +1858,11 @@ impl<'a> Parser<'a> {
         } else {
             return Some(lo);
         };
+        if !self.deeper() {
+            return None;
+        }
         let hi = self.parse_or(ctx)?;
+        self.depth = depth;
         let span = start.merge(hi.span);
         Some(Spanned::new(
             Expr::Range {
@@ -1753,9 +1875,12 @@ impl<'a> Parser<'a> {
     }
 
     fn parse_or(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let mut lhs = self.parse_and(ctx)?;
         while self.eat(&Token::PipePipe).is_some() {
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_and(ctx)?;
             let span = start.merge(rhs.span);
             lhs = Spanned::new(
@@ -1767,13 +1892,17 @@ impl<'a> Parser<'a> {
                 span,
             );
         }
+        self.depth = depth;
         Some(lhs)
     }
 
     fn parse_and(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let mut lhs = self.parse_compare(ctx)?;
         while self.eat(&Token::AmpAmp).is_some() {
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_compare(ctx)?;
             let span = start.merge(rhs.span);
             lhs = Spanned::new(
@@ -1785,11 +1914,12 @@ impl<'a> Parser<'a> {
                 span,
             );
         }
+        self.depth = depth;
         Some(lhs)
     }
 
     fn parse_compare(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let lhs = self.parse_bitor(ctx)?;
 
         let op = match self.peek() {
@@ -1804,7 +1934,11 @@ impl<'a> Parser<'a> {
 
         if let Some(op) = op {
             self.advance();
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_bitor(ctx)?;
+            self.depth = depth;
             let span = start.merge(rhs.span);
             let result = Spanned::new(
                 Expr::Binary {
@@ -1830,9 +1964,12 @@ impl<'a> Parser<'a> {
     }
 
     fn parse_bitor(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let mut lhs = self.parse_bitxor(ctx)?;
         while self.eat(&Token::Pipe).is_some() {
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_bitxor(ctx)?;
             let span = start.merge(rhs.span);
             lhs = Spanned::new(
@@ -1844,13 +1981,17 @@ impl<'a> Parser<'a> {
                 span,
             );
         }
+        self.depth = depth;
         Some(lhs)
     }
 
     fn parse_bitxor(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let mut lhs = self.parse_bitand(ctx)?;
         while self.eat(&Token::Caret).is_some() {
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_bitand(ctx)?;
             let span = start.merge(rhs.span);
             lhs = Spanned::new(
@@ -1862,13 +2003,17 @@ impl<'a> Parser<'a> {
                 span,
             );
         }
+        self.depth = depth;
         Some(lhs)
     }
 
     fn parse_bitand(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let mut lhs = self.parse_shift(ctx)?;
         while self.eat(&Token::Amp).is_some() {
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_shift(ctx)?;
             let span = start.merge(rhs.span);
             lhs = Spanned::new(
@@ -1880,11 +2025,12 @@ impl<'a> Parser<'a> {
                 span,
             );
         }
+        self.depth = depth;
         Some(lhs)
     }
 
     fn parse_shift(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let mut lhs = self.parse_add(ctx)?;
         loop {
             let op = match self.peek() {
@@ -1893,6 +2039,9 @@ impl<'a> Parser<'a> {
                 _ => break,
             };
             self.advance();
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_add(ctx)?;
             let span = start.merge(rhs.span);
             lhs = Spanned::new(
@@ -1904,11 +2053,12 @@ impl<'a> Parser<'a> {
                 span,
             );
         }
+        self.depth = depth;
         Some(lhs)
     }
 
     fn parse_add(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let mut lhs = self.parse_mul(ctx)?;
         loop {
             let op = match self.peek() {
@@ -1923,6 +2073,9 @@ impl<'a> Parser<'a> {
                 _ => break,
             };
             self.advance();
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_mul(ctx)?;
             let span = start.merge(rhs.span);
             lhs = Spanned::new(
@@ -1934,11 +2087,12 @@ impl<'a> Parser<'a> {
                 span,
             );
         }
+        self.depth = depth;
         Some(lhs)
     }
 
     fn parse_mul(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let mut lhs = self.parse_cast(ctx)?;
         loop {
             let op = match self.peek() {
@@ -1948,6 +2102,9 @@ impl<'a> Parser<'a> {
                 _ => break,
             };
             self.advance();
+            if !self.deeper() {
+                return None;
+            }
             let rhs = self.parse_cast(ctx)?;
             let span = start.merge(rhs.span);
             lhs = Spanned::new(
@@ -1959,15 +2116,19 @@ impl<'a> Parser<'a> {
                 span,
             );
         }
+        self.depth = depth;
         Some(lhs)
     }
 
     fn parse_cast(&mut self, ctx: &str) -> Option<Spanned<Expr>> {
-        let start = self.peek_span();
+        let (start, depth) = (self.peek_span(), self.depth);
         let mut expr = self.parse_unary(ctx)?;
         loop {
             if self.eat(&Token::As).is_some() {
-                let ty = self.parse_type(ctx)?;
+                if !self.deeper() {
+                    return None;
+                }
+                let ty = self.parse_type_here(ctx)?;
                 let span = start.merge(ty.span);
                 expr = Spanned::new(
                     Expr::Cast {
@@ -1980,6 +2141,7 @@ impl<'a> Parser<'a> {
                 break;
             }
         }
+        self.depth = depth;
         Some(expr)
     }
 
@@ -1994,7 +2156,7 @@ impl<'a> Parser<'a> {
                 // Could be `&` or `&mut`
                 self.advance();
                 let is_mut = self.eat(&Token::Mut).is_some();
-                let expr = self.parse_postfix(ctx)?;
+                let expr = self.nested(|p| p.parse_postfix(ctx))?;
                 let span = start.merge(expr.span);
                 let op = if is_mut { UnOp::RefMut } else { UnOp::Ref };
                 return Some(Spanned::new(
@@ -2010,7 +2172,7 @@ impl<'a> Parser<'a> {
 
         if let Some(op) = op {
             self.advance();
-            let expr = self.parse_unary(ctx)?;
+            let expr = self.nested(|p| p.parse_unary(ctx))?;
             let span = start.merge(expr.span);
             Some(Spanned::new(
                 Expr::Unary {
@@ -2031,16 +2193,35 @@ impl<'a> Parser<'a> {
     }
 
     /// The calls, indexes, fields, `.await`s and `?`s after `expr`, which
-    /// began at `start`.
+    /// began at `start`: each after the first one level deeper than the one
+    /// before it, for the rest of the chain, a method call `.name(…)` one
+    /// operator ([`Parser::deeper`]); the depth restored after the chain
+    /// however it ends.
     fn parse_postfix_from(
+        &mut self,
+        start: Span,
+        expr: Spanned<Expr>,
+        ctx: &str,
+    ) -> Option<Spanned<Expr>> {
+        let depth = self.depth;
+        let expr = self.parse_postfix_chain(start, expr, ctx);
+        self.depth = depth;
+        expr
+    }
+
+    /// [`Parser::parse_postfix_from`]'s operators, the depth raised after
+    /// each but the first, as after a binary operator: past the limit, at
+    /// the token after it.
+    fn parse_postfix_chain(
         &mut self,
         start: Span,
         mut expr: Spanned<Expr>,
         ctx: &str,
     ) -> Option<Spanned<Expr>> {
+        let (mut first, mut after_dot) = (true, false);
         loop {
             let cur_span = expr.span;
-            match self.peek().clone() {
+            let op = match self.peek() {
                 // A `(` or `[` after a line break in statement context
                 // begins the next statement (spec §3.5).
                 Token::LParen | Token::LBracket if self.newline_ends_expr() => {
@@ -2052,9 +2233,20 @@ impl<'a> Parser<'a> {
                     self.record_split(kind);
                     break;
                 }
+                op @ (Token::LParen | Token::LBracket | Token::Dot | Token::Question) => op.clone(),
+                _ => break,
+            };
+            self.advance();
+            // A method call's `(…)` counts with its `.name`.
+            let method_call = after_dot && matches!(op, Token::LParen);
+            after_dot = matches!(op, Token::Dot);
+            if !first && !method_call && !self.deeper() {
+                return None;
+            }
+            first = false;
+            match op {
                 Token::LParen => {
                     // Function call
-                    self.advance();
                     let args = self.in_stmt_ctx(false, |p| {
                         let mut args = Vec::new();
                         while !p.check(&Token::RParen) && !p.is_at_end() {
@@ -2083,7 +2275,6 @@ impl<'a> Parser<'a> {
                 }
                 Token::LBracket => {
                     // Index
-                    self.advance();
                     let index = self.in_stmt_ctx(false, |p| p.parse_expr(ctx))?;
                     self.expect(&Token::RBracket, "index expression")?;
                     let end = self
@@ -2100,7 +2291,6 @@ impl<'a> Parser<'a> {
                     );
                 }
                 Token::Dot => {
-                    self.advance();
                     // `.await` or `.field` or `.method()`
                     if self.eat(&Token::Await).is_some() {
                         let end = self
@@ -2167,8 +2357,8 @@ impl<'a> Parser<'a> {
                         );
                     }
                 }
-                Token::Question => {
-                    self.advance();
+                _ => {
+                    // `?`
                     let end = self
                         .tokens
                         .get(self.pos.saturating_sub(1))
@@ -2176,7 +2366,6 @@ impl<'a> Parser<'a> {
                         .unwrap_or(cur_span);
                     expr = Spanned::new(Expr::Try(Box::new(expr)), start.merge(end));
                 }
-                _ => break,
             }
         }
 
@@ -2478,7 +2667,7 @@ impl<'a> Parser<'a> {
         let then = self.parse_block(ctx)?;
         let else_ = if self.eat(&Token::Else).is_some() {
             if self.check(&Token::If) {
-                let e = self.parse_if_expr(ctx)?;
+                let e = self.nested(|p| p.parse_if_expr(ctx))?;
                 Some(Box::new(e))
             } else {
                 let b = self.parse_block(ctx)?;
@@ -2648,6 +2837,11 @@ impl<'a> Parser<'a> {
     /// `{ child* }`, the children of a `view` or an element. A `{` inside is
     /// an element's or a block's again, whatever context the view is in.
     fn parse_view_children(&mut self) -> Option<Vec<Spanned<ViewNode>>> {
+        self.nested(|p| p.parse_view_children_here())
+    }
+
+    /// [`Parser::parse_view_children`] at the cursor's level.
+    fn parse_view_children_here(&mut self) -> Option<Vec<Spanned<ViewNode>>> {
         self.expect(&Token::LBrace, "view children")?;
         let saved = std::mem::replace(&mut self.no_struct_literal, false);
         let mut children = Vec::new();
@@ -2838,6 +3032,12 @@ impl<'a> Parser<'a> {
 
 impl<'a> Parser<'a> {
     fn parse_pattern(&mut self, ctx: &str) -> Option<Spanned<Pattern>> {
+        self.nested(|p| p.parse_pattern_here(ctx))
+    }
+
+    /// [`Parser::parse_pattern`] at the cursor's level: an atom, or
+    /// alternatives.
+    fn parse_pattern_here(&mut self, ctx: &str) -> Option<Spanned<Pattern>> {
         let start = self.peek_span();
         let pat = self.parse_pattern_atom(ctx)?;
 
@@ -2845,13 +3045,13 @@ impl<'a> Parser<'a> {
         if self.check(&Token::Pipe) {
             let mut alternatives = vec![pat];
             while self.eat(&Token::Pipe).is_some() {
-                if let Some(p) = self.parse_pattern_atom(ctx) {
-                    alternatives.push(p);
+                if let Some(alt) = self.parse_pattern_atom(ctx) {
+                    alternatives.push(alt);
                 } else {
                     break;
                 }
             }
-            let end = alternatives.last().map(|p| p.span).unwrap_or(start);
+            let end = alternatives.last().map(|a| a.span).unwrap_or(start);
             return Some(Spanned::new(Pattern::Or(alternatives), start.merge(end)));
         }
 
diff --git a/crates/nova-parser/src/lib.rs b/crates/nova-parser/src/lib.rs
index 329d7506a13800a2af9bcad2d8b36c8fcb1ef988..841841218ea1335d0c69fd4b373a37f0c8ff5569 100644
--- a/crates/nova-parser/src/lib.rs
+++ b/crates/nova-parser/src/lib.rs
@@ -29,6 +29,11 @@ pub use error::ParseError;
 use nova_ast::File;
 use nova_diagnostics::{FileId, Spanned};
 use nova_lexer::Tokens;
+/// How deeply source may nest: past it, the parser reports
+/// [`ParseError::TooDeep`] (P0002) and stops (spec
+/// `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+/// §4.4, ADR 0052). `grammar.rs`'s `Parser::deeper` says what a level is.
+pub const MAX_DEPTH: usize = 128;
 
 /// Parse a token stream into a `File` AST.
 ///
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index f09ecd9c46b26b7ace69737c1608f111a707378b..a3ad6f2d320e95e7c8b9328286400be83c0d3024 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -2455,7 +2455,7 @@ fn std_module(src: &str, file_id: FileId) -> (Option<File>, Vec<Diagnostic>) {
     let (ast, parse_errors) = nova_parser::parse(&tokens, file_id);
     diags.extend(parse_errors.iter().map(|e| {
         e.notes().into_iter().fold(
-            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here"),
+            Diagnostic::error(e.code(), e.to_string()).with_primary_label(e.span(), "here"),
             |d, n| d.with_note(n),
         )
     }));
````

### Task 2: S0's pipeline on a 64 MB thread

Spec §4.4; decisions 10, 20, 21, 36 and 37.

**Files:**
- Modify: `crates/nova-driver/src/lib.rs` (`PIPELINE_STACK`, the
  `CompiledProgram` export), `crates/nova-cli/src/main.rs` (the thread,
  `Ran`, `dispatch`), `crates/nova-cli/src/cmd/run.rs` (`Compiled`, its
  `Send`, `compile`), `crates/nova-cli/src/cmd/dev.rs` (the rebuilds'
  thread), `crates/nova-lsp/src/checker.rs` (the checker's thread)
- Test: `crates/nova-cli/tests/lsp.rs` (one), `selfhost.rs` (three)

- [ ] **Step 1: The tests.** `patch_block t2-tests`. The patch holds:
  - `lsp.rs`: `a_file_nested_too_deeply_gets_one_p0002` (300
    parentheses: one diagnostic, P0002; a fix clears it);
  - `selfhost.rs`, the head of 5.1b's section and the thread's tests:
    `a_panic_on_the_pipelines_thread_exits_101_with_its_message` (Linux:
    `nova parse` printing into `/dev/full`: exit 101, one `panicked at`,
    `thread 'main'`), `a_long_decimal_number_lexes_with_a_1_mb_main_thread`
    (Unix: `--dump-tokens` of 4,000 digits under `ulimit -s 1024`), and
    `nova_run_runs_the_program_on_the_main_thread` (Unix, `ulimit -s
    8192`: a recursion 100,000 deep prints `100000`, one 2,000,000 deep
    overflows).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which
  ends with `Finished`.
  - `cargo test --locked --workspace --test lsp -- a_file_nested_too_deeply`.
    Expected: `0 passed; 1 failed`: `thread 'nova-lsp-checker' … has overflowed its
    stack`, and the client reads no diagnostics.
  - `SH -- a_panic_on_the_pipelines_thread a_long_decimal_number nova_run_runs_the_program`.
    Expected: `2 passed; 1 failed`:
    `a_long_decimal_number_lexes_with_a_1_mb_main_thread`, S0 aborting with
    `thread 'main' … has overflowed its stack`; the panic test and the
    `nova run` test pass, as at the base (decision 47).
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`nova-driver`:** `PIPELINE_STACK`, 64 MB, its doc the reason; `pub
    use nova_codegen_cranelift::CompiledProgram`.
  - **`main.rs`:** after the arguments, logging and the embedded runtime,
    the command runs on a thread named `main` with `PIPELINE_STACK`;
    `Ran::Done` returns its result, `Ran::Program` runs `nova run`'s
    program on the main thread, a panic resumes by `resume_unwind`.
  - **`run.rs`:** `compile`, `nova run`'s compiling half, returning
    `Compiled`, the program and its arguments, `unsafe impl Send` with
    its argument (decision 21), and `Compiled::run`.
  - **`dev.rs`, `checker.rs`:** their threads get `PIPELINE_STACK`; a
    rebuild thread that cannot start is an error.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --test lsp -- a_file_nested_too_deeply`.
    Expected: `1 passed; 0 failed`.
  - `SH -- a_panic_on_the_pipelines_thread a_long_decimal_number nova_run_runs_the_program`.
    Expected: `3 passed; 0 failed`.
  - `S` (6 min 20 s). Expected: `2828 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `cli: every command's pipeline on a 64 MB thread`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/lsp.rs b/crates/nova-cli/tests/lsp.rs
index 2c045d53a4b4895946cc7bac9fe0edecc22b8846..46bf4a9a2a4ed5d0746eab703df08e85294f8b90 100644
--- a/crates/nova-cli/tests/lsp.rs
+++ b/crates/nova-cli/tests/lsp.rs
@@ -332,6 +332,25 @@ fn loose(name: &str, text: &str) -> (Client, String) {
     (client, uri)
 }
 
+/// A file nested past the parser's limit gets one diagnostic, P0002, from
+/// the server's analysis, which runs on the pipeline's stack (spec
+/// `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+/// §6.2); and the server goes on.
+#[test]
+fn a_file_nested_too_deeply_gets_one_p0002() {
+    let text = format!(
+        "fn main() {{\n    let x = {}1{}\n}}\n",
+        "(".repeat(300),
+        ")".repeat(300)
+    );
+    let (mut client, uri) = loose("too-deep", &text);
+    let diagnostics = client.diagnostics(&uri, nonempty);
+    assert_eq!(codes(&diagnostics), ["P0002"], "{diagnostics}");
+    // A fix clears it.
+    change(&mut client, &uri, 2, "fn main() {}\n");
+    client.diagnostics(&uri, |p| !nonempty(p));
+}
+
 const BROKEN_WITH_DOTS: &str = "record Point { x: Int, y: Int }\n\
 fn broken( {\n}\n\
 fn main() {\n    let p = Point { x: 1, y: 2 }\n    let v: Vec<Int> = Vec::new()\n    p.\n    let n = 1\n    v.\n}\n";
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index fff81a853bc64a30540c08e518ea262d3f446086..c8b3d598cbd144e9a8594d7d09c930742c32eeff 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -887,3 +887,115 @@ fn a_missing_final_line_break_is_marked() {
     let m = first_difference(b"== a.nova\n0 0 Eof\n", b"== a.nova\n0 1 Eof").expect("a difference");
     assert_eq!(m.n.as_deref(), Some("0 1 Eof"));
 }
+
+// --- 5.1b: the AST dump, the depth limit and S0's thread (ADR 0052) ---
+
+/// A panic on the pipeline's thread is printed once, naming the thread
+/// `main` as before, and `nova` exits 101: here `println!` into a full
+/// device, Linux's `/dev/full`.
+#[cfg(target_os = "linux")]
+#[test]
+fn a_panic_on_the_pipelines_thread_exits_101_with_its_message() {
+    let dir = fresh_dir("panic");
+    std::fs::write(dir.join("a.nova"), "fn main() {}\n").expect("write a.nova");
+    let full = std::fs::OpenOptions::new()
+        .write(true)
+        .open("/dev/full")
+        .expect("open /dev/full");
+    let out = Command::new(nova())
+        .current_dir(&dir)
+        .args(["parse", "a.nova"])
+        .env("RUST_BACKTRACE", "0")
+        .stdout(full)
+        .output()
+        .expect("run nova parse");
+    let stderr = String::from_utf8_lossy(&out.stderr);
+    assert_eq!(out.status.code(), Some(101), "{stderr}");
+    assert!(
+        stderr.starts_with("\nthread 'main' ") || stderr.starts_with("thread 'main' "),
+        "{stderr}"
+    );
+    assert_eq!(stderr.matches("panicked at").count(), 1, "{stderr}");
+    assert!(
+        stderr.contains("failed printing to stdout: No space left on device"),
+        "{stderr}"
+    );
+}
+
+/// ADR 0051's known bug: a decimal number of 4,000 characters overflowed
+/// S0's main thread at 1 MB, Windows' size. On the pipeline's thread,
+/// `nova parse --dump-tokens` lexes it with the main thread held at 1 MB.
+#[cfg(unix)]
+#[test]
+fn a_long_decimal_number_lexes_with_a_1_mb_main_thread() {
+    let dir = fresh_dir("long-number");
+    std::fs::write(
+        dir.join("a.nova"),
+        format!("let a = {}\n", "1".repeat(4000)),
+    )
+    .expect("write a.nova");
+    let script = format!(
+        "ulimit -s 1024 && exec '{}' parse --dump-tokens a.nova",
+        nova().display()
+    );
+    let out = output_within(
+        Command::new("sh")
+            .current_dir(&dir)
+            .args(["-c", &script])
+            .stdin(Stdio::null()),
+        CHILD_LIMIT,
+        "nova parse --dump-tokens under ulimit -s 1024",
+    )
+    .unwrap_or_else(|e| panic!("{e}"));
+    let stdout = String::from_utf8_lossy(&out.stdout);
+    assert_eq!(
+        out.status.code(),
+        Some(0),
+        "{}",
+        String::from_utf8_lossy(&out.stderr)
+    );
+    assert!(stdout.contains(" Eof NL\n"), "{stdout}");
+}
+
+/// `nova run` compiles on the pipeline's thread and runs the program on the
+/// main thread, so the program has the stack it has when built, the
+/// platform's main thread's (ADR 0052): with the main thread held at 8 MB,
+/// a recursion 100,000 deep runs, and one 2,000,000 deep, which 64 MB would
+/// hold, overflows it.
+#[cfg(unix)]
+#[test]
+fn nova_run_runs_the_program_on_the_main_thread() {
+    let dir = fresh_dir("run-stack");
+    let program = |n: usize| {
+        format!(
+            "fn down(n: Int) -> Int {{\n    if n == 0 {{ return 0 }}\n    1 + down(n - 1)\n}}\n\n\
+             fn main() {{\n    println(\"${{down({n})}}\")\n}}\n"
+        )
+    };
+    for (n, file) in [(100_000, "shallow.nova"), (2_000_000, "deep.nova")] {
+        std::fs::write(dir.join(file), program(n)).expect("write a program");
+    }
+    let run = |file: &str| {
+        let script = format!("ulimit -s 8192 && exec '{}' run {file}", nova().display());
+        let out = output_within(
+            Command::new("sh")
+                .current_dir(&dir)
+                .env("NOVA_HOME", dir.join("home"))
+                .args(["-c", &script])
+                .stdin(Stdio::null()),
+            CHILD_LIMIT,
+            &format!("nova run {file} under ulimit -s 8192"),
+        )
+        .unwrap_or_else(|e| panic!("{e}"));
+        (
+            out.status.code(),
+            String::from_utf8_lossy(&out.stdout).into_owned(),
+            String::from_utf8_lossy(&out.stderr).into_owned(),
+        )
+    };
+    let (code, stdout, stderr) = run("shallow.nova");
+    assert_eq!((code, stdout.as_str()), (Some(0), "100000\n"), "{stderr}");
+    let (code, _, stderr) = run("deep.nova");
+    assert_ne!(code, Some(0), "{stderr}");
+    assert!(stderr.contains("has overflowed its stack"), "{stderr}");
+}
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-cli/src/cmd/dev.rs b/crates/nova-cli/src/cmd/dev.rs
index 9f5a484435a0916078d9e3576fb51af50344db69..6d0a43bfa7141e595b6aab7adda4e6e8091e8f91 100644
--- a/crates/nova-cli/src/cmd/dev.rs
+++ b/crates/nova-cli/src/cmd/dev.rs
@@ -96,7 +96,11 @@ pub fn run(cmd: DevCmd) -> Result<()> {
         open(&url);
     }
 
-    std::thread::spawn(move || watch(seen, || watched.poll(), || builder.build()));
+    // Each rebuild runs the pipeline, so on a stack of its size (ADR 0052).
+    std::thread::Builder::new()
+        .stack_size(nova_driver::PIPELINE_STACK)
+        .spawn(move || watch(seen, || watched.poll(), || builder.build()))
+        .context("could not start the thread that rebuilds")?;
     server::serve(
         listener,
         Server {
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index 64f65a6caac4bccb32122025ec6473733411142f..4a80950e3b62257ad1a2864100cb723cfc914cba 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -123,15 +123,45 @@ fn entry(program: &Program) -> PathBuf {
         .unwrap_or_default()
 }
 
-pub fn run(cmd: RunCmd) -> Result<()> {
+/// A program `nova run` compiled, with the arguments it runs with. The
+/// command's thread compiles it, and `main` runs it on the process's main
+/// thread (`crate::main`): so the program keeps the stack it had before the
+/// pipeline moved to a thread of its own, the platform's main thread's,
+/// which is the stack it has when built (ADR 0052).
+pub struct Compiled {
+    program: nova_driver::CompiledProgram,
+    args: Vec<String>,
+}
+
+// SAFETY: a `CompiledProgram` is the JIT module that owns the program's
+// code and data, and a pointer to its `main` in that memory; it is not
+// `Send` only because the module holds raw pointers. That memory is mapped
+// for the process, not for a thread, and nothing in it belongs to the
+// thread that compiled it, which has ended by the time `main` runs it, so
+// one thread uses it at a time. The runtime's own state (its heap, its
+// executor, its file and socket tables) is per thread, made on first use
+// by the thread that runs the program: compiling calls nothing of the
+// runtime but its symbol table, and `set_args` is process-wide (ADR 0052).
+unsafe impl Send for Compiled {}
+
+impl Compiled {
+    /// Run the program on the calling thread, which `main` makes the main
+    /// thread.
+    pub fn run(self) {
+        self.program.run_with_args(self.args);
+    }
+}
+
+/// `nova run`'s compiling half: the program, ready for [`Compiled::run`].
+pub fn compile(cmd: RunCmd) -> Result<Compiled> {
     let mode = project::mode(cmd.file)?;
     project::refuse_library(&mode)?;
     project::sync(&mode)?;
     let program = project::program_to_run(&mode)?;
     let mut args = vec![entry(&program).to_string_lossy().into_owned()];
     args.extend(cmd.args.iter().map(|a| a.to_string_lossy().into_owned()));
-    match nova_driver::run_program(program, args)? {
-        Outcome::Ok(()) => Ok(()),
+    match nova_driver::compile_program(program)? {
+        Outcome::Ok(program) => Ok(Compiled { program, args }),
         Outcome::Failed { errors } => anyhow::bail!(
             "could not compile due to {errors} previous error{}",
             if errors == 1 { "" } else { "s" }
diff --git a/crates/nova-cli/src/main.rs b/crates/nova-cli/src/main.rs
index 61c819326fb5651b53554c520190b6ca831456b7..f90706bf9497f44c3c952478299ee40ad60c233d 100644
--- a/crates/nova-cli/src/main.rs
+++ b/crates/nova-cli/src/main.rs
@@ -90,6 +90,14 @@ enum Command {
     Version,
 }
 
+/// What a command's thread hands the main thread: the command's result,
+/// or, for `nova run`, the program it compiled, which runs on the main
+/// thread.
+enum Ran {
+    Done(Result<()>),
+    Program(Box<cmd::run::Compiled>),
+}
+
 fn main() -> Result<()> {
     let cli = Cli::parse();
     let filter = tracing_subscriber::EnvFilter::from_default_env()
@@ -108,9 +116,44 @@ fn main() -> Result<()> {
     }
 
     nova_driver::set_embedded_runtime(embedded::runtime());
-    match cli.command {
+    // The command runs on a thread with the pipeline's stack, so that a
+    // deeply nested source meets the parser's limit, not the platform's
+    // stack (spec
+    // `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+    // §4.4, ADR 0052). The thread is named `main`, so a panic's message
+    // reads as it did; the panic then resumes here, unwinding out of `main`
+    // as it did, with the exit code 101, and is not printed again. A
+    // stack overflow aborts the process from either thread, and
+    // `std::process::exit` ends it from either.
+    let command = cli.command;
+    let thread = std::thread::Builder::new()
+        .name("main".to_string())
+        .stack_size(nova_driver::PIPELINE_STACK)
+        .spawn(move || dispatch(command));
+    let ran = match thread {
+        Ok(thread) => thread.join(),
+        Err(e) => anyhow::bail!("could not start the compiler's thread: {e}"),
+    };
+    match ran {
+        Ok(Ran::Done(result)) => result,
+        Ok(Ran::Program(program)) => {
+            program.run();
+            Ok(())
+        }
+        Err(panic) => std::panic::resume_unwind(panic),
+    }
+}
+
+/// Run `command`, on the thread `main` starts for it.
+fn dispatch(command: Command) -> Ran {
+    let result = match command {
         Command::Parse(cmd) => cmd::parse::run(cmd),
-        Command::Run(cmd) => cmd::run::run(cmd),
+        Command::Run(cmd) => {
+            return match cmd::run::compile(cmd) {
+                Ok(program) => Ran::Program(Box::new(program)),
+                Err(e) => Ran::Done(Err(e)),
+            }
+        }
         Command::Build(cmd) => cmd::run::build(cmd),
         Command::Bundle(cmd) => cmd::run::bundle(cmd),
         Command::Dev(cmd) => cmd::dev::run(cmd),
@@ -131,5 +174,6 @@ fn main() -> Result<()> {
         Command::Package => cmd::package::package(),
         Command::Publish => cmd::package::publish(),
         Command::Version => cmd::version::run(),
-    }
+    };
+    Ran::Done(result)
 }
diff --git a/crates/nova-driver/src/lib.rs b/crates/nova-driver/src/lib.rs
index 741135da094c32a3d4d08ae06fe71842e469c924..585fe35f55b96b8287e4f55afbfada87172516f4 100644
--- a/crates/nova-driver/src/lib.rs
+++ b/crates/nova-driver/src/lib.rs
@@ -12,6 +12,7 @@ mod program;
 mod runtime_cache;
 
 pub use analyze::{analyze, analyze_program, Analysis, DiskSources, Options, Probe, Sources};
+pub use nova_codegen_cranelift::CompiledProgram;
 pub use nova_resolver::CompileTarget as Target;
 pub use program::{package_of, Program, Root, RootKind, Roots};
 pub use runtime_cache::{set_embedded_runtime, EmbeddedRuntime};
@@ -21,7 +22,6 @@ use std::hash::{Hash, Hasher};
 use std::path::{Path, PathBuf};
 
 use anyhow::{Context, Result};
-use nova_codegen_cranelift::CompiledProgram;
 use nova_diagnostics::{render, Diagnostic, FileDb, FileId, Severity, Span};
 use nova_hir as hir;
 use nova_hir::Ty;
@@ -110,6 +110,17 @@ pub fn check_program_counted_for(program: Program, target: Target) -> Result<Che
     })
 }
 
+/// The stack of the threads the pipeline runs on: every command's (`nova`'s
+/// `main`), the language server's checker's and `nova dev`'s rebuilds'
+/// (spec
+/// `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+/// §4.4, ADR 0052). The parser recurses once per level of nesting, up to
+/// `nova_parser::MAX_DEPTH`, and the stages after it about once per node,
+/// so a debug build on Windows' 1 MB main thread could overflow before the
+/// parser's limit; with 64 MB, the limit, not the platform, decides what a
+/// deep input meets. The WASM backend's own thread has 256 MB.
+pub const PIPELINE_STACK: usize = 64 << 20;
+
 /// Compile a file to native code via the Cranelift JIT.
 pub fn compile_file(path: &Path) -> Result<Outcome<CompiledProgram>> {
     compile_program(Program::for_file(path))
diff --git a/crates/nova-lsp/src/checker.rs b/crates/nova-lsp/src/checker.rs
index 3d692fc2ba35e9d252bf4b6c9f49b11446787174..0667330c6c15c77bc4c81c50085814d7367aac70 100644
--- a/crates/nova-lsp/src/checker.rs
+++ b/crates/nova-lsp/src/checker.rs
@@ -55,8 +55,11 @@ impl Checker {
         let dirs: Arc<Mutex<HashMap<ProjectKey, Vec<PathKey>>>> = Arc::default();
         let seen = Arc::clone(&newest);
         let written = Arc::clone(&dirs);
+        // The analysis is the pipeline's, so its thread has the pipeline's
+        // stack (ADR 0052).
         std::thread::Builder::new()
             .name("nova-lsp-checker".to_string())
+            .stack_size(nova_driver::PIPELINE_STACK)
             .spawn(move || serve(&queue, &seen, &written, &publish))
             .expect("start the checker thread");
         Checker { jobs, newest, dirs }
````

### Task 3: S0's AST dump and the hidden `--dump-ast`

Spec §4.3; decisions 3, 4, 15 to 17, 31 and 42.

**Files:**
- Create: `crates/nova-ast/src/dump.rs` (`dump_file`, `quote`,
  `escape_message`, `KINDS`, four unit tests),
  `crates/nova-parser/src/dump.rs` (`dump_ast`, two unit tests)
- Modify: `crates/nova-ast/src/lib.rs`, `crates/nova-parser/src/lib.rs`
  (the modules, the export), `crates/nova-cli/src/cmd/parse.rs`
  (`--dump-ast`, `dump_each`)
- Create: `crates/nova-cli/tests/selfhost/cases/every_other_node.nova`
- Test: `selfhost.rs` (two)

- [ ] **Step 1: The tests.** `patch_block t3-tests`. The patch holds:
  - `selfhost.rs`: `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`
    (absent from `--help`; each conflict; `a.nova`, `missing.nova`,
    `b.nova`: two dumps around `IOERR NotFound`, written out, exit 2);
    `kinds_on`, the forms a dump line names; and
    `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`, which
    reads S0's dumps alone until Task 6;
  - `every_other_node.nova`, the twenty kinds and three values the
    base's corpus lacks (decision 42).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which
  ends with `Finished`.
  - `SH -- the_ast_dump_flag every_kind_appears the_fixed_cases`.
    Expected: it does not compile, ``error[E0433]: cannot find `dump` in `nova_ast` `` and
    ``error: could not compile `nova-cli` (test "selfhost") due to 1 previous error`` (decision 47).
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`nova-ast`'s `dump.rs`:** its module doc the format's summary
    (ADR 0052 §1 the whole); `KINDS`; `dump_file`; `quote` and
    `escape_message`, the token dump's rules; the walk, a function per
    node in field order, as `nova-fmt`'s printer walks the tree; tests of
    a tree with every kind of field written out, of `KINDS`, of `KINDS`
    against the forms `nova-ast`'s declarations imply, read by a small
    scanner, and of the scanner (decision 17).
  - **`nova-parser`'s `dump.rs`:** `dump_ast`: lex, parse, the tree, then
    `ERR` for each lex error, `PARSE` and `NOTE`s for each parse error;
    tests of the order, a note and a split, and of a file nested 200 deep,
    one `PARSE` P0002.
  - **`parse.rs`:** `--dump-ast` (`hide = true`, conflicting with
    `--ast`, `--check` and `--dump-tokens`); `dump_each(files, dump)` for
    both dumps; `more` documented for both.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH -- the_ast_dump_flag every_kind_appears the_fixed_cases`.
    Expected: `3 passed; 0 failed`.
  - `cargo test --locked --workspace --lib -- dump::tests`. Expected:
    `4 passed`, `nova-ast`'s, and `2 passed`, `nova-parser`'s; every
    other binary `0 passed`.
  - `S` (5 min 51 s). Expected: `2836 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `ast, parser, cli: the AST dump, and the hidden nova parse --dump-ast`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index c8b3d598cbd144e9a8594d7d09c930742c32eeff..be7ee63b3ad88b376dff4ef3cae677221676561e 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -890,6 +890,122 @@ fn a_missing_final_line_break_is_marked() {
 
 // --- 5.1b: the AST dump, the depth limit and S0's thread (ADR 0052) ---
 
+#[test]
+fn the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header() {
+    let help = Command::new(nova())
+        .args(["parse", "--help"])
+        .output()
+        .expect("run nova parse --help");
+    let help = String::from_utf8_lossy(&help.stdout);
+    assert!(help.contains("--check"), "{help}");
+    assert!(!help.contains("dump-ast"), "{help}");
+    for other in ["--ast", "--check", "--dump-tokens"] {
+        let both = Command::new(nova())
+            .args(["parse", "--dump-ast", other, "a.nova"])
+            .output()
+            .expect("run nova parse");
+        let stderr = String::from_utf8_lossy(&both.stderr);
+        assert_eq!(both.status.code(), Some(2), "{other}: {stderr}");
+        assert!(stderr.contains("cannot be used with"), "{other}: {stderr}");
+    }
+    let dir = fresh_dir("ast-headers");
+    std::fs::write(dir.join("a.nova"), "fn f() {}\n").expect("write a.nova");
+    std::fs::write(dir.join("b.nova"), "fn g( {").expect("write b.nova");
+    let out = Command::new(nova())
+        .current_dir(&dir)
+        .args(["parse", "--dump-ast", "a.nova", "missing.nova", "b.nova"])
+        .output()
+        .expect("run nova parse --dump-ast");
+    assert_eq!(
+        String::from_utf8_lossy(&out.stdout),
+        "== a.nova\n(File\n  (Item.Function 0..9\n    \
+         (Function vis=Private is_async=false is_component=false name=\"f\"@3..4\n      \
+         (Block 7..9))))\n\
+         == missing.nova\nIOERR NotFound\n\
+         == b.nova\n(File)\n\
+         PARSE 6 7 expected identifier (in parameter name), found `{`\n\
+         PARSE 6 7 expected `)` (in function parameters), found `{`\n"
+    );
+    assert_eq!(out.status.code(), Some(2));
+}
+
+/// What `line` of an AST dump names among [`nova_ast::dump::KINDS`]'s
+/// forms: its node's kind, `(Kind`; each word outside quotes that is a
+/// field `key=value`, after the kind, `(Kind key=value`, when the value
+/// is a `bool`'s or `children=`'s; and each word that begins with a
+/// literal's prefix, `Int:`.
+fn kinds_on(line: &str, found: &mut std::collections::HashSet<String>) {
+    let node = line.trim_start();
+    let mut kind = String::new();
+    if let Some(rest) = node.strip_prefix('(') {
+        kind = rest
+            .chars()
+            .take_while(|c| *c != ' ' && *c != ')')
+            .collect();
+        found.insert(format!("({kind}"));
+    }
+    // Words, split at the spaces outside quotes; a quote inside one is
+    // written `\x22`, so each `"` opens or closes one.
+    let mut quoted = false;
+    let mut word = String::new();
+    for c in node.chars().chain([' ']) {
+        match c {
+            '"' => {
+                quoted = !quoted;
+                word.push(c);
+            }
+            ' ' if !quoted => {
+                let w = word.trim_end_matches(')');
+                if let Some((prefix, _)) = w.split_once(':') {
+                    if ["Int", "Float", "Str", "Char", "Bool"].contains(&prefix) {
+                        found.insert(format!("{prefix}:"));
+                    }
+                }
+                match w.split_once('=') {
+                    Some((_, "true" | "false" | "Some" | "None")) => {
+                        found.insert(format!("({kind} {w}"));
+                    }
+                    Some(_) if !w.contains('"') => {
+                        found.insert(w.to_string());
+                    }
+                    _ => {}
+                }
+                word.clear();
+            }
+            _ => word.push(c),
+        }
+    }
+}
+
+/// Every kind of node, field-less value and literal the dump can print
+/// appears in S0's dump of some corpus file or fixed case (spec §4.3).
+#[test]
+fn every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases() {
+    let root = root();
+    let mut files = corpus();
+    nova_files(&root, &cases_dir(), &[], &mut files);
+    let s0: &[&str] = &["parse", "--dump-ast"];
+    for (side, program, args) in [("S0", nova(), s0)] {
+        let mut found = std::collections::HashSet::new();
+        for chunk in files.chunks(BATCH) {
+            let out = run(&program, args, &root, chunk).unwrap_or_else(|e| panic!("{e}"));
+            assert_eq!(out.code, Some(0), "{side}: {}", out.stderr);
+            for line in String::from_utf8_lossy(&out.stdout).lines() {
+                kinds_on(line, &mut found);
+            }
+        }
+        let missing: Vec<&str> = nova_ast::dump::KINDS
+            .iter()
+            .copied()
+            .filter(|k| !found.contains(*k))
+            .collect();
+        assert!(
+            missing.is_empty(),
+            "no file's dump by {side} holds {missing:?}"
+        );
+    }
+}
+
 /// A panic on the pipeline's thread is printed once, naming the thread
 /// `main` as before, and `nova` exits 101: here `println!` into a full
 /// device, Linux's `/dev/full`.
diff --git a/crates/nova-cli/tests/selfhost/cases/every_other_node.nova b/crates/nova-cli/tests/selfhost/cases/every_other_node.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a828602006423611a716593439878d603ab743ed
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/every_other_node.nova
@@ -0,0 +1,41 @@
+// The kinds of node and values that no corpus file holds (ADR 0052), so
+// that the corpus and this file name every kind: a cast; `@`, record and
+// array patterns, a record pattern with and without `..`, its fields with
+// and without a pattern; reference, pointer (with and without `mut`) and
+// optional types; an import alias; the prefix operators `~`, `&`, `&mut`
+// and `*`; a module, a type alias, a trait's `async` signature and an item
+// in a block; and the newline rule's splits before a `[`, a `-` and the
+// value after a bare `return`.
+import json as j
+
+module m
+
+type Alias = Int
+
+trait Load {
+    async fn load() -> Int
+}
+
+fn g(a: &Int, b: *mut Int, c: Int?, d: &mut Point, e: *Int) -> Int {
+    let n = 1 as Float
+    let m = ~n
+    let r = &n
+    let s = &mut n
+    let t = *r
+    match d {
+        whole @ Point { x, y: 0, .. } => x,
+        Point { x } => x,
+        _ => 0,
+    }
+}
+
+fn h(xs: [Int]) -> Int {
+    fn inner() {}
+    let [a, b] = xs
+    xs
+    [0]
+    a
+    - b
+    return
+    a
+}
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-ast/src/dump.rs b/crates/nova-ast/src/dump.rs
new file mode 100644
index 0000000000000000000000000000000000000000..4e3001e6616c9c404a55b0786a1a09a1088cf19c
--- /dev/null
+++ b/crates/nova-ast/src/dump.rs
@@ -0,0 +1,1585 @@
+//! The AST dump: one file's tree as text, one node per line, which the
+//! self-hosted parser (`selfhost/`) prints too, so that the two parsers
+//! can be compared byte for byte (spec
+//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+//! §4.3; ADR 0052 writes the grammar out, with an example of every kind).
+//! It carries no stability promise: it changes whenever the tree does, on
+//! both sides in one pull request. `nova_parser::dump_ast` adds the
+//! errors after it, and `nova parse --dump-ast` a `== FILE` header before.
+//!
+//! - A node is a line: its indentation, two spaces per depth; `(` and its
+//!   kind; then, each after one space, its span `start..end` if the tree
+//!   holds one; its positional values; its fields `key=value`. Its
+//!   children follow, one per line, one level deeper, and its `)` closes
+//!   it at the end of its last line, after its last child's.
+//! - The kinds: a struct's name, and an enum's value `Enum.Variant`, both
+//!   as Rust names them, but for three the spec names: a `///` line is
+//!   `Doc`, an `@attribute` `Attr` and a newline split `Split`.
+//! - A `Spanned` node prints its span. An enum value that holds a struct,
+//!   such as `Item::Function(Function)`, is a node whose one child is the
+//!   struct's node, which has no span of its own.
+//! - A struct's or a variant's named fields print in their order: the
+//!   scalars on its line, `key=value` with the Rust field's name; the
+//!   nodes as children. A tuple variant's scalars print on its line as
+//!   positional values, without a key; so do a `Doc`'s text, an `Attr`'s
+//!   name and arguments, and a `Path` node's segments.
+//! - An absent `Option` prints nothing, and so does an empty list. The
+//!   one `Option` whose absence a list's emptiness would hide, an
+//!   element's children in a view, also prints `children=Some` or
+//!   `children=None`.
+//! - Values: a `bool` as `true` or `false`; a field-less enum (`Visibility`,
+//!   `BinOp`, `UnOp`, `AssignOp`, `SplitKind`) as its variant's name; a
+//!   string in double quotes, with each character below 0x20, 0x7F, `"` and
+//!   `\` written `\xHH` (two lowercase hex digits) and every other as it
+//!   is, as the token dump writes them; a name (`Spanned<String>`) as its
+//!   quoted string, `@` and its span; a path as its segments' names joined
+//!   by `::`; a literal as `Int:` and the value in decimal, `Float:` and
+//!   the 16 lowercase hex digits of `f64::to_bits`, `Str:` and the quoted
+//!   string, `Char:` and the code point in decimal, or `Bool:` and `true` or
+//!   `false`.
+//! - The tree is one `(File` node: its items, then its newline splits,
+//!   `(Split start..end kind=Kind)`. The text ends with a line break.
+
+use std::fmt::Write;
+
+use nova_diagnostics::{Span, Spanned};
+
+use crate::expr::{Expr, FieldInit, Literal, MatchArm, StringPart, ViewArg, ViewNode};
+use crate::item::{
+    AssocTypeBinding, Attribute, ConstDecl, ExternBlock, ExternItem, Function, FunctionSig,
+    ImplBlock, Import, ImportKind, Module, Param, Record, RecordField, TraitDecl, TraitItem,
+    TypeDecl, TypeDef, Variant, WhereBound,
+};
+use crate::pattern::{FieldPat, Pattern};
+use crate::ty::{Type, TypeParam};
+use crate::{Block, File, Item, Path, Stmt};
+
+/// Every kind and value the dump can print, by which the differential
+/// checks that the corpus and the cases hold each of them
+/// (`crates/nova-cli/tests/selfhost.rs`): a node's kind as `(Kind`, a
+/// field-less enum's value as `key=Value`, a literal's as `Int:`, and each
+/// `bool` field's two values and `children=`'s two, after their node's
+/// kind, as `(Kind key=value`. Every struct, enum variant and `bool` field
+/// of the tree is here: `every_declared_form_is_listed` reads them from
+/// this crate's sources and fails until a new one is listed.
+pub const KINDS: &[&str] = &[
+    // The 24 structs (`Attribute` is `Attr`, `NewlineSplit` is `Split`),
+    // and the `///` lines.
+    "(File",
+    "(Split",
+    "(Block",
+    "(Path",
+    "(TypeParam",
+    "(FieldPat",
+    "(MatchArm",
+    "(FieldInit",
+    "(Attr",
+    "(Param",
+    "(WhereBound",
+    "(Function",
+    "(Record",
+    "(RecordField",
+    "(TypeDecl",
+    "(Variant",
+    "(TraitDecl",
+    "(FunctionSig",
+    "(ImplBlock",
+    "(AssocTypeBinding",
+    "(ConstDecl",
+    "(Import",
+    "(Module",
+    "(ExternBlock",
+    "(Doc",
+    // `Item`
+    "(Item.Function",
+    "(Item.Record",
+    "(Item.Type",
+    "(Item.Trait",
+    "(Item.Impl",
+    "(Item.Const",
+    "(Item.Import",
+    "(Item.Module",
+    "(Item.Extern",
+    // `Stmt`
+    "(Stmt.Let",
+    "(Stmt.Expr",
+    "(Stmt.Item",
+    // `Expr`
+    "(Expr.Lit",
+    "(Expr.Path",
+    "(Expr.Tuple",
+    "(Expr.Array",
+    "(Expr.ArrayRepeat",
+    "(Expr.Block",
+    "(Expr.If",
+    "(Expr.Match",
+    "(Expr.While",
+    "(Expr.For",
+    "(Expr.Range",
+    "(Expr.Return",
+    "(Expr.Break",
+    "(Expr.Continue",
+    "(Expr.Closure",
+    "(Expr.Record",
+    "(Expr.Binary",
+    "(Expr.Unary",
+    "(Expr.Call",
+    "(Expr.Index",
+    "(Expr.Field",
+    "(Expr.Try",
+    "(Expr.Await",
+    "(Expr.Cast",
+    "(Expr.Assign",
+    "(Expr.StringInterp",
+    "(Expr.View",
+    // `ViewNode`, `ViewArg`, `StringPart`
+    "(ViewNode.Element",
+    "(ViewNode.Expr",
+    "(ViewArg.Named",
+    "(ViewArg.Positional",
+    "(StringPart.Lit",
+    "(StringPart.Expr",
+    // `Pattern`
+    "(Pattern.Wildcard",
+    "(Pattern.Lit",
+    "(Pattern.Ident",
+    "(Pattern.Binding",
+    "(Pattern.TupleStruct",
+    "(Pattern.Record",
+    "(Pattern.Tuple",
+    "(Pattern.Array",
+    "(Pattern.Or",
+    "(Pattern.Range",
+    "(Pattern.Path",
+    // `Type`
+    "(Type.Path",
+    "(Type.Ref",
+    "(Type.Ptr",
+    "(Type.Array",
+    "(Type.Tuple",
+    "(Type.Fn",
+    "(Type.Optional",
+    "(Type.Infer",
+    // `TypeDef`, `TraitItem`, `ImportKind`, `ExternItem`
+    "(TypeDef.Alias",
+    "(TypeDef.Sum",
+    "(TraitItem.Required",
+    "(TraitItem.Provided",
+    "(TraitItem.AssocType",
+    "(ImportKind.Simple",
+    "(ImportKind.Alias",
+    "(ImportKind.List",
+    "(ExternItem.Fn",
+    // `Literal`
+    "Int:",
+    "Float:",
+    "Str:",
+    "Char:",
+    "Bool:",
+    // `Visibility`
+    "vis=Pub",
+    "vis=Private",
+    // `SplitKind`
+    "kind=Call",
+    "kind=Index",
+    "kind=Minus",
+    "kind=Value",
+    // `BinOp`
+    "op=Or",
+    "op=And",
+    "op=Eq",
+    "op=Ne",
+    "op=Lt",
+    "op=Le",
+    "op=Gt",
+    "op=Ge",
+    "op=BitOr",
+    "op=BitXor",
+    "op=BitAnd",
+    "op=Shl",
+    "op=Shr",
+    "op=Add",
+    "op=Sub",
+    "op=Mul",
+    "op=Div",
+    "op=Rem",
+    // `UnOp`
+    "op=Neg",
+    "op=Not",
+    "op=BitNot",
+    "op=Ref",
+    "op=RefMut",
+    "op=Deref",
+    // `AssignOp`
+    "op=Assign",
+    "op=AddAssign",
+    "op=SubAssign",
+    "op=MulAssign",
+    "op=DivAssign",
+    "op=RemAssign",
+    "op=BitOrAssign",
+    "op=BitAndAssign",
+    "op=BitXorAssign",
+    "op=ShlAssign",
+    "op=ShrAssign",
+    // The `bool` fields, and an element's `children`, whose absence the
+    // dump prints (`Pattern.Ident`'s `is_mut` is never `true`: no parser
+    // builds a `mut` binding inside a pattern).
+    "(Stmt.Let is_mut=true",
+    "(Stmt.Let is_mut=false",
+    "(Param is_mut=true",
+    "(Param is_mut=false",
+    "(Function is_async=true",
+    "(Function is_async=false",
+    "(Function is_component=true",
+    "(Function is_component=false",
+    "(FunctionSig is_async=true",
+    "(FunctionSig is_async=false",
+    "(Expr.Range inclusive=true",
+    "(Expr.Range inclusive=false",
+    "(Pattern.Ident is_mut=false",
+    "(Pattern.Record rest=true",
+    "(Pattern.Record rest=false",
+    "(Pattern.Range inclusive=true",
+    "(Pattern.Range inclusive=false",
+    "(Type.Ref is_mut=true",
+    "(Type.Ref is_mut=false",
+    "(Type.Ptr is_mut=true",
+    "(Type.Ptr is_mut=false",
+    "(ViewNode.Element children=Some",
+    "(ViewNode.Element children=None",
+];
+
+/// The dump of `file`'s tree (module docs).
+pub fn dump_file(file: &File) -> String {
+    let mut d = Dump::default();
+    d.open(0, "File", None);
+    for item in &file.items {
+        d.item(1, Some(item.span), &item.value);
+    }
+    for split in &file.newline_splits {
+        d.open(1, "Split", Some(split.at));
+        d.field("kind", format_args!("{:?}", split.kind));
+        d.close();
+    }
+    d.close();
+    d.out.push('\n');
+    d.out
+}
+
+/// `text` in double quotes, with each character below 0x20, 0x7F, `"` and
+/// `\` as `\xHH` and every other as it is: the token dump's rule
+/// (`nova_lexer::dump_tokens`).
+pub fn quote(out: &mut String, text: &str) {
+    out.push('"');
+    for c in text.chars() {
+        if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {
+            let _ = write!(out, "\\x{:02x}", c as u32);
+        } else {
+            out.push(c);
+        }
+    }
+    out.push('"');
+}
+
+/// An error's message or note, which is not quoted: each character below
+/// 0x20 and 0x7F as `\xHH`, so that it stays on its line, and every other,
+/// `"` and `\` included, as it is (the token dump's `ERR` rule).
+pub fn escape_message(out: &mut String, text: &str) {
+    for c in text.chars() {
+        if (c as u32) < 0x20 || c == '\x7f' {
+            let _ = write!(out, "\\x{:02x}", c as u32);
+        } else {
+            out.push(c);
+        }
+    }
+}
+
+#[derive(Default)]
+struct Dump {
+    out: String,
+}
+
+impl Dump {
+    // --- Lines and values ---
+
+    /// A node's line, up to its kind and span.
+    fn open(&mut self, depth: usize, kind: &str, span: Option<Span>) {
+        if !self.out.is_empty() {
+            self.out.push('\n');
+        }
+        for _ in 0..depth {
+            self.out.push_str("  ");
+        }
+        self.out.push('(');
+        self.out.push_str(kind);
+        if let Some(span) = span {
+            let _ = write!(self.out, " {}..{}", span.start, span.end);
+        }
+    }
+
+    fn close(&mut self) {
+        self.out.push(')');
+    }
+
+    /// ` key=value`.
+    fn field(&mut self, key: &str, value: std::fmt::Arguments) {
+        let _ = write!(self.out, " {key}={value}");
+    }
+
+    fn bool_field(&mut self, key: &str, value: bool) {
+        self.field(key, format_args!("{value}"));
+    }
+
+    /// ` key=` and the name.
+    fn name_field(&mut self, key: &str, name: &Spanned<String>) {
+        let _ = write!(self.out, " {key}=");
+        self.name(name);
+    }
+
+    /// ` key=` and the path.
+    fn path_field(&mut self, key: &str, path: &Path) {
+        let _ = write!(self.out, " {key}=");
+        self.path(path);
+    }
+
+    /// A name: quoted, `@`, its span.
+    fn name(&mut self, name: &Spanned<String>) {
+        quote(&mut self.out, &name.value);
+        let _ = write!(self.out, "@{}..{}", name.span.start, name.span.end);
+    }
+
+    /// A path: its segments' names, joined by `::`.
+    fn path(&mut self, path: &Path) {
+        for (i, segment) in path.segments.iter().enumerate() {
+            if i > 0 {
+                self.out.push_str("::");
+            }
+            self.name(segment);
+        }
+    }
+
+    /// ` ` and a positional string.
+    fn text(&mut self, text: &str) {
+        self.out.push(' ');
+        quote(&mut self.out, text);
+    }
+
+    /// ` ` and a literal.
+    fn literal(&mut self, lit: &Literal) {
+        match lit {
+            Literal::Int(n) => {
+                let _ = write!(self.out, " Int:{n}");
+            }
+            Literal::Float(f) => {
+                let _ = write!(self.out, " Float:{:016x}", f.to_bits());
+            }
+            Literal::Str(s) => {
+                self.out.push_str(" Str:");
+                quote(&mut self.out, s);
+            }
+            Literal::Char(c) => {
+                let _ = write!(self.out, " Char:{}", *c as u32);
+            }
+            Literal::Bool(b) => {
+                let _ = write!(self.out, " Bool:{b}");
+            }
+        }
+    }
+
+    // --- Shared children ---
+
+    fn docs(&mut self, depth: usize, docs: &[Spanned<String>]) {
+        for doc in docs {
+            self.open(depth, "Doc", Some(doc.span));
+            self.text(&doc.value);
+            self.close();
+        }
+    }
+
+    fn attrs(&mut self, depth: usize, attrs: &[Attribute]) {
+        for attr in attrs {
+            self.open(depth, "Attr", Some(attr.span));
+            self.out.push(' ');
+            self.name(&attr.name);
+            for arg in &attr.args {
+                self.out.push(' ');
+                self.name(arg);
+            }
+            self.close();
+        }
+    }
+
+    fn path_node(&mut self, depth: usize, path: &Spanned<Path>) {
+        self.open(depth, "Path", Some(path.span));
+        self.out.push(' ');
+        self.path(&path.value);
+        self.close();
+    }
+
+    fn paths(&mut self, depth: usize, paths: &[Spanned<Path>]) {
+        for path in paths {
+            self.path_node(depth, path);
+        }
+    }
+
+    fn generics(&mut self, depth: usize, generics: &[TypeParam]) {
+        for g in generics {
+            self.open(depth, "TypeParam", None);
+            self.name_field("name", &g.name);
+            self.paths(depth + 1, &g.bounds);
+            self.close();
+        }
+    }
+
+    fn params(&mut self, depth: usize, params: &[Param]) {
+        for p in params {
+            self.open(depth, "Param", None);
+            self.bool_field("is_mut", p.is_mut);
+            self.name_field("name", &p.name);
+            self.ty(depth + 1, &p.ty);
+            self.close();
+        }
+    }
+
+    fn where_clause(&mut self, depth: usize, bounds: &[WhereBound]) {
+        for b in bounds {
+            self.open(depth, "WhereBound", None);
+            self.ty(depth + 1, &b.ty);
+            self.paths(depth + 1, &b.bounds);
+            self.close();
+        }
+    }
+
+    // --- Items ---
+
+    fn item(&mut self, depth: usize, span: Option<Span>, item: &Item) {
+        let kind = match item {
+            Item::Function(_) => "Item.Function",
+            Item::Record(_) => "Item.Record",
+            Item::Type(_) => "Item.Type",
+            Item::Trait(_) => "Item.Trait",
+            Item::Impl(_) => "Item.Impl",
+            Item::Const(_) => "Item.Const",
+            Item::Import(_) => "Item.Import",
+            Item::Module(_) => "Item.Module",
+            Item::Extern(_) => "Item.Extern",
+        };
+        self.open(depth, kind, span);
+        let depth = depth + 1;
+        match item {
+            Item::Function(f) => self.function(depth, f),
+            Item::Record(r) => self.record(depth, r),
+            Item::Type(t) => self.type_decl(depth, t),
+            Item::Trait(t) => self.trait_decl(depth, t),
+            Item::Impl(i) => self.impl_block(depth, i),
+            Item::Const(c) => self.const_decl(depth, c),
+            Item::Import(i) => self.import(depth, i),
+            Item::Module(m) => self.module(depth, m),
+            Item::Extern(e) => self.extern_block(depth, e),
+        }
+        self.close();
+    }
+
+    fn function(&mut self, depth: usize, f: &Function) {
+        self.open(depth, "Function", None);
+        self.field("vis", format_args!("{:?}", f.vis));
+        self.bool_field("is_async", f.is_async);
+        self.bool_field("is_component", f.is_component);
+        self.name_field("name", &f.name);
+        let depth = depth + 1;
+        self.docs(depth, &f.docs);
+        self.attrs(depth, &f.attrs);
+        self.generics(depth, &f.generics);
+        self.params(depth, &f.params);
+        if let Some(t) = &f.return_ty {
+            self.ty(depth, t);
+        }
+        self.where_clause(depth, &f.where_clause);
+        self.block(depth, Some(f.body.span), &f.body.value);
+        self.close();
+    }
+
+    fn function_sig(&mut self, depth: usize, f: &FunctionSig) {
+        self.open(depth, "FunctionSig", None);
+        self.bool_field("is_async", f.is_async);
+        self.name_field("name", &f.name);
+        let depth = depth + 1;
+        self.docs(depth, &f.docs);
+        self.generics(depth, &f.generics);
+        self.params(depth, &f.params);
+        if let Some(t) = &f.return_ty {
+            self.ty(depth, t);
+        }
+        self.where_clause(depth, &f.where_clause);
+        self.close();
+    }
+
+    fn record(&mut self, depth: usize, r: &Record) {
+        self.open(depth, "Record", None);
+        self.field("vis", format_args!("{:?}", r.vis));
+        self.name_field("name", &r.name);
+        let depth = depth + 1;
+        self.docs(depth, &r.docs);
+        self.attrs(depth, &r.attrs);
+        self.generics(depth, &r.generics);
+        for f in &r.fields {
+            self.record_field(depth, f);
+        }
+        self.close();
+    }
+
+    fn record_field(&mut self, depth: usize, f: &RecordField) {
+        self.open(depth, "RecordField", None);
+        self.field("vis", format_args!("{:?}", f.vis));
+        self.name_field("name", &f.name);
+        self.docs(depth + 1, &f.docs);
+        self.ty(depth + 1, &f.ty);
+        self.close();
+    }
+
+    fn type_decl(&mut self, depth: usize, t: &TypeDecl) {
+        self.open(depth, "TypeDecl", None);
+        self.field("vis", format_args!("{:?}", t.vis));
+        self.name_field("name", &t.name);
+        let depth = depth + 1;
+        self.docs(depth, &t.docs);
+        self.attrs(depth, &t.attrs);
+        self.generics(depth, &t.generics);
+        match &t.def {
+            TypeDef::Alias(ty) => {
+                self.open(depth, "TypeDef.Alias", None);
+                self.ty(depth + 1, ty);
+                self.close();
+            }
+            TypeDef::Sum(variants) => {
+                self.open(depth, "TypeDef.Sum", None);
+                for v in variants {
+                    self.variant(depth + 1, v);
+                }
+                self.close();
+            }
+        }
+        self.close();
+    }
+
+    fn variant(&mut self, depth: usize, v: &Variant) {
+        self.open(depth, "Variant", None);
+        self.name_field("name", &v.name);
+        self.docs(depth + 1, &v.docs);
+        for t in &v.fields {
+            self.ty(depth + 1, t);
+        }
+        self.close();
+    }
+
+    fn trait_decl(&mut self, depth: usize, t: &TraitDecl) {
+        self.open(depth, "TraitDecl", None);
+        self.field("vis", format_args!("{:?}", t.vis));
+        self.name_field("name", &t.name);
+        let depth = depth + 1;
+        self.docs(depth, &t.docs);
+        self.attrs(depth, &t.attrs);
+        self.generics(depth, &t.generics);
+        self.paths(depth, &t.supertraits);
+        self.where_clause(depth, &t.where_clause);
+        for item in &t.items {
+            match item {
+                TraitItem::Required(sig) => {
+                    self.open(depth, "TraitItem.Required", None);
+                    self.function_sig(depth + 1, sig);
+                }
+                TraitItem::Provided(f) => {
+                    self.open(depth, "TraitItem.Provided", None);
+                    self.function(depth + 1, f);
+                }
+                TraitItem::AssocType { docs, name, bounds } => {
+                    self.open(depth, "TraitItem.AssocType", None);
+                    self.name_field("name", name);
+                    self.docs(depth + 1, docs);
+                    self.paths(depth + 1, bounds);
+                }
+            }
+            self.close();
+        }
+        self.close();
+    }
+
+    fn impl_block(&mut self, depth: usize, i: &ImplBlock) {
+        self.open(depth, "ImplBlock", None);
+        let depth = depth + 1;
+        self.docs(depth, &i.docs);
+        self.attrs(depth, &i.attrs);
+        self.generics(depth, &i.generics);
+        if let Some(t) = &i.trait_ {
+            self.path_node(depth, t);
+        }
+        self.ty(depth, &i.ty);
+        self.where_clause(depth, &i.where_clause);
+        for f in &i.functions {
+            self.function(depth, f);
+        }
+        for c in &i.consts {
+            self.const_decl(depth, c);
+        }
+        for a in &i.assoc_types {
+            self.assoc_type_binding(depth, a);
+        }
+        self.close();
+    }
+
+    fn assoc_type_binding(&mut self, depth: usize, a: &AssocTypeBinding) {
+        self.open(depth, "AssocTypeBinding", None);
+        self.name_field("name", &a.name);
+        self.docs(depth + 1, &a.docs);
+        self.ty(depth + 1, &a.ty);
+        self.close();
+    }
+
+    fn const_decl(&mut self, depth: usize, c: &ConstDecl) {
+        self.open(depth, "ConstDecl", None);
+        self.field("vis", format_args!("{:?}", c.vis));
+        self.name_field("name", &c.name);
+        let depth = depth + 1;
+        self.docs(depth, &c.docs);
+        self.attrs(depth, &c.attrs);
+        self.ty(depth, &c.ty);
+        self.expr(depth, &c.value);
+        self.close();
+    }
+
+    fn import(&mut self, depth: usize, i: &Import) {
+        self.open(depth, "Import", None);
+        let depth = depth + 1;
+        self.docs(depth, &i.docs);
+        self.attrs(depth, &i.attrs);
+        self.path_node(depth, &i.path);
+        match &i.kind {
+            ImportKind::Simple => self.open(depth, "ImportKind.Simple", None),
+            ImportKind::Alias(name) => {
+                self.open(depth, "ImportKind.Alias", None);
+                self.out.push(' ');
+                self.name(name);
+            }
+            ImportKind::List(names) => {
+                self.open(depth, "ImportKind.List", None);
+                for name in names {
+                    self.out.push(' ');
+                    self.name(name);
+                }
+            }
+        }
+        self.close();
+        self.close();
+    }
+
+    fn module(&mut self, depth: usize, m: &Module) {
+        self.open(depth, "Module", None);
+        self.docs(depth + 1, &m.docs);
+        self.attrs(depth + 1, &m.attrs);
+        self.path_node(depth + 1, &m.path);
+        self.close();
+    }
+
+    fn extern_block(&mut self, depth: usize, e: &ExternBlock) {
+        self.open(depth, "ExternBlock", None);
+        if let Some(abi) = &e.abi {
+            self.out.push_str(" abi=");
+            quote(&mut self.out, abi);
+        }
+        let depth = depth + 1;
+        self.docs(depth, &e.docs);
+        self.attrs(depth, &e.attrs);
+        for item in &e.items {
+            match item {
+                ExternItem::Fn(sig) => {
+                    self.open(depth, "ExternItem.Fn", None);
+                    self.function_sig(depth + 1, sig);
+                    self.close();
+                }
+            }
+        }
+        self.close();
+    }
+
+    // --- Blocks and statements ---
+
+    fn block(&mut self, depth: usize, span: Option<Span>, b: &Block) {
+        self.open(depth, "Block", span);
+        for s in &b.stmts {
+            self.stmt(depth + 1, s);
+        }
+        if let Some(t) = &b.trailing {
+            self.expr(depth + 1, t);
+        }
+        self.close();
+    }
+
+    fn stmt(&mut self, depth: usize, s: &Spanned<Stmt>) {
+        match &s.value {
+            Stmt::Let {
+                is_mut,
+                pattern,
+                ty,
+                init,
+            } => {
+                self.open(depth, "Stmt.Let", Some(s.span));
+                self.bool_field("is_mut", *is_mut);
+                self.pattern(depth + 1, pattern);
+                if let Some(t) = ty {
+                    self.ty(depth + 1, t);
+                }
+                if let Some(e) = init {
+                    self.expr(depth + 1, e);
+                }
+            }
+            Stmt::Expr(e) => {
+                self.open(depth, "Stmt.Expr", Some(s.span));
+                self.expr(depth + 1, e);
+            }
+            Stmt::Item(item) => {
+                self.open(depth, "Stmt.Item", Some(s.span));
+                self.item(depth + 1, None, item);
+            }
+        }
+        self.close();
+    }
+
+    // --- Expressions ---
+
+    fn exprs(&mut self, depth: usize, exprs: &[Spanned<Expr>]) {
+        for e in exprs {
+            self.expr(depth, e);
+        }
+    }
+
+    fn expr(&mut self, depth: usize, e: &Spanned<Expr>) {
+        let span = Some(e.span);
+        let inner = depth + 1;
+        match &e.value {
+            Expr::Lit(lit) => {
+                self.open(depth, "Expr.Lit", span);
+                self.literal(lit);
+            }
+            Expr::Path(path) => {
+                self.open(depth, "Expr.Path", span);
+                self.out.push(' ');
+                self.path(path);
+            }
+            Expr::Tuple(items) => {
+                self.open(depth, "Expr.Tuple", span);
+                self.exprs(inner, items);
+            }
+            Expr::Array(items) => {
+                self.open(depth, "Expr.Array", span);
+                self.exprs(inner, items);
+            }
+            Expr::ArrayRepeat { init, len } => {
+                self.open(depth, "Expr.ArrayRepeat", span);
+                self.expr(inner, init);
+                self.expr(inner, len);
+            }
+            Expr::Block(b) => {
+                self.open(depth, "Expr.Block", span);
+                self.block(inner, None, b);
+            }
+            Expr::If { cond, then, else_ } => {
+                self.open(depth, "Expr.If", span);
+                self.expr(inner, cond);
+                self.block(inner, Some(then.span), &then.value);
+                if let Some(e) = else_ {
+                    self.expr(inner, e);
+                }
+            }
+            Expr::Match { scrutinee, arms } => {
+                self.open(depth, "Expr.Match", span);
+                self.expr(inner, scrutinee);
+                for arm in arms {
+                    self.match_arm(inner, arm);
+                }
+            }
+            Expr::While { cond, body } => {
+                self.open(depth, "Expr.While", span);
+                self.expr(inner, cond);
+                self.block(inner, Some(body.span), &body.value);
+            }
+            Expr::For {
+                pattern,
+                iter,
+                body,
+            } => {
+                self.open(depth, "Expr.For", span);
+                self.pattern(inner, pattern);
+                self.expr(inner, iter);
+                self.block(inner, Some(body.span), &body.value);
+            }
+            Expr::Range { lo, hi, inclusive } => {
+                self.open(depth, "Expr.Range", span);
+                self.bool_field("inclusive", *inclusive);
+                self.expr(inner, lo);
+                self.expr(inner, hi);
+            }
+            Expr::Return(value) => {
+                self.open(depth, "Expr.Return", span);
+                if let Some(v) = value {
+                    self.expr(inner, v);
+                }
+            }
+            Expr::Break(value) => {
+                self.open(depth, "Expr.Break", span);
+                if let Some(v) = value {
+                    self.expr(inner, v);
+                }
+            }
+            Expr::Continue => self.open(depth, "Expr.Continue", span),
+            Expr::Closure { params, ret, body } => {
+                self.open(depth, "Expr.Closure", span);
+                self.params(inner, params);
+                if let Some(t) = ret {
+                    self.ty(inner, t);
+                }
+                self.expr(inner, body);
+            }
+            Expr::Record { path, fields, base } => {
+                self.open(depth, "Expr.Record", span);
+                self.path_field("path", path);
+                for f in fields {
+                    self.field_init(inner, f);
+                }
+                if let Some(b) = base {
+                    self.expr(inner, b);
+                }
+            }
+            Expr::Binary { op, lhs, rhs } => {
+                self.open(depth, "Expr.Binary", span);
+                self.field("op", format_args!("{op:?}"));
+                self.expr(inner, lhs);
+                self.expr(inner, rhs);
+            }
+            Expr::Unary { op, expr } => {
+                self.open(depth, "Expr.Unary", span);
+                self.field("op", format_args!("{op:?}"));
+                self.expr(inner, expr);
+            }
+            Expr::Call { callee, args } => {
+                self.open(depth, "Expr.Call", span);
+                self.expr(inner, callee);
+                self.exprs(inner, args);
+            }
+            Expr::Index { target, index } => {
+                self.open(depth, "Expr.Index", span);
+                self.expr(inner, target);
+                self.expr(inner, index);
+            }
+            Expr::Field { target, field } => {
+                self.open(depth, "Expr.Field", span);
+                self.name_field("field", field);
+                self.expr(inner, target);
+            }
+            Expr::Try(e) => {
+                self.open(depth, "Expr.Try", span);
+                self.expr(inner, e);
+            }
+            Expr::Await(e) => {
+                self.open(depth, "Expr.Await", span);
+                self.expr(inner, e);
+            }
+            Expr::Cast { expr, ty } => {
+                self.open(depth, "Expr.Cast", span);
+                self.expr(inner, expr);
+                self.ty(inner, ty);
+            }
+            Expr::Assign { op, lhs, rhs } => {
+                self.open(depth, "Expr.Assign", span);
+                self.field("op", format_args!("{op:?}"));
+                self.expr(inner, lhs);
+                self.expr(inner, rhs);
+            }
+            Expr::StringInterp(parts) => {
+                self.open(depth, "Expr.StringInterp", span);
+                for part in parts {
+                    match part {
+                        StringPart::Lit(text) => {
+                            self.open(inner, "StringPart.Lit", None);
+                            self.text(text);
+                        }
+                        StringPart::Expr(e) => {
+                            self.open(inner, "StringPart.Expr", None);
+                            self.expr(inner + 1, e);
+                        }
+                    }
+                    self.close();
+                }
+            }
+            Expr::View(nodes) => {
+                self.open(depth, "Expr.View", span);
+                self.view_nodes(inner, nodes);
+            }
+        }
+        self.close();
+    }
+
+    fn match_arm(&mut self, depth: usize, arm: &MatchArm) {
+        self.open(depth, "MatchArm", None);
+        self.pattern(depth + 1, &arm.pattern);
+        if let Some(g) = &arm.guard {
+            self.expr(depth + 1, g);
+        }
+        self.expr(depth + 1, &arm.body);
+        self.close();
+    }
+
+    fn field_init(&mut self, depth: usize, f: &FieldInit) {
+        self.open(depth, "FieldInit", None);
+        self.name_field("name", &f.name);
+        if let Some(v) = &f.value {
+            self.expr(depth + 1, v);
+        }
+        self.close();
+    }
+
+    fn view_nodes(&mut self, depth: usize, nodes: &[Spanned<ViewNode>]) {
+        for node in nodes {
+            match &node.value {
+                ViewNode::Element {
+                    name,
+                    args,
+                    children,
+                } => {
+                    self.open(depth, "ViewNode.Element", Some(node.span));
+                    self.name_field("name", name);
+                    let some = if children.is_some() { "Some" } else { "None" };
+                    self.field("children", format_args!("{some}"));
+                    for arg in args {
+                        match arg {
+                            ViewArg::Named { name, value } => {
+                                self.open(depth + 1, "ViewArg.Named", None);
+                                self.name_field("name", name);
+                                self.expr(depth + 2, value);
+                            }
+                            ViewArg::Positional(e) => {
+                                self.open(depth + 1, "ViewArg.Positional", None);
+                                self.expr(depth + 2, e);
+                            }
+                        }
+                        self.close();
+                    }
+                    if let Some(children) = children {
+                        self.view_nodes(depth + 1, children);
+                    }
+                }
+                ViewNode::Expr(e) => {
+                    self.open(depth, "ViewNode.Expr", Some(node.span));
+                    self.expr(depth + 1, e);
+                }
+            }
+            self.close();
+        }
+    }
+
+    // --- Patterns ---
+
+    fn patterns(&mut self, depth: usize, patterns: &[Spanned<Pattern>]) {
+        for p in patterns {
+            self.pattern(depth, p);
+        }
+    }
+
+    fn pattern(&mut self, depth: usize, p: &Spanned<Pattern>) {
+        let span = Some(p.span);
+        let inner = depth + 1;
+        match &p.value {
+            Pattern::Wildcard => self.open(depth, "Pattern.Wildcard", span),
+            Pattern::Lit(lit) => {
+                self.open(depth, "Pattern.Lit", span);
+                self.literal(lit);
+            }
+            Pattern::Ident { is_mut, name } => {
+                self.open(depth, "Pattern.Ident", span);
+                self.bool_field("is_mut", *is_mut);
+                self.name_field("name", name);
+            }
+            Pattern::Binding { name, inner: pat } => {
+                self.open(depth, "Pattern.Binding", span);
+                self.name_field("name", name);
+                self.pattern(inner, pat);
+            }
+            Pattern::TupleStruct { path, fields } => {
+                self.open(depth, "Pattern.TupleStruct", span);
+                self.path_field("path", path);
+                self.patterns(inner, fields);
+            }
+            Pattern::Record { path, fields, rest } => {
+                self.open(depth, "Pattern.Record", span);
+                self.path_field("path", path);
+                self.bool_field("rest", *rest);
+                for f in fields {
+                    self.field_pat(inner, f);
+                }
+            }
+            Pattern::Tuple(items) => {
+                self.open(depth, "Pattern.Tuple", span);
+                self.patterns(inner, items);
+            }
+            Pattern::Array(items) => {
+                self.open(depth, "Pattern.Array", span);
+                self.patterns(inner, items);
+            }
+            Pattern::Or(items) => {
+                self.open(depth, "Pattern.Or", span);
+                self.patterns(inner, items);
+            }
+            Pattern::Range { lo, hi, inclusive } => {
+                self.open(depth, "Pattern.Range", span);
+                self.bool_field("inclusive", *inclusive);
+                self.pattern(inner, lo);
+                self.pattern(inner, hi);
+            }
+            Pattern::Path(path) => {
+                self.open(depth, "Pattern.Path", span);
+                self.out.push(' ');
+                self.path(path);
+            }
+        }
+        self.close();
+    }
+
+    fn field_pat(&mut self, depth: usize, f: &FieldPat) {
+        self.open(depth, "FieldPat", None);
+        self.name_field("name", &f.name);
+        if let Some(p) = &f.pattern {
+            self.pattern(depth + 1, p);
+        }
+        self.close();
+    }
+
+    // --- Types ---
+
+    fn ty(&mut self, depth: usize, t: &Spanned<Type>) {
+        let span = Some(t.span);
+        let inner = depth + 1;
+        match &t.value {
+            Type::Path { path, args } => {
+                self.open(depth, "Type.Path", span);
+                self.path_field("path", path);
+                for a in args {
+                    self.ty(inner, a);
+                }
+            }
+            Type::Ref { is_mut, inner: ty } => {
+                self.open(depth, "Type.Ref", span);
+                self.bool_field("is_mut", *is_mut);
+                self.ty(inner, ty);
+            }
+            Type::Ptr { is_mut, inner: ty } => {
+                self.open(depth, "Type.Ptr", span);
+                self.bool_field("is_mut", *is_mut);
+                self.ty(inner, ty);
+            }
+            Type::Array(ty) => {
+                self.open(depth, "Type.Array", span);
+                self.ty(inner, ty);
+            }
+            Type::Tuple(items) => {
+                self.open(depth, "Type.Tuple", span);
+                for ty in items {
+                    self.ty(inner, ty);
+                }
+            }
+            Type::Fn { params, ret } => {
+                self.open(depth, "Type.Fn", span);
+                for ty in params {
+                    self.ty(inner, ty);
+                }
+                self.ty(inner, ret);
+            }
+            Type::Optional(ty) => {
+                self.open(depth, "Type.Optional", span);
+                self.ty(inner, ty);
+            }
+            Type::Infer => self.open(depth, "Type.Infer", span),
+        }
+        self.close();
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::expr::BinOp;
+    use crate::item::Visibility;
+    use crate::{NewlineSplit, SplitKind};
+    use nova_diagnostics::FileId;
+
+    fn sp<T>(value: T, start: u32, end: u32) -> Spanned<T> {
+        Spanned::new(value, Span::new(start, end, FileId::DUMMY))
+    }
+
+    fn name(s: &str, start: u32) -> Spanned<String> {
+        sp(s.to_string(), start, start + s.len() as u32)
+    }
+
+    /// A small tree with a field of every kind: a doc, an attribute with
+    /// an argument, a field-less enum, a bool, a name, a path, a literal
+    /// of each kind with a string to escape, an absent option, an empty
+    /// list, a node without a span, and a split.
+    #[test]
+    fn a_tree_with_every_kind_of_field() {
+        let lit = |l: Literal, at: u32| sp(Expr::Lit(l), at, at + 1);
+        let body = Block {
+            stmts: vec![
+                sp(
+                    Stmt::Let {
+                        is_mut: true,
+                        pattern: sp(
+                            Pattern::Ident {
+                                is_mut: false,
+                                name: name("x", 30),
+                            },
+                            30,
+                            31,
+                        ),
+                        ty: None,
+                        init: Some(sp(
+                            Expr::Binary {
+                                op: BinOp::Add,
+                                lhs: Box::new(lit(Literal::Float(0.1), 34)),
+                                rhs: Box::new(lit(Literal::Char('é'), 38)),
+                            },
+                            34,
+                            39,
+                        )),
+                    },
+                    22,
+                    40,
+                ),
+                sp(
+                    Stmt::Expr(lit(Literal::Str("a\"b\\c\né".into()), 41)),
+                    41,
+                    42,
+                ),
+            ],
+            trailing: Some(Box::new(sp(
+                Expr::Path(Path {
+                    segments: vec![name("std", 43), name("x", 48)],
+                }),
+                43,
+                49,
+            ))),
+        };
+        let f = Function {
+            docs: vec![sp(" Adds.".to_string(), 0, 9)],
+            attrs: vec![Attribute {
+                name: name("test", 11),
+                args: vec![name("should_panic", 16)],
+                span: Span::new(10, 11, FileId::DUMMY),
+            }],
+            vis: Visibility::Pub,
+            is_async: false,
+            is_component: false,
+            name: name("f", 17),
+            generics: Vec::new(),
+            params: Vec::new(),
+            return_ty: None,
+            where_clause: Vec::new(),
+            body: sp(body, 20, 50),
+        };
+        let file = File {
+            items: vec![sp(Item::Function(f), 0, 50)],
+            newline_splits: vec![NewlineSplit {
+                at: Span::new(41, 42, FileId::DUMMY),
+                kind: SplitKind::Minus,
+            }],
+        };
+        assert_eq!(
+            dump_file(&file),
+            "(File\n\
+             \x20 (Item.Function 0..50\n\
+             \x20   (Function vis=Pub is_async=false is_component=false name=\"f\"@17..18\n\
+             \x20     (Doc 0..9 \" Adds.\")\n\
+             \x20     (Attr 10..11 \"test\"@11..15 \"should_panic\"@16..28)\n\
+             \x20     (Block 20..50\n\
+             \x20       (Stmt.Let 22..40 is_mut=true\n\
+             \x20         (Pattern.Ident 30..31 is_mut=false name=\"x\"@30..31)\n\
+             \x20         (Expr.Binary 34..39 op=Add\n\
+             \x20           (Expr.Lit 34..35 Float:3fb999999999999a)\n\
+             \x20           (Expr.Lit 38..39 Char:233)))\n\
+             \x20       (Stmt.Expr 41..42\n\
+             \x20         (Expr.Lit 41..42 Str:\"a\\x22b\\x5cc\\x0aé\"))\n\
+             \x20       (Expr.Path 43..49 \"std\"@43..46::\"x\"@48..49))))\n\
+             \x20 (Split 41..42 kind=Minus))\n"
+        );
+        assert_eq!(dump_file(&File::default()), "(File)\n");
+    }
+
+    /// Every entry of [`KINDS`] is a kind, a field-less value, a literal's
+    /// prefix or a kind and a value, and none is there twice.
+    #[test]
+    fn the_kinds_are_well_formed_and_distinct() {
+        let mut seen = std::collections::HashSet::new();
+        for k in KINDS {
+            assert!(seen.insert(k), "{k} is listed twice");
+            assert!(
+                k.starts_with('(') || k.contains('=') || k.ends_with(':'),
+                "{k}"
+            );
+        }
+        // 24 structs and `Doc`, the enums' 119 variants, then 11 `bool`
+        // fields' values but one and `children`'s two.
+        assert_eq!(KINDS.len(), 25 + 119 + 21 + 2);
+    }
+
+    /// What the declarations allow but no parser builds (see [`KINDS`]).
+    const NEVER_BUILT: &[&str] = &["(Pattern.Ident is_mut=true"];
+
+    /// [`KINDS`] is exactly what this crate's declarations imply, by
+    /// [`declared_forms`], with `Doc` added and [`NEVER_BUILT`] taken out:
+    /// a new struct, variant or `bool` field fails here until it is listed.
+    #[test]
+    fn every_declared_form_is_listed() {
+        let mut want = declared_forms();
+        want.insert("(Doc".to_string());
+        for never in NEVER_BUILT {
+            assert!(want.remove(*never), "{never} is not declared");
+        }
+        let have: std::collections::BTreeSet<String> =
+            KINDS.iter().map(|k| k.to_string()).collect();
+        let missing: Vec<&String> = want.difference(&have).collect();
+        let extra: Vec<&String> = have.difference(&want).collect();
+        assert!(
+            missing.is_empty() && extra.is_empty(),
+            "declared but not in KINDS: {missing:?}; in KINDS but not declared: {extra:?}"
+        );
+    }
+
+    /// One declaration's body: a struct's fields, or an enum's variants,
+    /// each with its named fields (none for a unit or tuple variant) and
+    /// whether it holds anything.
+    enum Decl {
+        Struct(Fields),
+        Enum(Vec<(String, bool, Fields)>),
+    }
+
+    /// Named fields: each one's name and its type's tokens joined.
+    type Fields = Vec<(String, String)>;
+
+    /// The forms [`KINDS`] lists, read from the `struct` and `enum`
+    /// declarations of this crate's sources but this file by a small
+    /// scanner ([`scan`]): a struct's kind (`Attribute` is `Attr`, `NewlineSplit`
+    /// `Split`); an enum's variants, `Literal`'s as `Variant:`, a field-less
+    /// enum's as `key=Variant` for each field of that type, any other's as
+    /// `(Enum.Variant`; and for each `bool` field, and each
+    /// `Option<Vec<…>>`, its two values after its node's kind.
+    fn declared_forms() -> std::collections::BTreeSet<String> {
+        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
+        let mut decls: Vec<(String, Decl)> = Vec::new();
+        let mut paths: Vec<_> = std::fs::read_dir(dir)
+            .expect("read nova-ast's src")
+            .map(|e| e.expect("an entry").path())
+            .filter(|p| p.extension().is_some_and(|x| x == "rs"))
+            .filter(|p| !p.ends_with("dump.rs"))
+            .collect();
+        paths.sort();
+        assert!(paths.len() >= 5, "{paths:?}");
+        for path in paths {
+            let text = std::fs::read_to_string(&path).expect("read a source");
+            decls.extend(scan(&tokens(&text)));
+        }
+        let mut forms = std::collections::BTreeSet::new();
+        let field_forms = |forms: &mut std::collections::BTreeSet<String>,
+                           kind: &str,
+                           fields: &[(String, String)]| {
+            for (name, ty) in fields {
+                let values = match ty.as_str() {
+                    "bool" => ["true", "false"],
+                    t if t.starts_with("Option<Vec<") => ["Some", "None"],
+                    _ => continue,
+                };
+                for v in values {
+                    forms.insert(format!("({kind} {name}={v}"));
+                }
+            }
+        };
+        for (name, decl) in &decls {
+            match decl {
+                Decl::Struct(fields) => {
+                    let kind = match name.as_str() {
+                        "Attribute" => "Attr",
+                        "NewlineSplit" => "Split",
+                        n => n,
+                    };
+                    forms.insert(format!("({kind}"));
+                    field_forms(&mut forms, kind, fields);
+                }
+                Decl::Enum(variants) if name == "Literal" => {
+                    for (v, _, _) in variants {
+                        forms.insert(format!("{v}:"));
+                    }
+                }
+                Decl::Enum(variants) if variants.iter().all(|(_, holds, _)| !holds) => {
+                    // The fields of this type, anywhere, give the key.
+                    let keys: Vec<&String> = decls
+                        .iter()
+                        .flat_map(|(_, d)| match d {
+                            Decl::Struct(fields) => fields.iter().collect::<Vec<_>>(),
+                            Decl::Enum(vs) => vs.iter().flat_map(|(_, _, f)| f).collect(),
+                        })
+                        .filter(|(_, ty)| ty == name)
+                        .map(|(key, _)| key)
+                        .collect();
+                    assert!(!keys.is_empty(), "no field has the type {name}");
+                    for key in keys {
+                        for (v, _, _) in variants {
+                            forms.insert(format!("{key}={v}"));
+                        }
+                    }
+                }
+                Decl::Enum(variants) => {
+                    for (v, _, fields) in variants {
+                        let kind = format!("{name}.{v}");
+                        forms.insert(format!("({kind}"));
+                        field_forms(&mut forms, &kind, fields);
+                    }
+                }
+            }
+        }
+        forms
+    }
+
+    /// `text`'s words and punctuation, line comments left out.
+    fn tokens(text: &str) -> Vec<String> {
+        let mut out = Vec::new();
+        for line in text.lines() {
+            let code = line.split("//").next().unwrap_or("");
+            let mut word = String::new();
+            for c in code.chars() {
+                if c.is_alphanumeric() || c == '_' {
+                    word.push(c);
+                    continue;
+                }
+                if !word.is_empty() {
+                    out.push(std::mem::take(&mut word));
+                }
+                if !c.is_whitespace() {
+                    out.push(c.to_string());
+                }
+            }
+            if !word.is_empty() {
+                out.push(word);
+            }
+        }
+        out
+    }
+
+    /// The index just past the bracket that closes the one at `at`. The
+    /// `>` of a `->` closes nothing.
+    fn past_close(t: &[String], at: usize) -> usize {
+        let mut depth = 0i32;
+        for (i, tok) in t.iter().enumerate().skip(at) {
+            match tok.as_str() {
+                "(" | "[" | "{" | "<" => depth += 1,
+                ">" if i > 0 && t[i - 1] == "-" => {}
+                ")" | "]" | "}" | ">" => depth -= 1,
+                _ => {}
+            }
+            if depth == 0 {
+                return i + 1;
+            }
+        }
+        panic!("an unclosed {} at token {at}", t[at]);
+    }
+
+    /// The named fields between the `{` at `at` and its `}`, `pub` and
+    /// attributes left out, each type's tokens joined.
+    fn fields(t: &[String], at: usize) -> Fields {
+        let end = past_close(t, at) - 1;
+        let mut out = Vec::new();
+        let mut i = at + 1;
+        while i < end {
+            match t[i].as_str() {
+                "#" => i = past_close(t, i + 1),
+                "pub" if t[i + 1] == "(" => i = past_close(t, i + 1),
+                "pub" | "," => i += 1,
+                _ => {
+                    assert_eq!(t[i + 1], ":", "a field at token {i}");
+                    let mut j = i + 2;
+                    let mut ty = String::new();
+                    while j < end && t[j] != "," {
+                        let k = if "([{<".contains(t[j].as_str()) {
+                            past_close(t, j)
+                        } else {
+                            j + 1
+                        };
+                        ty.extend(t[j..k].iter().map(String::as_str));
+                        j = k;
+                    }
+                    out.push((t[i].clone(), ty));
+                    i = j;
+                }
+            }
+        }
+        out
+    }
+
+    /// Each `struct` and `enum` declaration among `t`, whatever its
+    /// visibility: its name, any generic parameters and `where` clause,
+    /// then its body, `{…}`, or for a struct `;`. Any other `struct` or
+    /// `enum`, such as a tuple struct, whose fields have no names to key a
+    /// value on, or a variant it cannot read, fails here: no declaration is
+    /// passed over unread.
+    fn scan(t: &[String]) -> Vec<(String, Decl)> {
+        let mut out = Vec::new();
+        let mut i = 0;
+        while i < t.len() {
+            let keyword = t[i].as_str();
+            if keyword != "struct" && keyword != "enum" {
+                i += 1;
+                continue;
+            }
+            let at = |k: usize| t.get(k).map(String::as_str).unwrap_or("");
+            let name = at(i + 1).to_string();
+            let named = name.starts_with(|c: char| c.is_alphabetic() || c == '_');
+            assert!(named, "`{keyword}` without a name at token {i}");
+            let mut open = i + 2;
+            if at(open) == "<" {
+                open = past_close(t, open);
+            }
+            if at(open) == "where" {
+                while open < t.len() && at(open) != "{" && at(open) != ";" {
+                    open += 1;
+                }
+            }
+            match (keyword, at(open)) {
+                ("struct", ";") => {
+                    out.push((name, Decl::Struct(Vec::new())));
+                    i = open + 1;
+                }
+                ("struct", "{") => {
+                    out.push((name, Decl::Struct(fields(t, open))));
+                    i = past_close(t, open);
+                }
+                ("enum", "{") => {
+                    out.push((name, Decl::Enum(variants(t, open))));
+                    i = past_close(t, open);
+                }
+                (_, other) => panic!(
+                    "`{keyword} {name}` at token {i}: `{other}` where a body was due \
+                     (a tuple struct's fields have no names)"
+                ),
+            }
+        }
+        out
+    }
+
+    /// The variants between the `{` at `at` and its `}`: each one's name,
+    /// whether it holds anything, and its named fields.
+    fn variants(t: &[String], at: usize) -> Vec<(String, bool, Fields)> {
+        let end = past_close(t, at) - 1;
+        let mut out = Vec::new();
+        let mut j = at + 1;
+        while j < end {
+            match t[j].as_str() {
+                "#" => j = past_close(t, j + 1),
+                "," => j += 1,
+                v => {
+                    let named = v.starts_with(|c: char| c.is_alphabetic() || c == '_');
+                    assert!(named, "a variant at token {j}: `{v}`");
+                    match t[j + 1].as_str() {
+                        "(" => {
+                            out.push((v.to_string(), true, Vec::new()));
+                            j = past_close(t, j + 1);
+                        }
+                        "{" => {
+                            out.push((v.to_string(), true, fields(t, j + 1)));
+                            j = past_close(t, j + 1);
+                        }
+                        "," | "}" => {
+                            out.push((v.to_string(), false, Vec::new()));
+                            j += 1;
+                        }
+                        other => panic!("the variant `{v}` at token {j}: `{other}` after it"),
+                    }
+                }
+            }
+        }
+        out
+    }
+
+    /// The scanner reads what the hand-kept list says of a few known
+    /// declarations, so a fault in it cannot hide behind an empty set.
+    #[test]
+    fn the_scanner_reads_the_declarations() {
+        let forms = declared_forms();
+        for f in [
+            "(File",
+            "(Attr",
+            "(Split",
+            "(Expr.Cast",
+            "(ImportKind.Simple",
+            "Int:",
+            "vis=Private",
+            "kind=Value",
+            "op=ShrAssign",
+            "op=Deref",
+            "(Pattern.Record rest=false",
+            "(Type.Ptr is_mut=true",
+            "(ViewNode.Element children=None",
+        ] {
+            assert!(forms.contains(f), "{f} not read: {forms:?}");
+        }
+        assert!(!forms.contains("(Attribute") && !forms.contains("(Literal.Int"));
+        // Each declaration's shape is read, or fails: none is passed over.
+        let shown = |src: &str| -> String {
+            let decls = std::panic::catch_unwind(|| scan(&tokens(src)));
+            let Ok(decls) = decls else {
+                return "fails".to_string();
+            };
+            let fields = |f: &Fields| {
+                let f: Vec<String> = f.iter().map(|(n, t)| format!("{n}:{t}")).collect();
+                f.join(",")
+            };
+            let decls: Vec<String> = decls
+                .iter()
+                .map(|(name, decl)| match decl {
+                    Decl::Struct(f) => format!("{name}{{{}}}", fields(f)),
+                    Decl::Enum(vs) => {
+                        let vs: Vec<String> = vs
+                            .iter()
+                            .map(|(v, holds, f)| match (holds, f.is_empty()) {
+                                (false, _) => v.clone(),
+                                (true, true) => format!("{v}()"),
+                                (true, false) => format!("{v}{{{}}}", fields(f)),
+                            })
+                            .collect();
+                        format!("{name}{{{}}}", vs.join(","))
+                    }
+                })
+                .collect();
+            decls.join(" ")
+        };
+        let mut wrong = Vec::new();
+        for (src, want) in [
+            ("pub struct G<T> { a: bool }", "G{a:bool}"),
+            (
+                "pub enum E<T> { A(T), B { x: bool }, C }",
+                "E{A(),B{x:bool},C}",
+            ),
+            ("pub struct W where T: Clone { a: bool }", "W{a:bool}"),
+            ("pub(crate) struct C { pub(crate) a: bool }", "C{a:bool}"),
+            ("struct S { a: bool } enum D { A, B }", "S{a:bool} D{A,B}"),
+            (
+                "pub struct F { f: fn(Int) -> Int, b: bool }",
+                "F{f:fn(Int)->Int,b:bool}",
+            ),
+            ("pub struct U;", "U{}"),
+            ("pub struct T(pub bool);", "fails"),
+            ("pub enum N { A = 1 }", "fails"),
+        ] {
+            let got = shown(src);
+            if got != want {
+                wrong.push(format!("`{src}`: read as `{got}`, not `{want}`"));
+            }
+        }
+        assert!(wrong.is_empty(), "{wrong:#?}");
+    }
+}
diff --git a/crates/nova-ast/src/lib.rs b/crates/nova-ast/src/lib.rs
index 053c4dfce096cd2c84741713acf5cd026b9dc07d..fba2a8cd1be5af496b6bd05805ca3db107ecbe0e 100644
--- a/crates/nova-ast/src/lib.rs
+++ b/crates/nova-ast/src/lib.rs
@@ -8,7 +8,10 @@
 //! - `expr` — expressions
 //! - `pattern` — match patterns
 //! - `ty` — type expressions
+//!
+//! `dump` renders a tree as the canonical text both parsers print (ADR 0052).
 
+pub mod dump;
 pub mod expr;
 pub mod item;
 pub mod pattern;
diff --git a/crates/nova-cli/src/cmd/parse.rs b/crates/nova-cli/src/cmd/parse.rs
index 2119bd14ab0ce24ad8bb0779b4421e984b78ac61..9e3ac5e1ee5f2f933a9baf0d86bc00b00c310296 100644
--- a/crates/nova-cli/src/cmd/parse.rs
+++ b/crates/nova-cli/src/cmd/parse.rs
@@ -3,7 +3,9 @@
 //! The hidden `--dump-tokens FILE…` prints each file's token dump instead,
 //! what the self-hosted lexer's `selfhost tokens` prints (spec
 //! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
-//! §3.3, ADR 0051).
+//! §3.3, ADR 0051), and the hidden `--dump-ast FILE…` each file's AST
+//! dump, what the self-hosted parser's `selfhost ast` prints (spec §4.3,
+//! ADR 0052).
 
 use std::ffi::{OsStr, OsString};
 use std::io::Write;
@@ -35,20 +37,31 @@ pub struct ParseCmd {
     #[arg(long, hide = true, conflicts_with_all = ["ast", "check"])]
     dump_tokens: bool,
 
-    /// The files after FILE, which only `--dump-tokens` takes. Taken as
-    /// they are, an empty one included, so that without the flag `run`
-    /// refuses them in clap's words, and with it an empty one is a file
-    /// that is not there, as N reads it.
+    /// Print the AST dump of FILE and of every further file given, for the
+    /// self-hosted parser's differential test. Hidden: it carries no
+    /// stability promise.
+    #[arg(long, hide = true, conflicts_with_all = ["ast", "check", "dump_tokens"])]
+    dump_ast: bool,
+
+    /// The files after FILE, which only `--dump-tokens` and `--dump-ast`
+    /// take. Taken as they are, an empty one included, so that without
+    /// either flag `run` refuses them in clap's words, and with one an
+    /// empty one is a file that is not there, as N reads it.
     #[arg(hide = true, value_parser = OsStringValueParser::new().map(PathBuf::from))]
     more: Vec<PathBuf>,
 }
 
 pub fn run(cmd: ParseCmd) -> Result<()> {
-    if cmd.dump_tokens {
+    if cmd.dump_tokens || cmd.dump_ast {
         let files: Vec<&Path> = std::iter::once(cmd.file.as_path())
             .chain(cmd.more.iter().map(PathBuf::as_path))
             .collect();
-        let code = dump_tokens(&files)?;
+        let dump = if cmd.dump_ast {
+            nova_parser::dump_ast
+        } else {
+            nova_lexer::dump_tokens
+        };
+        let code = dump_each(&files, dump)?;
         if code != 0 {
             std::process::exit(code);
         }
@@ -56,7 +69,7 @@ pub fn run(cmd: ParseCmd) -> Result<()> {
     }
     if let Some(extra) = cmd.more.first() {
         // What clap said before `more` existed, without naming the hidden
-        // flag: its "unexpected argument" error, built as clap builds it,
+        // flags: its "unexpected argument" error, built as clap builds it,
         // so a terminal shows it in the same colours, with its usage line
         // naming the command as clap does (`parse_bin_name`).
         use clap::error::{ContextKind, ContextValue, ErrorKind};
@@ -156,17 +169,18 @@ fn parse_bin_name(arg0: Option<OsString>) -> String {
     }
 }
 
-/// `--dump-tokens`: for each file, `== FILE` (the path as given), then its
-/// token dump; for a file that cannot be read, `== FILE` then `IOERR` and
-/// its kind, after which the rest are dumped and the exit code is 2.
-fn dump_tokens(files: &[&Path]) -> Result<i32> {
+/// `--dump-tokens` and `--dump-ast`: for each file, `== FILE` (the path as
+/// given), then its `dump`; for a file that cannot be read, `== FILE` then
+/// `IOERR` and its kind, after which the rest are dumped and the exit code
+/// is 2.
+fn dump_each(files: &[&Path], dump: fn(&str) -> String) -> Result<i32> {
     let stdout = std::io::stdout();
     let mut out = stdout.lock();
     let mut code = 0;
     for file in files {
         writeln!(out, "== {}", file.display())?;
         match std::fs::read_to_string(file) {
-            Ok(source) => out.write_all(nova_lexer::dump_tokens(&source).as_bytes())?,
+            Ok(source) => out.write_all(dump(&source).as_bytes())?,
             Err(e) => {
                 writeln!(out, "IOERR {}", io_kind_name(e.kind()))?;
                 code = 2;
diff --git a/crates/nova-parser/src/dump.rs b/crates/nova-parser/src/dump.rs
new file mode 100644
index 0000000000000000000000000000000000000000..d443fcd1d9eb07dbb849eaf644aa9652ab84c92c
--- /dev/null
+++ b/crates/nova-parser/src/dump.rs
@@ -0,0 +1,100 @@
+//! The AST dump of a source (spec
+//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+//! §4.3, ADR 0052): its tree, as `nova_ast::dump` writes it, then its
+//! errors, one per line, in the order they were found:
+//!
+//! - `ERR start end message` for each lex error, all of them first, as the
+//!   callers render them before the parse errors;
+//! - `PARSE start end message` for each parse error, then `NOTE note` for
+//!   each of its notes. The code is not written: P0002's message names it,
+//!   and every other parse error is P0001.
+//!
+//! A message or a note writes each character below 0x20 and 0x7F as
+//! `\xHH`, and every other as it is. The self-hosted parser prints the same
+//! bytes (`selfhost ast`), and `nova parse --dump-ast` a `== FILE` header
+//! before them.
+
+use std::fmt::Write;
+
+use nova_ast::dump::{dump_file, escape_message};
+use nova_diagnostics::FileId;
+
+use crate::parse;
+
+/// The AST dump of `source` (module docs).
+pub fn dump_ast(source: &str) -> String {
+    let (tokens, lex_errors) = nova_lexer::lex(source, FileId::DUMMY);
+    let (file, errors) = parse(&tokens, FileId::DUMMY);
+    let mut out = dump_file(&file.unwrap_or_default());
+    for e in &lex_errors {
+        let span = e.span();
+        let _ = write!(out, "ERR {} {} ", span.start, span.end);
+        escape_message(&mut out, &e.to_string());
+        out.push('\n');
+    }
+    for e in &errors {
+        let span = e.span();
+        let _ = write!(out, "PARSE {} {} ", span.start, span.end);
+        escape_message(&mut out, &e.to_string());
+        out.push('\n');
+        for note in e.notes() {
+            out.push_str("NOTE ");
+            escape_message(&mut out, &note);
+            out.push('\n');
+        }
+    }
+    out
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    #[test]
+    fn the_errors_follow_the_tree() {
+        let out = dump_ast("fn f() { \"x }\nfn g( {}\n");
+        let errors: Vec<&str> = out
+            .lines()
+            .filter(|l| !l.starts_with('(') && !l.starts_with(' '))
+            .collect();
+        assert_eq!(
+            errors,
+            [
+                "ERR 10 23 unterminated string literal",
+                "PARSE 23 23 unexpected end of file",
+                "PARSE 23 23 expected `}` (in function body), found end of file",
+            ],
+            "{out}"
+        );
+        // A note follows its error.
+        let out =
+            dump_ast("fn f() {\n    match x {\n        A => g\n        (a + 1) => h\n    }\n}\n");
+        let lines: Vec<&str> = out.lines().collect();
+        let at = lines
+            .iter()
+            .position(|l| l.starts_with("PARSE "))
+            .expect("an error");
+        assert!(
+            lines[at + 1].starts_with("NOTE a line that starts with `(`"),
+            "{out}"
+        );
+        // The newline rule's splits close the tree.
+        let out = dump_ast("fn f() {\n    g\n    (x)\n}\n");
+        assert!(out.ends_with("\n  (Split 19..20 kind=Call))\n"), "{out}");
+    }
+
+    #[test]
+    fn a_file_nested_too_deeply_is_one_p0002() {
+        let src = format!("fn f() {{ {}1{} }}\n", "(".repeat(200), ")".repeat(200));
+        let out = std::thread::Builder::new()
+            .stack_size(64 << 20)
+            .spawn(move || dump_ast(&src))
+            .expect("a thread")
+            .join()
+            .expect("the dump");
+        assert_eq!(
+            out,
+            "(File)\nPARSE 138 139 this is nested too deeply (more than 128 levels)\n"
+        );
+    }
+}
diff --git a/crates/nova-parser/src/lib.rs b/crates/nova-parser/src/lib.rs
index 841841218ea1335d0c69fd4b373a37f0c8ff5569..59ca954ed968bb6ace3cf7f813b83dc6890d99d8 100644
--- a/crates/nova-parser/src/lib.rs
+++ b/crates/nova-parser/src/lib.rs
@@ -21,14 +21,17 @@
 //! let (ast, _parse_errs) = parse(&tokens, file_id);
 //! ```
 
+mod dump;
 mod error;
 mod grammar;
 
+pub use dump::dump_ast;
 pub use error::ParseError;
 
 use nova_ast::File;
 use nova_diagnostics::{FileId, Spanned};
 use nova_lexer::Tokens;
+
 /// How deeply source may nest: past it, the parser reports
 /// [`ParseError::TooDeep`] (P0002) and stops (spec
 /// `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
````

### Task 4: The error cases and the deep generator

Spec §3.5 generator 3, §4.4, §4.6; decisions 5, 11, 24 and 25.

**Files:**
- Create: 262 files under `crates/nova-cli/tests/selfhost/cases/`, each
  `<test file>__<test>[_n].nova`, `chains_and_casts_at_the_limit.nova`,
  `abandoned_attempts_past_the_limit.nova` and
  `a_rolled_back_crossing_that_stands.nova`
- Modify: `crates/nova-cli/tests/selfhost/fuzz.rs` (`DEEP_KINDS`,
  `DEEP_DEPTHS`, `deep`, `deeps`; nine edge inputs)
- Test: `selfhost.rs` (two)

- [ ] **Step 1: The tests.** `patch_block t4-tests`. The patch holds:
  - the 262 error cases, which `the_fixed_cases_agree` runs through the
    token differential with the rest, and one by hand that the mutants
    asked for: chains followed by a lower operator and 128 casts, at the
    limit but not past it (decision 49), and the rollback's two: three
    attempts rolled back past the limit whose expressions parse, then an
    error that stays P0001, and a cast's generic type 200 deep whose
    crossing stands (decision 19);
  - `selfhost.rs`: `DEEP_SEEDS` and `DEEP_MIXED`; `the_deep_inputs_agree`
    (seeds 201 and 202, 90 inputs each, through `agree_on_inputs`, the
    token dump alone until Task 6); `nova_on` and
    `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack` (seed
    201's inputs through `nova parse --check`, and `nova check` at 128
    and 129: no overflow, no error to 128 but the nested items' E0900,
    one P0002 past it).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which
  ends with `Finished`.
  - `SH`. Expected: it does not compile, ``error[E0425]: cannot find function `deeps` in module `fuzz` ``, twice,
    and ``error: could not compile `nova-cli` (test "selfhost") due to 2 previous errors`` (decision 47).
- [ ] **Step 3: The code.** `patch_block t4-code`: `fuzz.rs`'s module doc
  names generator 3; `edges` gains the six sources with a control
  character and the three 4,000-character numbers; `DEEP_KINDS`, the 14
  kinds and `mixed`; `DEEP_DEPTHS`, 126 to 130; `deep(rng, kind, depth)`;
  `deeps(seed, mixed)`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH`. Expected: `26 passed; 0 failed`.
  - `S` (5 min 28 s). Expected: `2838 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `cli tests: the error cases and the deep inputs, through the token dump`.

**Patch `t4-tests`:**

<!-- patch t4-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index be7ee63b3ad88b376dff4ef3cae677221676561e..31b44f26cd0a4708567ce5d1e559197f9faf6f52 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -48,6 +48,10 @@ const SOUP_SEEDS: [u64; 3] = [1, 2, 3];
 const SOUP_COUNT: usize = 2000;
 const MUTATION_SEEDS: [u64; 3] = [101, 102, 103];
 const MUTATION_COUNT: usize = 250;
+/// The deep inputs' seeds, and how many of each seed's are mixed: with
+/// every other kind at each of five depths, 90 inputs a seed.
+const DEEP_SEEDS: [u64; 2] = [201, 202];
+const DEEP_MIXED: usize = 20;
 
 /// The beginning of each of the seven lex errors' messages
 /// (nova-lexer's `error.rs`).
@@ -1006,6 +1010,77 @@ fn every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases() {
     }
 }
 
+#[test]
+fn the_deep_inputs_agree() {
+    for seed in DEEP_SEEDS {
+        let inputs: Vec<String> = fuzz::deeps(seed, DEEP_MIXED)
+            .into_iter()
+            .map(|(_, _, src)| src)
+            .collect();
+        agree_on_inputs(&format!("deep-{seed}"), &inputs);
+    }
+}
+
+/// `nova` with `args` and `file`, in `dir`: its exit code and stderr.
+fn nova_on(dir: &Path, args: &[&str], file: &str) -> (Option<i32>, String) {
+    let out = output_within(
+        Command::new(nova())
+            .current_dir(dir)
+            .env("NOVA_HOME", dir.join("home"))
+            .args(args)
+            .arg(file)
+            .stdin(Stdio::null()),
+        CHILD_LIMIT,
+        &format!("nova {} {file}", args.join(" ")),
+    )
+    .unwrap_or_else(|e| panic!("{e}"));
+    (
+        out.status.code(),
+        String::from_utf8_lossy(&out.stderr).into_owned(),
+    )
+}
+
+/// S0 meets the depth limit, not its stack, on every deep input, on every
+/// system (spec §4.4, §4.8): `nova parse --check` passes each at 128 levels
+/// and below and gives one P0002 past it, as `nova check` does at 128 and
+/// 129, every stage after the parser included.
+#[test]
+fn s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack() {
+    let dir = fresh_dir("deep-s0");
+    for (i, (kind, depth, src)) in fuzz::deeps(DEEP_SEEDS[0], DEEP_MIXED)
+        .into_iter()
+        .enumerate()
+    {
+        let file = format!("{kind}-{depth}-{i}.nova");
+        std::fs::write(dir.join(&file), &src).expect("write a deep input");
+        let mut commands: Vec<&[&str]> = vec![&["parse", "--check"]];
+        if depth == 128 || depth == 129 {
+            commands.push(&["check"]);
+        }
+        for args in commands {
+            let (code, stderr) = nova_on(&dir, args, &file);
+            let what = format!("nova {} {file}", args.join(" "));
+            assert!(!stderr.contains("overflowed"), "{what}:\n{stderr}");
+            let p0002 = stderr
+                .matches("error[P0002]: this is nested too deeply")
+                .count();
+            if depth <= 128 {
+                // Nested items are E0900 to the checker; nothing else is an
+                // error.
+                let errors_ok =
+                    stderr.matches("error[").count() == stderr.matches("error[E0900]").count();
+                let code_ok = code == Some(0) || (kind == "items" && code == Some(1));
+                assert!(
+                    p0002 == 0 && errors_ok && code_ok,
+                    "{what}: {code:?}\n{stderr}"
+                );
+            } else {
+                assert!(p0002 == 1 && code == Some(1), "{what}: {code:?}\n{stderr}");
+            }
+        }
+    }
+}
+
 /// A panic on the pipeline's thread is printed once, naming the thread
 /// `main` as before, and `nova` exits 101: here `println!` into a full
 /// device, Linux's `/dev/full`.
diff --git a/crates/nova-cli/tests/selfhost/cases/a_rolled_back_crossing_that_stands.nova b/crates/nova-cli/tests/selfhost/cases/a_rolled_back_crossing_that_stands.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d5f1684f0e459ef33c9952dd00ff0995a6c27d7e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/a_rolled_back_crossing_that_stands.nova
@@ -0,0 +1,8 @@
+// A cast's type 200 generic lists deep: each list's attempt is rolled
+// back from the innermost out, and the tokens, read again as comparisons,
+// fail before the token where the first attempt crossed the depth limit,
+// so that crossing stands: one P0002 there (ADR 0052). A parser that drops
+// it reports the comparisons instead.
+fn f(x: Int) -> Bool {
+    x as V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<V<I>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/abandoned_attempts_past_the_limit.nova b/crates/nova-cli/tests/selfhost/cases/abandoned_attempts_past_the_limit.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bc2eabe1534da99f111296147033b83c2865e34c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/abandoned_attempts_past_the_limit.nova
@@ -0,0 +1,24 @@
+// A type after a cast and `<` is first tried as generic arguments, a level
+// deeper than the expression the tokens then are, and after two casts two
+// levels. At 127 and 126 parentheses that attempt crosses the depth limit
+// and is rolled back, and the tokens read again as an expression do not
+// cross it: no P0002, and no error the attempt pushed after it crossed
+// (the tuple type's missing `)`). The crossing waits until its token is
+// read, so the error in `k` is the file's only one. A parser whose
+// rollback keeps the crossing, or keeps it waiting, reports P0002 here
+// (ADR 0052).
+fn f(x: Int, a: Int) -> Bool {
+    x as Int < (((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((a)))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+}
+
+fn g(x: Int, a: Int, b: Int) -> Bool {
+    x as Int < (b, ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((a)))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+}
+
+fn h(x: Int, a: Int) -> Bool {
+    x as A as B < ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((a))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+}
+
+fn k() {
+    let = 1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/chains_and_casts_at_the_limit.nova b/crates/nova-cli/tests/selfhost/cases/chains_and_casts_at_the_limit.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4008267d35ce89678c576c7d8cf68046dfe00358
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/chains_and_casts_at_the_limit.nova
@@ -0,0 +1,21 @@
+// Chains of every binary precedence and a postfix chain, each followed by
+// an operator of the next lower precedence whose operand is 100
+// parentheses deep; a view whose first child is a postfix chain and whose
+// second's argument is 100 deep; and 128 casts. Each statement is at most
+// 102 levels deep, and 128 for the casts, so a parser that does not lower
+// the depth after a chain, or counts a cast's type one level deeper,
+// reports P0002 here (ADR 0052).
+fn f() {
+    let a0 = 1 as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int * ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    let a1 = 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 * 1 + ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    let a2 = 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 << ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    let a3 = 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 << 1 & ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    let a4 = 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 & 1 ^ ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    let a5 = 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 ^ 1 | ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    let a6 = 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 | 1 < ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    let a7 = true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true && true || ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    let a8 = true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true || true .. ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    let a9 = r.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x * ((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))
+    view { a.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x g(((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((((1))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))))) }
+    let c = 0 as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int as Int
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/keywords__every_keyword_lexes_as_itself_not_as_an_identifier.nova b/crates/nova-cli/tests/selfhost/cases/keywords__every_keyword_lexes_as_itself_not_as_an_identifier.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b4f5270dd39aea82dc580d4a65f2b159a98fb808
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/keywords__every_keyword_lexes_as_itself_not_as_an_identifier.nova
@@ -0,0 +1,32 @@
+let
+mut
+const
+fn
+return
+if
+else
+while
+for
+in
+break
+continue
+match
+type
+record
+trait
+impl
+import
+module
+pub
+async
+await
+extern
+unsafe
+true
+false
+as
+is
+where
+with
+self
+Self
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break_1.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..26349a7518a6f6f28c7d776c664c62e208462fe9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break_1.nova
@@ -0,0 +1,2 @@
+a /* x
+ y */ b
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break_2.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..099dbf4bd95ac75609cfa878dcdff4ddd1694645
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_block_comment_marks_the_token_after_it_only_when_it_holds_a_line_break_2.nova
@@ -0,0 +1 @@
+a /* x */ b
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_comment_inside_an_interpolation_hole_is_captured.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_comment_inside_an_interpolation_hole_is_captured.nova
new file mode 100644
index 0000000000000000000000000000000000000000..409775665d9651a555067e70c66c9876eebf0745
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_comment_inside_an_interpolation_hole_is_captured.nova
@@ -0,0 +1 @@
+let s = "${x /* c */}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_comment_marker_inside_a_string_is_not_a_comment.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_comment_marker_inside_a_string_is_not_a_comment.nova
new file mode 100644
index 0000000000000000000000000000000000000000..13c12a94f4d0b4f911d3486d3b93477034f80404
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_comment_marker_inside_a_string_is_not_a_comment.nova
@@ -0,0 +1 @@
+let u = "http://example.com/*x*/" // real
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_after_a_dot_splits_into_two_indexes.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_after_a_dot_splits_into_two_indexes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..232177a757f0cfeed2a7160b2dd36c1a3efe0bb7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_after_a_dot_splits_into_two_indexes.nova
@@ -0,0 +1 @@
+t.1.0
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_1.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..07d0c659ce6b62a86a24f28a4e4f3d093a7b3017
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_1.nova
@@ -0,0 +1 @@
+0..1
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_2.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2ecbc932ba7bf6d5aca9615d973bb7407bafc602
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_2.nova
@@ -0,0 +1 @@
+0..1.5
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_3.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9f8e9b69a33f4e8067d5b21661a35d8856758aba
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_3.nova
@@ -0,0 +1 @@
+1.0
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_4.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..afaf360d37fb71bcfa8cc082882f910ac2628bda
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_4.nova
@@ -0,0 +1 @@
+1.0.0
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_5.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d919f2b01bf79346e1abbc3ea8db3bd4eb236511
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_elsewhere_is_not_split_5.nova
@@ -0,0 +1 @@
+x = 1.5
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole_1.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0c9d199139021d3134cab2b9db4d974f54090ba5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole_1.nova
@@ -0,0 +1 @@
+t.1.0e3
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole_2.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..184fb415c5483daae9b69c8e7e0d18abc800a9f0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole_2.nova
@@ -0,0 +1 @@
+t.1_0.2
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole_3.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0a492d62a17b83b5aa4a89bbea48bb57cf20ff6f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_float_with_an_underscore_or_an_exponent_after_a_dot_is_left_whole_3.nova
@@ -0,0 +1 @@
+t.1e3
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_line_break_inside_a_string_marks_nothing.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_line_break_inside_a_string_marks_nothing.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c3018da3aafaba075f55a073db721f841b261f03
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_line_break_inside_a_string_marks_nothing.nova
@@ -0,0 +1,2 @@
+f("a
+b") (c)
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_line_comment_marks_the_token_after_it.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_line_comment_marks_the_token_after_it.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5f02f7fba2cb59158ded9154943988451fe11b7a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_line_comment_marks_the_token_after_it.nova
@@ -0,0 +1,2 @@
+a // note
+b
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_tuple_index_is_an_integer_after_a_dot.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_tuple_index_is_an_integer_after_a_dot.nova
new file mode 100644
index 0000000000000000000000000000000000000000..04d521e905dc7dbf0300d4a229d97dc42104d246
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__a_tuple_index_is_an_integer_after_a_dot.nova
@@ -0,0 +1 @@
+t.0
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__an_unterminated_block_comment_is_an_error.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__an_unterminated_block_comment_is_an_error.nova
new file mode 100644
index 0000000000000000000000000000000000000000..37bc6dd3f16cce2a89097a4679e7cb8d95fe4e99
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__an_unterminated_block_comment_is_an_error.nova
@@ -0,0 +1 @@
+fn main() {} /* never closed
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__char_literal.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__char_literal.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9b81b76fc1925632e6c4982ff29091d310beb242
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__char_literal.nova
@@ -0,0 +1 @@
+'a' '\n' '\t'
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__comment_before_string_literal_1.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__comment_before_string_literal_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..72b4fc78625cadfcd6255691ec82d6b3fdb761a4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__comment_before_string_literal_1.nova
@@ -0,0 +1,2 @@
+/* c */ r"a
+"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__comment_before_string_literal_2.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__comment_before_string_literal_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..34c115ade8e52a8dc44820280c460eca8ffbc0b3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__comment_before_string_literal_2.nova
@@ -0,0 +1 @@
+let m = /* c */ "hi"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__doc_comment.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__doc_comment.nova
new file mode 100644
index 0000000000000000000000000000000000000000..de7e3eab440c79ab7f733677a13a0a1e8f247e73
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__doc_comment.nova
@@ -0,0 +1,2 @@
+/// This is a doc comment
+fn foo() {}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__doc_comment_is_not_skipped.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__doc_comment_is_not_skipped.nova
new file mode 100644
index 0000000000000000000000000000000000000000..72fbf0d8969950cdd7ccbc5c34d43013b4c028c6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__doc_comment_is_not_skipped.nova
@@ -0,0 +1,2 @@
+/// docs
+fn f() {}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__eof_always_last.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__eof_always_last.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3b0c82647f2a365647b6b7b19fe479acaabad134
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__eof_always_last.nova
@@ -0,0 +1 @@
+let x = 1
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__float_literals.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__float_literals.nova
new file mode 100644
index 0000000000000000000000000000000000000000..093115a842b01ffd56fc3366bca20576f1978bd5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__float_literals.nova
@@ -0,0 +1 @@
+1.5 1.5e3 1.5e-3 1_000.5
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__four_slashes_make_a_plain_comment.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__four_slashes_make_a_plain_comment.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2741bad8b0fb67064d6757fcd195beff53cf2ae4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__four_slashes_make_a_plain_comment.nova
@@ -0,0 +1,2 @@
+//// ----
+fn f() {}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__identifier.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__identifier.nova
new file mode 100644
index 0000000000000000000000000000000000000000..381d330edca07b5b12aa291748e88c48bdce0f54
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__identifier.nova
@@ -0,0 +1 @@
+hello_world _private __init camelCase PascalCase
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__integer_literals.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__integer_literals.nova
new file mode 100644
index 0000000000000000000000000000000000000000..78e7417eddab30a8643dafe1c17a69c12d1f01c5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__integer_literals.nova
@@ -0,0 +1 @@
+0 42 1_000_000 0xff 0b1010 0o755
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_1.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3cc762b5501e0d558314aecc7aed03c835319ff9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_1.nova
@@ -0,0 +1 @@
+""
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_2.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..46edbc896847e091e47513f28fe776ecb2e05237
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_2.nova
@@ -0,0 +1 @@
+"${a}${b}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_3.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b68d568e2a3241b23c7d09424baf2ac3e2deaaa0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_3.nova
@@ -0,0 +1 @@
+"${n}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_4.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..38362c3007f696c86437c91ed4b8fcbc9d8ee7c0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_4.nova
@@ -0,0 +1 @@
+"a${x}b${y}c"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_5.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ebfd27034afc1ee7b693d88747feb8d5422e5b2a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_5.nova
@@ -0,0 +1 @@
+"cost: $5, $ and }"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_6.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_6.nova
new file mode 100644
index 0000000000000000000000000000000000000000..005a9e62abf40aa239512fe62bb17770a63076ab
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_basics_still_lex_6.nova
@@ -0,0 +1 @@
+fn f() { "${a}" }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_balances_a_block_expression.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_balances_a_block_expression.nova
new file mode 100644
index 0000000000000000000000000000000000000000..662e2121f57d5245a1c9b7156b674a7594405fad
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_balances_a_block_expression.nova
@@ -0,0 +1 @@
+"${if a { 1 } else { 2 }}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_balances_braces_of_a_record_literal.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_balances_braces_of_a_record_literal.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a24da3869216f1903a9d3861ccdd6810410a785e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_balances_braces_of_a_record_literal.nova
@@ -0,0 +1 @@
+"${f(R { v: 1 })}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_balances_nested_braces.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_balances_nested_braces.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1a61dabc89199a63cf002b46be0990d7b5526fed
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_balances_nested_braces.nova
@@ -0,0 +1 @@
+"${f(R { v: G { w: 1 } })}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_a_nested_string_1.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_a_nested_string_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4452a77982c3a6a047789f643e2442521836e7ce
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_a_nested_string_1.nova
@@ -0,0 +1 @@
+"${g("a\"}\\")}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_a_nested_string_2.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_a_nested_string_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3e5f686e66f72fe9362ac1992a6caa621fac954e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_a_nested_string_2.nova
@@ -0,0 +1 @@
+"${g("}")}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_char_and_raw_literals_1.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_char_and_raw_literals_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7f7970958735024d72f921a2240c175e334d4a40
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_char_and_raw_literals_1.nova
@@ -0,0 +1 @@
+"${g('}')}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_char_and_raw_literals_2.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_char_and_raw_literals_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4e85fee8f6a0ed2257c7cf6f7ad860f6b0204f89
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_ignores_braces_inside_char_and_raw_literals_2.nova
@@ -0,0 +1 @@
+"${g(r"}")}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_inside_a_nested_string_tracks_its_own_brace_depth.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_inside_a_nested_string_tracks_its_own_brace_depth.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e6634c7e9e7803cc20d3ad309cba023a9d9427b5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_hole_inside_a_nested_string_tracks_its_own_brace_depth.nova
@@ -0,0 +1 @@
+"${f("${g(R { v: 1 })}")}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_preserves_leading_whitespace_after_close.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_preserves_leading_whitespace_after_close.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d78d79cdacbd30eac9ada2cd49854ce4f86731e8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__interpolation_preserves_leading_whitespace_after_close.nova
@@ -0,0 +1 @@
+"${a} b ${c}"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__keywords.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__keywords.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ea1cacbca662b63e0a8413f996fc4a389c93c44f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__keywords.nova
@@ -0,0 +1 @@
+let mut const fn return if else while for in break continue match type record trait impl import module pub async await extern unsafe true false as is where with self Self
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__lex_with_comments_returns_each_comment_in_order.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__lex_with_comments_returns_each_comment_in_order.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8601d324b3e65deecd40c2593dd8ecdf501d6c00
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__lex_with_comments_returns_each_comment_in_order.nova
@@ -0,0 +1,2 @@
+// one
+fn /* two */ main() { f(/* three */"s") } // four
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__line_and_block_comments_are_skipped.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__line_and_block_comments_are_skipped.nova
new file mode 100644
index 0000000000000000000000000000000000000000..68aace274053bb5d4486c7227b65afa37f338846
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__line_and_block_comments_are_skipped.nova
@@ -0,0 +1,3 @@
+// leading
+fn /* mid */ main() {}
+// trailing
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__line_comment_skipped.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__line_comment_skipped.nova
new file mode 100644
index 0000000000000000000000000000000000000000..639a21883c02560cedcb42e85ae0c0c2014889bb
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__line_comment_skipped.nova
@@ -0,0 +1,2 @@
+let x // this is a comment
+= 1
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__only_plain_decimal_integers_are_marked_as_tuple_indexes.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__only_plain_decimal_integers_are_marked_as_tuple_indexes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..677c9fdc646f4e50736525db3e31b7e4fb64680e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__only_plain_decimal_integers_are_marked_as_tuple_indexes.nova
@@ -0,0 +1 @@
+t.0 t.7 t.12 t.01 t.1_0 t.0x1 t.00 t.0b1 t.0o7 x
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__operators.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__operators.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c8cc4eeaf8d96349878faf49b266ee4ddd05a2e3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__operators.nova
@@ -0,0 +1 @@
+( ) [ ] { } , ; : :: -> => . .. ..= @ ? = == != < <= > >= && || ! + - * / % += -= *= /= %= & | ^ ~ << >> &= |= ^= <<= >>=
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__raw_string.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__raw_string.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a81c51c20fe06bb0f446200d5e99b40fa4121eb7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__raw_string.nova
@@ -0,0 +1 @@
+r#"raw "quoted" string"#
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__string_escapes.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__string_escapes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e581136a2b86b2c27cd9445cf78303b752cfd677
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__string_escapes.nova
@@ -0,0 +1 @@
+"tab:\t newline:\n quote:\" backslash:\\"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__string_interpolation_simple.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__string_interpolation_simple.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3f35294aca8ccd62cb9ee34f3df7e8d36da73fa6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__string_interpolation_simple.nova
@@ -0,0 +1 @@
+"Hello, ${name}!"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__string_simple.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__string_simple.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cfcb15cf66c9fded1f7ef15784f493379aa60489
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__string_simple.nova
@@ -0,0 +1 @@
+"hello world"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_first_token_is_marked_only_after_a_line_break_1.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_first_token_is_marked_only_after_a_line_break_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..09f70f25a7b8f914e614a74670a154bd1b8d2206
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_first_token_is_marked_only_after_a_line_break_1.nova
@@ -0,0 +1,3 @@
+
+
+x
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_first_token_is_marked_only_after_a_line_break_2.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_first_token_is_marked_only_after_a_line_break_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c1b0730e0133447badcfd47fd144e254807b06e1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_first_token_is_marked_only_after_a_line_break_2.nova
@@ -0,0 +1 @@
+x
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_marks_leave_the_tokens_as_they_were.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_marks_leave_the_tokens_as_they_were.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7e1532c815563d1d9bdbfc649f310fd4f2833a48
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_marks_leave_the_tokens_as_they_were.nova
@@ -0,0 +1,4 @@
+fn main() {
+    let x = 1
+    -x
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_split_chains.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_split_chains.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e56615185d6d766adc09f046b31f60f0680dc8ef
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_split_chains.nova
@@ -0,0 +1 @@
+t.0.1.2
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_split_follows_the_source_text_not_the_value.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_split_follows_the_source_text_not_the_value.nova
new file mode 100644
index 0000000000000000000000000000000000000000..70ecf217a5e2bfdc4409f1be9f771f08d5323bf6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__the_split_follows_the_source_text_not_the_value.nova
@@ -0,0 +1 @@
+t.1.10
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__unterminated_interpolation_hole_is_an_error_not_a_hang_1.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__unterminated_interpolation_hole_is_an_error_not_a_hang_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1fb789755a78bec27c43cf82b1b7fd0a299ab019
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__unterminated_interpolation_hole_is_an_error_not_a_hang_1.nova
@@ -0,0 +1 @@
+"${
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/lexer_tests__unterminated_interpolation_hole_is_an_error_not_a_hang_2.nova b/crates/nova-cli/tests/selfhost/cases/lexer_tests__unterminated_interpolation_hole_is_an_error_not_a_hang_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a0954b9abb0d33f9273a8dcd13134622e0206008
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/lexer_tests__unterminated_interpolation_hole_is_an_error_not_a_hang_2.nova
@@ -0,0 +1 @@
+"${f(R { v: 1 }"
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_before_a_token_no_expression_starts_with_reads_as_before_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_before_a_token_no_expression_starts_with_reads_as_before_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9dede263325ba6b4b47a3d9c651a8d97a98c2c44
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_before_a_token_no_expression_starts_with_reads_as_before_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+g(return
+)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_before_a_token_no_expression_starts_with_reads_as_before_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_before_a_token_no_expression_starts_with_reads_as_before_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..647e83da97175683209f5e77dc67208baac30096
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_before_a_token_no_expression_starts_with_reads_as_before_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+let v = [return
+]
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_ending_an_arm_records_no_split.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_ending_an_arm_records_no_split.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f4a166ce3441c777aaba3814f272d885d9e38ddc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_ending_an_arm_records_no_split.nova
@@ -0,0 +1,7 @@
+fn f() {
+match x {
+    0 => return
+    (1) => 2
+    _ => 3
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_in_a_match_arm_ends_the_arm.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_in_a_match_arm_ends_the_arm.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5aa3885b871ff7a463ad49616fc5a799dc3744c5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_in_a_match_arm_ends_the_arm.nova
@@ -0,0 +1,6 @@
+fn f() {
+match x {
+    A => return
+    B => 1
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_in_a_view_child_ends_the_child.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_in_a_view_child_ends_the_child.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e258dd770964bfb216d2c76664d4600848039037
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bare_return_in_a_view_child_ends_the_child.nova
@@ -0,0 +1,6 @@
+fn f() -> Html {
+    view {
+        return
+        (1)
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_block_comment_that_holds_a_line_break_is_one_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_block_comment_that_holds_a_line_break_is_one_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2e5b2284f3cfeafdeb6797377de6c39cf8ada233
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_block_comment_that_holds_a_line_break_is_one_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+g /* a
+ b */ (x)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_block_comment_that_holds_a_line_break_is_one_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_block_comment_that_holds_a_line_break_is_one_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..64002c4d52aaba66012f4a8157cfb178df1e35d4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_block_comment_that_holds_a_line_break_is_one_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+g /* a */ (x)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_block_inside_parentheses_turns_the_rule_back_on.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_block_inside_parentheses_turns_the_rule_back_on.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2c933f91316863801d79a2848c1ef336b701bbd4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_block_inside_parentheses_turns_the_rule_back_on.nova
@@ -0,0 +1,6 @@
+fn f() {
+let v = g({
+    h
+    (1)
+})
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_brace_on_a_new_line_keeps_its_meaning.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_brace_on_a_new_line_keeps_its_meaning.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7c2749afdfcec2dfa188d8402f84bb6225ee65fd
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_brace_on_a_new_line_keeps_its_meaning.nova
@@ -0,0 +1,4 @@
+fn f() {
+let r = R
+{ a: 1 }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bracket_after_a_while_begins_a_statement.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bracket_after_a_while_begins_a_statement.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5d9e3920b739992177ad7117e6738187af6848a9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bracket_after_a_while_begins_a_statement.nova
@@ -0,0 +1,4 @@
+fn f() {
+while c { g() }
+[1]
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bracket_at_a_line_start_begins_a_statement_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bracket_at_a_line_start_begins_a_statement_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..71486a69ad6bcb3d35defd9077e80ce21370106b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bracket_at_a_line_start_begins_a_statement_1.nova
@@ -0,0 +1,3 @@
+fn f() {
+a[0]
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bracket_at_a_line_start_begins_a_statement_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bracket_at_a_line_start_begins_a_statement_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1ad0b94353b353f0d24228e7280421ee8a6f49df
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_bracket_at_a_line_start_begins_a_statement_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+g
+[1, 2]
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_closures_bare_return_before_a_parenthesis_or_bracket_is_one_split_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_closures_bare_return_before_a_parenthesis_or_bracket_is_one_split_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..815416784cd6b15de88839a1927593a3fc7342d6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_closures_bare_return_before_a_parenthesis_or_bracket_is_one_split_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+let h = || return
+(5)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_closures_bare_return_before_a_parenthesis_or_bracket_is_one_split_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_closures_bare_return_before_a_parenthesis_or_bracket_is_one_split_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..495dcc2a0632f8fcb9c4621c1fa78f23a825c876
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_closures_bare_return_before_a_parenthesis_or_bracket_is_one_split_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+let h = || return
+[1, 2].len()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_line_comment_is_a_line_break.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_line_comment_is_a_line_break.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d3bc0d93150295685188dacb173e6995766b961b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_line_comment_is_a_line_break.nova
@@ -0,0 +1,4 @@
+fn f() {
+g // note
+(x)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9b2521bd516d9ef81bb5321140fea7aff6bc5655
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms_1.nova
@@ -0,0 +1,6 @@
+fn f() {
+k(match x {
+    A => g
+    [a] => h
+})
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cbff131eabec3d2d29d8801ef0b82e23312b7f28
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_match_inside_parentheses_turns_the_rule_back_on_in_its_arms_2.nova
@@ -0,0 +1,6 @@
+fn f() {
+let v = (match x {
+    A => g
+    (a) => h
+})
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_end_continues_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_end_continues_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9d8ffa198e4805ac32bd1cb701821d9dd4aad9e4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_end_continues_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+let z = x -
+    -y
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_end_continues_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_end_continues_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e4e15ec5834c236bcaad079817cc99d1e8fd56b4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_end_continues_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+let z = x -
+    y
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_start_begins_a_statement_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_start_begins_a_statement_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d97ce6a596df45873ae1e00958c62d2ea0c4a3d5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_start_begins_a_statement_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+let z = x
+-y
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_start_begins_a_statement_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_start_begins_a_statement_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..586f90858be0936680124948f3d68097f2705644
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_minus_at_a_line_start_begins_a_statement_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+let z = x - y
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_after_a_match_begins_a_statement.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_after_a_match_begins_a_statement.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a4b18900a891cb749dd0895ea0a8220808641122
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_after_a_match_begins_a_statement.nova
@@ -0,0 +1,4 @@
+fn f() {
+match x { _ => g }
+(a)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_after_an_if_begins_a_statement.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_after_an_if_begins_a_statement.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4bb5dd476a19d4e44c02f9f94f9ca6c70b55a4e9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_after_an_if_begins_a_statement.nova
@@ -0,0 +1,4 @@
+fn f() {
+let w = if c { 1 } else { 2 }
+(a, b)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_at_a_line_start_begins_a_statement_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_at_a_line_start_begins_a_statement_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1b8d04f218c96259451a7ef51d11225363b08925
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_at_a_line_start_begins_a_statement_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+g
+(x)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_at_a_line_start_begins_a_statement_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_at_a_line_start_begins_a_statement_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cc18280e49aac8623ceeed7fe26a20f5470841e4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_at_a_line_start_begins_a_statement_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+g
+(x, y)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_at_a_line_start_begins_a_statement_3.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_at_a_line_start_begins_a_statement_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..231485c5fa093be8a2e3f6965f44d27ffcab2e1a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parenthesis_at_a_line_start_begins_a_statement_3.nova
@@ -0,0 +1,3 @@
+fn f() {
+g(x)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parse_error_anywhere_in_an_arms_head_the_rule_began_names_the_rule_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parse_error_anywhere_in_an_arms_head_the_rule_began_names_the_rule_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e504e031f67963d8d35412477afa8651373f6b25
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parse_error_anywhere_in_an_arms_head_the_rule_began_names_the_rule_1.nova
@@ -0,0 +1,7 @@
+fn f() {
+match x {
+    0 => g
+    (5)
+    _ => 1
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parse_error_anywhere_in_an_arms_head_the_rule_began_names_the_rule_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parse_error_anywhere_in_an_arms_head_the_rule_began_names_the_rule_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2b5c16dbb1a4d9f47287bebe01a992617a5e13b5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parse_error_anywhere_in_an_arms_head_the_rule_began_names_the_rule_2.nova
@@ -0,0 +1,7 @@
+fn f() {
+match x {
+    None => return
+    0,
+    Some(y) => y,
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parse_error_at_an_arm_the_rule_began_names_the_rule.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parse_error_at_an_arm_the_rule_began_names_the_rule.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6a63859a04864825c18d7bbddd443b02530d309d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_parse_error_at_an_arm_the_rule_began_names_the_rule.nova
@@ -0,0 +1,7 @@
+fn f() {
+match x {
+    0 => a
+    - 1,
+    _ => 2
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_semicolon_still_ends_a_statement_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_semicolon_still_ends_a_statement_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..70739d0475532cd3e8caaaf6e6609c16fe123d04
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_semicolon_still_ends_a_statement_1.nova
@@ -0,0 +1,3 @@
+fn f() {
+g; (x)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_semicolon_still_ends_a_statement_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_semicolon_still_ends_a_statement_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..623ae927069f2a56213ac461ace6e6902d66b334
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_semicolon_still_ends_a_statement_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+let z = x; -y
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_split_after_a_bare_return_is_recorded_once_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_split_after_a_bare_return_is_recorded_once_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0f210bb7a43f2210e5fc110d0526f7931365c7ab
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_split_after_a_bare_return_is_recorded_once_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+return
+(g)(1)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_split_after_a_bare_return_is_recorded_once_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_split_after_a_bare_return_is_recorded_once_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..59d6e3729fe64130f92e155d29748bd5860a4cd3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_split_after_a_bare_return_is_recorded_once_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+return
+-1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_tuple_pattern_arm_after_an_arm_body_begins_an_arm.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_tuple_pattern_arm_after_an_arm_body_begins_an_arm.nova
new file mode 100644
index 0000000000000000000000000000000000000000..467c89784101e15871196db4ee46a3e4a7157f98
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_tuple_pattern_arm_after_an_arm_body_begins_an_arm.nova
@@ -0,0 +1,6 @@
+fn f() {
+match p {
+    (0, b) => b
+    (a, b) => a
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d8f165bcc1118334d904ca37a2f1243257e6f5ed
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_1.nova
@@ -0,0 +1,7 @@
+fn f() {
+match x {
+    0 => break a
+    (1) => 2
+    _ => 3
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..03d629a85d99b86fd5a733c4ae792d47c02e8b45
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_2.nova
@@ -0,0 +1,7 @@
+fn f() {
+match x {
+    0 => return a
+    (1) => 2
+    _ => 3
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_3.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dcf38b4f515f0e44a66bcaa5f7a0ec3c6355194d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_3.nova
@@ -0,0 +1,7 @@
+fn f() {
+match x {
+    0 => return { a }
+    (1) => 2
+    _ => 3
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_4.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..46885a488bcf8c0ab9d5b704ae6b9d0f4e95a3a1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_value_ending_an_arm_records_no_split_however_often_it_is_met_4.nova
@@ -0,0 +1,7 @@
+fn f() {
+match x {
+    0 => |y| y
+    (1) => 2
+    _ => 3
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_view_keeps_its_rule.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_view_keeps_its_rule.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b3bc3635648b55b5111107dddd4f057e6e9e7770
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_view_keeps_its_rule.nova
@@ -0,0 +1,2 @@
+fn f() -> Html { view { g
+(x) } }
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_written_semicolon_records_no_split_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_written_semicolon_records_no_split_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..27a14617bb4e84262a41ad7919dfb45dc92bf980
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_written_semicolon_records_no_split_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+g;
+(1)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_written_semicolon_records_no_split_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_written_semicolon_records_no_split_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ec8438b46156a5c956fe18bbf29bc04a5ac12630
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_written_semicolon_records_no_split_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+let z = x;
+-y
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__a_written_semicolon_records_no_split_3.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_written_semicolon_records_no_split_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..396a810f7c8bd5258ca51b8713079ec9be8e0768
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__a_written_semicolon_records_no_split_3.nova
@@ -0,0 +1,4 @@
+fn f() {
+return;
+g()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__break_at_a_line_end_takes_no_value_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__break_at_a_line_end_takes_no_value_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..60f8ab755626b4e0c3dda69f66d0f2760d9a7232
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__break_at_a_line_end_takes_no_value_1.nova
@@ -0,0 +1,6 @@
+fn f() {
+while c {
+    break
+    g()
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__break_at_a_line_end_takes_no_value_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__break_at_a_line_end_takes_no_value_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ee71957512251e7e783cd460908526cd95977c4b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__break_at_a_line_end_takes_no_value_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+while c { break x }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..aef85d9be743d60e5f88a233659e86f50189edb4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+g
+(1)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..484ebce54e417d4cc4576ac9f82b399f5c46fe4d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+g
+[1]
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_3.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..aeca4d8ed55f4d349cc65b07a570636af80f681a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_3.nova
@@ -0,0 +1,4 @@
+fn f() {
+return
+g()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_4.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..439cabfc77198d1a4e066a1fe0a8563921aec261
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_4.nova
@@ -0,0 +1,4 @@
+fn f() {
+return a
+- b
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_5.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..677ef0ac994908e50409213dddf5f95b0b282e66
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__each_split_the_rule_makes_is_recorded_5.nova
@@ -0,0 +1,6 @@
+fn f() {
+while true {
+    break
+    x
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..49e44b3d6ca0c0a0ac5ff59c41633fe69f90b53e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_1.nova
@@ -0,0 +1,5 @@
+fn f() {
+let s = parts
+    .iter()
+    .count()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8a4d52666ae3653167266288996a03af4bbe6e21
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+let v = x
+?
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_3.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..49898645f1cd68b4a320133a759701db74170ca0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_3.nova
@@ -0,0 +1,4 @@
+fn f() {
+let z = x
+* y
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_4.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b2702fde0b9654e93e7e888cc16206cfdbdd2f45
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_4.nova
@@ -0,0 +1,4 @@
+fn f() {
+let z = x
++ y
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_5.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..43108ba3f5c11279fff10edb496947ad8666dc10
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__other_operators_and_dots_continue_the_line_before_5.nova
@@ -0,0 +1,4 @@
+fn f() {
+let z = x
+|| y
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a54fa53d41c4afe689836ad6a61416c0df3757d2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+return
+cleanup()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..659acd21a7f6bef6bc0aaa2bf0867f2711ab6ac7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+return g(
+    1)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_3.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..57c51ffa03eca6566f993c4d94beec57b0f9aeab
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_3.nova
@@ -0,0 +1,3 @@
+fn f() {
+return x
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_inside_parentheses_too.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_inside_parentheses_too.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f00b94bfa1b5d768af71d7ae55261c3a2652c141
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__return_at_a_line_end_takes_no_value_inside_parentheses_too.nova
@@ -0,0 +1,4 @@
+fn f() {
+g(return
+    x)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_let_value_and_a_tail_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_let_value_and_a_tail_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c46c9fbed516155b850dcef3029989b8e1d154a0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_let_value_and_a_tail_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+g
+-1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_let_value_and_a_tail_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_let_value_and_a_tail_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..05b6591b820384096363504596d0fb3779aaf273
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_let_value_and_a_tail_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+let a = g
+(h)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_match_arm_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_match_arm_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..031fee6475f977470e046429e9433bdfff1f57c4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_match_arm_1.nova
@@ -0,0 +1,6 @@
+fn f() {
+match x {
+    A => g
+    (a) => h
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_match_arm_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_match_arm_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8a1090a58b10fa48b02118b32624cf1977654dea
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_holds_in_a_match_arm_2.nova
@@ -0,0 +1,6 @@
+fn f() {
+match x {
+    A => g
+    [a] => h
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_a_guard.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_a_guard.nova
new file mode 100644
index 0000000000000000000000000000000000000000..feae3e945cb0f50725612497718e7f5d6ebd2100
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_a_guard.nova
@@ -0,0 +1,7 @@
+fn f() {
+match x {
+    a if a
+    -1 < 0 => a
+    _ => 0
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_a_nested_items_signature_and_value.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_a_nested_items_signature_and_value.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2c2ae5ea9a6bf834b7b8da2a7ed6423097249213
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_a_nested_items_signature_and_value.nova
@@ -0,0 +1,5 @@
+fn f() {
+const C: Int = a
+-1
+C
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_if_head_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_if_head_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..69449ac33a0f10a51d09587334e5e5490a0bc514
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_if_head_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+if x
+-y < 0 { g() }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_if_head_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_if_head_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6602f542bb53376d84de8afb4bc6fa7634f21a18
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_if_head_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+match g
+(1) { _ => 0 }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_if_head_3.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_if_head_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2fddea9d034bd5cf2f4efaac3fbc119052f0282d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_if_head_3.nova
@@ -0,0 +1,4 @@
+fn f() {
+while g
+(1) { h() }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_interpolation_hole.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_interpolation_hole.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2afb32d2ab24061c93653febc13441e77112abdc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_in_an_interpolation_hole.nova
@@ -0,0 +1,5 @@
+fn f() {
+let s = "${a
+(b)} and ${c
+-d}"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_a_record_literal.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_a_record_literal.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e77ade99a6a6a030aeb57f29fdee10b062ef5c60
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_a_record_literal.nova
@@ -0,0 +1,5 @@
+fn f() {
+let r = R { a: x
+-y, b: g
+(1) }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_arguments.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_arguments.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8eebb8b94ed7db4dc9cb54bbe1d6e0af58cd206f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_arguments.nova
@@ -0,0 +1,5 @@
+fn f() {
+h(x
+-y, g
+[0])
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_brackets_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_brackets_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..25204d8fb4d49e8ef0adf00d625be5150104cca7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_brackets_1.nova
@@ -0,0 +1,5 @@
+fn f() {
+let a = [x
+-y, g
+(1)]
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_brackets_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_brackets_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9f10b83cc0093490b3a833585327249092957c73
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_brackets_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+let a = b[i
+-1]
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_parentheses_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_parentheses_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1bf80fe47b10e10ce65ce1e3a85937e83019dcbe
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_parentheses_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+let t = (g
+(1), 2)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_parentheses_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_parentheses_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b8373ae528c93f599a74d5e36a9dee9b6db658a0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_parentheses_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+let z = (g
+(1))
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_parentheses_3.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_parentheses_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..681edccc5c9365d12c32dbf8eaf29e52019dceff
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__the_rule_is_off_inside_parentheses_3.nova
@@ -0,0 +1,4 @@
+fn f() {
+let z = (x
+-y)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__what_the_old_parser_could_not_have_continued_records_no_split_1.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__what_the_old_parser_could_not_have_continued_records_no_split_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..42ec836175928d712e5a015de88f9a9c3d98907f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__what_the_old_parser_could_not_have_continued_records_no_split_1.nova
@@ -0,0 +1,6 @@
+fn f() {
+match x {
+    A => return
+    B => h
+}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__what_the_old_parser_could_not_have_continued_records_no_split_2.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__what_the_old_parser_could_not_have_continued_records_no_split_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..06b371e02e288df5146092b97df18c3b0e6237a2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__what_the_old_parser_could_not_have_continued_records_no_split_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+return
+fn g() {}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/newline_rule__what_the_old_parser_could_not_have_continued_records_no_split_3.nova b/crates/nova-cli/tests/selfhost/cases/newline_rule__what_the_old_parser_could_not_have_continued_records_no_split_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4a297dbf7f731c7bf1a26e92002d308a8d0b6559
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/newline_rule__what_the_old_parser_could_not_have_continued_records_no_split_3.nova
@@ -0,0 +1,4 @@
+fn f() {
+return
+let y = 1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_bare_at_sign_is_a_parse_error_and_does_not_hang.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_bare_at_sign_is_a_parse_error_and_does_not_hang.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7c9d716a4bbf4142bb1edf6512b047378289f9e7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_bare_at_sign_is_a_parse_error_and_does_not_hang.nova
@@ -0,0 +1,2 @@
+@
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_bare_attribute_parses_onto_a_function.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_bare_attribute_parses_onto_a_function.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ec6e6671a8ef61a429ce58543ed4eaf3f48ef38e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_bare_attribute_parses_onto_a_function.nova
@@ -0,0 +1,2 @@
+@test
+fn t() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_component_is_a_function_that_writes_no_return_type.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_component_is_a_function_that_writes_no_return_type.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e306a3cfb39be9a5dd169770f603685f745d40ed
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_component_is_a_function_that_writes_no_return_type.nova
@@ -0,0 +1,2 @@
+component Card(title: String) { 1 }
+pub component Page() { 2 }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_component_with_a_return_type_or_type_parameters_does_not_parse_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_component_with_a_return_type_or_type_parameters_does_not_parse_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0b6b1023c1d4ea5c69b53aa6d19b17fb73c7412d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_component_with_a_return_type_or_type_parameters_does_not_parse_1.nova
@@ -0,0 +1,2 @@
+component A() -> Int { 1 }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_component_with_a_return_type_or_type_parameters_does_not_parse_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_component_with_a_return_type_or_type_parameters_does_not_parse_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..afed4bdaee638f94864127919bedf896582e343b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_component_with_a_return_type_or_type_parameters_does_not_parse_2.nova
@@ -0,0 +1,2 @@
+component A<T>() { 1 }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_keyword_method_name_followed_by_a_generic_trait_method_terminates.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_keyword_method_name_followed_by_a_generic_trait_method_terminates.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6a5f53f49a475952182eb65512d892ee078735ee
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_keyword_method_name_followed_by_a_generic_trait_method_terminates.nova
@@ -0,0 +1,4 @@
+record B<T> { v: T }
+impl<T> B<T> { fn with(self) -> Int { 1 } }
+trait M { fn remap<U>(self, u: U) -> Int }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fcad8fc851277753cbae72287497243b48eca749
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_1.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { type = Int }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..637287c97900a5353221bb78e0bf3562877a7aef
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_2.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { type Item = }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_3.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1d317c8edba0df468d8750f74ef9096fb581ac0f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_3.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { type Item }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_4.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2f06742d46d63d7af6ad3f70e3eade57f283bd0b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_malformed_associated_type_binding_terminates_4.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { type }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b995b0f90f2f56dc835acbf186c24c26c8165d28
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_1.nova
@@ -0,0 +1,4 @@
+fn f(
+    /// no
+    x: Int,
+) {}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2c7d452c2134fb43b51a7733f43cec0199baaa4b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_2.nova
@@ -0,0 +1,4 @@
+fn main() {
+    /// no
+    f()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_3.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fd68f68c9af5ce32ee5b84c43daf00e202e5b38e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_3.nova
@@ -0,0 +1,4 @@
+fn main() {
+    /// no
+    let x = 1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_4.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..65eb08725f2a37116e9875e83c2f2320b42d2b3d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_4.nova
@@ -0,0 +1,4 @@
+fn main() {
+    f()
+    /// no
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_5.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..19a31e28ee7edf19d333e83917dc7f24a773e438
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_5.nova
@@ -0,0 +1,6 @@
+fn main() {
+    match x {
+        /// no
+        _ => 1
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_6.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_6.nova
new file mode 100644
index 0000000000000000000000000000000000000000..01032995314c57d3b60dd5146caba9321d09ef49
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_6.nova
@@ -0,0 +1,2 @@
+fn main() {}
+/// no
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_7.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_7.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b1df1f27fc53973dc766fb6ca52ea49b6cf22e70
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_misplaced_doc_comment_is_an_error_7.nova
@@ -0,0 +1,3 @@
+trait T {
+    /// no
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_named_argument_outside_a_view_is_an_error.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_named_argument_outside_a_view_is_an_error.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0870c685e17f5d562925f64817497410b6a9fd35
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_named_argument_outside_a_view_is_an_error.nova
@@ -0,0 +1 @@
+fn main() { f(a: 1) }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_non_item_token_inside_an_impl_body_still_recovers_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_non_item_token_inside_an_impl_body_still_recovers_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..022ebff4851e2cd353502138d0c2453d6b9095f2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_non_item_token_inside_an_impl_body_still_recovers_1.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { 42 }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_non_item_token_inside_an_impl_body_still_recovers_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_non_item_token_inside_an_impl_body_still_recovers_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..31d29b7bebb0d6ef3cd1d86af5eeab1405429b98
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_non_item_token_inside_an_impl_body_still_recovers_2.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { let x = 1 }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_parenthesised_pattern_spans_its_parentheses.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_parenthesised_pattern_spans_its_parentheses.nova
new file mode 100644
index 0000000000000000000000000000000000000000..34f78676bc5d95cd60197b13811b874525ec3ab7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_parenthesised_pattern_spans_its_parentheses.nova
@@ -0,0 +1,4 @@
+fn main() {
+    let (x) = 1
+    let () = ()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_semicolon_ends_a_child_before_a_parenthesis_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_semicolon_ends_a_child_before_a_parenthesis_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1c3be3e94ad4fc9c78fb49a58df5fdc12c62645b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_semicolon_ends_a_child_before_a_parenthesis_1.nova
@@ -0,0 +1 @@
+fn f() { view { text("a") (text("b")) } }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_semicolon_ends_a_child_before_a_parenthesis_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_semicolon_ends_a_child_before_a_parenthesis_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..868aea20300cc39819b62d668ca47844a7e9d42a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_semicolon_ends_a_child_before_a_parenthesis_2.nova
@@ -0,0 +1 @@
+fn f() { view { text("a"); (text("b")) } }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_trailing_comma_ends_a_where_clause_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_trailing_comma_ends_a_where_clause_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fe2c6b2dbeef117870c690e2125e306f1636155d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_trailing_comma_ends_a_where_clause_1.nova
@@ -0,0 +1 @@
+fn f<T>(x: T) -> T where T: A { x }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_trailing_comma_ends_a_where_clause_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_trailing_comma_ends_a_where_clause_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8989335f8527fe176a4bb3bd335eb4bda110b77c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_trailing_comma_ends_a_where_clause_2.nova
@@ -0,0 +1 @@
+fn f<T>(x: T) -> T where T: A, { x }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_variant_list_does_not_take_the_next_items_doc.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_variant_list_does_not_take_the_next_items_doc.nova
new file mode 100644
index 0000000000000000000000000000000000000000..27edce8fa7d796c570f9a1f4daa232f7081a7d52
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_variant_list_does_not_take_the_next_items_doc.nova
@@ -0,0 +1,3 @@
+type T = | A | B
+/// next
+fn f() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__a_view_holds_elements_and_expressions.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_view_holds_elements_and_expressions.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4ffa3183eb368e5387c02ca808cc9426e49ee15c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__a_view_holds_elements_and_expressions.nova
@@ -0,0 +1,11 @@
+fn f() {
+    view {
+        div(class: "a", for: "b", type: "c", attr("d", "e")) {
+            text("x")
+            input(value: v)
+            span { }
+            Counter()
+            show(open, || view { p { } })
+        }
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_attribute_on_a_record_parses_and_is_kept.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_attribute_on_a_record_parses_and_is_kept.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ea21c33af2b04cf41b20c15f630407d3a51c4079
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_attribute_on_a_record_parses_and_is_kept.nova
@@ -0,0 +1,2 @@
+@test
+record R { n: Int }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_attribute_precedes_pub.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_attribute_precedes_pub.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8ea3bf9d5b9133f2d01cf15a208c704e7a88c688
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_attribute_precedes_pub.nova
@@ -0,0 +1,2 @@
+@test
+pub fn t() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_attribute_with_arguments_parses.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_attribute_with_arguments_parses.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2ab75274b98843e27bd21e3f8b52899e43743b5b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_attribute_with_arguments_parses.nova
@@ -0,0 +1,2 @@
+@test(should_panic)
+fn t() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_impl_associated_type_binding_may_not_be_pub_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_impl_associated_type_binding_may_not_be_pub_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fe130e08650bec1653e8d93a928be08c95257898
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_impl_associated_type_binding_may_not_be_pub_1.nova
@@ -0,0 +1 @@
+impl It for W { pub type Item = Int }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_impl_associated_type_binding_may_not_be_pub_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_impl_associated_type_binding_may_not_be_pub_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..500c6cfccf7938ea802f1851a781a8fbf1e41dec
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_impl_associated_type_binding_may_not_be_pub_2.nova
@@ -0,0 +1 @@
+impl It for W { type Item = Int }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_dropped_after_its_name_is_reported_by_name.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_dropped_after_its_name_is_reported_by_name.nova
new file mode 100644
index 0000000000000000000000000000000000000000..838fc41f5a23d81c3e4637a34055cd63875313fe
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_dropped_after_its_name_is_reported_by_name.nova
@@ -0,0 +1,3 @@
+fn f(x: Int {
+}
+fn g() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dd1c6e31c36bea0261c66855c1386cee9d11c6e8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_1.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { extern { fn e() -> Int } }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8d0fe75c98ec7f7e0d574d7416aad43f7673502d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_2.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { impl W { } }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_3.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0bcfe1b78489960a06ed78614d7761c8b386ef8a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_3.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { import foo }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_4.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4ce52d24cf0b37cba6c471c023241fbd9bb0eb33
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_4.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { module foo }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_5.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fa47b72d10057a34ce4c4782188a06a42e76c53f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_5.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { pub record R { a: Int } }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_6.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_6.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4d7134c4f2ee644f012d7d47e6769b98df311aba
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_6.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { record R { a: Int } }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_7.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_7.nova
new file mode 100644
index 0000000000000000000000000000000000000000..972248368d751d792ec35d0214d42fbb976be542
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_item_start_token_inside_an_impl_body_terminates_7.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { trait Q { fn q(self) -> Int } }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__an_unfinished_member_access_keeps_its_receiver.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_unfinished_member_access_keeps_its_receiver.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f4d7a94dbdfd340954bef044205b316bcbe7c377
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__an_unfinished_member_access_keeps_its_receiver.nova
@@ -0,0 +1,3 @@
+fn main() {
+    v.
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__async_function.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__async_function.nova
new file mode 100644
index 0000000000000000000000000000000000000000..38e4a38a3f7100172ec5c22c5213e5bcae061049
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__async_function.nova
@@ -0,0 +1 @@
+async fn f() -> Int { 42 }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__component_and_view_are_names_elsewhere.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__component_and_view_are_names_elsewhere.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0fe684e7781d1da98a49bcd539e07e6a29fcb7c1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__component_and_view_are_names_elsewhere.nova
@@ -0,0 +1,15 @@
+fn component() -> Int { 1 }
+fn view(x: Int) -> Int { x }
+fn f() -> Int {
+    let component = component()
+    let view = 2
+    if view > component { view } else { 0 }
+}
+fn g(view: Bool) -> Int {
+    if view { 1 } else { 2 }
+}
+fn h() {
+    let mut component = 1
+    component
+    g(true)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_attach_to_a_nested_item.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_attach_to_a_nested_item.nova
new file mode 100644
index 0000000000000000000000000000000000000000..98647f31c3073bb815b1d8daedcf578107ba138c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_attach_to_a_nested_item.nova
@@ -0,0 +1,4 @@
+fn main() {
+    /// inner
+    fn helper() {}
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_attach_to_each_kind_of_item.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_attach_to_each_kind_of_item.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b9a0905014d6403bf36de90bffcd4f003ac43eca
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_attach_to_each_kind_of_item.nova
@@ -0,0 +1,18 @@
+/// f
+fn f() {}
+/// r
+record R { x: Int }
+/// t
+type T = Int
+/// tr
+trait Tr {}
+/// i
+impl R {}
+/// c
+const C: Int = 1
+/// im
+import m
+/// mo
+module n
+/// e
+extern "C" {}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_attach_to_members_fields_and_variants.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_attach_to_members_fields_and_variants.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1ffbe9fdae18a1787321f40a3be28fe2909dec81
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_attach_to_members_fields_and_variants.nova
@@ -0,0 +1,28 @@
+trait Tr {
+    /// req
+    fn a(self)
+    /// prov
+    fn b(self) {}
+    /// assoc
+    type Item
+}
+impl Tr for R {
+    /// m
+    fn a(self) {}
+    /// k
+    const K: Int = 1
+    /// bind
+    type Item = Int
+}
+record R {
+    /// x
+    x: Int,
+}
+type T =
+    /// a
+    | A
+    | B
+extern "C" {
+    /// ext
+    fn ext(x: Int) -> Int
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_may_come_between_attributes_on_a_top_level_item.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_may_come_between_attributes_on_a_top_level_item.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0d99e5eb4ce24f5ac7f4001e9a272484e694c2bb
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__docs_may_come_between_attributes_on_a_top_level_item.nova
@@ -0,0 +1,4 @@
+/// one
+@test
+/// two
+fn t() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__if_else_expr.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__if_else_expr.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d92461d00b71cb018315830233b657bf2ea93195
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__if_else_expr.nova
@@ -0,0 +1 @@
+fn f(x: Int) -> Int { if x > 0 { x } else { 0 } }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__impl_body_recovery_stops_at_the_impls_closing_brace.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__impl_body_recovery_stops_at_the_impls_closing_brace.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b5f35c6e1eac2b9d42a56f52d2f09791cf1a35b2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__impl_body_recovery_stops_at_the_impls_closing_brace.nova
@@ -0,0 +1,4 @@
+record W { v: Int }
+impl W { 42 }
+record R { a: Int }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__impl_for_type.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__impl_for_type.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e4771c09e6bcc80214f053116152f094af7a5a4b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__impl_for_type.nova
@@ -0,0 +1,6 @@
+
+impl Display for Point {
+    fn fmt(self) -> String {
+        "point"
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__match_with_guard.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__match_with_guard.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b4fc85b2cf7d9112a456b43c40688b6005bb168e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__match_with_guard.nova
@@ -0,0 +1,8 @@
+
+fn describe(n: Int) -> String {
+    match n {
+        0 => "zero",
+        n if n < 0 => "negative",
+        _ => "positive",
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__multiple_arguments_and_multiple_attributes_parse.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__multiple_arguments_and_multiple_attributes_parse.nova
new file mode 100644
index 0000000000000000000000000000000000000000..66d8cee5b554bb9d31dee4aa047b92742273629a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__multiple_arguments_and_multiple_attributes_parse.nova
@@ -0,0 +1,3 @@
+@derive(Copy, Clone)
+@test
+fn t() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__nested_generics_with_glued_gt_parse_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__nested_generics_with_glued_gt_parse_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cf83bcfb46cdebe8d247a64b83d9ceb5217a042a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__nested_generics_with_glued_gt_parse_1.nova
@@ -0,0 +1 @@
+fn f() -> Option<Option<Int>> { None }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__nested_generics_with_glued_gt_parse_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__nested_generics_with_glued_gt_parse_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6fd11b06cdf740944cde5f185171ef68c15019fa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__nested_generics_with_glued_gt_parse_2.nova
@@ -0,0 +1 @@
+fn f() -> Result<Int, Option<Int>> { Ok(1) }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__nested_generics_with_glued_gt_parse_3.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__nested_generics_with_glued_gt_parse_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2e32a72ba9953ce20bff5ddfe9c556541b4a462c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__nested_generics_with_glued_gt_parse_3.nova
@@ -0,0 +1,2 @@
+record Box<T> { value: T }
+fn f(b: Box<Box<Box<Int>>>) -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__parse_recovering_agrees_with_parse_on_a_clean_file.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__parse_recovering_agrees_with_parse_on_a_clean_file.nova
new file mode 100644
index 0000000000000000000000000000000000000000..927d0d1c9ca165521ed323f04b398848d2b0ad48
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__parse_recovering_agrees_with_parse_on_a_clean_file.nova
@@ -0,0 +1,4 @@
+fn main() {
+    let v = 1
+    println("${v}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__parses_repeat_array_literal.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__parses_repeat_array_literal.nova
new file mode 100644
index 0000000000000000000000000000000000000000..51a6d5990225cccccd621350d4a086187f988810
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__parses_repeat_array_literal.nova
@@ -0,0 +1,2 @@
+fn main() { let n = 3
+ let a = [0; n] }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..caa8e3ab9bbc8c93cf29e8b7c2b5562a4a7f640f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere_1.nova
@@ -0,0 +1 @@
+fn f() { || -> Int { 1 } }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2ace048367c56b685680a1fa5fca5fc0e653f9fc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere_2.nova
@@ -0,0 +1 @@
+fn f() { || 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere_3.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2b37e681353235364b9ab0ca7db5282df6e1b377
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__pipe_pipe_is_a_closure_where_an_expression_starts_and_an_or_elsewhere_3.nova
@@ -0,0 +1 @@
+fn f(a: Bool, b: Bool) -> Bool { a || b }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__plain_and_empty_array_literals_still_parse.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__plain_and_empty_array_literals_still_parse.nova
new file mode 100644
index 0000000000000000000000000000000000000000..67df22bd26681d71e9dbabf0ea9ab3fcc68f0f21
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__plain_and_empty_array_literals_still_parse.nova
@@ -0,0 +1,3 @@
+fn main() { let a = [1, 2, 3]
+ let e = []
+ let t = [1,] }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_in_an_interpolation_parses_in_a_no_struct_literal_position_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_in_an_interpolation_parses_in_a_no_struct_literal_position_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..db0d02f0c6fb4adad4fc0d3c0952c695a7290b6c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_in_an_interpolation_parses_in_a_no_struct_literal_position_1.nova
@@ -0,0 +1,7 @@
+record R { v: Int }
+fn f(r: R) -> Int { r.v }
+fn main() {
+if "${f(R { v: 1 })}" == "1" { println("y") }
+while "${f(R { v: 2 })}" == "" { println("n") }
+match "${f(R { v: 3 })}" { _ => println("m") }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_in_an_interpolation_parses_in_a_no_struct_literal_position_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_in_an_interpolation_parses_in_a_no_struct_literal_position_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5c8dee10824cbd297c64538682049473d2e3f4e0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_in_an_interpolation_parses_in_a_no_struct_literal_position_2.nova
@@ -0,0 +1,3 @@
+record R { v: Int }
+fn main() { let r = R { v: 1 }
+ if r == r { println("y") } }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_inside_a_string_interpolation_parses_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_inside_a_string_interpolation_parses_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..38f8764440becebab0e11b2cb3bbe2b0f06ebb26
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_inside_a_string_interpolation_parses_1.nova
@@ -0,0 +1,2 @@
+fn main() { let a = true
+ println("${if a { 1 } else { 2 }}") }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_inside_a_string_interpolation_parses_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_inside_a_string_interpolation_parses_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..48c2101573bc57b3de16d107698f15b6c17c46a7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_inside_a_string_interpolation_parses_2.nova
@@ -0,0 +1,3 @@
+record R { v: G }
+record G { w: Int }
+fn main() { println("${R { v: G { w: 1 } }.v.w}") }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_inside_a_string_interpolation_parses_3.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_inside_a_string_interpolation_parses_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fcdbacd06fc5632fcce526f9dfcedc88271d1023
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__record_literal_inside_a_string_interpolation_parses_3.nova
@@ -0,0 +1,3 @@
+record R { v: Int }
+fn f(r: R) -> Int { r.v }
+fn main() { println("${f(R { v: 1 })}") }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__recovery_missing_closing_brace.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__recovery_missing_closing_brace.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9a24a243be949cc09835c8b1a9974e8536f9caa3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__recovery_missing_closing_brace.nova
@@ -0,0 +1 @@
+fn broken() { let x = 1
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__shift_right_operator_still_parses.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__shift_right_operator_still_parses.nova
new file mode 100644
index 0000000000000000000000000000000000000000..62d72e51876bb107b73bc46fd1a85a9fc9d47c8c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__shift_right_operator_still_parses.nova
@@ -0,0 +1 @@
+fn main() { let x = 256 >> 2 }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_function_with_generics.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_function_with_generics.nova
new file mode 100644
index 0000000000000000000000000000000000000000..27681c6d91572eed0f16f793310f0db8ebe10c52
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_function_with_generics.nova
@@ -0,0 +1 @@
+fn identity<T>(x: T) -> T { x }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_hello.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_hello.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f302690d825b5b00edbf664c0f7eaf7e25fa150b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_hello.nova
@@ -0,0 +1 @@
+fn main() { println("Hello, World!") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_impl_with_associated_type_binding.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_impl_with_associated_type_binding.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d5ae1476c5c4551fda5fe83861c461ad0bb97df1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_impl_with_associated_type_binding.nova
@@ -0,0 +1 @@
+impl It for W { type Item = Int  const N: Int = 1  fn get(self) -> Int { 1 } }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_record_decl.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_record_decl.nova
new file mode 100644
index 0000000000000000000000000000000000000000..89a94458fbe08e6a4a317c502b5fe291a7a38532
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_record_decl.nova
@@ -0,0 +1 @@
+record Point { x: Float, y: Float, }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_trait_with_associated_type.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_trait_with_associated_type.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2375b5cc89c80e8f57a893df693663ae6ee2e975
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_trait_with_associated_type.nova
@@ -0,0 +1 @@
+trait It { type Item  fn next(self) -> Int }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_trait_with_bounded_associated_type.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_trait_with_bounded_associated_type.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fa30c01c81ca20754372c1a07469ca2e7a8f9f6f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__snapshot_trait_with_bounded_associated_type.nova
@@ -0,0 +1 @@
+trait It { type Item: Display }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_1.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1d1b716fe74e8292ef662460a353d0a126dc86c8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_1.nova
@@ -0,0 +1,2 @@
+fn main() { }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_2.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5c34318c2147fb2285de5a1513f46168e33e6baf
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_2.nova
@@ -0,0 +1 @@
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_3.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5596718fa3745b9ae76374f353271a30f5274e9c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_3.nova
@@ -0,0 +1,2 @@
+}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_4.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..27f7440b40cd3e5d4c9e64b49a4dcebf9deef7cd
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__top_level_recovery_terminates_on_a_stray_closing_brace_4.nova
@@ -0,0 +1,3 @@
+}
+}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__trait_declaration.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__trait_declaration.nova
new file mode 100644
index 0000000000000000000000000000000000000000..330d1899a3133642e193fa3fe19f7be460562988
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__trait_declaration.nova
@@ -0,0 +1,4 @@
+
+trait Greet {
+    fn hello(self) -> String;
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/parser_tests__view_before_an_if_body_is_a_name.nova b/crates/nova-cli/tests/selfhost/cases/parser_tests__view_before_an_if_body_is_a_name.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5ae17cac82b2b5f978bbd7e50829b7ed902e2dd6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/parser_tests__view_before_an_if_body_is_a_name.nova
@@ -0,0 +1 @@
+fn f(view: Bool) -> Int { if view { 1 } else { 2 } }
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_float_elsewhere_stays_a_float.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_float_elsewhere_stays_a_float.nova
new file mode 100644
index 0000000000000000000000000000000000000000..52768eb5f961dc779e5a6a6cb3517ebfddf726c0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_float_elsewhere_stays_a_float.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0 + 1.5
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_long_tuple_breaks_one_element_a_line_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_long_tuple_breaks_one_element_a_line_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c39c9fdfae06766b7c5b2ff6568a6e6c3e50cf26
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_long_tuple_breaks_one_element_a_line_1.nova
@@ -0,0 +1,8 @@
+fn main() {
+    let t = (
+        "a long string here",
+        "another long string",
+        "and a third, much, much longer one",
+        12345,
+    )
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_long_tuple_breaks_one_element_a_line_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_long_tuple_breaks_one_element_a_line_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..30384dafbe324b69197f83dc9eb949b57630da82
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_long_tuple_breaks_one_element_a_line_2.nova
@@ -0,0 +1 @@
+fn main() { let t = ("a long string here", "another long string", "and a third, much, much longer one", 12345) }
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_space_around_the_dot_of_a_tuple_field_goes_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_space_around_the_dot_of_a_tuple_field_goes_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ce91b28787c6855a2e566d27780923df3c7c160b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_space_around_the_dot_of_a_tuple_field_goes_1.nova
@@ -0,0 +1,4 @@
+fn main() {
+    let x = t.0
+    println("${x}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_space_around_the_dot_of_a_tuple_field_goes_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_space_around_the_dot_of_a_tuple_field_goes_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..de62db8e57948b728847e2cc0e956c20e19651ac
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_space_around_the_dot_of_a_tuple_field_goes_2.nova
@@ -0,0 +1,2 @@
+fn main() { let x = t . 0
+ println("${x}") }
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tail_tuple_after_a_statement_keeps_its_semicolon.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tail_tuple_after_a_statement_keeps_its_semicolon.nova
new file mode 100644
index 0000000000000000000000000000000000000000..66017ab38eb590c29f651c6c9d78038371e09aad
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tail_tuple_after_a_statement_keeps_its_semicolon.nova
@@ -0,0 +1,4 @@
+fn f() -> (Int, Int) {
+    g();
+    (1, 2)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_a_place_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_a_place_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0b73186ea0f11236b8eb6e7b75b9f6292fa5e273
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_a_place_1.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0 = 1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_a_place_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_a_place_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c559c40cd8c3de5cfd82a816b3a5a9a396effce2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_a_place_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.1.0 += 2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_named_by_its_index_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_named_by_its_index_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fb99b23a0e228f0e3293ff0930fc687ae3877997
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_named_by_its_index_1.nova
@@ -0,0 +1,3 @@
+fn f() {
+f(x).1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_named_by_its_index_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_named_by_its_index_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..166693e567a7fd66697a50d472374bb725fac7c6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_named_by_its_index_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_named_by_its_index_3.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_named_by_its_index_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..eacf14a11e0c5fae8734c60d9a6316bbc6410cdd
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_is_named_by_its_index_3.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.12
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..19a2146270fb59b875d7694ace79be8841137a72
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_1.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0 + t.1 * 2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bb65609c1740f417085d9628d715f011d7bbd457
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0.await
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_3.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dfe7f4a08fb79ff0768a7084a3b9e5f068c0a2f3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_3.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0.len()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_4.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..332e3d079d860116e531722253b8dab340f7a1a2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_takes_the_postfix_forms_after_it_4.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0.x.1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_write_and_compound_assignment_print_as_written.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_write_and_compound_assignment_print_as_written.nova
new file mode 100644
index 0000000000000000000000000000000000000000..85e084dc57aa1fae891b563dd84f13d6af68052e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_field_write_and_compound_assignment_print_as_written.nova
@@ -0,0 +1,5 @@
+fn main() {
+    let mut t = (1, 2)
+    t.0 = 3
+    t.1 += t.0
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b64e2b9bc09bce734a329048b98710f27666e451
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_1.nova
@@ -0,0 +1,3 @@
+fn f() {
+((1, 2), (3,))
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e47c63eb7ba190b36732b50135f0ba816ebdc002
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+(1, 2)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_3.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..906f7408988f9ee916a031cf287f9c65beef4611
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_3.nova
@@ -0,0 +1,3 @@
+fn f() {
+(1, 2,)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_4.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d4618358866ad438867f9409e8a0d9f1e5a2131b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_literal_has_two_or_more_elements_or_a_trailing_comma_4.nova
@@ -0,0 +1,3 @@
+fn f() {
+(5,)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_one_keeps_its_comma_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_one_keeps_its_comma_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4c5d0b0cfcb71f1a5a588d1f8e252d787b9e0c2b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_one_keeps_its_comma_1.nova
@@ -0,0 +1 @@
+fn f(p: (Int ,)) -> (Int ,) { (p.0 ,) }
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_one_keeps_its_comma_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_one_keeps_its_comma_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..065597cbf5cdcd928d28fb64e976830d1d45c82f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_one_keeps_its_comma_2.nova
@@ -0,0 +1 @@
+fn f(p: (Int,)) -> (Int,) { (p.0,) }
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_two_loses_a_trailing_comma_when_it_fits_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_two_loses_a_trailing_comma_when_it_fits_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..40e8b720c74ebea9306807a14454900b2ad17c70
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_two_loses_a_trailing_comma_when_it_fits_1.nova
@@ -0,0 +1 @@
+fn f() -> (Int, Int) { (1, 2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_two_loses_a_trailing_comma_when_it_fits_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_two_loses_a_trailing_comma_when_it_fits_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3a8329d9c9efcd9cc7cbaf7a993bec842c61b14a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_of_two_loses_a_trailing_comma_when_it_fits_2.nova
@@ -0,0 +1 @@
+fn f() -> (Int, Int,) { (1, 2,) }
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_on_the_line_after_a_statement_begins_a_new_one_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_on_the_line_after_a_statement_begins_a_new_one_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ad989b3d891d676ce580036ac289617ba73c9946
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_on_the_line_after_a_statement_begins_a_new_one_1.nova
@@ -0,0 +1,4 @@
+fn f() {
+g();
+(1, 2)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_on_the_line_after_a_statement_begins_a_new_one_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_on_the_line_after_a_statement_begins_a_new_one_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..15abbe0f9469e8898c71c5c363f882c6e40f2399
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__a_tuple_on_the_line_after_a_statement_begins_a_new_one_2.nova
@@ -0,0 +1,4 @@
+fn f() {
+let x = g()
+(x, x)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..10f63e1d049eda0d2559142e776f31eea591dc4d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_1.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.00
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dd4b187728c075d81ae1fe6caa30b6317d60206f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.01
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_3.nova b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e220bf660da6523e059b275734c651b30d54b1ab
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_3.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0b1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_4.nova b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f843022185f554e58f92fd4f8b56eb9d8e8f0b09
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_4.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0x1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_5.nova b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..81c4184a8802e9b50c245f46ca5ed04dd21eda75
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_that_is_not_plain_decimal_digits_is_refused_5.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.1_0
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__an_index_with_an_exponent_is_a_float_and_refused.nova b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_with_an_exponent_is_a_float_and_refused.nova
new file mode 100644
index 0000000000000000000000000000000000000000..30c3175d976f9f0876c1a92400e5573e75da925a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__an_index_with_an_exponent_is_a_float_and_refused.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.1e5
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b67bd16252c31553c1b05bb34ea4c7f9f8cd2051
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_1.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0.1.2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..94f865566cef03181fadc5d8991829b34839f853
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.0.1.2.3.4
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_3.nova b/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a4902c027397b8a253845740b7dbbc54d11c8efe
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_3.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.1.0
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_4.nova b/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..32051dbce67f40f072358fd65a0fd6e44df8981c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__nested_tuple_fields_read_left_to_right_4.nova
@@ -0,0 +1,3 @@
+fn f() {
+t.1.10
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__the_newline_rule_is_off_inside_a_tuple_literal.nova b/crates/nova-cli/tests/selfhost/cases/tuples__the_newline_rule_is_off_inside_a_tuple_literal.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6ce91dbbcc32762e5d69c14ecfb9aa392d4836ba
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__the_newline_rule_is_off_inside_a_tuple_literal.nova
@@ -0,0 +1,9 @@
+fn f() {
+let t = (
+    f
+    (1),
+    a
+    - b,
+)
+t
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__tuple_fields_print_as_written_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__tuple_fields_print_as_written_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7d93a154af2b512f27f00108f41df158c614c366
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__tuple_fields_print_as_written_1.nova
@@ -0,0 +1 @@
+fn f(t: ((Int, Int), Int)) -> Int { t.0.1 + t.1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__tuple_fields_print_as_written_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__tuple_fields_print_as_written_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..df5621c03922d5144ec74c9e02f6aee172bfd031
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__tuple_fields_print_as_written_2.nova
@@ -0,0 +1,5 @@
+fn main() {
+    let n = t.0.1.2.3
+    let m = t.1.10
+    println("${n.0}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__tuple_types_parse_with_unit_and_a_parenthesised_type_unchanged_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__tuple_types_parse_with_unit_and_a_parenthesised_type_unchanged_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6cf0d841da3a2fe82bd42df4fb3802cf9fe751df
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__tuple_types_parse_with_unit_and_a_parenthesised_type_unchanged_1.nova
@@ -0,0 +1 @@
+fn f(a: (Int, String), b: (Int,), c: (Int), d: ()) -> (Bool, (Int, Int)) {}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__tuple_types_parse_with_unit_and_a_parenthesised_type_unchanged_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__tuple_types_parse_with_unit_and_a_parenthesised_type_unchanged_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fbbde1b5d8f761d97c61139e17843ba53b5762e9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__tuple_types_parse_with_unit_and_a_parenthesised_type_unchanged_2.nova
@@ -0,0 +1 @@
+fn f(v: Vec<(Int, String)>, o: Option<(Int,)>, a: [(Int, Int)]) {}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__unit_and_a_parenthesised_expression_stay_what_they_are_1.nova b/crates/nova-cli/tests/selfhost/cases/tuples__unit_and_a_parenthesised_expression_stay_what_they_are_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3e9f5c1be54be7a92226ce3b5514a69103227699
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__unit_and_a_parenthesised_expression_stay_what_they_are_1.nova
@@ -0,0 +1,3 @@
+fn f() {
+((5))
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__unit_and_a_parenthesised_expression_stay_what_they_are_2.nova b/crates/nova-cli/tests/selfhost/cases/tuples__unit_and_a_parenthesised_expression_stay_what_they_are_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3588c8936d9104d56a5875566c70983786123971
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__unit_and_a_parenthesised_expression_stay_what_they_are_2.nova
@@ -0,0 +1,3 @@
+fn f() {
+()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/tuples__unit_and_a_parenthesised_expression_stay_what_they_are_3.nova b/crates/nova-cli/tests/selfhost/cases/tuples__unit_and_a_parenthesised_expression_stay_what_they_are_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fc53c7591e463e1216b64c414a95ea58226f9722
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuples__unit_and_a_parenthesised_expression_stay_what_they_are_3.nova
@@ -0,0 +1,3 @@
+fn f() {
+(5)
+}
````

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/crates/nova-cli/tests/selfhost/fuzz.rs b/crates/nova-cli/tests/selfhost/fuzz.rs
index d1f41aa4085677007dd32740029d4892d79a631b..a5cb90ca5a603713c886c5796ee9cd04691deafd 100644
--- a/crates/nova-cli/tests/selfhost/fuzz.rs
+++ b/crates/nova-cli/tests/selfhost/fuzz.rs
@@ -15,9 +15,10 @@
 //!    swapped with another or inserted elsewhere, or one ASCII byte
 //!    replaced. Most of the result stays well formed, so a lexer, and in
 //!    5.1b a parser, gets far into it.
-//! 3. Deep nesting is 5.1b's (spec §3.5 item 3, §4.4): a `deep(rng, kind,
-//!    depth) -> String` here, beside these two, whose inputs `selfhost.rs`
-//!    hands to `agree_on_inputs` as it does theirs.
+//! 3. [`deep`]: a program nested in one way, or in several mixed, to a
+//!    given depth around the parser's limit of 128 levels (spec §3.5 item
+//!    3, §4.4; ADR 0052): well typed up to the limit, so that `nova check`
+//!    goes through every stage on it, and P0002 past it.
 //!
 //! Beside them, [`edges`]: fixed inputs that no file under `cases/` can
 //! hold as it is, such as a lone `\r` or a NUL.
@@ -380,6 +381,32 @@ pub fn edges() -> Vec<String> {
     .collect();
     out.push(c(0xFEFF));
     out.push(format!("{}let a = 1\n", c(0xFEFF)));
+    // The lexer's tests' inline sources that a file under `cases/` cannot
+    // hold, for a `\r` or another control character (ADR 0052).
+    out.extend(
+        [
+            // lexer_tests.rs: `a_line_comments_span_stops_before_a_crlf`
+            "// a\r\nfn main() {}\r\n",
+            // lexer_tests.rs: `a_lone_carriage_return_is_a_line_break`
+            "b\r(d)",
+            // lexer_tests.rs: `a_token_after_a_line_break_is_marked`
+            "a\n(b)\r\n  - c",
+            // lexer_tests.rs: `doc_text_keeps_its_indentation_and_drops_trailing_whitespace`
+            "///   indented  \r\nfn f() {}",
+            // lexer_tests.rs: `the_token_dump_writes_every_field`
+            "let s = \"a\x01\x7f\\\"\\\\é\" r#\"q\\\"# // c\n/* b */ 07 1_0 0x1 12 2.5 'é' t.1.2 \x02 \"\\q\"\n/// doc \nfn 0.1",
+            // lexer_tests.rs: `the_token_dump_of_nothing_is_eof`
+            "\r\n1",
+        ]
+        .map(String::from),
+    );
+    // Decimal numbers of 4,000 characters, plain, with `_` and before
+    // `.5`: `logos` recurses once per character on them, which overflowed
+    // S0's 1 MB main thread on Windows until its pipeline moved to a
+    // thread of its own (ADR 0051's known bug; ADR 0052).
+    out.push(format!("let a = {}\n", "1".repeat(4000)));
+    out.push(format!("let a = {}1\n", "1_".repeat(2000)));
+    out.push(format!("let a = {}.5\n", "1".repeat(4000)));
     // Every escape, then each kind of Unicode escape, built from `BS`: the
     // largest, past it, the surrogates, a leading `+` and `-`, empty, past
     // `u32`, seventeen digits whose value wraps an `Int` into a character,
@@ -416,3 +443,129 @@ pub fn edges() -> Vec<String> {
     }
     out
 }
+
+/// The kinds of nesting [`deep`] writes: the spec's parentheses, blocks,
+/// `if`, `match`, calls, types and patterns, and its `else if` chains,
+/// binary chains and prefix operators, with `while`, `for`, postfix chains
+/// and items in blocks; and all of them mixed.
+pub const DEEP_KINDS: [&str; 15] = [
+    "parentheses",
+    "blocks",
+    "ifs",
+    "else-ifs",
+    "matches",
+    "whiles",
+    "fors",
+    "calls",
+    "prefix",
+    "binary",
+    "postfix",
+    "types",
+    "patterns",
+    "items",
+    "mixed",
+];
+
+/// The depths [`deeps`] writes each kind at: around the limit, 128.
+pub const DEEP_DEPTHS: std::ops::RangeInclusive<usize> = 126..=130;
+
+/// A program whose deepest level, as the parser counts them, is `depth`
+/// (at least 2), nested as `kind` names ([`DEEP_KINDS`]). The outermost
+/// expression, type or pattern is level 0, and each wrapper one more, so
+/// `depth` is how many wrappers there are: 128 parentheses parse, 129 are
+/// P0002.
+/// Up to the limit, each is well typed but `items`, whose nested functions
+/// are E0900; `rng` picks the wrappers of `mixed`.
+pub fn deep(rng: &mut SplitMix64, kind: &str, depth: usize) -> String {
+    let d = depth;
+    let wrap = |open: &str, inner: &str, close: &str, n: usize| {
+        format!("{}{inner}{}", open.repeat(n), close.repeat(n))
+    };
+    let main = |body: String| format!("fn main() {{\n    {body}\n}}\n");
+    let value = |x: String| main(format!("let x = {x}\n    println(\"${{x}}\")"));
+    match kind {
+        "parentheses" => value(wrap("(", "1", ")", d)),
+        "blocks" => value(wrap("{ ", "1", " }", d)),
+        "ifs" => value(wrap("if true { ", "1", " } else { 0 }", d)),
+        "else-ifs" => value(format!(
+            "if false {{ 0 }}{} else {{ 1 }}",
+            " else if false { 0 }".repeat(d - 1)
+        )),
+        "matches" => value(wrap("match 0 { _ => ", "1", " }", d)),
+        "whiles" => main(wrap("while false { ", "", " }", d).replace("{  }", "{}")),
+        "fors" => main(format!(
+            "let xs = [1]\n    {}",
+            wrap("for x in xs { ", "", " }", d).replace("{  }", "{}")
+        )),
+        "calls" => format!(
+            "fn f(x: Int) -> Int {{ x }}\n\n{}",
+            value(wrap("f(", "1", ")", d))
+        ),
+        "prefix" => value(format!("{}true", "!".repeat(d))),
+        "binary" => value(format!("{}1", "1 + ".repeat(d))),
+        // `d` method calls, then a field: the chain's first operator is
+        // not a level, each after it one more, a method call one operator.
+        "postfix" => format!(
+            "record R {{ x: Int }}\n\nimpl R {{\n    fn f(self) -> R {{ self }}\n}}\n\n{}",
+            value(format!("R {{ x: 1 }}{}.x", ".f()".repeat(d)))
+        ),
+        "types" => format!(
+            "fn f(x: {}) {{}}\n\nfn main() {{}}\n",
+            wrap("Option<", "Int", ">", d)
+        ),
+        "patterns" => main(format!(
+            "let {} = 1\n    println(\"${{x}}\")",
+            wrap("(", "x", ")", d)
+        )),
+        "items" => main(wrap("fn g() { ", "()", " }", d)),
+        "mixed" => {
+            // Each wrapper one level, every one an `Int`. A prefix `-`
+            // binds tighter than `+`, so `1 + ` never comes straight
+            // inside one.
+            const WRAPPERS: [(&str, &str); 8] = [
+                ("(", ")"),
+                ("{ ", " }"),
+                ("if true { ", " } else { 0 }"),
+                ("match 0 { _ => ", " }"),
+                ("f(", ")"),
+                ("1 + ", ""),
+                ("[", "][0]"),
+                ("- ", ""),
+            ];
+            let (mut open, mut close) = (String::new(), String::new());
+            let mut last = 0;
+            for _ in 0..d {
+                let mut w = rng.below(WRAPPERS.len());
+                if last == 7 && w == 5 {
+                    w = 0;
+                }
+                last = w;
+                open.push_str(WRAPPERS[w].0);
+                close.insert_str(0, WRAPPERS[w].1);
+            }
+            format!(
+                "fn f(x: Int) -> Int {{ x }}\n\n{}",
+                value(format!("{open}1{close}"))
+            )
+        }
+        other => panic!("no kind of nesting {other}"),
+    }
+}
+
+/// Every kind but `mixed` at each of [`DEEP_DEPTHS`], then `mixed`
+/// `count` times, at depths `seed` picks among them.
+pub fn deeps(seed: u64, mixed: usize) -> Vec<(String, usize, String)> {
+    let mut rng = SplitMix64::new(seed);
+    let mut out = Vec::new();
+    for kind in DEEP_KINDS.iter().filter(|k| **k != "mixed") {
+        for depth in DEEP_DEPTHS {
+            out.push((kind.to_string(), depth, deep(&mut rng, kind, depth)));
+        }
+    }
+    let depths: Vec<usize> = DEEP_DEPTHS.collect();
+    for _ in 0..mixed {
+        let depth = *rng.pick(&depths);
+        out.push(("mixed".to_string(), depth, deep(&mut rng, "mixed", depth)));
+    }
+    out
+}
````

### Task 5: N's AST, AST dump and parser

Spec §4.1 to §4.3; decisions 12 to 14, 28, 34 and 35.

**Files:**
- Create: `selfhost/src/ast.nova`, `parser.nova`, `parse_item.nova`,
  `parse_type.nova`, `parse_stmt.nova`, `parse_expr.nova`,
  `parse_view.nova`, `parse_pattern.nova`, `ast_dump.nova`,
  `parser_tests.nova`
- Modify: `selfhost/src/token.nova` (`Spanned<T>`, `Token.kind`,
  `Span::point`, `Span.merge`, `Spanned::new`), `lexer.nova`
  (`Spanned<Token>`, `KEYWORDS`), `dump.nova` (`push_escaped` and `hex16`
  public), `main.nova` (`selfhost ast`)
- Test: N's `@test`s, through `the_selfhost_tests_pass`

- [ ] **Step 1: The tests.** `patch_block t5-tests`: `parser_tests.nova`,
  six `@test`s, each expected dump S0's: the generic arguments' rollback
  and `>>`, statement recovery, the trailing expression, `component` and
  `view`, a bare `return` and the arm's note, 128 against 129
  parentheses.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which
  ends with `Finished`.
  - `SH -- the_selfhost_tests_pass`. Expected: `0 passed; 1 failed`: `the_selfhost_tests_pass`,
    `` ` 17 passed; 0 failed;` is not in: ``, `nova test` running 11:
    `parser_tests.nova` is not imported yet.
- [ ] **Step 3: The code.** `patch_block t5-code`:
  - **`ast.nova`:** the tree (decision 12).
  - **`parser.nova` and the `parse_*.nova`:** `grammar.rs` function for
    function (decisions 13 and 14), the depth count entry for entry
    (decisions 18 and 28).
  - **`ast_dump.nova`:** ADR 0052's grammar, `dump_ast(source)`.
  - **`token.nova`, `lexer.nova`, `dump.nova`:** what the parser and the
    dump need of them.
  - **`main.nova`:** `selfhost ast FILE…` beside `selfhost tokens`, the
    usage line naming both; `parser_tests` imported.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH`. Expected: `26 passed; 0 failed`.
  - `(cd selfhost && ../target/debug/nova test) | tail -n 3`. Expected:
    `test result: ok. 17 passed; 0 failed; 0 trapped; 17 total`.
  - `git status --short`. Expected: nothing.
  - `S` (5 min 33 s). Expected: `2838 1 10`: no new Rust test; N's
    six run inside `the_selfhost_tests_pass`.
- [ ] **Step 5: The checks,** as the conventions list them; `nova fmt
  --check` reads the new modules.
- [ ] **Step 6: Commit** `selfhost: N's AST, its dump and the parser`.

**Patch `t5-tests`:**

<!-- patch t5-tests -->

````diff
diff --git a/selfhost/src/parser_tests.nova b/selfhost/src/parser_tests.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1b102278b684a901d2b0e6c0aa9221331dcf703d
--- /dev/null
+++ b/selfhost/src/parser_tests.nova
@@ -0,0 +1,182 @@
+// N's own tests of the parser's edges: the generic-arguments rollback that
+// keeps its errors, `>>` splitting, statement recovery, the trailing
+// expression, the `component` and `view` lookahead, the newline rule's
+// splits and notes, and the depth limit. Each expected dump is S0's
+// (`nova parse --dump-ast`) for the same source; the differential compares
+// every other input.
+//
+// They live in `src/`, imported by `main.nova`, for the reason
+// lexer_tests.nova gives.
+
+import ast_dump
+
+// The lines, each ended by a line break, as a dump ends each.
+fn ast_lines(lines: [String]) -> String { "\n".join(lines) + "\n" }
+
+// `fn f() { (…(1)…) }`, `n` parentheses deep.
+fn parenthesised(n: Int) -> String {
+    let mut out = StringBuilder::new()
+    out.push("fn f() { ")
+    let mut i = 0
+    while i < n {
+        out.push("(")
+        i += 1
+    }
+    out.push("1")
+    while i > 0 {
+        out.push(")")
+        i -= 1
+    }
+    out.push(" }\n")
+    out.to_string()
+}
+
+@test
+fn generic_arguments_that_fail_roll_back_and_keep_their_errors() {
+    // A type's `<…>` that does not parse as types is rolled back to its `<`,
+    // and the errors pushed while trying stay (grammar.rs's
+    // `parse_generic_args_opt`); `>>` closing two lists is split.
+    let source = "fn g(y: Option<Vec<Int>>) {}\nfn f(x: Vec<1>) {}\n"
+    let expected = ast_lines(
+        [
+            "(File",
+            "  (Item.Function 0..28",
+            "    (Function vis=Private is_async=false is_component=false name=\"g\"@3..4",
+            "      (Param is_mut=false name=\"y\"@5..6",
+            "        (Type.Path 8..24 path=\"Option\"@8..14",
+            "          (Type.Path 15..24 path=\"Vec\"@15..18",
+            "            (Type.Path 19..22 path=\"Int\"@19..22))))",
+            "      (Block 26..28))))",
+            "PARSE 41 42 expected type (in parameter type), found integer literal",
+            "PARSE 40 41 expected `)` (in function parameters), found `<`",
+            "PARSE 46 47 expected item (fn, record, type, trait, impl, const, import, module, extern), found `}`",
+        ],
+    )
+    assert_eq(dump_ast(source), expected)
+}
+
+@test
+fn statement_recovery_skips_to_a_semicolon_or_a_brace_not_a_line_break() {
+    // After a bad statement the parser skips to the next `;` or `}`, so the
+    // next line's error is never reported (`sync_to_stmt_boundary`).
+    let source = "fn f() {\n    let = 1\n    let y = (1 +\n    g()\n}\n"
+    let expected = ast_lines(
+        [
+            "(File",
+            "  (Item.Function 0..47",
+            "    (Function vis=Private is_async=false is_component=false name=\"f\"@3..4",
+            "      (Block 7..47))))",
+            "PARSE 17 18 expected pattern (in let pattern), found `=`",
+        ],
+    )
+    assert_eq(dump_ast(source), expected)
+}
+
+@test
+fn the_last_expression_is_the_trailing_one_with_or_without_its_semicolon() {
+    // `try_parse_stmt` drops `has_semi`: the expression before `}` is the
+    // block's value either way.
+    let source = "fn f() { g(); 1; }\n"
+    let expected = ast_lines(
+        [
+            "(File",
+            "  (Item.Function 0..18",
+            "    (Function vis=Private is_async=false is_component=false name=\"f\"@3..4",
+            "      (Block 7..18",
+            "        (Stmt.Expr 9..13",
+            "          (Expr.Call 9..12",
+            "            (Expr.Path 9..10 \"g\"@9..10)))",
+            "        (Expr.Lit 14..15 Int:1)))))",
+        ],
+    )
+    assert_eq(dump_ast(source), expected)
+}
+
+@test
+fn component_and_view_are_keywords_only_where_they_begin_one() {
+    // `component Name (` with a body after its `)` is an item, and `view {`
+    // a view; elsewhere each is a name (`component_at`, `parse_atom`).
+    let source = "component Card(t: String) {\n    view { div(class: \"c\") { text(t) } }\n}\nfn g() {\n    component\n    (f)\n    view\n}\n"
+    let expected = ast_lines(
+        [
+            "(File",
+            "  (Item.Function 0..70",
+            "    (Function vis=Private is_async=false is_component=true name=\"Card\"@10..14",
+            "      (Param is_mut=false name=\"t\"@15..16",
+            "        (Type.Path 18..24 path=\"String\"@18..24))",
+            "      (Block 26..70",
+            "        (Expr.View 32..68",
+            "          (ViewNode.Element 39..66 name=\"div\"@39..42 children=Some",
+            "            (ViewArg.Named name=\"class\"@43..48",
+            "              (Expr.StringInterp 50..53",
+            "                (StringPart.Lit \"c\")))",
+            "            (ViewNode.Expr 57..64",
+            "              (Expr.Call 57..64",
+            "                (Expr.Path 57..61 \"text\"@57..61)",
+            "                (Expr.Path 62..63 \"t\"@62..63))))))))",
+            "  (Item.Function 71..112",
+            "    (Function vis=Private is_async=false is_component=false name=\"g\"@74..75",
+            "      (Block 78..112",
+            "        (Stmt.Expr 84..93",
+            "          (Expr.Path 84..93 \"component\"@84..93))",
+            "        (Stmt.Expr 98..101",
+            "          (Expr.Path 98..101 \"f\"@99..100))",
+            "        (Expr.Path 106..110 \"view\"@106..110))))",
+            "  (Split 98..99 kind=Call))",
+        ],
+    )
+    assert_eq(dump_ast(source), expected)
+}
+
+@test
+fn a_bare_return_ends_at_its_line_and_the_next_arm_gets_a_note() {
+    // A `return` at the end of a line takes no value, and the line after it
+    // is split off as a `Value`: in a match, it begins the next arm, whose
+    // error then names the newline rule (`bare_keyword_ends`,
+    // `note_arm_split`).
+    let source = "fn f() -> Int {\n    match x {\n        A => return\n        (a + 1) => h\n    }\n    return\n    1\n}\n"
+    let expected = ast_lines(
+        [
+            "(File",
+            "  (Item.Function 0..95",
+            "    (Function vis=Private is_async=false is_component=false name=\"f\"@3..4",
+            "      (Type.Path 10..13 path=\"Int\"@10..13)",
+            "      (Block 14..95",
+            "        (Stmt.Expr 20..76",
+            "          (Expr.Match 20..76",
+            "            (Expr.Path 26..27 \"x\"@26..27)",
+            "            (MatchArm",
+            "              (Pattern.Ident 38..39 is_mut=false name=\"A\"@38..39)",
+            "              (Expr.Return 43..49))))",
+            "        (Stmt.Expr 81..87",
+            "          (Expr.Return 81..87))",
+            "        (Expr.Lit 92..93 Int:1))))",
+            "  (Split 92..93 kind=Value))",
+            "PARSE 61 62 expected `)` (in tuple pattern), found `+`",
+            "NOTE a `return` or `break` at the end of a line takes no value, so the next line begins the next match arm (the newline rule); to return that line's value, put it on the same line as the `return` or `break`",
+        ],
+    )
+    assert_eq(dump_ast(source), expected)
+}
+
+@test
+fn one_hundred_and_twenty_eight_parentheses_parse_and_129_are_p0002() {
+    // 128 levels of nesting parse; at 129 the parser reports P0002 at the
+    // token where the limit was crossed and stops, dropping the item it
+    // was in (`deeper`, `parse_file`). Parentheses are no node of their
+    // own: the literal's span widens over them.
+    let expected = ast_lines(
+        [
+            "(File",
+            "  (Item.Function 0..268",
+            "    (Function vis=Private is_async=false is_component=false name=\"f\"@3..4",
+            "      (Block 7..268",
+            "        (Expr.Lit 9..266 Int:1)))))",
+        ],
+    )
+    assert_eq(dump_ast(parenthesised(128)), expected)
+    let expected = ast_lines(
+        ["(File)", "PARSE 138 139 this is nested too deeply (more than 128 levels)"],
+    )
+    assert_eq(dump_ast(parenthesised(129)), expected)
+}
````

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/selfhost/src/ast.nova b/selfhost/src/ast.nova
new file mode 100644
index 0000000000000000000000000000000000000000..178bb88ccbca3a37624fb338360baf1b19b85f65
--- /dev/null
+++ b/selfhost/src/ast.nova
@@ -0,0 +1,403 @@
+// The syntax tree: a port of crates/nova-ast/src/{lib,expr,item,pattern,ty}.rs,
+// one record for each struct and one sum for each enum, with the same
+// fields in the same order (spec
+// docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md
+// §4.1, ADR 0052).
+//
+// - A variant is named `EnumVariant` (`Expr::If` is `ExprIf`): a variant's
+//   name is a value of the module, and one module may not hold two of a
+//   name or import two, so the 119 variants, many of whose Rust names repeat
+//   from one enum to the next (`Lit`, `Path`, `Tuple`) or a token's (`If`,
+//   `Fn`), each carry their enum's.
+// - A variant with named fields holds them positionally, in their order;
+//   the comment after it names them.
+// - A `Spanned` is token.nova's `Spanned<T>`, and a span two `Int`s, with no
+//   `FileId`: N parses one file at a time.
+// - A sum is recursive without a box, so `Box<Spanned<Expr>>` is
+//   `Spanned<Expr>` and `Option<Box<…>>` is `Option<…>`.
+
+import token
+
+// ---------------------------------------------------------------------------
+// lib.rs
+// ---------------------------------------------------------------------------
+
+/// The root of a parsed source file.
+pub record File { pub items: Vec<Spanned<Item>>, pub newline_splits: Vec<NewlineSplit> }
+
+/// A place where the newline rule ended a statement that the parser before
+/// it would have continued; the checker reports E0103 at it.
+pub record NewlineSplit { pub at: Span, pub kind: SplitKind }
+
+/// Which construct the newline rule declined at a split.
+pub type SplitKind =
+    /// A `(` after a line break: no call.
+    | SplitKindCall
+    /// A `[` after a line break: no index.
+    | SplitKindIndex
+    /// A `-` after a line break: no subtraction.
+    | SplitKindMinus
+    /// A line after a bare `return` or `break`: not its value.
+    | SplitKindValue
+
+/// A top-level item.
+pub type Item =
+    | ItemFunction(Function)
+    | ItemRecord(Record)
+    | ItemType(TypeDecl)
+    | ItemTrait(TraitDecl)
+    | ItemImpl(ImplBlock)
+    | ItemConst(ConstDecl)
+    | ItemImport(Import)
+    | ItemModule(Module)
+    | ItemExtern(ExternBlock)
+
+/// A statement inside a block.
+pub type Stmt =
+    // is_mut, pattern, ty, init
+    | StmtLet(Bool, Spanned<Pattern>, Option<Spanned<Type>>, Option<Spanned<Expr>>)
+    | StmtExpr(Spanned<Expr>)
+    | StmtItem(Item)
+
+/// A block: statements, and the trailing expression that is its value.
+pub record Block { pub stmts: Vec<Spanned<Stmt>>, pub trailing: Option<Spanned<Expr>> }
+
+/// A `::`-separated path.
+pub record Path { pub segments: Vec<Spanned<String>> }
+
+/// A path of one segment (`Path::single`).
+pub fn single_path(name: Spanned<String>) -> Path {
+    let mut segments: Vec<Spanned<String>> = Vec::new()
+    segments.push(name)
+    Path { segments: segments }
+}
+
+// ---------------------------------------------------------------------------
+// expr.rs
+// ---------------------------------------------------------------------------
+
+/// An expression.
+pub type Expr =
+    | ExprLit(Literal)
+    | ExprPath(Path)
+    | ExprTuple(Vec<Spanned<Expr>>)
+    | ExprArray(Vec<Spanned<Expr>>)
+    // init, len
+    | ExprArrayRepeat(Spanned<Expr>, Spanned<Expr>)
+    | ExprBlock(Block)
+    // cond, then, else_
+    | ExprIf(Spanned<Expr>, Spanned<Block>, Option<Spanned<Expr>>)
+    // scrutinee, arms
+    | ExprMatch(Spanned<Expr>, Vec<MatchArm>)
+    // cond, body
+    | ExprWhile(Spanned<Expr>, Spanned<Block>)
+    // pattern, iter, body
+    | ExprFor(Spanned<Pattern>, Spanned<Expr>, Spanned<Block>)
+    // lo, hi, inclusive
+    | ExprRange(Spanned<Expr>, Spanned<Expr>, Bool)
+    | ExprReturn(Option<Spanned<Expr>>)
+    | ExprBreak(Option<Spanned<Expr>>)
+    | ExprContinue
+    // params, ret, body
+    | ExprClosure(Vec<Param>, Option<Spanned<Type>>, Spanned<Expr>)
+    // path, fields, base
+    | ExprRecord(Path, Vec<FieldInit>, Option<Spanned<Expr>>)
+    // op, lhs, rhs
+    | ExprBinary(BinOp, Spanned<Expr>, Spanned<Expr>)
+    // op, expr
+    | ExprUnary(UnOp, Spanned<Expr>)
+    // callee, args
+    | ExprCall(Spanned<Expr>, Vec<Spanned<Expr>>)
+    // target, index
+    | ExprIndex(Spanned<Expr>, Spanned<Expr>)
+    // target, field
+    | ExprField(Spanned<Expr>, Spanned<String>)
+    | ExprTry(Spanned<Expr>)
+    | ExprAwait(Spanned<Expr>)
+    // expr, ty
+    | ExprCast(Spanned<Expr>, Spanned<Type>)
+    // op, lhs, rhs
+    | ExprAssign(AssignOp, Spanned<Expr>, Spanned<Expr>)
+    | ExprStringInterp(Vec<StringPart>)
+    | ExprView(Vec<Spanned<ViewNode>>)
+
+/// One node of a `view { … }` (RFC 0001).
+pub type ViewNode =
+    // name, args, children
+    | ViewNodeElement(Spanned<String>, Vec<ViewArg>, Option<Vec<Spanned<ViewNode>>>)
+    | ViewNodeExpr(Spanned<Expr>)
+
+/// One argument of an element.
+pub type ViewArg =
+    // name, value
+    | ViewArgNamed(Spanned<String>, Spanned<Expr>)
+    | ViewArgPositional(Spanned<Expr>)
+
+/// A part of an interpolated string.
+pub type StringPart = | StringPartLit(String) | StringPartExpr(Spanned<Expr>)
+
+/// A literal.
+pub type Literal =
+    | LiteralInt(Int)
+    | LiteralFloat(Float)
+    | LiteralStr(String)
+    | LiteralChar(Char)
+    | LiteralBool(Bool)
+
+/// One arm of a `match`.
+pub record MatchArm {
+    pub pattern: Spanned<Pattern>,
+    pub guard: Option<Spanned<Expr>>,
+    pub body: Spanned<Expr>,
+}
+
+/// One field of a record literal; `value` is absent for the shorthand
+/// `R { x }`.
+pub record FieldInit { pub name: Spanned<String>, pub value: Option<Spanned<Expr>> }
+
+/// A binary operator.
+pub type BinOp =
+    | BinOpOr
+    | BinOpAnd
+    | BinOpEq
+    | BinOpNe
+    | BinOpLt
+    | BinOpLe
+    | BinOpGt
+    | BinOpGe
+    | BinOpBitOr
+    | BinOpBitXor
+    | BinOpBitAnd
+    | BinOpShl
+    | BinOpShr
+    | BinOpAdd
+    | BinOpSub
+    | BinOpMul
+    | BinOpDiv
+    | BinOpRem
+
+/// A prefix operator.
+pub type UnOp = | UnOpNeg | UnOpNot | UnOpBitNot | UnOpRef | UnOpRefMut | UnOpDeref
+
+/// An assignment operator.
+pub type AssignOp =
+    | AssignOpAssign
+    | AssignOpAddAssign
+    | AssignOpSubAssign
+    | AssignOpMulAssign
+    | AssignOpDivAssign
+    | AssignOpRemAssign
+    | AssignOpBitOrAssign
+    | AssignOpBitAndAssign
+    | AssignOpBitXorAssign
+    | AssignOpShlAssign
+    | AssignOpShrAssign
+
+// ---------------------------------------------------------------------------
+// item.rs
+// ---------------------------------------------------------------------------
+
+/// `@name` or `@name(arg, …)` before an item; `span` covers the `@`.
+pub record Attribute { pub name: Spanned<String>, pub args: Vec<Spanned<String>>, pub span: Span }
+
+/// An item's visibility.
+pub type Visibility = | VisibilityPub | VisibilityPrivate
+
+/// A function's or a closure's parameter.
+pub record Param { pub is_mut: Bool, pub name: Spanned<String>, pub ty: Spanned<Type> }
+
+/// One bound of a `where` clause: `ty: bounds`.
+pub record WhereBound { pub ty: Spanned<Type>, pub bounds: Vec<Spanned<Path>> }
+
+/// A function, or a component (RFC 0001).
+pub record Function {
+    pub docs: Vec<Spanned<String>>,
+    pub attrs: Vec<Attribute>,
+    pub vis: Visibility,
+    pub is_async: Bool,
+    pub is_component: Bool,
+    pub name: Spanned<String>,
+    pub generics: Vec<TypeParam>,
+    pub params: Vec<Param>,
+    pub return_ty: Option<Spanned<Type>>,
+    pub where_clause: Vec<WhereBound>,
+    pub body: Spanned<Block>,
+}
+
+/// A record declaration.
+pub record Record {
+    pub docs: Vec<Spanned<String>>,
+    pub attrs: Vec<Attribute>,
+    pub vis: Visibility,
+    pub name: Spanned<String>,
+    pub generics: Vec<TypeParam>,
+    pub fields: Vec<RecordField>,
+}
+
+/// One field of a record declaration.
+pub record RecordField {
+    pub docs: Vec<Spanned<String>>,
+    pub vis: Visibility,
+    pub name: Spanned<String>,
+    pub ty: Spanned<Type>,
+}
+
+/// A `type` declaration.
+pub record TypeDecl {
+    pub docs: Vec<Spanned<String>>,
+    pub attrs: Vec<Attribute>,
+    pub vis: Visibility,
+    pub name: Spanned<String>,
+    pub generics: Vec<TypeParam>,
+    pub def: TypeDef,
+}
+
+/// What a `type` declares: an alias, or a sum of variants.
+pub type TypeDef = | TypeDefAlias(Spanned<Type>) | TypeDefSum(Vec<Variant>)
+
+/// One variant of a sum.
+pub record Variant {
+    pub docs: Vec<Spanned<String>>,
+    pub name: Spanned<String>,
+    pub fields: Vec<Spanned<Type>>,
+}
+
+/// A trait declaration.
+pub record TraitDecl {
+    pub docs: Vec<Spanned<String>>,
+    pub attrs: Vec<Attribute>,
+    pub vis: Visibility,
+    pub name: Spanned<String>,
+    pub generics: Vec<TypeParam>,
+    pub supertraits: Vec<Spanned<Path>>,
+    pub where_clause: Vec<WhereBound>,
+    pub items: Vec<TraitItem>,
+}
+
+/// An item inside a trait.
+pub type TraitItem =
+    | TraitItemRequired(FunctionSig)
+    | TraitItemProvided(Function)
+    // docs, name, bounds
+    | TraitItemAssocType(Vec<Spanned<String>>, Spanned<String>, Vec<Spanned<Path>>)
+
+/// A function's signature, without a body.
+pub record FunctionSig {
+    pub docs: Vec<Spanned<String>>,
+    pub is_async: Bool,
+    pub name: Spanned<String>,
+    pub generics: Vec<TypeParam>,
+    pub params: Vec<Param>,
+    pub return_ty: Option<Spanned<Type>>,
+    pub where_clause: Vec<WhereBound>,
+}
+
+/// An `impl` block, of a type or of a trait for a type.
+pub record ImplBlock {
+    pub docs: Vec<Spanned<String>>,
+    pub attrs: Vec<Attribute>,
+    pub generics: Vec<TypeParam>,
+    pub trait_: Option<Spanned<Path>>,
+    pub ty: Spanned<Type>,
+    pub where_clause: Vec<WhereBound>,
+    pub functions: Vec<Function>,
+    pub consts: Vec<ConstDecl>,
+    pub assoc_types: Vec<AssocTypeBinding>,
+}
+
+/// `type Name = Ty` inside an `impl`.
+pub record AssocTypeBinding {
+    pub docs: Vec<Spanned<String>>,
+    pub name: Spanned<String>,
+    pub ty: Spanned<Type>,
+}
+
+/// A `const` declaration.
+pub record ConstDecl {
+    pub docs: Vec<Spanned<String>>,
+    pub attrs: Vec<Attribute>,
+    pub vis: Visibility,
+    pub name: Spanned<String>,
+    pub ty: Spanned<Type>,
+    pub value: Spanned<Expr>,
+}
+
+/// An `import`.
+pub record Import {
+    pub docs: Vec<Spanned<String>>,
+    pub attrs: Vec<Attribute>,
+    pub path: Spanned<Path>,
+    pub kind: ImportKind,
+}
+
+/// What an `import` brings in.
+pub type ImportKind =
+    | ImportKindSimple
+    | ImportKindAlias(Spanned<String>)
+    | ImportKindList(Vec<Spanned<String>>)
+
+/// A `module` declaration.
+pub record Module {
+    pub docs: Vec<Spanned<String>>,
+    pub attrs: Vec<Attribute>,
+    pub path: Spanned<Path>,
+}
+
+/// An `extern` block.
+pub record ExternBlock {
+    pub docs: Vec<Spanned<String>>,
+    pub attrs: Vec<Attribute>,
+    pub abi: Option<String>,
+    pub items: Vec<ExternItem>,
+}
+
+/// An item inside an `extern` block.
+pub type ExternItem = | ExternItemFn(FunctionSig)
+
+// ---------------------------------------------------------------------------
+// pattern.rs
+// ---------------------------------------------------------------------------
+
+/// A pattern.
+pub type Pattern =
+    | PatternWildcard
+    | PatternLit(Literal)
+    // is_mut, name
+    | PatternIdent(Bool, Spanned<String>)
+    // name, inner
+    | PatternBinding(Spanned<String>, Spanned<Pattern>)
+    // path, fields
+    | PatternTupleStruct(Path, Vec<Spanned<Pattern>>)
+    // path, fields, rest
+    | PatternRecord(Path, Vec<FieldPat>, Bool)
+    | PatternTuple(Vec<Spanned<Pattern>>)
+    | PatternArray(Vec<Spanned<Pattern>>)
+    | PatternOr(Vec<Spanned<Pattern>>)
+    // lo, hi, inclusive
+    | PatternRange(Spanned<Pattern>, Spanned<Pattern>, Bool)
+    | PatternPath(Path)
+
+/// One field of a record pattern; `pattern` is absent for the shorthand.
+pub record FieldPat { pub name: Spanned<String>, pub pattern: Option<Spanned<Pattern>> }
+
+// ---------------------------------------------------------------------------
+// ty.rs
+// ---------------------------------------------------------------------------
+
+/// A type, as written.
+pub type Type =
+    // path, args
+    | TypePath(Path, Vec<Spanned<Type>>)
+    // is_mut, inner
+    | TypeRef(Bool, Spanned<Type>)
+    // is_mut, inner
+    | TypePtr(Bool, Spanned<Type>)
+    | TypeArray(Spanned<Type>)
+    | TypeTuple(Vec<Spanned<Type>>)
+    // params, ret
+    | TypeFn(Vec<Spanned<Type>>, Spanned<Type>)
+    | TypeOptional(Spanned<Type>)
+    | TypeInfer
+
+/// A type parameter and its bounds.
+pub record TypeParam { pub name: Spanned<String>, pub bounds: Vec<Spanned<Path>> }
diff --git a/selfhost/src/ast_dump.nova b/selfhost/src/ast_dump.nova
new file mode 100644
index 0000000000000000000000000000000000000000..04bc35edc02925a968602e3a8de94f795547b12d
--- /dev/null
+++ b/selfhost/src/ast_dump.nova
@@ -0,0 +1,906 @@
+// The AST dump: one source's tree and errors as text, one node per line,
+// exactly as S0's `nova parse --dump-ast` prints them: a port of
+// crates/nova-ast/src/dump.rs (the tree) and crates/nova-parser/src/dump.rs
+// (the errors), function by function. ADR 0052 writes the grammar out, with
+// an example of every kind; dump.nova's rules for strings are the token
+// dump's.
+
+import ast
+import dump
+import lexer
+import parser
+import token
+
+/// The AST dump of `source`: its tree, then `ERR` for each lex error,
+/// `PARSE` for each parse error and `NOTE` for each of its notes
+/// (nova-parser's `dump_ast`).
+pub fn dump_ast(source: String) -> String {
+    let (tokens, lex_errors) = lex(source)
+    let (file, errors) = parse(tokens)
+    let mut out = dump_file(file)
+    for e in lex_errors.iter() {
+        let span = e.span()
+        out.push("ERR ${span.start} ${span.end} ")
+        push_escaped(out, "${e}", false)
+        out.push("\n")
+    }
+    for e in errors.iter() {
+        let span = e.span()
+        out.push("PARSE ${span.start} ${span.end} ")
+        push_escaped(out, "${e}", false)
+        out.push("\n")
+        for note in e.notes().iter() {
+            out.push("NOTE ")
+            push_escaped(out, note, false)
+            out.push("\n")
+        }
+    }
+    out.to_string()
+}
+
+/// The dump of `file`'s tree, into a builder the errors are added to
+/// (nova-ast's `dump_file`).
+pub fn dump_file(file: File) -> StringBuilder {
+    let mut d = Dump { out: StringBuilder::new() }
+    d.open(0, "File", None)
+    for item in file.items.iter() { d.item(1, Some(item.span), item.value) }
+    for split in file.newline_splits.iter() {
+        d.open(1, "Split", Some(split.at))
+        d.field("kind", split_kind_name(split.kind))
+        d.close()
+    }
+    d.close()
+    d.out.push("\n")
+    d.out
+}
+
+// The dump being written (dump.rs's `Dump`).
+record Dump { out: StringBuilder }
+
+impl Dump {
+    // --- Lines and values ---
+
+    // A node's line, up to its kind and span.
+    fn open(mut self, depth: Int, kind: String, span: Option<Span>) {
+        if !self.out.is_empty() { self.out.push("\n") }
+        let mut i = 0
+        while i < depth {
+            self.out.push("  ")
+            i += 1
+        }
+        self.out.push("(")
+        self.out.push(kind)
+        match span {
+            Some(s) => self.out.push(" ${s.start}..${s.end}")
+            None => ()
+        }
+    }
+
+    fn close(mut self) { self.out.push(")") }
+
+    // ` key=value`.
+    fn field(mut self, key: String, value: String) { self.out.push(" ${key}=${value}") }
+
+    fn bool_field(mut self, key: String, value: Bool) { self.field(key, "${value}") }
+
+    // ` key=` and the name.
+    fn name_field(mut self, key: String, name: Spanned<String>) {
+        self.out.push(" ${key}=")
+        self.name(name)
+    }
+
+    // ` key=` and the path.
+    fn path_field(mut self, key: String, path: Path) {
+        self.out.push(" ${key}=")
+        self.path(path)
+    }
+
+    // A name: quoted, `@`, its span.
+    fn name(mut self, name: Spanned<String>) {
+        self.quote(name.value)
+        self.out.push("@${name.span.start}..${name.span.end}")
+    }
+
+    // A path: its segments' names, joined by `::`.
+    fn path(mut self, path: Path) {
+        let mut i = 0
+        while i < path.segments.len() {
+            if i > 0 { self.out.push("::") }
+            self.name(path.segments.at(i))
+            i += 1
+        }
+    }
+
+    // `text` in double quotes, escaped as the token dump escapes it
+    // (dump.rs's `quote`).
+    fn quote(mut self, text: String) {
+        self.out.push("\"")
+        push_escaped(self.out, text, true)
+        self.out.push("\"")
+    }
+
+    // ` ` and a positional string.
+    fn text(mut self, text: String) {
+        self.out.push(" ")
+        self.quote(text)
+    }
+
+    // ` ` and a literal.
+    fn literal(mut self, lit: Literal) {
+        match lit {
+            LiteralInt(n) => self.out.push(" Int:${n}")
+            LiteralFloat(f) => {
+                self.out.push(" Float:")
+                self.out.push(hex16(f.to_bits()))
+            }
+            LiteralStr(s) => {
+                self.out.push(" Str:")
+                self.quote(s)
+            }
+            LiteralChar(c) => self.out.push(" Char:${c.to_int()}")
+            LiteralBool(b) => self.out.push(" Bool:${b}")
+        }
+    }
+
+    // --- Shared children ---
+
+    fn docs(mut self, depth: Int, docs: Vec<Spanned<String>>) {
+        for doc in docs.iter() {
+            self.open(depth, "Doc", Some(doc.span))
+            self.text(doc.value)
+            self.close()
+        }
+    }
+
+    fn attrs(mut self, depth: Int, attrs: Vec<Attribute>) {
+        for attr in attrs.iter() {
+            self.open(depth, "Attr", Some(attr.span))
+            self.out.push(" ")
+            self.name(attr.name)
+            for arg in attr.args.iter() {
+                self.out.push(" ")
+                self.name(arg)
+            }
+            self.close()
+        }
+    }
+
+    fn path_node(mut self, depth: Int, path: Spanned<Path>) {
+        self.open(depth, "Path", Some(path.span))
+        self.out.push(" ")
+        self.path(path.value)
+        self.close()
+    }
+
+    fn paths(mut self, depth: Int, paths: Vec<Spanned<Path>>) {
+        for path in paths.iter() { self.path_node(depth, path) }
+    }
+
+    fn generics(mut self, depth: Int, generics: Vec<TypeParam>) {
+        for g in generics.iter() {
+            self.open(depth, "TypeParam", None)
+            self.name_field("name", g.name)
+            self.paths(depth + 1, g.bounds)
+            self.close()
+        }
+    }
+
+    fn params(mut self, depth: Int, params: Vec<Param>) {
+        for p in params.iter() {
+            self.open(depth, "Param", None)
+            self.bool_field("is_mut", p.is_mut)
+            self.name_field("name", p.name)
+            self.ty(depth + 1, p.ty)
+            self.close()
+        }
+    }
+
+    fn where_clause(mut self, depth: Int, bounds: Vec<WhereBound>) {
+        for b in bounds.iter() {
+            self.open(depth, "WhereBound", None)
+            self.ty(depth + 1, b.ty)
+            self.paths(depth + 1, b.bounds)
+            self.close()
+        }
+    }
+
+    // --- Items ---
+
+    fn item(mut self, depth: Int, span: Option<Span>, item: Item) {
+        let kind = match item {
+            ItemFunction(_) => "Item.Function"
+            ItemRecord(_) => "Item.Record"
+            ItemType(_) => "Item.Type"
+            ItemTrait(_) => "Item.Trait"
+            ItemImpl(_) => "Item.Impl"
+            ItemConst(_) => "Item.Const"
+            ItemImport(_) => "Item.Import"
+            ItemModule(_) => "Item.Module"
+            ItemExtern(_) => "Item.Extern"
+        }
+        self.open(depth, kind, span)
+        let depth = depth + 1
+        match item {
+            ItemFunction(f) => self.function(depth, f)
+            ItemRecord(r) => self.record_decl(depth, r)
+            ItemType(t) => self.type_decl(depth, t)
+            ItemTrait(t) => self.trait_decl(depth, t)
+            ItemImpl(i) => self.impl_block(depth, i)
+            ItemConst(c) => self.const_decl(depth, c)
+            ItemImport(i) => self.import_decl(depth, i)
+            ItemModule(m) => self.module_decl(depth, m)
+            ItemExtern(e) => self.extern_block(depth, e)
+        }
+        self.close()
+    }
+
+    fn function(mut self, depth: Int, f: Function) {
+        self.open(depth, "Function", None)
+        self.field("vis", visibility_name(f.vis))
+        self.bool_field("is_async", f.is_async)
+        self.bool_field("is_component", f.is_component)
+        self.name_field("name", f.name)
+        let depth = depth + 1
+        self.docs(depth, f.docs)
+        self.attrs(depth, f.attrs)
+        self.generics(depth, f.generics)
+        self.params(depth, f.params)
+        match f.return_ty {
+            Some(t) => self.ty(depth, t)
+            None => ()
+        }
+        self.where_clause(depth, f.where_clause)
+        self.block(depth, Some(f.body.span), f.body.value)
+        self.close()
+    }
+
+    fn function_sig(mut self, depth: Int, f: FunctionSig) {
+        self.open(depth, "FunctionSig", None)
+        self.bool_field("is_async", f.is_async)
+        self.name_field("name", f.name)
+        let depth = depth + 1
+        self.docs(depth, f.docs)
+        self.generics(depth, f.generics)
+        self.params(depth, f.params)
+        match f.return_ty {
+            Some(t) => self.ty(depth, t)
+            None => ()
+        }
+        self.where_clause(depth, f.where_clause)
+        self.close()
+    }
+
+    // `record`, a keyword in Nova.
+    fn record_decl(mut self, depth: Int, r: Record) {
+        self.open(depth, "Record", None)
+        self.field("vis", visibility_name(r.vis))
+        self.name_field("name", r.name)
+        let depth = depth + 1
+        self.docs(depth, r.docs)
+        self.attrs(depth, r.attrs)
+        self.generics(depth, r.generics)
+        for f in r.fields.iter() { self.record_field(depth, f) }
+        self.close()
+    }
+
+    fn record_field(mut self, depth: Int, f: RecordField) {
+        self.open(depth, "RecordField", None)
+        self.field("vis", visibility_name(f.vis))
+        self.name_field("name", f.name)
+        self.docs(depth + 1, f.docs)
+        self.ty(depth + 1, f.ty)
+        self.close()
+    }
+
+    fn type_decl(mut self, depth: Int, t: TypeDecl) {
+        self.open(depth, "TypeDecl", None)
+        self.field("vis", visibility_name(t.vis))
+        self.name_field("name", t.name)
+        let depth = depth + 1
+        self.docs(depth, t.docs)
+        self.attrs(depth, t.attrs)
+        self.generics(depth, t.generics)
+        match t.def {
+            TypeDefAlias(ty) => {
+                self.open(depth, "TypeDef.Alias", None)
+                self.ty(depth + 1, ty)
+                self.close()
+            }
+            TypeDefSum(variants) => {
+                self.open(depth, "TypeDef.Sum", None)
+                for v in variants.iter() { self.variant(depth + 1, v) }
+                self.close()
+            }
+        }
+        self.close()
+    }
+
+    fn variant(mut self, depth: Int, v: Variant) {
+        self.open(depth, "Variant", None)
+        self.name_field("name", v.name)
+        self.docs(depth + 1, v.docs)
+        for t in v.fields.iter() { self.ty(depth + 1, t) }
+        self.close()
+    }
+
+    fn trait_decl(mut self, depth: Int, t: TraitDecl) {
+        self.open(depth, "TraitDecl", None)
+        self.field("vis", visibility_name(t.vis))
+        self.name_field("name", t.name)
+        let depth = depth + 1
+        self.docs(depth, t.docs)
+        self.attrs(depth, t.attrs)
+        self.generics(depth, t.generics)
+        self.paths(depth, t.supertraits)
+        self.where_clause(depth, t.where_clause)
+        for item in t.items.iter() {
+            match item {
+                TraitItemRequired(sig) => {
+                    self.open(depth, "TraitItem.Required", None)
+                    self.function_sig(depth + 1, sig)
+                }
+                TraitItemProvided(f) => {
+                    self.open(depth, "TraitItem.Provided", None)
+                    self.function(depth + 1, f)
+                }
+                TraitItemAssocType(docs, name, bounds) => {
+                    self.open(depth, "TraitItem.AssocType", None)
+                    self.name_field("name", name)
+                    self.docs(depth + 1, docs)
+                    self.paths(depth + 1, bounds)
+                }
+            }
+            self.close()
+        }
+        self.close()
+    }
+
+    fn impl_block(mut self, depth: Int, i: ImplBlock) {
+        self.open(depth, "ImplBlock", None)
+        let depth = depth + 1
+        self.docs(depth, i.docs)
+        self.attrs(depth, i.attrs)
+        self.generics(depth, i.generics)
+        match i.trait_ {
+            Some(t) => self.path_node(depth, t)
+            None => ()
+        }
+        self.ty(depth, i.ty)
+        self.where_clause(depth, i.where_clause)
+        for f in i.functions.iter() { self.function(depth, f) }
+        for c in i.consts.iter() { self.const_decl(depth, c) }
+        for a in i.assoc_types.iter() { self.assoc_type_binding(depth, a) }
+        self.close()
+    }
+
+    fn assoc_type_binding(mut self, depth: Int, a: AssocTypeBinding) {
+        self.open(depth, "AssocTypeBinding", None)
+        self.name_field("name", a.name)
+        self.docs(depth + 1, a.docs)
+        self.ty(depth + 1, a.ty)
+        self.close()
+    }
+
+    fn const_decl(mut self, depth: Int, c: ConstDecl) {
+        self.open(depth, "ConstDecl", None)
+        self.field("vis", visibility_name(c.vis))
+        self.name_field("name", c.name)
+        let depth = depth + 1
+        self.docs(depth, c.docs)
+        self.attrs(depth, c.attrs)
+        self.ty(depth, c.ty)
+        self.expr(depth, c.value)
+        self.close()
+    }
+
+    // `import`, a keyword in Nova.
+    fn import_decl(mut self, depth: Int, i: Import) {
+        self.open(depth, "Import", None)
+        let depth = depth + 1
+        self.docs(depth, i.docs)
+        self.attrs(depth, i.attrs)
+        self.path_node(depth, i.path)
+        match i.kind {
+            ImportKindSimple => self.open(depth, "ImportKind.Simple", None)
+            ImportKindAlias(name) => {
+                self.open(depth, "ImportKind.Alias", None)
+                self.out.push(" ")
+                self.name(name)
+            }
+            ImportKindList(names) => {
+                self.open(depth, "ImportKind.List", None)
+                for name in names.iter() {
+                    self.out.push(" ")
+                    self.name(name)
+                }
+            }
+        }
+        self.close()
+        self.close()
+    }
+
+    // `module`, a keyword in Nova.
+    fn module_decl(mut self, depth: Int, m: Module) {
+        self.open(depth, "Module", None)
+        self.docs(depth + 1, m.docs)
+        self.attrs(depth + 1, m.attrs)
+        self.path_node(depth + 1, m.path)
+        self.close()
+    }
+
+    fn extern_block(mut self, depth: Int, e: ExternBlock) {
+        self.open(depth, "ExternBlock", None)
+        match e.abi {
+            Some(abi) => {
+                self.out.push(" abi=")
+                self.quote(abi)
+            }
+            None => ()
+        }
+        let depth = depth + 1
+        self.docs(depth, e.docs)
+        self.attrs(depth, e.attrs)
+        for item in e.items.iter() {
+            match item {
+                ExternItemFn(sig) => {
+                    self.open(depth, "ExternItem.Fn", None)
+                    self.function_sig(depth + 1, sig)
+                    self.close()
+                }
+            }
+        }
+        self.close()
+    }
+
+    // --- Blocks and statements ---
+
+    fn block(mut self, depth: Int, span: Option<Span>, b: Block) {
+        self.open(depth, "Block", span)
+        for s in b.stmts.iter() { self.stmt(depth + 1, s) }
+        match b.trailing {
+            Some(t) => self.expr(depth + 1, t)
+            None => ()
+        }
+        self.close()
+    }
+
+    fn stmt(mut self, depth: Int, s: Spanned<Stmt>) {
+        match s.value {
+            StmtLet(is_mut, pattern, ty, init) => {
+                self.open(depth, "Stmt.Let", Some(s.span))
+                self.bool_field("is_mut", is_mut)
+                self.pattern(depth + 1, pattern)
+                match ty {
+                    Some(t) => self.ty(depth + 1, t)
+                    None => ()
+                }
+                match init {
+                    Some(e) => self.expr(depth + 1, e)
+                    None => ()
+                }
+            }
+            StmtExpr(e) => {
+                self.open(depth, "Stmt.Expr", Some(s.span))
+                self.expr(depth + 1, e)
+            }
+            StmtItem(item) => {
+                self.open(depth, "Stmt.Item", Some(s.span))
+                self.item(depth + 1, None, item)
+            }
+        }
+        self.close()
+    }
+
+    // --- Expressions ---
+
+    fn exprs(mut self, depth: Int, exprs: Vec<Spanned<Expr>>) {
+        for e in exprs.iter() { self.expr(depth, e) }
+    }
+
+    fn expr(mut self, depth: Int, e: Spanned<Expr>) {
+        let span = Some(e.span)
+        let inner = depth + 1
+        match e.value {
+            ExprLit(lit) => {
+                self.open(depth, "Expr.Lit", span)
+                self.literal(lit)
+            }
+            ExprPath(path) => {
+                self.open(depth, "Expr.Path", span)
+                self.out.push(" ")
+                self.path(path)
+            }
+            ExprTuple(items) => {
+                self.open(depth, "Expr.Tuple", span)
+                self.exprs(inner, items)
+            }
+            ExprArray(items) => {
+                self.open(depth, "Expr.Array", span)
+                self.exprs(inner, items)
+            }
+            ExprArrayRepeat(init, len) => {
+                self.open(depth, "Expr.ArrayRepeat", span)
+                self.expr(inner, init)
+                self.expr(inner, len)
+            }
+            ExprBlock(b) => {
+                self.open(depth, "Expr.Block", span)
+                self.block(inner, None, b)
+            }
+            ExprIf(cond, then, else_) => {
+                self.open(depth, "Expr.If", span)
+                self.expr(inner, cond)
+                self.block(inner, Some(then.span), then.value)
+                match else_ {
+                    Some(e) => self.expr(inner, e)
+                    None => ()
+                }
+            }
+            ExprMatch(scrutinee, arms) => {
+                self.open(depth, "Expr.Match", span)
+                self.expr(inner, scrutinee)
+                for arm in arms.iter() { self.match_arm(inner, arm) }
+            }
+            ExprWhile(cond, body) => {
+                self.open(depth, "Expr.While", span)
+                self.expr(inner, cond)
+                self.block(inner, Some(body.span), body.value)
+            }
+            ExprFor(pattern, iter, body) => {
+                self.open(depth, "Expr.For", span)
+                self.pattern(inner, pattern)
+                self.expr(inner, iter)
+                self.block(inner, Some(body.span), body.value)
+            }
+            ExprRange(lo, hi, inclusive) => {
+                self.open(depth, "Expr.Range", span)
+                self.bool_field("inclusive", inclusive)
+                self.expr(inner, lo)
+                self.expr(inner, hi)
+            }
+            ExprReturn(value) => {
+                self.open(depth, "Expr.Return", span)
+                match value {
+                    Some(v) => self.expr(inner, v)
+                    None => ()
+                }
+            }
+            ExprBreak(value) => {
+                self.open(depth, "Expr.Break", span)
+                match value {
+                    Some(v) => self.expr(inner, v)
+                    None => ()
+                }
+            }
+            ExprContinue => self.open(depth, "Expr.Continue", span)
+            ExprClosure(params, ret, body) => {
+                self.open(depth, "Expr.Closure", span)
+                self.params(inner, params)
+                match ret {
+                    Some(t) => self.ty(inner, t)
+                    None => ()
+                }
+                self.expr(inner, body)
+            }
+            ExprRecord(path, fields, base) => {
+                self.open(depth, "Expr.Record", span)
+                self.path_field("path", path)
+                for f in fields.iter() { self.field_init(inner, f) }
+                match base {
+                    Some(b) => self.expr(inner, b)
+                    None => ()
+                }
+            }
+            ExprBinary(op, lhs, rhs) => {
+                self.open(depth, "Expr.Binary", span)
+                self.field("op", bin_op_name(op))
+                self.expr(inner, lhs)
+                self.expr(inner, rhs)
+            }
+            ExprUnary(op, operand) => {
+                self.open(depth, "Expr.Unary", span)
+                self.field("op", un_op_name(op))
+                self.expr(inner, operand)
+            }
+            ExprCall(callee, args) => {
+                self.open(depth, "Expr.Call", span)
+                self.expr(inner, callee)
+                self.exprs(inner, args)
+            }
+            ExprIndex(target, index) => {
+                self.open(depth, "Expr.Index", span)
+                self.expr(inner, target)
+                self.expr(inner, index)
+            }
+            ExprField(target, field) => {
+                self.open(depth, "Expr.Field", span)
+                self.name_field("field", field)
+                self.expr(inner, target)
+            }
+            ExprTry(operand) => {
+                self.open(depth, "Expr.Try", span)
+                self.expr(inner, operand)
+            }
+            ExprAwait(operand) => {
+                self.open(depth, "Expr.Await", span)
+                self.expr(inner, operand)
+            }
+            ExprCast(operand, ty) => {
+                self.open(depth, "Expr.Cast", span)
+                self.expr(inner, operand)
+                self.ty(inner, ty)
+            }
+            ExprAssign(op, lhs, rhs) => {
+                self.open(depth, "Expr.Assign", span)
+                self.field("op", assign_op_name(op))
+                self.expr(inner, lhs)
+                self.expr(inner, rhs)
+            }
+            ExprStringInterp(parts) => {
+                self.open(depth, "Expr.StringInterp", span)
+                for part in parts.iter() {
+                    match part {
+                        StringPartLit(text) => {
+                            self.open(inner, "StringPart.Lit", None)
+                            self.text(text)
+                        }
+                        StringPartExpr(e) => {
+                            self.open(inner, "StringPart.Expr", None)
+                            self.expr(inner + 1, e)
+                        }
+                    }
+                    self.close()
+                }
+            }
+            ExprView(nodes) => {
+                self.open(depth, "Expr.View", span)
+                self.view_nodes(inner, nodes)
+            }
+        }
+        self.close()
+    }
+
+    fn match_arm(mut self, depth: Int, arm: MatchArm) {
+        self.open(depth, "MatchArm", None)
+        self.pattern(depth + 1, arm.pattern)
+        match arm.guard {
+            Some(g) => self.expr(depth + 1, g)
+            None => ()
+        }
+        self.expr(depth + 1, arm.body)
+        self.close()
+    }
+
+    fn field_init(mut self, depth: Int, f: FieldInit) {
+        self.open(depth, "FieldInit", None)
+        self.name_field("name", f.name)
+        match f.value {
+            Some(v) => self.expr(depth + 1, v)
+            None => ()
+        }
+        self.close()
+    }
+
+    fn view_nodes(mut self, depth: Int, nodes: Vec<Spanned<ViewNode>>) {
+        for node in nodes.iter() {
+            match node.value {
+                ViewNodeElement(name, args, children) => {
+                    self.open(depth, "ViewNode.Element", Some(node.span))
+                    self.name_field("name", name)
+                    let some = match children {
+                        Some(_) => "Some"
+                        None => "None"
+                    }
+                    self.field("children", some)
+                    for arg in args.iter() {
+                        match arg {
+                            ViewArgNamed(arg_name, value) => {
+                                self.open(depth + 1, "ViewArg.Named", None)
+                                self.name_field("name", arg_name)
+                                self.expr(depth + 2, value)
+                            }
+                            ViewArgPositional(e) => {
+                                self.open(depth + 1, "ViewArg.Positional", None)
+                                self.expr(depth + 2, e)
+                            }
+                        }
+                        self.close()
+                    }
+                    match children {
+                        Some(c) => self.view_nodes(depth + 1, c)
+                        None => ()
+                    }
+                }
+                ViewNodeExpr(e) => {
+                    self.open(depth, "ViewNode.Expr", Some(node.span))
+                    self.expr(depth + 1, e)
+                }
+            }
+            self.close()
+        }
+    }
+
+    // --- Patterns ---
+
+    fn patterns(mut self, depth: Int, patterns: Vec<Spanned<Pattern>>) {
+        for p in patterns.iter() { self.pattern(depth, p) }
+    }
+
+    fn pattern(mut self, depth: Int, p: Spanned<Pattern>) {
+        let span = Some(p.span)
+        let inner = depth + 1
+        match p.value {
+            PatternWildcard => self.open(depth, "Pattern.Wildcard", span)
+            PatternLit(lit) => {
+                self.open(depth, "Pattern.Lit", span)
+                self.literal(lit)
+            }
+            PatternIdent(is_mut, name) => {
+                self.open(depth, "Pattern.Ident", span)
+                self.bool_field("is_mut", is_mut)
+                self.name_field("name", name)
+            }
+            PatternBinding(name, pat) => {
+                self.open(depth, "Pattern.Binding", span)
+                self.name_field("name", name)
+                self.pattern(inner, pat)
+            }
+            PatternTupleStruct(path, fields) => {
+                self.open(depth, "Pattern.TupleStruct", span)
+                self.path_field("path", path)
+                self.patterns(inner, fields)
+            }
+            PatternRecord(path, fields, rest) => {
+                self.open(depth, "Pattern.Record", span)
+                self.path_field("path", path)
+                self.bool_field("rest", rest)
+                for f in fields.iter() { self.field_pat(inner, f) }
+            }
+            PatternTuple(items) => {
+                self.open(depth, "Pattern.Tuple", span)
+                self.patterns(inner, items)
+            }
+            PatternArray(items) => {
+                self.open(depth, "Pattern.Array", span)
+                self.patterns(inner, items)
+            }
+            PatternOr(items) => {
+                self.open(depth, "Pattern.Or", span)
+                self.patterns(inner, items)
+            }
+            PatternRange(lo, hi, inclusive) => {
+                self.open(depth, "Pattern.Range", span)
+                self.bool_field("inclusive", inclusive)
+                self.pattern(inner, lo)
+                self.pattern(inner, hi)
+            }
+            PatternPath(path) => {
+                self.open(depth, "Pattern.Path", span)
+                self.out.push(" ")
+                self.path(path)
+            }
+        }
+        self.close()
+    }
+
+    fn field_pat(mut self, depth: Int, f: FieldPat) {
+        self.open(depth, "FieldPat", None)
+        self.name_field("name", f.name)
+        match f.pattern {
+            Some(p) => self.pattern(depth + 1, p)
+            None => ()
+        }
+        self.close()
+    }
+
+    // --- Types ---
+
+    fn ty(mut self, depth: Int, t: Spanned<Type>) {
+        let span = Some(t.span)
+        let inner = depth + 1
+        match t.value {
+            TypePath(path, args) => {
+                self.open(depth, "Type.Path", span)
+                self.path_field("path", path)
+                for a in args.iter() { self.ty(inner, a) }
+            }
+            TypeRef(is_mut, ty) => {
+                self.open(depth, "Type.Ref", span)
+                self.bool_field("is_mut", is_mut)
+                self.ty(inner, ty)
+            }
+            TypePtr(is_mut, ty) => {
+                self.open(depth, "Type.Ptr", span)
+                self.bool_field("is_mut", is_mut)
+                self.ty(inner, ty)
+            }
+            TypeArray(ty) => {
+                self.open(depth, "Type.Array", span)
+                self.ty(inner, ty)
+            }
+            TypeTuple(items) => {
+                self.open(depth, "Type.Tuple", span)
+                for ty in items.iter() { self.ty(inner, ty) }
+            }
+            TypeFn(params, ret) => {
+                self.open(depth, "Type.Fn", span)
+                for ty in params.iter() { self.ty(inner, ty) }
+                self.ty(inner, ret)
+            }
+            TypeOptional(ty) => {
+                self.open(depth, "Type.Optional", span)
+                self.ty(inner, ty)
+            }
+            TypeInfer => self.open(depth, "Type.Infer", span)
+        }
+        self.close()
+    }
+}
+
+// The field-less enums' values, as the Rust `Debug` names them.
+
+fn visibility_name(v: Visibility) -> String {
+    match v {
+        VisibilityPub => "Pub"
+        VisibilityPrivate => "Private"
+    }
+}
+
+fn split_kind_name(k: SplitKind) -> String {
+    match k {
+        SplitKindCall => "Call"
+        SplitKindIndex => "Index"
+        SplitKindMinus => "Minus"
+        SplitKindValue => "Value"
+    }
+}
+
+fn bin_op_name(op: BinOp) -> String {
+    match op {
+        BinOpOr => "Or"
+        BinOpAnd => "And"
+        BinOpEq => "Eq"
+        BinOpNe => "Ne"
+        BinOpLt => "Lt"
+        BinOpLe => "Le"
+        BinOpGt => "Gt"
+        BinOpGe => "Ge"
+        BinOpBitOr => "BitOr"
+        BinOpBitXor => "BitXor"
+        BinOpBitAnd => "BitAnd"
+        BinOpShl => "Shl"
+        BinOpShr => "Shr"
+        BinOpAdd => "Add"
+        BinOpSub => "Sub"
+        BinOpMul => "Mul"
+        BinOpDiv => "Div"
+        BinOpRem => "Rem"
+    }
+}
+
+fn un_op_name(op: UnOp) -> String {
+    match op {
+        UnOpNeg => "Neg"
+        UnOpNot => "Not"
+        UnOpBitNot => "BitNot"
+        UnOpRef => "Ref"
+        UnOpRefMut => "RefMut"
+        UnOpDeref => "Deref"
+    }
+}
+
+fn assign_op_name(op: AssignOp) -> String {
+    match op {
+        AssignOpAssign => "Assign"
+        AssignOpAddAssign => "AddAssign"
+        AssignOpSubAssign => "SubAssign"
+        AssignOpMulAssign => "MulAssign"
+        AssignOpDivAssign => "DivAssign"
+        AssignOpRemAssign => "RemAssign"
+        AssignOpBitOrAssign => "BitOrAssign"
+        AssignOpBitAndAssign => "BitAndAssign"
+        AssignOpBitXorAssign => "BitXorAssign"
+        AssignOpShlAssign => "ShlAssign"
+        AssignOpShrAssign => "ShrAssign"
+    }
+}
diff --git a/selfhost/src/dump.nova b/selfhost/src/dump.nova
index 57e1f35c4138b94b605c99bfc313ba64f3532120..10e3e8b360e4b2f56f57ff63fd67748228d8bc1c 100644
--- a/selfhost/src/dump.nova
+++ b/selfhost/src/dump.nova
@@ -70,7 +70,7 @@ fn push_quoted(mut out: StringBuilder, kind: String, text: String) {
 // hex digits, and, when `quoted`, `"` and `\` too. Every other byte is kept,
 // so a non-ASCII character prints as it is. The bytes escaped are all ASCII,
 // so the runs between them are whole characters.
-fn push_escaped(mut out: StringBuilder, text: String, quoted: Bool) {
+pub fn push_escaped(mut out: StringBuilder, text: String, quoted: Bool) {
     let n = text.byte_len()
     let mut run = 0
     let mut i = 0
@@ -89,7 +89,7 @@ fn push_escaped(mut out: StringBuilder, text: String, quoted: Bool) {
 }
 
 // The 64 bits of `bits` as 16 lowercase hex digits, the sign bit first.
-fn hex16(bits: Int) -> String {
+pub fn hex16(bits: Int) -> String {
     let mut out = StringBuilder::new()
     let mut shift = 60
     while shift >= 0 {
diff --git a/selfhost/src/lexer.nova b/selfhost/src/lexer.nova
index a41bc7563cbad7622869bf12549cd1cee7ef5fba..798a555d8742e04427033395d4ea31f6f7d9629f 100644
--- a/selfhost/src/lexer.nova
+++ b/selfhost/src/lexer.nova
@@ -115,6 +115,43 @@ impl Display for LexError {
 // lib.rs: comments, and the lexer's entry points
 // ---------------------------------------------------------------------------
 
+/// Every keyword, in the order the lexer declares its tokens (lib.rs's
+/// `KEYWORDS`). The parser reads one as an element's argument name.
+pub const KEYWORDS: [String] = [
+    "let",
+    "mut",
+    "const",
+    "fn",
+    "return",
+    "if",
+    "else",
+    "while",
+    "for",
+    "in",
+    "break",
+    "continue",
+    "match",
+    "type",
+    "record",
+    "trait",
+    "impl",
+    "import",
+    "module",
+    "pub",
+    "async",
+    "await",
+    "extern",
+    "unsafe",
+    "true",
+    "false",
+    "as",
+    "is",
+    "where",
+    "with",
+    "self",
+    "Self",
+]
+
 /// Which kind of comment (lib.rs's `CommentKind`).
 pub type CommentKind =
     /// `// …`, including `//// …`.
@@ -149,13 +186,13 @@ pub fn lex_with_comments(source: String) -> (Tokens, Vec<Comment>, Vec<LexError>
 /// The tokens of one source, ending with `Eof`, each marked with whether a
 /// line break comes before it, and whether it is an integer in plain decimal
 /// digits (tokens.rs's `Tokens`).
-pub record Tokens { pub tokens: Vec<Spanned>, breaks: Vec<Bool>, plain: Vec<Bool> }
+pub record Tokens { pub tokens: Vec<Spanned<Token>>, breaks: Vec<Bool>, plain: Vec<Bool> }
 
 /// `tokens`, lexed from `source`, each marked by the source text between it
 /// and the token before it, or the start of the source (tokens.rs's
 /// `Tokens::marked`). A line break is a `\n` or a `\r`, and one inside a
 /// comment counts.
-pub fn marked(tokens: Vec<Spanned>, source: String) -> Tokens {
+pub fn marked(tokens: Vec<Spanned<Token>>, source: String) -> Tokens {
     let n = source.byte_len()
     let mut prev_end = 0
     let mut breaks: Vec<Bool> = Vec::new()
@@ -181,7 +218,7 @@ pub fn marked(tokens: Vec<Spanned>, source: String) -> Tokens {
 
 // An `Int` whose text is ASCII digits with no leading zero, unless it is
 // exactly `0`.
-fn is_plain_decimal(t: Spanned, source: String) -> Bool {
+fn is_plain_decimal(t: Spanned<Token>, source: String) -> Bool {
     let is_int = match t.value {
         Int(_) => true
         _ => false
@@ -203,7 +240,7 @@ impl Tokens {
     pub fn len(self) -> Int { self.tokens.len() }
 
     /// The token at `index`, which must be less than `len()`.
-    pub fn at(self, index: Int) -> Spanned { self.tokens.at(index) }
+    pub fn at(self, index: Int) -> Spanned<Token> { self.tokens.at(index) }
 
     /// Whether a line break comes before the token at `index`; `false` past
     /// the end (tokens.rs's `line_break_before`).
@@ -259,8 +296,8 @@ pub fn new_lexer(source: String) -> Lexer {
 
 impl Lexer {
     /// Every token, `Eof` last, and every error (lib.rs's `tokenize`).
-    pub fn tokenize(mut self) -> (Vec<Spanned>, Vec<LexError>) {
-        let mut tokens: Vec<Spanned> = Vec::new()
+    pub fn tokenize(mut self) -> (Vec<Spanned<Token>>, Vec<LexError>) {
+        let mut tokens: Vec<Spanned<Token>> = Vec::new()
         let mut errors: Vec<LexError> = Vec::new()
         while self.pos < self.len {
             match self.next_token() {
@@ -285,8 +322,8 @@ impl Lexer {
     // fitting an `Int`, becomes an integer, a `.` and an integer, so `t.1.0`
     // reads as `(t.1).0` (lib.rs's `split_tuple_indexes`). It splits by the
     // text, not the value.
-    fn split_tuple_indexes(self, tokens: Vec<Spanned>) -> Vec<Spanned> {
-        let mut out: Vec<Spanned> = Vec::new()
+    fn split_tuple_indexes(self, tokens: Vec<Spanned<Token>>) -> Vec<Spanned<Token>> {
+        let mut out: Vec<Spanned<Token>> = Vec::new()
         for tok in tokens.iter() {
             let after_dot = !out.is_empty() && is_dot(out.last().value)
             if after_dot && is_float(tok.value) {
@@ -393,7 +430,7 @@ impl Lexer {
     }
 
     // The next token, nothing, or an error (lib.rs's `next_token`).
-    fn next_token(mut self) -> Result<Option<Spanned>, LexError> {
+    fn next_token(mut self) -> Result<Option<Spanned<Token>>, LexError> {
         // Inside a string, whitespace is text: lex the content before any
         // skipping.
         if self.in_string {
@@ -412,7 +449,7 @@ impl Lexer {
     }
 
     // lib.rs's `lex_string_start`.
-    fn lex_string_start(mut self) -> Result<Option<Spanned>, LexError> {
+    fn lex_string_start(mut self) -> Result<Option<Spanned<Token>>, LexError> {
         let start = self.pos
         self.pos += 1
         self.in_string = true
@@ -423,7 +460,7 @@ impl Lexer {
     // its escapes decoded (lib.rs's `lex_string_content`). Runs of plain text
     // are copied a run at a time rather than a character at a time, which
     // gives the same text.
-    fn lex_string_content(mut self) -> Result<Option<Spanned>, LexError> {
+    fn lex_string_content(mut self) -> Result<Option<Spanned<Token>>, LexError> {
         let src = self.source
         let n = self.len
         let start = self.pos
@@ -538,7 +575,7 @@ impl Lexer {
     // `r"…"` and `r#"…"#`, with any number of `#`s (lib.rs's
     // `lex_raw_string`). An `r#` with no `"` after its `#`s is the one
     // source of `InvalidNumberLiteral`.
-    fn lex_raw_string(mut self) -> Result<Option<Spanned>, LexError> {
+    fn lex_raw_string(mut self) -> Result<Option<Spanned<Token>>, LexError> {
         let src = self.source
         let n = self.len
         let start = self.pos
@@ -579,7 +616,7 @@ impl Lexer {
     // A `}` closes the innermost hole only when that hole has no unmatched
     // `{` of its own; otherwise it is an ordinary `RBrace` (lib.rs's
     // `maybe_close_interp`).
-    fn maybe_close_interp(mut self, start: Int) -> Spanned {
+    fn maybe_close_interp(mut self, start: Int) -> Spanned<Token> {
         if self.interp.is_empty() { return spanned(RBrace, start, self.pos) }
         let mut frame = self.interp.last()
         if frame.braces > 0 {
@@ -594,7 +631,7 @@ impl Lexer {
     // What logos recognizes, at `pos` (lib.rs's `lex_logos`). Braces are
     // the one pair that interacts with interpolation: a `{` opened inside a
     // hole must be matched before the hole's own `}`.
-    fn lex_logos(mut self) -> Result<Option<Spanned>, LexError> {
+    fn lex_logos(mut self) -> Result<Option<Spanned<Token>>, LexError> {
         let start = self.pos
         match self.raw_token(start) {
             Some((tok, end)) => {
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index ba9719d0dc41784071ab650ef59fe5017eeb0e47..d8c53adccfb9dec0f9ef0a39e745329a64a76901 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -1,25 +1,32 @@
-// N, the self-hosted Nova compiler, so far its lexer (spec
+// N, the self-hosted Nova compiler, so far its lexer and its parser (spec
 // docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md
-// §3.2, ADR 0051).
+// §3.2 and §4.3, ADR 0051 and ADR 0052).
 //
 //   selfhost tokens FILE...   prints each file's token dump, after a header
 //                             line `== FILE`, as `nova parse --dump-tokens`
 //                             does
+//   selfhost ast FILE...      prints each file's AST dump, its tree and its
+//                             errors, after the same header, as
+//                             `nova parse --dump-ast` does
 //
-// A file with lex errors is dumped with them, and the exit code stays 0. A
-// file that cannot be read prints `IOERR` and its kind after its header;
-// the rest are dumped, then the exit code is 2.
+// A file with lex or parse errors is dumped with them, and the exit code
+// stays 0. A file that cannot be read prints `IOERR` and its kind after its
+// header; the rest are dumped, then the exit code is 2.
 
+import ast_dump
 import dump
 import lexer
 import lexer_tests
+import parser_tests
 import token
 
 async fn main() {
     let argv = args()
     // No file is refused, as S0's clap refuses it.
-    if argv.len() < 3 || argv.at(1) != "tokens" {
-        eprintln("usage: selfhost tokens FILE...")
+    let command = if argv.len() < 3 { "" } else { argv.at(1) }
+    let tokens = command == "tokens"
+    if !tokens && command != "ast" {
+        eprintln("usage: selfhost tokens|ast FILE...")
         exit(2)
         return
     }
@@ -28,7 +35,10 @@ async fn main() {
     while i < argv.len() {
         let path = argv.at(i)
         match read_to_string(path).await {
-            Ok(source) => print("== ${path}\n${dump_tokens(source)}")
+            Ok(source) => {
+                let out = if tokens { dump_tokens(source) } else { dump_ast(source) }
+                print("== ${path}\n${out}")
+            }
             Err(e) => {
                 print("== ${path}\nIOERR ${io_kind_name(e.kind)}\n")
                 failed = true
diff --git a/selfhost/src/parse_expr.nova b/selfhost/src/parse_expr.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d240140cc18f64ff3e2ab872098f70215da4b397
--- /dev/null
+++ b/selfhost/src/parse_expr.nova
@@ -0,0 +1,913 @@
+// Expressions: a port of crates/nova-parser/src/grammar.rs's "Expressions
+// (precedence climbing)" section, but for its views (parse_view.nova),
+// function by function (parser.nova says how its idioms translate). The
+// precedence ladder is a function a level, as in Rust: `parse_expr` →
+// `parse_assign` → `parse_range` → `parse_or` → `parse_and` →
+// `parse_compare` → `parse_bitor` → `parse_bitxor` → `parse_bitand` →
+// `parse_shift` → `parse_add` → `parse_mul` → `parse_cast` →
+// `parse_unary` → `parse_postfix` → `parse_atom`.
+
+import ast
+import parse_pattern
+import parse_stmt
+import parse_type
+import parse_view
+import parser
+import token
+
+impl Parser {
+    pub fn parse_expr(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        // `nested`
+        let depth = self.depth
+        let expr = if self.deeper() { self.parse_assign(ctx) } else { None }
+        self.depth = depth
+        self.end_halt()
+        expr
+    }
+
+    /// The expression that holds a bare `return` or `break` ends here: what
+    /// follows reads the token again.
+    pub fn end_halt(mut self) { if self.halted() { self.halt = -1 } }
+
+    pub fn parse_assign(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let lhs = self.parse_range(ctx)?
+
+        let op = match self.peek() {
+            Eq => Some(AssignOpAssign)
+            PlusEq => Some(AssignOpAddAssign)
+            MinusEq => Some(AssignOpSubAssign)
+            StarEq => Some(AssignOpMulAssign)
+            SlashEq => Some(AssignOpDivAssign)
+            PercentEq => Some(AssignOpRemAssign)
+            PipeEq => Some(AssignOpBitOrAssign)
+            AmpEq => Some(AssignOpBitAndAssign)
+            CaretEq => Some(AssignOpBitXorAssign)
+            LtLtEq => Some(AssignOpShlAssign)
+            GtGtEq => Some(AssignOpShrAssign)
+            _ => None
+        }
+
+        match op {
+            Some(op) => {
+                let _ = self.advance()
+                if !self.deeper() { return None }
+                // right-associative
+                let rhs = self.parse_assign(ctx)?
+                self.depth = depth
+                let span = start.merge(rhs.span)
+                Some(Spanned::new(ExprAssign(op, lhs, rhs), span))
+            }
+            None => Some(lhs)
+        }
+    }
+
+    /// Range expressions `lo..hi` / `lo..=hi`, just above assignment in
+    /// precedence and non-chainable.
+    pub fn parse_range(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let lo = self.parse_or(ctx)?
+        let inclusive = if self.eat(DotDotEq).is_some() {
+            true
+        } else if self.eat(DotDot).is_some() {
+            false
+        } else {
+            return Some(lo)
+        }
+        if !self.deeper() { return None }
+        let hi = self.parse_or(ctx)?
+        self.depth = depth
+        let span = start.merge(hi.span)
+        Some(Spanned::new(ExprRange(lo, hi, inclusive), span))
+    }
+
+    pub fn parse_or(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let mut lhs = self.parse_and(ctx)?
+        while self.eat(PipePipe).is_some() {
+            if !self.deeper() { return None }
+            let rhs = self.parse_and(ctx)?
+            let span = start.merge(rhs.span)
+            lhs = Spanned::new(ExprBinary(BinOpOr, lhs, rhs), span)
+        }
+        self.depth = depth
+        Some(lhs)
+    }
+
+    pub fn parse_and(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let mut lhs = self.parse_compare(ctx)?
+        while self.eat(AmpAmp).is_some() {
+            if !self.deeper() { return None }
+            let rhs = self.parse_compare(ctx)?
+            let span = start.merge(rhs.span)
+            lhs = Spanned::new(ExprBinary(BinOpAnd, lhs, rhs), span)
+        }
+        self.depth = depth
+        Some(lhs)
+    }
+
+    pub fn parse_compare(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let lhs = self.parse_bitor(ctx)?
+
+        let op = match self.peek() {
+            EqEq => Some(BinOpEq)
+            BangEq => Some(BinOpNe)
+            Lt => Some(BinOpLt)
+            LtEq => Some(BinOpLe)
+            Gt => Some(BinOpGt)
+            GtEq => Some(BinOpGe)
+            _ => None
+        }
+
+        match op {
+            Some(op) => {
+                let _ = self.advance()
+                if !self.deeper() { return None }
+                let rhs = self.parse_bitor(ctx)?
+                self.depth = depth
+                let span = start.merge(rhs.span)
+                let result = Spanned::new(ExprBinary(op, lhs, rhs), span)
+                // Non-chainable: `a < b < c` is an error
+                match self.peek() {
+                    EqEq | BangEq | Lt | LtEq | Gt | GtEq => {
+                        let chain_span = self.peek_span()
+                        self.errors.push(ChainedComparison(chain_span))
+                    }
+                    _ => ()
+                }
+                Some(result)
+            }
+            None => Some(lhs)
+        }
+    }
+
+    pub fn parse_bitor(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let mut lhs = self.parse_bitxor(ctx)?
+        while self.eat(Pipe).is_some() {
+            if !self.deeper() { return None }
+            let rhs = self.parse_bitxor(ctx)?
+            let span = start.merge(rhs.span)
+            lhs = Spanned::new(ExprBinary(BinOpBitOr, lhs, rhs), span)
+        }
+        self.depth = depth
+        Some(lhs)
+    }
+
+    pub fn parse_bitxor(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let mut lhs = self.parse_bitand(ctx)?
+        while self.eat(Caret).is_some() {
+            if !self.deeper() { return None }
+            let rhs = self.parse_bitand(ctx)?
+            let span = start.merge(rhs.span)
+            lhs = Spanned::new(ExprBinary(BinOpBitXor, lhs, rhs), span)
+        }
+        self.depth = depth
+        Some(lhs)
+    }
+
+    pub fn parse_bitand(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let mut lhs = self.parse_shift(ctx)?
+        while self.eat(Amp).is_some() {
+            if !self.deeper() { return None }
+            let rhs = self.parse_shift(ctx)?
+            let span = start.merge(rhs.span)
+            lhs = Spanned::new(ExprBinary(BinOpBitAnd, lhs, rhs), span)
+        }
+        self.depth = depth
+        Some(lhs)
+    }
+
+    pub fn parse_shift(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let mut lhs = self.parse_add(ctx)?
+        while true {
+            let op = match self.peek() {
+                LtLt => BinOpShl
+                GtGt => BinOpShr
+                _ => break
+            }
+            let _ = self.advance()
+            if !self.deeper() { return None }
+            let rhs = self.parse_add(ctx)?
+            let span = start.merge(rhs.span)
+            lhs = Spanned::new(ExprBinary(op, lhs, rhs), span)
+        }
+        self.depth = depth
+        Some(lhs)
+    }
+
+    pub fn parse_add(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let mut lhs = self.parse_mul(ctx)?
+        while true {
+            let op = match self.peek() {
+                Plus => BinOpAdd
+                // A `-` after a line break in statement context begins the
+                // next statement.
+                Minus if self.newline_ends_expr() => {
+                    self.record_split(SplitKindMinus)
+                    break
+                }
+                Minus => BinOpSub
+                _ => break
+            }
+            let _ = self.advance()
+            if !self.deeper() { return None }
+            let rhs = self.parse_mul(ctx)?
+            let span = start.merge(rhs.span)
+            lhs = Spanned::new(ExprBinary(op, lhs, rhs), span)
+        }
+        self.depth = depth
+        Some(lhs)
+    }
+
+    pub fn parse_mul(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let mut lhs = self.parse_cast(ctx)?
+        while true {
+            let op = match self.peek() {
+                Star => BinOpMul
+                Slash => BinOpDiv
+                Percent => BinOpRem
+                _ => break
+            }
+            let _ = self.advance()
+            if !self.deeper() { return None }
+            let rhs = self.parse_cast(ctx)?
+            let span = start.merge(rhs.span)
+            lhs = Spanned::new(ExprBinary(op, lhs, rhs), span)
+        }
+        self.depth = depth
+        Some(lhs)
+    }
+
+    pub fn parse_cast(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let depth = self.depth
+        let mut expr = self.parse_unary(ctx)?
+        while true {
+            if self.eat(As).is_some() {
+                if !self.deeper() { return None }
+                let ty = self.parse_type_here(ctx)?
+                let span = start.merge(ty.span)
+                expr = Spanned::new(ExprCast(expr, ty), span)
+            } else {
+                break
+            }
+        }
+        self.depth = depth
+        Some(expr)
+    }
+
+    pub fn parse_unary(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let op = match self.peek() {
+            Minus => Some(UnOpNeg)
+            Bang => Some(UnOpNot)
+            Tilde => Some(UnOpBitNot)
+            Star => Some(UnOpDeref)
+            Amp => {
+                // Could be `&` or `&mut`
+                let _ = self.advance()
+                let is_mut = self.eat(Mut).is_some()
+                // `nested`
+                let depth = self.depth
+                let expr = if self.deeper() { self.parse_postfix(ctx) } else { None }
+                self.depth = depth
+                let expr = expr?
+                let span = start.merge(expr.span)
+                let op = if is_mut { UnOpRefMut } else { UnOpRef }
+                return Some(Spanned::new(ExprUnary(op, expr), span))
+            }
+            _ => None
+        }
+
+        match op {
+            Some(op) => {
+                let _ = self.advance()
+                // `nested`
+                let depth = self.depth
+                let expr = if self.deeper() { self.parse_unary(ctx) } else { None }
+                self.depth = depth
+                let expr = expr?
+                let span = start.merge(expr.span)
+                Some(Spanned::new(ExprUnary(op, expr), span))
+            }
+            None => self.parse_postfix(ctx)
+        }
+    }
+
+    pub fn parse_postfix(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let expr = self.parse_atom(ctx)?
+        self.parse_postfix_from(start, expr, ctx)
+    }
+
+    /// The calls, indexes, fields, `.await`s and `?`s after `expr`, which
+    /// began at `start`: each after the first one level deeper than the one
+    /// before it, for the rest of the chain, a method call `.name(…)` one
+    /// operator (`deeper`); the depth restored after the chain however it
+    /// ends.
+    pub fn parse_postfix_from(
+        mut self,
+        start: Span,
+        expr: Spanned<Expr>,
+        ctx: String,
+    ) -> Option<Spanned<Expr>> {
+        let depth = self.depth
+        let expr = self.parse_postfix_chain(start, expr, ctx)
+        self.depth = depth
+        expr
+    }
+
+    /// `parse_postfix_from`'s operators, the depth raised after each but the
+    /// first, as after a binary operator: past the limit, at the token after
+    /// it.
+    pub fn parse_postfix_chain(
+        mut self,
+        start: Span,
+        expr: Spanned<Expr>,
+        ctx: String,
+    ) -> Option<Spanned<Expr>> {
+        let mut expr = expr
+        let mut first = true
+        let mut after_dot = false
+        while true {
+            let cur_span = expr.span
+            let op = self.peek()
+            match op {
+                // A `(` or `[` after a line break in statement context
+                // begins the next statement.
+                LParen | LBracket if self.newline_ends_expr() => {
+                    let kind = if self.check(LParen) { SplitKindCall } else { SplitKindIndex }
+                    self.record_split(kind)
+                    break
+                }
+                LParen | LBracket | Dot | Question => ()
+                _ => break
+            }
+            let _ = self.advance()
+            // A method call's `(…)` counts with its `.name`.
+            let method_call = match op {
+                LParen => after_dot
+                _ => false
+            }
+            after_dot = match op {
+                Dot => true
+                _ => false
+            }
+            if !first && !method_call && !self.deeper() { return None }
+            first = false
+            match op {
+                LParen => {
+                    // Function call
+                    // `in_stmt_ctx(false)`
+                    let prev = self.enter_stmt_ctx(false)
+                    let mut args: Vec<Spanned<Expr>> = Vec::new()
+                    while !self.check(RParen) && !self.is_at_end() {
+                        match self.parse_expr(ctx) {
+                            Some(a) => args.push(a)
+                            None => ()
+                        }
+                        if self.eat(Comma).is_none() { break }
+                    }
+                    self.stmt_ctx = prev
+                    let _ = self.expect(RParen, "call arguments")?
+                    let end = self.prev_span_or(cur_span)
+                    expr = Spanned::new(ExprCall(expr, args), start.merge(end))
+                }
+                LBracket => {
+                    // Index
+                    // `in_stmt_ctx(false)`
+                    let prev = self.enter_stmt_ctx(false)
+                    let index = self.parse_expr(ctx)
+                    self.stmt_ctx = prev
+                    let index = index?
+                    let _ = self.expect(RBracket, "index expression")?
+                    let end = self.prev_span_or(cur_span)
+                    expr = Spanned::new(ExprIndex(expr, index), start.merge(end))
+                }
+                Dot => {
+                    // `.await` or `.field` or `.method()`
+                    if self.eat(Await).is_some() {
+                        let end = self.prev_span_or(cur_span)
+                        expr = Spanned::new(ExprAwait(expr), start.merge(end))
+                    } else {
+                        match self.peek() {
+                            Int(n) => {
+                                // `t.N`, a tuple's element: plain decimal
+                                // digits only, so `t.01`, `t.1_0` and `t.0x1`
+                                // are refused, as Rust does.
+                                let span = self.peek_span()
+                                if !self.tokens.plain_decimal(self.pos) {
+                                    self.errors.push(
+                                        Custom(
+                                            "a tuple index is plain decimal digits, with no leading "
+                                                + "zero, `_` or prefix: write `.${n}`",
+                                            span,
+                                        ),
+                                    )
+                                }
+                                let _ = self.advance()
+                                expr = Spanned::new(
+                                    ExprField(expr, Spanned::new("${n}", span)),
+                                    start.merge(span),
+                                )
+                            }
+                            Ident(_) | SelfLower | SelfUpper => {
+                                let field = self.parse_ident("field access")?
+                                let end = field.span
+                                expr = Spanned::new(ExprField(expr, field), start.merge(end))
+                            }
+                            _ => {
+                                // An unfinished `foo.` keeps its receiver, so
+                                // an editor can complete after the `.`. The
+                                // error is the one `parse_ident` reports.
+                                self.errors.push(
+                                    Expected(
+                                        "identifier (in field access)",
+                                        self.peek().description(),
+                                        self.peek_span(),
+                                    ),
+                                )
+                                let dot_end = self.tokens.at(self.pos - 1).span.end
+                                let gap = Span { start: dot_end, end: dot_end }
+                                expr = Spanned::new(
+                                    ExprField(expr, Spanned::new("", gap)),
+                                    start.merge(gap),
+                                )
+                                break
+                            }
+                        }
+                    }
+                }
+                _ => {
+                    // `?`
+                    let end = self.prev_span_or(cur_span)
+                    expr = Spanned::new(ExprTry(expr), start.merge(end))
+                }
+            }
+        }
+
+        Some(expr)
+    }
+
+    pub fn parse_atom(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        match self.peek() {
+            // Literals
+            Int(n) => {
+                let _ = self.advance()
+                Some(Spanned::new(ExprLit(LiteralInt(n)), start))
+            }
+            Float(f) => {
+                let _ = self.advance()
+                Some(Spanned::new(ExprLit(LiteralFloat(f)), start))
+            }
+            Char(c) => {
+                let _ = self.advance()
+                Some(Spanned::new(ExprLit(LiteralChar(c)), start))
+            }
+            True => {
+                let _ = self.advance()
+                Some(Spanned::new(ExprLit(LiteralBool(true)), start))
+            }
+            False => {
+                let _ = self.advance()
+                Some(Spanned::new(ExprLit(LiteralBool(false)), start))
+            }
+
+            // String interpolation
+            StrStart => self.parse_string_expr()
+            RawStr(s) => {
+                let _ = self.advance()
+                Some(Spanned::new(ExprLit(LiteralStr(s)), start))
+            }
+
+            // Parenthesised / tuple: `in_stmt_ctx(false)`
+            LParen => {
+                let prev = self.enter_stmt_ctx(false)
+                let e = self.parse_paren(start, ctx)
+                self.stmt_ctx = prev
+                e
+            }
+
+            // Array literal, `[a, b, c]` or the repeat form `[init; n]`:
+            // `in_stmt_ctx(false)`
+            LBracket => {
+                let prev = self.enter_stmt_ctx(false)
+                let e = self.parse_array_literal(start, ctx)
+                self.stmt_ctx = prev
+                e
+            }
+
+            // Block expression
+            LBrace => {
+                let block = self.parse_block(ctx)?
+                let span = block.span
+                Some(Spanned::new(ExprBlock(block.value), span))
+            }
+
+            // If
+            If => self.parse_if_expr(ctx)
+
+            // Match
+            Match => self.parse_match_expr(ctx)
+
+            // While
+            While => {
+                let _ = self.advance()
+                // `in_head_ctx`
+                let ns = self.enter_no_struct_ctx(true)
+                let sc = self.enter_stmt_ctx(false)
+                let cond = self.parse_expr(ctx)
+                self.stmt_ctx = sc
+                self.no_struct_literal = ns
+                let cond = cond?
+                let body = self.parse_block(ctx)?
+                let span = start.merge(body.span)
+                Some(Spanned::new(ExprWhile(cond, body), span))
+            }
+
+            // For
+            For => {
+                let _ = self.advance()
+                let pattern = self.parse_pattern(ctx)?
+                let _ = self.expect(In, "for loop")?
+                // `in_head_ctx`
+                let ns = self.enter_no_struct_ctx(true)
+                let sc = self.enter_stmt_ctx(false)
+                let iter = self.parse_expr(ctx)
+                self.stmt_ctx = sc
+                self.no_struct_literal = ns
+                let iter = iter?
+                let body = self.parse_block(ctx)?
+                let span = start.merge(body.span)
+                Some(Spanned::new(ExprFor(pattern, iter, body), span))
+            }
+
+            // Return
+            Return => {
+                let _ = self.advance()
+                // Optional expression; none at the end of a line.
+                let val = if !self.bare_keyword_ends() { Some(self.parse_expr(ctx)?) } else { None }
+                let end = match val {
+                    Some(e) => e.span
+                    None => start
+                }
+                Some(Spanned::new(ExprReturn(val), start.merge(end)))
+            }
+
+            // Break
+            Break => {
+                let _ = self.advance()
+                let val = if !self.bare_keyword_ends() { Some(self.parse_expr(ctx)?) } else { None }
+                let end = match val {
+                    Some(e) => e.span
+                    None => start
+                }
+                Some(Spanned::new(ExprBreak(val), start.merge(end)))
+            }
+
+            Continue => {
+                let _ = self.advance()
+                Some(Spanned::new(ExprContinue, start))
+            }
+
+            // Closure: |params| body, or `||` with none (RFC 0001).
+            Pipe | PipePipe => self.parse_closure(ctx)
+
+            // `view { … }`, where `view` is a keyword (RFC 0001).
+            Ident(w) if w == "view"
+                && !self.no_struct_literal
+                && self.value_at(self.pos + 1).kind() == LBrace.kind() => {
+                self.parse_view()
+            }
+
+            // Path or record literal
+            Ident(_) | SelfLower | SelfUpper => {
+                let path = self.parse_path(ctx)?
+                let path_end = self.prev_span_or(start)
+
+                // Record literal: `Type { field: val, ... }`, only when not
+                // in a scrutinee context (if/while/for/match conditions),
+                // where `{` is the block.
+                if self.check(LBrace) && !self.no_struct_literal {
+                    return self.parse_record_literal(path, start)
+                }
+
+                Some(Spanned::new(ExprPath(path), start.merge(path_end)))
+            }
+
+            _ => {
+                let span = self.peek_span()
+                self.errors.push(
+                    Expected("expression (in ${ctx})", self.peek().description(), span),
+                )
+                None
+            }
+        }
+    }
+
+    /// `()`, `(e)` or a tuple, at `(`, which began at `start`.
+    pub fn parse_paren(mut self, start: Span, ctx: String) -> Option<Spanned<Expr>> {
+        let _ = self.advance()
+        if self.eat(RParen).is_some() {
+            // Unit `()`
+            let end = self.prev_span_or(start)
+            return Some(Spanned::new(ExprTuple(Vec::new()), start.merge(end)))
+        }
+        let first = self.parse_expr(ctx)?
+        if self.eat(RParen).is_some() {
+            // Just a parenthesised expression
+            let end = self.prev_span_or(start)
+            return Some(Spanned::new(first.value, start.merge(end)))
+        }
+        // Tuple
+        let mut elems: Vec<Spanned<Expr>> = Vec::new()
+        elems.push(first)
+        while self.eat(Comma).is_some() && !self.check(RParen) {
+            match self.parse_expr(ctx) {
+                Some(e) => elems.push(e)
+                None => ()
+            }
+        }
+        let _ = self.expect(RParen, "tuple")?
+        let end = self.prev_span_or(start)
+        Some(Spanned::new(ExprTuple(elems), start.merge(end)))
+    }
+
+    /// An array literal, `[a, b, c]` or `[init; n]`, at `[`, which began at
+    /// `start`.
+    pub fn parse_array_literal(mut self, start: Span, ctx: String) -> Option<Spanned<Expr>> {
+        let _ = self.advance()
+        let mut elems: Vec<Spanned<Expr>> = Vec::new()
+        let mut first = true
+        while !self.check(RBracket) && !self.is_at_end() {
+            match self.parse_expr(ctx) {
+                Some(e) => {
+                    // `[init; n]`: a `;` after the *first* element switches
+                    // to the repeat form, whose second operand is a length
+                    // rather than another element.
+                    if first && self.eat(Semicolon).is_some() {
+                        let len = self.parse_expr(ctx)?
+                        let _ = self.expect(RBracket, "repeat array literal")?
+                        let end = self.prev_span_or(start)
+                        return Some(Spanned::new(ExprArrayRepeat(e, len), start.merge(end)))
+                    }
+                    elems.push(e)
+                }
+                None => ()
+            }
+            first = false
+            if self.eat(Comma).is_none() { break }
+        }
+        let _ = self.expect(RBracket, "array literal")?
+        let end = self.prev_span_or(start)
+        Some(Spanned::new(ExprArray(elems), start.merge(end)))
+    }
+
+    pub fn parse_string_expr(mut self) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let _ = self.expect(StrStart, "string literal")?
+        let mut parts: Vec<StringPart> = Vec::new()
+
+        while true {
+            match self.peek() {
+                StrEnd => {
+                    let _ = self.advance()
+                    break
+                }
+                StrPart(s) => {
+                    let _ = self.advance()
+                    parts.push(StringPartLit(s))
+                }
+                InterpOpen => {
+                    let _ = self.advance()
+                    // A hole is delimited, so a record literal inside it is
+                    // unambiguous even when the string itself sits in a
+                    // no-struct-literal position: `in_struct_ok_ctx`.
+                    let ns = self.enter_no_struct_ctx(false)
+                    let sc = self.enter_stmt_ctx(false)
+                    let expr = self.parse_expr("string interpolation")
+                    self.stmt_ctx = sc
+                    self.no_struct_literal = ns
+                    match expr {
+                        Some(e) => parts.push(StringPartExpr(e))
+                        None => ()
+                    }
+                    let _ = self.expect(InterpClose, "string interpolation")?
+                }
+                Eof => {
+                    let span = self.peek_span()
+                    self.errors.push(UnexpectedEof(span))
+                    break
+                }
+                _ => { let _ = self.advance() }
+            }
+        }
+
+        let end = self.prev_span_or(start)
+        Some(Spanned::new(ExprStringInterp(parts), start.merge(end)))
+    }
+
+    pub fn parse_if_expr(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let _ = self.expect(If, "if expression")?
+        // `in_head_ctx`
+        let ns = self.enter_no_struct_ctx(true)
+        let sc = self.enter_stmt_ctx(false)
+        let cond = self.parse_expr(ctx)
+        self.stmt_ctx = sc
+        self.no_struct_literal = ns
+        let cond = cond?
+        let then = self.parse_block(ctx)?
+        let else_ = if self.eat(Else).is_some() {
+            if self.check(If) {
+                // `nested`
+                let depth = self.depth
+                let e = if self.deeper() { self.parse_if_expr(ctx) } else { None }
+                self.depth = depth
+                Some(e?)
+            } else {
+                let b = self.parse_block(ctx)?
+                let span = b.span
+                Some(Spanned::new(ExprBlock(b.value), span))
+            }
+        } else {
+            None
+        }
+        let end = match else_ {
+            Some(e) => e.span
+            None => then.span
+        }
+        Some(Spanned::new(ExprIf(cond, then, else_), start.merge(end)))
+    }
+
+    pub fn parse_match_expr(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let _ = self.expect(Match, "match expression")?
+        // `in_head_ctx`
+        let ns = self.enter_no_struct_ctx(true)
+        let sc = self.enter_stmt_ctx(false)
+        let scrutinee = self.parse_expr(ctx)
+        self.stmt_ctx = sc
+        self.no_struct_literal = ns
+        let scrutinee = scrutinee?
+        let _ = self.expect(LBrace, "match body")?
+        let mut arms: Vec<MatchArm> = Vec::new()
+        while !self.check(RBrace) && !self.is_at_end() {
+            if self.reject_docs() { continue }
+            match self.parse_match_arm(ctx) {
+                Some(arm) => arms.push(arm)
+                None => self.sync_to_stmt_boundary()
+            }
+        }
+        let _ = self.expect(RBrace, "match body")?
+        let end = self.prev_span_or(start)
+        Some(Spanned::new(ExprMatch(scrutinee, arms), start.merge(end)))
+    }
+
+    pub fn parse_match_arm(mut self, ctx: String) -> Option<MatchArm> {
+        // The split that ended the arm before, if this arm begins at it.
+        let start = self.peek_span().start
+        let begun = match self.arm_end_split {
+            Some(s) if s.at.start == start => Some(s)
+            _ => None
+        }
+        self.arm_end_split = None
+        let errors = self.errors.len()
+        let (pattern, guard) = match self.parse_match_arm_head(ctx) {
+            Some(head) => head
+            None => {
+                self.note_arm_split(begun, errors)
+                return None
+            }
+        }
+        // An arm's body is statement context, so a line that starts with
+        // `(` begins the next arm: `in_stmt_ctx(true)`.
+        let splits = self.splits.len()
+        let prev = self.enter_stmt_ctx(true)
+        let body = self.parse_expr(ctx)
+        self.stmt_ctx = prev
+        let body = body?
+        // A split at the body's end begins the next arm: no warning for it,
+        // however many times it was met. Where the parser before the rule
+        // read that line into the body, the next arm cannot parse, and its
+        // P0001 gets a note naming the rule.
+        let end = self.peek_span().start
+        self.arm_end_split = None
+        while self.splits.len() > splits && self.splits.last().at.start == end {
+            self.arm_end_split = self.splits.pop()
+        }
+        let _ = self.eat(Comma)
+        Some(MatchArm { pattern: pattern, guard: guard, body: body })
+    }
+
+    /// An arm's pattern, its guard if any, and its `=>`.
+    pub fn parse_match_arm_head(
+        mut self,
+        ctx: String,
+    ) -> Option<(Spanned<Pattern>, Option<Spanned<Expr>>)> {
+        let pattern = self.parse_pattern(ctx)?
+        // A guard ends at its `=>`, as a head ends at its `{`:
+        // `in_stmt_ctx(false)`.
+        let guard = if self.eat(If).is_some() {
+            let prev = self.enter_stmt_ctx(false)
+            let g = self.parse_expr(ctx)
+            self.stmt_ctx = prev
+            Some(g?)
+        } else {
+            None
+        }
+        let _ = self.expect(FatArrow, "match arm")?
+        Some((pattern, guard))
+    }
+
+    pub fn parse_closure(mut self, ctx: String) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        let mut params: Vec<Param> = Vec::new()
+        if self.eat(PipePipe).is_none() {
+            let _ = self.expect(Pipe, "closure")?
+            let _ = self.parse_closure_params(params, ctx)?
+        }
+        let ret = if self.eat(Arrow).is_some() { Some(self.parse_type(ctx)?) } else { None }
+        let body = self.parse_expr(ctx)?
+        let span = start.merge(body.span)
+        Some(Spanned::new(ExprClosure(params, ret, body), span))
+    }
+
+    /// A closure's parameters, after its `|`, up to and with its closing
+    /// `|`.
+    pub fn parse_closure_params(mut self, mut params: Vec<Param>, ctx: String) -> Option<()> {
+        while !self.check(Pipe) && !self.is_at_end() {
+            if self.reject_docs() { continue }
+            let is_mut = self.eat(Mut).is_some()
+            let name = self.parse_ident("closure parameter")?
+            let ty = if self.eat(Colon).is_some() {
+                self.parse_type(ctx)?
+            } else {
+                // Inferred type
+                let span = name.span
+                Spanned::new(TypeInfer, span)
+            }
+            params.push(Param { is_mut: is_mut, name: name, ty: ty })
+            if self.eat(Comma).is_none() { break }
+        }
+        let _ = self.expect(Pipe, "closure parameters end")?
+        Some(())
+    }
+
+    pub fn parse_record_literal(mut self, path: Path, start: Span) -> Option<Spanned<Expr>> {
+        // `in_stmt_ctx(false)`
+        let prev = self.enter_stmt_ctx(false)
+        let e = self.parse_record_literal_inner(path, start)
+        self.stmt_ctx = prev
+        e
+    }
+
+    pub fn parse_record_literal_inner(mut self, path: Path, start: Span) -> Option<Spanned<Expr>> {
+        let _ = self.expect(LBrace, "record literal")?
+        let mut fields: Vec<FieldInit> = Vec::new()
+        let mut base: Option<Spanned<Expr>> = None
+
+        while !self.check(RBrace) && !self.is_at_end() {
+            if self.eat(DotDot).is_some() {
+                base = Some(self.parse_expr("record spread")?)
+                let _ = self.eat(Comma)
+                break
+            }
+            let name = match self.parse_ident("field name") {
+                Some(n) => n
+                None => break
+            }
+            let value = if self.eat(Colon).is_some() {
+                Some(self.parse_expr("field value")?)
+            } else {
+                None
+            }
+            fields.push(FieldInit { name: name, value: value })
+            if self.eat(Comma).is_none() { break }
+        }
+
+        let _ = self.expect(RBrace, "record literal")?
+        let end = self.prev_span_or(start)
+        Some(Spanned::new(ExprRecord(path, fields, base), start.merge(end)))
+    }
+}
diff --git a/selfhost/src/parse_item.nova b/selfhost/src/parse_item.nova
new file mode 100644
index 0000000000000000000000000000000000000000..94a3d129f82e150e30a1848e3eef02dfc0e9095e
--- /dev/null
+++ b/selfhost/src/parse_item.nova
@@ -0,0 +1,771 @@
+// Items: a port of crates/nova-parser/src/grammar.rs's "File", "Functions"
+// and "Records, types, traits, impls, const, import, module, extern"
+// sections, function by function (parser.nova says how its idioms
+// translate).
+
+import ast
+import parse_expr
+import parse_stmt
+import parse_type
+import parser
+import token
+
+// ---------------------------------------------------------------------------
+// File
+// ---------------------------------------------------------------------------
+
+impl Parser {
+    pub fn parse_file(mut self) -> File {
+        let mut items: Vec<Spanned<Item>> = Vec::new()
+        while !self.is_at_end() {
+            match self.try_parse_item() {
+                Some(item) => items.push(item)
+                None => {
+                    // Progress, guaranteed here rather than assumed from the
+                    // two halves: neither `try_parse_item`'s fallthrough nor
+                    // `sync_to_item_boundary` consumes a stop token.
+                    let before = self.pos
+                    self.sync_to_item_boundary()
+                    if self.pos == before { let _ = self.advance() }
+                }
+            }
+        }
+        // Past the depth limit: one error, P0002, for all that came after.
+        let crossing = match self.too_deep {
+            Some(crossing) => Some(crossing)
+            None => self.crossing_stands()
+        }
+        match crossing {
+            Some((errors, span)) => {
+                while self.errors.len() > errors { let _ = self.errors.pop() }
+                self.errors.push(TooDeep(span))
+            }
+            None => ()
+        }
+        let newline_splits = sorted_by_start(self.splits)
+        self.splits = Vec::new()
+        File { items: items, newline_splits: newline_splits }
+    }
+
+    /// Parse zero or more leading `@name` / `@name(a, b)` attributes.
+    /// Placement is not checked here: that is the resolver's decision.
+    pub fn parse_attributes(mut self) -> Vec<Attribute> {
+        let mut attrs: Vec<Attribute> = Vec::new()
+        while self.check(At) {
+            let start = self.peek_span()
+            let _ = self.advance()
+            let name = match self.parse_ident("in attribute name") {
+                Some(n) => n
+                None => break
+            }
+            let mut args: Vec<Spanned<String>> = Vec::new()
+            if self.check(LParen) {
+                let _ = self.advance()
+                while !self.check(RParen) && !self.is_at_end() {
+                    let arg = match self.parse_ident("in attribute arguments") {
+                        Some(a) => a
+                        None => break
+                    }
+                    args.push(arg)
+                    if self.check(Comma) { let _ = self.advance() } else { break }
+                }
+                let _ = self.expect(RParen, "in attribute arguments")
+            }
+            let span = start
+            attrs.push(Attribute { name: name, args: args, span: span })
+        }
+        attrs
+    }
+
+    pub fn try_parse_item(mut self) -> Option<Spanned<Item>> {
+        let start = self.peek_span()
+        let (docs, attrs) = self.parse_item_prefix()
+        let vis = self.parse_visibility()
+
+        let item = match self.peek() {
+            Fn | Async => {
+                let mut func = self.parse_function(vis)?
+                func.attrs = attrs
+                func.docs = docs
+                ItemFunction(func)
+            }
+            Record => {
+                let _ = self.advance()
+                let mut rec = self.parse_record(vis)?
+                rec.attrs = attrs
+                rec.docs = docs
+                ItemRecord(rec)
+            }
+            Type => {
+                let _ = self.advance()
+                let mut td = self.parse_type_decl(vis)?
+                td.attrs = attrs
+                td.docs = docs
+                ItemType(td)
+            }
+            Trait => {
+                let _ = self.advance()
+                let mut tr = self.parse_trait_decl(vis)?
+                tr.attrs = attrs
+                tr.docs = docs
+                ItemTrait(tr)
+            }
+            Impl => {
+                let _ = self.advance()
+                let mut impl_ = self.parse_impl_block()?
+                impl_.attrs = attrs
+                impl_.docs = docs
+                ItemImpl(impl_)
+            }
+            Const => {
+                let _ = self.advance()
+                let mut c = self.parse_const_decl(vis)?
+                c.attrs = attrs
+                c.docs = docs
+                ItemConst(c)
+            }
+            Import => {
+                let _ = self.advance()
+                let mut imp = self.parse_import()?
+                imp.attrs = attrs
+                imp.docs = docs
+                ItemImport(imp)
+            }
+            Module => {
+                let _ = self.advance()
+                let mut m = self.parse_module()?
+                m.attrs = attrs
+                m.docs = docs
+                ItemModule(m)
+            }
+            Extern => {
+                let _ = self.advance()
+                let mut e = self.parse_extern_block()?
+                e.attrs = attrs
+                e.docs = docs
+                ItemExtern(e)
+            }
+            Ident(_) if self.component_at(self.pos) => {
+                let mut func = self.parse_component(vis)?
+                func.attrs = attrs
+                func.docs = docs
+                ItemFunction(func)
+            }
+            _ => {
+                let span = self.peek_span()
+                if !docs.is_empty() {
+                    // Docs with no item after them.
+                    self.misplaced_doc(docs.at(0).span)
+                    return None
+                }
+                self.errors.push(
+                    Expected(
+                        "item (fn, record, type, trait, impl, const, import, module, extern)",
+                        self.peek().description(),
+                        span,
+                    ),
+                )
+                return None
+            }
+        }
+
+        let end = self.prev_span_or(start)
+        let span = start.merge(end)
+        Some(Spanned::new(item, span))
+    }
+
+    pub fn parse_visibility(mut self) -> Visibility {
+        if self.eat(Pub).is_some() { VisibilityPub } else { VisibilityPrivate }
+    }
+}
+
+// `splits`, sorted by their starts, stably (`sort_by_key(|s| s.at.start)`):
+// they are recorded nearly in order, so an insertion sort.
+fn sorted_by_start(splits: Vec<NewlineSplit>) -> Vec<NewlineSplit> {
+    let mut out: Vec<NewlineSplit> = Vec::new()
+    for s in splits.iter() {
+        out.push(s)
+        let mut i = out.len() - 1
+        while i > 0 && out.at(i - 1).at.start > s.at.start {
+            out.set(i, out.at(i - 1))
+            i -= 1
+        }
+        out.set(i, s)
+    }
+    out
+}
+
+// ---------------------------------------------------------------------------
+// Functions
+// ---------------------------------------------------------------------------
+
+impl Parser {
+    pub fn parse_function(mut self, vis: Visibility) -> Option<Function> {
+        let is_async = self.eat(Async).is_some()
+        let _ = self.expect(Fn, "function declaration")?
+        let name = self.parse_ident("function name")?
+        let generics = self.parse_generics_opt()
+        let _ = self.expect(LParen, "function parameters")?
+        let params = self.parse_params()
+        let _ = self.expect(RParen, "function parameters")?
+        let return_ty = if self.eat(Arrow).is_some() {
+            Some(self.parse_type("return type")?)
+        } else {
+            None
+        }
+        let where_clause = self.parse_where_clause_opt()
+        let body = self.parse_block("function body")?
+
+        Some(
+            Function {
+                docs: Vec::new(),
+                attrs: Vec::new(),
+                vis: vis,
+                is_async: is_async,
+                is_component: false,
+                name: name,
+                generics: generics,
+                params: params,
+                return_ty: return_ty,
+                where_clause: where_clause,
+                body: body,
+            },
+        )
+    }
+
+    /// `component Name(params) { body }` (RFC 0001): a function returning
+    /// `std/ui`'s `View`, with no type parameters and no written return
+    /// type.
+    pub fn parse_component(mut self, vis: Visibility) -> Option<Function> {
+        let _ = self.advance()
+        let name = self.parse_ident("component name")?
+        let _ = self.expect(LParen, "component parameters")?
+        let params = self.parse_params()
+        let _ = self.expect(RParen, "component parameters")?
+        match self.eat(Arrow) {
+            Some(arrow) => {
+                self.errors.push(
+                    Custom("a component returns a `View`, which it does not write", arrow.span),
+                )
+                // The type is read and dropped, so that the body still parses.
+                let _ = self.parse_type("return type")?
+            }
+            None => ()
+        }
+        let body = self.parse_block("component body")?
+        Some(
+            Function {
+                docs: Vec::new(),
+                attrs: Vec::new(),
+                vis: vis,
+                is_async: false,
+                is_component: true,
+                name: name,
+                generics: Vec::new(),
+                params: params,
+                return_ty: None,
+                where_clause: Vec::new(),
+                body: body,
+            },
+        )
+    }
+
+    pub fn parse_function_sig(mut self) -> Option<FunctionSig> {
+        let is_async = self.eat(Async).is_some()
+        let _ = self.expect(Fn, "function signature")?
+        let name = self.parse_ident("function name")?
+        let generics = self.parse_generics_opt()
+        let _ = self.expect(LParen, "function parameters")?
+        let params = self.parse_params()
+        let _ = self.expect(RParen, "function parameters")?
+        let return_ty = if self.eat(Arrow).is_some() {
+            Some(self.parse_type("return type")?)
+        } else {
+            None
+        }
+        let where_clause = self.parse_where_clause_opt()
+        Some(
+            FunctionSig {
+                docs: Vec::new(),
+                is_async: is_async,
+                name: name,
+                generics: generics,
+                params: params,
+                return_ty: return_ty,
+                where_clause: where_clause,
+            },
+        )
+    }
+
+    pub fn parse_params(mut self) -> Vec<Param> {
+        let mut params: Vec<Param> = Vec::new()
+        while !self.check(RParen) && !self.is_at_end() {
+            if self.reject_docs() { continue }
+            match self.parse_param() {
+                Some(p) => params.push(p)
+                None => ()
+            }
+            if self.eat(Comma).is_none() { break }
+        }
+        params
+    }
+
+    pub fn parse_param(mut self) -> Option<Param> {
+        let is_mut = self.eat(Mut).is_some()
+        let name = self.parse_ident("parameter name")?
+
+        // `self` may appear without a type annotation in method signatures.
+        if name.value == "self" && !self.check(Colon) {
+            let span = name.span
+            return Some(Param { is_mut: is_mut, name: name, ty: Spanned::new(TypeInfer, span) })
+        }
+
+        let _ = self.expect(Colon, "parameter type")?
+        let ty = self.parse_type("parameter type")?
+        Some(Param { is_mut: is_mut, name: name, ty: ty })
+    }
+
+    pub fn parse_generics_opt(mut self) -> Vec<TypeParam> {
+        let mut params: Vec<TypeParam> = Vec::new()
+        if !self.check(Lt) { return params }
+        let _ = self.advance()
+        while !self.check(Gt) && !self.is_at_end() {
+            let name = match self.parse_ident("type parameter") {
+                Some(n) => n
+                None => break
+            }
+            let mut bounds: Vec<Spanned<Path>> = Vec::new()
+            if self.eat(Colon).is_some() { bounds = self.parse_trait_bounds() }
+            params.push(TypeParam { name: name, bounds: bounds })
+            if self.eat(Comma).is_none() { break }
+        }
+        let _ = self.eat(Gt)
+        params
+    }
+
+    pub fn parse_trait_bounds(mut self) -> Vec<Spanned<Path>> {
+        let mut bounds: Vec<Spanned<Path>> = Vec::new()
+        while true {
+            let span = self.peek_span()
+            match self.try_parse_path() {
+                Some(path) => bounds.push(Spanned::new(path, span))
+                None => break
+            }
+            if self.eat(Plus).is_none() { break }
+        }
+        bounds
+    }
+
+    pub fn parse_where_clause_opt(mut self) -> Vec<WhereBound> {
+        let mut bounds: Vec<WhereBound> = Vec::new()
+        if !self.check(Where) { return bounds }
+        let _ = self.advance()
+        while true {
+            let ty = match self.parse_type("where bound type") {
+                Some(t) => t
+                None => break
+            }
+            if self.expect(Colon, "where bound").is_none() { break }
+            let trait_bounds = self.parse_trait_bounds()
+            bounds.push(WhereBound { ty: ty, bounds: trait_bounds })
+            if self.eat(Comma).is_none() { break }
+            // A trailing comma: the clause ends at the body or the end of a
+            // signature.
+            match self.peek() {
+                LBrace | Semicolon | RBrace => break
+                _ => ()
+            }
+        }
+        bounds
+    }
+}
+
+// ---------------------------------------------------------------------------
+// Records, types, traits, impls, const, import, module, extern
+// ---------------------------------------------------------------------------
+
+impl Parser {
+    pub fn parse_record(mut self, vis: Visibility) -> Option<Record> {
+        let name = self.parse_ident("record name")?
+        let generics = self.parse_generics_opt()
+        let _ = self.expect(LBrace, "record body")?
+        let mut fields: Vec<RecordField> = Vec::new()
+        while !self.check(RBrace) && !self.is_at_end() {
+            let docs = self.take_docs()
+            if self.check(RBrace) || self.is_at_end() {
+                if !docs.is_empty() { self.misplaced_doc(docs.at(0).span) }
+                continue
+            }
+            let fvis = self.parse_visibility()
+            let fname = match self.parse_ident("field name") {
+                Some(n) => n
+                None => {
+                    self.sync_to_stmt_boundary()
+                    continue
+                }
+            }
+            if self.expect(Colon, "field type").is_none() {
+                self.sync_to_stmt_boundary()
+                continue
+            }
+            let fty = match self.parse_type("field type") {
+                Some(t) => t
+                None => {
+                    self.sync_to_stmt_boundary()
+                    continue
+                }
+            }
+            let _ = self.eat(Comma)
+            fields.push(RecordField { docs: docs, vis: fvis, name: fname, ty: fty })
+        }
+        let _ = self.expect(RBrace, "record body")?
+        Some(
+            Record {
+                docs: Vec::new(),
+                attrs: Vec::new(),
+                vis: vis,
+                name: name,
+                generics: generics,
+                fields: fields,
+            },
+        )
+    }
+
+    pub fn parse_type_decl(mut self, vis: Visibility) -> Option<TypeDecl> {
+        let name = self.parse_ident("type name")?
+        let generics = self.parse_generics_opt()
+        let _ = self.expect(Eq, "type definition")?
+
+        // Sum type: starts with `|`, after the first variant's docs if any.
+        // A variant's docs come before its `|`; docs with no `|` after them
+        // belong to whatever follows the type, so they are left for it.
+        let def = if self.next_after_docs_is(Pipe) {
+            let mut variants: Vec<Variant> = Vec::new()
+            while self.next_after_docs_is(Pipe) {
+                let docs = self.take_docs()
+                let _ = self.advance()
+                let vname = match self.parse_ident("variant name") {
+                    Some(n) => n
+                    None => break
+                }
+                let mut fields: Vec<Spanned<Type>> = Vec::new()
+                if self.eat(LParen).is_some() {
+                    while !self.check(RParen) && !self.is_at_end() {
+                        match self.parse_type("variant field type") {
+                            Some(t) => fields.push(t)
+                            None => ()
+                        }
+                        if self.eat(Comma).is_none() { break }
+                    }
+                    let _ = self.eat(RParen)
+                }
+                variants.push(Variant { docs: docs, name: vname, fields: fields })
+            }
+            TypeDefSum(variants)
+        } else {
+            let ty = self.parse_type("type alias")?
+            TypeDefAlias(ty)
+        }
+
+        let _ = self.eat(Semicolon)
+        Some(
+            TypeDecl {
+                docs: Vec::new(),
+                attrs: Vec::new(),
+                vis: vis,
+                name: name,
+                generics: generics,
+                def: def,
+            },
+        )
+    }
+
+    pub fn parse_trait_decl(mut self, vis: Visibility) -> Option<TraitDecl> {
+        let name = self.parse_ident("trait name")?
+        let generics = self.parse_generics_opt()
+        let mut supertraits: Vec<Spanned<Path>> = Vec::new()
+        if self.eat(Colon).is_some() { supertraits = self.parse_trait_bounds() }
+        let where_clause = self.parse_where_clause_opt()
+        let _ = self.expect(LBrace, "trait body")?
+        let mut items: Vec<TraitItem> = Vec::new()
+        while !self.check(RBrace) && !self.is_at_end() {
+            let docs = self.take_docs()
+            if self.check(RBrace) || self.is_at_end() {
+                if !docs.is_empty() { self.misplaced_doc(docs.at(0).span) }
+                continue
+            }
+            // An associated type declaration (`type Item` or `type Item:
+            // Display`) must be handled before the speculative
+            // `parse_function_sig()` below.
+            if self.eat(Type).is_some() {
+                match self.parse_ident("associated type name") {
+                    Some(name) => {
+                        let bounds = if self.eat(Colon).is_some() {
+                            self.parse_trait_bounds()
+                        } else {
+                            Vec::new()
+                        }
+                        let _ = self.eat(Semicolon)
+                        items.push(TraitItemAssocType(docs, name, bounds))
+                    }
+                    None => self.sync_to_stmt_boundary()
+                }
+                continue
+            }
+            // Look ahead: if there's a body `{`, it's a provided method
+            let saved_pos = self.pos
+            let saved_errors_len = self.errors.len()
+            match self.parse_function_sig() {
+                Some(sig) => {
+                    let mut sig = sig
+                    if self.check(LBrace) {
+                        // Provided method: roll back and parse as a full
+                        // function.
+                        self.pos = saved_pos
+                        while self.errors.len() > saved_errors_len { let _ = self.errors.pop() }
+                        let func_vis = VisibilityPrivate
+                        match self.parse_function(func_vis) {
+                            Some(func) => {
+                                let mut func = func
+                                func.docs = docs
+                                items.push(TraitItemProvided(func))
+                            }
+                            None => ()
+                        }
+                    } else {
+                        // Required method: expect semicolon.
+                        let _ = self.eat(Semicolon)
+                        sig.docs = docs
+                        items.push(TraitItemRequired(sig))
+                    }
+                }
+                None => self.sync_to_stmt_boundary()
+            }
+        }
+        let _ = self.expect(RBrace, "trait body")?
+        Some(
+            TraitDecl {
+                docs: Vec::new(),
+                attrs: Vec::new(),
+                vis: vis,
+                name: name,
+                generics: generics,
+                supertraits: supertraits,
+                where_clause: where_clause,
+                items: items,
+            },
+        )
+    }
+
+    pub fn parse_impl_block(mut self) -> Option<ImplBlock> {
+        let generics = self.parse_generics_opt()
+
+        // Could be `impl Type` or `impl Trait for Type`: parse a type; if
+        // followed by `for`, treat it as the trait path.
+        let first_ty = self.parse_type("impl target")?
+        let mut trait_: Option<Spanned<Path>> = None
+        let mut ty = first_ty
+        if self.eat(For).is_some() {
+            let for_ty = self.parse_type("impl target type")?
+            let trait_path = type_to_path(first_ty)?
+            let trait_span = first_ty.span
+            trait_ = Some(Spanned::new(trait_path, trait_span))
+            ty = for_ty
+        }
+
+        let where_clause = self.parse_where_clause_opt()
+        let _ = self.expect(LBrace, "impl block")?
+        let mut functions: Vec<Function> = Vec::new()
+        let mut consts: Vec<ConstDecl> = Vec::new()
+        let mut assoc_types: Vec<AssocTypeBinding> = Vec::new()
+        // Loop invariant: every arm below advances `self.pos` by at least
+        // one token, so this terminates.
+        while !self.check(RBrace) && !self.is_at_end() {
+            let docs = self.take_docs()
+            if self.check(RBrace) || self.is_at_end() {
+                if !docs.is_empty() { self.misplaced_doc(docs.at(0).span) }
+                continue
+            }
+            // Captured before `parse_visibility` consumes it, so the `pub`
+            // rejection in the `type` arm can point at the `pub` itself.
+            let vis_span = self.peek_span()
+            let vis = self.parse_visibility()
+            match self.peek() {
+                Fn | Async => {
+                    match self.parse_function(vis) {
+                        Some(f) => {
+                            let mut f = f
+                            f.docs = docs
+                            functions.push(f)
+                        }
+                        None => self.sync_to_item_boundary()
+                    }
+                }
+                // An associated-type binding: `type Item = Int`.
+                Type => {
+                    let _ = self.advance()
+                    match vis {
+                        VisibilityPrivate => ()
+                        _ => self.errors.push(
+                            Custom(
+                                "an associated type binding in an impl cannot be `pub`",
+                                vis_span,
+                            ),
+                        )
+                    }
+                    match self.parse_ident("associated type name") {
+                        Some(name) => {
+                            if self.expect(Eq, "associated type binding").is_some() {
+                                match self.parse_type("associated type binding") {
+                                    Some(ty) => {
+                                        let _ = self.eat(Semicolon)
+                                        assoc_types.push(
+                                            AssocTypeBinding { docs: docs, name: name, ty: ty },
+                                        )
+                                    }
+                                    None => self.sync_to_stmt_boundary()
+                                }
+                            } else {
+                                self.sync_to_stmt_boundary()
+                            }
+                        }
+                        None => self.sync_to_stmt_boundary()
+                    }
+                }
+                Const => {
+                    let _ = self.advance()
+                    match self.parse_const_decl(vis) {
+                        Some(c) => {
+                            let mut c = c
+                            c.docs = docs
+                            consts.push(c)
+                        }
+                        None => self.sync_to_stmt_boundary()
+                    }
+                }
+                _ => {
+                    let span = self.peek_span()
+                    self.errors.push(
+                        Expected("fn or const inside impl", self.peek().description(), span),
+                    )
+                    // Consume the offending token before syncing: the arm's
+                    // half of the loop invariant above.
+                    let _ = self.advance()
+                    self.sync_to_item_boundary()
+                }
+            }
+        }
+        let _ = self.expect(RBrace, "impl block")?
+        Some(
+            ImplBlock {
+                docs: Vec::new(),
+                attrs: Vec::new(),
+                generics: generics,
+                trait_: trait_,
+                ty: ty,
+                where_clause: where_clause,
+                functions: functions,
+                consts: consts,
+                assoc_types: assoc_types,
+            },
+        )
+    }
+
+    pub fn parse_const_decl(mut self, vis: Visibility) -> Option<ConstDecl> {
+        let name = self.parse_ident("const name")?
+        let _ = self.expect(Colon, "const type")?
+        let ty = self.parse_type("const type")?
+        let _ = self.expect(Eq, "const value")?
+        let value = self.parse_expr("const value")?
+        let _ = self.eat(Semicolon)
+        Some(
+            ConstDecl {
+                docs: Vec::new(),
+                attrs: Vec::new(),
+                vis: vis,
+                name: name,
+                ty: ty,
+                value: value,
+            },
+        )
+    }
+
+    pub fn parse_import(mut self) -> Option<Import> {
+        let start = self.peek_span()
+        let path = self.parse_path("import path")?
+        let path_span = start.merge(self.prev_span_or(start))
+
+        let kind = if self.eat(As).is_some() {
+            let alias = self.parse_ident("import alias")?
+            ImportKindAlias(alias)
+        } else if self.eat(LBrace).is_some() {
+            let mut names: Vec<Spanned<String>> = Vec::new()
+            while !self.check(RBrace) && !self.is_at_end() {
+                match self.parse_ident("import name") {
+                    Some(n) => names.push(n)
+                    None => ()
+                }
+                if self.eat(Comma).is_none() { break }
+            }
+            let _ = self.expect(RBrace, "import list")?
+            ImportKindList(names)
+        } else {
+            ImportKindSimple
+        }
+
+        let _ = self.eat(Semicolon)
+        Some(
+            Import {
+                docs: Vec::new(),
+                attrs: Vec::new(),
+                path: Spanned::new(path, path_span),
+                kind: kind,
+            },
+        )
+    }
+
+    pub fn parse_module(mut self) -> Option<Module> {
+        let start = self.peek_span()
+        let path = self.parse_path("module path")?
+        let span = start.merge(self.prev_span_or(start))
+        let _ = self.eat(Semicolon)
+        Some(Module { docs: Vec::new(), attrs: Vec::new(), path: Spanned::new(path, span) })
+    }
+
+    pub fn parse_extern_block(mut self) -> Option<ExternBlock> {
+        let mut abi: Option<String> = None
+        if self.check(StrStart) {
+            // consume StrStart, StrPart (the ABI string), StrEnd
+            let _ = self.advance()
+            match self.peek() {
+                StrPart(s) => {
+                    let _ = self.advance()
+                    abi = Some(s)
+                }
+                _ => ()
+            }
+            let _ = self.eat(StrEnd)
+        }
+
+        let _ = self.expect(LBrace, "extern block")?
+        let mut items: Vec<ExternItem> = Vec::new()
+        while !self.check(RBrace) && !self.is_at_end() {
+            let docs = self.take_docs()
+            if self.check(RBrace) || self.is_at_end() {
+                if !docs.is_empty() { self.misplaced_doc(docs.at(0).span) }
+                continue
+            }
+            match self.parse_function_sig() {
+                Some(sig) => {
+                    let mut sig = sig
+                    sig.docs = docs
+                    let _ = self.eat(Semicolon)
+                    items.push(ExternItemFn(sig))
+                }
+                None => self.sync_to_stmt_boundary()
+            }
+        }
+        let _ = self.expect(RBrace, "extern block")?
+        Some(ExternBlock { docs: Vec::new(), attrs: Vec::new(), abi: abi, items: items })
+    }
+}
diff --git a/selfhost/src/parse_pattern.nova b/selfhost/src/parse_pattern.nova
new file mode 100644
index 0000000000000000000000000000000000000000..43e2e547e4c5f99eb1943efaf262424945dbf9fb
--- /dev/null
+++ b/selfhost/src/parse_pattern.nova
@@ -0,0 +1,286 @@
+// Patterns: a port of crates/nova-parser/src/grammar.rs's "Patterns"
+// section, function by function (parser.nova says how its idioms
+// translate).
+
+import ast
+import parser
+import token
+
+impl Parser {
+    pub fn parse_pattern(mut self, ctx: String) -> Option<Spanned<Pattern>> {
+        // `nested`
+        let depth = self.depth
+        let result = if self.deeper() { self.parse_pattern_here(ctx) } else { None }
+        self.depth = depth
+        result
+    }
+
+    /// `parse_pattern` at the cursor's level: an atom, or alternatives.
+    pub fn parse_pattern_here(mut self, ctx: String) -> Option<Spanned<Pattern>> {
+        let start = self.peek_span()
+        let pat = self.parse_pattern_atom(ctx)?
+
+        // Or patterns
+        if self.check(Pipe) {
+            let mut alternatives: Vec<Spanned<Pattern>> = Vec::new()
+            alternatives.push(pat)
+            while self.eat(Pipe).is_some() {
+                match self.parse_pattern_atom(ctx) {
+                    Some(alt) => alternatives.push(alt)
+                    None => break
+                }
+            }
+            let end = alternatives.last().span
+            return Some(Spanned::new(PatternOr(alternatives), start.merge(end)))
+        }
+
+        Some(pat)
+    }
+
+    pub fn parse_pattern_atom(mut self, ctx: String) -> Option<Spanned<Pattern>> {
+        let start = self.peek_span()
+        match self.peek() {
+            Ident(s) if s == "_" => {
+                let _ = self.advance()
+                Some(Spanned::new(PatternWildcard, start))
+            }
+            Int(n) => {
+                let _ = self.advance()
+                // Range pattern?
+                if self.eat(DotDot).is_some() {
+                    let inclusive = self.eat(Eq).is_some()
+                    let hi_start = self.peek_span()
+                    match self.peek() {
+                        Int(hi) => {
+                            let _ = self.advance()
+                            let end = self.prev_span_or(hi_start)
+                            return Some(
+                                Spanned::new(
+                                    PatternRange(
+                                        Spanned::new(PatternLit(LiteralInt(n)), start),
+                                        Spanned::new(PatternLit(LiteralInt(hi)), end),
+                                        inclusive,
+                                    ),
+                                    start.merge(end),
+                                ),
+                            )
+                        }
+                        _ => ()
+                    }
+                } else if self.eat(DotDotEq).is_some() {
+                    let hi_start = self.peek_span()
+                    match self.peek() {
+                        Int(hi) => {
+                            let _ = self.advance()
+                            let end = self.prev_span_or(hi_start)
+                            return Some(
+                                Spanned::new(
+                                    PatternRange(
+                                        Spanned::new(PatternLit(LiteralInt(n)), start),
+                                        Spanned::new(PatternLit(LiteralInt(hi)), end),
+                                        true,
+                                    ),
+                                    start.merge(end),
+                                ),
+                            )
+                        }
+                        _ => ()
+                    }
+                }
+                Some(Spanned::new(PatternLit(LiteralInt(n)), start))
+            }
+            Float(f) => {
+                let _ = self.advance()
+                Some(Spanned::new(PatternLit(LiteralFloat(f)), start))
+            }
+            Char(c) => {
+                let _ = self.advance()
+                // Range: 'a'..='z'
+                if self.eat(DotDotEq).is_some() {
+                    let hi_start = self.peek_span()
+                    match self.peek() {
+                        Char(hi) => {
+                            let _ = self.advance()
+                            let end = self.prev_span_or(hi_start)
+                            return Some(
+                                Spanned::new(
+                                    PatternRange(
+                                        Spanned::new(PatternLit(LiteralChar(c)), start),
+                                        Spanned::new(PatternLit(LiteralChar(hi)), end),
+                                        true,
+                                    ),
+                                    start.merge(end),
+                                ),
+                            )
+                        }
+                        _ => ()
+                    }
+                }
+                Some(Spanned::new(PatternLit(LiteralChar(c)), start))
+            }
+            True => {
+                let _ = self.advance()
+                Some(Spanned::new(PatternLit(LiteralBool(true)), start))
+            }
+            False => {
+                let _ = self.advance()
+                Some(Spanned::new(PatternLit(LiteralBool(false)), start))
+            }
+            StrStart => {
+                // A string literal in a pattern
+                let _ = self.advance()
+                let mut s = StringBuilder::new()
+                while true {
+                    match self.peek() {
+                        StrPart(p) => {
+                            s.push(p)
+                            let _ = self.advance()
+                        }
+                        StrEnd => {
+                            let _ = self.advance()
+                            break
+                        }
+                        _ => break
+                    }
+                }
+                let end = self.prev_span_or(start)
+                Some(Spanned::new(PatternLit(LiteralStr(s.to_string())), start.merge(end)))
+            }
+            LParen => {
+                let _ = self.advance()
+                // Spans over the parentheses, as for expressions and types,
+                // so the formatter can keep them.
+                match self.eat(RParen) {
+                    Some(close) => {
+                        return Some(Spanned::new(PatternTuple(Vec::new()), start.merge(close.span)))
+                    }
+                    None => ()
+                }
+                let first = self.parse_pattern(ctx)?
+                match self.eat(RParen) {
+                    Some(close) => return Some(Spanned::new(first.value, start.merge(close.span)))
+                    None => ()
+                }
+                let mut elems: Vec<Spanned<Pattern>> = Vec::new()
+                elems.push(first)
+                while self.eat(Comma).is_some() && !self.check(RParen) {
+                    match self.parse_pattern(ctx) {
+                        Some(p) => elems.push(p)
+                        None => ()
+                    }
+                }
+                let _ = self.expect(RParen, "tuple pattern")?
+                let end = self.prev_span_or(start)
+                Some(Spanned::new(PatternTuple(elems), start.merge(end)))
+            }
+            LBracket => {
+                let _ = self.advance()
+                let mut elems: Vec<Spanned<Pattern>> = Vec::new()
+                while !self.check(RBracket) && !self.is_at_end() {
+                    match self.parse_pattern(ctx) {
+                        Some(p) => elems.push(p)
+                        None => ()
+                    }
+                    if self.eat(Comma).is_none() { break }
+                }
+                let _ = self.expect(RBracket, "array pattern")?
+                let end = self.prev_span_or(start)
+                Some(Spanned::new(PatternArray(elems), start.merge(end)))
+            }
+            Ident(_) | SelfLower => {
+                let name = self.parse_ident(ctx)?
+
+                // Path with `::`: could be an enum variant
+                if self.check(ColonColon) {
+                    let mut path = single_path(name)
+                    while self.eat(ColonColon).is_some() {
+                        match self.parse_ident(ctx) {
+                            Some(seg) => path.segments.push(seg)
+                            None => break
+                        }
+                    }
+                    // Tuple variant: `Variant(pat, ...)`
+                    if self.check(LParen) {
+                        let _ = self.advance()
+                        let mut fields: Vec<Spanned<Pattern>> = Vec::new()
+                        while !self.check(RParen) && !self.is_at_end() {
+                            match self.parse_pattern(ctx) {
+                                Some(p) => fields.push(p)
+                                None => ()
+                            }
+                            if self.eat(Comma).is_none() { break }
+                        }
+                        let _ = self.expect(RParen, "variant pattern")?
+                        let end = self.prev_span_or(start)
+                        return Some(
+                            Spanned::new(PatternTupleStruct(path, fields), start.merge(end)),
+                        )
+                    }
+                    let end = self.prev_span_or(start)
+                    return Some(Spanned::new(PatternPath(path), start.merge(end)))
+                }
+
+                // Binding with `@`
+                if self.eat(At).is_some() {
+                    let inner = self.parse_pattern(ctx)?
+                    let span = start.merge(inner.span)
+                    return Some(Spanned::new(PatternBinding(name, inner), span))
+                }
+
+                // Tuple struct: `Some(pat)`
+                if self.check(LParen) {
+                    let path = single_path(name)
+                    let _ = self.advance()
+                    let mut fields: Vec<Spanned<Pattern>> = Vec::new()
+                    while !self.check(RParen) && !self.is_at_end() {
+                        match self.parse_pattern(ctx) {
+                            Some(p) => fields.push(p)
+                            None => ()
+                        }
+                        if self.eat(Comma).is_none() { break }
+                    }
+                    let _ = self.expect(RParen, "tuple struct pattern")?
+                    let end = self.prev_span_or(start)
+                    return Some(Spanned::new(PatternTupleStruct(path, fields), start.merge(end)))
+                }
+
+                // Record pattern: `Point { x, y }`
+                if self.check(LBrace) {
+                    let path = single_path(name)
+                    let _ = self.advance()
+                    let mut field_pats: Vec<FieldPat> = Vec::new()
+                    let mut rest = false
+                    while !self.check(RBrace) && !self.is_at_end() {
+                        if self.eat(DotDot).is_some() {
+                            rest = true
+                            break
+                        }
+                        let fname = match self.parse_ident(ctx) {
+                            Some(n) => n
+                            None => break
+                        }
+                        let pattern = if self.eat(Colon).is_some() {
+                            Some(self.parse_pattern(ctx)?)
+                        } else {
+                            None
+                        }
+                        field_pats.push(FieldPat { name: fname, pattern: pattern })
+                        if self.eat(Comma).is_none() { break }
+                    }
+                    let _ = self.expect(RBrace, "record pattern")?
+                    let end = self.prev_span_or(start)
+                    return Some(
+                        Spanned::new(PatternRecord(path, field_pats, rest), start.merge(end)),
+                    )
+                }
+
+                Some(Spanned::new(PatternIdent(false, name), start))
+            }
+            _ => {
+                let span = self.peek_span()
+                self.errors.push(Expected("pattern (in ${ctx})", self.peek().description(), span))
+                None
+            }
+        }
+    }
+}
diff --git a/selfhost/src/parse_stmt.nova b/selfhost/src/parse_stmt.nova
new file mode 100644
index 0000000000000000000000000000000000000000..20845de15e49b882223be5186b315dc61a20d103
--- /dev/null
+++ b/selfhost/src/parse_stmt.nova
@@ -0,0 +1,102 @@
+// Blocks and statements: a port of crates/nova-parser/src/grammar.rs's
+// "Blocks and statements" section, function by function (parser.nova says
+// how its idioms translate).
+
+import ast
+import parse_expr
+import parse_item
+import parse_pattern
+import parse_type
+import parser
+import token
+
+impl Parser {
+    pub fn parse_block(mut self, ctx: String) -> Option<Spanned<Block>> {
+        // A block anywhere turns statement context on (`in_stmt_ctx(true)`).
+        let prev = self.enter_stmt_ctx(true)
+        let block = self.parse_block_inner(ctx)
+        self.stmt_ctx = prev
+        block
+    }
+
+    pub fn parse_block_inner(mut self, ctx: String) -> Option<Spanned<Block>> {
+        let start = self.peek_span()
+        let _ = self.expect(LBrace, ctx)?
+        let mut stmts: Vec<Spanned<Stmt>> = Vec::new()
+        let mut trailing: Option<Spanned<Expr>> = None
+
+        while !self.check(RBrace) && !self.is_at_end() {
+            // A `///` documents a nested item. Anywhere else in a block it is
+            // misplaced.
+            if is_doc(self.peek()) && !self.is_item_start_at(self.docs_end()) {
+                let _ = self.reject_docs()
+                continue
+            }
+            // Parse a statement; if it's an expression without semicolon at
+            // the block-final position, treat it as trailing.
+            match self.try_parse_stmt() {
+                Some(stmt) => {
+                    match stmt.value {
+                        StmtExpr(_) if !self.check(RBrace) => stmts.push(stmt)
+                        // Could be trailing, but we already consumed it:
+                        // with or without its semicolon, it is.
+                        StmtExpr(e) => trailing = Some(e)
+                        _ => stmts.push(stmt)
+                    }
+                }
+                None => self.sync_to_stmt_boundary()
+            }
+        }
+
+        let _ = self.expect(RBrace, ctx)?
+        let end = self.prev_span_or(start)
+        Some(Spanned::new(Block { stmts: stmts, trailing: trailing }, start.merge(end)))
+    }
+
+    pub fn try_parse_stmt(mut self) -> Option<Spanned<Stmt>> {
+        let start = self.peek_span()
+
+        // `let` statement
+        if self.check(Let) { return self.parse_let_stmt() }
+
+        // Nested items, documented or not: `nested`, `in_stmt_ctx(false)`.
+        if self.is_item_start_at(self.docs_end()) {
+            let depth = self.depth
+            let mut item: Option<Spanned<Item>> = None
+            if self.deeper() {
+                let prev = self.enter_stmt_ctx(false)
+                item = self.try_parse_item()
+                self.stmt_ctx = prev
+            }
+            self.depth = depth
+            let item = item?
+            let span = item.span
+            return Some(Spanned::new(StmtItem(item.value), span))
+        }
+
+        // Expression statement
+        let expr = self.parse_expr("statement")?
+        let has_semi = self.eat(Semicolon).is_some()
+        let end = self.prev_span_or(start)
+        let span = start.merge(end)
+
+        let _ = has_semi
+        Some(Spanned::new(StmtExpr(expr), span))
+    }
+
+    pub fn parse_let_stmt(mut self) -> Option<Spanned<Stmt>> {
+        let start = self.peek_span()
+        let _ = self.expect(Let, "let statement")?
+        let is_mut = self.eat(Mut).is_some()
+        let pattern = self.parse_pattern("let pattern")?
+        let ty = if self.eat(Colon).is_some() { Some(self.parse_type("let type")?) } else { None }
+        let init = if self.eat(Eq).is_some() {
+            Some(self.parse_expr("let initializer")?)
+        } else {
+            None
+        }
+        let _ = self.eat(Semicolon)
+        let end = self.prev_span_or(start)
+        Some(Spanned::new(StmtLet(is_mut, pattern, ty, init), start.merge(end)))
+    }
+}
diff --git a/selfhost/src/parse_type.nova b/selfhost/src/parse_type.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ca0cff62d4a5aa632e938532005e5318a30b5694
--- /dev/null
+++ b/selfhost/src/parse_type.nova
@@ -0,0 +1,204 @@
+// Types: a port of crates/nova-parser/src/grammar.rs's "Types" section,
+// function by function (parser.nova says how its idioms translate).
+
+import ast
+import parser
+import token
+
+impl Parser {
+    pub fn parse_type(mut self, ctx: String) -> Option<Spanned<Type>> {
+        // `nested`
+        let depth = self.depth
+        let result = if self.deeper() { self.parse_type_here(ctx) } else { None }
+        self.depth = depth
+        result
+    }
+
+    /// A type at the cursor's level, which is not one more: the type after
+    /// `as`, which its operator's level counts, as a binary operator's
+    /// counts its right operand (`deeper`).
+    pub fn parse_type_here(mut self, ctx: String) -> Option<Spanned<Type>> {
+        let start = self.peek_span()
+        let mut ty = self.parse_type_atom(ctx)?
+
+        // Optional sugar: `T?`
+        while self.eat(Question).is_some() {
+            let end = self.prev_span_or(start)
+            ty = Spanned::new(TypeOptional(ty), start.merge(end))
+        }
+
+        Some(ty)
+    }
+
+    pub fn parse_type_atom(mut self, ctx: String) -> Option<Spanned<Type>> {
+        let start = self.peek_span()
+        match self.peek() {
+            Amp => {
+                let _ = self.advance()
+                let is_mut = self.eat(Mut).is_some()
+                let inner = self.parse_type(ctx)?
+                let span = start.merge(inner.span)
+                Some(Spanned::new(TypeRef(is_mut, inner), span))
+            }
+            Star => {
+                let _ = self.advance()
+                let is_mut = self.eat(Mut).is_some()
+                let inner = self.parse_type(ctx)?
+                let span = start.merge(inner.span)
+                Some(Spanned::new(TypePtr(is_mut, inner), span))
+            }
+            LBracket => {
+                let _ = self.advance()
+                let inner = self.parse_type(ctx)?
+                let _ = self.expect(RBracket, "array type")?
+                let end = self.prev_span_or(start)
+                Some(Spanned::new(TypeArray(inner), start.merge(end)))
+            }
+            LParen => {
+                let _ = self.advance()
+                if self.eat(RParen).is_some() {
+                    // unit type `()`
+                    let end = self.prev_span_or(start)
+                    return Some(Spanned::new(TypeTuple(Vec::new()), start.merge(end)))
+                }
+                let first = self.parse_type(ctx)?
+                if self.eat(RParen).is_some() {
+                    // Single-element paren: not a tuple, just the type
+                    let end = self.prev_span_or(start)
+                    return Some(Spanned::new(first.value, start.merge(end)))
+                }
+                // Tuple
+                let mut elems: Vec<Spanned<Type>> = Vec::new()
+                elems.push(first)
+                while self.eat(Comma).is_some() && !self.check(RParen) {
+                    match self.parse_type(ctx) {
+                        Some(t) => elems.push(t)
+                        None => ()
+                    }
+                }
+                let _ = self.expect(RParen, "tuple type")?
+                let end = self.prev_span_or(start)
+                Some(Spanned::new(TypeTuple(elems), start.merge(end)))
+            }
+            Fn => {
+                let _ = self.advance()
+                let _ = self.expect(LParen, "fn type params")?
+                let mut params: Vec<Spanned<Type>> = Vec::new()
+                while !self.check(RParen) && !self.is_at_end() {
+                    match self.parse_type(ctx) {
+                        Some(t) => params.push(t)
+                        None => ()
+                    }
+                    if self.eat(Comma).is_none() { break }
+                }
+                let _ = self.expect(RParen, "fn type params")?
+                let ret = if self.eat(Arrow).is_some() {
+                    self.parse_type(ctx)?
+                } else {
+                    let unit_span = self.peek_span()
+                    Spanned::new(TypeTuple(Vec::new()), unit_span)
+                }
+                let end = ret.span
+                Some(Spanned::new(TypeFn(params, ret), start.merge(end)))
+            }
+            Ident(_) | SelfUpper => {
+                let path = self.parse_path(ctx)?
+                // Generic args `<T, U>`
+                let args = self.parse_generic_args_opt(ctx)
+                let end = self.prev_span_or(start)
+                Some(Spanned::new(TypePath(path, args), start.merge(end)))
+            }
+            _ => {
+                let span = self.peek_span()
+                self.errors.push(Expected("type (in ${ctx})", self.peek().description(), span))
+                None
+            }
+        }
+    }
+
+    /// Whether the cursor is at a closing `>` for a generic argument list:
+    /// a `>` pending from an earlier split `>>`, or a `>` / `>>` token.
+    pub fn at_generic_close(self) -> Bool {
+        if self.pending_gt > 0 { return true }
+        match self.peek() {
+            Gt | GtGt => true
+            _ => false
+        }
+    }
+
+    /// Consume one closing `>`. A glued `>>` token yields one `>` now and
+    /// records the other in `pending_gt` for the enclosing list, so
+    /// `Option<Option<Int>>` closes correctly. Returns false if not at a
+    /// `>`.
+    pub fn eat_generic_close(mut self) -> Bool {
+        if self.pending_gt > 0 {
+            self.pending_gt -= 1
+            return true
+        }
+        match self.peek() {
+            Gt => {
+                let _ = self.advance()
+                true
+            }
+            GtGt => {
+                let _ = self.advance()
+                self.pending_gt += 1
+                true
+            }
+            _ => false
+        }
+    }
+
+    /// Undo a speculative parse that began at `pos`, with `pending_gt`
+    /// pending `>`s and the P0002 state `too_deep`. The errors it pushed
+    /// stand, as the parser before the depth limit left them, but for
+    /// those after it crossed the limit, which it read at the end of the
+    /// file. Its crossing does not decide alone: it waits in
+    /// `rolled_back`, at the token at the cursor, where it was made, for
+    /// the tokens to be read again the other way.
+    pub fn roll_back(mut self, pos: Int, pending_gt: Int, too_deep: Option<(Int, Span)>) {
+        match (too_deep, self.too_deep) {
+            (None, Some((errors, span))) => {
+                while self.errors.len() > errors { let _ = self.errors.pop() }
+                self.rolled_back = Some((errors, span, self.pos))
+            }
+            _ => ()
+        }
+        self.pos = pos
+        self.pending_gt = pending_gt
+        self.too_deep = too_deep
+    }
+
+    pub fn parse_generic_args_opt(mut self, ctx: String) -> Vec<Spanned<Type>> {
+        let mut args: Vec<Spanned<Type>> = Vec::new()
+        if !self.check(Lt) { return args }
+        // Peek ahead: if next after `<` looks like a type, treat as generic
+        // args. Otherwise this `<` might be a comparison operator. A rollback
+        // restores the position, and keeps the errors pushed meanwhile
+        // (`roll_back`).
+        let saved = self.pos
+        let saved_pending = self.pending_gt
+        let saved_deep = self.too_deep
+        let _ = self.advance()
+        while !self.at_generic_close() && !self.is_at_end() {
+            match self.parse_type(ctx) {
+                Some(t) => args.push(t)
+                None => {
+                    // Wasn't a type: roll back
+                    self.roll_back(saved, saved_pending, saved_deep)
+                    return Vec::new()
+                }
+            }
+            // A `>` split from a `>>` now closes this list; a following
+            // comma (if any) belongs to an enclosing list, so stop here.
+            if self.pending_gt > 0 { break }
+            if self.eat(Comma).is_none() { break }
+        }
+        if !self.eat_generic_close() {
+            // Didn't close: roll back
+            self.roll_back(saved, saved_pending, saved_deep)
+            return Vec::new()
+        }
+        args
+    }
+}
diff --git a/selfhost/src/parse_view.nova b/selfhost/src/parse_view.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fbef663371007c25229d142eb5d259e2535baefb
--- /dev/null
+++ b/selfhost/src/parse_view.nova
@@ -0,0 +1,166 @@
+// Views (RFC 0001's `view { … }`): a port of the view functions of
+// crates/nova-parser/src/grammar.rs's expressions section, function by
+// function (parser.nova says how its idioms translate).
+
+import ast
+import lexer
+import parse_expr
+import parser
+import token
+
+impl Parser {
+    /// `view { child* }` (RFC 0001), at `view`.
+    pub fn parse_view(mut self) -> Option<Spanned<Expr>> {
+        let start = self.peek_span()
+        // A view's children keep their `;` rule: the newline rule is off in
+        // them, as in an element's arguments: `in_stmt_ctx(false)`.
+        let _ = self.advance()
+        let prev = self.enter_stmt_ctx(false)
+        let children = self.parse_view_children()
+        self.stmt_ctx = prev
+        let children = children?
+        let span = start.merge(self.prev_span_or(start))
+        Some(Spanned::new(ExprView(children), span))
+    }
+
+    /// `{ child* }`, the children of a `view` or an element, one level
+    /// deeper (`nested`). A `{` inside is an element's or a block's again,
+    /// whatever context the view is in.
+    pub fn parse_view_children(mut self) -> Option<Vec<Spanned<ViewNode>>> {
+        let depth = self.depth
+        let result = if self.deeper() { self.parse_view_children_here() } else { None }
+        self.depth = depth
+        result
+    }
+
+    /// `parse_view_children` at the cursor's level.
+    pub fn parse_view_children_here(mut self) -> Option<Vec<Spanned<ViewNode>>> {
+        let _ = self.expect(LBrace, "view children")?
+        let saved = self.enter_no_struct_ctx(false)
+        let mut children: Vec<Spanned<ViewNode>> = Vec::new()
+        while !self.check(RBrace) && !self.is_at_end() {
+            // A `;` may end a child, as one ends a statement, so that the
+            // next can start with `(`, `[` or `{`.
+            if self.reject_docs() || self.eat(Semicolon).is_some() { continue }
+            match self.parse_view_child() {
+                Some(child) => children.push(child)
+                None => {
+                    self.no_struct_literal = saved
+                    return None
+                }
+            }
+        }
+        self.no_struct_literal = saved
+        let _ = self.expect(RBrace, "view children")?
+        Some(children)
+    }
+
+    /// One child of a view: an element, or a postfix expression.
+    pub fn parse_view_child(mut self) -> Option<Spanned<ViewNode>> {
+        let start = self.peek_span()
+        match self.peek() {
+            Ident(name) => {
+                let next = self.value_at(self.pos + 1)
+                let brace_next = next.kind() == LBrace.kind()
+                if name != "view" && brace_next {
+                    let name = Spanned::new(name, start)
+                    let _ = self.advance()
+                    let children = self.parse_view_children()?
+                    let span = start.merge(self.prev_span_or(start))
+                    return Some(
+                        Spanned::new(ViewNodeElement(name, Vec::new(), Some(children)), span),
+                    )
+                }
+                if next.kind() == LParen.kind() {
+                    let name = Spanned::new(name, start)
+                    // the name, then `(`
+                    let _ = self.advance()
+                    let _ = self.advance()
+                    let args = self.parse_view_args()?
+                    let mut named = false
+                    for a in args.iter() {
+                        match a {
+                            ViewArgNamed(_, _) => named = true
+                            _ => ()
+                        }
+                    }
+                    if self.check(LBrace) {
+                        let children = self.parse_view_children()?
+                        let span = start.merge(self.prev_span_or(start))
+                        return Some(Spanned::new(ViewNodeElement(name, args, Some(children)), span))
+                    }
+                    let span = start.merge(self.prev_span_or(start))
+                    if named { return Some(Spanned::new(ViewNodeElement(name, args, None), span)) }
+                    // Positional arguments alone and no block: an ordinary
+                    // call.
+                    let mut call_args: Vec<Spanned<Expr>> = Vec::new()
+                    for a in args.iter() {
+                        match a {
+                            ViewArgPositional(e) => call_args.push(e)
+                            ViewArgNamed(_, value) => call_args.push(value)
+                        }
+                    }
+                    let callee = Spanned::new(ExprPath(single_path(name)), name.span)
+                    let call = Spanned::new(ExprCall(callee, call_args), span)
+                    let expr = self.parse_postfix_from(start, call, "view child")?
+                    self.end_halt()
+                    let span = expr.span
+                    return Some(Spanned::new(ViewNodeExpr(expr), span))
+                }
+            }
+            _ => ()
+        }
+        // A child is an expression of its own, as `parse_expr` reads one: a
+        // bare `return` or `break` in it ends with it.
+        let expr = self.parse_postfix("view child")
+        self.end_halt()
+        let expr = expr?
+        let span = expr.span
+        Some(Spanned::new(ViewNodeExpr(expr), span))
+    }
+
+    /// An element's arguments, after its `(`, up to and with its `)`:
+    /// `name: value` or an expression, separated by commas.
+    pub fn parse_view_args(mut self) -> Option<Vec<ViewArg>> {
+        let mut args: Vec<ViewArg> = Vec::new()
+        while !self.check(RParen) && !self.is_at_end() {
+            let colon_next = self.value_at(self.pos + 1).kind() == Colon.kind()
+            match (self.arg_name(), colon_next) {
+                (Some(word), true) => {
+                    let name = Spanned::new(word, self.peek_span())
+                    // the name, then `:`
+                    let _ = self.advance()
+                    let _ = self.advance()
+                    let value = self.parse_expr("element argument")?
+                    args.push(ViewArgNamed(name, value))
+                }
+                _ => args.push(ViewArgPositional(self.parse_expr("element argument")?))
+            }
+            if self.eat(Comma).is_none() { break }
+        }
+        let _ = self.expect(RParen, "element arguments")?
+        Some(args)
+    }
+
+    /// The word at the cursor, if it can name an element's argument: an
+    /// identifier, or a keyword such as `type` or `for`.
+    pub fn arg_name(self) -> Option<String> {
+        match self.peek() {
+            Ident(w) => Some(w)
+            other => {
+                let word = trim_backticks(other.description())
+                for k in KEYWORDS.iter() { if k == word { return Some(word) } }
+                None
+            }
+        }
+    }
+}
+
+// `text` without the backticks at either end (`trim_matches('`')`).
+fn trim_backticks(text: String) -> String {
+    let mut start = 0
+    let mut end = text.byte_len()
+    while start < end && text.byte_at(start) == 96 { start += 1 }
+    while end > start && text.byte_at(end - 1) == 96 { end -= 1 }
+    text.byte_slice(start, end)
+}
diff --git a/selfhost/src/parser.nova b/selfhost/src/parser.nova
new file mode 100644
index 0000000000000000000000000000000000000000..af140d0e65a9433fed57c460cc1a063734b46ec0
--- /dev/null
+++ b/selfhost/src/parser.nova
@@ -0,0 +1,676 @@
+// The parser's state, its token access, recovery and errors: a port of
+// crates/nova-parser/src/grammar.rs's "Parser state" and "Paths and
+// identifiers" sections and its helpers, of error.rs and of lib.rs (spec
+// docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md
+// §4.2, ADR 0052). parse_item.nova, parse_type.nova, parse_stmt.nova,
+// parse_expr.nova, parse_view.nova and parse_pattern.nova hold the rest of
+// grammar.rs, area by area; each Rust function has a Nova one of the same
+// name and shape. A change to the grammar changes both parsers in one pull
+// request; the differential test in crates/nova-cli/tests/selfhost.rs holds
+// their trees and errors equal.
+//
+// The Rust idioms, each translated one way (ADR 0052):
+// - `std::mem::discriminant` comparisons compare `Token.kind()`;
+// - a match on `self.peek().clone()` is a match on the token `peek` returns;
+// - the `in_*_ctx(|p| …)` closures are an explicit save of the flag before
+//   the call and its restore after; `nested(|p| …)` is the same with the
+//   depth, and a closure with more than a call in it is a function named
+//   `…_here`, as S0's are;
+// - `static HALT` is the field `eof`, an `Eof` token;
+// - the `&'a Tokens` is the field `tokens`;
+// - `self.tokens.get(self.pos.saturating_sub(1)).map(|s| s.span)
+//   .unwrap_or(start)` is `self.prev_span_or(start)`.
+// The names of dropped items are not kept: only the language server reads
+// them, and it stays Rust (ADR 0048).
+
+import ast
+import lexer
+import parse_expr
+import parse_item
+import parse_pattern
+import parse_stmt
+import parse_type
+import parse_view
+import token
+
+/// How deeply source may nest: past it, the parser reports `TooDeep`
+/// (P0002) and stops (lib.rs's `MAX_DEPTH`).
+pub const MAX_DEPTH: Int = 128
+
+// The error for a `///` that documents nothing.
+const MISPLACED_DOC: String = "a doc comment must come right before an item, a field or a variant; use `//` for a plain comment"
+
+// ---------------------------------------------------------------------------
+// error.rs
+// ---------------------------------------------------------------------------
+
+/// A parse error with source location (error.rs's `ParseError`).
+pub type ParseError =
+    // expected, found, span
+    | Expected(String, String, Span)
+    | UnexpectedEof(Span)
+    | ChainedComparison(Span)
+    // message, span
+    | Custom(String, Span)
+    /// More than `MAX_DEPTH` levels of nesting, at the token where the
+    /// limit was crossed: P0002. The parser stops there, so it is the
+    /// file's last error.
+    | TooDeep(Span)
+    /// `error`, with a note that says why it may have come about.
+    // error, note
+    | Noted(ParseError, String)
+
+impl ParseError {
+    pub fn span(self) -> Span {
+        match self {
+            Expected(_, _, span) => span
+            UnexpectedEof(span) => span
+            ChainedComparison(span) => span
+            Custom(_, span) => span
+            TooDeep(span) => span
+            Noted(error, _) => error.span()
+        }
+    }
+
+    /// The code a caller gives the error's diagnostic: P0002 for `TooDeep`,
+    /// P0001 for every other.
+    pub fn code(self) -> String {
+        match self {
+            TooDeep(_) => "P0002"
+            Noted(error, _) => error.code()
+            _ => "P0001"
+        }
+    }
+
+    /// The notes that go with the error's diagnostic.
+    pub fn notes(self) -> Vec<String> {
+        match self {
+            Noted(error, note) => {
+                let mut notes = error.notes()
+                notes.push(note)
+                notes
+            }
+            _ => Vec::new()
+        }
+    }
+}
+
+// The `#[error(…)]` texts, word for word.
+impl Display for ParseError {
+    fn fmt(self) -> String {
+        match self {
+            Expected(expected, found, _) => "expected ${expected}, found ${found}"
+            UnexpectedEof(_) => "unexpected end of file"
+            ChainedComparison(_) => "chained comparison operators are not allowed (use parentheses)"
+            Custom(message, _) => message
+            TooDeep(_) => "this is nested too deeply (more than ${MAX_DEPTH} levels)"
+            Noted(error, _) => "${error}"
+        }
+    }
+}
+
+// ---------------------------------------------------------------------------
+// Parser state
+// ---------------------------------------------------------------------------
+
+/// The parser's state (grammar.rs's `Parser`).
+pub record Parser {
+    tokens: Tokens,
+    pos: Int,
+    errors: Vec<ParseError>,
+    // When `true`, `Ident {` in expression position is NOT parsed as a
+    // record literal. Set by scrutinee positions (if/while/for/match
+    // conditions) to avoid ambiguity with the following `{ block }`.
+    no_struct_literal: Bool,
+    // Whether the parser is in statement context, where the newline rule
+    // holds: a line break ends the expression before a `(`, a `[` or a
+    // binary `-`. On for a block's statements and tail and a match arm's
+    // body; off inside parentheses, brackets, arguments, record literals,
+    // interpolation holes, views, items, and the heads of `if`, `while`,
+    // `for` and `match`. Saved and restored like `no_struct_literal`.
+    stmt_ctx: Bool,
+    // Count of closing `>` "borrowed" from a `>>` token while closing
+    // nested generic argument lists (`Option<Option<Int>>`).
+    pending_gt: Int,
+    // Each place the newline rule ended a statement the parser before it
+    // would have continued, for the checker's E0103.
+    splits: Vec<NewlineSplit>,
+    // The position after a `return` or `break` that a line break ended, or
+    // -1: no operator, call or index there takes the bare keyword as its
+    // operand. Cleared when the expression that holds it ends.
+    halt: Int,
+    // The split that ended the last match arm's body: a P0001 in the head
+    // of the arm that begins there gets a note naming the rule.
+    arm_end_split: Option<NewlineSplit>,
+    // How many levels have been entered at the cursor, the outermost
+    // included: one more than the cursor's level (`deeper`).
+    depth: Int,
+    // Where the depth first passed `MAX_DEPTH`: how many errors there were
+    // then, and the span of the token at the cursor. From then on the
+    // parser reads the end of the file (`peek`), and when the file ends,
+    // P0002 at that span replaces every error pushed since.
+    too_deep: Option<(Int, Span)>,
+    // A crossing that an abandoned attempt made (`roll_back`): how many
+    // errors there were then, its span, and the token where it was made.
+    // The parser reads those tokens again another way, and the crossing
+    // stands, as `too_deep` (`crossing_stands`), if an error comes before
+    // that token is read; once it is read, the crossing goes.
+    rolled_back: Option<(Int, Span, Int)>,
+    // What `peek` reads where a line break ended a bare `return` or `break`
+    // (`halted`), or past the depth limit: nothing (grammar.rs's `HALT`).
+    eof: Token,
+}
+
+/// A parser at the start of `tokens` (`Parser::new`).
+pub fn new_parser(tokens: Tokens) -> Parser {
+    Parser {
+        tokens: tokens,
+        pos: 0,
+        errors: Vec::new(),
+        no_struct_literal: false,
+        stmt_ctx: false,
+        pending_gt: 0,
+        splits: Vec::new(),
+        halt: -1,
+        arm_end_split: None,
+        depth: 0,
+        too_deep: None,
+        rolled_back: None,
+        eof: Eof,
+    }
+}
+
+impl Parser {
+    /// One level deeper. Whether the parser may go on: past `MAX_DEPTH`
+    /// levels it may not, and the first time, it records P0002 at the token
+    /// at the cursor (`too_deep`). A level is:
+    ///
+    /// - an expression (`parse_expr`), a type (`parse_type`) or a pattern
+    ///   (`parse_pattern`), so each operand, type argument or sub-pattern
+    ///   inside another;
+    /// - the operand of a prefix operator, and each `else if`;
+    /// - each binary operator, `as` and assignment included, for the rest
+    ///   of its chain: the right operand of a chain's `k`th operator is `k`
+    ///   levels deeper than the chain, and so is the type after its `k`th
+    ///   `as`;
+    /// - an item inside a block, and the children of a view or an element.
+    ///
+    /// The file's outermost expression, type, pattern or nested item is
+    /// level 0, so `n` nested parentheses, blocks, calls, `if`s or types
+    /// are `n` levels, and 128 of any one kind parse: `depth`, which counts
+    /// every entry, the outermost's included, is one more than the level.
+    /// A block is not a level of its own: its statements' expressions are.
+    /// Each of these is entered as `nested` enters it, with the depth
+    /// restored after it, but for the binary operators, whose function
+    /// restores it when the chain ends.
+    pub fn deeper(mut self) -> Bool {
+        self.depth += 1
+        if self.depth <= MAX_DEPTH + 1 { return true }
+        if self.too_deep.is_none() {
+            self.too_deep = match self.crossing_stands() {
+                Some(crossing) => Some(crossing)
+                None => Some((self.errors.len(), self.peek_span()))
+            }
+        }
+        false
+    }
+
+    /// The crossing an abandoned attempt made (`rolled_back`), if an error
+    /// has come since: the tokens read again the other way failed before
+    /// the token where it was made, so it stands, as the parser's own
+    /// crossing does, and the parser reads the end of the file (`peek`).
+    pub fn crossing_stands(self) -> Option<(Int, Span)> {
+        match self.rolled_back {
+            Some((errors, span, _)) => {
+                if self.errors.len() > errors { return Some((errors, span)) }
+                None
+            }
+            None => None
+        }
+    }
+
+    /// `in_stmt_ctx`'s save: `stmt_ctx` becomes `on`, and what it was is
+    /// returned, for the caller to restore.
+    pub fn enter_stmt_ctx(mut self, on: Bool) -> Bool {
+        let prev = self.stmt_ctx
+        self.stmt_ctx = on
+        prev
+    }
+
+    /// `in_no_struct_ctx`'s and `in_struct_ok_ctx`'s save:
+    /// `no_struct_literal` becomes `on`, and what it was is returned.
+    pub fn enter_no_struct_ctx(mut self, on: Bool) -> Bool {
+        let prev = self.no_struct_literal
+        self.no_struct_literal = on
+        prev
+    }
+
+    /// Whether a line break comes before the token at the cursor.
+    pub fn line_break_here(self) -> Bool { self.tokens.line_break_before(self.pos) }
+
+    /// Whether the newline rule ends the expression before the token at
+    /// the cursor: a line break before it, in statement context. The
+    /// callers ask it of a `(`, a `[` and a binary `-`.
+    pub fn newline_ends_expr(self) -> Bool { self.stmt_ctx && self.line_break_here() }
+
+    /// Whether a `return` or `break` just read takes no value: the next
+    /// token ends a statement or a block, or a line break comes before it,
+    /// in any context.
+    pub fn bare_keyword_ends(mut self) -> Bool {
+        match self.peek() {
+            Semicolon | RBrace | Eof => return true
+            _ => ()
+        }
+        if !self.line_break_here() { return false }
+        // A `let` or an item after the line break is a statement of its own.
+        if self.check(Let) || self.is_item_start_at(self.pos) {
+            self.halt = self.pos
+            return true
+        }
+        // A token that cannot begin an expression, such as `)`, `,` or
+        // `=>`, is read as before the rule: as the value's start, an error.
+        if !can_start_expr(self.peek()) { return false }
+        // The line after would have been the value: the rule split it off.
+        self.record_split(SplitKindValue)
+        self.halt = self.pos
+        true
+    }
+
+    /// Whether the parser stands where a line break ended a bare `return`
+    /// or `break`: the token there belongs to whatever follows the
+    /// keyword's expression, not to the expression.
+    pub fn halted(self) -> Bool { self.halt == self.pos }
+
+    /// A P0001 in the head of an arm that the newline rule began, by ending
+    /// the arm before at a line break: the first error the arm gave, from
+    /// `first` on, wherever it lands, gets a note naming the rule.
+    pub fn note_arm_split(mut self, begun: Option<NewlineSplit>, first: Int) {
+        let split = match begun {
+            Some(s) => s
+            None => return
+        }
+        if first >= self.errors.len() { return }
+        let note = match split.kind {
+            SplitKindValue => {
+                "a `return` or `break` at the end of a line takes no value, so the "
+                    + "next line begins the next match arm (the newline rule); to return "
+                    + "that line's value, put it on the same line as the `return` or `break`"
+            }
+            _ => {
+                "a line that starts with `(`, `[` or `-` begins the next match arm "
+                    + "(the newline rule); to continue the arm's body, end the line before "
+                    + "with the operator"
+            }
+        }
+        let error = self.errors.at(first)
+        self.errors.set(first, Noted(error, note))
+    }
+
+    /// Record that the newline rule ends a statement before the token at
+    /// the cursor. One split a token: the first recorded there stands.
+    pub fn record_split(mut self, kind: SplitKind) {
+        let at = self.peek_span()
+        for s in self.splits.iter() { if s.at.start == at.start && s.at.end == at.end { return } }
+        self.splits.push(NewlineSplit { at: at, kind: kind })
+    }
+
+    // --- Token access ---
+
+    pub fn peek(self) -> Token {
+        // Past the depth limit, the file has ended.
+        if self.halted() || self.too_deep.is_some() || self.crossing_stands().is_some() {
+            return self.eof
+        }
+        if self.pos < self.tokens.len() { self.tokens.at(self.pos).value } else { self.eof }
+    }
+
+    pub fn peek_span(self) -> Span {
+        if self.pos < self.tokens.len() { self.tokens.at(self.pos).span } else { Span::point(0) }
+    }
+
+    pub fn advance(mut self) -> Spanned<Token> {
+        // A crossing an abandoned attempt made stands if an error came
+        // first, and goes once its token is read.
+        match self.rolled_back {
+            Some((_, _, at)) => {
+                match self.crossing_stands() {
+                    Some(crossing) => {
+                        if self.too_deep.is_none() { self.too_deep = Some(crossing) }
+                        self.rolled_back = None
+                    }
+                    None => if self.pos >= at { self.rolled_back = None }
+                }
+            }
+            None => ()
+        }
+        if self.pos < self.tokens.len() {
+            let tok = self.tokens.at(self.pos)
+            self.pos += 1
+            return tok
+        }
+        Spanned::new(Eof, Span::point(0))
+    }
+
+    pub fn check(self, tok: Token) -> Bool { self.peek().kind() == tok.kind() }
+
+    pub fn eat(mut self, tok: Token) -> Option<Spanned<Token>> {
+        if self.peek().kind() == tok.kind() { Some(self.advance()) } else { None }
+    }
+
+    pub fn expect(mut self, tok: Token, ctx: String) -> Option<Spanned<Token>> {
+        if self.peek().kind() == tok.kind() { return Some(self.advance()) }
+        let span = self.peek_span()
+        self.errors.push(
+            Expected("${tok.description()} (in ${ctx})", self.peek().description(), span),
+        )
+        None
+    }
+
+    pub fn is_at_end(self) -> Bool { self.check(Eof) }
+
+    /// The span of the token just consumed, or `fallback` at the start.
+    pub fn prev_span_or(self, fallback: Span) -> Span {
+        let i = if self.pos > 0 { self.pos - 1 } else { 0 }
+        if i < self.tokens.len() { self.tokens.at(i).span } else { fallback }
+    }
+
+    // The token at `i`, or `Eof` past the end, where the Rust reads
+    // `self.tokens.get(i).map(|t| &t.value)`: every caller asks for a token
+    // other than `Eof`, so `None` and `Eof` match alike.
+    pub fn value_at(self, i: Int) -> Token {
+        if i < self.tokens.len() { self.tokens.at(i).value } else { self.eof }
+    }
+
+    // --- Synchronisation (error recovery) ---
+
+    /// Skip tokens until we find something that looks like the start of an
+    /// item or a closing delimiter. Used at item level for error recovery.
+    pub fn sync_to_item_boundary(mut self) {
+        while !self.is_at_end() {
+            if self.component_at(self.pos) { break }
+            match self.peek() {
+                Fn
+                    | Pub
+                    | Record
+                    | Trait
+                    | Impl
+                    | Type
+                    | Const
+                    | Import
+                    | Module
+                    | Extern
+                    | RBrace => break
+                _ => { let _ = self.advance() }
+            }
+        }
+    }
+
+    /// Skip to next `;` or `}` for statement recovery.
+    pub fn sync_to_stmt_boundary(mut self) {
+        while !self.is_at_end() {
+            match self.peek() {
+                Semicolon => {
+                    let _ = self.advance()
+                    break
+                }
+                RBrace => break
+                _ => { let _ = self.advance() }
+            }
+        }
+    }
+
+    // --- Doc comments ---
+
+    /// Consume consecutive `///` lines and return them, in order.
+    pub fn take_docs(mut self) -> Vec<Spanned<String>> {
+        let mut docs: Vec<Spanned<String>> = Vec::new()
+        while true {
+            match self.peek() {
+                DocComment(text) => {
+                    let span = self.peek_span()
+                    let _ = self.advance()
+                    docs.push(Spanned::new(text, span))
+                }
+                _ => break
+            }
+        }
+        docs
+    }
+
+    /// Report a `///` that documents nothing.
+    pub fn misplaced_doc(mut self, span: Span) { self.errors.push(Custom(MISPLACED_DOC, span)) }
+
+    /// Report and skip the `///` lines here, which document nothing.
+    /// Returns whether there were any.
+    pub fn reject_docs(mut self) -> Bool {
+        let docs = self.take_docs()
+        for d in docs.iter() { self.misplaced_doc(d.span) }
+        !docs.is_empty()
+    }
+
+    /// The index of the first token at or after the cursor that is not a
+    /// `///` line.
+    pub fn docs_end(self) -> Int {
+        let mut i = self.pos
+        while i < self.tokens.len() && is_doc(self.tokens.at(i).value) { i += 1 }
+        i
+    }
+
+    /// Whether the token after any `///` lines is `tok`.
+    pub fn next_after_docs_is(self, tok: Token) -> Bool {
+        let i = self.docs_end()
+        i < self.tokens.len() && self.tokens.at(i).value.kind() == tok.kind()
+    }
+
+    /// Whether the tokens at `i` are `component Name (`, where `component`
+    /// is a keyword (RFC 0001); anywhere else it is an identifier.
+    pub fn component_at(self, i: Int) -> Bool {
+        let is_word = match self.value_at(i) {
+            Ident(w) => w == "component"
+            _ => false
+        }
+        if !(is_word
+            && is_ident(self.value_at(i + 1))
+            && self.value_at(i + 2).kind() == LParen.kind()) {
+            return false
+        }
+        // Its parameters' `)`, then its body's `{`, or a `->` that
+        // `parse_component` refuses.
+        let mut depth = 0
+        let mut k = i + 2
+        while true {
+            if k >= self.tokens.len() { return false }
+            match self.tokens.at(k).value {
+                LParen => depth += 1
+                RParen => {
+                    depth -= 1
+                    if depth == 0 { break }
+                }
+                Eof => return false
+                _ => ()
+            }
+            k += 1
+        }
+        match self.value_at(k + 1) {
+            LBrace => true
+            Arrow => true
+            _ => false
+        }
+    }
+
+    /// Whether the token at `i` starts an item that may appear inside a
+    /// block.
+    pub fn is_item_start_at(self, i: Int) -> Bool {
+        let at_pub = self.value_at(i).kind() == Pub.kind()
+        if self.component_at(i) || (at_pub && self.component_at(i + 1)) { return true }
+        let item = match self.value_at(i) {
+            Fn | Async | Record | Trait | Impl | Type | Const | Import | Module | Extern => true
+            _ => false
+        }
+        let pub_item = match self.value_at(i + 1) {
+            Fn | Record | Trait | Type | Const | Impl => true
+            _ => false
+        }
+        item || (at_pub && pub_item)
+    }
+
+    /// A top-level item's docs and `@attributes`, which may be interleaved.
+    pub fn parse_item_prefix(mut self) -> (Vec<Spanned<String>>, Vec<Attribute>) {
+        let mut docs: Vec<Spanned<String>> = Vec::new()
+        let mut attrs: Vec<Attribute> = Vec::new()
+        while true {
+            for d in self.take_docs().iter() { docs.push(d) }
+            if !self.check(At) { break }
+            for a in self.parse_attributes().iter() { attrs.push(a) }
+        }
+        (docs, attrs)
+    }
+}
+
+// ---------------------------------------------------------------------------
+// Public entry point
+// ---------------------------------------------------------------------------
+
+/// Parse a token stream into a `File` and its errors (lib.rs's `parse`,
+/// grammar.rs's `parse_file`). The file is there even when errors are.
+pub fn parse(tokens: Tokens) -> (File, Vec<ParseError>) {
+    let mut p = new_parser(tokens)
+    let file = p.parse_file()
+    (file, p.errors)
+}
+
+// ---------------------------------------------------------------------------
+// Paths and identifiers
+// ---------------------------------------------------------------------------
+
+impl Parser {
+    pub fn parse_ident(mut self, ctx: String) -> Option<Spanned<String>> {
+        let span = self.peek_span()
+        match self.peek() {
+            Ident(s) => {
+                let _ = self.advance()
+                Some(Spanned::new(s, span))
+            }
+            SelfLower => {
+                let _ = self.advance()
+                Some(Spanned::new("self", span))
+            }
+            SelfUpper => {
+                let _ = self.advance()
+                Some(Spanned::new("Self", span))
+            }
+            _ => {
+                self.errors.push(
+                    Expected("identifier (in ${ctx})", self.peek().description(), span),
+                )
+                None
+            }
+        }
+    }
+
+    pub fn try_parse_path(mut self) -> Option<Path> {
+        let span = self.peek_span()
+        let first = match self.peek() {
+            Ident(s) => {
+                let _ = self.advance()
+                Spanned::new(s, span)
+            }
+            SelfLower => {
+                let _ = self.advance()
+                Spanned::new("self", span)
+            }
+            SelfUpper => {
+                let _ = self.advance()
+                Spanned::new("Self", span)
+            }
+            _ => return None
+        }
+        let mut path = single_path(first)
+        while self.eat(ColonColon).is_some() {
+            let seg_span = self.peek_span()
+            match self.peek() {
+                Ident(s) => {
+                    let _ = self.advance()
+                    path.segments.push(Spanned::new(s, seg_span))
+                }
+                _ => break
+            }
+        }
+        Some(path)
+    }
+
+    pub fn parse_path(mut self, ctx: String) -> Option<Path> {
+        match self.try_parse_path() {
+            Some(p) => Some(p)
+            None => {
+                let span = self.peek_span()
+                self.errors.push(Expected("path (in ${ctx})", self.peek().description(), span))
+                None
+            }
+        }
+    }
+}
+
+// ---------------------------------------------------------------------------
+// Helpers
+// ---------------------------------------------------------------------------
+
+/// Try to convert a `Type::Path` to a `Path` (for trait-impl
+/// disambiguation).
+pub fn type_to_path(ty: Spanned<Type>) -> Option<Path> {
+    match ty.value {
+        TypePath(path, _) => Some(path)
+        _ => None
+    }
+}
+
+/// Whether `tok` can begin an expression: a literal, a name, a bracket, a
+/// prefix operator, a closure, or a keyword that begins one.
+pub fn can_start_expr(tok: Token) -> Bool {
+    match tok {
+        Int(_)
+            | Float(_)
+            | Char(_)
+            | True
+            | False
+            | StrStart
+            | RawStr(_)
+            | LParen
+            | LBracket
+            | LBrace
+            | If
+            | Match
+            | While
+            | For
+            | Return
+            | Break
+            | Continue
+            | Pipe
+            | PipePipe
+            | Ident(_)
+            | SelfLower
+            | SelfUpper
+            | Minus
+            | Bang
+            | Tilde
+            | Star
+            | Amp => true
+        _ => false
+    }
+}
+
+// Whether `tok` is a `///` line.
+pub fn is_doc(tok: Token) -> Bool {
+    match tok {
+        DocComment(_) => true
+        _ => false
+    }
+}
+
+// Whether `tok` is an identifier.
+pub fn is_ident(tok: Token) -> Bool {
+    match tok {
+        Ident(_) => true
+        _ => false
+    }
+}
diff --git a/selfhost/src/token.nova b/selfhost/src/token.nova
index ed3ee1da376f910766cd9cab1483f381c5dfb694..c886403dfc22bdd540ce3161ad0741249bcb59cb 100644
--- a/selfhost/src/token.nova
+++ b/selfhost/src/token.nova
@@ -1,6 +1,6 @@
 // The token kinds of the Nova language, and each kind's description: a
 // port of crates/nova-lexer/src/token.rs, kind for kind and word for word,
-// and of the `Spanned` and `Span` the Rust lexer takes from
+// and of the `Spanned` and `Span` the Rust lexer and parser take from
 // nova-diagnostics. A span is the byte offsets `start..end`; N lexes one
 // file at a time, so it keeps no `FileId`.
 
@@ -108,14 +108,32 @@ pub type Token =
     // End of file
     | Eof
 
-/// A byte range in the source: `start..end`.
+/// A byte range in the source: `start..end` (nova-diagnostics' `Span`).
 pub record Span { pub start: Int, pub end: Int }
 
-/// A token and its span (nova-diagnostics' `Spanned<Token>`).
-pub record Spanned { pub value: Token, pub span: Span }
+impl Span {
+    /// A zero-width span at `offset` (`Span::point`).
+    pub fn point(offset: Int) -> Span { Span { start: offset, end: offset } }
+
+    /// The span that covers both (`Span::merge`).
+    pub fn merge(self, other: Span) -> Span {
+        let start = if other.start < self.start { other.start } else { self.start }
+        let end = if other.end > self.end { other.end } else { self.end }
+        Span { start: start, end: end }
+    }
+}
+
+/// A value and its span (nova-diagnostics' `Spanned<T>`): a token, or a
+/// node or name of the tree.
+pub record Spanned<T> { pub value: T, pub span: Span }
+
+impl<T> Spanned<T> {
+    /// `value` at `span` (`Spanned::new`).
+    pub fn new(value: T, span: Span) -> Spanned<T> { Spanned { value: value, span: span } }
+}
 
 /// A token spanning `start..end`.
-pub fn spanned(value: Token, start: Int, end: Int) -> Spanned {
+pub fn spanned(value: Token, start: Int, end: Int) -> Spanned<Token> {
     Spanned { value: value, span: Span { start: start, end: end } }
 }
 
@@ -218,4 +236,105 @@ impl Token {
             Eof => "end of file"
         }
     }
+
+    /// The token's kind, a number for each variant in declaration order:
+    /// what the parser compares where the Rust compares
+    /// `std::mem::discriminant`s, so `Ident("a")` and `Ident("b")` are one
+    /// kind.
+    pub fn kind(self) -> Int {
+        match self {
+            Let => 0
+            Mut => 1
+            Const => 2
+            Fn => 3
+            Return => 4
+            If => 5
+            Else => 6
+            While => 7
+            For => 8
+            In => 9
+            Break => 10
+            Continue => 11
+            Match => 12
+            Type => 13
+            Record => 14
+            Trait => 15
+            Impl => 16
+            Import => 17
+            Module => 18
+            Pub => 19
+            Async => 20
+            Await => 21
+            Extern => 22
+            Unsafe => 23
+            True => 24
+            False => 25
+            As => 26
+            Is => 27
+            Where => 28
+            With => 29
+            SelfLower => 30
+            SelfUpper => 31
+            LParen => 32
+            RParen => 33
+            LBracket => 34
+            RBracket => 35
+            LBrace => 36
+            RBrace => 37
+            Comma => 38
+            Semicolon => 39
+            Colon => 40
+            ColonColon => 41
+            Arrow => 42
+            FatArrow => 43
+            Dot => 44
+            DotDot => 45
+            DotDotEq => 46
+            At => 47
+            Question => 48
+            Eq => 49
+            EqEq => 50
+            BangEq => 51
+            Lt => 52
+            LtEq => 53
+            Gt => 54
+            GtEq => 55
+            AmpAmp => 56
+            PipePipe => 57
+            Bang => 58
+            Plus => 59
+            Minus => 60
+            Star => 61
+            Slash => 62
+            Percent => 63
+            PlusEq => 64
+            MinusEq => 65
+            StarEq => 66
+            SlashEq => 67
+            PercentEq => 68
+            Amp => 69
+            Pipe => 70
+            Caret => 71
+            Tilde => 72
+            LtLt => 73
+            GtGt => 74
+            AmpEq => 75
+            PipeEq => 76
+            CaretEq => 77
+            LtLtEq => 78
+            GtGtEq => 79
+            Int(_) => 80
+            Float(_) => 81
+            Char(_) => 82
+            StrStart => 83
+            StrPart(_) => 84
+            StrEnd => 85
+            InterpOpen => 86
+            InterpClose => 87
+            RawStr(_) => 88
+            Ident(_) => 89
+            DocComment(_) => 90
+            Eof => 91
+        }
+    }
 }
````

### Task 6: The AST comparison, and N's depth

Spec §4.3 to §4.5, §4.8; decisions 17, 22, 23, 33, 41 and 46.

**Files:**
- Modify: `crates/nova-cli/tests/selfhost/compare.rs` (`AST`; an AST
  line's source line)
- Test: `crates/nova-cli/tests/selfhost.rs` (the AST pass and its
  assertions, the every-kind test's N side, N's depth test, the AST parts
  of the older tests)

- [ ] **Step 1: The tests.** `patch_block t6-tests`. The patch holds, in
  `selfhost.rs`: the module doc's 5.1b paragraph; `AST` imported; the
  counts' doc and `AST_SOUP_SEED`; `agree` through both dumps, returning
  both; `agree_on_inputs` with its `ast` flag, and each caller's; the
  corpus test counting one tree a file, the soup's and the mutations'
  finding trees, the deep inputs' finding P0002; `both_sides_refuse_no_file`
  for both dumps; the mismatch test's AST lines; the every-kind test
  reading N's dumps too; `n_on_stack` and
  `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`.
- [ ] **Step 2: Run, and see the tests fail.** First the build, which
  ends with `Finished`.
  - `SH`. Expected: it does not compile, ``error[E0432]: unresolved import `compare::AST` ``, an `error[E0277]`
    that follows from it, and
    ``error: could not compile `nova-cli` (test "selfhost") due to 2 previous errors`` (decision 47).
- [ ] **Step 3: The code.** `patch_block t6-code`: `compare.rs`'s `AST`,
  `["parse", "--dump-ast"]` and `["ast"]`; `source_line` reading a
  `PARSE` line's start and a node's span's.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH`. Expected: `27 passed; 0 failed`.
  - `S` (6 min 5 s). Expected: `2839 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `cli tests: the AST dumps compared, and N at 128 levels`.

**Patch `t6-tests`:**

<!-- patch t6-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 31b44f26cd0a4708567ce5d1e559197f9faf6f52..3df871427e8bbab881aec7c436554fd33d172358 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -14,6 +14,18 @@
 //!   `tests/selfhost/cases/`; on the edge inputs; and on the fuzzer's soup
 //!   and mutated corpus files from fixed seeds (`tests/selfhost/fuzz.rs`).
 //!
+//! 5.1b (spec §4, ADR 0052): the hidden `nova parse --dump-ast FILE…`, and
+//! N's `selfhost ast FILE…`, which must print the same tree, errors and
+//! notes byte for byte on every input set above, the token dump's beside
+//! it, and must each name every kind of node somewhere in the corpus and
+//! the cases; the error cases in `tests/selfhost/cases/`, the lexer's and
+//! the parser's tests' inline sources; the deep inputs, through both
+//! differentials, through S0's `parse --check` and `check`, which must meet
+//! the depth limit, P0002, and never overflow their stack, and through N on
+//! a 1 MB main thread, Windows' size, at 128 and 129 levels; and S0's
+//! big-stack thread, which keeps a panic's message and exit code, and
+//! leaves `nova run`'s program on the main thread.
+//!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
 //!
@@ -35,19 +47,28 @@ use std::sync::OnceLock;
 use std::thread::JoinHandle;
 use std::time::{Duration, Instant};
 
-use compare::{first_difference, report, Dump, TOKENS};
+use compare::{first_difference, report, Dump, AST, TOKENS};
 use fuzz::SplitMix64;
 
 /// How many files one process lexes.
 const BATCH: usize = 50;
 
 /// The fixed seeds and counts every run checks: 6,000 inputs of soup and
-/// 750 mutated corpus files. Measured on a 4-core Linux machine, debug
-/// build: the whole file runs in about 10 s, building N included.
+/// 750 mutated corpus files through the token dump, and 2,000 of the soup's
+/// ([`AST_SOUP_SEED`]'s) and all 750 mutated files through the AST dump
+/// too. Measured on a 4-core Linux machine, debug build: the whole file ran
+/// in about 10 s with the token dump alone, building N included, and runs
+/// in about 21 s (about 50 s of CPU) with the AST dump; all the soup through
+/// both would add about 1 s of CPU.
 const SOUP_SEEDS: [u64; 3] = [1, 2, 3];
 const SOUP_COUNT: usize = 2000;
+/// The one soup seed whose inputs go through the AST dump as well: soup is
+/// made for the lexer, and most of it is one parse error after another, so
+/// a third of it is kept for the parser, and its time for the mutations.
+const AST_SOUP_SEED: u64 = 1;
 const MUTATION_SEEDS: [u64; 3] = [101, 102, 103];
 const MUTATION_COUNT: usize = 250;
+
 /// The deep inputs' seeds, and how many of each seed's are mixed: with
 /// every other kind at each of five depths, 90 inputs a seed.
 const DEEP_SEEDS: [u64; 2] = [201, 202];
@@ -516,16 +537,20 @@ fn agree_on(dump: &Dump, dir: &Path, files: &[String], batch: usize, code: i32)
     all
 }
 
-/// [`agree_on`] the token dump.
+/// [`agree_on`] the token dump, then the AST dump (ADR 0052): every input
+/// set goes through both. Returns S0's token dump, then its AST dump.
 fn agree(dir: &Path, files: &[String], batch: usize, code: i32) -> String {
-    agree_on(&TOKENS, dir, files, batch, code)
+    let tokens = agree_on(&TOKENS, dir, files, batch, code);
+    let ast = agree_on(&AST, dir, files, batch, code);
+    tokens + &ast
 }
 
 /// Write `inputs` to a fixed directory named after `name`, as
-/// `f00000.nova` and on, and check that both sides agree on them. Returns
-/// S0's output. The fuzzer's generators all come through here, 5.1b's deep
-/// nesting included.
-fn agree_on_inputs(name: &str, inputs: &[String]) -> String {
+/// `f00000.nova` and on, and check that both sides agree on them, in the
+/// token dump, and in the AST dump too if `ast`. Returns S0's output, its
+/// token dump and, if `ast`, its AST dump after it. The fuzzer's generators all come through here, 5.1b's deep nesting
+/// included.
+fn agree_on_inputs(name: &str, inputs: &[String], ast: bool) -> String {
     let dir = fresh_dir(name);
     let mut files = Vec::with_capacity(inputs.len());
     for (i, input) in inputs.iter().enumerate() {
@@ -533,7 +558,11 @@ fn agree_on_inputs(name: &str, inputs: &[String]) -> String {
         std::fs::write(dir.join(&file), input).expect("write a fuzz input");
         files.push(file);
     }
-    agree(&dir, &files, 100, 0)
+    if ast {
+        agree(&dir, &files, 100, 0)
+    } else {
+        agree_on(&TOKENS, &dir, &files, 100, 0)
+    }
 }
 
 /// `count` mutated files from `seed`, each one of `files`, under `root`,
@@ -597,7 +626,10 @@ fn the_corpus_dumps_agree() {
     // 281 under std/, tests/ and examples/ alone at 5.0c.
     assert!(files.len() >= 290, "only {} corpus files", files.len());
     assert!(files.iter().any(|f| f == "selfhost/src/lexer.nova"));
-    agree(&root(), &files, BATCH, 0);
+    let dumps = agree(&root(), &files, BATCH, 0);
+    // Each file's tree, so the AST dumps were compared too.
+    let trees = dumps.lines().filter(|l| l.starts_with("(File")).count();
+    assert_eq!(trees, files.len());
 }
 
 /// Both lexers must agree on the corpus as a Windows checkout has it
@@ -636,7 +668,7 @@ fn the_fixed_cases_agree() {
 
 #[test]
 fn the_edge_inputs_agree() {
-    agree_on_inputs("edges", &fuzz::edges());
+    agree_on_inputs("edges", &fuzz::edges(), true);
 }
 
 #[test]
@@ -646,8 +678,11 @@ fn the_soup_agrees_and_holds_every_lex_error() {
         dumps.push_str(&agree_on_inputs(
             &format!("soup-{seed}"),
             &fuzz::soups(seed, SOUP_COUNT),
+            seed == AST_SOUP_SEED,
         ));
     }
+    // One seed's trees were compared too.
+    assert!(dumps.contains("\n(File"), "no tree in the soup's dumps");
     for error in LEX_ERRORS {
         let found = dumps.lines().any(|line| {
             line.starts_with("ERR ")
@@ -663,7 +698,13 @@ fn the_soup_agrees_and_holds_every_lex_error() {
 #[test]
 fn the_mutated_corpus_agrees() {
     for seed in MUTATION_SEEDS {
-        agree_on_inputs(&format!("mutated-{seed}"), &mutations(seed, MUTATION_COUNT));
+        let out = agree_on_inputs(
+            &format!("mutated-{seed}"),
+            &mutations(seed, MUTATION_COUNT),
+            true,
+        );
+        // Their trees were compared too.
+        assert!(out.contains("\n(File"), "seed {seed}");
     }
 }
 
@@ -678,7 +719,7 @@ fn a_longer_search_from_the_environment() {
         .split_once(':')
         .and_then(|(s, c)| Some((s.parse::<u64>().ok()?, c.parse::<usize>().ok()?)))
         .expect("NOVA_SELFHOST_FUZZ is SEED:COUNT");
-    agree_on_inputs("search-soup", &fuzz::soups(seed, count));
+    agree_on_inputs("search-soup", &fuzz::soups(seed, count), true);
     // Mutations go in batches of 500, each from a seed of its own, so a long
     // search keeps few inputs on disk at a time.
     let mut done = 0;
@@ -687,6 +728,7 @@ fn a_longer_search_from_the_environment() {
         agree_on_inputs(
             "search-mutated",
             &mutations(seed.wrapping_add(done as u64), n),
+            true,
         );
         done += n;
     }
@@ -714,11 +756,14 @@ fn unreadable_files_are_reported_alike() {
     assert!(out.ends_with("== \nIOERR NotFound\n"), "{out}");
 }
 
-/// No file at all: both sides refuse it, print nothing and exit 2.
+/// No file at all: both sides refuse it, print nothing and exit 2, for
+/// either dump.
 #[test]
 fn both_sides_refuse_no_file() {
-    let out = compare_batch(&TOKENS, &root(), &[], 2, &nova(), selfhost());
-    assert_eq!(out, Ok(String::new()));
+    for dump in [&TOKENS, &AST] {
+        let out = compare_batch(dump, &root(), &[], 2, &nova(), selfhost());
+        assert_eq!(out, Ok(String::new()));
+    }
 }
 
 #[test]
@@ -764,6 +809,31 @@ fn the_mismatch_report_names_the_file_and_line() {
     )
     .expect("a difference");
     assert_eq!((m.file.as_deref(), m.dump_line), (Some("a.nova"), 1));
+    // An AST dump's line names its source line by its node's start or its
+    // error's.
+    std::fs::write(dir.join("c.nova"), "fn f() {\n    1\n}\n").expect("write c.nova");
+    let files = ["b.nova", "c.nova"].map(String::from);
+    let s0 = run(&nova(), AST.s0, &dir, &files).expect("run S0").stdout;
+    let s0 = String::from_utf8(s0).expect("a dump of UTF-8 files is UTF-8");
+    for (from, to, part) in [
+        (
+            "found `let`",
+            "found `lot`",
+            "in b.nova, at line 5 of its dump, source line 3",
+        ),
+        (
+            "Int:1)",
+            "Int:2)",
+            "in c.nova, at line 5 of its dump, source line 2",
+        ),
+    ] {
+        let wrong = s0.replace(from, to);
+        assert_ne!(s0, wrong, "{from} is not in:\n{s0}");
+        let m = first_difference(s0.as_bytes(), wrong.as_bytes()).expect("a difference");
+        let text = report(&m, &AST, &dir, &nova(), Path::new("selfhost"));
+        assert!(text.contains(part), "{part:?} is not in:\n{text}");
+        assert!(text.contains("selfhost ast "), "{text}");
+    }
 }
 
 /// A side that fails, here a stand-in for N that is `nova` given an
@@ -982,14 +1052,16 @@ fn kinds_on(line: &str, found: &mut std::collections::HashSet<String>) {
 }
 
 /// Every kind of node, field-less value and literal the dump can print
-/// appears in S0's dump of some corpus file or fixed case (spec §4.3).
+/// appears in S0's dump of some corpus file or fixed case, and in N's
+/// (spec §4.3): it follows from the dumps' being equal, which
+/// `the_corpus_dumps_agree` and `the_fixed_cases_agree` check, and each
+/// side is checked on its own all the same.
 #[test]
 fn every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases() {
     let root = root();
     let mut files = corpus();
     nova_files(&root, &cases_dir(), &[], &mut files);
-    let s0: &[&str] = &["parse", "--dump-ast"];
-    for (side, program, args) in [("S0", nova(), s0)] {
+    for (side, program, args) in [("S0", nova(), AST.s0), ("N", selfhost().into(), AST.n)] {
         let mut found = std::collections::HashSet::new();
         for chunk in files.chunks(BATCH) {
             let out = run(&program, args, &root, chunk).unwrap_or_else(|e| panic!("{e}"));
@@ -1017,7 +1089,84 @@ fn the_deep_inputs_agree() {
             .into_iter()
             .map(|(_, _, src)| src)
             .collect();
-        agree_on_inputs(&format!("deep-{seed}"), &inputs);
+        let out = agree_on_inputs(&format!("deep-{seed}"), &inputs, true);
+        // The AST dumps were compared too: past the limit, P0002.
+        assert!(out.contains(" this is nested too deeply"), "seed {seed}");
+    }
+}
+
+/// N, on a main thread of `stack` KB where `ulimit -s` can set it (Unix),
+/// or of the platform's size (Windows: 1 MB): `selfhost ast file` in `dir`,
+/// its exit code and stdout.
+fn n_on_stack(dir: &Path, stack: Option<u32>, file: &str) -> (Option<i32>, String) {
+    let mut cmd = match stack {
+        Some(kb) => {
+            let mut sh = Command::new("sh");
+            sh.args([
+                "-c",
+                &format!(
+                    "ulimit -s {kb} && exec '{}' ast '{file}'",
+                    selfhost().display()
+                ),
+            ]);
+            sh
+        }
+        None => {
+            let mut n = Command::new(selfhost());
+            n.args(["ast", file]);
+            n
+        }
+    };
+    let out = output_within(
+        cmd.current_dir(dir).stdin(Stdio::null()),
+        CHILD_LIMIT,
+        &format!("selfhost ast {file} on a {stack:?} KB stack"),
+    )
+    .unwrap_or_else(|e| panic!("{e}"));
+    (
+        out.status.code(),
+        String::from_utf8_lossy(&out.stdout).into_owned(),
+    )
+}
+
+/// N's own stack (spec §4.4, §4.8): every kind of nesting the deep
+/// generator writes, at 128 levels and at 129, through N on a main thread
+/// of 8 MB, Linux's, and of 1 MB, Windows', on every system: it neither
+/// overflows nor reports P0002 at 128, and reports one P0002 at 129. A
+/// Nova program runs on its platform's main thread, so on Windows N runs
+/// as it is, and elsewhere `ulimit -s` sets the size. N needed 256 KB at
+/// most when measured (ADR 0052).
+#[test]
+fn n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread() {
+    let dir = fresh_dir("deep-n");
+    let mut rng = SplitMix64::new(DEEP_SEEDS[0]);
+    let mut files = Vec::new();
+    for kind in fuzz::DEEP_KINDS {
+        for depth in [128, 129] {
+            let file = format!("{kind}-{depth}.nova");
+            std::fs::write(dir.join(&file), fuzz::deep(&mut rng, kind, depth))
+                .expect("write a deep input");
+            files.push((depth, file));
+        }
+    }
+    let stacks: &[Option<u32>] = if cfg!(unix) {
+        &[Some(8192), Some(1024)]
+    } else {
+        &[None]
+    };
+    for &stack in stacks {
+        for (depth, file) in &files {
+            let (code, out) = n_on_stack(&dir, stack, file);
+            let p0002 = out
+                .lines()
+                .filter(|l| l.starts_with("PARSE ") && l.contains("nested too deeply"))
+                .count();
+            let expected = usize::from(*depth > 128);
+            assert!(
+                code == Some(0) && p0002 == expected,
+                "N on {file} with a {stack:?} KB stack: exit {code:?}, {p0002} P0002\n{out}"
+            );
+        }
     }
 }
 
````

**Patch `t6-code`:**

<!-- patch t6-code -->

````diff
diff --git a/crates/nova-cli/tests/selfhost/compare.rs b/crates/nova-cli/tests/selfhost/compare.rs
index 3da8f66042ef637c20206a60886d4ea16124bede..4c1506ab4f358738f0a86ef4504099ca64894729 100644
--- a/crates/nova-cli/tests/selfhost/compare.rs
+++ b/crates/nova-cli/tests/selfhost/compare.rs
@@ -11,12 +11,18 @@ pub struct Dump {
     pub n: &'static [&'static str],
 }
 
-/// 5.1a's token dump. 5.1b's AST dump is a second `Dump` beside it.
+/// 5.1a's token dump.
 pub const TOKENS: Dump = Dump {
     s0: &["parse", "--dump-tokens"],
     n: &["tokens"],
 };
 
+/// 5.1b's AST dump (ADR 0052).
+pub const AST: Dump = Dump {
+    s0: &["parse", "--dump-ast"],
+    n: &["ast"],
+};
+
 /// Where two dumps first differ.
 #[derive(Debug, PartialEq, Eq)]
 pub struct Mismatch {
@@ -100,12 +106,14 @@ fn text(bytes: &[u8]) -> String {
 }
 
 /// The 1-based source line of the offset a dump line starts with (a token's
-/// start, or an error's or comment's), if `file` can be read from `dir`.
+/// start, or an error's or comment's; in an AST dump, a node's span's start
+/// or a parse error's), if `file` can be read from `dir`.
 fn source_line(dir: &Path, file: &str, line: &str) -> Option<usize> {
-    let mut fields = line.split(' ');
+    let mut fields = line.trim_start().split(' ');
     let first = fields.next()?;
     let offset: usize = match first {
-        "ERR" | "COMMENT" => fields.next()?.parse().ok()?,
+        "ERR" | "COMMENT" | "PARSE" => fields.next()?.parse().ok()?,
+        node if node.starts_with('(') => fields.next()?.split_once("..")?.0.parse().ok()?,
         _ => first.parse().ok()?,
     };
     let source = std::fs::read(dir.join(file)).ok()?;
````

### Task 7: ADR 0052 and the records

Spec §8; decisions 27 to 44.

**Files:**
- Create: `docs/adr/0052-the-self-hosted-parser-the-ast-dump-and-the-depth-limit.md`
- Modify: `CHANGELOG.md`, `docs/phase-5-plan.md`,
  `nova-spec/00-MASTER-SPEC.md`, `nova-spec/11-PARSER.md`,
  `nova-spec/12-TYPESYSTEM.md`

- [ ] **Step 1: The records.** `patch_block t7-code`:
  - **ADR 0052,** the self-hosted parser, the AST dump and the depth
    limit: its status, accepted, with the branch; its context; its
    decisions: the dump's grammar, its table and an example of every
    kind; the depth limit, its levels, postfix chains among them, the
    stop, the measured maxima and the probes; the thread, the `Send`
    argument and the probes; the error cases; the deep generator; N's AST
    and parser with their line counts, each idiom's translation, N's
    names and its tests; the comparison and its time; N's stack at the
    limit; the speed; the rule that the grammar changes in both
    compilers; its consequences; its references.
  - **`CHANGELOG.md`,** Unreleased: the heading names 5.1b; Added (the
    hidden `nova parse --dump-ast`; the self-hosted parser and its tree,
    the comparison, the speed); Changed (P0002, postfix chains included;
    every command's pipeline on a 64 MB thread).
  - **Dated notes,** each dated 2026-10-31, naming the branch
    `claude/loving-archimedes-frtf5q-51b` and citing the spec:
    `docs/phase-5-plan.md`'s 5.1 (5.1b built, the speed and N's stack,
    postfix chains counted); `00-MASTER-SPEC.md`'s Phase 5 (step 2
    ported); `11-PARSER.md` §4 (the AST dump) and §5 (P0002).
  - **`12-TYPESYSTEM.md`:** P0002 in the codes' table, and rows for
    P0001 and L0001, which it lacked.
- [ ] **Step 2: Read them against the code.** Each record's claim against
  the tree: the ADR's decisions against Tasks 1 to 6 and this plan's
  decisions; its examples against `nova parse --dump-ast` of their
  sources (decision 39), by the prototype's script, which writes the
  ADR's block afresh, and a comparison:

```sh
cat > "$P/adr_examples.py" <<'EOF'
#!/usr/bin/env python3
"""ADR 0052's examples: for each kind of node, a small source, S0's dump
of it (`nova parse --dump-ast`), and the subtree of the first node of that
kind, closed as a tree of its own. Prints Markdown. Usage: adr_examples.py NOVA"""
import os, subprocess, sys, tempfile

NOVA = sys.argv[1]

def body(s):
    return "fn f() {\n    " + s + "\n}\n"

# (kinds shown, source). The first node of the first kind in each entry is
# the example; the others listed appear inside it.
EXAMPLES = [
    ("File", "fn main() {}\n"),
    ("Split", body("g\n    (x)")),
    ("Doc", "/// Adds.\nfn f() {}\n"),
    ("Attr", "@test(should_panic)\nfn t() {}\n"),
    ("Item.Function Function Block", "pub async fn f() {}\n"),
    ("Param", "fn f(mut x: Int, self) {}\n"),
    ("TypeParam Path", "fn f<T: std::fmt::Display>() {}\n"),
    ("WhereBound", "fn f<T>(x: T) where T: Show + Eq {}\n"),
    ("Item.Record Record RecordField", "pub record P {\n    pub x: Int,\n}\n"),
    ("Item.Type TypeDecl TypeDef.Sum Variant", "type C = | Red | Rgb(Int, Int)\n"),
    ("TypeDef.Alias", "type Name = String\n"),
    ("Item.Trait TraitDecl", "trait Show: Base {}\n"),
    ("TraitItem.Required FunctionSig", "trait Show {\n    fn show(self) -> String\n}\n"),
    ("TraitItem.Provided", "trait Show {\n    fn twice(self) {}\n}\n"),
    ("TraitItem.AssocType", "trait It {\n    type Item: Show\n}\n"),
    ("Item.Impl ImplBlock", "impl Show for P {}\n"),
    ("AssocTypeBinding", "impl It for P {\n    type Item = Int\n}\n"),
    ("Item.Const ConstDecl", "const K: Int = 1\n"),
    ("Item.Import Import ImportKind.Simple", "import json\n"),
    ("ImportKind.Alias", "import json as j\n"),
    ("ImportKind.List", "import a::{b, c}\n"),
    ("Item.Module Module", "module util\n"),
    ("Item.Extern ExternBlock ExternItem.Fn", "extern \"C\" {\n    fn abs(x: Int) -> Int\n}\n"),
    ("Stmt.Let", body("let mut x: Int = 1")),
    ("Stmt.Expr", body("g();\n    1")),
    ("Stmt.Item", body("fn g() {}")),
    ("Expr.Lit", body("1")),
    ("Expr.Path", body("a::b")),
    ("Expr.Tuple", body("(true, r\"a\\n\", 2.5)")),
    ("Expr.Array", body("['a', 'b']")),
    ("Expr.ArrayRepeat", body("[0; 3]")),
    ("Expr.Block", body("{ 1 }")),
    ("Expr.If", body("if c { 1 } else { 2 }")),
    ("Expr.Match MatchArm", body("match x { 0 if y => 1, _ => 2 }")),
    ("Expr.While", body("while c { g() }")),
    ("Expr.For", body("for i in xs { g(i) }")),
    ("Expr.Range", body("0..=9")),
    ("Expr.Return", body("return 1")),
    ("Expr.Break", body("break")),
    ("Expr.Continue", body("continue")),
    ("Expr.Closure", body("|x: Int| -> Int x")),
    ("Expr.Record FieldInit", body("P { x, y: 1, ..p }")),
    ("Expr.Binary", body("a + b")),
    ("Expr.Unary", body("-x")),
    ("Expr.Call", body("g(1, x)")),
    ("Expr.Index", body("a[i]")),
    ("Expr.Field", body("p.x")),
    ("Expr.Try", body("g()?")),
    ("Expr.Await", body("g().await")),
    ("Expr.Cast", body("n as Float")),
    ("Expr.Assign", body("x += 1")),
    ("Expr.StringInterp StringPart.Lit StringPart.Expr", body("\"a${b}\"")),
    ("Expr.View ViewNode.Element ViewArg.Positional ViewArg.Named ViewNode.Expr",
     body("view { button(on, label: \"x\") { text(\"hi\") } }")),
    ("Pattern.Wildcard", body("let _ = 1")),
    ("Pattern.Lit", body("match c { 'a' => 1 }")),
    ("Pattern.Ident", body("let x = 1")),
    ("Pattern.Binding", body("match n { d @ 1..=9 => d }")),
    ("Pattern.TupleStruct", body("let Some(x) = o")),
    ("Pattern.Record FieldPat", body("let P { x, y: 0, .. } = p")),
    ("Pattern.Tuple", body("let (a, b) = t")),
    ("Pattern.Array", body("let [a, b] = xs")),
    ("Pattern.Or", body("match c { A | B => 1 }")),
    ("Pattern.Range", body("match c { 'a'..='z' => 1 }")),
    ("Pattern.Path", body("match c { C::Red => 1 }")),
    ("Type.Path", "fn f(x: Vec<Int>) {}\n"),
    ("Type.Ref", "fn f(x: &mut Int) {}\n"),
    ("Type.Ptr", "fn f(x: *Int) {}\n"),
    ("Type.Array", "fn f(x: [Int]) {}\n"),
    ("Type.Tuple", "fn f(x: (Int, Bool)) {}\n"),
    ("Type.Fn", "fn f(x: fn(Int) -> Bool) {}\n"),
    ("Type.Optional", "fn f(x: Int?) {}\n"),
    ("Type.Infer", body("|x| x")),
]

def dump(src):
    with tempfile.TemporaryDirectory() as d:
        p = os.path.join(d, "a.nova")
        open(p, "w").write(src)
        out = subprocess.run([NOVA, "parse", "--dump-ast", "a.nova"], cwd=d,
                             capture_output=True, text=True).stdout
    return out.splitlines()[1:]

def subtree(lines, kind):
    for i, l in enumerate(lines):
        t = l.lstrip(" ")
        if t.startswith("(" + kind) and t[len(kind) + 1:len(kind) + 2] in (" ", ")", ""):
            depth = len(l) - len(t)
            out = [l]
            for m in lines[i + 1:]:
                mt = m.lstrip(" ")
                if not m.startswith(" ") or len(m) - len(mt) <= depth:
                    break
                out.append(m)
            # Re-close it as a tree of its own: its own nodes' parens only.
            stripped = [x.rstrip(")") for x in out]
            depths = [(len(x) - len(x.lstrip(" "))) // 2 for x in stripped]
            res = []
            for j, x in enumerate(stripped):
                nxt = depths[j + 1] if j + 1 < len(stripped) else depths[0]
                closes = depths[j] - nxt + 1
                res.append(x[depth:] + ")" * closes)
            return res, i
    return None, None

print("```")
for kinds, src in EXAMPLES:
    lines = dump(src)
    first = kinds.split()[0]
    tree, _ = subtree(lines, first)
    if tree is None:
        sys.exit("no %s in the dump of %r:\n%s" % (first, src, "\n".join(lines)))
    for k in kinds.split()[1:]:
        if not any(x.lstrip(" ").startswith("(" + k) for x in tree):
            sys.exit("no %s inside %s" % (k, first))
    shown = src.replace("\n", "\\n")
    print(";; " + kinds.replace(" ", ", ") + ": " + shown)
    print("\n".join(tree))
    print()
print("```")
EOF
```

```sh
cat > "$P/adrcheck.sh" <<'EOF'
#!/bin/bash
# adrcheck.sh: run from the repository's root, after the build, with $P set
# and $P/adr_examples.py written. Regenerates ADR 0052's examples, one of
# every kind, from S0's `nova parse --dump-ast` of their sources, and
# compares them with the ADR's block, byte for byte.
set -u
: "${P:?set P to the scratch directory}"
adr=docs/adr/0052-the-self-hosted-parser-the-ast-dump-and-the-depth-limit.md
python3 -I "$P/adr_examples.py" "$PWD/target/debug/nova" > "$P/adr-examples.txt" || exit 1
awk '/^\*\*One example of every kind\*\*/ { on = 1 } on && /^```$/ { n++; print; if (n == 2) exit; next } on && n == 1 { print }' "$adr" > "$P/adr-block.txt"
n=$(grep -c '^;; ' "$P/adr-block.txt")
if cmp -s "$P/adr-examples.txt" "$P/adr-block.txt"; then echo "adr: $n examples, identical"; else
  echo "adr: the examples differ"; diff "$P/adr-block.txt" "$P/adr-examples.txt" | head -20; exit 1; fi
EOF
```

  ```sh
  P="$P" bash "$P/adrcheck.sh"
  ```

  Expected: `adr: 72 examples, identical`. Then the CHANGELOG's entries against the
  patches;
  every figure (167 forms, 278 cases, 262 recorded, the seeds and counts,
  the corpus's 25, the chain and router figures, N's 2,485 lines and 17
  tests, 6.1×)
  against this plan's; the branch name in each note. Any difference goes
  in the ledger and to Task 8's review, which fixes it in its one commit,
  so that this task's tree stays the replay's.
- [ ] **Step 3: The checks,** as the conventions list them, after the
  build.
- [ ] **Step 4: Commit**
  `docs: ADR 0052, the self-hosted parser, and the notes for 5.1b`.
- [ ] **Step 5: Run,** on the commit.
  - `git diff --check HEAD~1 HEAD`. Expected: nothing.
  - `git diff --stat 5a68e7b9 HEAD -- . ':!docs/superpowers/plans'`.
    Expected: nothing: but for this plan's file, which the prototype does
    not hold, the tree is the prototype's last, `5a68e7b9`. A difference
    stops the task: record it in the ledger.

**Patch `t7-code`:**

<!-- patch t7-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 873aeeff83fe3c93a71987d6e96633f398ca271c..cce468fc170aac4168e4ecf81653d72cb58bff07 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -15,7 +15,8 @@ Phase 5.0a, the small language and CI (spec
 patterns (§4; ADR 0049), and 5.0c, std (§5; ADR 0050). Phase 5.1a, the
 self-hosted lexer and the differential (spec
 `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
-§3; ADR 0051).
+§3; ADR 0051), and 5.1b, the self-hosted parser, the AST dump and the
+depth limit (§4; ADR 0052).
 
 ### Added
 
@@ -168,9 +169,59 @@ self-hosted lexer and the differential (spec
   stability promise. `nova parse` without it still takes one file, and
   refuses a second in clap's words as before; the further files are a
   hidden argument only the flag reads (ADR 0051).
+- **`nova parse --dump-ast FILE…`, hidden:** each file's tree as an
+  indented S-expression, one node per line with its span and fields, its
+  newline splits, then its lex errors, its parse errors and their notes,
+  after a header `== FILE`, with `--dump-tokens`'s further files, `IOERR`
+  and exit code 2. The text comes from `nova_parser::dump_ast`. It is
+  absent from `--help` and carries no stability promise (ADR 0052).
+- **The self-hosted parser and its tree, in `selfhost/`:** N parses a file
+  as `nova-parser` does, function for function, and builds the same tree;
+  `selfhost ast FILE…` prints the same dump as `nova parse --dump-ast`,
+  errors, notes and newline splits included. The differential compares
+  the two dumps on every input set it compares the token dumps on (one
+  soup seed of three), checks that every kind of node appears in both
+  sides' dumps, and runs N on 128 and 129 levels of every kind of nesting
+  on a 1 MB main thread. A change to the grammar now changes both parsers
+  in one pull request. N lexes, parses and dumps all of std about 6× as
+  slowly as release S0 with a release runtime (ADR 0052).
 
 ### Changed
 
+- **Deeply nested source is now P0002, not a crash.** Past 128 levels of
+  nesting, "this is nested too deeply (more than 128 levels)" at the token
+  where the limit was crossed; the parser stops there, so it is the
+  file's last error and the items before it are kept. A level is an
+  expression, a type or a pattern inside another, a prefix operator's
+  operand, an `else if`, each further operator of a binary chain, each
+  further operator of a postfix chain (a method call `.f(…)` is one), an
+  item inside a block, or a view's children; the outermost is level 0, so
+  128 nested parentheses, blocks, calls, `if`s or types parse and 129 do
+  not. A call's arguments are a level deeper than the call, so after a
+  name a chain of 129 method calls without arguments parses and 130 does
+  not, and of 128 with arguments parses and 129 does not; a router built
+  as one chain from `Router::new()`, a route a `.get("/", h)`, parses 127
+  routes and not 128. Counting postfix chains, which the 5.1 spec's
+  levels do not name, is the user's ruling of 2026-10-31 (ADR 0052).
+  Before, a debug `nova check` aborted with a stack overflow: it passed
+  230 nested parentheses and aborted at 240, passed 190 nested `if`s and
+  aborted at 200, passed a chain of 750 operators and aborted at 800, and
+  passed a chain of 500 method calls and aborted at 600. But for the test
+  cases written to sit at the limit, the deepest file in the repository
+  nests 25 levels. `ParseError::code` gives P0002 or P0001, and every
+  caller attaches it.
+- **Every command runs its pipeline on a thread with a 64 MB stack,** as
+  do the language server's checker and `nova dev`'s rebuilds, so the
+  depth limit, not the platform's main-thread stack (1 MB on Windows),
+  decides what deeply nested source meets. A decimal number of 4,000
+  characters no longer overflows `nova` on a 1 MB main thread (ADR 0051's
+  known bug), though one of 210,000 digits or more still aborts a debug
+  build's 64 MB thread, `logos`'s per-digit recursion being no level of
+  the limit, and shorter ones lex in time quadratic in their length
+  (20,000 digits take about 8 s). A panic prints and exits 101 as
+  before; `nova run` still runs the program on the main thread, the stack
+  it has when built (ADR 0052).
+
 - **The newline rule.** In a block, a line break ends an expression
   before a `(`, a `[` or a binary `-`: the token begins the next
   statement or match arm. A `return` or `break` at the end of a line
diff --git a/docs/adr/0052-the-self-hosted-parser-the-ast-dump-and-the-depth-limit.md b/docs/adr/0052-the-self-hosted-parser-the-ast-dump-and-the-depth-limit.md
new file mode 100644
index 0000000000000000000000000000000000000000..b451517090d9704c004d3d26571e26212a465b7c
--- /dev/null
+++ b/docs/adr/0052-the-self-hosted-parser-the-ast-dump-and-the-depth-limit.md
@@ -0,0 +1,1065 @@
+# ADR 0052 — The self-hosted parser, the AST dump and the depth limit
+
+## Status
+
+Accepted (2026-10-31). Branch `claude/loving-archimedes-frtf5q-51b`
+(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+§4). §§1–5 are S0's side of 5.1b: the AST dump and `nova parse
+--dump-ast`, the depth limit and P0002, S0's pipeline thread, the error
+cases and the deep generator. §§6–10 are N's: its AST and parser, each
+Rust idiom's Nova translation, the comparison of the two dumps, N's own
+stack at the limit, the speed figures, and the rule that the grammar
+changes in both compilers at once. The limit of 128 levels, the code
+P0002, the 64 MB thread and the error corpus are the spec's
+recommendations, which the user did not overturn; the dump's indented
+S-expression with spans is the user's decision 3 of 2026-10-30; counting
+each postfix operator after a chain's first toward the same 128 levels,
+which the spec's list of levels does not name, is the user's choice of
+2026-10-31 (their ruling 8); the rest is what the spec left to the plan.
+
+## Context
+
+5.1b ports `nova-parser` and `nova-ast` to Nova, and compares N's tree
+and errors with S0's through a dump both print (spec §4.3). S0's only
+rendering of a tree was `nova parse --ast`, Rust's pretty `Debug`: 51.8 MB
+for all of std, with floats and strings in Rust's own syntax. S0's parser
+had no depth guard: on its 8 MB main thread, under `nova check`, a debug
+build passed 230 nested parentheses and aborted at 240, passed 190 nested
+`if`s and aborted at 200, passed a chain of 750 operators and aborted at
+800, and passed a chain of 500 method calls and aborted at 600; Windows
+gives a main thread 1 MB. ADR 0051 found that `logos`
+recurses once per digit, so a 4,000-character decimal number overflowed
+S0 at 1 MB. The parser's errors were all P0001, attached by each caller.
+
+## Decision
+
+### 1. The AST dump
+
+`nova_ast::dump::dump_file(&File) -> String` writes the tree;
+`nova_parser::dump_ast(source) -> String` lexes and parses `source` and
+adds its errors; the hidden `nova parse --dump-ast FILE…` prints, for
+each file, `== FILE` (the path as given) and its dump, or `IOERR` and the
+error's kind for a file it cannot read, then exits 2 after the rest, as
+`--dump-tokens` does (ADR 0051). The tree lives in `nova-ast`, where the
+nodes are; the errors need the parser's and the lexer's types, so the
+whole dump is `nova-parser`'s. The flag refuses `--ast`, `--check` and
+`--dump-tokens`, and `--help` is unchanged.
+
+**The grammar.** Every line ends with `\n`, the last included.
+
+```
+dump      = tree errors
+tree      = "(File" { "\n" node(1) } ")" "\n"
+node(d)   = indent(d) "(" kind [ " " span ] { " " value } { " " field }
+            { "\n" node(d + 1) } ")"
+indent(d) = d times "  "
+span      = uint ".." uint                       -- byte offsets
+field     = key "=" value                        -- key: the Rust field's name
+value     = bool | word | string | name | path | literal
+bool      = "true" | "false"
+word      = a field-less enum's variant: Visibility (Pub, Private),
+            BinOp, UnOp, AssignOp, SplitKind (Call, Index, Minus, Value);
+            or Some, None (an element's children)
+string    = '"' { char } '"'                     -- a char below 0x20, 0x7F,
+                                                 -- '"' and '\' as "\x" hh
+name      = string "@" span                      -- a Spanned<String>
+path      = name { "::" name }
+literal   = "Int:" int | "Float:" 16 hex digits of f64::to_bits
+          | "Str:" string | "Char:" code point in decimal | "Bool:" bool
+errors    = { "ERR " uint " " uint " " message "\n" }
+            { "PARSE " uint " " uint " " message "\n" { "NOTE " message "\n" } }
+message   = the Display text, each char below 0x20 and 0x7F as "\x" hh
+```
+
+`hh` is two lowercase hex digits; `uint` and `int` are decimal. A node's
+`)` comes right after its last child's, so a line ends with as many `)` as
+nodes it closes. Its values are positional (a tuple variant's scalars, a
+`Doc`'s text, an `Attr`'s name and arguments, a `Path` node's segments);
+its fields are its named scalars, in the struct's order, then its children
+in the struct's order. An absent `Option` and an empty list print nothing.
+A `Spanned` node prints its span; a node the tree holds without one (every
+struct but `Block` and `Path` under a `Spanned`, an `Item` under
+`Stmt.Item`, a `Block` under `Expr.Block`) prints none. Lex errors come
+first, all of them, as the callers render them before the parse errors.
+
+**Every kind.** A struct's kind is its name, and an enum value's
+`Enum.Variant`, both as Rust names them, but for three the spec named:
+`Doc` (a `///` line), `Attr` (`Attribute`) and `Split` (`NewlineSplit`).
+Two of them print otherwise than the spec's sketch (§4.3) **(differs
+from the spec)**: a split's kind is the field `kind=Call`, not a bare
+`Call`, the uniform `key=value` form of every field-less enum; and an
+attribute's name and arguments are names with their spans,
+`"test"@1..5 "should_panic"@6..18`, not a bare name and quoted strings,
+since each is a `Spanned<String>` and the dump gives every name its span.
+An enum value that holds a struct is a node whose one child is the
+struct's node. `nova_ast::dump::KINDS` lists the 167 forms (24 structs,
+`Doc`, the 18 enums' 119 variants as node kinds, `key=Word` values and
+literal prefixes, and each of the 11 `bool` fields' two values and
+`children=`'s two, after their node's kind, as `(Type.Ptr is_mut=false`,
+but `Pattern.Ident`'s `is_mut=true`, which no parser builds). A test reads
+the `struct` and `enum` declarations of `nova-ast`'s sources and
+checks that `KINDS` is exactly what they imply, so a new variant or
+`bool` field fails until it is listed; another checks that each form
+appears in S0's dump, and in N's, of some corpus file or case.
+
+| Node | Span | On its line, in order | Children, in order |
+|---|---|---|---|
+| `File` | – | – | `Item.*` (spanned), `Split` |
+| `Split` | `at` | `kind=` | – |
+| `Doc` | yes | text | – |
+| `Attr` | the `@` | name, each argument (names) | – |
+| `Item.Function` … `Item.Extern` (9) | under `File` | – | its struct |
+| `Function` | – | `vis= is_async= is_component= name=` | `Doc`, `Attr`, `TypeParam`, `Param`, return type, `WhereBound`, `Block` |
+| `FunctionSig` | – | `is_async= name=` | `Doc`, `TypeParam`, `Param`, return type, `WhereBound` |
+| `Param` | – | `is_mut= name=` | type |
+| `TypeParam` | – | `name=` | `Path` (bounds) |
+| `WhereBound` | – | – | type, `Path` (bounds) |
+| `Record` | – | `vis= name=` | `Doc`, `Attr`, `TypeParam`, `RecordField` |
+| `RecordField` | – | `vis= name=` | `Doc`, type |
+| `TypeDecl` | – | `vis= name=` | `Doc`, `Attr`, `TypeParam`, `TypeDef.*` |
+| `TypeDef.Alias` | – | – | type |
+| `TypeDef.Sum` | – | – | `Variant` |
+| `Variant` | – | `name=` | `Doc`, types |
+| `TraitDecl` | – | `vis= name=` | `Doc`, `Attr`, `TypeParam`, `Path` (supertraits), `WhereBound`, `TraitItem.*` |
+| `TraitItem.Required` / `.Provided` | – | – | `FunctionSig` / `Function` |
+| `TraitItem.AssocType` | – | `name=` | `Doc`, `Path` (bounds) |
+| `ImplBlock` | – | – | `Doc`, `Attr`, `TypeParam`, `Path` (the trait), type, `WhereBound`, `Function`, `ConstDecl`, `AssocTypeBinding` |
+| `AssocTypeBinding` | – | `name=` | `Doc`, type |
+| `ConstDecl` | – | `vis= name=` | `Doc`, `Attr`, type, expression |
+| `Import` | – | – | `Doc`, `Attr`, `Path`, `ImportKind.*` |
+| `ImportKind.Simple` / `.Alias` / `.List` | – | – / name / names | – |
+| `Module` | – | – | `Doc`, `Attr`, `Path` |
+| `ExternBlock` | – | `abi=` (a string, if any) | `Doc`, `Attr`, `ExternItem.Fn` |
+| `ExternItem.Fn` | – | – | `FunctionSig` |
+| `Path` | yes | the path | – |
+| `Block` | under a `Spanned` | – | `Stmt.*`, the trailing expression |
+| `Stmt.Let` | yes | `is_mut=` | pattern, type, initializer |
+| `Stmt.Expr` / `Stmt.Item` | yes | – | expression / `Item.*` (no span) |
+| `Expr.Lit` | yes | literal | – |
+| `Expr.Path` | yes | the path | – |
+| `Expr.Tuple`, `Expr.Array` | yes | – | expressions |
+| `Expr.ArrayRepeat` | yes | – | `init`, `len` |
+| `Expr.Block` | yes | – | `Block` (no span) |
+| `Expr.If` | yes | – | condition, `Block`, else |
+| `Expr.Match` | yes | – | scrutinee, `MatchArm` |
+| `MatchArm` | – | – | pattern, guard, body |
+| `Expr.While` | yes | – | condition, `Block` |
+| `Expr.For` | yes | – | pattern, iterable, `Block` |
+| `Expr.Range` | yes | `inclusive=` | `lo`, `hi` |
+| `Expr.Return`, `Expr.Break` | yes | – | the value |
+| `Expr.Continue` | yes | – | – |
+| `Expr.Closure` | yes | – | `Param`, return type, body |
+| `Expr.Record` | yes | `path=` | `FieldInit`, the base |
+| `FieldInit` | – | `name=` | value |
+| `Expr.Binary` | yes | `op=` (BinOp) | `lhs`, `rhs` |
+| `Expr.Unary` | yes | `op=` (UnOp) | operand |
+| `Expr.Call` | yes | – | callee, arguments |
+| `Expr.Index` | yes | – | target, index |
+| `Expr.Field` | yes | `field=` | target |
+| `Expr.Try`, `Expr.Await` | yes | – | operand |
+| `Expr.Cast` | yes | – | operand, type |
+| `Expr.Assign` | yes | `op=` (AssignOp) | `lhs`, `rhs` |
+| `Expr.StringInterp` | yes | – | `StringPart.*` |
+| `StringPart.Lit` / `.Expr` | – | text / – | – / expression |
+| `Expr.View` | yes | – | `ViewNode.*` |
+| `ViewNode.Element` | yes | `name= children=Some\|None` | `ViewArg.*`, `ViewNode.*` |
+| `ViewNode.Expr` | yes | – | expression |
+| `ViewArg.Named` / `.Positional` | – | `name=` / – | expression |
+| `Pattern.Wildcard` | yes | – | – |
+| `Pattern.Lit` | yes | literal | – |
+| `Pattern.Ident` | yes | `is_mut= name=` | – |
+| `Pattern.Binding` | yes | `name=` | pattern |
+| `Pattern.TupleStruct` | yes | `path=` | patterns |
+| `Pattern.Record` | yes | `path= rest=` | `FieldPat` |
+| `FieldPat` | – | `name=` | pattern |
+| `Pattern.Tuple`, `.Array`, `.Or` | yes | – | patterns |
+| `Pattern.Range` | yes | `inclusive=` | `lo`, `hi` |
+| `Pattern.Path` | yes | the path | – |
+| `Type.Path` | yes | `path=` | arguments |
+| `Type.Ref`, `Type.Ptr` | yes | `is_mut=` | type |
+| `Type.Array`, `Type.Optional` | yes | – | type |
+| `Type.Tuple` | yes | – | types |
+| `Type.Fn` | yes | – | parameters, return type |
+| `Type.Infer` | yes | – | – |
+
+`ViewNode.Element`'s `children=` is the one field the spec's rules did not
+give: its children are an `Option` of a list, and `None` and an empty list
+would print alike.
+
+**One example of every kind**, each S0's dump of the source after it,
+reduced to the node's own subtree, the kinds after the first appearing
+inside it:
+
+```
+;; File: fn main() {}\n
+(File
+  (Item.Function 0..12
+    (Function vis=Private is_async=false is_component=false name="main"@3..7
+      (Block 10..12))))
+
+;; Split: fn f() {\n    g\n    (x)\n}\n
+(Split 19..20 kind=Call)
+
+;; Doc: /// Adds.\nfn f() {}\n
+(Doc 0..9 " Adds.")
+
+;; Attr: @test(should_panic)\nfn t() {}\n
+(Attr 0..1 "test"@1..5 "should_panic"@6..18)
+
+;; Item.Function, Function, Block: pub async fn f() {}\n
+(Item.Function 0..19
+  (Function vis=Pub is_async=true is_component=false name="f"@13..14
+    (Block 17..19)))
+
+;; Param: fn f(mut x: Int, self) {}\n
+(Param is_mut=true name="x"@9..10
+  (Type.Path 12..15 path="Int"@12..15))
+
+;; TypeParam, Path: fn f<T: std::fmt::Display>() {}\n
+(TypeParam name="T"@5..6
+  (Path 8..11 "std"@8..11::"fmt"@13..16::"Display"@18..25))
+
+;; WhereBound: fn f<T>(x: T) where T: Show + Eq {}\n
+(WhereBound
+  (Type.Path 20..21 path="T"@20..21)
+  (Path 23..27 "Show"@23..27)
+  (Path 30..32 "Eq"@30..32))
+
+;; Item.Record, Record, RecordField: pub record P {\n    pub x: Int,\n}\n
+(Item.Record 0..32
+  (Record vis=Pub name="P"@11..12
+    (RecordField vis=Pub name="x"@23..24
+      (Type.Path 26..29 path="Int"@26..29))))
+
+;; Item.Type, TypeDecl, TypeDef.Sum, Variant: type C = | Red | Rgb(Int, Int)\n
+(Item.Type 0..30
+  (TypeDecl vis=Private name="C"@5..6
+    (TypeDef.Sum
+      (Variant name="Red"@11..14)
+      (Variant name="Rgb"@17..20
+        (Type.Path 21..24 path="Int"@21..24)
+        (Type.Path 26..29 path="Int"@26..29)))))
+
+;; TypeDef.Alias: type Name = String\n
+(TypeDef.Alias
+  (Type.Path 12..18 path="String"@12..18))
+
+;; Item.Trait, TraitDecl: trait Show: Base {}\n
+(Item.Trait 0..19
+  (TraitDecl vis=Private name="Show"@6..10
+    (Path 12..16 "Base"@12..16)))
+
+;; TraitItem.Required, FunctionSig: trait Show {\n    fn show(self) -> String\n}\n
+(TraitItem.Required
+  (FunctionSig is_async=false name="show"@20..24
+    (Param is_mut=false name="self"@25..29
+      (Type.Infer 25..29))
+    (Type.Path 34..40 path="String"@34..40)))
+
+;; TraitItem.Provided: trait Show {\n    fn twice(self) {}\n}\n
+(TraitItem.Provided
+  (Function vis=Private is_async=false is_component=false name="twice"@20..25
+    (Param is_mut=false name="self"@26..30
+      (Type.Infer 26..30))
+    (Block 32..34)))
+
+;; TraitItem.AssocType: trait It {\n    type Item: Show\n}\n
+(TraitItem.AssocType name="Item"@20..24
+  (Path 26..30 "Show"@26..30))
+
+;; Item.Impl, ImplBlock: impl Show for P {}\n
+(Item.Impl 0..18
+  (ImplBlock
+    (Path 5..9 "Show"@5..9)
+    (Type.Path 14..15 path="P"@14..15)))
+
+;; AssocTypeBinding: impl It for P {\n    type Item = Int\n}\n
+(AssocTypeBinding name="Item"@25..29
+  (Type.Path 32..35 path="Int"@32..35))
+
+;; Item.Const, ConstDecl: const K: Int = 1\n
+(Item.Const 0..16
+  (ConstDecl vis=Private name="K"@6..7
+    (Type.Path 9..12 path="Int"@9..12)
+    (Expr.Lit 15..16 Int:1)))
+
+;; Item.Import, Import, ImportKind.Simple: import json\n
+(Item.Import 0..11
+  (Import
+    (Path 7..11 "json"@7..11)
+    (ImportKind.Simple)))
+
+;; ImportKind.Alias: import json as j\n
+(ImportKind.Alias "j"@15..16)
+
+;; ImportKind.List: import a::{b, c}\n
+(ImportKind.List "b"@11..12 "c"@14..15)
+
+;; Item.Module, Module: module util\n
+(Item.Module 0..11
+  (Module
+    (Path 7..11 "util"@7..11)))
+
+;; Item.Extern, ExternBlock, ExternItem.Fn: extern "C" {\n    fn abs(x: Int) -> Int\n}\n
+(Item.Extern 0..40
+  (ExternBlock abi="C"
+    (ExternItem.Fn
+      (FunctionSig is_async=false name="abs"@20..23
+        (Param is_mut=false name="x"@24..25
+          (Type.Path 27..30 path="Int"@27..30))
+        (Type.Path 35..38 path="Int"@35..38)))))
+
+;; Stmt.Let: fn f() {\n    let mut x: Int = 1\n}\n
+(Stmt.Let 13..31 is_mut=true
+  (Pattern.Ident 21..22 is_mut=false name="x"@21..22)
+  (Type.Path 24..27 path="Int"@24..27)
+  (Expr.Lit 30..31 Int:1))
+
+;; Stmt.Expr: fn f() {\n    g();\n    1\n}\n
+(Stmt.Expr 13..17
+  (Expr.Call 13..16
+    (Expr.Path 13..14 "g"@13..14)))
+
+;; Stmt.Item: fn f() {\n    fn g() {}\n}\n
+(Stmt.Item 13..22
+  (Item.Function
+    (Function vis=Private is_async=false is_component=false name="g"@16..17
+      (Block 20..22))))
+
+;; Expr.Lit: fn f() {\n    1\n}\n
+(Expr.Lit 13..14 Int:1)
+
+;; Expr.Path: fn f() {\n    a::b\n}\n
+(Expr.Path 13..17 "a"@13..14::"b"@16..17)
+
+;; Expr.Tuple: fn f() {\n    (true, r"a\n", 2.5)\n}\n
+(Expr.Tuple 13..32
+  (Expr.Lit 14..18 Bool:true)
+  (Expr.Lit 20..26 Str:"a\x5cn")
+  (Expr.Lit 28..31 Float:4004000000000000))
+
+;; Expr.Array: fn f() {\n    ['a', 'b']\n}\n
+(Expr.Array 13..23
+  (Expr.Lit 14..17 Char:97)
+  (Expr.Lit 19..22 Char:98))
+
+;; Expr.ArrayRepeat: fn f() {\n    [0; 3]\n}\n
+(Expr.ArrayRepeat 13..19
+  (Expr.Lit 14..15 Int:0)
+  (Expr.Lit 17..18 Int:3))
+
+;; Expr.Block: fn f() {\n    { 1 }\n}\n
+(Expr.Block 13..18
+  (Block
+    (Expr.Lit 15..16 Int:1)))
+
+;; Expr.If: fn f() {\n    if c { 1 } else { 2 }\n}\n
+(Expr.If 13..34
+  (Expr.Path 16..17 "c"@16..17)
+  (Block 18..23
+    (Expr.Lit 20..21 Int:1))
+  (Expr.Block 29..34
+    (Block
+      (Expr.Lit 31..32 Int:2))))
+
+;; Expr.Match, MatchArm: fn f() {\n    match x { 0 if y => 1, _ => 2 }\n}\n
+(Expr.Match 13..44
+  (Expr.Path 19..20 "x"@19..20)
+  (MatchArm
+    (Pattern.Lit 23..24 Int:0)
+    (Expr.Path 28..29 "y"@28..29)
+    (Expr.Lit 33..34 Int:1))
+  (MatchArm
+    (Pattern.Wildcard 36..37)
+    (Expr.Lit 41..42 Int:2)))
+
+;; Expr.While: fn f() {\n    while c { g() }\n}\n
+(Expr.While 13..28
+  (Expr.Path 19..20 "c"@19..20)
+  (Block 21..28
+    (Expr.Call 23..26
+      (Expr.Path 23..24 "g"@23..24))))
+
+;; Expr.For: fn f() {\n    for i in xs { g(i) }\n}\n
+(Expr.For 13..33
+  (Pattern.Ident 17..18 is_mut=false name="i"@17..18)
+  (Expr.Path 22..24 "xs"@22..24)
+  (Block 25..33
+    (Expr.Call 27..31
+      (Expr.Path 27..28 "g"@27..28)
+      (Expr.Path 29..30 "i"@29..30))))
+
+;; Expr.Range: fn f() {\n    0..=9\n}\n
+(Expr.Range 13..18 inclusive=true
+  (Expr.Lit 13..14 Int:0)
+  (Expr.Lit 17..18 Int:9))
+
+;; Expr.Return: fn f() {\n    return 1\n}\n
+(Expr.Return 13..21
+  (Expr.Lit 20..21 Int:1))
+
+;; Expr.Break: fn f() {\n    break\n}\n
+(Expr.Break 13..18)
+
+;; Expr.Continue: fn f() {\n    continue\n}\n
+(Expr.Continue 13..21)
+
+;; Expr.Closure: fn f() {\n    |x: Int| -> Int x\n}\n
+(Expr.Closure 13..30
+  (Param is_mut=false name="x"@14..15
+    (Type.Path 17..20 path="Int"@17..20))
+  (Type.Path 25..28 path="Int"@25..28)
+  (Expr.Path 29..30 "x"@29..30))
+
+;; Expr.Record, FieldInit: fn f() {\n    P { x, y: 1, ..p }\n}\n
+(Expr.Record 13..31 path="P"@13..14
+  (FieldInit name="x"@17..18)
+  (FieldInit name="y"@20..21
+    (Expr.Lit 23..24 Int:1))
+  (Expr.Path 28..29 "p"@28..29))
+
+;; Expr.Binary: fn f() {\n    a + b\n}\n
+(Expr.Binary 13..18 op=Add
+  (Expr.Path 13..14 "a"@13..14)
+  (Expr.Path 17..18 "b"@17..18))
+
+;; Expr.Unary: fn f() {\n    -x\n}\n
+(Expr.Unary 13..15 op=Neg
+  (Expr.Path 14..15 "x"@14..15))
+
+;; Expr.Call: fn f() {\n    g(1, x)\n}\n
+(Expr.Call 13..20
+  (Expr.Path 13..14 "g"@13..14)
+  (Expr.Lit 15..16 Int:1)
+  (Expr.Path 18..19 "x"@18..19))
+
+;; Expr.Index: fn f() {\n    a[i]\n}\n
+(Expr.Index 13..17
+  (Expr.Path 13..14 "a"@13..14)
+  (Expr.Path 15..16 "i"@15..16))
+
+;; Expr.Field: fn f() {\n    p.x\n}\n
+(Expr.Field 13..16 field="x"@15..16
+  (Expr.Path 13..14 "p"@13..14))
+
+;; Expr.Try: fn f() {\n    g()?\n}\n
+(Expr.Try 13..17
+  (Expr.Call 13..16
+    (Expr.Path 13..14 "g"@13..14)))
+
+;; Expr.Await: fn f() {\n    g().await\n}\n
+(Expr.Await 13..22
+  (Expr.Call 13..16
+    (Expr.Path 13..14 "g"@13..14)))
+
+;; Expr.Cast: fn f() {\n    n as Float\n}\n
+(Expr.Cast 13..23
+  (Expr.Path 13..14 "n"@13..14)
+  (Type.Path 18..23 path="Float"@18..23))
+
+;; Expr.Assign: fn f() {\n    x += 1\n}\n
+(Expr.Assign 13..19 op=AddAssign
+  (Expr.Path 13..14 "x"@13..14)
+  (Expr.Lit 18..19 Int:1))
+
+;; Expr.StringInterp, StringPart.Lit, StringPart.Expr: fn f() {\n    "a${b}"\n}\n
+(Expr.StringInterp 13..20
+  (StringPart.Lit "a")
+  (StringPart.Expr
+    (Expr.Path 17..18 "b"@17..18)))
+
+;; Expr.View, ViewNode.Element, ViewArg.Positional, ViewArg.Named, ViewNode.Expr: fn f() {\n    view { button(on, label: "x") { text("hi") } }\n}\n
+(Expr.View 13..59
+  (ViewNode.Element 20..57 name="button"@20..26 children=Some
+    (ViewArg.Positional
+      (Expr.Path 27..29 "on"@27..29))
+    (ViewArg.Named name="label"@31..36
+      (Expr.StringInterp 38..41
+        (StringPart.Lit "x")))
+    (ViewNode.Expr 45..55
+      (Expr.Call 45..55
+        (Expr.Path 45..49 "text"@45..49)
+        (Expr.StringInterp 50..54
+          (StringPart.Lit "hi"))))))
+
+;; Pattern.Wildcard: fn f() {\n    let _ = 1\n}\n
+(Pattern.Wildcard 17..18)
+
+;; Pattern.Lit: fn f() {\n    match c { 'a' => 1 }\n}\n
+(Pattern.Lit 23..26 Char:97)
+
+;; Pattern.Ident: fn f() {\n    let x = 1\n}\n
+(Pattern.Ident 17..18 is_mut=false name="x"@17..18)
+
+;; Pattern.Binding: fn f() {\n    match n { d @ 1..=9 => d }\n}\n
+(Pattern.Binding 23..32 name="d"@23..24
+  (Pattern.Range 27..32 inclusive=true
+    (Pattern.Lit 27..28 Int:1)
+    (Pattern.Lit 31..32 Int:9)))
+
+;; Pattern.TupleStruct: fn f() {\n    let Some(x) = o\n}\n
+(Pattern.TupleStruct 17..24 path="Some"@17..21
+  (Pattern.Ident 22..23 is_mut=false name="x"@22..23))
+
+;; Pattern.Record, FieldPat: fn f() {\n    let P { x, y: 0, .. } = p\n}\n
+(Pattern.Record 17..34 path="P"@17..18 rest=true
+  (FieldPat name="x"@21..22)
+  (FieldPat name="y"@24..25
+    (Pattern.Lit 27..28 Int:0)))
+
+;; Pattern.Tuple: fn f() {\n    let (a, b) = t\n}\n
+(Pattern.Tuple 17..23
+  (Pattern.Ident 18..19 is_mut=false name="a"@18..19)
+  (Pattern.Ident 21..22 is_mut=false name="b"@21..22))
+
+;; Pattern.Array: fn f() {\n    let [a, b] = xs\n}\n
+(Pattern.Array 17..23
+  (Pattern.Ident 18..19 is_mut=false name="a"@18..19)
+  (Pattern.Ident 21..22 is_mut=false name="b"@21..22))
+
+;; Pattern.Or: fn f() {\n    match c { A | B => 1 }\n}\n
+(Pattern.Or 23..28
+  (Pattern.Ident 23..24 is_mut=false name="A"@23..24)
+  (Pattern.Ident 27..28 is_mut=false name="B"@27..28))
+
+;; Pattern.Range: fn f() {\n    match c { 'a'..='z' => 1 }\n}\n
+(Pattern.Range 23..32 inclusive=true
+  (Pattern.Lit 23..26 Char:97)
+  (Pattern.Lit 29..32 Char:122))
+
+;; Pattern.Path: fn f() {\n    match c { C::Red => 1 }\n}\n
+(Pattern.Path 23..29 "C"@23..24::"Red"@26..29)
+
+;; Type.Path: fn f(x: Vec<Int>) {}\n
+(Type.Path 8..16 path="Vec"@8..11
+  (Type.Path 12..15 path="Int"@12..15))
+
+;; Type.Ref: fn f(x: &mut Int) {}\n
+(Type.Ref 8..16 is_mut=true
+  (Type.Path 13..16 path="Int"@13..16))
+
+;; Type.Ptr: fn f(x: *Int) {}\n
+(Type.Ptr 8..12 is_mut=false
+  (Type.Path 9..12 path="Int"@9..12))
+
+;; Type.Array: fn f(x: [Int]) {}\n
+(Type.Array 8..13
+  (Type.Path 9..12 path="Int"@9..12))
+
+;; Type.Tuple: fn f(x: (Int, Bool)) {}\n
+(Type.Tuple 8..19
+  (Type.Path 9..12 path="Int"@9..12)
+  (Type.Path 14..18 path="Bool"@14..18))
+
+;; Type.Fn: fn f(x: fn(Int) -> Bool) {}\n
+(Type.Fn 8..23
+  (Type.Path 11..14 path="Int"@11..14)
+  (Type.Path 19..23 path="Bool"@19..23))
+
+;; Type.Optional: fn f(x: Int?) {}\n
+(Type.Optional 8..12
+  (Type.Path 8..11 path="Int"@8..11))
+
+;; Type.Infer: fn f() {\n    |x| x\n}\n
+(Type.Infer 14..15)
+
+```
+
+### 2. The depth limit, P0002
+
+**A level.** `Parser::deeper` raises one counter, `depth`, and every level
+is entered through it. It counts:
+
+- each expression (`parse_expr`), type (`parse_type`) and pattern
+  (`parse_pattern`): so every operand inside parentheses, a block's
+  statements, a call's arguments, an index, an array, a record literal's
+  fields, an interpolation hole, a closure's body, a `return`'s value, the
+  heads of `if`, `while`, `for` and `match`, a match arm's guard and body;
+  each type argument; each sub-pattern;
+- the operand of a prefix operator, `&` and `&mut` included;
+- each `else if`;
+- each operator of a binary chain, assignment, `..` and `as` included,
+  for the rest of its chain: the right operand of a chain's `k`th
+  operator is `k` levels deeper than the chain's first operand, and so is
+  the type after its `k`th `as` (`Parser::parse_type_here` reads a type at
+  the cursor's level; `parse_type` is `nested(parse_type_here)`);
+- each postfix operator of a chain after its first (a call, an index, a
+  field, `?`, `.await`), for the rest of the chain, a method call
+  `.name(…)` counting as one operator: the `k`th operator of a chain is
+  `k - 1` levels deeper than its receiver, and a call's or an index's
+  expressions one deeper again, as any expression is. The spec does not
+  list postfix chains: they are counted by the user's choice of
+  2026-10-31 (ruling 8), asked because, uncounted, a chain of about
+  5,000 method calls overflowed `nova check` even on the 64 MB thread
+  (§3); see below;
+- an item inside a block;
+- the children of a `view` and of each element.
+
+The file's outermost expression, type, pattern or nested item is level 0,
+so `n` nested constructs of any one kind are `n` levels: 128 nested
+parentheses, blocks, calls, `if`s (an `if` and its `else if`s count
+alike), `match`es, prefix operators, binary operators, casts, types,
+patterns or items parse, and 129 are P0002, as the spec's gate (§4.8)
+reads. `depth` counts every entry, the outermost's included, so it is one
+more than the level, and `deeper` refuses once it passes `MAX_DEPTH + 1`.
+(The first build of this counted the outermost as level 1, so only 127
+parentheses fitted, and the type after `as` was a level of its own beyond
+its operator's.) A block is no level of its own: its statements'
+expressions are. Each entry but the binary and postfix operators' is
+`Parser::nested(f)`, which saves the depth, calls `deeper`, runs `f` and
+restores the depth however `f` ends. `parse_postfix_from` saves the depth,
+runs the chain (`parse_postfix_chain`, which calls `deeper` after taking
+each operator but the first and a method call's `(`) and restores it
+however the chain ends, so a view child, which reads a postfix expression
+outside `parse_expr`, gets it back too. Each binary function
+(`parse_assign`, `parse_range`, `parse_or`, …, `parse_mul`, `parse_cast`)
+saves the depth on entry, calls `deeper` after each operator it takes, and
+restores the depth when its chain ends; a failure returns `None` straight
+up to `parse_expr`, which restores it. So, as a statement, `((x))` is 2
+levels deep, `{ { x } }` 2, `if c { if d { x } }` 2 (an `if`'s condition
+and its block's tail are one deeper than the `if`), `if a { x } else if b
+{ y }` 2, `1 + 2 + x` 2, `!!x` 2, `0 as Int as T` 2, `a.b.c` 1,
+`a.f().g()` 1, `a.f(x)` 0 with `x` at 1, and `fn g() { fn h() { x } }` 2;
+as a parameter's type, `Vec<Vec<Int>>` is 2. So after a name 129 field
+accesses in a row parse, and 129 method calls without arguments; a call's
+arguments are a level deeper than the call, so 128 calls with arguments
+parse and the 129th's are P0002. `Router::new()` is itself the chain's
+first operator, so a router built as one chain, one `.get("/", h)` a
+route, parses 127 routes and is P0002 at the 128th's arguments (measured
+on both parsers). Past the limit, P0002 is at the token after the
+operator that crossed it, as after a binary operator.
+
+**Past the limit.** `nova_parser::MAX_DEPTH` is 128. The first time the
+depth passes it, `deeper` records the number of errors so far and the span
+of the token at the cursor, and returns `false`; the caller returns
+`None`. From then on `peek` reads the end of the file, as it does after a
+bare `return`, so every function unwinds as at the end of the file. When
+the file ends, the errors pushed since are dropped and
+`ParseError::TooDeep { span }` is pushed: "this is nested too deeply (more
+than 128 levels)". The items finished before are kept, the one being
+parsed is dropped, no split is recorded after it, and P0002 is the
+file's last error. `ParseError::code()` gives "P0002" for it, "P0001" for
+every other, and every caller (`nova parse`, the driver's loader, the
+resolver's std loader, `nova fmt`) attaches it; the language server's
+organize-imports treats P0002 as a syntax error too. The dump's `PARSE`
+line carries no code: P0002's message names it.
+
+**An abandoned attempt.** The parser tries the tokens after a type's `<`
+as generic arguments and rolls back if they are not: after a cast, `x as
+Int < (a)` is then a comparison, and a type elsewhere ends before the
+`<`, which is then an error. The attempt can be deeper than the reading
+that follows: in `x as Int < ((a))` the attempt reads `((a))` two levels
+below the start of the line (the `as`, then the type argument), and the
+comparison reads it one below (the `<`), so 127 parentheses took the
+attempt past the limit and S0 reported P0002 where
+the base parsed the line (a fault the second fact-check found, in both
+parsers). The rollback (`Parser::roll_back`) now restores the position,
+the pending `>`s and the P0002 state as they were, drops the errors the
+attempt pushed after it crossed (which it read at the end of the file;
+those before stay, as the base kept them), and keeps the crossing waiting
+in `rolled_back`, with the token where it was made. If an error comes
+before the parser, reading the tokens the other way, reaches that token,
+the crossing stands, as if never rolled back: a parameter's type
+`Vec<Vec<…>>` past the limit, whose lists roll back one by one and leave
+`Vec` before a `<` that the parameter list refuses, is one P0002 at its
+deepest token, as before. Once the token is read, the crossing goes: the
+comparison above parses at 127 parentheses and is P0002 at its own
+deepest token at 128, and after two casts the attempt is two levels
+deeper, so 126 parse. Where the base's own attempt reported an error past
+the limit, as with `x as Int < (((1)))` at 127 parentheses (no type is
+`1`), that error goes with the crossing; where the other reading fails
+before the token, as `x as Int < (*mut (((a))))` does at `mut`, which no
+expression has, P0002 stands where the base reported P0001. The trait
+body's lookahead, the other rollback, re-reads its tokens only after a
+`{`, which a crossing hides (`peek` reads the end of the file), and at the
+same depth; the `component` and `view` lookaheads read tokens, not
+nesting. `depth.rs` and two cases hold each shape.
+
+**Measured.** Over the repository's tracked `.nova` files, N's new
+sources and the cases (588 files), but for the three cases written to sit
+at or past the limit (`chains_and_casts_at_the_limit.nova` and the two of
+the rollback), the deepest is 25 levels
+(`examples/07-fullstack-blog/src/server.nova`, whose router is one chain
+of 21 calls, `Server::new()` and 20 method calls, 41 postfix operators,
+the repository's longest); the next are 18 (`selfhost/src/lexer.nova`)
+and 14 (`tests/dom/ui_show_for_each.nova`, `tests/dom/timers.nova`), and
+N's parser nests at most 13 (`parse_pattern.nova`). Each is counted by
+building N with `MAX_DEPTH` lowered until the file meets P0002. The
+longest binary chain is 9 operators and the longest `else if` chain 14,
+both in `lexer.nova`. No file comes near 128.
+
+**Probed at the limit,** each by S0 and N, the dumps compared: 128 and
+129 levels of parentheses, blocks, `if`s, `else if`s, match arms,
+closures' bodies, interpolation holes (nested, and holding parentheses),
+calls, indexes, arrays, tuples, prefix operators, binary chains,
+assignments, ranges, casts, a cast's generic type, generic arguments in a
+parameter's and a `let`'s type, tuple, function and optional types,
+tuple-struct, tuple, record and or-patterns, items in blocks, `while`,
+`for`, `return`, record literals, views, and chains of fields, indexes,
+`?`, `.await`, tuple fields, calls and method calls: every one parses at
+128 and is one P0002, the file's last line, at 129; `nova check` runs
+every stage on each at 128 without overflowing, and `nova fmt` on each at
+129 reports the one P0002 and leaves the file as it is. Uncounted, a
+chain of about 5,000 method calls overflowed `nova check`, `run` and
+`build` on the 64 MB thread (4,000 passed), and the base binary on its
+8 MB main thread passed 500 and aborted at 600; counted, a chain of
+100,000 is P0002.
+Wide inputs need no count: 20,000 `let`s, statements, array elements,
+interpolations, match arms, items or tuple elements pass `nova check`.
+
+### 3. S0's pipeline on a 64 MB thread
+
+`nova`'s `main` parses its arguments, sets up logging and the embedded
+runtime, then runs the command on a thread with
+`nova_driver::PIPELINE_STACK`, 64 MB, named `main`:
+
+- the command's result returns to the main thread, which returns it as
+  before (an error prints `Error: …` and exits 1);
+- a panic prints on that thread, `thread 'main' … panicked at`, as on the
+  main thread before, and resumes on the main thread with
+  `resume_unwind`, which unwinds out of `main` without printing again:
+  exit code 101;
+- `std::process::exit` (`fmt`, `lsp`, `parse --dump-*`) ends the process
+  from either thread, flushing standard output as before; a stack
+  overflow aborts it from either, with its message and exit code 134;
+- stdin and stdout are the process's; no command installs a signal
+  handler, so Ctrl-C ends `nova` as before. Probed against the base
+  binary: every command's exit code and both streams on success and
+  failure; `nova run` of a program that reads stdin, exits with a code,
+  panics, or recurses past its stack (the same depth overflows both,
+  between 520,000 and 550,000 frames on 8 MB); `nova test` with a passing,
+  failing, panicking, exiting and overflowing test; `nova dev`, `nova run`
+  and `nova lsp` under SIGINT and SIGTERM: all as before;
+- **`nova run` compiles on that thread and runs the program on the main
+  thread.** The thread hands back the JIT's `CompiledProgram`, wrapped in
+  `cmd::run::Compiled` with an `unsafe impl Send`. Rust cannot prove it
+  `Send` because the JIT module holds raw pointers; it is sound because
+  the module owns the code and data those pointers name (mapped memory,
+  not thread-bound), the entry point is a pointer into that memory, the
+  thread that compiled it has ended, so it is used by one thread at a
+  time, and the runtime's per-thread state (the heap, the executor, the
+  file and socket tables) is made on first use by the thread that runs
+  the program: compiling calls nothing of the runtime but its symbol
+  table, and `set_args`, which the main thread calls, is process-wide. So
+  a program has the same stack under `nova run` as before, and as when
+  built: the platform's main thread's. Running it on the 64 MB thread
+  would have let a deep recursion pass under `nova run` and crash when
+  built; `nova_run_runs_the_program_on_the_main_thread` holds the main
+  thread at 8 MB and checks that a recursion 2,000,000 deep overflows it
+  where 100,000 runs.
+
+The language server's checker thread, and `nova dev`'s rebuild thread,
+get the same stack; its protocol thread is the command's.
+
+The thread bounds what the parser's recursion needs, since the depth
+limit bounds that recursion, but not every overflow: `logos`'s per-digit
+recursion (ADR 0051) is not a level, so a decimal literal of 210,000
+digits or more aborts debug S0's 64 MB thread (measured with `nova parse
+--dump-tokens` and `--dump-ast`: 210,000, 250,000 and 300,000 digits abort
+in about 50 ms, in the first scan of the literal, one frame a digit; a
+release build's smaller frames take more). Shorter literals do not
+overflow, but are slow: the literal fails as a number, and each digit is
+then scanned again with the digits after it, every scan shallower than
+the first, so the time is quadratic (20,000 digits take 7.6 s, 40,000
+take 42 s); 207,500 digits had not aborted after 60 s. No generator
+makes such a number. `nova test` runs built test binaries, whose stack
+is the platform's, as before.
+
+### 4. The error cases
+
+The spec's ~160 inline sources became 262 files in
+`crates/nova-cli/tests/selfhost/cases/`. They were not copied by hand: the
+five test binaries (`lexer_tests`, `keywords`, `parser_tests`,
+`newline_rule`, `tuples`) ran with `lex` and `lex_with_comments`
+temporarily recording each source and the name of the test thread that
+lexed it, so a source a helper builds (`fn f() {\n…\n}`) is recorded as it
+is parsed. Then: the property tests' random inputs were skipped; the test
+that lexes every `.nova` file of the tree was dropped; a source equal to a
+tracked file (the parser's six fixtures, already corpus files) was
+dropped; duplicates were kept once, under the first test in sorted order;
+the 32 keywords became one file, a line each. A file is named `<test
+file>__<test>[_<n>].nova`. The six sources holding a `\r` or another
+control character, which a checked-out text file cannot keep, joined the
+differential's edge inputs (`fuzz::edges`), as did three decimal numbers
+of 4,000 characters (ADR 0051's known bug). One more case,
+`every_other_node.nova`, holds the twenty kinds the base's corpus does not
+(twelve that no test's source holds either, and a module, an item in a
+block, an array pattern, a type alias and the splits before `[`, `-` and a
+value) and the three values no corpus file holds (a record pattern
+without `..`, a pointer type without `mut`, a trait's `async`
+signature), so that the corpus and that file alone name every form. A last,
+`chains_and_casts_at_the_limit.nova`, holds a chain of each binary
+precedence and a postfix chain, each followed by an operator of the next
+lower one whose operand is 100 parentheses deep, a view whose first
+child is a postfix chain and whose second's argument is 100 deep, and 128
+casts: a parser that does not lower the depth after a chain, or counts a
+cast's type one level deeper, reports P0002 there (the mutants found that
+no input did). Both lexers agree on all of them.
+
+### 5. The deep generator
+
+`fuzz::deep(rng, kind, depth)` writes a program whose deepest level is
+`depth`, nested as one of 14 kinds (parentheses, blocks, `if`, `else if`
+chains, `match`, `while`, `for`, calls, prefix operators, binary chains,
+chains of method calls, types, patterns, items in blocks) or mixed, eight
+wrappers of one level each in a random order. Up to the limit each is well
+typed (but nested items, which the checker refuses as E0900), so `nova
+check` runs every stage on it. `fuzz::deeps(seed, mixed)` gives each kind
+at 126 to 130 levels and `mixed` inputs at depths in that range: seeds 201
+and 202, 20 mixed each, 180 inputs, through the token and AST
+differentials (§7). S0's `parse --check` gives no error up to 128 and
+exactly one P0002 past it on all of seed 201's, and `check` the same at
+128 and 129, but for the nested items' E0900; neither overflows.
+
+### 6. N's AST and parser
+
+`selfhost/src/` gains nine modules:
+
+| Module | What it ports | Code lines | Rust's |
+|---|---|---:|---:|
+| `ast.nova` | `nova-ast`'s `lib`, `expr`, `item`, `pattern`, `ty` | 245 | 477 |
+| `parser.nova` | `grammar.rs`'s state, token access, recovery, docs, `component` lookahead, paths and helpers; `error.rs`; `lib.rs` | 420 | |
+| `parse_item.nova` | the file, attributes, items, functions, records … `extern` | 681 | |
+| `parse_type.nova` | types, generic arguments, `>>` | 151 | |
+| `parse_stmt.nova` | blocks and statements | 78 | |
+| `parse_expr.nova` | the precedence ladder, postfix, atoms, `if`/`match`, closures, record literals | 762 | |
+| `parse_view.nova` | `view { … }` | 134 | |
+| `parse_pattern.nova` | patterns | 259 | |
+| the parser, together | `grammar.rs`, `error.rs`, `lib.rs` | 2,485 | 2,948 |
+| `ast_dump.nova` | `nova-ast`'s `dump.rs` and `nova-parser`'s | 813 | 962 |
+| `parser_tests.nova` | N's own tests (six) | 145 | – |
+
+Code lines are counted as 5.1a counted them (no blank or comment lines,
+no `#[cfg(test)]` item); the Rust parser was 2,836 at the spec's base and
+is 2,948 after P0002 and this ADR's changes, the AST 476 and 477; the Rust
+dump's figure holds `KINDS`, which N has no use for. `token.nova` grows
+from 196 code lines to 303 (`Token.kind`, `Span::point`, `Span.merge`,
+`Spanned::new`), and `lexer.nova` from 793 to 827 (`KEYWORDS`).
+
+The modules follow `grammar.rs`'s areas, as the spec planned; Nova's flat
+modules allowed it as it is, since an `impl Parser` may sit in any module
+that imports `parser`, and the modules may import one another in a cycle.
+Each Rust function has a Nova function of the same name and shape, in the
+same order, with its comments; the one function added is `value_at` (the
+token at an index, or `Eof` past the end, where the Rust reads
+`self.tokens.get(i)` and every caller asks for a token other than
+`Eof`). Every message, context string and note is the Rust's word for
+word; so are the generic-arguments rollback that keeps its errors and
+holds back a crossing (§2), `>>`
+splitting through `pending_gt`, the statement recovery that ignores line
+breaks, the trailing expression with or without its `;`, the
+`component` and `view` lookahead, the trait body's lookahead that does
+truncate its errors, the newline rule's splits, the halt after a bare
+`return` or `break`, the note on the arm after a split, and the depth
+counter, entry for entry. Not ported: the names of dropped items, which
+only the language server reads (spec §4.2).
+
+**The tree.** A struct is a record with the same fields in the same order,
+an enum a sum with the same variants in the same order, a variant with
+named fields holding them positionally (a comment names them). Spans are
+`token.nova`'s `Span`, two `Int`s with no `FileId`. A sum is recursive
+without a box, so `Box<Spanned<Expr>>` is `Spanned<Expr>` and
+`Option<Box<…>>` is `Option<…>`. Two choices the spec did not make:
+
+- **A variant is named `EnumVariant`** (`Expr::If` is `ExprIf`,
+  `Pattern::Lit` is `PatternLit`, `BinOp::Add` is `BinOpAdd`). A variant's
+  name is a value of its module, one module may hold only one of a name
+  (E0002), and a module may not import two (E0002 again); the AST's Rust
+  names repeat across its enums (`Lit`, `Path`, `Tuple`, `Array`, `Range`,
+  `Ref`) and with the tokens' (`If`, `Match`, `Fn`, `Type`, `Pub`, `Eq`,
+  `Int`), which every parser module imports. The dump's `Enum.Variant` is
+  the name with a dot.
+- **`Spanned` is generic**, `Spanned<T>`, as nova-diagnostics' is: 5.1a's
+  `Spanned` held a `Token` alone, and the tree needs it for nodes and
+  names (`Spanned<Expr>`, `Spanned<String>`); the lexer's 17 uses became
+  `Spanned<Token>`.
+
+**Each Rust idiom's translation**, one way everywhere:
+
+| Rust | Nova |
+|---|---|
+| `std::mem::discriminant(a) == std::mem::discriminant(b)` in `check`, `eat`, `expect`, `next_after_docs_is` | `a.kind() == b.kind()`: `Token.kind()` numbers the 92 variants in declaration order; every kind the parser asks for has no payload |
+| `match self.peek().clone()`, `if let Token::X(v) = …` | a `match` on the token `peek` returns: a token is a reference, so nothing is copied |
+| `self.in_stmt_ctx(on, \|p\| …)`, `in_no_struct_ctx`, `in_head_ctx`, `in_struct_ok_ctx` (21 uses) | the flag saved by `enter_stmt_ctx(on)` / `enter_no_struct_ctx(on)`, which set it and return the old value, the call, then the flag restored; a `?` comes after the restore |
+| `self.nested(\|p\| …)` | the depth saved, `deeper()`, the call, the depth restored; a closure with more than one call in it is a function `…_here` (`parse_type_here`, `parse_pattern_here`, `parse_view_children_here`), and S0's are the same functions, so both read alike |
+| `static HALT: Token = Token::Eof` | the field `eof`, an `Eof` |
+| `&'a Tokens` | the field `tokens` |
+| `halt: Option<usize>` | `halt: Int`, -1 for none |
+| `self.tokens.get(self.pos.saturating_sub(1)).map(\|s\| s.span).unwrap_or(start)` (36 copies) | `self.prev_span_or(start)`, the Rust's own helper |
+| `matches!(t, A \| B)` | a `match` with an or-pattern |
+| `errors.truncate(n)`; `errors.remove(i)` then `insert(i, …)` | `pop` to `n`; `set(i, …)` |
+| `splits.sort_by_key(\|s\| s.at.start)` | a stable insertion sort (`sorted_by_start`) |
+| `let mut f = …` bound by a pattern | `let mut f = f`: a pattern's bindings are immutable |
+| `nova_lexer::KEYWORDS`, `trim_matches('`')` | `lexer.nova`'s `KEYWORDS`, `trim_backticks` |
+
+`peek`, `peek_span`, `advance` and `prev_span_or` index the tokens after a
+bounds check rather than through `Vec.get`, whose `Some` a Nova program
+allocates. Three functions are named apart from the Rust's because their
+names are Nova keywords: the dump's `record`, `import` and `module` are
+`record_decl`, `import_decl` and `module_decl`, and a local `record` is
+`rec`.
+
+**The command line.** `selfhost ast FILE…` prints, for each file, `== FILE`
+and its AST dump, `IOERR` and the kind for a file it cannot read and exit
+code 2 after the rest, and refuses no file with exit code 2, as `selfhost
+tokens` does (ADR 0051). `ast_dump.nova` writes the grammar of §1 byte for
+byte, through `dump.nova`'s `push_escaped` and `hex16`.
+
+**N's own tests.** Six `@test`s in `parser_tests.nova` (in `src/`, as
+5.1a's are, ADR 0051): the generic-arguments rollback and `>>`, statement
+recovery, the trailing expression, `component` and `view`, a bare
+`return` and the arm note, and 128 against 129 parentheses. Each expected
+dump is S0's. `nova test` in `selfhost/` runs 17.
+
+**Differences found while building.** None: on its first build N's dump
+equalled S0's on every tracked file and case, and no input of the fixed
+seeds or of the longer searches below has shown a difference. So no case
+was added.
+
+### 7. The comparison
+
+`selfhost.rs`'s `agree` runs every batch through `nova parse --dump-ast`
+and `selfhost ast` after the token dumps, so the AST dumps, errors and
+notes included, are compared byte for byte on: the corpus (every tracked
+`.nova` file outside `selfhost/`, then N's sources) and its Windows
+checkout; the 278 cases (5.1a's 12, the 262 recorded in §4,
+`every_other_node.nova`, `chains_and_casts_at_the_limit.nova` and the two
+of the rollback, §2); the
+edge inputs; the soup of seed 1 (2,000 inputs; seeds 2 and 3 go through
+the token dump alone); the 750 mutated corpus files of seeds 101–103; and
+the 180 deep inputs of seeds 201 and 202. The every-kind test reads N's
+dumps of the corpus and the cases as well as S0's, and each holds all 167
+forms of `KINDS`. A mismatch's report names the source line of an AST line
+too (a node's start, or a `PARSE` line's). A command with no file is
+refused by both sides, with nothing printed and exit code 2, for either
+dump. The corpus test counts one tree per file in S0's output, the soup's
+and the mutations' find trees in it, and the deep inputs' find P0002, so a
+pass that skips the AST dump fails.
+
+**Time.** On a 4-core Linux machine, debug build, `selfhost.rs` ran in
+about 10 s with the token dump alone; with the AST dump, the soup's pass
+cut to one seed of three, it takes 21.5–23.4 s (about 50 s of CPU), three
+runs, with the postfix kind, the hand case and the AST pass's assertions
+that step 0 added; all the soup through both would add about 1 s of CPU.
+In 5.1a CI took 1.4 to 2.1 times as long on macOS as here, so about 35 to
+50 s there, under the minute past which the spec asks the user. N's AST
+dump is the cost: with the debug runtime it parses the corpus in about 3 s
+where S0's debug build takes 0.4 s. The soup is made for the lexer and is
+mostly parse errors, so its AST pass was the one cut; the mutations, which
+reach far into the parser, all go through.
+
+**Longer searches**, locally: `NOVA_SELFHOST_FUZZ=7:6000` and
+`11:20000`, 26,000 inputs of soup and 26,000 mutated files through both
+dumps, showed no difference.
+
+### 8. N's own stack at the limit
+
+A Nova program runs on its platform's main thread (spec §4.4), so N, built
+by `nova build` (Cranelift), was run on the deep generator's 128-level
+input of every kind (and 126, 127, 129, 130) with `ulimit -s 8192` and
+`ulimit -s 1024`, which stands in for Windows' main thread:
+
+| Kind | 8 MB | 1 MB | Least stack that passed at 128 (of 512, 256, 128, 64, 32 KB) |
+|---|---|---|---|
+| parentheses, blocks, `if`, `match`, `while`, `for`, calls | passes | passes | 256 KB |
+| items in blocks | passes | passes | 128 KB |
+| `else if`, prefix operators, binary chains, method calls, types, patterns | passes | passes | 64 KB |
+
+So N has at least four times the stack it needs at 128 levels on 1 MB,
+measured on Linux, and past the limit it stops at 129 like S0. Windows
+x64 frames may be 1.5 to 2 times larger (the callee's shadow space, saved
+`xmm` registers), which would leave twice the stack needed; CI's Windows
+run of the test below, on that system's real 1 MB main thread, is the
+check. The limit stays 128 and the
+link is unchanged. `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`
+runs every kind at 128 and 129 through N on 8 MB and 1 MB on Linux and
+macOS, and on Windows' own 1 MB main thread, and checks that N exits 0,
+with no P0002 at 128 and one at 129.
+
+### 9. Speed
+
+Measured locally on 4 cores (spec §4.7), best of three, peak memory by
+`wait4`'s `ru_maxrss` (`/usr/bin/time` is not installed here). N is
+`selfhost ast`, built by `nova build` with `NOVA_RUNTIME_LIB` pointing at
+a release `libnova_runtime.a` built in an isolated target directory, and
+with the debug runtime. A release `nova` would have taken the disk below
+the 6.3 GB this work keeps free, so S0 was measured two ways: its debug
+`nova parse` (the `nova` binary), and its release lexer, parser and dump
+(`nova_parser::dump_ast`, or `lex` and `parse` alone for `--check`)
+built into a temporary example of `nova-parser` that reads the files and
+prints the same dump on a 64 MB thread, not kept. Inputs: all of `std/`
+(25 files, 697 KB) concatenated, the 25 files given to one process, and
+the 25 files one process each.
+
+| | std concatenated | | 25 files, one process | | 25 processes | |
+|---|---:|---:|---:|---:|---:|---:|
+| N, release runtime | 0.244 s | 46 MB | 0.149 s | 12 MB | 0.253 s | 11 MB |
+| N, debug runtime | 1.21 s | 47 MB | 0.89 s | 13 MB | 1.09 s | 12 MB |
+| S0 release, dump | 0.040 s | 18 MB | 0.027 s | 9 MB | 0.109 s | 9 MB |
+| S0 release, lex and parse | 0.030 s | 14 MB | 0.022 s | 9 MB | 0.101 s | 9 MB |
+| S0 debug, `parse --dump-ast` | 0.117 s | 29 MB | 0.108 s | 16 MB | 0.261 s | 16 MB |
+| S0 debug, `parse --check` | 0.081 s | 26 MB | – | – | 0.227 s | 16 MB |
+
+**The factor, N over S0, lexing, parsing and dumping, with the release
+runtime: 6.1× release S0 on std concatenated, 5.5× on the files in one
+process** (2.1× and 1.4× S0's debug build); with the debug runtime, 30×
+and 33× release S0. That is under the 10× past which the user is asked
+(user decision 6). N's peak memory is 2.6× release S0's on the one large
+file. N with the debug runtime is what CI runs.
+
+### 10. The grammar changes in both compilers
+
+From 5.1b on the grammar exists twice, as the tokens have since 5.1a
+(ADR 0051). A change to the parser or the tree, in what it accepts, what
+it builds, what it reports or where, changes `nova-parser`/`nova-ast` and
+`selfhost/` in one pull request, the dump on both sides with them; the
+differential fails the change that touches one alone, and an input that
+once showed a difference becomes a case under `cases/`.
+
+## Consequences
+
+- A program nested past 128 levels no longer compiles: P0002, where
+  debug S0 aborted from 240 nested parentheses, 200 nested `if`s, 800
+  operators or 600 method calls in a chain. Nothing in the repository but
+  the cases written to test the limit is past 25.
+- P0002 drops the whole item in progress, however much of it parsed, and
+  everything after it in the file: one error for a file nested too deeply,
+  and nothing else from it, the items before it kept.
+- A `nova` panic and its exit code are as they were; a test runs one
+  (`println!` into `/dev/full`, on Linux).
+- Each command costs one thread and 64 MB of address space, committed
+  only as used.
+- `nova run`'s program and the compiler are on different threads, which
+  needs one `unsafe impl Send`. If the JIT ever keeps thread-bound state,
+  that impl is the place to look.
+- A postfix chain counts too, by the user's choice of 2026-10-31, which
+  the spec does not list (§2): each postfix operator after a chain's
+  first is a level, a method call one, and a call's arguments are a level
+  deeper than the call. After a name, a chain of 130 field accesses, or
+  of 130 method calls without arguments, is P0002, and of 129 with
+  arguments. A router built as one chain from `Router::new()`, one
+  `.get("/", h)` a route, parses 127 routes and is P0002 at the 128th;
+  the longest chain in the repository is the router's in
+  `examples/07-fullstack-blog/src/server.nova`, 21 calls.
+- The error cases are 262 small files, regenerated by the recording rather
+  than kept in step with the tests by hand; a new test's source joins
+  them only when the recording is run again.
+- Every grammar change is made twice (§10), and checked on every system by
+  a harness that now takes about twice the CPU it took in 5.1a.
+- The AST's variants carry their enum's name in N and not in S0, so a
+  reviewer reads `ExprIf` against `Expr::If`.
+- N is about 6× slower than release S0 with a release runtime and 30×
+  with the debug one; the debug runtime is what the differential uses, so
+  its time grows with the inputs N parses.
+
+## References
+
+- `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+  §§1, 2, 4–9.
+- ADR 0051 (the token dump, the harness, and S0's known stack bug).
+- `nova-spec/11-PARSER.md` §§4 and 5, and `12-TYPESYSTEM.md` §8, their
+  notes of 2026-10-31.
+- `crates/nova-ast/src/dump.rs`; `crates/nova-parser/src/{grammar,dump,error,lib}.rs`
+  and `tests/depth.rs`; `crates/nova-cli/src/{main.rs,cmd/parse.rs,cmd/run.rs}`;
+  `crates/nova-cli/tests/selfhost.rs` and `tests/selfhost/`.
+- `selfhost/src/{ast,parser,parse_item,parse_type,parse_stmt,parse_expr,parse_view,parse_pattern,ast_dump,parser_tests}.nova`.
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index 865e49112d5a7e682c6ea685949dd640b9ee983f..a831fa1bd92c9e9ceb0bfc81ae6f60e9bd743133 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -380,6 +380,19 @@ The three questions below are answered by that spec: the AST dump is an
 indented S-expression with spans (§4.3); the fuzzer mutates corpus files
 token by token (§3.5); and the depth is capped at 128 in both parsers, with
 S0's pipeline on a 64 MB stack (§4.4).
+
+**Amended 2026-10-31 (branch `claude/loving-archimedes-frtf5q-51b`):**
+5.1b is built (ADR 0052). Two figures the plan's decisions wait on: N,
+built by `nova build` with a release runtime, lexes, parses and dumps all
+of std in 0.24 s against 0.04 s for release S0's parser, about 6×, under
+decision 8's 10×, and about 30× with the debug runtime CI uses; and at
+the depth limit N needs at most 256 KB of stack, so Windows' 1 MB main
+thread needs no larger link. The depth counts the outermost construct as
+level 0, so 128 nested parentheses parse and 129 are P0002; by the
+user's ruling of 2026-10-31 it counts postfix chains too, which the spec
+does not list, so a chain of 130 method calls is P0002 (129 with
+arguments, a call's arguments being a level deeper than the call).
+
 - **N's first modules:** the tokens, a hand-written scanner, the AST,
   recursive descent with error recovery, and the P-codes.
 - **The dumps:** a token dump on both sides (the spike's format), and a
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 9009339d9363b883f90b194cefa0115987fb3d5a..3b52b45488a6574809a16b651711fcdf7ca17832 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -773,6 +773,17 @@ repository and on fuzzed inputs, on all three systems. From here a change to
 the tokens changes both compilers in one pull request. Step 2, the parser
 and the AST, is 5.1b.
 
+**Amended 2026-10-31 (branch `claude/loving-archimedes-frtf5q-51b`):** step 2, the parser and the AST, is ported
+(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+§4; ADR 0052): `selfhost/` gains the tree and a parser that follows
+`nova-parser` function for function, and `selfhost ast FILE…` prints the
+AST dump S0's hidden `nova parse --dump-ast` prints, errors, notes and
+newline splits included; the differential compares the two wherever it
+compares the token dumps. Both parsers refuse source nested past 128
+levels with P0002, and S0 runs its pipeline on a 64 MB thread. N lexes and
+parses std about 6× as slowly as release S0 with a release runtime. From
+here a change to the grammar changes both compilers in one pull request.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/nova-spec/11-PARSER.md b/nova-spec/11-PARSER.md
index 8e9a88217e4e13f918fd7c99bc929a6327f1b4f0..1772ae121519b36acbdd5631e5c21df5e41cb679 100644
--- a/nova-spec/11-PARSER.md
+++ b/nova-spec/11-PARSER.md
@@ -476,6 +476,23 @@ prints it after a `== FILE` header per file, and the self-hosted lexer's
 promise: it changes with the tokens, in both compilers at once. 5.1b adds an
 AST dump beside it.
 
+**Amended 2026-10-31 (branch `claude/loving-archimedes-frtf5q-51b`):** **the AST dump**
+(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+§4.3; ADR 0052, which writes its grammar out with an example of every
+kind). `nova_ast::dump::dump_file` renders a tree as an indented
+S-expression, one node per line: `(Kind start..end values fields`, with
+its children on the lines after it and its `)` at the end of its last
+line. A struct's kind is its name and an enum value's `Enum.Variant`;
+scalar fields are `key=value`, names `"text"@start..end`, literals
+`Int:1`, `Float:` and the bits, `Str:"…"`, `Char:` and the code point,
+`Bool:true`; the newline splits close the tree. `nova_parser::dump_ast`
+adds `ERR`, `PARSE` and `NOTE` lines for the lex errors, the parse errors
+and their notes. The hidden `nova parse --dump-ast FILE…` prints it after a
+`== FILE` header per file, and the self-hosted parser's `selfhost ast
+FILE…` prints the same bytes, which the differential compares on every
+input; like the token dump, it changes with the tree, in both compilers
+at once.
+
 ---
 
 ## 5. Error Recovery
@@ -487,6 +504,43 @@ Chumsky's `recover_with` strategies — apply at these levels (most permissive f
 
 Every error → `ParseError` with span + expected/found info → diagnostics crate renders.
 
+**Amended 2026-10-31 (branch `claude/loving-archimedes-frtf5q-51b`): the depth limit, P0002**
+(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+§4.4; ADR 0052). The parser counts how deeply it is nested. A level is:
+an expression, a type or a pattern (so each operand, type argument and
+sub-pattern inside another); the operand of a prefix operator; each `else
+if`; each operator of a binary chain, assignment, range and `as` included,
+for the rest of its chain (the type after the `k`th `as` is at its
+operator's level); each postfix operator of a chain after its first (a
+call, an index, a field, `?`, `.await`; a method call `.f(…)` is one), for
+the rest of the chain; an item inside a block; and the children of a
+`view` or of an element. The outermost is level 0, so `n` nested constructs of one
+kind are `n` levels: 128 nested parentheses parse and 129 do not. A block
+is no level of its own; its statements' expressions are. Past
+`nova_parser::MAX_DEPTH`, 128 levels, the parser reports **P0002**, "this
+is nested too deeply (more than 128 levels)", at the token where the limit
+was crossed, and stops: from there it reads the end of the file, the
+items it finished before are kept, and every error it would report after
+is dropped, so P0002 is the file's last error. An attempt the parser
+rolls back (generic arguments after a type's `<`, which may be a
+comparison) does not decide alone: its crossing stands only if reading
+the tokens the other way fails before the token where it was made. Every
+other parse error is P0001: the parser reports none with a code, and its
+callers attach `ParseError::code()`. But for the test cases written to
+sit at the limit, the deepest file in the repository nests 25 levels (a
+router built as one chain of 21 calls), its longest binary chain is 9
+operators and its longest `else if` chain 14. The self-hosted parser
+counts the same levels and stops at the same token; built by `nova
+build`, it parses 128 levels of every kind on a 1 MB main thread with
+four times the stack it needs to spare, as measured on Linux; Windows
+x64 frames may be larger, and CI's run of that test on Windows' own 1 MB
+main thread is the check. S0 runs its pipeline on a thread with a 64 MB
+stack, so no nesting meets the platform's stack first; the lexer's
+per-digit recursion (ADR 0051) is no level, and a decimal literal of
+210,000 digits or more still aborts a debug build's thread (shorter ones
+do not, but take time quadratic in their length).
+Counting postfix chains is the user's ruling of 2026-10-31.
+
 ---
 
 ## 6. Operator Precedence Table
diff --git a/nova-spec/12-TYPESYSTEM.md b/nova-spec/12-TYPESYSTEM.md
index 5ea327d7c8e0a4a4cf9c39a27eda7820f1a3f97e..6c1be3dc0bcffa201a3bfc067f9bf35ec5de2478 100644
--- a/nova-spec/12-TYPESYSTEM.md
+++ b/nova-spec/12-TYPESYSTEM.md
@@ -280,6 +280,9 @@ types differ.
 | E0103 | This value is not used, or a line break ended a statement; a warning (added 2026-10-26, Phase 5.0a) |
 | E0104 | A pattern binds a name that starts with an upper-case letter, not a plain `let X` or `for X`; a warning (added 2026-10-28, Phase 5.0b) |
 | E0105 | One of std's inherent methods hides a program's trait method of the same name; a warning (added 2026-10-29, Phase 5.0c) |
+| L0001 | Any lexical error: an unexpected character, an unterminated string, interpolation or block comment, an invalid escape or numeric literal (row added 2026-10-31, Phase 5.1b; the code is older) |
+| P0001 | Any syntax error but P0002: expected one thing, found another; an unexpected end of file; chained comparisons (row added 2026-10-31, Phase 5.1b; the code is older) |
+| P0002 | This is nested too deeply (more than 128 levels): a parser code, beside P0001, any other syntax error, and L0001, any lexical error (added 2026-10-31, Phase 5.1b; `11-PARSER.md` §5) |
 
 Each gets a fully detailed error in `nova-diagnostics::messages` with example, suggestion, and link.
 
````

### Task 8: Verification, the mutants, the review, the PR, and the merge

Every step runs in a shell set up as the conventions say, the process
limit included.

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 6G
  free; if not, remove older duplicate test executables.

  ```sh
  cargo build --locked --workspace
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-51b.txt" 2>&1
  grep '^test result' "$P/full-51b.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-51b.txt"
  cargo test --locked --workspace -- --list > "$P/final-list-51b.txt" 2>&1
  python3 "$P/names.py" "$P/base-list-51b.txt" "$P/final-list-51b.txt"
  ```

  `names.py` compares Task 0's list with this one binary by binary, by
  name: a binary is its `Running` or `Doc-tests` line, its hash stripped,
  an integration test's prefixed with the crate whose unit tests cargo
  listed just before it, so two crates' `tests/tuples.rs` stay apart; a
  test is a line ending `: test`. It prints each list's binaries and
  tests, each binary added or removed with its tests, each binary whose
  tests changed with the names added and gone, and the net. Write it
  before the run, as the replay ran it:

```sh
cat > "$P/names.py" <<'EOF'
"""Compare two `cargo test --workspace -- --list` outputs binary by binary:
a binary is its `Running`/`Doc-tests` line, hash stripped; a test is a line
`NAME: test`. An integration test binary is named after the crate whose
unit tests cargo listed last before it, so two crates' `tests/tuples.rs`
stay apart. Prints each binary's count, and every name added or removed."""
import collections
import re
import sys


def tests(path):
    out = collections.OrderedDict()
    binary = None
    crate = ["?"]

    def binary_name(m, d):
        if d:
            return f"doc-tests {d.group(1)}"
        if m.group(1).startswith("unittests"):
            crate[0] = m.group(2)
            return f"{m.group(2)} ({m.group(1)})"
        return f"{crate[0]}: {m.group(1)} ({m.group(2)})"

    for line in open(path, encoding="utf-8", errors="replace"):
        m = re.match(r"\s+Running (.+?) \(target/debug/deps/(.+)-[0-9a-f]{16}\)", line)
        d = re.match(r"\s+Doc-tests (\S+)", line)
        if m or d:
            binary = binary_name(m, d)
            out.setdefault(binary, [])
            continue
        t = re.match(r"(.+): (test|bench)$", line.rstrip("\n"))
        if t and binary is not None:
            out[binary].append(t.group(1))
    return out


a, b = tests(sys.argv[1]), tests(sys.argv[2])
print(f"binaries: base {len(a)}, now {len(b)}")
print(f"tests: base {sum(map(len, a.values()))}, now {sum(map(len, b.values()))}")
added = 0
for k in list(a) + [k for k in b if k not in a]:
    if k not in a:
        print(f"binary added: {k}, {len(b[k])} tests")
        for t in b[k]:
            print(f"  + {t}")
        added += len(b[k])
        continue
    if k not in b:
        print(f"binary removed: {k}")
        continue
    ca, cb = collections.Counter(a[k]), collections.Counter(b[k])
    plus = sorted((cb - ca).elements())
    minus = sorted((ca - cb).elements())
    if plus or minus:
        print(f"{k}: {len(a[k])} -> {len(b[k])}, +{len(plus)} -{len(minus)}")
        for t in plus:
            print(f"  + {t}")
        for t in minus:
            print(f"  - {t}")
        added += len(plus) - len(minus)
print(f"net added: {added}")
EOF
```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with no
    warning or error;
  - the suite, in 6 min 5 s: `2839 1 10`, the baseline's figures and
    23 more tests;
  - the one failure is Task 0's IPv6 test;
  - `names.py` prints `binaries: base 111, now 112` and
    `tests: base 2827, now 2850`: 2827 tests in 111 binaries before and 2850 in 112 after, one binary new, and
    `net added: 23`, 23 added and 0 gone:

    - `nova_ast (unittests src/lib.rs)`, 0 to 4:
      `dump::tests::a_tree_with_every_kind_of_field`;
      `dump::tests::every_declared_form_is_listed`;
      `dump::tests::the_kinds_are_well_formed_and_distinct`;
      `dump::tests::the_scanner_reads_the_declarations`;
    - `nova: tests/fmt.rs (fmt)`, 16 to 17:
      `a_file_nested_too_deeply_is_one_p0002_and_left_untouched`;
    - `nova: tests/lsp.rs (lsp)`, 37 to 38:
      `a_file_nested_too_deeply_gets_one_p0002`;
    - `nova: tests/selfhost.rs (selfhost)`, 19 to 27:
      `a_long_decimal_number_lexes_with_a_1_mb_main_thread`;
      `a_panic_on_the_pipelines_thread_exits_101_with_its_message`;
      `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`;
      `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`;
      `nova_run_runs_the_program_on_the_main_thread`;
      `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`;
      `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`;
      `the_deep_inputs_agree`;
    - `nova_parser (unittests src/lib.rs)`, 0 to 2:
      `dump::tests::a_file_nested_too_deeply_is_one_p0002`;
      `dump::tests::the_errors_follow_the_tree`;
    - `nova_parser: tests/depth.rs (depth)`, new, 7 tests:
      `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`;
      `errors_before_the_limit_stay_and_there_is_one_p0002`;
      `every_kind_of_nesting_parses_at_the_limit`;
      `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`;
      `far_past_the_limit_it_stops_without_overflowing`;
      `the_depth_goes_back_down_after_each_sibling`;
      `view_children_are_a_level`.
- [ ] **Step 2: CI's Linux leg, as CI runs it.** The `test` job's
  `ubuntu-24.04` leg runs the whole suite with the golden suite required,
  with CI's own flags, `--locked --workspace --all-features --no-fail-fast`:

  ```sh
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast > "$P/ci-51b.txt" 2>&1
  grep '^test result' "$P/ci-51b.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/ci-51b.txt"
  grep -c 'release_golden_suite ... ok' "$P/ci-51b.txt"
  ```

  Expected, in 6 min 40 s: `2839 1 10`, Step 1's figures, the one failure the
  IPv6 test, and `1`: the golden suite ran and passed inside the whole run,
  as on CI, where Step 1's run skips it.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in
  36 s of wall time:
  `release golden: 147 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)` and `1 passed`: unchanged, since 5.1b adds no runtime
  program.
- [ ] **Step 4: Parsing and checking every file the base tracks.** Two
  scripts, as the replay ran them, compare the base's `nova` (Task 0's
  copy, `$P/nova-base-51b`) with the tree's on every `.nova` file
  `git ls-tree -r --name-only 4f32f13d` lists, 312 files, each
  run's output, both streams, and exit status compared byte for byte.
  None nests past 128 levels (decision 18), so P0002 changes nothing
  here. Both run from the repository's root after the build, and read
  `$P`.
  - `parsecmp.sh`: `nova parse --check` and `nova parse --ast` of each
    file, the base's on the file as `4f32f13d` has it, from a
    `git archive 4f32f13d` unpacked in `$P/pc-src-51b`, the tree's on the
    tree's, each passed the path relative to its root:

```sh
cat > "$P/parsecmp.sh" <<'EOF'
#!/bin/bash
# parsecmp.sh: run from the repository's root, after the build, with $P set.
# For every `.nova` file `git ls-tree -r --name-only 4f32f13d` lists (the
# base's tracked files, so none 5.1b adds), `nova parse --check` and
# `nova parse --ast`, by the base's `nova` ($P/nova-base-51b, Task 0) and
# by the tree's `target/debug/nova`, both on the file as 4f32f13d has it,
# from a `git archive 4f32f13d` unpacked in $P/pc-src-51b: 5.1b changes
# four of N's sources, which the base's text keeps out of the comparison.
# Each run passes the path relative to that root and keeps its output, both
# streams, and its exit status; every pair must be byte-identical, since
# no file the base tracks nests past 128 levels, where 5.1b's P0002 begins.
# Prints each file that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
src="$P/pc-src-51b"
rm -rf "$src"; mkdir -p "$src"
git archive 4f32f13d | tar -x -C "$src"
n=0; differ=0; nz_base=0; nz_new=0
while read -r f; do
  for mode in --check --ast; do
    a=$( (cd "$src" && "$P/nova-base-51b" parse $mode "$f") 2>&1; echo "exit=$?")
    b=$( (cd "$src" && "$nova" parse $mode "$f") 2>&1; echo "exit=$?")
    n=$((n + 1))
    if [ "$mode" = --check ]; then
      [ "$(tail -1 <<< "$a")" != exit=0 ] && nz_base=$((nz_base + 1))
      [ "$(tail -1 <<< "$b")" != exit=0 ] && nz_new=$((nz_new + 1))
    fi
    if [ "$a" != "$b" ]; then differ=$((differ + 1)); echo "differs: $mode $f"; fi
  done
done < <(git ls-tree -r --name-only 4f32f13d | grep '\.nova$')
echo "parse: $n runs over $((n / 2)) files, $differ differ; --check exits nonzero on $nz_base at the base, $nz_new now"
rm -rf "$src"
[ "$differ" = 0 ]
EOF
```

  - `checkcmp.sh`: `nova check` by both, on the tree's files. A file's
    package root is the nearest directory above it, in the tree, that
    holds a `nova.toml`, below the repository's root: each package is
    checked once, by `nova check` run in its root, and every other file
    alone, by `nova check FILE` from the repository's root:

```sh
cat > "$P/checkcmp.sh" <<'EOF'
#!/bin/bash
# checkcmp.sh: run from the repository's root, after the build, with $P set.
# `nova check` by the base's `nova` ($P/nova-base-51b, Task 0) and by the
# tree's `target/debug/nova`, both on the tree's files, for every `.nova`
# file `git ls-tree -r --name-only 4f32f13d` lists. A file's package root
# is the nearest directory above it, in the tree, that holds a
# `nova.toml`, stopping below the repository's root: a file under one is
# checked as its package, once per package, by `nova check` run in the
# root with no argument; any other file alone, by `nova check FILE` from
# the repository's root. Each check keeps both streams and the exit
# status, and the pair must be byte-identical. Prints each that differs,
# with both sides' diagnostic codes, then the counts.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
n=0; differ=0; stddiff=0
declare -A seen
codes() { grep -oE '^(error|warning)\[[A-Z0-9]+\]: .*' <<< "$1" | sort | uniq -c; }
while read -r f; do
  dir=$(dirname "$f"); pkg=""
  while [ "$dir" != "." ] && [ "$dir" != "/" ]; do
    if [ -f "$dir/nova.toml" ]; then pkg=$dir; break; fi
    dir=$(dirname "$dir")
  done
  if [ -n "$pkg" ]; then
    [ -n "${seen[$pkg]:-}" ] && continue
    seen[$pkg]=1
    a=$( (cd "$pkg" && "$P/nova-base-51b" check) 2>&1; echo "exit=$?")
    b=$( (cd "$pkg" && "$nova" check) 2>&1; echo "exit=$?")
    what="$pkg/ (package)"
  else
    a=$("$P/nova-base-51b" check "$f" 2>&1; echo "exit=$?")
    b=$("$nova" check "$f" 2>&1; echo "exit=$?")
    what=$f
  fi
  n=$((n + 1))
  if [ "$a" != "$b" ]; then
    case $what in std/*) stddiff=$((stddiff + 1));; *) differ=$((differ + 1));; esac
    echo "differs: $what"
    echo "  base: $(codes "$a" | tr -s ' ' | tr '\n' ';') $(tail -1 <<< "$a")"
    echo "  now:  $(codes "$b" | tr -s ' ' | tr '\n' ';') $(tail -1 <<< "$b")"
  fi
done < <(git ls-tree -r --name-only 4f32f13d | grep '\.nova$')
echo "check: $n checks, $differ outside std differ, $stddiff std files differ"
[ "$differ" = 0 ] && [ "$stddiff" = 0 ]
EOF
```

  - Then:

    ```sh
    P="$P" bash "$P/parsecmp.sh"
    P="$P" bash "$P/checkcmp.sh"
    ```

    Expected: `parse: 624 runs over 312 files, 0 differ; --check exits nonzero on 14 at the base, 14 now`: no file the base tracks changes, and none
    nests past the limit; the two that exit nonzero are the fixtures that do not
    parse, `crates/nova-parser/tests/fixtures/async.nova` and
    `tools/vscode-nova/test/fixture/src/complete.nova`. Then
    `check: 274 checks, 0 outside std differ, 0 std files differ`. Both exit 0.
- [ ] **Step 5: The hidden flags stay hidden.**
  - `helpcmp.sh` runs `--help` of `nova` and of each of its 21 commands,
    `help` included, by the base's `nova` and the tree's, both through a
    link named `nova`, and checks that `nova parse --help` names neither
    hidden flag nor `MORE` (decisions 16 and 40):

```sh
cat > "$P/helpcmp.sh" <<'EOF'
#!/bin/bash
# helpcmp.sh: run from the repository's root, after the build, with $P set.
# `--help` of `nova` and of every command, by the base's `nova`
# ($P/nova-base-51b, Task 0) and the tree's `target/debug/nova`, both run
# through a link named `nova` so the usage lines agree: both streams and the
# exit status must be byte-identical, the hidden flags and `more` absent.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
d="$P/help-51b"
rm -rf "$d"; mkdir -p "$d/base" "$d/tree"
ln -s "$P/nova-base-51b" "$d/base/nova"
ln -s "$PWD/target/debug/nova" "$d/tree/nova"
n=0; same=0
for cmd in "" parse run build bundle dev check test fmt doc lsp new init add remove fetch update login package publish version help; do
  a=$("$d/base/nova" $cmd --help 2>&1; echo "exit=$?")
  b=$("$d/tree/nova" $cmd --help 2>&1; echo "exit=$?")
  n=$((n + 1))
  if [ "$a" == "$b" ]; then same=$((same + 1)); else echo "differs: nova $cmd --help"; fi
done
hidden=$("$d/tree/nova" parse --help | grep -c 'dump-\|MORE')
echo "help: $same of $n identical; hidden flags shown: $hidden"
rm -rf "$d"
[ "$same" = "$n" ] && [ "$hidden" = 0 ]
EOF
```

  - The second file's error, as 5.1a checked it, from the base's `nova`
    and the tree's, each run with the same first argument: `othernova`, a
    renamed binary, and four with no file name in UTF-8, `""`, `/`, `..`
    and the bytes ff fe; then, on the tree alone, `--dump-ast`'s errors
    (its conflicts, no file, an empty file, a value given to it) under
    the same five, which must be `--dump-tokens`'s with the flag's name
    changed. `argv0.py` runs a program by `execv` with the first argument
    it is given, empty included, which Python's `os.execv` refuses;
    `usagecmp.sh` runs them, and two of 5.1a's in a terminal, through
    `script`'s pseudo-terminal, colours included:

```sh
cat > "$P/argv0.py" <<'EOF'
"""argv0.py ARGV0 PROGRAM ARG...: run PROGRAM, by execv, with ARGV0 as its
first argument and the ARGs after it. ARGV0 is written with Python's
escapes, so '\xff\xfe' is those two bytes, and may be empty, which
os.execv refuses: libc's execv is called through ctypes."""
import ctypes
import sys

arg0 = sys.argv[1].encode().decode("unicode_escape").encode("latin-1")
argv = [arg0] + [a.encode() for a in sys.argv[3:]]
array = (ctypes.c_char_p * (len(argv) + 1))(*argv, None)
libc = ctypes.CDLL(None, use_errno=True)
libc.execv(sys.argv[2].encode(), array)
sys.exit(f"execv failed: errno {ctypes.get_errno()}")
EOF
```

```sh
cat > "$P/usagecmp.sh" <<'EOF'
#!/bin/bash
# usagecmp.sh: run from the repository's root, after the build, with $P set
# and $P/argv0.py written. The error for a second file without the flag,
# from the base's `nova` ($P/nova-base-51b, Task 0) and the tree's
# `target/debug/nova`, each run by execv with the same first argument:
# `othernova` (a renamed binary), and four with no file name in UTF-8,
# "", "/", ".." and the bytes ff fe. Both streams and the exit status must
# be byte-identical. Then the same in a terminal, colours included, through
# `script`'s pseudo-terminal with NO_COLOR unset. Then `--dump-ast`, which
# the base lacks, against the tree's own `--dump-tokens`, with each first
# argument: its conflicts, no file, an empty file and a value given to it
# must print what `--dump-tokens` prints with the flag's name changed.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
runs=0; same=0
for arg0 in othernova "" / .. '\xff\xfe'; do
  for args in "parse a b" "parse --ast a b" "parse --check a b" "parse -- a b"; do
    python3 -I "$P/argv0.py" "$arg0" "$P/nova-base-51b" $args > "$P/usage-base.txt" 2>&1
    echo "exit $?" >> "$P/usage-base.txt"
    python3 -I "$P/argv0.py" "$arg0" "$nova" $args > "$P/usage-tree.txt" 2>&1
    echo "exit $?" >> "$P/usage-tree.txt"
    runs=$((runs + 1))
    if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then same=$((same + 1)); else
      echo "differs: [$arg0] $args"; diff "$P/usage-base.txt" "$P/usage-tree.txt"; fi
  done
  echo "[$arg0] $(grep -a '^Usage:' "$P/usage-tree.txt"), $(tail -1 "$P/usage-tree.txt")"
done
tty=0
for args in "parse a b" "parse -- a b"; do
  env -u NO_COLOR script -qc "python3 -I '$P/argv0.py' othernova '$P/nova-base-51b' $args" /dev/null > "$P/usage-base.txt"
  env -u NO_COLOR script -qc "python3 -I '$P/argv0.py' othernova '$nova' $args" /dev/null > "$P/usage-tree.txt"
  if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then tty=$((tty + 1)); else echo "differs in a terminal: $args"; fi
done
grep -qa $'\e\\[' "$P/usage-tree.txt" || echo "no colour in the terminal's output"
asts=0; astsame=0
for arg0 in othernova "" / .. '\xff\xfe'; do
  for args in "parse --dump-ast --ast a b" "parse --dump-ast --check a" "parse --dump-ast" "parse --dump-ast ''" "parse --dump-ast=1 a"; do
    tok=${args//dump-ast/dump-tokens}
    eval "python3 -I '$P/argv0.py' '$arg0' '$nova' $args" > "$P/usage-base.txt" 2>&1
    echo "exit $?" >> "$P/usage-base.txt"
    eval "python3 -I '$P/argv0.py' '$arg0' '$nova' $tok" 2>&1 | sed 's/dump-tokens/dump-ast/g' > "$P/usage-tree.txt"
    echo "exit ${PIPESTATUS[0]}" >> "$P/usage-tree.txt"
    asts=$((asts + 1))
    if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then astsame=$((astsame + 1)); else
      echo "--dump-ast differs: [$arg0] $args"; diff "$P/usage-base.txt" "$P/usage-tree.txt"; fi
  done
done
echo "usage: $same of $runs runs identical; in a terminal, $tty of 2; --dump-ast as --dump-tokens: $astsame of $asts"
rm -f "$P/usage-base.txt" "$P/usage-tree.txt"
[ "$same" = "$runs" ] && [ "$tty" = 2 ] && [ "$astsame" = "$asts" ]
EOF
```

    ```sh
    P="$P" bash "$P/helpcmp.sh"
    P="$P" bash "$P/usagecmp.sh"
    ```

    Expected: `help: 22 of 22 identical; hidden flags shown: 0`, then

    ```
[othernova] Usage: othernova parse [OPTIONS] <FILE>, exit 2
[] Usage: parse [OPTIONS] <FILE>, exit 2
[/] Usage: parse [OPTIONS] <FILE>, exit 2
[..] Usage: parse [OPTIONS] <FILE>, exit 2
[\xff\xfe] Usage: parse [OPTIONS] <FILE>, exit 2
usage: 20 of 20 runs identical; in a terminal, 2 of 2; --dump-ast as --dump-tokens: 25 of 25
    ```

    and both exit 0: each error names `othernova` or `parse` alone in its
    usage line and exits 2, as the base's clap does.
- [ ] **Step 6: The lockfile and Rust 1.78.**
  - `git diff --numstat 4f32f13d -- Cargo.lock`. Expected: nothing.
  - Then:

    ```sh
    RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-51b cargo check --locked --workspace
    rm -rf /dev/shm/msrv-51b
    ```

    Expected: clean, `Finished` in 42.28 s. It checks the libraries
    and binaries, as CI's MSRV job does, not the tests: `cargo check
    --workspace` compiles no test target, so the tests' Rust meets 1.78 by
    review alone. Its target
    directory is in `/dev/shm`, memory, so the disk stays above 6G.
- [ ] **Step 7: The differential's time, and N's tests.** `SH --no-run`,
  then `SH` three times, each timed, then
  `(cd selfhost && ../target/debug/nova test) | tail -n 1`. Expected:
  `27 passed; 0 failed` each time, in 24.7 s, 25.3 s and 24.4 s of wall time, building N
  included (decision 43); then `test result: ok. 17 passed; 0 failed; 0 trapped; 17 total`. Record them in the ledger
  beside CI's, once the PR's run gives each system's.
- [ ] **Step 8: The mutants.** One at a time, each edit is made, its
  command run, and the file restored with `git checkout -- <file>`, all on
  the last commit, in a shell under the process limit.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - Each command starts with `cargo build --locked --workspace`. A mutant
    of N needs no Rust rebuilt: `selfhost.rs` builds N from `selfhost/`
    each run.
  - The replay's run took 45 min 38 s in all; a mutant of S0 about
    40 seconds, one of N about 21.
  - Each caught mutant must fail as written: the tests named.
    Four mutants pass by design (mutants
    24, 25, 36
    and 19, decision 49): each must pass,
    and is named in the PR. Any other mutant that passes is a gap: record
    it in the ledger, name it in the PR, and give it to the review (Step
    10), which says whether a check is added.
  - At the end, `git status --short` prints nothing, and
    `cargo build --locked --workspace` rebuilds the last commit's binaries
    (decision 51).

  1. **The limit one lower.** `crates/nova-parser/src/grammar.rs`:
     `if self.depth <= MAX_DEPTH + 1 {` becomes `if self.depth <= MAX_DEPTH {`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
     - Expected: fails: `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `every_kind_of_nesting_parses_at_the_limit`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`, `the_fixed_cases_agree`, `view_children_are_a_level`.
  2. **The limit one higher.** `crates/nova-parser/src/grammar.rs`:
     `if self.depth <= MAX_DEPTH + 1 {` becomes `if self.depth <= MAX_DEPTH + 2 {`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
     - Expected: fails: `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `errors_before_the_limit_stay_and_there_is_one_p0002`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`, `the_fixed_cases_agree`, `view_children_are_a_level`.
  3. **An expression is no level.** `crates/nova-parser/src/grammar.rs`:
     `let expr = self.nested(|p| p.parse_assign(ctx));` becomes `let expr = self.parse_assign(ctx);`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
     - Expected: fails: `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `errors_before_the_limit_stay_and_there_is_one_p0002`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`, `the_fixed_cases_agree`, `view_children_are_a_level`.
  4. **A type is no level.** `crates/nova-parser/src/grammar.rs`:
     `self.nested(|p| p.parse_type_here(ctx))` becomes `self.parse_type_here(ctx)`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
     - Expected: fails: `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`, `the_fixed_cases_agree`.
  5. **A pattern is no level.** `crates/nova-parser/src/grammar.rs`:
     `self.nested(|p| p.parse_pattern_here(ctx))` becomes `self.parse_pattern_here(ctx)`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
     - Expected: fails: `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`.
  6. **A prefix operator's operand is no level.** `crates/nova-parser/src/grammar.rs`:
     `let expr = self.nested(|p| p.parse_unary(ctx))?;` becomes `let expr = self.parse_unary(ctx)?;`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
     - Expected: fails: `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`.
  7. **An `else if` is no level.** `crates/nova-parser/src/grammar.rs`:
     `let e = self.nested(|p| p.parse_if_expr(ctx))?;` becomes `let e = self.parse_if_expr(ctx)?;`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
     - Expected: fails: `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`.
  8. **An item in a block is no level.** `crates/nova-parser/src/grammar.rs`:
     `let item = self.nested(|p| p.in_stmt_ctx(false, |p| p.try_parse_item()))?;` becomes `let item = self.in_stmt_ctx(false, |p| p.try_parse_item())?;`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
     - Expected: fails: `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`.
  9. **A view's children are no level.** `crates/nova-parser/src/grammar.rs`:
     `self.nested(|p| p.parse_view_children_here())` becomes `self.parse_view_children_here()`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
     - Expected: fails: `view_children_are_a_level`.
  10. **`+` and `-` are no level.** `crates/nova-parser/src/grammar.rs`:
      replace

      ```
                  if !self.deeper() {
                      return None;
                  }
                  let rhs = self.parse_mul(ctx)?;
      ```

      with

      ```
                  let rhs = self.parse_mul(ctx)?;
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `far_past_the_limit_it_stops_without_overflowing`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`.
  11. **`||`'s chain keeps its depth.** `crates/nova-parser/src/grammar.rs`:
      replace

      ```
                  );
              }
              self.depth = depth;
              Some(lhs)
          }

          fn parse_and(
      ```

      with

      ```
                  );
              }
              Some(lhs)
          }

          fn parse_and(
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `the_depth_goes_back_down_after_each_sibling`, `the_fixed_cases_agree`.
  12. **A cast's type one level deeper.** `crates/nova-parser/src/grammar.rs`:
      `let ty = self.parse_type_here(ctx)?;` becomes `let ty = self.parse_type(ctx)?;`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `every_kind_of_nesting_parses_at_the_limit`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `the_fixed_cases_agree`.
  13. **A postfix chain's first operator is a level.** `crates/nova-parser/src/grammar.rs`:
      `if !first && !method_call && !self.deeper() {` becomes `if !method_call && !self.deeper() {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `every_kind_of_nesting_parses_at_the_limit`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`.
  14. **A method call is two operators.** `crates/nova-parser/src/grammar.rs`:
      `let method_call = after_dot && matches!(op, Token::LParen);` becomes `let method_call = false;`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `every_kind_of_nesting_parses_at_the_limit`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`, `the_depth_goes_back_down_after_each_sibling`.
  15. **A postfix chain is not counted.** `crates/nova-parser/src/grammar.rs`:
      `if !first && !method_call && !self.deeper() {` becomes `if false && !self.deeper() {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `far_past_the_limit_it_stops_without_overflowing`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`.
  16. **A postfix chain keeps its depth.** `crates/nova-parser/src/grammar.rs`:
      replace

      ```
              let expr = self.parse_postfix_chain(start, expr, ctx);
              self.depth = depth;
      ```

      with

      ```
              let expr = self.parse_postfix_chain(start, expr, ctx);
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `the_depth_goes_back_down_after_each_sibling`, `the_fixed_cases_agree`.
  17. **The parser goes on past P0002.** `crates/nova-parser/src/grammar.rs`:
      `if self.halted() || self.too_deep.is_some() || self.crossing_stands().is_some() {` becomes `if self.halted() {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `errors_before_the_limit_stay_and_there_is_one_p0002`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `the_deep_inputs_agree`, `the_fixed_cases_agree`.
  18. **The errors after P0002 stay.** `crates/nova-parser/src/grammar.rs`:
      replace

      ```
                  self.errors.truncate(errors);
                  self.errors.push(ParseError::TooDeep { span });
      ```

      with

      ```
                  self.errors.push(ParseError::TooDeep { span });
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `errors_before_the_limit_stay_and_there_is_one_p0002`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `far_past_the_limit_it_stops_without_overflowing`, `the_deep_inputs_agree`, `the_fixed_cases_agree`, `view_children_are_a_level`.
  19. **P0002 at the last crossing, not the first.** `crates/nova-parser/src/grammar.rs`:
      replace

      ```
              if self.too_deep.is_none() {
                  let here = (self.errors.len(), self.peek_span());
                  self.too_deep = Some(self.crossing_stands().unwrap_or(here));
              }
      ```

      with

      ```
              let here = (self.errors.len(), self.peek_span());
              self.too_deep = Some(self.crossing_stands().unwrap_or(here));
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: passes, accepted: equivalent: after the first crossing every token reads as the end of the file, and the parser unwinds without entering a level again, so nothing crosses twice; the guard keeps the first crossing should a path ever do so (decision 49).
  20. **P0002's code is P0001.** `crates/nova-parser/src/error.rs`:
      `ParseError::TooDeep { .. } => "P0002",` becomes `ParseError::TooDeep { .. } => "P0001",`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth`, `cargo test --locked --workspace --test fmt -- a_file_nested_too_deeply` and `SH`, each to its end.
      - Expected: fails: `a_file_nested_too_deeply_is_one_p0002_and_left_untouched`, `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `errors_before_the_limit_stay_and_there_is_one_p0002`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `far_past_the_limit_it_stops_without_overflowing`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `view_children_are_a_level`.
  21. **The driver's loader attaches P0001.** `crates/nova-driver/src/program.rs`:
      `Diagnostic::error(e.code(), e.to_string())` becomes `Diagnostic::error("P0001", e.to_string())`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`.
  22. **`nova fmt` attaches P0001.** `crates/nova-fmt/src/source.rs`:
      `Diagnostic::error(e.code(), e.to_string())` becomes `Diagnostic::error("P0001", e.to_string())`.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --test fmt -- a_file_nested_too_deeply`, each to its end.
      - Expected: fails: `a_file_nested_too_deeply_is_one_p0002_and_left_untouched`.
  23. **`nova parse` attaches P0001.** `crates/nova-cli/src/cmd/parse.rs`:
      `nova_diagnostics::Diagnostic::error(e.code(), e.to_string())` becomes `nova_diagnostics::Diagnostic::error("P0001", e.to_string())`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`.
  24. **Std's loader attaches P0001.** `crates/nova-resolver/src/lib.rs`:
      `Diagnostic::error(e.code(), e.to_string())` becomes `Diagnostic::error("P0001", e.to_string())`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: std's modules nest at most 13 levels, so its loader never meets P0002 (decision 49).
  25. **Organize-imports takes P0002 as no syntax error.** `crates/nova-lsp/src/organize.rs`:
      `matches!(d.code.as_str(), "L0001" | "P0001" | "P0002")` becomes `matches!(d.code.as_str(), "L0001" | "P0001")`.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --test lsp`, each to its end.
      - Expected: passes, accepted: a P0002 in the file being organized has its label there, which refuses already; only one in another module of the project needs the code, and no test builds such a project (decision 49).
  26. **The generic-arguments rollback keeps the crossing.** `crates/nova-parser/src/grammar.rs`:
      `self.too_deep = too_deep;`: the line goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `the_fixed_cases_agree`.
  27. **The rollback drops the crossing, never to stand.** `crates/nova-parser/src/grammar.rs`:
      `self.rolled_back = Some((errors, span, self.pos));`: the line goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `every_kind_of_nesting_past_the_limit_is_one_p0002_at_its_deepest_token`, `far_past_the_limit_it_stops_without_overflowing`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`, `the_deep_inputs_agree`, `the_fixed_cases_agree`.
  28. **The rollback keeps the errors pushed after the crossing.** `crates/nova-parser/src/grammar.rs`:
      replace

      ```
                  self.errors.truncate(errors);
                  self.rolled_back = Some((errors, span, self.pos));
      ```

      with

      ```
                  self.rolled_back = Some((errors, span, self.pos));
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `the_fixed_cases_agree`.
  29. **A rolled-back crossing waits past its token.** `crates/nova-parser/src/grammar.rs`:
      replace

      ```
                  } else if self.pos >= at {
                      self.rolled_back = None;
                  }
      ```

      with

      ```
                  }
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test depth` and `SH`, each to its end.
      - Expected: fails: `a_rolled_back_crossing_stands_only_if_the_tokens_fail_the_other_way`, `the_fixed_cases_agree`.
  30. **The pipeline's stack is 2 MB.** `crates/nova-driver/src/lib.rs`:
      `pub const PIPELINE_STACK: usize = 64 << 20;` becomes `pub const PIPELINE_STACK: usize = 2 << 20;`.
      - Run `cargo build --locked --workspace`, `SH a_panic_on_the_pipelines_thread a_long_decimal_number nova_run_runs_the_program s0_meets_the_depth_limit` and `cargo test --locked --workspace --test lsp -- a_file_nested_too_deeply`, each to its end.
      - Expected: fails: `a_file_nested_too_deeply_gets_one_p0002`, `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`.
  31. **Commands run on a thread of the default size.** `crates/nova-cli/src/main.rs`:
      replace

      ```
              .stack_size(nova_driver::PIPELINE_STACK)
              .spawn(move || dispatch(command));
      ```

      with

      ```
              .spawn(move || dispatch(command));
      ```

      - Run `cargo build --locked --workspace` and `SH a_panic_on_the_pipelines_thread a_long_decimal_number nova_run_runs_the_program s0_meets_the_depth_limit`, each to its end.
      - Expected: fails: `s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack`.
  32. **A panic exits 1.** `crates/nova-cli/src/main.rs`:
      `Err(panic) => std::panic::resume_unwind(panic),` becomes `Err(_) => std::process::exit(1),`.
      - Run `cargo build --locked --workspace` and `SH a_panic_on_the_pipelines_thread a_long_decimal_number nova_run_runs_the_program s0_meets_the_depth_limit`, each to its end.
      - Expected: fails: `a_panic_on_the_pipelines_thread_exits_101_with_its_message`.
  33. **The thread is unnamed.** `crates/nova-cli/src/main.rs`:
      `.name("main".to_string())`: the line goes.
      - Run `cargo build --locked --workspace` and `SH a_panic_on_the_pipelines_thread a_long_decimal_number nova_run_runs_the_program s0_meets_the_depth_limit`, each to its end.
      - Expected: fails: `a_panic_on_the_pipelines_thread_exits_101_with_its_message`.
  34. **`nova run`'s program runs on the 64 MB thread.** `crates/nova-cli/src/main.rs`:
      replace

      ```
                      Ok(program) => Ran::Program(Box::new(program)),
      ```

      with

      ```
                      Ok(program) => {
                          program.run();
                          Ran::Done(Ok(()))
                      }
      ```

      - Run `cargo build --locked --workspace` and `SH a_panic_on_the_pipelines_thread a_long_decimal_number nova_run_runs_the_program s0_meets_the_depth_limit`, each to its end.
      - Expected: fails: `nova_run_runs_the_program_on_the_main_thread`.
  35. **The language server's checker has the default stack.** `crates/nova-lsp/src/checker.rs`:
      `.stack_size(nova_driver::PIPELINE_STACK)`: the line goes.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --test lsp -- a_file_nested_too_deeply`, each to its end.
      - Expected: fails: `a_file_nested_too_deeply_gets_one_p0002`.
  36. **`nova dev`'s rebuilds have the default stack.** `crates/nova-cli/src/cmd/dev.rs`:
      `.stack_size(nova_driver::PIPELINE_STACK)`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: `nova dev` serves on a port, which no test may bind (decision 49).
  37. **A node's span prints its start twice.** `crates/nova-ast/src/dump.rs`:
      `let _ = write!(self.out, " {}..{}", span.start, span.end);` becomes `let _ = write!(self.out, " {}..{}", span.start, span.start);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_tree_with_every_kind_of_field`, `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_deep_inputs_agree`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  38. **One space a depth.** `crates/nova-ast/src/dump.rs`:
      replace

      ```
                  self.out.push_str("  ");
              }
              self.out.push('(');
      ```

      with

      ```
                  self.out.push_str(" ");
              }
              self.out.push('(');
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_tree_with_every_kind_of_field`, `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_deep_inputs_agree`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  39. **A name without its span.** `crates/nova-ast/src/dump.rs`:
      `let _ = write!(self.out, "@{}..{}", name.span.start, name.span.end);`: the line goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_tree_with_every_kind_of_field`, `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_deep_inputs_agree`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  40. **A float's bits in upper case.** `crates/nova-ast/src/dump.rs`:
      `let _ = write!(self.out, " Float:{:016x}", f.to_bits());` becomes `let _ = write!(self.out, " Float:{:016X}", f.to_bits());`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_tree_with_every_kind_of_field`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`.
  41. **`\` unescaped in a string.** `crates/nova-ast/src/dump.rs`:
      `if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {` becomes `if (c as u32) < 0x20 || c == '\x7f' || c == '"' {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_tree_with_every_kind_of_field`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_mutated_corpus_agrees`.
  42. **A message's `"` escaped.** `crates/nova-ast/src/dump.rs`:
      replace

      ```
              if (c as u32) < 0x20 || c == '\x7f' {
                  let _ = write!(out, "\\x{:02x}", c as u32);
      ```

      with

      ```
              if (c as u32) < 0x20 || c == '\x7f' || c == '`' {
                  let _ = write!(out, "\\x{:02x}", c as u32);
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::the_errors_follow_the_tree`, `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mismatch_report_names_the_file_and_line`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `unreadable_files_are_reported_alike`.
  43. **`Stmt.Let` without `is_mut`.** `crates/nova-ast/src/dump.rs`:
      replace

      ```
                      self.open(depth, "Stmt.Let", Some(s.span));
                      self.bool_field("is_mut", *is_mut);
      ```

      with

      ```
                      self.open(depth, "Stmt.Let", Some(s.span));
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_tree_with_every_kind_of_field`, `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_deep_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`.
  44. **A split without its kind.** `crates/nova-ast/src/dump.rs`:
      `d.field("kind", format_args!("{:?}", split.kind));`: the line goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_tree_with_every_kind_of_field`, `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`.
  45. **Lex errors after the parse errors.** `crates/nova-parser/src/dump.rs`:
      `for e in &lex_errors {` becomes `for e in lex_errors.iter().filter(|_| false) {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::the_errors_follow_the_tree`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  46. **Notes left out.** `crates/nova-parser/src/dump.rs`:
      `for note in e.notes() {` becomes `for note in e.notes().into_iter().skip(1) {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::the_errors_follow_the_tree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`.
  47. **`--dump-ast` shown in `--help`.** `crates/nova-cli/src/cmd/parse.rs`:
      `#[arg(long, hide = true, conflicts_with_all = ["ast", "check", "dump_tokens"])]` becomes `#[arg(long, conflicts_with_all = ["ast", "check", "dump_tokens"])]`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`.
  48. **`--dump-ast` takes `--dump-tokens`.** `crates/nova-cli/src/cmd/parse.rs`:
      `#[arg(long, hide = true, conflicts_with_all = ["ast", "check", "dump_tokens"])]` becomes `#[arg(long, hide = true, conflicts_with_all = ["ast", "check"])]`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`.
  49. **`--dump-ast` prints the token dump.** `crates/nova-cli/src/cmd/parse.rs`:
      `let dump = if cmd.dump_ast {` becomes `let dump = if !cmd.dump_ast {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_long_decimal_number_lexes_with_a_1_mb_main_thread`, `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`, `the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_deep_inputs_agree`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mismatch_report_names_the_file_and_line`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`, `unreadable_files_are_reported_alike`.
  50. **The AST pass skipped.** `crates/nova-cli/tests/selfhost.rs`:
      `let ast = agree_on(&AST, dir, files, batch, code);` becomes `let ast = String::new();`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_deep_inputs_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  51. **The fuzzer's inputs skip the AST pass.** `crates/nova-cli/tests/selfhost.rs`:
      replace

      ```
          if ast {
              agree(&dir, &files, 100, 0)
      ```

      with

      ```
          if false {
              agree(&dir, &files, 100, 0)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_deep_inputs_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  52. **A kind misspelt in KINDS.** `crates/nova-ast/src/dump.rs`:
      `"(Expr.Try",` becomes `"(Expr.Trys",`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`.
  53. **A kind missing from KINDS, its count kept.** `crates/nova-ast/src/dump.rs`:
      `"(Expr.Await",` becomes `"(Pattern.Ident is_mut=true",`.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --lib -- dump::tests`, each to its end.
      - Expected: fails: `dump::tests::every_declared_form_is_listed`.
  54. **A `bool` value missing from KINDS, its count kept.** `crates/nova-ast/src/dump.rs`:
      `"(Type.Ptr is_mut=false",` becomes `"(Pattern.Ident is_mut=true",`.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --lib -- dump::tests`, each to its end.
      - Expected: fails: `dump::tests::every_declared_form_is_listed`.
  55. **The scanner reads no `bool` field.** `crates/nova-ast/src/dump.rs`:
      `"bool" => ["true", "false"],` becomes `"Bool" => ["true", "false"],`.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --lib -- dump::tests`, each to its end.
      - Expected: fails: `dump::tests::every_declared_form_is_listed`, `dump::tests::the_scanner_reads_the_declarations`.
  56. **The scanner reads no variant's fields.** `crates/nova-ast/src/dump.rs`:
      `out.push((v.to_string(), true, fields(t, j + 1)));` becomes `out.push((v.to_string(), true, Vec::new()));`.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --lib -- dump::tests`, each to its end.
      - Expected: fails: `dump::tests::every_declared_form_is_listed`, `dump::tests::the_scanner_reads_the_declarations`.
  57. **The scanner skips `ty.rs`.** `crates/nova-ast/src/dump.rs`:
      `.filter(|p| !p.ends_with("dump.rs"))` becomes `.filter(|p| !p.ends_with("dump.rs") && !p.ends_with("ty.rs"))`.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --lib -- dump::tests`, each to its end.
      - Expected: fails: `dump::tests::every_declared_form_is_listed`, `dump::tests::the_scanner_reads_the_declarations`.
  58. **The scanner passes over a declaration it cannot read.** `crates/nova-ast/src/dump.rs`:
      replace

      ```
                      (_, other) => panic!(
                          "`{keyword} {name}` at token {i}: `{other}` where a body was due \
                           (a tuple struct's fields have no names)"
                      ),
      ```

      with

      ```
                      _ => i += 1,
      ```

      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --lib -- dump::tests`, each to its end.
      - Expected: fails: `dump::tests::the_scanner_reads_the_declarations`.
  59. **The scanner closes a bracket at `->`.** `crates/nova-ast/src/dump.rs`:
      `">" if i > 0 && t[i - 1] == "-" => {}`: the line goes.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --lib -- dump::tests`, each to its end.
      - Expected: fails: `dump::tests::the_scanner_reads_the_declarations`.
  60. **The scanner reads no generic parameters.** `crates/nova-ast/src/dump.rs`:
      replace

      ```
                  if at(open) == "<" {
                      open = past_close(t, open);
                  }
      ```

      with

      ```

      ```

      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --lib -- dump::tests`, each to its end.
      - Expected: fails: `dump::tests::the_scanner_reads_the_declarations`.
  61. **No case holds a record pattern without `..`.** `crates/nova-cli/tests/selfhost/cases/every_other_node.nova`:
      `Point { x } => x,`: the line goes.
      - Run `cargo build --locked --workspace` and `SH every_kind_appears`, each to its end.
      - Expected: fails: `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`.
  62. **The every-kind check reads a `bool` without its node's kind.** `crates/nova-cli/tests/selfhost.rs`:
      `found.insert(format!("({kind} {w}"));` becomes `found.insert(w.to_string());`.
      - Run `cargo build --locked --workspace` and `SH every_kind_appears`, each to its end.
      - Expected: fails: `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`.
  63. **N's depth test asks no P0002 at 129.** `crates/nova-cli/tests/selfhost.rs`:
      `let expected = usize::from(*depth > 128);` becomes `let expected = 0;`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`.
  64. **An AST line names no source line.** `crates/nova-cli/tests/selfhost/compare.rs`:
      `node if node.starts_with('(') => fields.next()?.split_once("..")?.0.parse().ok()?,`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mismatch_report_names_the_file_and_line`.
  65. **N's limit one lower.** `selfhost/src/parser.nova`:
      `if self.depth <= MAX_DEPTH + 1 { return true }` becomes `if self.depth <= MAX_DEPTH { return true }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_deep_inputs_agree`, `the_fixed_cases_agree`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `one_hundred_and_twenty_eight_parentheses_parse_and_129_are_p0002`.
  66. **N goes on past P0002.** `selfhost/src/parser.nova`:
      `if self.halted() || self.too_deep.is_some() || self.crossing_stands().is_some() {` becomes `if self.halted() {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_deep_inputs_agree`, `the_fixed_cases_agree`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `one_hundred_and_twenty_eight_parentheses_parse_and_129_are_p0002`.
  67. **N's generic-arguments rollback keeps the crossing.** `selfhost/src/parse_type.nova`:
      `self.too_deep = too_deep`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`.
  68. **N's rollback drops the crossing, never to stand.** `selfhost/src/parse_type.nova`:
      `self.rolled_back = Some((errors, span, self.pos))`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_deep_inputs_agree`, `the_fixed_cases_agree`.
  69. **N's rolled-back crossing waits past its token.** `selfhost/src/parser.nova`:
      `None => if self.pos >= at { self.rolled_back = None }` becomes `None => ()`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`.
  70. **N counts a method call as two operators.** `selfhost/src/parse_expr.nova`:
      `LParen => after_dot` becomes `LParen => false`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_deep_inputs_agree`.
  71. **N counts a postfix chain's first operator.** `selfhost/src/parse_expr.nova`:
      `if !first && !method_call && !self.deeper() { return None }` becomes `if !method_call && !self.deeper() { return None }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_deep_inputs_agree`.
  72. **N's `||` chain keeps its depth.** `selfhost/src/parse_expr.nova`:
      replace

      ```
                  lhs = Spanned::new(ExprBinary(BinOpOr, lhs, rhs), span)
              }
              self.depth = depth
      ```

      with

      ```
                  lhs = Spanned::new(ExprBinary(BinOpOr, lhs, rhs), span)
              }
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`.
  73. **N's postfix chain keeps its depth.** `selfhost/src/parse_expr.nova`:
      replace

      ```
              let expr = self.parse_postfix_chain(start, expr, ctx)
              self.depth = depth
      ```

      with

      ```
              let expr = self.parse_postfix_chain(start, expr, ctx)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`.
  74. **N's cast type one level deeper.** `selfhost/src/parse_expr.nova`:
      `let ty = self.parse_type_here(ctx)?` becomes `let ty = self.parse_type(ctx)?`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`.
  75. **N's item recovery stops at no `fn`.** `selfhost/src/parser.nova`:
      replace

      ```
                      Fn
                          | Pub
      ```

      with

      ```
                      Pub
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  76. **N's statement recovery keeps its `;`.** `selfhost/src/parser.nova`:
      replace

      ```
                      Semicolon => {
                          let _ = self.advance()
                          break
                      }
                      RBrace => break
                      _ => { let _ = self.advance() }
      ```

      with

      ```
                      Semicolon => break
                      RBrace => break
                      _ => { let _ = self.advance() }
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mutated_corpus_agrees`.
  77. **N's rollback drops its errors.** `selfhost/src/parse_type.nova`:
      replace

      ```
                          // Wasn't a type: roll back
                          self.roll_back(saved, saved_pending, saved_deep)
      ```

      with

      ```
                          // Wasn't a type: roll back
                          self.roll_back(saved, saved_pending, saved_deep)
                          while self.errors.len() > 0 { let _ = self.errors.pop() }
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `generic_arguments_that_fail_roll_back_and_keep_their_errors`.
  78. **N's `>>` closes one list.** `selfhost/src/parse_type.nova`:
      replace

      ```
                      let _ = self.advance()
                      self.pending_gt += 1
                      true
      ```

      with

      ```
                      let _ = self.advance()
                      true
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_deep_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `generic_arguments_that_fail_roll_back_and_keep_their_errors`.
  79. **N's `-` after a line break goes on.** `selfhost/src/parse_expr.nova`:
      replace

      ```
                      Minus if self.newline_ends_expr() => {
                          self.record_split(SplitKindMinus)
                          break
                      }
      ```

      with

      ```

      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`.
  80. **N keeps a final expression as a statement.** `selfhost/src/parse_stmt.nova`:
      `StmtExpr(e) => trailing = Some(e)` becomes `StmtExpr(e) => stmts.push(stmt)`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_deep_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test`s `a_bare_return_ends_at_its_line_and_the_next_arm_gets_a_note`, `component_and_view_are_keywords_only_where_they_begin_one`, `one_hundred_and_twenty_eight_parentheses_parse_and_129_are_p0002`, `the_last_expression_is_the_trailing_one_with_or_without_its_semicolon`.
  81. **N's `component` lookahead takes `->`.** `selfhost/src/parser.nova`:
      replace

      ```
                  LBrace => true
                  Arrow => true
      ```

      with

      ```
                  LBrace => true
                  Arrow => false
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`.
  82. **N reads `view` where a record literal may not be.** `selfhost/src/parse_expr.nova`:
      replace

      ```
                  Ident(w) if w == "view"
                      && !self.no_struct_literal
      ```

      with

      ```
                  Ident(w) if w == "view"
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`.
  83. **N reads a record literal in a condition.** `selfhost/src/parse_expr.nova`:
      replace

      ```
                      if self.check(LBrace) && !self.no_struct_literal {
                          return self.parse_record_literal(path, start)
      ```

      with

      ```
                      if self.check(LBrace) {
                          return self.parse_record_literal(path, start)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`, `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_deep_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_bare_return_ends_at_its_line_and_the_next_arm_gets_a_note`.
  84. **N lets comparisons chain.** `selfhost/src/parse_expr.nova`:
      `self.errors.push(ChainedComparison(chain_span))`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mutated_corpus_agrees`.
  85. **N's `%` is `/`.** `selfhost/src/parse_expr.nova`:
      `Percent => BinOpRem` becomes `Percent => BinOpDiv`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_mutated_corpus_agrees`.
  86. **A context string differs.** `selfhost/src/parse_stmt.nova`:
      replace

      ```
      let init = if self.eat(Eq).is_some() {
                  Some(self.parse_expr("let initializer")?)
      ```

      with

      ```
      let init = if self.eat(Eq).is_some() {
                  Some(self.parse_expr("let initialiser")?)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`, `the_mutated_corpus_agrees`.
  87. **N's bare `return` before an item takes it as the value.** `selfhost/src/parser.nova`:
      `if self.check(Let) || self.is_item_start_at(self.pos) {` becomes `if self.check(Let) {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`.
  88. **N's arm note left out.** `selfhost/src/parser.nova`:
      `self.errors.set(first, Noted(error, note))` becomes `self.errors.set(first, error)`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_bare_return_ends_at_its_line_and_the_next_arm_gets_a_note`.
  89. **N's element prints `children=Some` for none.** `selfhost/src/ast_dump.nova`:
      `Some(_) => "Some"` becomes `Some(_) => "None"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `component_and_view_are_keywords_only_where_they_begin_one`.
  90. **N's notes left out.** `selfhost/src/ast_dump.nova`:
      `out.push("NOTE ")` becomes `out.push("NOTE")`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_bare_return_ends_at_its_line_and_the_next_arm_gets_a_note`.
  91. **N quotes a message's `"`.** `selfhost/src/ast_dump.nova`:
      replace

      ```
              push_escaped(out, "${e}", false)
              out.push("\n")
              for
      ```

      with

      ```
              push_escaped(out, "${e}", true)
              out.push("\n")
              for
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  92. **N's record pattern without `rest`.** `selfhost/src/ast_dump.nova`:
      `self.bool_field("rest", rest)`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases`, `the_fixed_cases_agree`.
- [ ] **Step 9: The review's inputs.** The full run's figures, the new
  tests by binary, the golden suite's line and time, the parse, check and
  help comparisons, the differential's times, and the mutants' outcome,
  with each gap, go in the ledger.
- [ ] **Step 10: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0052 and this plan, and is told to hunt for:
    - an input on which N's parser and S0's part that no generator, case
      or edge input makes: a message, context or note worded otherwise, a
      recovery point, the rollback's errors, `>>`, a split or the halt, the
      trailing expression, `component` or `view`, the record literal's
      restriction, a precedence, the trait body's truncation (decisions 13
      and 14);
    - a nesting that is not counted, or counted twice, on either side, or
      counted differently by the two; a depth not restored after an
      operand, a chain or a postfix chain, so siblings add up; anything
      after P0002; a P0002 at another token; an abandoned attempt's
      crossing kept where the other reading passes, or dropped where it
      fails, on either side; a later stage of S0 that a
      shallow but long input overflows on the 64 MB thread (decisions 18,
      19 and 28 to 30);
    - anything the thread changes: an exit code, a stream, a signal, a
      panic's message, printed twice or not at all, `std::process::exit`'s
      flush, a thread-local the language server keeps, `nova run`'s
      program on the wrong thread, the `Send` argument (decisions 20, 21
      and 37);
    - anything that differs only on Windows or macOS: N's stack on
      Windows' 1 MB main thread, a test that runs only on Unix and hides a
      Windows failure, a path, a line ending (decisions 22 and 36);
    - the dump on either side against ADR 0052 §1: a span, an escape, a
      field's order, `children=`, the errors' order, an example in the
      ADR that S0 no longer prints (decisions 15 and 39);
    - a way for the harness to pass when the two sides differ: an input
      set that skips the AST pass, a kind or value the every-kind test
      does not read or `KINDS` does not list, an exit code not checked (decisions 17, 23 and 41);
    - something that breaks only under repetition: the deep inputs or
      cases differing by system, a fixed directory shared, a stale N;
    - the hidden flag shown anywhere, any command's `--help` changed, its
      conflicts, its further files (decisions 16 and 40);
    - a commit between Task 1 and Task 7 that builds wrong code or fails
      the suite (decisions 45 to 48);
    - the records: ADR 0052, the CHANGELOG, the dated notes, the
      catalog, the branch name in each;
    - each mutant of Step 8 that passes, the accepted ones included, and
      whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock`, a test that
      touches the internet, a temporary name that is not fixed, `unsafe`
      beyond decision 21's.
  - Each finding is fixed in one commit, `5.1b: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Steps 1 and 2 run again.
- [ ] **Step 11: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-51b`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 5.1b adds: N's AST, parser and AST dump, `selfhost ast`, N's
      `@test`s; S0's `nova_ast::dump`, `nova_parser::dump_ast` and the
      hidden `nova parse --dump-ast`; P0002 and the depth limit in both
      parsers; every command's pipeline on a 64 MB thread; the error
      cases, the deep generator and the AST comparison; ADR 0052;
    - the language change: past 128 levels, P0002, postfix chains
      counted by the user's ruling of 2026-10-31 (decision 8); a decimal
      literal of 210,000 digits or more still aborts S0's thread; every
      `.nova` file the base tracks parses and checks as
      before, and every command's `--help` is unchanged;
    - the figures: the suite's `2839 1 10`, the golden suite's
      `147 passed, 9 excluded`, the differential's time here, N's speed
      (6.1× release S0), N's stack at the limit, and that `Cargo.lock` is
      unchanged;
    - the user's decisions, and this plan's decisions, numbered as here,
      with 15, 28 to 32 and 35 marked as differing from the spec;
    - the mutants, those that pass by design and any gap they found, and
      the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems; on the
    `test` job's `ubuntu-24.04` leg, `cargo test`'s log holds
    `test release_golden_suite ... ok`, and on each system `selfhost.rs`'s
    tests pass, 27 on Linux, fewer where a test is for Unix or Linux
    alone; record each system's time for it, and the run's address, in
    the ledger.
  - **If the differential fails on CI and not here,** record its report,
    stop and ask the user: a Windows or macOS difference is the likeliest
    (decision 36). **If N fails at 128 levels on Windows' main thread,**
    stop and ask the user (spec §4.4): the limit is lowered or the link
    gains a larger stack. **If `selfhost.rs` takes more than a minute on
    a system,** ask the user before lowering the counts (spec §9 risk 5;
    decision 43).
- [ ] **Step 12: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 11's.
