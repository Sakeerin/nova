# Phase 5.3a, "The type checker's items", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** The first of Phase 5.3's two parts. S0's type checker gains
`nova_typeck::check_items`, its item stage (passes 1 to 5 of `check_with`)
as a function `check_with` calls first, a pure refactor, and a hidden HIR
dump, `nova check --dump-hir` (ADR 0053's mechanism, the resolution dump's
own parser), which prints what the item stage built for each program:
records, sums, traits, impls, every signature and the externs, by
structure and with the resolution dump's ids, then every diagnostic so far,
fixes included; `--std` prints std's part once, `--target wasm` and
`--test` as `--dump-resolution` takes them. Both sides collect std's items
once per process and mode (the user's decision 2). N (`selfhost/`) ports
passes 2 to 5 function for function (`hir.nova`, `typeck.nova`,
`typeck_ty.nova`, `typeck_traits.nova`, `fixes.nova`), the dump
(`hir_dump.nova`) and `selfhost hir`, and prints the same dump for the
same roots. The differential (`crates/nova-cli/tests/selfhost.rs`)
compares the two on std's three parts, the corpus in three modes (325
programs natively: 251 loose and 74 package files; 39 for WASM; 10 with
tests, where the base, and the spec, had 318, 67 and 9: N's seven new
modules are package files, and its new `typeck_tests.nova` holds tests),
164 / 2 / 1 cases (with Windows line endings too), the type
fuzzer's item inputs and the refusals, on Linux, macOS and Windows; each of
the 16 item codes occurs. `selfhost.rs` moves to a CI job of its own on each
system, N linked against a release runtime (the user's choice of
2026-11-06), and its target says `test = false`. Nothing a user runs
changes. ADR 0055 records the dump, the sharing, N's code, the figures and
the CI job.

**Architecture:**
- **S0** (Task 1): `crates/nova-typeck/src/check/items.rs` (new:
  `check_items`, `ItemTables`, `SharedItems`, `check_items_shared`, the
  splices and `Rebase`), `check.rs` (`check_with` calls `check_items`),
  `dump.rs` (new: the HIR dump's printer, `KINDS`, `LATER_KINDS`),
  `lib.rs`; `crates/nova-resolver/src/lib.rs`
  (`resolve_program_with_shared_std_for_check`, `SharedStd::alone`,
  `Def::rebased`, `ModuleScope::rebased`) and `dump.rs` (`std_start`,
  `write_diagnostics`, `write_place`); `crates/nova-driver/src/lib.rs`
  (`HirDumps`, `dump_std_hir`, `Resolve::SharedForCheck`);
  `crates/nova-cli/src/cmd/run.rs` (`DumpCmd`, `DumpKind`) and `main.rs`;
  the type checker's cases under `tests/selfhost/cases/check/`,
  `check-wasm/` and `check-test/`.
- **The harness, S0's side** (Task 2): `selfhost.rs`'s HIR dump, its
  refusals, case passes, the shared-std fallback test and the type fuzzer's
  test, through S0 alone until N exists (`N_HIR`);
  `tests/selfhost/fuzz.rs`'s `type_mutate` and `TYPE_ITEMS`.
- **N, and the CI job** (Task 3): `selfhost/src/hir.nova`, `typeck.nova`,
  `typeck_ty.nova`, `typeck_traits.nova`, `fixes.nova`, `hir_dump.nova`
  (new), `main.nova`'s `selfhost hir`, `resolver.nova`'s and
  `resolution_dump.nova`'s helpers; N's `@test`s in `typeck_tests.nova`
  (new); `selfhost.rs` sets `N_HIR`; `.github/workflows/ci.yml`'s Selfhost
  job and second clippy command, `crates/nova-cli/Cargo.toml`'s
  `[[test]] … test = false`, `CONTRIBUTING.md`, `agent.md` and the pull
  request template.
- **Records** (Task 4): ADR 0055 (new); `CHANGELOG.md`; dated notes in
  `docs/phase-5-plan.md`, `nova-spec/00-MASTER-SPEC.md` and
  `nova-spec/12-TYPESYSTEM.md`.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §6).
  `crates/nova-cli/Cargo.toml` gains a `[[test]]` table (decision 18).
- N is built by `nova build` through Cranelift. `clang-18` (CI's pin) for
  the golden suite, and Node 22 for the WASM tests, as before.

**Spec:**
`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`,
approved by the user on 2026-11-06, `28f49f2e`, this branch's first commit.
- Read its header, §3 (5.3a), §§1, 2 and 5 to 10 before Task 1.
- The user's decisions are decisions 1 to 8 here, with the dated lines
  of 2026-11-06 and 2026-11-10 (decisions 9 to 14); the defaults the spec
  took, "not asked", are decision 15.

**Base:** `2a67c6ce` ("5.2b: the review's fixes"), `main` after 5.2b, and
the spec's commit on it, `28f49f2e` ("docs: the Phase 5.3 spec is
approved"), whose full suite gives `2869 1 10` (`selfhost.rs`'s 44 among
them; the IPv6 test the only failure here). The branch is
`claude/loving-archimedes-frtf5q-53a`, from `28f49f2e`, so the spec's
commit comes first in the pull request (decision 19).

**Checked on 2026-11-10 and 2026-11-11, before execution:**
- **A prototype built 5.3a whole** on a local branch from `28f49f2e`, in
  this tree: part A, S0's side, `05ad59d8` (`check_items`), `d22bfabc` (the
  hidden `--dump-hir` and std's item tables shared), `ae6221b7` (the
  cases), `7c36b53c` (the harness's HIR dump and the type fuzzer, S0's
  side), `a5a90f71` (ADR 0055 begun), `9e87b1da`, `819b9224` (std's tables
  spliced without cloning them twice; clippy's type aliases); part B, N's
  side, `f3e45072` (N's item passes, its dump and `selfhost hir`),
  `cb0c7165` (N's tests and the refusals compared), `6aa855ab` (ADR 0055
  finished), `d8016169` (rustfmt); part C, `cc38ac37` (`selfhost.rs` in a
  CI job of its own, N on a release runtime). Then step 0, `746a20bb`, "5.3a
  prototype: before the split, the mutants' gaps, a 128-level item test,
  and the records' fixes" (decision 20), on `cc38ac37` and on no branch (the
  split's scripts make it again).
- **CI, measured twice from the prototype** (decisions 11 to 14), each from
  a separate branch with a draft pull request, closed after:
  - run 42201484370 (draft PR #136, head `d8016169`): all 16 checks green;
    `selfhost.rs` in the Test job took Linux 70.80 s (51 tests), macOS
    86.85 s (49), Windows 92.54 s (45), against PR #135's 52.60, 53.25 and
    61.58 s; the projection had said 87, 88 and 102 s. Windows was past
    70 s: the user was asked, and chose a job of its own on a release
    runtime, with no coverage cut (decision 12);
  - run 42774093193 (draft PR #137, head `cc38ac37`): all checks green.
    The Selfhost job: `selfhost.rs` Linux 37.10 s (51 tests), macOS 27.53 s
    (49), Windows 55.38 s (45); job totals 2m48s, 2m13s and 4m57s (build
    1m20s, 59s, 2m19s; the test target's build 27, 21, 54 s); N's tests on
    the debug runtime (`the_selfhost_tests_pass`) 5.49, 3.89, 7.35 s. The
    Test job: 9m38s, 7m00s, 9m02s (Windows had taken 12m31s), Linux `2830
    passed, 10 ignored`, Windows `2807 passed, 9 ignored`. Rust's cache said
    "No cache found" on that first run. Every figure under its line
    (Windows 70 s, the others 60 s): no question needed.
- **The split** (decision 16 **(differs)**): four tasks and the final one,
  each test first but the records: Task 1, S0's `check_items`, dump and
  sharing, with the cases; Task 2, the harness and the type fuzzer on S0's
  side; Task 3, N, `N_HIR` and the CI job; Task 4, the records.
  `split.py` checks that the last commit's tree is `746a20bb`'s
  (`git diff 746a20bb split/5-3a` prints nothing) and that every patch
  applies with plain `git apply`, in order, on `28f49f2e`, with no output,
  each applied tree its commit's, and that no added line holds trailing
  whitespace, a CR or a backslash-u escape.
- **The replay** (`replay.sh`, under the process limit, without
  `NOVA_EMBED_RUNTIME`, as the Test job runs) ran each step as written
  here: each `tN-tests` failed as Steps 2 record it (Task 1's library tests
  do not compile in three crates; Task 2's harness does not compile; Task
  3's N does not compile, so 34 of `selfhost.rs`'s 52 tests fail), and each
  `tN-code` passed: the whole suite `2874 1 10`, `2880 1 10` and `2830 1
  10` (`selfhost.rs` among them, 44 and 50 tests, until Task 3's
  `test = false`; then apart, `SH` 52 passed), the IPv6 test the one
  failure, its formatters and both clippy commands clean; Task 4's ADR
  examples found in S0's dump. Logs: `replay.sh`'s, one an item, at the
  commits of the split this plan's patches are; after the two
  fact-checks' fixes, which changed Tasks 3 and 4 alone, every step of
  theirs ran again at this split's commits (the builds, `SH`, `cargo fmt`,
  both clippy commands, `nova fmt --check`, `adrcheck.py` and the list)
  but Task 3's `S`, which no longer runs `selfhost.rs`, and stands from
  the earlier split's `3b8bf2e7`, whose tree differs in `selfhost.rs` and
  a comment of `ci.yml` alone.
- **At the last commit,** `992a1b06`:
  - the whole suite as the Test job runs it: `2830 1 10`, in 244 s: the
    base's `2869 1 10` less `selfhost.rs`'s 44 and 5 tests more (Task 5,
    Step 1); `selfhost.rs` apart, 52 tests, the base's 44 and 8 more;
  - CI's Linux leg as the Test job runs it, `--all-features` with the
    golden suite required: `2830 1 10` and the golden suite inside it, in
    253 s (Task 5, Step 2);
  - the golden suite: `release golden: 147 passed, 9 excluded, Ubuntu clang
    version 18.1.3 (1ubuntu1)`, in 21 s (Task 5, Step 3);
  - `nova parse` over every base-tracked `.nova` file: `parse: 1948 runs
    over 974 files, 0 differ; --check exits nonzero on 123 at the base, 123
    now`; `nova check`: `check: 1078 checks, 0 outside std differ, 0 std
    files differ`; `nova test` and `nova build` on every package:
    `packages: 22 runs, 0 differ` (Task 5, Step 4);
  - `--help`: `help: 22 of 22 identical; hidden flags shown: 0`;
    `usagecmp.sh`: `usage: 306 of 306 runs identical; in a terminal, 10 of
    10; --dump-ast as --dump-tokens: 25 of 25; --dump-resolution refused:
    114 of 114; --dump-hir refused: 126 of 126` (Task 5, Step 5);
  - Rust 1.78: clean, `Finished` in 45 s; `Cargo.lock` unchanged, and of
    the manifests only `crates/nova-cli/Cargo.toml`, `+10` (Task 5, Step 6);
  - `selfhost.rs` without the release runtime, as `SH` runs it, three
    times: 41.4, 42.1 and 40.9 s (user CPU 130 to 132 s); with
    `NOVA_EMBED_RUNTIME=1`, as the Selfhost job runs it, three times: 21.9,
    21.6 and 21.2 s (user CPU 57 to 59 s); N's `nova test`: `test result:
    ok. 44 passed; 0 failed; 0 trapped; 44 total`, on the debug runtime
    (without the variable, and with it under `NOVA_RUNTIME_LIB`) and on the
    release runtime (with it, embedded); `the_selfhost_tests_pass` on the
    debug runtime, the Selfhost job's last step, 4.43 s (Task 5, Step 7);
  - N's live heap and stack, measured again (ADR 0055 §§12 and 13 now hold
    both measurements): the same peaks and live heaps, the times about half
    as long again on this container; on 1 MB N survives `k` = 124 (7,936
    levels) on both runtimes; the debug runtime overflows at 125 (8,000
    levels), the release runtime at 125 or 126, borderline at 125, which
    it took in two of three measurements: within bytes of the limit (Task
    5, Step 7); every written nesting of a type at 128 levels,
    in each item position, N checks on 1 MB as S0 does (Task 3's
    `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`;
    the item passes walk arrays, generic arguments, tuples and `fn` types
    128 levels deep, while parentheses are gone once parsed and optionals,
    references and pointers stop at the outermost level with one E0900,
    their depth the parser's).

  - Where each ran: Tasks 3's and 4's replay and the second fact-check's
    nine mutants at the last commit itself; `selfhost.rs`'s times and the
    full mutant run at `750bbf5a`, whose tree differs from the last
    commit's in ADR 0055's text, a comment of `ci.yml` and the item
    test's settling of N's output, which is the identity on Linux; the
    whole suite, CI's Linux leg and the golden suite at `5bda68ce`, which
    lacks besides the 128-level item test, the fallback test's new
    comment and the dated notes' branch, none of them in the Test job's
    suite; the comparisons (Steps 4 and 5), the lockfile check and Rust
    1.78 at `3705677b`, whose tree is `5bda68ce`'s but for ADR 0055's
    text; the heap at `4d0eca73`, and the stack at `4d0eca73`, at
    `5bda68ce` and, in the second fact-check, at `e7cfdd54`; `4d0eca73`
    lacks besides step 0's last case (decisions 21 and 22). `nova` and N
    are byte for byte the same at each.
- **The mutants** (decision 21): 150 breakages of what 5.3a adds, each a
  text replaced in one file, run one at a time at the split's last commit
  under the process limit (Task 5, Step 8): S0's dump forms (34),
  `check_items`' boundary and std's tables shared (15), the driver's and the
  resolver's sharing (9), the hidden flag's refusals (5), N's item passes at
  each code site (50), N's fixes in each form (12), N's printer (8), N's
  sharing and command (6), the harness and the fuzzer (10), the CI job (1).
  142 are caught and 8 pass by design. The first run, at the split of
  `cc38ac37`, found seventeen that passed and should not; step 0's first
  pass closed sixteen of them, the second run, at `4d0eca73`, found the
  seventeenth, mutant 125, still passing, and step 0's last case closed it
  (decision 20). The last full run, of all 150 after the first
  fact-check's fixes, found the same 8 passing, and the 128-level item
  test among the failures of 32 caught mutants; after the second
  fact-check's, mutant 142 and the 8 ran again, unchanged.
- **What the checks left standing,** each recorded or handed on:
  - macOS and Windows are reasoned from the code here and were run on CI
    twice from the prototype (decisions 11 and 14), before step 0's fixes,
    which add tests and cases only; the PR's runs are the check;
  - N overflows its 1 MB main thread on a type normalization builds 8,000
    levels deep, where S0 does not (decision 10, ADR 0055 §12);
  - the mutants that pass by design (decision 21).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` under `RUSTUP_TOOLCHAIN=1.78.0`
  with `RUSTFLAGS=-D warnings`. No `std::sync::LazyLock` (1.80), no
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79). 5.3a's Rust
  adds `slice::sort_by_key` (1.7), `str::replacen` (1.16), `Option::filter`
  (1.27), `str::repeat` (1.16), `Iterator::copied` (1.36), `std::mem::take` (1.40), `matches!`
  (1.42), `str::strip_prefix` (1.45), `bool::then` (1.50), an `assert!` in
  a constant (1.57), `std::thread::scope` (1.63), `let`-`else` (1.65), and
  `Result::is_ok_and` and `std::sync::OnceLock` (1.70): each at most 1.78
  (a `grep` over the diff's added lines).
- **No new crate, and `Cargo.lock` does not change** (spec §6). If a step
  seems to need either, stop that task there and ask the user; do not add
  the crate or change the lockfile first. `crates/nova-cli/Cargo.toml` gains one
  table, `[[test]] name = "selfhost" … test = false` (decision 18), and no
  dependency.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`. 5.3a's Rust outside tests adds none; N's
  `unwrap()`s are on values its own code has checked.
- **No `unsafe`.** 5.3a adds none.
- **Tests never touch the internet,** bind no port, and need no `clang`
  (spec §6). The differential runs `nova` and the N it builds, nothing
  else.
- **Temporary names are fixed** (spec §6): under `std::env::temp_dir()`,
  `nova-selfhost-<name>`; 5.3a adds `hir-headers`, `crlf-hir`, `types`
  (the type fuzzer's seeds), `item-error-std` (an altered std copy) and
  `deep-n-items` (the 128-level item inputs).
  These are every fixed temporary name in the diff
  (`grep -n 'fresh_dir(\|std_copy(\|temp_dir'` over its added lines); the
  driver's and the type checker's new tests and N's new `@test`s write
  nothing. Each is removed and made afresh by the test that uses it. A test
  that needs `$NOVA_HOME` sets a fresh one.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and the golden suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the suite here with `NOVA_REQUIRE_NODE=1`. No test of 5.3a needs Node.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and
  passes. 5.3a adds no runtime program, so it stays at 147.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary, which only a build
  of the workspace writes. `nova build` of `selfhost/` links against that
  runtime, or, in a `nova` built with `NOVA_EMBED_RUNTIME=1`, against the
  release runtime embedded in it, unless `NOVA_RUNTIME_LIB` names one.
  Every run step starts with `cargo build --locked --workspace`
  (Conventions).
- **`selfhost.rs` is not in `cargo test --workspace` from Task 3 on:** its
  target says `test = false`. `S` runs the whole suite as the Test job
  does, `SH` runs `selfhost.rs` as the Selfhost job does; every figure here
  counts the two apart. Before Task 3, `S` holds `selfhost.rs` too, and its
  count is given apart.
- **`NOVA_EMBED_RUNTIME` is set for one command only, or for a whole step
  that says so** (Task 5, Steps 7 and 8): a change of the variable makes
  cargo rebuild `nova-cli` with or without the release runtime, and every
  other test that links a program then runs on that runtime. Each such
  step ends with `cargo build --locked --workspace` without it.
- **The process limit.** The differential starts two processes a batch,
  and the suite thousands; every test, mutant and suite runs under the
  limit of the Conventions, so that a mistake that forks without end fails
  to fork rather than filling the machine.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other. Never build in another worktree, and never run two
  suites, or a suite and `SH`, at once: the differential's fixed
  directories are shared.
- **Keep `df -h /` above 6G.** Older duplicate test executables in
  `target/debug/deps` may be removed between commands, never while one
  runs. Each run of the whole suite leaves about 0.3 GB under
  `/tmp/nova-todo-*`, which is never cleaned: begin with room for this
  plan's eight runs (the replay began with 21G free and ended with 20G).
  Every `cargo` command here selects the whole workspace (Conventions), so
  cargo keeps one copy of each test executable. Task 5's Rust 1.78 check
  uses its own target directory, in `/dev/shm`, removed after.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.3a's
  patches holds one. Apply the patches with `patch_block`, from the
  committed plan.
- **No patch adds a CR, a NUL or trailing whitespace.** The inputs that
  need them are written by the tests at run time (the CRLF cases), not
  kept as files. The cases' `nova.toml` files are LF in the repository; a
  Windows checkout may give them CRLF, which both readers take.
- **Nothing writes into the repository but the patches.** `nova build -o`
  writes N under the system's temporary directory and `nova test` writes
  nothing into the package; `git status --short` is empty after every run.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.3a.
- **This machine is Linux.** macOS and Windows results come from CI; what
  differs there is reasoned from the code (Review Focus 1).

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-53a.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-53a-<task>.txt`:
  the subject the task names, a blank line, and these two trailer lines,
  exactly:

  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
  ```

  Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-53a-<task>.txt"`. A task with a tests
  patch and a code patch makes one commit, after both. Every commit of
  this plan ends with those two lines: the plan's own (Task 0), each
  task's and the review's fixes (Task 5, Step 10). The pull request's
  body ends as Task 5, Step 11 gives it. No model ID (a `claude-…` model
  name) goes in any artifact: no commit, file, pull request, comment or
  ledger line names one.
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
  mutants ran under it on 4 CPUs. Test threads scale with the cores, so a
  machine with many more may need a higher cap: if a test fails to start a
  thread, read the group's `pids.peak`, raise `pids.max`, and record it.
  `pids.peak` counts from the group's creation.
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
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-53a"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG NOVA_EMBED_RUNTIME
  mkdir -p "$NOVA_HOME"
  SH() { cargo test --locked --workspace --test selfhost -- "$@"; }
  S() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test run_tests -- release_golden_suite --nocapture "$@"; }
  ```

  So `SH` runs `selfhost.rs`, the differential and the flags' tests, `S`
  the whole suite (from Task 3 on, without `selfhost.rs`, as the Test job
  runs it), and `G` the golden suite through LLVM as CI's Linux leg runs
  it. `NOVA_HOME` keeps a run by hand off `~/.nova`; every test that needs
  one sets its own.
- **Every `cargo` command selects the whole workspace,** `--workspace`
  with `--test NAME` or `--lib` to narrow it, never `-p`. Cargo unifies
  features over the packages a command selects, so each `-p` selection
  builds and links its own copy of every test executable it needs, which
  the disk does not have room for; a `--workspace` command reuses one
  copy. `S` gives each code step the whole suite, about four to six
  minutes; its one failure is the IPv6 test (Task 0). The exceptions are
  the two clippy commands and `cargo fmt --all`, written as CI writes
  them, without `--workspace`: run from the root, whose `Cargo.toml` is a
  virtual manifest (`[workspace]` alone), cargo selects every member all
  the same, so they too select the whole workspace.
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
  PLAN=docs/superpowers/plans/2026-11-10-phase-5-3a-the-type-checkers-items.md   # this file, committed
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
  nothing: no 5.3a patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. Task 1's tests patch
  adds the 178 case files, 1,212 lines (the whole patch, with the Rust
  tests beside them and every file's header and hunk lines, is
  2,852 lines), and Task 3's code
  patch 4,286 added lines (a 4,540-line patch): each is one block
  all the same. The non-ASCII characters the patches add are `§` (Tasks 1
  to 4), `…` (Tasks 1 to 4), `—` (Tasks 1, 3 and 4) and `×` (Task 4). A
  block that does not apply means the tree differs from the plan's: stop,
  and record why in the ledger.
- **Each task** but Task 4 is test first: its tests' patch, their run and
  the expected failure, then its code's patch, its run and the expected
  pass. Task 4's patch is the records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy,
    `cargo clippy --locked --all-targets --all-features -- -D warnings`, as
    CI runs it, which ends with `Finished` and no warning;
  - `selfhost.rs`'s clippy,
    `cargo clippy --locked --all-features --test selfhost -- -D warnings`,
    CI's second command from Task 3 on, since `--all-targets` skips a
    `test = false` target (run in every task; before Task 3 it repeats
    part of the first), which ends with `Finished` and no warning;
  - `target/debug/nova fmt --check std examples tests/wasm tests/dom
    tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost`,
    with the `nova` the step's build made, which prints nothing. CI runs
    `nova fmt --check std examples selfhost` (`selfhost` since 5.1a); this
    set is a superset of CI's, the earlier plans' own, and adds the
    Nova sources under `tests/`.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost
  if wrong.


## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
check in the task that owns the code:

1. **Windows paths** (`settled_stdout` in `selfhost.rs`'s `run`): the HIR
   dump prints the loader's E0004 and E0093 when a program stops before
   the type checker, whose messages hold a path S0 writes with `\` on
   Windows; `run` settles S0's `check --dump-hir` and N's `hir` as it
   settles the resolution dump (ADR 0054 §7), and every S0/N comparison
   takes its output from `run`. No type checker message prints a path →
   `every_run_of_a_hir_dump_settles_e0004s_paths_on_windows` (Task 2)
   (mutant 142). The PR's Windows and macOS legs are the runs.
2. **N's messages and fixes word for word** (`typeck*.nova`'s 16 codes at
   every site, `fixes.nova`'s import fix, its edit after the last import
   or above the first item's comments, with the file's line ending, and
   "did you mean") against S0's `check.rs`, `check/items.rs` and
   `check/fixes.rs` → the differential on the cases (both line endings),
   the corpus and the type fuzzer, N's `@test`s (Tasks 2 and 3) (mutants
   64 to 125). The reviewer reads each N pass beside its S0 function, same
   names.
3. **std shared at the item level** (`check_items_shared` and the splices,
   `Rebase`, `resolve_program_with_shared_std_for_check`, `HirDumps`; N's
   `share_items` and rebasing): a program reads std's tables rebased past
   its own ids, std's items collected once; std alone with an item
   diagnostic falls back to the full check →
   `shared_items_check_what_the_full_check_does` (every table),
   `std_shared_for_check_defines_what_the_full_path_does`,
   `std_checked_once_dumps_what_std_checked_for_each_program_dumps`,
   `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`
   (Tasks 1 and 2) (mutants 36 to 58, 134 to 138). The reviewer reads the
   splices against the passes: each adds std's part where the full pass
   would have collected it, after the program's.
4. **`check_items` is a pure refactor** (`check_with` calls it, then the
   bodies from the restored state): every `nova check`, `build` and `test`
   prints what the base printed → `check_items_is_the_prefix_of_check_with`
   (Task 1) (mutant 35), `checkcmp.sh`, `pkgcmp.sh` (Task 5, Step 4).
5. **The hidden flag stays hidden** (`DumpCmd::from_args`, `DumpKind`):
   only `check --dump-hir` first after `check` takes it; `--help`, clap's
   tips and every other command line are the base's, byte for byte; the
   form's refusals name `--dump-hir` →
   `the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps` (Task
   2) (mutants 59 to 63), `helpcmp.sh`, `usagecmp.sh` (Task 5, Step 5).
6. **The harness keeps every input** (`agree_on_hir_cases`,
   `agree_in_parts`, the part counts, the floors, `ITEM_CODES`, the type
   fuzzer's floors and `N_HIR`): every root is compared in its mode, each
   part counted by its header, each aimed code at least twice →
   `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`,
   `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code` (Tasks 2
   and 3) (mutants 140 to 149).
7. **The CI job** (`ci.yml`'s Selfhost job, `test = false`, the second
   clippy command): `selfhost.rs` runs on each system with a release
   runtime embedded in `nova`, N's own tests once more on the debug
   runtime; the Test job and `cargo test --workspace` skip it → `names.py`
   (Task 5, Step 1) (mutant 150); the reviewer reads the job against the
   Test job's steps and ADR 0055 §15. Only the PR's CI runs it.
8. **N's stack** (the user's decision 10, **(differs)** from S0): N
   overflows a 1 MB main thread on a normalized type about 8,000 levels
   deep (it survives 7,936, `k` = 124; the debug runtime overflows at
   `k` = 125, the release runtime at 125 or 126, as last measured), which
   nothing in the corpus, the cases or the fuzzer approaches → ADR 0055
   §12's figures, measured again (Task 5, Step 7). Two harness tests run
   N on a 1 MB main thread on every system: N's parser,
   `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread` (`selfhost
   ast` on every nesting the deep generator writes, 128 and 129 levels),
   and N's item passes,
   `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`
   (Task 3: `selfhost hir` on every written nesting of a type at 128
   levels in each item position, its dump compared with S0's; the item
   passes walk arrays, generic arguments, tuples and `fn` types 128 levels
   deep, while parentheses are gone once parsed and optionals, references
   and pointers stop at the outermost level with one E0900, their depth
   the parser's).

## Decisions

The user's, from the spec's header (2026-11-06):

1. **Two pull requests, split by stage** (the user's decision 1): this is
   5.3a, the items: signatures, records, sums, traits, impls, externs and
   the item-level errors, with a dump of the tables over std and the whole
   corpus. 5.3b, the bodies, follows with the full HIR.
2. **std is shared, and CI is measured before the plan** (the user's
   decision 2, as given: "share std (type-check std once per process on
   both sides, as 5.2b's option B, from the start) and measure real CI
   from the prototype before writing the plan, then ask the user again
   with real figures (recommended)"). Here: both sides collect std's items once per
   process and mode (`SharedItems`, `share_items`), the item part of
   type-checking std, 5.3a's; std's bodies are 5.3b's. CI was measured
   from the prototype twice (decisions 11 and 14) and the user answered
   (decision 12).
3. **The dump is the full HIR** (the user's decision 3): 5.3a prints its
   part, the tables, every signature and the diagnostics; 5.3b adds every
   function's locals and expression tree after the externs (ADR 0055 §2's
   grammar marks the place).
4. **S0 checks methods in `DefId` order** (the user's decision 4, as
   given: "check methods in DefId order (recommended); std's closures get ids in a
   segment of their own so std's part of the dump prints once.
   CHANGELOG."). Here: all of it is 5.3b's, which checks the bodies, gives
   std's closures their segment and writes the CHANGELOG line for the new
   order; 5.3a checks no body, reorders nothing and has no closure to
   number, so its CHANGELOG says nothing of the order.
5. **Diagnostics are compared completely, fixes included** (the user's
   decision 5, as given: "compare everything, fixes included (recommended): code, span,
   message, notes, secondary labels and fixes; `fixes.rs` is ported in
   5.3b"). Here: code, severity, span, message, notes, secondary labels
   and fixes are compared for the 16 item codes. The user put `fixes.rs`'s
   port in 5.3b; porting the part of it the item passes call (the import
   fix and its edit, "did you mean") here, in `fixes.nova`, is the spec's
   own default, which the user was not asked (decision 15), since the
   item diagnostics cannot be compared with their fixes otherwise; the
   rest of `fixes.rs` is 5.3b's, as the user said.
6. **The inline tests become cases** (the user's decision 6, as given:
   "extract ~527 tests (637 sources, `format!` and loops expanded) into
   `.nova` case files both compilers check (recommended), as 5.2 did for the resolver;
   ~46 internal tests stay Rust-only"). Here: the item-level ones, 154
   cases from the 158 sources of 114 tests (ADR 0055 §5), beside 10 by
   hand under `check`, 2 for WASM and 1 under test mode; 5.3b extracts
   the body-level ones by the same rule, the rest of the ~527, and the
   ~46 internal tests stay Rust's.
7. **A new type-level fuzzer** (the user's decision 7, as given: "a new
   type-level fuzzer (recommended): mutate
   programs that check (literal types, argument counts, field names,
   deleted match arms, `?` on non-Option, ...), with floors so every main
   code occurs, within the CI-time budget"). Here: its item mutations
   (`fuzz::type_mutate`, `TYPE_ITEMS`: a parameter's, return's or field's
   type, a generic list, an impl's method, a duplicated impl, an appended
   item), with fixed seeds 401 to 403 and a floor of two on each code it
   aims at; the body mutations the user named (literal types, argument
   counts, field names, deleted match arms, `?` on a value that is not an
   `Option`) are 5.3b's, whose codes they aim at.
8. **The language server's probe and index are not ported** (the user's
   decision 8): pass 1 runs in S0's `check_items` and records nothing
   unless asked; N ports passes 2 to 5.

The dated lines of 2026-11-06:

9. **CI measured from the prototype** (the user's line of 2026-11-06,
   after the local projection of 87, 88 and 102 s): the prototype was
   pushed to `claude/loving-archimedes-frtf5q-53a-ci` with a draft pull
   request, closed after, never merged.
10. **N's stack** **(differs)** (the user's line of 2026-11-06, on part
    B's measurement of a contrived normalized type 8,128 levels deep): N
    overflows its 1 MB main thread on such a type, where S0, on 64 MB,
    does not; ADR 0055 §12 records the depth N survives, as last measured
    `k` = 124 (7,936 levels), overflowing at `k` = 125 (8,000) on the
    debug runtime and at 125 or 126 on the release runtime, with part
    B's 126 and 127 kept as its history; 5.5's driver runs N on a
    large-stack thread. No language or runtime change now.
11. **CI run 42201484370** (the user's line of 2026-11-06; draft PR #136,
    head `d8016169`, closed unmerged): all 16 checks green; `selfhost.rs`
    Linux 70.80 s (51 tests), macOS 86.85 s (49), Windows 92.54 s (45),
    against PR #135's 52.60, 53.25 and 61.58 s. Windows past 70 s: the user
    was asked.
12. **A CI job of its own, on a release runtime** **(differs)** (the
    user's choice, 2026-11-06, of the cut options measured): options (c)
    and (d) only: `selfhost.rs` runs in a job of its own on each system,
    with N linked against a release runtime (`NOVA_EMBED_RUNTIME=1`); no
    coverage cut. Not chosen: (a) covering roots, (b) fewer fuzz inputs.
    The spec does not say where the harness runs, but implies the Test
    job: §3.6 extends `selfhost.rs` ("`selfhost.rs` gains a fourth kind of
    dump"), which ran in the Test job then, and §6 keeps the earlier
    differentials where they were ("The 5.1 and 5.2 differentials stay as
    they are."). Decision 17 says where the change lands.

The dated lines of 2026-11-10:

13. **CI measured again** (the user's line of 2026-11-10, after a
    container reset): part C, `cc38ac37`, was pushed to the same
    measurement branch with a new draft pull request, closed after, never
    merged.
14. **CI run 42774093193** (the user's line of 2026-11-10; draft PR #137,
    head `cc38ac37`, closed unmerged): all checks green. The Selfhost job:
    `selfhost.rs` Linux 37.10 s (51 tests), macOS 27.53 s (49), Windows
    55.38 s (45); job totals 2m48s, 2m13s, 4m57s (build 1m20s, 59s, 2m19s;
    the test target's build 27, 21, 54 s); the debug runtime's
    `the_selfhost_tests_pass` 5.49, 3.89, 7.35 s. The Test job 9m38s,
    7m00s, 9m02s (Windows had taken 12m31s); its sums Linux `2830 passed,
    10 ignored`, Windows `2807 passed, 9 ignored`. Rust's cache said "No
    cache found" on the first run. Every figure under its line (Windows
    70 s, the others 60 s): no question needed. The release workflow ran
    too, as a dry run ("Publish the release" skipped).

The spec's defaults, "not asked" (its header), as built:

15. **(differs)** where marked:
    - the HIR dump is separate from the resolution dump: S0's flag
      `nova check --dump-hir`, N's command `selfhost hir`;
    - S0's order of bodies is 5.3b's;
    - the part of `fixes.rs` the item passes call is ported here
      (decision 5);
    - floats print as `to_bits`' 16 hex digits: 5.3b's, since no item
      holds a float;
    - the dump shows the type checker's output before the driver's
      `keep_the_entry_main`;
    - the fuzzer's item mutations here, its body mutations in 5.3b.
    - ADR 0055 §7's and §10's decisions, each with its cost if wrong.
      Three follow the spec where it states them, and are not
      departures: `check_items` covers passes 1 to 5 and N ports 2 to 5
      (§3.1); a record and a sum print as their id (§3.3, "a record or
      sum as its id and arguments"); the dump lives in `nova-typeck`
      (§7, "a `dump` module"). **(differs)**, the spec's open points as
      the prototype settled them: the dump reuses the resolution dump's
      diagnostics printer, places and `StdStart`; an impl's place is its
      self type's span; every generic
      parameter's bounds print, empty or not; the sharing splices per
      pass; a record prints no bounds; `abi=Js` occurs only in std's WASM
      part; E0099's note is a body diagnostic (5.3b's), so
      `cases/check-test/` holds tests beside items; N appends std's module
      scopes unrebased and adds std's offset where it reads one; N keeps
      lists by `DefId` where S0 has maps; `selfhost hir` shares `selfhost
      resolve`'s parser and refusals; N's tests resolve their sources
      without std.

How this plan is cut and checked:

16. **Four tasks, three test first, then the records** **(differs)**: the
    spec's §7 lists files by crate, and the prototype went S0, the cases,
    the harness on S0's side, ADR begun, two fixes, N, N's tests, ADR
    finished, rustfmt, the CI job. The split keeps that order with four
    changes, each the nearest clean cut:
    - **the cases go in Task 1:** the driver's
      `std_checked_once_dumps_what_std_checked_for_each_program_dumps`
      reads `cases/check*` (S0's shared against full dumps), so the cases
      are Task 1's test inputs, not the harness's;
    - **the prototype's fixes go with their code:** `9e87b1da`'s splice in
      Task 1, `819b9224`'s type aliases in Tasks 1 and 2, `d8016169`'s
      layout in Task 3, step 0's tests in the task whose code they test
      (`split.py` applies step 0's harness fixes to each version of
      `selfhost.rs` that holds the text);
    - **ADR 0055 goes whole in Task 4,** not begun in part A;
    - **the CI job goes in Task 3** (decision 17).
    One file has versions the head does not: `selfhost.rs` compares S0
    alone in Task 2 (`N_HIR` false, each set's assertions on S0's dump),
    until Task 3 sets `N_HIR` and compares both sides. Task 1's tests
    patch, applied before Step 3, builds with one warning, the unused
    `use super::*;` of `items.rs`, which holds only its tests until the
    code patch. Each task's commit, made after its code patch, builds and
    passes the whole suite, as each of the split's code commits did.
17. **The CI job in Task 3** **(differs)** (decision 12; spec §7 names
    none of these files): `ci.yml`'s Selfhost job and second clippy
    command, `crates/nova-cli/Cargo.toml`'s `test = false`, `selfhost.rs`'s
    note, `CONTRIBUTING.md`, `agent.md` and the pull request template land
    with N, whose item stage made `selfhost.rs` take the time the job
    pays for: no commit has the Test job running N's item stage, and
    `selfhost.rs`'s counts stand apart from Task 3's code commit on.
18. **`test = false`** **(differs)** (decision 12): `cargo test
    --workspace` no longer runs `selfhost.rs`; `cargo test --workspace
    --test selfhost` does, here (`SH`) and in the Selfhost job; clippy's
    `--all-targets` skips the target, so CI lints it by name. 5.2b's
    constraint "every `Cargo.toml` unchanged" becomes "no dependency
    changes": `crates/nova-cli/Cargo.toml` gains the `[[test]]` table.
    Every count here gives the Test job's suite (`S`) and `selfhost.rs`
    (`SH`) apart.
19. **The branch starts from the spec's commit** **(differs)** (the
    brief's choice; the spec's header says "Each part is built on a branch
    of its own from `main`; its plan names it"):
    `claude/loving-archimedes-frtf5q-53a` from `28f49f2e` (`main` `2a67c6ce`
    plus the approved spec, which `main` does not hold yet), then this
    plan's commit. So the pull request carries the spec's commit first,
    and the merge by rebase puts the spec on `main` before the plan and
    the code that cite it. Task 0 checks it.
20. **Step 0, the prototype's fixes before the split** (`746a20bb`):
    - **the mutants' gaps,** seventeen mutants that passed and should
      not in the first run (decision 21), mutants 38 to 42, 45 to 47, 49,
      56, 59, 122 to 125, 140 and 147:
      `shared_items_check_what_the_full_check_does` compares every table
      of the shared and the full check, its std given an extern (std's
      supertraits, records, sums, signatures, externs, impl heads, method
      places, impl selves and trait defaults spliced); the new resolver
      test `std_shared_for_check_defines_what_the_full_path_does` (std's
      scopes rebased for the type checker); the HIR dump's own refusals
      must name `--dump-hir`; five cases, `std_supertraits_in_a_programs_bounds`,
      `fixes_did_you_mean_a_primitive_or_a_type_parameter`,
      `fixes_held_back_by_a_clash`, `fixes_none_when_two_modules_export_the_name`
      and `fixes_none_in_a_dependency`, the last a package, whose dump
      differs when the package cases are not compared (mutant 147, which
      no case caught before); and `N_HIR` asserted true in a constant, so
      the harness cannot quietly compare S0 alone;
    - **ADR 0055's example source** keeps its indentation (`;;       fn
      scale`): the dump's spans are of the indented source, which
      `adrcheck.py` (Task 4) reads;
    - **the harness's case count,** 164 under `check` (154 extracted, 10 by
      hand), where the prototype's comment said 158 (154 and 4);
    - **ADR 0055 §§12 and 13** hold the stack and the heap measured again
      here, beside the prototype's figures (decision 22);
    - **after the plan's fact-check:** the harness's
      `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`
      (spec §3.7: N's item passes on 128-level types on a 1 MB stack,
      which no test ran: `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`
      runs N's parser alone), in Task 3's tests, since it needs N's
      `selfhost hir`; the fallback test's comment, which claimed a test of
      S0's fallback that no input can reach (mutant 50); and the records:
      ADR 0055's Status accepted on this branch, as ADR 0054's, and the
      dated notes naming the branch, not the prototype's; §9's case count,
      164; §12's part B figures marked as history beside the latest
      measurement, and the two harness tests named; no scratch path cited;
      §13's second measurement dated and placed truly; §15 holding CI run
      42774093193 and the head's test count; a Consequences section.
    Step 0 was made in five passes as the checks found them (the first
    run's gaps but mutant 125, and the example; mutant 125's case and the
    figures; the stack's second day; the first fact-check's findings; the
    second's), one commit in the end; `make_fix.py` makes it.
21. **The mutants** (Task 5, Step 8): 150 mutants (S0 63, N 76, the
    harness 10, the CI job 1), 142 caught, 8 accepted, each with its
    reason in Task 5, Step 8: mutants 11, 12, 50, 52, 53, 132, 136 and 143.
    The first run, at the split of `cc38ac37`, found decision 20's
    seventeen among those that passed; all 150 ran again at `4d0eca73`,
    the split of step 0 without its last case, where mutant 125 (two
    modules exporting the name) still passed and the other sixteen were
    caught; the 38 whose failing
    tests could grow with that case (each that failed neither
    `the_hir_dumps_agree_on_the_cases_with_windows_line_endings` nor
    anything of S0's alone) ran once more at `3705677b`: only mutant 125
    changed, now caught. After the fact-check's fixes, which added the
    128-level item test, all 150 ran a fourth time, at `750bbf5a`, whose
    tree is the last commit's but for five lines of ADR 0055: the same 8
    passed, every other was caught by the same tests as before, and 32
    (mutants 1, 5, 6, 8, 9, 13 to 15, 18, 22 to 28, 32, 34, 44, 54, 57, 62,
    68, 81, 84, 86, 87, 98, 99, 126, 130 and 139) failed the new test
    too, which Task 5, Step 8 lists; it catches none that passed. After
    the second fact-check's fixes (the item test's N output settled as
    `run` settles it, comments and the ADR), mutant 142, the one whose
    target the change touches, and the 8 accepted ran again at
    `992a1b06`: the same outcomes and failing tests. The
    mutants ran with `NOVA_EMBED_RUNTIME=1`, as the Selfhost job runs
    `selfhost.rs`: a mutant of S0 about 60 s, one of N about 24 s, 98
    minutes in all.
22. **The figures, this machine's** (Task 5): this container is slower
    than the prototype's (`selfhost.rs` without the release runtime 40.9
    to 42.1 s at the last commit, against the prototype's 33.7 s), runs stable Rust
    1.97, Node v22.23.4, `clang-18` 18.1.3, on 4 CPUs, and its speed varied
    by a third between the replay's two days (the same `selfhost.rs` 63 s,
    then 43 s). N's live heap and stack were measured again at `4d0eca73`,
    whose Rust and Nova are the last commit's: the peaks and live heaps
    within 0.2 MB of ADR 0055 §13's, the times about half as long again;
    on 1 MB N survives `k` = 124 on both runtimes and the debug runtime
    overflows at 125, as before; the release runtime took `k` = 125 three
    times on the first day, failed it three times on the second, and took
    it in the second fact-check's run: at 124 and 125 N is within a few
    hundred bytes of the
    limit, which a run's environment moves; the least stacks 540 to 544 KB
    at `k` = 64 and 1,016 to 1,020 KB at 124, a step apart between runs.
    The stack was measured on both days, at `4d0eca73` and at `5bda68ce`,
    and once more by the second fact-check at `e7cfdd54`, each holding the
    last commit's `nova` and N. ADR 0055 §12's second table records it,
    the latest measurement, which every summary here follows: N survives
    `k` = 124 (7,936 levels); the debug runtime overflows at 125 (8,000),
    the release runtime at 125 or 126, borderline at 125.
    These are local
    figures only; CI's are decision 14's, measured, not projected.
23. **CI's lines for the PR** (decisions 11, 12 and 14): `selfhost.rs` in
    the Selfhost job is read on all three systems and compared with run
    42774093193's (37.10, 27.53, 55.38 s); past 70 s on Windows or 60 s on
    Linux or macOS, the user is asked. The Test job no longer runs it.
    The counts expected from the head, one test more in each than run
    42774093193 had: the Selfhost job's `selfhost.rs` 52 tests on Linux,
    50 on macOS and 46 on Windows (run 42774093193: 51, 49, 45; the
    128-level item test added); the Test job's sums `2831 passed, 10
    ignored` on Linux and `2808 passed, 9 ignored` on Windows (run
    42774093193: 2830 and 2807; step 0's resolver test added, which has
    no `cfg`). Here the Linux suite gives `2830 1 10`: the same 2,841
    tests, the IPv6 test failing for want of IPv6, which CI's runners
    have.

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-53a`: from `28f49f2e` (the
approved spec on `main`'s `2a67c6ce`), then this plan's commit, which adds
this file alone, so the patches apply to it as to `28f49f2e` (decision
19).

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-53a`.
  - `git log --format='%h %s' -3` prints the plan's commit, then
    `28f49f2e docs: the Phase 5.3 spec is approved` and `2a67c6ce 5.2b:
    the review's fixes`.
  - `git diff --stat 28f49f2e HEAD` lists only
    `docs/superpowers/plans/2026-11-10-phase-5-3a-the-type-checkers-items.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 5's review reads from it.
  - `df -h /` shows at least 6G free; if not, remove older duplicate test
    executables in `target/debug/deps` first. The replay began with 21G
    free.
- [ ] **Step 2: The tools and the process limit.**
  - `clang-18 --version | head -n 1` prints
    `Ubuntu clang version 18.1.3 (1ubuntu1)`: CI's pin, for the golden
    suite.
  - `node --version` prints `v22.23.4` here, for the WASM tests (any
    Node 22).
  - `RUSTUP_TOOLCHAIN=1.78.0 cargo --version` prints `cargo 1.78.0`, for
    Task 5.
  - `gdb --version` prints a version, for Task 5's GC pauses.
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
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-53a.txt" 2>&1
  grep '^test result' "$P/base-53a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  awk '/Running tests\/selfhost.rs/ {s = 1} s && /^test result/ {print; exit}' "$P/base-53a.txt"
  grep ' FAILED$' "$P/base-53a.txt"
  cp target/debug/nova "$P/nova-base-53a"
  cargo test --locked --workspace -- --list > "$P/base-list-53a.txt" 2>&1
  grep -c ': test$' "$P/base-list-53a.txt"
  cargo test --locked --workspace --test selfhost -- --list > "$P/base-list-sh-53a.txt" 2>&1
  ```

  Expected, from the replay on `28f49f2e`, in 348 s: `2869 1 10`;
  `selfhost.rs`'s line `test result: ok. 44 passed; 0 failed; …`, 44 of
  the 2869; and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. The list holds `2880` names, the run's 2869 + 1 + 10; the
  replay's `nova` was byte for byte the one the prototype's base build
  kept. The copy of `nova` is the base's, for Task 5's comparisons; the
  lists are the base's names. Task 5 expects `2830 1 10` from the whole
  suite, `selfhost.rs`'s 44 gone from it and 5 tests added, and 52 from
  `selfhost.rs` apart, 8 added.
- [ ] **Step 4: The ledger.** Start `$P/ledger-53a.md`, headed with the
  base commit, the baseline's figures and the process limit's form.


### Task 1: S0's `check_items`, the hidden HIR dump, std's item tables shared, and the cases

Spec §3.1 to §3.4, §3.6's cases; decisions 2, 3, 5, 6, 15, 16 and 20.

**Files:**
- Create: `crates/nova-typeck/src/check/items.rs`,
  `crates/nova-typeck/src/dump.rs`, the cases under
  `crates/nova-cli/tests/selfhost/cases/check/` (164 programs, 175 files),
  `cases/check-wasm/` (2) and `cases/check-test/` (1)
- Modify: `crates/nova-typeck/src/check.rs`, `crates/nova-typeck/src/lib.rs`,
  `crates/nova-resolver/src/lib.rs`, `crates/nova-resolver/src/dump.rs`,
  `crates/nova-driver/src/lib.rs` (`HirDumps`, `dump_std_hir`, the unit
  tests `resolution_dumps`), `crates/nova-cli/src/cmd/run.rs`,
  `crates/nova-cli/src/main.rs`

- [ ] **Step 1: The tests.** `patch_block t1-tests`:
  - `items.rs` with its tests alone, and `check.rs`'s `mod items;`:
    `check_items_is_the_prefix_of_check_with` (four programs: the item
    diagnostics are `check_with`'s first ones, and its tables the module's)
    and `shared_items_check_what_the_full_check_does` (a std of one more
    module, a trait, two impls and an extern, which a program's impls pair
    with: E0074 and E0105 printed alike, every table of the shared check
    the full check's, and a std with an item diagnostic not shareable);
  - the resolver's `std_shared_for_check_defines_what_the_full_path_does`:
    the shared path for the type checker gives a program std's
    definitions, module scopes and items' modules as the full path does;
  - the driver's `resolution_dumps` module:
    `std_checked_once_dumps_what_std_checked_for_each_program_dumps` (S0's
    shared and full HIR dumps over every corpus root and case of each
    mode, in order and reversed) and `std_part_of_a_programs_check_is_std_part`;
  - the cases: 154 extracted from the type checker's item-level tests
    (`<testfile>__<test>[_n]`), 10 by hand under `check` (every item E0001
    site, every E0012 form, unsupported types and an extern's `where`, the
    array `len` and inherent overlaps, the import fix and did-you-mean, and
    step 0's five), 2 for WASM and 1 under test mode.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished` and one warning, `unused import: super::*` in `items.rs`,
  which holds only its tests until Step 3.
  - `cargo test --locked --workspace --lib -- items::tests`, then
    `cargo test --locked --workspace --lib -- resolution_dumps`, then
    `cargo test --locked --workspace --lib -- std_shared_for_check`.
    Expected: no test runs, as three crates' library tests do not compile:
    `nova-resolver`'s, 2 errors: `E0425`
    (`resolve_program_with_shared_std_for_check`) and `E0599` (no
    `SharedStd::shareable`); `nova-typeck`'s, 16 errors: `E0425`
    (`check_items` twice, `check_items_shared`,
    `resolve_program_with_shared_std_for_check`, `sorted` five times,
    `ItemTables` twice), `E0433` (`crate::dump`, `SharedItems` twice) and
    `E0599` (no `alone`, no `shareable`); and `nova-driver`'s, 6 errors:
    `E0425` (`nova_typeck::check_items`, `dump_std_hir`, `HirDumps`) and
    `E0433` (`nova_typeck::dump`, `HirDumps` twice). Each run builds every
    library's tests, so it reports the crate or crates cargo reaches
    before it stops, which vary between runs (the replay's three runs
    named `nova-resolver`, `nova-typeck` and `nova-driver`, one each; a
    later check's named `nova-driver`, `nova-driver`, then `nova-driver`
    and `nova-typeck`); what holds either way is that each run ends with
    `error: could not compile` for one or more of the three and none runs
    a test.
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`items.rs`:** `ItemTables`, `check_items` (passes 1 to 5),
    `SharedItems`, `check_items_shared`, `Rebase`, `Splice` and
    `Checker::splice`: std's part of each table added, rebased, where the
    full pass collects it; `Checker::own`;
  - **`check.rs`:** `check_with` calls `check_items` and restores the
    checker from its tables; the passes skip std's items when shared
    (`own`) and call `splice`; `FnSig`'s fields `pub(crate)`;
  - **`dump.rs` (typeck):** `dump_program`, `dump_std`, `std_part_of`,
    the printer, `KINDS` and `LATER_KINDS`;
  - **`lib.rs` (resolver):** `SharedStd::alone` and `shareable`,
    `resolve_program_with_shared_std_for_check`, `Def::rebased`,
    `ModuleScope::rebased`; **`dump.rs` (resolver):** `std_start`,
    `write_diagnostics`, `write_place`;
  - **`lib.rs` (driver):** `HirDumps` (`new`, `unshared`, `dump`),
    `dump_std_hir`, `Resolve::SharedForCheck`, `hir_dump`;
  - **`run.rs` and `main.rs`:** `DumpCmd` and `DumpKind`: `check
    --dump-hir` first after `check` takes the resolution dump's parser
    under the usage `nova check --dump-hir`; every other command line goes
    to clap as before.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --lib -- items::tests resolution_dumps
    std_shared_for_check`. Expected: three `test result` lines with tests
    in them: `nova_driver`'s `4 passed; 0 failed; …; 37 filtered out` (the
    two new and 5.2b's two), `nova_resolver`'s `1 passed; 0 failed; …; 69
    filtered out`, `nova_typeck`'s `2 passed; 0 failed; …; 585 filtered
    out`.
  - `S` (271 s). Expected: `2874 1 10`, the IPv6 test the one failure;
    `selfhost.rs`'s line among them, `44 passed`, as at the base.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `nova-typeck: check_items, the hidden HIR dump,
      std's item tables shared, and the item cases`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost/cases/check-test/tests_beside_items_and_an_item_error.nova b/crates/nova-cli/tests/selfhost/cases/check-test/tests_beside_items_and_an_item_error.nova
new file mode 100644
index 00000000..c8ef90a0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check-test/tests_beside_items_and_an_item_error.nova
@@ -0,0 +1,12 @@
+record P { x: Int }
+impl Eq for P {
+    fn eq(self, other: P) -> Bool { true }
+}
+fn helper<T: Eq>(a: T, b: T) -> Bool { a == b }
+@test
+fn compares() {
+    assert(helper(P { x: 1 }, P { x: 1 }))
+}
+@test(should_panic)
+fn panics() { panic("no") }
+impl Missing for P {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check-wasm/a_component_returns_std_uis_view.nova b/crates/nova-cli/tests/selfhost/cases/check-wasm/a_component_returns_std_uis_view.nova
new file mode 100644
index 00000000..08a44094
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check-wasm/a_component_returns_std_uis_view.nova
@@ -0,0 +1,6 @@
+import ui
+record Page { title: String, count: Int }
+component card(p: Page) {
+    view { text(p.title) }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check-wasm/every_item_e0092_site.nova b/crates/nova-cli/tests/selfhost/cases/check-wasm/every_item_e0092_site.nova
new file mode 100644
index 00000000..e5577c86
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check-wasm/every_item_e0092_site.nova
@@ -0,0 +1,7 @@
+import ui
+trait Reads: Read {}
+impl Write for Int {}
+fn bounded<T: Read>(x: T) {}
+fn clause<T>(x: T) where T: Write {}
+fn typed(f: File) -> IoError { panic("no") }
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/an_array_len_and_two_inherent_impls_overlap.nova b/crates/nova-cli/tests/selfhost/cases/check/an_array_len_and_two_inherent_impls_overlap.nova
new file mode 100644
index 00000000..9e295081
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/an_array_len_and_two_inherent_impls_overlap.nova
@@ -0,0 +1,9 @@
+impl [Int] {
+    fn len(self) -> Int { 0 }
+}
+impl [Bool] {
+    fn first(self) -> Bool { true }
+}
+impl [Bool] {
+    fn first(self) -> Bool { false }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_longer_than_the_depth_limit_is_a_diagnostic_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_longer_than_the_depth_limit_is_a_diagnostic_1.nova
new file mode 100644
index 00000000..d1c0c8b2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_longer_than_the_depth_limit_is_a_diagnostic_1.nova
@@ -0,0 +1,136 @@
+record Pair<A, B> { a: A
+ b: B }
+trait Chain {
+ type A0
+ type A1
+ type A2
+ type A3
+ type A4
+ type A5
+ type A6
+ type A7
+ type A8
+ type A9
+ type A10
+ type A11
+ type A12
+ type A13
+ type A14
+ type A15
+ type A16
+ type A17
+ type A18
+ type A19
+ type A20
+ type A21
+ type A22
+ type A23
+ type A24
+ type A25
+ type A26
+ type A27
+ type A28
+ type A29
+ type A30
+ type A31
+ type A32
+ type A33
+ type A34
+ type A35
+ type A36
+ type A37
+ type A38
+ type A39
+ type A40
+ type A41
+ type A42
+ type A43
+ type A44
+ type A45
+ type A46
+ type A47
+ type A48
+ type A49
+ type A50
+ type A51
+ type A52
+ type A53
+ type A54
+ type A55
+ type A56
+ type A57
+ type A58
+ type A59
+ type A60
+ type A61
+ type A62
+ type A63
+ fn put(self, y: Self::A0) -> Int }
+record W { v: Int }
+impl Chain for W {
+ type A0 = Self::A1
+ type A1 = Self::A2
+ type A2 = Self::A3
+ type A3 = Self::A4
+ type A4 = Self::A5
+ type A5 = Self::A6
+ type A6 = Self::A7
+ type A7 = Self::A8
+ type A8 = Self::A9
+ type A9 = Self::A10
+ type A10 = Self::A11
+ type A11 = Self::A12
+ type A12 = Self::A13
+ type A13 = Self::A14
+ type A14 = Self::A15
+ type A15 = Self::A16
+ type A16 = Self::A17
+ type A17 = Self::A18
+ type A18 = Self::A19
+ type A19 = Self::A20
+ type A20 = Self::A21
+ type A21 = Self::A22
+ type A22 = Self::A23
+ type A23 = Self::A24
+ type A24 = Self::A25
+ type A25 = Self::A26
+ type A26 = Self::A27
+ type A27 = Self::A28
+ type A28 = Self::A29
+ type A29 = Self::A30
+ type A30 = Self::A31
+ type A31 = Self::A32
+ type A32 = Self::A33
+ type A33 = Self::A34
+ type A34 = Self::A35
+ type A35 = Self::A36
+ type A36 = Self::A37
+ type A37 = Self::A38
+ type A38 = Self::A39
+ type A39 = Self::A40
+ type A40 = Self::A41
+ type A41 = Self::A42
+ type A42 = Self::A43
+ type A43 = Self::A44
+ type A44 = Self::A45
+ type A45 = Self::A46
+ type A46 = Self::A47
+ type A47 = Self::A48
+ type A48 = Self::A49
+ type A49 = Self::A50
+ type A50 = Self::A51
+ type A51 = Self::A52
+ type A52 = Self::A53
+ type A53 = Self::A54
+ type A54 = Self::A55
+ type A55 = Self::A56
+ type A56 = Self::A57
+ type A57 = Self::A58
+ type A58 = Self::A59
+ type A59 = Self::A60
+ type A60 = Self::A61
+ type A61 = Self::A62
+ type A62 = Self::A63
+ type A63 = Int
+fn put(self, y: Self::A0) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_longer_than_the_depth_limit_is_a_diagnostic_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_longer_than_the_depth_limit_is_a_diagnostic_2.nova
new file mode 100644
index 00000000..2c782e73
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_longer_than_the_depth_limit_is_a_diagnostic_2.nova
@@ -0,0 +1,138 @@
+record Pair<A, B> { a: A
+ b: B }
+trait Chain {
+ type A0
+ type A1
+ type A2
+ type A3
+ type A4
+ type A5
+ type A6
+ type A7
+ type A8
+ type A9
+ type A10
+ type A11
+ type A12
+ type A13
+ type A14
+ type A15
+ type A16
+ type A17
+ type A18
+ type A19
+ type A20
+ type A21
+ type A22
+ type A23
+ type A24
+ type A25
+ type A26
+ type A27
+ type A28
+ type A29
+ type A30
+ type A31
+ type A32
+ type A33
+ type A34
+ type A35
+ type A36
+ type A37
+ type A38
+ type A39
+ type A40
+ type A41
+ type A42
+ type A43
+ type A44
+ type A45
+ type A46
+ type A47
+ type A48
+ type A49
+ type A50
+ type A51
+ type A52
+ type A53
+ type A54
+ type A55
+ type A56
+ type A57
+ type A58
+ type A59
+ type A60
+ type A61
+ type A62
+ type A63
+ type A64
+ fn put(self, y: Self::A0) -> Int }
+record W { v: Int }
+impl Chain for W {
+ type A0 = Self::A1
+ type A1 = Self::A2
+ type A2 = Self::A3
+ type A3 = Self::A4
+ type A4 = Self::A5
+ type A5 = Self::A6
+ type A6 = Self::A7
+ type A7 = Self::A8
+ type A8 = Self::A9
+ type A9 = Self::A10
+ type A10 = Self::A11
+ type A11 = Self::A12
+ type A12 = Self::A13
+ type A13 = Self::A14
+ type A14 = Self::A15
+ type A15 = Self::A16
+ type A16 = Self::A17
+ type A17 = Self::A18
+ type A18 = Self::A19
+ type A19 = Self::A20
+ type A20 = Self::A21
+ type A21 = Self::A22
+ type A22 = Self::A23
+ type A23 = Self::A24
+ type A24 = Self::A25
+ type A25 = Self::A26
+ type A26 = Self::A27
+ type A27 = Self::A28
+ type A28 = Self::A29
+ type A29 = Self::A30
+ type A30 = Self::A31
+ type A31 = Self::A32
+ type A32 = Self::A33
+ type A33 = Self::A34
+ type A34 = Self::A35
+ type A35 = Self::A36
+ type A36 = Self::A37
+ type A37 = Self::A38
+ type A38 = Self::A39
+ type A39 = Self::A40
+ type A40 = Self::A41
+ type A41 = Self::A42
+ type A42 = Self::A43
+ type A43 = Self::A44
+ type A44 = Self::A45
+ type A45 = Self::A46
+ type A46 = Self::A47
+ type A47 = Self::A48
+ type A48 = Self::A49
+ type A49 = Self::A50
+ type A50 = Self::A51
+ type A51 = Self::A52
+ type A52 = Self::A53
+ type A53 = Self::A54
+ type A54 = Self::A55
+ type A55 = Self::A56
+ type A56 = Self::A57
+ type A57 = Self::A58
+ type A58 = Self::A59
+ type A59 = Self::A60
+ type A60 = Self::A61
+ type A61 = Self::A62
+ type A62 = Self::A63
+ type A63 = Self::A64
+ type A64 = Int
+fn put(self, y: Self::A0) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_that_resolves_to_an_enormous_type_is_a_diagnostic_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_that_resolves_to_an_enormous_type_is_a_diagnostic_1.nova
new file mode 100644
index 00000000..a7e8447a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_that_resolves_to_an_enormous_type_is_a_diagnostic_1.nova
@@ -0,0 +1,42 @@
+record Pair<A, B> { a: A
+ b: B }
+trait Chain {
+ type A0
+ type A1
+ type A2
+ type A3
+ type A4
+ type A5
+ type A6
+ type A7
+ type A8
+ type A9
+ type A10
+ type A11
+ type A12
+ type A13
+ type A14
+ type A15
+ type A16
+ fn put(self, y: Self::A0) -> Int }
+record W { v: Int }
+impl Chain for W {
+ type A0 = Pair<Self::A1, Self::A1>
+ type A1 = Pair<Self::A2, Self::A2>
+ type A2 = Pair<Self::A3, Self::A3>
+ type A3 = Pair<Self::A4, Self::A4>
+ type A4 = Pair<Self::A5, Self::A5>
+ type A5 = Pair<Self::A6, Self::A6>
+ type A6 = Pair<Self::A7, Self::A7>
+ type A7 = Pair<Self::A8, Self::A8>
+ type A8 = Pair<Self::A9, Self::A9>
+ type A9 = Pair<Self::A10, Self::A10>
+ type A10 = Pair<Self::A11, Self::A11>
+ type A11 = Pair<Self::A12, Self::A12>
+ type A12 = Pair<Self::A13, Self::A13>
+ type A13 = Pair<Self::A14, Self::A14>
+ type A14 = Pair<Self::A15, Self::A15>
+ type A15 = Pair<Self::A16, Self::A16>
+ type A16 = Int
+fn put(self, y: Self::A0) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_that_resolves_to_an_enormous_type_is_a_diagnostic_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_that_resolves_to_an_enormous_type_is_a_diagnostic_2.nova
new file mode 100644
index 00000000..cfeefd08
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_binding_chain_that_resolves_to_an_enormous_type_is_a_diagnostic_2.nova
@@ -0,0 +1,18 @@
+record Pair<A, B> { a: A
+ b: B }
+trait Chain {
+ type A0
+ type A1
+ type A2
+ type A3
+ type A4
+ fn put(self, y: Self::A0) -> Int }
+record W { v: Int }
+impl Chain for W {
+ type A0 = Pair<Self::A1, Self::A1>
+ type A1 = Pair<Self::A2, Self::A2>
+ type A2 = Pair<Self::A3, Self::A3>
+ type A3 = Pair<Self::A4, Self::A4>
+ type A4 = Int
+fn put(self, y: Self::A0) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_bound_on_a_sum_type_parameter_is_still_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_bound_on_a_sum_type_parameter_is_still_e0900.nova
new file mode 100644
index 00000000..f256d4a6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_bound_on_a_sum_type_parameter_is_still_e0900.nova
@@ -0,0 +1,4 @@
+trait It { type Item
+ fn next(mut self) -> Option<Self::Item> }
+type S<I: It> = | A(I) | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_bound_on_an_associated_type_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_bound_on_an_associated_type_reports_e0900.nova
new file mode 100644
index 00000000..63a914ac
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_bound_on_an_associated_type_reports_e0900.nova
@@ -0,0 +1,2 @@
+trait It { type Item: Display }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_cyclic_associated_type_binding_nested_in_a_compound_type_is_rejected.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_cyclic_associated_type_binding_nested_in_a_compound_type_is_rejected.nova
new file mode 100644
index 00000000..71c335a0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_cyclic_associated_type_binding_nested_in_a_compound_type_is_rejected.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Int }
+record W { v: Int }
+impl It for W { type Item = [Self::Item]
+ fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_cyclic_associated_type_binding_nested_in_a_future_is_rejected.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_cyclic_associated_type_binding_nested_in_a_future_is_rejected.nova
new file mode 100644
index 00000000..6c4056bc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_cyclic_associated_type_binding_nested_in_a_future_is_rejected.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Int }
+record W { v: Int }
+impl It for W { type Item = Future<Self::Item>
+ fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_genuinely_wrong_impl_signature_still_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_genuinely_wrong_impl_signature_still_reports_e0072.nova
new file mode 100644
index 00000000..367e94b8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_genuinely_wrong_impl_signature_still_reports_e0072.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get_item(self) -> Self::Item }
+record W<T> { v: T }
+impl<T> It for W<T> { type Item = T
+ fn get_item(self) -> Bool { true } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_module_qualified_type_path_still_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_module_qualified_type_path_still_reports_e0900.nova
new file mode 100644
index 00000000..f359b7b9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_module_qualified_type_path_still_reports_e0900.nova
@@ -0,0 +1,2 @@
+fn f(x: some_mod::Thing) -> Int { 1 }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_mutually_recursive_pair_of_associated_type_bindings_is_rejected.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_mutually_recursive_pair_of_associated_type_bindings_is_rejected.nova
new file mode 100644
index 00000000..0711ea7d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_mutually_recursive_pair_of_associated_type_bindings_is_rejected.nova
@@ -0,0 +1,8 @@
+trait Two { type A
+ type B
+ fn get(self) -> Int }
+record W { v: Int }
+impl Two for W { type A = Self::B
+ type B = Self::A
+fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_parameter_named_self_is_rejected_but_still_scoped.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_parameter_named_self_is_rejected_but_still_scoped.nova
new file mode 100644
index 00000000..2fc0f9aa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_parameter_named_self_is_rejected_but_still_scoped.nova
@@ -0,0 +1,2 @@
+record W<Self> { v: Self }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_ambiguous_between_two_bounds_is_an_error.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_ambiguous_between_two_bounds_is_an_error.nova
new file mode 100644
index 00000000..128c1129
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_ambiguous_between_two_bounds_is_an_error.nova
@@ -0,0 +1,6 @@
+trait A { type Item
+ fn a(self) -> Int }
+trait B { type Item
+ fn b(self) -> Int }
+fn f<I: A + B>(x: I) -> I::Item { panic("unreachable") }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_1.nova
new file mode 100644
index 00000000..59f796d1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_1.nova
@@ -0,0 +1,7 @@
+trait It { type Item
+ fn g(self) -> Int }
+trait Tr { fn h(self) -> Int }
+record W<T> { v: T }
+impl<T: It> Tr for W<T::Item> { fn h(self) -> Int { 1 } }
+impl Tr for W<Int> { fn h(self) -> Int { 2 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_2.nova
new file mode 100644
index 00000000..3ef5baaa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_2.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn g(self) -> Int }
+trait Tr { fn h(self) -> Int }
+record W<T> { v: T }
+impl<T: It> Tr for W<T::Item> { fn h(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_3.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_3.nova
new file mode 100644
index 00000000..3e2def5f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_3.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn g(self) -> Int }
+trait Tr { fn h(self) -> Int }
+record W<T> { v: T }
+impl<T: It> Tr for T::Item { fn h(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_4.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_4.nova
new file mode 100644
index 00000000..0121de61
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_in_an_impls_self_type_is_rejected_4.nova
@@ -0,0 +1,7 @@
+trait It { type Item
+ fn g(self) -> Int }
+trait Tr { fn h(self) -> Int }
+record W<T> { v: T }
+impl<T> Tr for W<T> { fn h(self) -> Int { 1 } }
+impl Tr for W<Int> { fn h(self) -> Int { 2 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_naming_an_undeclared_associated_type_is_an_error.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_naming_an_undeclared_associated_type_is_an_error.nova
new file mode 100644
index 00000000..f13276cb
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_naming_an_undeclared_associated_type_is_an_error.nova
@@ -0,0 +1,4 @@
+trait It { type Item
+ fn get(self) -> Int }
+fn f<I: It>(x: I) -> I::Nope { panic("unreachable") }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_on_an_unbounded_record_parameter_is_still_e0001.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_on_an_unbounded_record_parameter_is_still_e0001.nova
new file mode 100644
index 00000000..35d09872
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_on_an_unbounded_record_parameter_is_still_e0001.nova
@@ -0,0 +1,4 @@
+trait It { type Item
+ fn next(mut self) -> Option<Self::Item> }
+record M<I, U> { it: I, f: fn(I::Item) -> U }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_projects_onto_the_correct_parameter_when_there_are_several.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_projects_onto_the_correct_parameter_when_there_are_several.nova
new file mode 100644
index 00000000..dae81b46
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_projects_onto_the_correct_parameter_when_there_are_several.nova
@@ -0,0 +1,4 @@
+trait It { type Item
+ fn get(self) -> Int }
+fn f<T, I: It>(x: T, y: I) -> I::Item { panic("unreachable") }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_resolves_against_its_own_parameters_bounds_not_every_parameters.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_resolves_against_its_own_parameters_bounds_not_every_parameters.nova
new file mode 100644
index 00000000..d3fe9c5f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_resolves_against_its_own_parameters_bounds_not_every_parameters.nova
@@ -0,0 +1,6 @@
+trait Foo { type Item
+ fn f(self) -> Int }
+trait It { type Other
+ fn get(self) -> Int }
+fn f<A: Foo, B: It>(x: A, y: B) -> A::Other { panic("x") }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_resolves_to_the_bounding_traits_own_associated_type_not_an_unrelated_one.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_resolves_to_the_bounding_traits_own_associated_type_not_an_unrelated_one.nova
new file mode 100644
index 00000000..02d6f141
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_resolves_to_the_bounding_traits_own_associated_type_not_an_unrelated_one.nova
@@ -0,0 +1,6 @@
+trait Other { type Item
+ fn o(self) -> Int }
+trait Keeper { type Item
+ fn get(self) -> Int }
+fn first<I: Keeper>(x: I) -> I::Item { panic("unreachable") }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_with_type_arguments_reports_e0012.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_with_type_arguments_reports_e0012.nova
new file mode 100644
index 00000000..8c9ea00a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_projection_with_type_arguments_reports_e0012.nova
@@ -0,0 +1,4 @@
+trait It { type Item
+ fn get(self) -> Int }
+fn first<I: It>(x: I) -> I::Item<Int> { panic("unreachable") }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_record_bound_naming_an_unknown_trait_is_e0001.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_record_bound_naming_an_unknown_trait_is_e0001.nova
new file mode 100644
index 00000000..f5ebd9b9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_record_bound_naming_an_unknown_trait_is_e0001.nova
@@ -0,0 +1,2 @@
+record M<I: NoSuchTrait> { it: I }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_reported_binding_cycle_is_poisoned_and_does_not_cascade.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_reported_binding_cycle_is_poisoned_and_does_not_cascade.nova
new file mode 100644
index 00000000..65a58779
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_reported_binding_cycle_is_poisoned_and_does_not_cascade.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Self::Item }
+record W { v: Int }
+impl It for W { type Item = Self::Item
+fn get(self) -> Int { 1 } }
+fn main() { println("${W { v: 1 }.get()}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_required_methods_own_signature_resolves_its_own_traits_self_item.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_required_methods_own_signature_resolves_its_own_traits_self_item.nova
new file mode 100644
index 00000000..55026579
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_required_methods_own_signature_resolves_its_own_traits_self_item.nova
@@ -0,0 +1,3 @@
+trait It { type Item
+ fn get(self) -> Self::Item }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_self_referential_associated_type_binding_is_rejected.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_self_referential_associated_type_binding_is_rejected.nova
new file mode 100644
index 00000000..5af66192
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_self_referential_associated_type_binding_is_rejected.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Self::Item }
+record W { v: Int }
+impl It for W { type Item = Self::Item
+fn get(self) -> Self::Item { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_1.nova
new file mode 100644
index 00000000..233410f3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_1.nova
@@ -0,0 +1,7 @@
+trait It { type Item
+ type Item
+ fn g(self) -> Int }
+record W { v: Int }
+impl It for W { type Item = Int
+ fn g(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_2.nova
new file mode 100644
index 00000000..328f57a1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_2.nova
@@ -0,0 +1,5 @@
+trait It { fn g(self) -> Int
+ fn g(self) -> Bool }
+record W { v: Int }
+impl It for W { fn g(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_3.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_3.nova
new file mode 100644
index 00000000..47f391b4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_3.nova
@@ -0,0 +1,5 @@
+trait It { fn g(self) -> Int
+ fn g() -> Int }
+record W { v: Int }
+impl It for W { fn g(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_4.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_4.nova
new file mode 100644
index 00000000..5818b878
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_4.nova
@@ -0,0 +1,10 @@
+trait It { type A
+ type B
+ fn g(self) -> Self::A
+ fn h(self) -> Self::B }
+record W { v: Int }
+impl It for W { type A = Int
+ type B = Bool
+fn g(self) -> Int { 1 }
+ fn h(self) -> Bool { true } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_5.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_5.nova
new file mode 100644
index 00000000..50790e6c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_may_not_declare_one_name_twice_5.nova
@@ -0,0 +1,9 @@
+trait It { type A
+ type A
+ type B
+fn g(self) -> Self::B }
+record W { v: Int }
+impl It for W { type A = Int
+ type B = Bool
+fn g(self) -> Bool { true } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_records_its_associated_types_in_order.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_records_its_associated_types_in_order.nova
new file mode 100644
index 00000000..1a7d287d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_trait_records_its_associated_types_in_order.nova
@@ -0,0 +1,4 @@
+trait Pair { type A
+ type B
+ fn get(self) -> Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_1.nova
new file mode 100644
index 00000000..f82457ac
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_1.nova
@@ -0,0 +1,2 @@
+fn f<Self>(x: Self) -> Int { 1 }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_3.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_3.nova
new file mode 100644
index 00000000..9c6bc512
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_3.nova
@@ -0,0 +1,2 @@
+type S<Self> = | A(Self)
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_4.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_4.nova
new file mode 100644
index 00000000..eae5df3b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_4.nova
@@ -0,0 +1,2 @@
+trait Q<Self> { fn q(self) -> Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_5.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_5.nova
new file mode 100644
index 00000000..c0535cd5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_5.nova
@@ -0,0 +1,2 @@
+trait Q { fn q<Self>(self, x: Self) -> Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_6.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_6.nova
new file mode 100644
index 00000000..17dd27d5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_6.nova
@@ -0,0 +1,3 @@
+record W<T> { v: T }
+impl<Self> W<Self> { fn m(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_7.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_7.nova
new file mode 100644
index 00000000..c7525240
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_type_parameter_named_self_is_rejected_at_every_generic_declaration_7.nova
@@ -0,0 +1,3 @@
+record W { v: Int }
+impl W { fn m<Self>(self, x: Self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__a_user_written_self_type_parameter_is_rejected_in_an_impl.nova b/crates/nova-cli/tests/selfhost/cases/check/check__a_user_written_self_type_parameter_is_rejected_in_an_impl.nova
new file mode 100644
index 00000000..75c3008d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__a_user_written_self_type_parameter_is_rejected_in_an_impl.nova
@@ -0,0 +1,5 @@
+trait It { type Item
+ fn get(self) -> Int }
+record W<T> { v: T }
+impl<Self: It> W<Self> { fn peek(self) -> Self::Item { panic("x") } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_binding_an_undeclared_associated_type_reports_e0071.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_binding_an_undeclared_associated_type_reports_e0071.nova
new file mode 100644
index 00000000..dd5974ca
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_binding_an_undeclared_associated_type_reports_e0071.nova
@@ -0,0 +1,7 @@
+trait It { type Item
+ fn get(self) -> Int }
+record W { v: Int }
+impl It for W { type Item = Int
+ type Extra = Bool
+ fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_binds_its_associated_type.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_binds_its_associated_type.nova
new file mode 100644
index 00000000..a07a5fe4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_binds_its_associated_type.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Int }
+record W<T> { v: T }
+impl<T> It for W<T> { type Item = T
+ fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_level_const_is_rejected_rather_than_silently_dropped.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_level_const_is_rejected_rather_than_silently_dropped.nova
new file mode 100644
index 00000000..2e218f95
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_level_const_is_rejected_rather_than_silently_dropped.nova
@@ -0,0 +1,4 @@
+record K { v: Int }
+impl K { const LIMIT: Int = 99
+ fn get(self) -> Int { self.v } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_1.nova
new file mode 100644
index 00000000..4959b708
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_1.nova
@@ -0,0 +1,8 @@
+trait Base { type Elem
+ fn base(self) -> Int }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W { v: Int }
+impl Base for W { type Elem = Int
+ fn base(self) -> Int { 1 } }
+impl Ext for W { fn peek(self) -> Self::Elem { 5 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_2.nova
new file mode 100644
index 00000000..4694928b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_2.nova
@@ -0,0 +1,8 @@
+trait Base { type Elem
+ fn base(self) -> Int }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W { v: Int }
+impl Base for W { type Elem = Int
+ fn base(self) -> Int { 1 } }
+impl Ext for W { fn peek(self) -> Bool { true } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_3.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_3.nova
new file mode 100644
index 00000000..d6c2aef6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_3.nova
@@ -0,0 +1,8 @@
+trait Base { type Elem
+ fn base(self) -> Int }
+trait Ext: Base { fn peek(self) -> Self::Nope }
+record W { v: Int }
+impl Base for W { type Elem = Int
+ fn base(self) -> Int { 1 } }
+impl Ext for W { fn peek(self) -> Self::Nope { 5 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_4.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_4.nova
new file mode 100644
index 00000000..76a66b71
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_may_echo_an_associated_type_inherited_from_a_supertrait_4.nova
@@ -0,0 +1,10 @@
+trait Base { type Elem
+ fn base(self) -> Int }
+trait Ext: Base { type Elem
+ fn peek(self) -> Self::Elem }
+record W { v: Int }
+impl Base for W { type Elem = Int
+ fn base(self) -> Int { 1 } }
+impl Ext for W { type Elem = Int
+ fn peek(self) -> Self::Elem { 5 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_missing_an_associated_type_reports_e0070.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_missing_an_associated_type_reports_e0070.nova
new file mode 100644
index 00000000..407b998c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_impl_missing_an_associated_type_reports_e0070.nova
@@ -0,0 +1,5 @@
+trait It { type Item
+ fn get(self) -> Int }
+record W { v: Int }
+impl It for W { fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_inherent_impl_cannot_bind_an_associated_type.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_inherent_impl_cannot_bind_an_associated_type.nova
new file mode 100644
index 00000000..e7c366da
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_inherent_impl_cannot_bind_an_associated_type.nova
@@ -0,0 +1,4 @@
+record W { v: Int }
+impl W { type Item = Int
+ fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__an_unknown_type_name_is_still_e0001_not_e0089.nova b/crates/nova-cli/tests/selfhost/cases/check/check__an_unknown_type_name_is_still_e0001_not_e0089.nova
new file mode 100644
index 00000000..b2904021
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__an_unknown_type_name_is_still_e0001_not_e0089.nova
@@ -0,0 +1,2 @@
+fn f(x: Nope) -> Int { 1 }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__associated_function_on_generic_impl_resolves_type_arg.nova b/crates/nova-cli/tests/selfhost/cases/check/check__associated_function_on_generic_impl_resolves_type_arg.nova
new file mode 100644
index 00000000..18b7a30e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__associated_function_on_generic_impl_resolves_type_arg.nova
@@ -0,0 +1,6 @@
+record Box<T> { value: T }
+impl<T> Box<T> { fn make(v: T) -> Box<T> { Box { value: v } } }
+fn main() {
+let b = Box::make(5)
+println("${b.value}")
+}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__async_default_trait_method_reports_e0900_once.nova b/crates/nova-cli/tests/selfhost/cases/check/check__async_default_trait_method_reports_e0900_once.nova
new file mode 100644
index 00000000..7be30876
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__async_default_trait_method_reports_e0900_once.nova
@@ -0,0 +1,4 @@
+trait Foo { async fn bar(self) -> Int { 7 } }
+record Thing { v: Int }
+impl Foo for Thing { fn bar(self) -> Int { self.v } }
+fn main() { let t = Thing { v: 1 } }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__async_extern_fn_still_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__async_extern_fn_still_reports_e0900.nova
new file mode 100644
index 00000000..a972b1f9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__async_extern_fn_still_reports_e0900.nova
@@ -0,0 +1,2 @@
+extern "C" { async fn c_thing() -> Int }
+fn main() {}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__async_trait_impl_method_is_refused_as_a_return_type_mismatch.nova b/crates/nova-cli/tests/selfhost/cases/check/check__async_trait_impl_method_is_refused_as_a_return_type_mismatch.nova
new file mode 100644
index 00000000..0c9f3051
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__async_trait_impl_method_is_refused_as_a_return_type_mismatch.nova
@@ -0,0 +1,4 @@
+trait T { fn m(self) -> Int }
+record C { v: Int }
+impl T for C { async fn m(self) -> Int { self.v } }
+fn main() {}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__async_trait_method_declaration_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__async_trait_method_declaration_reports_e0900.nova
new file mode 100644
index 00000000..eb595c31
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__async_trait_method_declaration_reports_e0900.nova
@@ -0,0 +1,4 @@
+trait Foo { async fn bar(self) -> Int }
+record Thing { v: Int }
+impl Foo for Thing { fn bar(self) -> Int { self.v } }
+fn main() { let t = Thing { v: 1 } }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__async_trait_method_still_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__async_trait_method_still_reports_e0900.nova
new file mode 100644
index 00000000..126e0fdf
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__async_trait_method_still_reports_e0900.nova
@@ -0,0 +1,2 @@
+trait T { async fn m(self) -> Int }
+fn main() {}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__bare_future_without_a_type_argument_is_rejected.nova b/crates/nova-cli/tests/selfhost/cases/check/check__bare_future_without_a_type_argument_is_rejected.nova
new file mode 100644
index 00000000..b7f05702
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__bare_future_without_a_type_argument_is_rejected.nova
@@ -0,0 +1,2 @@
+fn f(x: Future) -> Int { 1 }
+fn main() {}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__binding_the_same_associated_type_twice_is_rejected.nova b/crates/nova-cli/tests/selfhost/cases/check/check__binding_the_same_associated_type_twice_is_rejected.nova
new file mode 100644
index 00000000..6e721d4a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__binding_the_same_associated_type_twice_is_rejected.nova
@@ -0,0 +1,7 @@
+trait It { type Item
+ fn get(self) -> Int }
+record W { v: Int }
+impl It for W { type Item = Int
+ type Item = Bool
+ fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_compares_the_associated_type_sets_not_their_sizes.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_compares_the_associated_type_sets_not_their_sizes.nova
new file mode 100644
index 00000000..a735ccdb
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_compares_the_associated_type_sets_not_their_sizes.nova
@@ -0,0 +1,8 @@
+trait Pair { type A
+ type B
+ fn get(self) -> Int }
+record W { v: Int }
+impl Pair for W { type B = Int
+ type C = Bool
+ fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_a_selfless_methods_projection_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_a_selfless_methods_projection_1.nova
new file mode 100644
index 00000000..7586359e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_a_selfless_methods_projection_1.nova
@@ -0,0 +1,8 @@
+trait Zero { type Out
+fn zero() -> Self::Out
+fn of(x: Self::Out) -> Self::Out }
+record P { v: Int }
+impl Zero for P { type Out = Int
+fn zero() -> Int { 0 }
+fn of(x: Self::Out) -> Self::Out { x } }
+fn main() { println("${P::of(P::zero())}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_a_selfless_methods_projection_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_a_selfless_methods_projection_2.nova
new file mode 100644
index 00000000..9f913820
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_a_selfless_methods_projection_2.nova
@@ -0,0 +1,8 @@
+trait Zero { type Out
+fn zero() -> Self::Out
+fn of(x: Self::Out) -> Self::Out }
+record P { v: Int }
+impl Zero for P { type Out = Int
+fn zero() -> Int { 0 }
+fn of(x: Int) -> Int { x } }
+fn main() { println("${P::of(P::zero())}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_a_selfless_methods_projection_3.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_a_selfless_methods_projection_3.nova
new file mode 100644
index 00000000..3a03a69b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_a_selfless_methods_projection_3.nova
@@ -0,0 +1,8 @@
+trait Zero { type Out
+fn zero() -> Self::Out
+fn of(x: Self::Out) -> Self::Out }
+record P { v: Int }
+impl Zero for P { type Out = Int
+fn zero() -> Int { 0 }
+fn of(x: Bool) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_inside_a_compound_type_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_inside_a_compound_type_1.nova
new file mode 100644
index 00000000..a5c4056f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_inside_a_compound_type_1.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn items(self) -> [Self::Item] }
+record W<T> { v: T }
+impl<T> It for W<T> { type Item = T
+ fn items(self) -> [T] { [self.v] } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_inside_a_compound_type_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_inside_a_compound_type_2.nova
new file mode 100644
index 00000000..ef8458ee
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_inside_a_compound_type_2.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn items(self) -> [Self::Item] }
+record W<T> { v: T }
+impl<T> It for W<T> { type Item = T
+ fn items(self) -> [Bool] { [true] } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_parameter_types_not_only_the_return_type_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_parameter_types_not_only_the_return_type_1.nova
new file mode 100644
index 00000000..2a2d70ce
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_parameter_types_not_only_the_return_type_1.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn put(self, x: Self::Item) -> Int }
+record W<T> { v: T }
+impl<T> It for W<T> { type Item = T
+ fn put(self, x: T) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_parameter_types_not_only_the_return_type_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_parameter_types_not_only_the_return_type_2.nova
new file mode 100644
index 00000000..946b3674
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_parameter_types_not_only_the_return_type_2.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn put(self, x: Self::Item) -> Int }
+record W<T> { v: T }
+impl<T> It for W<T> { type Item = T
+ fn put(self, x: Bool) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_through_the_matching_impl_not_the_only_one_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_through_the_matching_impl_not_the_only_one_1.nova
new file mode 100644
index 00000000..880b6ec0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_through_the_matching_impl_not_the_only_one_1.nova
@@ -0,0 +1,9 @@
+trait It { type Item
+ fn peek(self) -> Self::Item }
+record W<T> { v: T }
+record K<T> { k: T }
+impl<T> It for W<T> { type Item = T
+ fn peek(self) -> T { self.v } }
+impl<T> It for K<T> { type Item = [T]
+ fn peek(self) -> [T] { [self.k] } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_through_the_matching_impl_not_the_only_one_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_through_the_matching_impl_not_the_only_one_2.nova
new file mode 100644
index 00000000..21832000
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_normalizes_through_the_matching_impl_not_the_only_one_2.nova
@@ -0,0 +1,9 @@
+trait It { type Item
+ fn peek(self) -> Self::Item }
+record W<T> { v: T }
+record K<T> { k: T }
+impl<T> It for W<T> { type Item = T
+ fn peek(self) -> [T] { [self.v] } }
+impl<T> It for K<T> { type Item = [T]
+ fn peek(self) -> T { self.k } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_1.nova
new file mode 100644
index 00000000..da1e24dc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_1.nova
@@ -0,0 +1,6 @@
+trait Base { type Elem }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W { v: Int }
+impl Ext for W { fn peek(self) -> Int { self.v } }
+impl Base for W { type Elem = Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_2.nova
new file mode 100644
index 00000000..09622613
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_2.nova
@@ -0,0 +1,6 @@
+trait Base { type Elem }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W { v: Int }
+impl Base for W { type Elem = Int }
+impl Ext for W { fn peek(self) -> Int { self.v } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_3.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_3.nova
new file mode 100644
index 00000000..8c47135e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_3.nova
@@ -0,0 +1,6 @@
+trait Base { type Elem }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W { v: Int }
+impl Ext for W { fn peek(self) -> Bool { true } }
+impl Base for W { type Elem = Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_4.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_4.nova
new file mode 100644
index 00000000..54c706cd
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_4.nova
@@ -0,0 +1,6 @@
+trait Base { type Elem }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W { v: Int }
+impl Base for W { type Elem = Int }
+impl Ext for W { fn peek(self) -> Bool { true } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_5.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_5.nova
new file mode 100644
index 00000000..a5ef1516
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_5.nova
@@ -0,0 +1,7 @@
+trait Base { type Elem }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W<A, B> { a: A
+ b: B }
+impl<T> Ext for W<Int, T> { fn peek(self) -> Int { self.a } }
+impl<A, B> Base for W<A, B> { type Elem = A }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_6.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_6.nova
new file mode 100644
index 00000000..d3a04ec0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_6.nova
@@ -0,0 +1,7 @@
+trait Base { type Elem }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W<A, B> { a: A
+ b: B }
+impl<A, B> Base for W<A, B> { type Elem = A }
+impl<T> Ext for W<Int, T> { fn peek(self) -> Int { self.a } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_7.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_7.nova
new file mode 100644
index 00000000..ba55f0c6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_7.nova
@@ -0,0 +1,7 @@
+trait Base { type Elem }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W<A, B> { a: A
+ b: B }
+impl<T> Ext for W<Int, T> { fn peek(self) -> Bool { true } }
+impl<A, B> Base for W<A, B> { type Elem = A }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_8.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_8.nova
new file mode 100644
index 00000000..28598195
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_resolves_a_projection_bound_by_a_later_declared_impl_8.nova
@@ -0,0 +1,7 @@
+trait Base { type Elem }
+trait Ext: Base { fn peek(self) -> Self::Elem }
+record W<A, B> { a: A
+ b: B }
+impl<A, B> Base for W<A, B> { type Elem = A }
+impl<T> Ext for W<Int, T> { fn peek(self) -> Bool { true } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_still_rejects_a_wrong_arity_and_a_wrong_receiver_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_still_rejects_a_wrong_arity_and_a_wrong_receiver_1.nova
new file mode 100644
index 00000000..2f625612
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_still_rejects_a_wrong_arity_and_a_wrong_receiver_1.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn put(self, x: Self::Item) -> Int }
+record W<T> { v: T }
+impl<T> It for W<T> { type Item = T
+ fn put(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__conformance_still_rejects_a_wrong_arity_and_a_wrong_receiver_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_still_rejects_a_wrong_arity_and_a_wrong_receiver_2.nova
new file mode 100644
index 00000000..1e45c3ec
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__conformance_still_rejects_a_wrong_arity_and_a_wrong_receiver_2.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get_item(self) -> Self::Item }
+record W<T> { v: T }
+impl<T> It for W<T> { type Item = T
+fn get_item() -> T { panic("no") } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_free_function_reports_e0403.nova b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_free_function_reports_e0403.nova
new file mode 100644
index 00000000..7f247ac3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_free_function_reports_e0403.nova
@@ -0,0 +1,2 @@
+fn f<U, U>(a: U, b: U) -> U { a }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_impl_block_reports_e0403.nova b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_impl_block_reports_e0403.nova
new file mode 100644
index 00000000..ec700df8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_impl_block_reports_e0403.nova
@@ -0,0 +1,3 @@
+record Box<T> { value: T }
+impl<T, T> Box<T> { fn get(self) -> Int { 0 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_record_reports_e0403.nova b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_record_reports_e0403.nova
new file mode 100644
index 00000000..efeee50d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_record_reports_e0403.nova
@@ -0,0 +1,2 @@
+record R<T, T> { a: T }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_sum_reports_e0403.nova b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_sum_reports_e0403.nova
new file mode 100644
index 00000000..90c3750a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_sum_reports_e0403.nova
@@ -0,0 +1,2 @@
+type E<T, T> = | A(T) | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_trait_method_reports_e0403.nova b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_trait_method_reports_e0403.nova
new file mode 100644
index 00000000..338295dd
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_generic_on_trait_method_reports_e0403.nova
@@ -0,0 +1,2 @@
+trait Tr { fn m<U, U>(self, a: U) -> U }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_method_generic_name_reports_e0403.nova b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_method_generic_name_reports_e0403.nova
new file mode 100644
index 00000000..5d7517e3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_method_generic_name_reports_e0403.nova
@@ -0,0 +1,4 @@
+record Box<T> { value: T }
+impl<T> Box<T> { fn weird<U, U>(self, a: U, b: U) -> U { a } }
+fn main() { let a = Box { value: 1 }
+ println("${a.value}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_supertrait_is_deduplicated.nova b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_supertrait_is_deduplicated.nova
new file mode 100644
index 00000000..3ae2adc9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__duplicate_supertrait_is_deduplicated.nova
@@ -0,0 +1,7 @@
+trait A { fn a(self) -> Int }
+trait B: A + A { fn b(self) -> Int }
+record R { v: Int }
+impl A for R { fn a(self) -> Int { 1 } }
+impl B for R { fn b(self) -> Int { 2 } }
+fn only_a<T: B>(x: T) -> Int { x.a() }
+fn main() { println("${only_a(R { v: 0 })}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__every_trait_call_agrees_with_its_callees_has_self.nova b/crates/nova-cli/tests/selfhost/cases/check/check__every_trait_call_agrees_with_its_callees_has_self.nova
new file mode 100644
index 00000000..0afd1650
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__every_trait_call_agrees_with_its_callees_has_self.nova
@@ -0,0 +1,10 @@
+trait Zero { fn zero() -> Self }
+trait Widen { fn widen(self, k: Int) -> Int }
+impl Zero for Int { fn zero() -> Int { 0 } }
+impl Widen for Int { fn widen(self, k: Int) -> Int { k } }
+fn make<T: Zero>() -> T { T::zero() }
+fn main() {
+let n: Int = make()
+let m: Int = Int::zero()
+println("${n.widen(1)}${m}")
+}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__extern_non_scalar_param_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__extern_non_scalar_param_reports_e0900.nova
new file mode 100644
index 00000000..feb87241
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__extern_non_scalar_param_reports_e0900.nova
@@ -0,0 +1,2 @@
+extern "C" { fn puts(s: String) -> Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__extern_reserved_symbol_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__extern_reserved_symbol_reports_e0900.nova
new file mode 100644
index 00000000..3a92ff6b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__extern_reserved_symbol_reports_e0900.nova
@@ -0,0 +1,2 @@
+extern "C" { fn nova_rt_alloc(x: Int) -> Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__extern_scalar_signature_is_accepted.nova b/crates/nova-cli/tests/selfhost/cases/check/check__extern_scalar_signature_is_accepted.nova
new file mode 100644
index 00000000..60cfe206
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__extern_scalar_signature_is_accepted.nova
@@ -0,0 +1,4 @@
+extern "C" { fn sqrt(x: Float) -> Float
+ fn llabs(x: Int) -> Int }
+fn main() { let r = sqrt(2.0)
+ println("${r}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__extern_unsupported_abi_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__extern_unsupported_abi_reports_e0900.nova
new file mode 100644
index 00000000..544520f1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__extern_unsupported_abi_reports_e0900.nova
@@ -0,0 +1,2 @@
+extern "stdcall" { fn foo() -> Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__future_with_two_type_arguments_is_rejected.nova b/crates/nova-cli/tests/selfhost/cases/check/check__future_with_two_type_arguments_is_rejected.nova
new file mode 100644
index 00000000..3d55b163
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__future_with_two_type_arguments_is_rejected.nova
@@ -0,0 +1,2 @@
+fn f(x: Future<Int, Bool>) -> Int { 1 }
+fn main() {}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__generic_extern_reports_e0900_without_cascade.nova b/crates/nova-cli/tests/selfhost/cases/check/check__generic_extern_reports_e0900_without_cascade.nova
new file mode 100644
index 00000000..84471c8d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__generic_extern_reports_e0900_without_cascade.nova
@@ -0,0 +1,2 @@
+extern "C" { fn id<T>(x: T) -> T }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__generic_impl_method_for_nongeneric_trait_method_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__generic_impl_method_for_nongeneric_trait_method_reports_e0072.nova
new file mode 100644
index 00000000..ee6f0baa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__generic_impl_method_for_nongeneric_trait_method_reports_e0072.nova
@@ -0,0 +1,4 @@
+trait Mapper { fn remap(self) -> Int }
+record Box<T> { value: T }
+impl<T> Mapper for Box<T> { fn remap<U>(self, x: U) -> Int { 0 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__generic_impl_of_subtrait_without_a_covering_supertrait_impl_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__generic_impl_of_subtrait_without_a_covering_supertrait_impl_reports_e0072.nova
new file mode 100644
index 00000000..f556c190
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__generic_impl_of_subtrait_without_a_covering_supertrait_impl_reports_e0072.nova
@@ -0,0 +1,9 @@
+trait A { fn a(self) -> Int }
+trait B: A { fn b(self) -> Int }
+record R { v: Int }
+record W<T> { inner: T }
+impl A for R { fn a(self) -> Int { 1 } }
+impl B for R { fn b(self) -> Int { 2 } }
+impl A for W<R> { fn a(self) -> Int { 3 } }
+impl<T: B> B for W<T> { fn b(self) -> Int { 4 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__generic_impl_wrong_self_param_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__generic_impl_wrong_self_param_reports_e0072.nova
new file mode 100644
index 00000000..7dfadf16
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__generic_impl_wrong_self_param_reports_e0072.nova
@@ -0,0 +1,5 @@
+record Box<T> { value: T }
+trait Dup { fn dup(self) -> Self }
+impl<T> Dup for Box<T> { fn dup(self) -> T { self.value } }
+fn main() { let b = Box { value: 1 }
+ let c = b.dup() }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__generic_trait_method_arity_mismatch_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__generic_trait_method_arity_mismatch_reports_e0072.nova
new file mode 100644
index 00000000..86fbd07f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__generic_trait_method_arity_mismatch_reports_e0072.nova
@@ -0,0 +1,4 @@
+trait Mapper { fn remap<U>(self, f: fn(Int) -> U) -> U }
+record Thing { v: Int }
+impl Mapper for Thing { fn remap(self, f: fn(Int) -> Int) -> Int { f(self.v) } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__hir_trait_def_records_resolved_supertraits.nova b/crates/nova-cli/tests/selfhost/cases/check/check__hir_trait_def_records_resolved_supertraits.nova
new file mode 100644
index 00000000..03a3e934
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__hir_trait_def_records_resolved_supertraits.nova
@@ -0,0 +1,6 @@
+trait A { fn a(self) -> Int }
+trait B: A { fn b(self) -> Int }
+record R { v: Int }
+impl A for R { fn a(self) -> Int { 1 } }
+impl B for R { fn b(self) -> Int { 2 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__impl_adding_undeclared_method_bound_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__impl_adding_undeclared_method_bound_reports_e0072.nova
new file mode 100644
index 00000000..8ee6364f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__impl_adding_undeclared_method_bound_reports_e0072.nova
@@ -0,0 +1,5 @@
+trait Show { fn show(self) -> String }
+trait Tagger { fn tag<U>(self, x: U) -> String }
+record Thing { v: Int }
+impl Tagger for Thing { fn tag<U: Show>(self, x: U) -> String { "tagged" } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__impl_dropping_trait_method_bound_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__impl_dropping_trait_method_bound_reports_e0072.nova
new file mode 100644
index 00000000..ce3fdbc6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__impl_dropping_trait_method_bound_reports_e0072.nova
@@ -0,0 +1,5 @@
+trait Show { fn show(self) -> String }
+trait Tagger { fn tag<U: Show>(self, x: U) -> String }
+record Thing { v: Int }
+impl Tagger for Thing { fn tag<U>(self, x: U) -> String { "no-show" } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__impl_missing_required_method_reports_e0070.nova b/crates/nova-cli/tests/selfhost/cases/check/check__impl_missing_required_method_reports_e0070.nova
new file mode 100644
index 00000000..1ad18ab4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__impl_missing_required_method_reports_e0070.nova
@@ -0,0 +1,4 @@
+record P { v: Int }
+trait Show { fn name(self) -> String }
+impl Show for P { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__impl_of_subtrait_without_supertrait_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__impl_of_subtrait_without_supertrait_reports_e0072.nova
new file mode 100644
index 00000000..04f3d03b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__impl_of_subtrait_without_supertrait_reports_e0072.nova
@@ -0,0 +1,5 @@
+trait A { fn a(self) -> Int }
+trait B: A { fn b(self) -> Int }
+record R { v: Int }
+impl B for R { fn b(self) -> Int { 2 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__impl_omitting_a_self_receiver_the_trait_declares_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__impl_omitting_a_self_receiver_the_trait_declares_reports_e0072.nova
new file mode 100644
index 00000000..f88ea237
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__impl_omitting_a_self_receiver_the_trait_declares_reports_e0072.nova
@@ -0,0 +1,4 @@
+record R { v: Int }
+trait Zero { fn zero(self) -> Int }
+impl Zero for R { fn zero() -> Int { 0 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__impl_self_receiver_disagreeing_with_trait_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__impl_self_receiver_disagreeing_with_trait_reports_e0072.nova
new file mode 100644
index 00000000..8651eab3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__impl_self_receiver_disagreeing_with_trait_reports_e0072.nova
@@ -0,0 +1,4 @@
+trait Zero { fn zero() -> Self }
+record R { v: Int }
+impl Zero for R { fn zero(self) -> R { self } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__impl_wrong_arity_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__impl_wrong_arity_reports_e0072.nova
new file mode 100644
index 00000000..5e8c06fd
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__impl_wrong_arity_reports_e0072.nova
@@ -0,0 +1,4 @@
+record P { v: Int }
+trait T { fn m(self, x: Int) -> Int }
+impl T for P { fn m(self) -> Int { 0 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__impl_wrong_param_type_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__impl_wrong_param_type_reports_e0072.nova
new file mode 100644
index 00000000..ab418ea6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__impl_wrong_param_type_reports_e0072.nova
@@ -0,0 +1,4 @@
+record P { v: Int }
+trait T { fn m(self, x: Int) -> String }
+impl T for P { fn m(self, x: String) -> String { x } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__impl_wrong_return_type_reports_e0072.nova b/crates/nova-cli/tests/selfhost/cases/check/check__impl_wrong_return_type_reports_e0072.nova
new file mode 100644
index 00000000..51d2422f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__impl_wrong_return_type_reports_e0072.nova
@@ -0,0 +1,4 @@
+record P { v: Int }
+trait T { fn m(self) -> Int }
+impl T for P { fn m(self) -> String { "x" } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__method_generic_shadowing_impl_generic_reports_e0403.nova b/crates/nova-cli/tests/selfhost/cases/check/check__method_generic_shadowing_impl_generic_reports_e0403.nova
new file mode 100644
index 00000000..8bb92e50
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__method_generic_shadowing_impl_generic_reports_e0403.nova
@@ -0,0 +1,4 @@
+record Box<T> { value: T }
+impl<T> Box<T> { fn weird<T>(self, x: T) -> T { x } }
+fn main() { let a = Box { value: 1 }
+ println("${a.weird(2)}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__method_not_in_trait_reports_e0071.nova b/crates/nova-cli/tests/selfhost/cases/check/check__method_not_in_trait_reports_e0071.nova
new file mode 100644
index 00000000..0506a0af
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__method_not_in_trait_reports_e0071.nova
@@ -0,0 +1,5 @@
+record P { v: Int }
+trait Show { fn name(self) -> String }
+impl Show for P { fn name(self) -> String { "p" }
+ fn extra(self) -> Int { 0 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__mut_self_trait_method_conformance_disagreement_is_e0072_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__mut_self_trait_method_conformance_disagreement_is_e0072_1.nova
new file mode 100644
index 00000000..499ce4d0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__mut_self_trait_method_conformance_disagreement_is_e0072_1.nova
@@ -0,0 +1,4 @@
+trait Bump { fn bump(mut self) }
+record C { n: Int }
+impl Bump for C { fn bump(self) { } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__mut_self_trait_method_conformance_disagreement_is_e0072_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__mut_self_trait_method_conformance_disagreement_is_e0072_2.nova
new file mode 100644
index 00000000..6ae31daa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__mut_self_trait_method_conformance_disagreement_is_e0072_2.nova
@@ -0,0 +1,4 @@
+trait Bump { fn bump(self) }
+record C { n: Int }
+impl Bump for C { fn bump(mut self) { } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__overlapping_inherent_impls_sharing_method_report_e0074.nova b/crates/nova-cli/tests/selfhost/cases/check/check__overlapping_inherent_impls_sharing_method_report_e0074.nova
new file mode 100644
index 00000000..06bf46f0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__overlapping_inherent_impls_sharing_method_report_e0074.nova
@@ -0,0 +1,4 @@
+record Box<T> { value: T }
+impl<T> Box<T> { fn tag(self) -> String { "g" } }
+impl Box<Int> { fn tag(self) -> String { "i" } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__overlapping_trait_impls_report_e0074.nova b/crates/nova-cli/tests/selfhost/cases/check/check__overlapping_trait_impls_report_e0074.nova
new file mode 100644
index 00000000..70dd30a5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__overlapping_trait_impls_report_e0074.nova
@@ -0,0 +1,5 @@
+record Box<T> { value: T }
+trait Kind { fn kind(self) -> String }
+impl<T> Kind for Box<T> { fn kind(self) -> String { "g" } }
+impl Kind for Box<Int> { fn kind(self) -> String { "i" } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__rejected_type_arguments_and_an_undeclared_name_do_not_contradict_each_other.nova b/crates/nova-cli/tests/selfhost/cases/check/check__rejected_type_arguments_and_an_undeclared_name_do_not_contradict_each_other.nova
new file mode 100644
index 00000000..a1b29298
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__rejected_type_arguments_and_an_undeclared_name_do_not_contradict_each_other.nova
@@ -0,0 +1,4 @@
+trait It { type Item
+ fn get(self) -> Int }
+fn f<I: It>(x: I) -> I::Nope<Int> { panic("unreachable") }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__rejected_type_arguments_on_a_projection_are_not_themselves_resolved.nova b/crates/nova-cli/tests/selfhost/cases/check/check__rejected_type_arguments_on_a_projection_are_not_themselves_resolved.nova
new file mode 100644
index 00000000..1588dd80
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__rejected_type_arguments_on_a_projection_are_not_themselves_resolved.nova
@@ -0,0 +1,4 @@
+trait It { type Item
+ fn get(self) -> Int }
+fn first<I: It>(x: I) -> I::Item<Nope> { panic("unreachable") }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__self_item_in_a_trait_impl_projects_onto_the_impls_own_self_type.nova b/crates/nova-cli/tests/selfhost/cases/check/check__self_item_in_a_trait_impl_projects_onto_the_impls_own_self_type.nova
new file mode 100644
index 00000000..0e4e898c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__self_item_in_a_trait_impl_projects_onto_the_impls_own_self_type.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Self::Item }
+record W<T> { v: T }
+impl<T> It for W<T> { type Item = T
+ fn get(self) -> Self::Item { panic("x") } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__self_item_in_an_inherent_impl_reports_that_it_has_no_trait.nova b/crates/nova-cli/tests/selfhost/cases/check/check__self_item_in_an_inherent_impl_reports_that_it_has_no_trait.nova
new file mode 100644
index 00000000..0d13254a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__self_item_in_an_inherent_impl_reports_that_it_has_no_trait.nova
@@ -0,0 +1,5 @@
+trait It { type Item
+ fn get(self) -> Int }
+record K { v: Int }
+impl K { fn peek(self) -> Self::Item { panic("x") } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__self_item_outside_any_impl_or_trait_still_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__self_item_outside_any_impl_or_trait_still_reports_e0900.nova
new file mode 100644
index 00000000..6bfaeb39
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__self_item_outside_any_impl_or_trait_still_reports_e0900.nova
@@ -0,0 +1,4 @@
+trait It { type Item
+ fn get(self) -> Int }
+fn f() -> Self::Item { panic("x") }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__self_naming_an_undeclared_associated_type_in_a_trait_impl_is_an_error.nova b/crates/nova-cli/tests/selfhost/cases/check/check__self_naming_an_undeclared_associated_type_in_a_trait_impl_is_an_error.nova
new file mode 100644
index 00000000..95593481
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__self_naming_an_undeclared_associated_type_in_a_trait_impl_is_an_error.nova
@@ -0,0 +1,7 @@
+trait It { type Item
+ fn get(self) -> Int }
+record W { v: Int }
+impl It for W { type Item = Int
+ fn get(self) -> Int { 1 }
+ fn other(self) -> Self::Nope { panic("x") } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__set_methods_typecheck.nova b/crates/nova-cli/tests/selfhost/cases/check/check__set_methods_typecheck.nova
new file mode 100644
index 00000000..c697f0f5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__set_methods_typecheck.nova
@@ -0,0 +1,6 @@
+fn main() {
+let mut s = Set::new()
+println("${s.insert(1)} ${s.insert(1)}")
+println("${s.len()} ${s.contains(1)} ${s.contains(2)}")
+println("${s.remove(1)} ${s.len()}")
+}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_1.nova
new file mode 100644
index 00000000..7955bc77
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_1.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Self::Item }
+record W { v: Int }
+impl It for W { type Item = Self::Item
+ fn get(self) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_2.nova
new file mode 100644
index 00000000..ed5be989
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_2.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Self::Item }
+record W { v: Int }
+impl It for W { type Item = Nope
+ fn get(self) -> Bool { true } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_3.nova b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_3.nova
new file mode 100644
index 00000000..2a0fd204
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_3.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Option<Self::Item> }
+record W { v: Int }
+impl It for W { type Item = Nope
+ fn get(self) -> Option<Bool> { None } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_4.nova b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_4.nova
new file mode 100644
index 00000000..99445c19
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_4.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self, x: Self::Item) -> Int }
+record W { v: Int }
+impl It for W { type Item = Nope
+ fn get(self, x: Bool) -> Int { 1 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_5.nova b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_5.nova
new file mode 100644
index 00000000..4e115c39
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__signatures_are_not_compared_when_either_side_is_already_poisoned_5.nova
@@ -0,0 +1,6 @@
+trait It { type Item
+ fn get(self) -> Int }
+record W { v: Int }
+impl It for W { type Item = Int
+ fn get(self) -> Bool { true } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__subtrait_bound_resolves_the_supertrait_method_to_the_supertrait.nova b/crates/nova-cli/tests/selfhost/cases/check/check__subtrait_bound_resolves_the_supertrait_method_to_the_supertrait.nova
new file mode 100644
index 00000000..a49263f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__subtrait_bound_resolves_the_supertrait_method_to_the_supertrait.nova
@@ -0,0 +1,7 @@
+trait A { fn a(self) -> Int }
+trait B: A { fn b(self) -> Int }
+record R { v: Int }
+impl A for R { fn a(self) -> Int { 1 } }
+impl B for R { fn b(self) -> Int { 2 } }
+fn only_a<T: B>(x: T) -> Int { x.a() }
+fn main() { println("${only_a(R { v: 0 })}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__sum_type_param_bound_e0900_reports_every_bounded_param.nova b/crates/nova-cli/tests/selfhost/cases/check/check__sum_type_param_bound_e0900_reports_every_bounded_param.nova
new file mode 100644
index 00000000..d35e8088
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__sum_type_param_bound_e0900_reports_every_bounded_param.nova
@@ -0,0 +1,4 @@
+trait A { fn a(self) -> Int }
+trait B { fn b(self) -> Int }
+type Two<K: A, V: B> = | X(K) | Y(V)
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__sum_type_param_bound_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__sum_type_param_bound_reports_e0900.nova
new file mode 100644
index 00000000..4ba667b2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__sum_type_param_bound_reports_e0900.nova
@@ -0,0 +1,5 @@
+trait Hash2 { fn h(self) -> Int }
+type Wrap<T: Hash2> = | Wrapped(T) | Empty
+record NoHash { n: Int }
+fn main() { let x = Wrapped(NoHash { n: 1 })
+ match x { Wrapped(_) => println("w"), Empty => println("e") } }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__supertrait_method_callable_from_a_default_body.nova b/crates/nova-cli/tests/selfhost/cases/check/check__supertrait_method_callable_from_a_default_body.nova
new file mode 100644
index 00000000..3d33da58
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__supertrait_method_callable_from_a_default_body.nova
@@ -0,0 +1,7 @@
+trait A { fn a(self) -> Int }
+trait B: A { fn b(self) -> Int { self.a() + 1 } }
+record R { v: Int }
+impl A for R { fn a(self) -> Int { 1 } }
+impl B for R { }
+fn main() { let x = R { v: 0 }
+ println("${x.b()}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__the_compiler_terminates_on_every_cyclic_binding_shape_1.nova b/crates/nova-cli/tests/selfhost/cases/check/check__the_compiler_terminates_on_every_cyclic_binding_shape_1.nova
new file mode 100644
index 00000000..cd9117c2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__the_compiler_terminates_on_every_cyclic_binding_shape_1.nova
@@ -0,0 +1,7 @@
+trait It { type Item
+ fn get(self) -> Self::Item }
+record W { v: Int }
+impl It for W { type Item = Self::Item
+ fn get(self) -> Self::Item { 1 } }
+fn use_it(w: W) -> Int { 1 }
+fn main() { println("${use_it(W { v: 1 })}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__the_compiler_terminates_on_every_cyclic_binding_shape_2.nova b/crates/nova-cli/tests/selfhost/cases/check/check__the_compiler_terminates_on_every_cyclic_binding_shape_2.nova
new file mode 100644
index 00000000..54f43661
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__the_compiler_terminates_on_every_cyclic_binding_shape_2.nova
@@ -0,0 +1,7 @@
+trait It { type Item
+ fn get(self) -> Self::Item }
+record W { v: Int }
+impl It for W { type Item = [Self::Item]
+ fn get(self) -> Int { 1 } }
+fn use_it(w: W) -> Int { 1 }
+fn main() { println("${use_it(W { v: 1 })}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__the_compiler_terminates_on_every_cyclic_binding_shape_3.nova b/crates/nova-cli/tests/selfhost/cases/check/check__the_compiler_terminates_on_every_cyclic_binding_shape_3.nova
new file mode 100644
index 00000000..5081f9d2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__the_compiler_terminates_on_every_cyclic_binding_shape_3.nova
@@ -0,0 +1,7 @@
+trait It { type Item
+ fn get(self) -> Self::Item }
+record W { v: Int }
+impl It for W { type Item = [[Self::Item]]
+ fn get(self) -> Int { 1 } }
+fn use_it(w: W) -> Int { 1 }
+fn main() { println("${use_it(W { v: 1 })}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__two_traits_with_the_same_associated_type_name_get_distinct_defids.nova b/crates/nova-cli/tests/selfhost/cases/check/check__two_traits_with_the_same_associated_type_name_get_distinct_defids.nova
new file mode 100644
index 00000000..ac5e4a65
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__two_traits_with_the_same_associated_type_name_get_distinct_defids.nova
@@ -0,0 +1,5 @@
+trait A { type Item
+ fn a(self) -> Int }
+trait B { type Item
+ fn b(self) -> Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__unknown_supertrait_reports_e0001.nova b/crates/nova-cli/tests/selfhost/cases/check/check__unknown_supertrait_reports_e0001.nova
new file mode 100644
index 00000000..32b5ee7b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__unknown_supertrait_reports_e0001.nova
@@ -0,0 +1,2 @@
+trait B: Nope { fn b(self) -> Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__unused_impl_param_reports_e0073.nova b/crates/nova-cli/tests/selfhost/cases/check/check__unused_impl_param_reports_e0073.nova
new file mode 100644
index 00000000..c9c9292f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__unused_impl_param_reports_e0073.nova
@@ -0,0 +1,4 @@
+record Box<T> { value: T }
+impl<T, U> Box<T> { fn get(self) -> T { self.value } }
+fn main() { let b = Box { value: 1 }
+ let x = b.get() }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__where_clause_on_concrete_type_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__where_clause_on_concrete_type_reports_e0900.nova
new file mode 100644
index 00000000..fe766f09
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__where_clause_on_concrete_type_reports_e0900.nova
@@ -0,0 +1,3 @@
+trait Show { fn show(self) -> String }
+fn f<T>(x: T) -> Int where Int: Show { 0 }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__where_clause_on_trait_declaration_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__where_clause_on_trait_declaration_reports_e0900.nova
new file mode 100644
index 00000000..31e1bfa7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__where_clause_on_trait_declaration_reports_e0900.nova
@@ -0,0 +1,5 @@
+trait A { fn a(self) -> Int }
+trait B where Self: A { fn b(self) -> Int }
+record R { v: Int }
+impl B for R { fn b(self) -> Int { 2 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/check__where_clause_on_trait_method_reports_e0900.nova b/crates/nova-cli/tests/selfhost/cases/check/check__where_clause_on_trait_method_reports_e0900.nova
new file mode 100644
index 00000000..fbf04bc6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/check__where_clause_on_trait_method_reports_e0900.nova
@@ -0,0 +1,2 @@
+trait Foo { fn f(self) -> Int where Self: Foo }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/check/every_e0012_form.nova b/crates/nova-cli/tests/selfhost/cases/check/every_e0012_form.nova
new file mode 100644
index 00000000..1a474204
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/every_e0012_form.nova
@@ -0,0 +1,10 @@
+record Pair<T> { a: T, b: T }
+trait It {
+    type Item
+}
+fn bare(p: Pair) {}
+fn extra(p: Pair<Int, Int>) {}
+fn param<T>(x: T<Int>) {}
+fn projected<T: It>(x: T::Item<Int>) {}
+fn prim(x: Int<Bool>) {}
+fn future(x: Future<Int, Bool>) {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/every_item_e0001_site.nova b/crates/nova-cli/tests/selfhost/cases/check/every_item_e0001_site.nova
new file mode 100644
index 00000000..2404f7fb
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/every_item_e0001_site.nova
@@ -0,0 +1,5 @@
+trait Shown: Nope {}
+impl Missing for Int {}
+fn bounded<T: Absent>(x: T) {}
+fn clause<T>(x: T) where T: Gone {}
+fn typed(x: Nowhere) {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_did_you_mean_a_primitive_or_a_type_parameter.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_did_you_mean_a_primitive_or_a_type_parameter.nova
new file mode 100644
index 00000000..0caab88a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_did_you_mean_a_primitive_or_a_type_parameter.nova
@@ -0,0 +1,2 @@
+fn f(x: Boool) {}
+fn g<Item>(x: Iten) {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_held_back_by_a_clash/main.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_held_back_by_a_clash/main.nova
new file mode 100644
index 00000000..71d7aa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_held_back_by_a_clash/main.nova
@@ -0,0 +1,3 @@
+import shapes
+record Meter { n: Int }
+impl Meter for Int {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_held_back_by_a_clash/shapes.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_held_back_by_a_clash/shapes.nova
new file mode 100644
index 00000000..7a473a4d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_held_back_by_a_clash/shapes.nova
@@ -0,0 +1,2 @@
+import units::{Meter}
+pub fn s() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_held_back_by_a_clash/units.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_held_back_by_a_clash/units.nova
new file mode 100644
index 00000000..cfe29e15
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_held_back_by_a_clash/units.nova
@@ -0,0 +1,2 @@
+pub trait Meter {}
+pub record Meter { n: Int }
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_import_and_did_you_mean/main.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_import_and_did_you_mean/main.nova
new file mode 100644
index 00000000..34cdbb28
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_import_and_did_you_mean/main.nova
@@ -0,0 +1,6 @@
+import shapes::{Dot}
+fn area(s: Shape) -> Int { 1 }
+fn long(m: Meter) -> Int { 1 }
+fn named(s: Strin) -> Int { 1 }
+fn bounded<T: Eqq>(x: T) {}
+impl Shows for Int {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_import_and_did_you_mean/shapes.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_import_and_did_you_mean/shapes.nova
new file mode 100644
index 00000000..5b5300f1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_import_and_did_you_mean/shapes.nova
@@ -0,0 +1,8 @@
+import units
+pub type Shape =
+  | Dot
+  | Line(Int)
+pub trait Shows {
+    fn show(self) -> String
+}
+pub fn span() -> Meter { Meter { n: 1 } }
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_import_and_did_you_mean/units.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_import_and_did_you_mean/units.nova
new file mode 100644
index 00000000..d4301924
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_import_and_did_you_mean/units.nova
@@ -0,0 +1 @@
+pub record Meter { n: Int }
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/nova.toml b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/nova.toml
new file mode 100644
index 00000000..7ee1487b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/pkgs/geom/nova.toml
new file mode 100644
index 00000000..55c1b534
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/pkgs/geom/src/lib.nova
new file mode 100644
index 00000000..bd0f648c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/pkgs/geom/src/lib.nova
@@ -0,0 +1 @@
+pub fn area(x: Boool) -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/src/main.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/src/main.nova
new file mode 100644
index 00000000..2ec0cbfc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_in_a_dependency/src/main.nova
@@ -0,0 +1,2 @@
+import geom
+fn f(x: Boool) {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/gauges.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/gauges.nova
new file mode 100644
index 00000000..d4301924
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/gauges.nova
@@ -0,0 +1 @@
+pub record Meter { n: Int }
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/main.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/main.nova
new file mode 100644
index 00000000..f32144a4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/main.nova
@@ -0,0 +1,3 @@
+import shapes
+import tools
+fn long(m: Meter) -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/shapes.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/shapes.nova
new file mode 100644
index 00000000..13542ee6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/shapes.nova
@@ -0,0 +1,2 @@
+import units
+pub fn s() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/tools.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/tools.nova
new file mode 100644
index 00000000..137b9677
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/tools.nova
@@ -0,0 +1,2 @@
+import gauges
+pub fn t() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/units.nova b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/units.nova
new file mode 100644
index 00000000..d4301924
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/fixes_none_when_two_modules_export_the_name/units.nova
@@ -0,0 +1 @@
+pub record Meter { n: Int }
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_supertraits_in_a_programs_bounds.nova b/crates/nova-cli/tests/selfhost/cases/check/std_supertraits_in_a_programs_bounds.nova
new file mode 100644
index 00000000..d5e07215
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_supertraits_in_a_programs_bounds.nova
@@ -0,0 +1,4 @@
+record Ranked<T: Ord> { x: T }
+fn rank<T: Ord + Hash>(x: T) -> Bool { true }
+trait Graded: Ord {}
+fn grade<T: Graded>(x: T) -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_array_method_that_std_has_is_e0074.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_array_method_that_std_has_is_e0074.nova
new file mode 100644
index 00000000..0c6f327f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_array_method_that_std_has_is_e0074.nova
@@ -0,0 +1,4 @@
+impl<T> [T] {
+    fn iter(self) -> Int { 0 }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_len_on_arrays_is_e0074_1.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_len_on_arrays_is_e0074_1.nova
new file mode 100644
index 00000000..0d2c9f07
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_len_on_arrays_is_e0074_1.nova
@@ -0,0 +1,6 @@
+impl [Int] {
+    fn len(self) -> Int { 99 }
+}
+fn main() {
+    println("${[1, 2, 3].len()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_len_on_arrays_is_e0074_2.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_len_on_arrays_is_e0074_2.nova
new file mode 100644
index 00000000..cbf709aa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_len_on_arrays_is_e0074_2.nova
@@ -0,0 +1,4 @@
+impl<T> [T] {
+    fn len(self, n: Int) -> Int { n }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_trait_method_hidden_by_a_std_method_is_e0105.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_trait_method_hidden_by_a_std_method_is_e0105.nova
new file mode 100644
index 00000000..5cb5d04e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_programs_trait_method_hidden_by_a_std_method_is_e0105.nova
@@ -0,0 +1,9 @@
+trait Top {
+    fn last(self) -> Int
+    fn at(self, i: Int) -> Int
+}
+impl<T> Top for Vec<T> {
+    fn last(self) -> Int { 0 - 1 }
+    fn at(self, i: Int) -> Int { i }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_static_trait_function_hidden_by_a_std_function_is_e0105_1.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_static_trait_function_hidden_by_a_std_function_is_e0105_1.nova
new file mode 100644
index 00000000..e4daa24d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_static_trait_function_hidden_by_a_std_function_is_e0105_1.nova
@@ -0,0 +1,7 @@
+trait Mk {
+    fn from_array(a: [Int]) -> Self
+}
+impl Mk for Vec<Int> {
+    fn from_array(a: [Int]) -> Vec<Int> { Vec::new() }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_std_only_builtins_name_as_a_type_gets_no_public_form.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_std_only_builtins_name_as_a_type_gets_no_public_form.nova
new file mode 100644
index 00000000..5f063eba
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_std_only_builtins_name_as_a_type_gets_no_public_form.nova
@@ -0,0 +1,2 @@
+fn f(x: str_as_bytes) {}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_impl_on_an_array_is_e0010_1.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_impl_on_an_array_is_e0010_1.nova
new file mode 100644
index 00000000..82290465
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_impl_on_an_array_is_e0010_1.nova
@@ -0,0 +1,4 @@
+impl Display for [Int] {
+    fn fmt(self) -> String { "a" }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_impl_on_an_array_is_e0010_2.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_impl_on_an_array_is_e0010_2.nova
new file mode 100644
index 00000000..1e55bc28
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_impl_on_an_array_is_e0010_2.nova
@@ -0,0 +1,4 @@
+impl<T> Hash for [T] {
+    fn hash(self) -> Int { 0 }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_impl_on_an_array_is_e0010_3.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_impl_on_an_array_is_e0010_3.nova
new file mode 100644
index 00000000..2c1d7bd0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_impl_on_an_array_is_e0010_3.nova
@@ -0,0 +1,4 @@
+impl [Int] {
+    fn total(self) -> Int { 0 }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_method_hidden_by_floats_to_bits_is_e0105.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_method_hidden_by_floats_to_bits_is_e0105.nova
new file mode 100644
index 00000000..6de7841b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_method_hidden_by_floats_to_bits_is_e0105.nova
@@ -0,0 +1,7 @@
+trait Bits {
+    fn to_bits(self) -> Int
+}
+impl Bits for Float {
+    fn to_bits(self) -> Int { 42 }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_method_hidden_by_strings_and_chars_methods_is_e0105.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_method_hidden_by_strings_and_chars_methods_is_e0105.nova
new file mode 100644
index 00000000..7122d5b0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_method_hidden_by_strings_and_chars_methods_is_e0105.nova
@@ -0,0 +1,11 @@
+trait Size {
+    fn byte_len(self) -> Int
+}
+impl Size for String {
+    fn byte_len(self) -> Int { self.len() }
+}
+trait Codes {
+    fn to_int(self) -> Int { 1000 }
+}
+impl Codes for Char {}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_method_hidden_by_the_parsers_is_e0105.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_method_hidden_by_the_parsers_is_e0105.nova
new file mode 100644
index 00000000..0dbf6294
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__a_trait_method_hidden_by_the_parsers_is_e0105.nova
@@ -0,0 +1,7 @@
+trait Parse {
+    fn parse_int_radix(self, r: Int) -> Option<Int>
+}
+impl Parse for String {
+    fn parse_int_radix(self, r: Int) -> Option<Int> { Some(r) }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__e0105s_note_names_the_traits_arguments_and_a_fresh_parameter_1.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__e0105s_note_names_the_traits_arguments_and_a_fresh_parameter_1.nova
new file mode 100644
index 00000000..352fbe0a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__e0105s_note_names_the_traits_arguments_and_a_fresh_parameter_1.nova
@@ -0,0 +1,7 @@
+trait Top {
+    fn at(self, i: Int, j: Int) -> Int
+}
+impl<T> Top for Vec<T> {
+    fn at(self, i: Int, j: Int) -> Int { i }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__e0105s_note_names_the_traits_arguments_and_a_fresh_parameter_2.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__e0105s_note_names_the_traits_arguments_and_a_fresh_parameter_2.nova
new file mode 100644
index 00000000..3d95bd3b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__e0105s_note_names_the_traits_arguments_and_a_fresh_parameter_2.nova
@@ -0,0 +1,7 @@
+trait Top {
+    fn at(self) -> Int
+}
+impl Top for Vec<Int> {
+    fn at(self) -> Int { 0 }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/std_tests__e0105s_note_names_the_traits_arguments_and_a_fresh_parameter_3.nova b/crates/nova-cli/tests/selfhost/cases/check/std_tests__e0105s_note_names_the_traits_arguments_and_a_fresh_parameter_3.nova
new file mode 100644
index 00000000..a6f99133
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/std_tests__e0105s_note_names_the_traits_arguments_and_a_fresh_parameter_3.nova
@@ -0,0 +1,7 @@
+trait Mk {
+    fn from_array(a: [Int]) -> Self
+}
+impl<T> Mk for Vec<T> {
+    fn from_array(a: [Int]) -> Vec<T> { Vec::new() }
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/tuple_tests__extern_c_refuses_a_tuple.nova b/crates/nova-cli/tests/selfhost/cases/check/tuple_tests__extern_c_refuses_a_tuple.nova
new file mode 100644
index 00000000..01d713b8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/tuple_tests__extern_c_refuses_a_tuple.nova
@@ -0,0 +1,4 @@
+extern "C" {
+    fn f(p: (Int, Int)) -> Int
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/tuple_tests__no_impl_can_name_a_tuple.nova b/crates/nova-cli/tests/selfhost/cases/check/tuple_tests__no_impl_can_name_a_tuple.nova
new file mode 100644
index 00000000..52de39b1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/tuple_tests__no_impl_can_name_a_tuple.nova
@@ -0,0 +1,4 @@
+trait Show { fn show(self) -> String }
+impl Show for (Int, Int) { fn show(self) -> String { "p" } }
+impl (Int, Bool) { fn f(self) -> Int { 1 } }
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/check/unsupported_types_and_an_extern_where.nova b/crates/nova-cli/tests/selfhost/cases/check/unsupported_types_and_an_extern_where.nova
new file mode 100644
index 00000000..5c71644a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/check/unsupported_types_and_an_extern_where.nova
@@ -0,0 +1,7 @@
+fn reference(x: &Int) {}
+fn optional(x: Int?) {}
+fn placeholder(x: _) {}
+fn qualified(x: core::Option<Int>) {}
+extern "C" {
+    fn clause(x: Int) where Int: Eq
+}
diff --git a/crates/nova-driver/src/lib.rs b/crates/nova-driver/src/lib.rs
index 199a4ced..1be3e1eb 100644
--- a/crates/nova-driver/src/lib.rs
+++ b/crates/nova-driver/src/lib.rs
@@ -1541,6 +1541,172 @@ mod resolution_dumps {
             }
         });
     }
+
+    /// The HIR dump's roots of each mode (5.3a): the corpus's and the
+    /// cases' of the resolution dump and of the type checker, as the
+    /// harness takes them.
+    fn hir_sets() -> Vec<(Target, bool, Vec<std::path::PathBuf>)> {
+        let root = repository();
+        let cases = root.join("crates/nova-cli/tests/selfhost/cases");
+        let mut corpus = Vec::new();
+        nova_files(&root, &cases, &mut corpus);
+        let has_test = |p: &std::path::PathBuf| {
+            std::fs::read_to_string(p)
+                .unwrap_or_default()
+                .lines()
+                .any(|l| l.trim_start().starts_with("@test"))
+        };
+        let under = |p: &std::path::PathBuf, dir: &str| p.starts_with(root.join(dir));
+        let cases_of = |dirs: &[&str]| -> Vec<std::path::PathBuf> {
+            dirs.iter()
+                .flat_map(|d| case_roots(&cases.join(d)))
+                .collect()
+        };
+        vec![
+            (
+                Target::Native,
+                false,
+                corpus
+                    .iter()
+                    .cloned()
+                    .chain(cases_of(&["resolve", "check"]))
+                    .collect(),
+            ),
+            (
+                Target::Wasm,
+                false,
+                corpus
+                    .iter()
+                    .filter(|p| under(p, "tests/dom") || under(p, "tests/wasm"))
+                    .cloned()
+                    .chain(cases_of(&["resolve-wasm", "check-wasm"]))
+                    .collect(),
+            ),
+            (
+                Target::Native,
+                true,
+                corpus
+                    .iter()
+                    .filter(|p| has_test(p))
+                    .cloned()
+                    .chain(cases_of(&["resolve-test", "check-test"]))
+                    .collect(),
+            ),
+        ]
+    }
+
+    /// std's item tables built once and shared by every program of a
+    /// process (5.3a, spec 5.3 §3.4) dump what checking std again for each
+    /// program dumps, byte for byte, under `check`, for the WASM target and
+    /// under test mode: on the corpus and the cases of each mode, in their
+    /// order and again in reverse, so that nothing a program leaves in the
+    /// shared state reaches the next.
+    #[test]
+    fn std_checked_once_dumps_what_std_checked_for_each_program_dumps() {
+        let sets = hir_sets();
+        std::thread::scope(|scope| {
+            let runs: Vec<_> = sets
+                .iter()
+                .map(|(target, test, roots)| {
+                    scope.spawn(move || {
+                        let dump_all = |dumps: &HirDumps, roots: &[std::path::PathBuf]| {
+                            roots
+                                .iter()
+                                .map(|r| dumps.dump(r).map_err(|e| e.kind()))
+                                .collect::<Vec<_>>()
+                        };
+                        let unshared = dump_all(&HirDumps::unshared(*target, *test), roots);
+                        let shared = HirDumps::new(*target, *test);
+                        assert!(shared.shared, "std is shared");
+                        let forward = dump_all(&shared, roots);
+                        let reversed: Vec<std::path::PathBuf> =
+                            roots.iter().rev().cloned().collect();
+                        let mut backward = dump_all(&shared, &reversed);
+                        backward.reverse();
+                        for (i, r) in roots.iter().enumerate() {
+                            assert_eq!(
+                                forward[i],
+                                unshared[i],
+                                "{target:?} {test}: {}",
+                                r.display()
+                            );
+                            assert_eq!(
+                                backward[i],
+                                unshared[i],
+                                "{target:?} {test}, reversed: {}",
+                                r.display()
+                            );
+                        }
+                        let checked = unshared
+                            .iter()
+                            .filter(|d| d.as_ref().is_ok_and(|d| d.contains(" checked=true")))
+                            .count();
+                        (roots.len(), checked)
+                    })
+                })
+                .collect();
+            for (run, floor) in runs.into_iter().zip([400, 60, 120]) {
+                let (roots, checked) = run.join().expect("a mode's thread");
+                assert!(
+                    roots >= floor && checked * 2 > roots,
+                    "{roots} roots, {checked} checked"
+                );
+            }
+        });
+    }
+
+    /// std's part of the HIR dump is the same in every program: std's
+    /// entries of a two-module program's check print as std's part of a
+    /// program of no module (5.3a, spec 5.3 §3.3), in each mode.
+    #[test]
+    fn std_part_of_a_programs_check_is_std_part() {
+        for (target, test) in [
+            (Target::Native, false),
+            (Target::Wasm, false),
+            (Target::Native, true),
+        ] {
+            let mut db = FileDb::new();
+            let (std_files, extra, mut labels) = register_std_for(&mut db, target, test);
+            let main_src = "import shapes\nrecord P { x: Int }\n\
+                impl Eq for P {\n    fn eq(self, other: P) -> Bool { true }\n}\n\
+                fn main() {}\n";
+            let lib_src = "pub type Shape = | Dot\npub trait Area {\n    fn area(self) -> Int\n}\n\
+                impl Area for Shape {\n    fn area(self) -> Int { 0 }\n}\n";
+            let main_id = db.add("main.nova", main_src);
+            let lib_id = db.add("shapes.nova", lib_src);
+            labels.push((main_id, "main".to_string()));
+            labels.push((lib_id, "shapes".to_string()));
+            let parse = |src: &str, id: FileId| {
+                let (tokens, _) = nova_lexer::lex(src, id);
+                nova_parser::parse_recovering(&tokens, id).file
+            };
+            let main = parse(main_src, main_id);
+            let lib = parse(lib_src, lib_id);
+            let mut sources = vec![
+                ModuleSource::new("main", &main),
+                ModuleSource::new("shapes", &lib),
+            ];
+            sources[0]
+                .imports
+                .insert("shapes".to_string(), nova_resolver::ImportTarget::Module(1));
+            let resolved = nova_resolver::resolve_program_for(target, &sources, &std_files, extra);
+            assert!(
+                resolved.diagnostics.is_empty(),
+                "{:?}",
+                resolved.diagnostics
+            );
+            let options = nova_typeck::CheckOptions {
+                sources: Some(&db),
+                ..Default::default()
+            };
+            let tables = nova_typeck::check_items(&resolved.file, &resolved.definitions, &options);
+            assert!(tables.diagnostics.is_empty(), "{:?}", tables.diagnostics);
+            let part = nova_typeck::dump::std_part_of(target, test, &labels, &resolved, &tables);
+            let std = dump_std_hir(target, test);
+            assert!(std.starts_with(&part), "{target:?} {test}");
+            assert_eq!(&std[part.len()..], "(Diagnostics)\n");
+        }
+    }
 }
 
 /// What doc tests need of a build (spec
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 2df4e736..1159c07d 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -4990,4 +4990,43 @@ mod tests {
             .collect();
         assert_eq!(messages, ["variant `Ok` hides std's `Result::Ok`"]);
     }
+
+    /// The shared path for the type checker (5.3a, ADR 0055 §4) gives a
+    /// program std's definitions, module scopes and items' modules rebased
+    /// past its own, numbered as the full path numbers them, though the item
+    /// passes read only part of them (a scope's values are the bodies',
+    /// 5.3b's): every definition, every scope's names and every item's
+    /// module are the full path's.
+    #[test]
+    fn std_shared_for_check_defines_what_the_full_path_does() {
+        fn scope(m: &ModuleScope) -> String {
+            let mut values: Vec<_> = m.values.iter().collect();
+            values.sort_by(|a, b| a.0.cmp(b.0));
+            let mut types: Vec<_> = m.types.iter().collect();
+            types.sort();
+            let mut traits: Vec<_> = m.traits.iter().collect();
+            traits.sort();
+            format!("{} {values:?} {types:?} {traits:?}", m.name)
+        }
+        let std = || {
+            let ids: Vec<FileId> = CompileTarget::Native
+                .std_modules()
+                .iter()
+                .map(|_| FileId::DUMMY)
+                .collect();
+            parse_std(CompileTarget::Native, &ids, None)
+        };
+        let shared = SharedStd::new(CompileTarget::Native, std());
+        assert!(shared.shareable());
+        let main =
+            parse_file("type Shape = | Dot | Line(Int)\nrecord P<T> { x: T }\nfn main() {}\n");
+        let sources = [ModuleSource::new("main", &main)];
+        let full = resolve_program_with_std(CompileTarget::Native, &sources, &std());
+        let ours = resolve_program_with_shared_std_for_check(&sources, &shared);
+        let (a, b) = (&full.definitions, &ours.definitions);
+        assert_eq!(format!("{:?}", a.defs), format!("{:?}", b.defs));
+        assert_eq!(a.item_module, b.item_module);
+        let scopes = |d: &Definitions| d.modules.iter().map(scope).collect::<Vec<_>>();
+        assert_eq!(scopes(a), scopes(b));
+    }
 }
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 8279b4f8..810a5861 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -20,6 +20,7 @@ use crate::{
 };
 
 mod fixes;
+mod items;
 #[cfg(test)]
 mod lang_tests;
 #[cfg(test)]
diff --git a/crates/nova-typeck/src/check/items.rs b/crates/nova-typeck/src/check/items.rs
new file mode 100644
index 00000000..5e5096d1
--- /dev/null
+++ b/crates/nova-typeck/src/check/items.rs
@@ -0,0 +1,192 @@
+//! The item stage: passes 1 to 5 of [`check_with`](super::check_with)
+//! (spec
+//! `docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
+//! §3.1, ADR 0055), as [`check_items`], and std's item tables built once
+//! and shared by many programs ([`SharedItems`], §3.4), for the hidden HIR
+//! dump alone.
+//!
+//! Pass 1 is the language server's index (`note_item_declarations`),
+//! which records nothing unless asked; passes 2 to 5 collect the generic
+//! parameters' checks, the arities, supertraits, records, sums, traits,
+//! impls with their checks, signatures and externs. [`check_items`] runs all
+//! five, so that [`check_with`](super::check_with) calls it first and its
+//! output is as it was; the self-hosted type checker ports passes 2 to 5.
+
+use super::*;
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use nova_diagnostics::FileId;
+    use nova_resolver::{ModuleSource, ParsedStd, ProgramResolution, SharedStd};
+
+    fn parse(src: &str) -> ast::File {
+        let (tokens, _) = nova_lexer::lex(src, FileId::DUMMY);
+        let parsed = nova_parser::parse_recovering(&tokens, FileId::DUMMY);
+        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
+        parsed.file
+    }
+
+    /// std for the native target, `extra` (a module's name and text) after
+    /// its modules.
+    fn std_with(extra: Option<(&'static str, &'static str)>) -> ParsedStd {
+        let ids: Vec<FileId> = CompileTarget::Native
+            .std_modules()
+            .iter()
+            .map(|_| FileId::DUMMY)
+            .collect();
+        nova_resolver::parse_std(
+            CompileTarget::Native,
+            &ids,
+            extra.map(|e| (e, FileId::DUMMY)),
+        )
+    }
+
+    /// The text of a program's part, from its resolution and tables.
+    fn shown(r: &ProgramResolution, t: &ItemTables) -> String {
+        let labels = [(FileId::DUMMY, "main".to_string())];
+        let mut diagnostics = r.diagnostics.clone();
+        diagnostics.extend(t.diagnostics.iter().cloned());
+        crate::dump::dump_program(
+            CompileTarget::Native,
+            false,
+            &labels,
+            Some((r, t)),
+            &diagnostics,
+        )
+    }
+
+    /// Every table of `t`, a map's or a set's entries sorted by key.
+    fn tables_text(t: &ItemTables) -> String {
+        fn set(s: &FxHashSet<DefId>) -> Vec<DefId> {
+            let mut v: Vec<DefId> = s.iter().copied().collect();
+            v.sort();
+            v
+        }
+        let sigs: Vec<_> = sorted(&t.sigs)
+            .into_iter()
+            .map(|k| (k, &t.sigs[&k]))
+            .collect();
+        let locs: Vec<_> = sorted(&t.method_locs)
+            .into_iter()
+            .map(|k| (k, t.method_locs[&k]))
+            .collect();
+        let supers: Vec<_> = sorted(&t.supertraits)
+            .into_iter()
+            .map(|k| (k, &t.supertraits[&k]))
+            .collect();
+        let selves: Vec<_> = sorted(&t.impl_selves)
+            .into_iter()
+            .map(|k| (k, &t.impl_selves[&k]))
+            .collect();
+        let arity: Vec<_> = sorted(&t.type_arity)
+            .into_iter()
+            .map(|k| (k, t.type_arity[&k]))
+            .collect();
+        format!(
+            "{sigs:?}\n{locs:?}\n{:?}\n{:?}\n{:?}\n{:?}\n{supers:?}\n{:?}\n{:?}\n{:?}\n{selves:?}\n\
+             {arity:?}\n{:?}\n{:?}\n{:?}\n{:?}",
+            set(&t.selfless),
+            set(&t.mut_self),
+            t.sums,
+            t.records,
+            t.traits,
+            t.impls,
+            t.impl_param_names,
+            t.externs,
+            t.impl_spans,
+            t.impl_in_std,
+            t.impl_method_spans,
+        )
+    }
+
+    const PROGRAMS: &[&str] = &[
+        "fn main() {}\n",
+        "record P<T> { x: T }\ntype S = | A(Int) | B\ntrait Show {\n    type Item\n    \
+         fn show(self) -> String\n}\nimpl<T> Show for P<T> {\n    type Item = T\n    \
+         fn show(self) -> String { \"p\" }\n}\nfn f<T: Show>(x: T) -> T::Item { panic(\"\") }\n\
+         fn main() {}\n",
+        "fn f<T: Nope, T>(x: Missing) -> Int { 1 }\nimpl Eq for Int {\n    \
+         fn eq(self, other: Int) -> Bool { true }\n}\nimpl [Int] {\n    fn len(self) -> Int { 0 }\n}\n\
+         trait A: B {}\nimpl A for Bool {}\nextern \"C\" {\n    fn nova_x(a: String) -> [Int]\n}\n\
+         fn main() {}\n",
+        "impl<T> Ord for [T] {}\ntrait It {\n    type Item\n    type Item\n    fn g(self)\n    \
+         fn g(self)\n}\nimpl It for Int {\n    type Item = Self::Item\n    fn h(self) {}\n}\n\
+         fn main() {}\n",
+    ];
+
+    /// `check_items` is the item stage of `check_with`: its diagnostics are
+    /// the prefix of `check_with`'s, and its tables the module's.
+    #[test]
+    fn check_items_is_the_prefix_of_check_with() {
+        for src in PROGRAMS {
+            let ast = parse(src);
+            let r = nova_resolver::resolve(&ast);
+            let options = CheckOptions::default();
+            let items = check_items(&r.file, &r.definitions, &options);
+            let all = check_with(&r.file, &r.definitions, &options);
+            let n = items.diagnostics.len();
+            assert!(all.diagnostics.len() >= n, "{src}");
+            assert_eq!(
+                format!("{:?}", &all.diagnostics[..n]),
+                format!("{:?}", items.diagnostics),
+                "{src}"
+            );
+            let m = &all.module;
+            assert_eq!(format!("{:?}", m.sums), format!("{:?}", items.sums));
+            assert_eq!(format!("{:?}", m.records), format!("{:?}", items.records));
+            assert_eq!(format!("{:?}", m.traits), format!("{:?}", items.traits));
+            assert_eq!(format!("{:?}", m.impls), format!("{:?}", items.impls));
+            assert_eq!(format!("{:?}", m.externs), format!("{:?}", items.externs));
+        }
+    }
+
+    /// The shared path gives a program what the full check gives it, with a
+    /// std of one more module whose trait and impl a program's items pair
+    /// with (E0074, E0105's kin and conformance), and std alone with an
+    /// item diagnostic is not shareable, so the driver checks in full.
+    #[test]
+    fn shared_items_check_what_the_full_check_does() {
+        let extra = (
+            "$std.zz",
+            "pub trait Zed {\n    fn z(self) -> Int\n}\nimpl Zed for Int {\n    \
+             fn z(self) -> Int { 1 }\n}\nimpl Bool {\n    pub fn z(self) -> Int { 2 }\n}\n\
+             extern \"C\" {\n    fn zz_abs(x: Int) -> Int\n}\n",
+        );
+        let program = "impl Zed for Int {\n    fn z(self) -> Int { 3 }\n}\n\
+                       impl Zed for Bool {\n    fn z(self) -> Int { 4 }\n}\n\
+                       record R { z: Option<Int> }\nfn main() {}\n";
+        let std = std_with(Some(extra));
+        let shared_std = SharedStd::new(CompileTarget::Native, std_with(Some(extra)));
+        assert!(shared_std.shareable());
+        let alone = shared_std.alone();
+        let options = CheckOptions::default();
+        let items = SharedItems::new(&alone.file, &alone.definitions, &options);
+        assert!(items.shareable());
+        let ast = parse(program);
+        let sources = [ModuleSource::new("main", &ast)];
+        let full = nova_resolver::resolve_program_with_std(CompileTarget::Native, &sources, &std);
+        let full_tables = check_items(&full.file, &full.definitions, &options);
+        let shared =
+            nova_resolver::resolve_program_with_shared_std_for_check(&sources, &shared_std);
+        let shared_tables = check_items_shared(&shared.file, &shared.definitions, &options, &items);
+        let text = shown(&full, &full_tables);
+        assert!(text.contains("E0074"), "{text}");
+        assert!(text.contains("E0105"), "{text}");
+        assert_eq!(text, shown(&shared, &shared_tables));
+        // Every table, std's part included: the splices add, rebased, what
+        // the full passes collect, though no item pass reads some of it
+        // (std's records, sums, method places and impl selves are the
+        // bodies', 5.3b's).
+        assert_eq!(tables_text(&full_tables), tables_text(&shared_tables));
+
+        let broken = ("$std.zz", "pub record Bad { x: Missing }\n");
+        let alone = nova_resolver::resolve_program_with_std(
+            CompileTarget::Native,
+            &[],
+            &std_with(Some(broken)),
+        );
+        let items = SharedItems::new(&alone.file, &alone.definitions, &options);
+        assert!(!items.shareable());
+    }
+}
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index 199dabe8..a7e458fb 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -478,7 +478,8 @@ pub fn check(cmd: CheckCmd) -> Result<()> {
     }
 }
 
-/// `nova check --dump-resolution …` (spec
+/// `nova check --dump-resolution …` and, with this parser's arguments,
+/// `nova check --dump-hir …` (5.3a, ADR 0055; [`DumpCmd`]) (spec
 /// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
 /// §3.4, ADR 0053 §1): the resolution dump, for the self-hosted resolver's
 /// differential test. Hidden, with no stability promise. It is taken only
@@ -520,63 +521,129 @@ pub struct DumpResolutionCmd {
     files: Vec<PathBuf>,
 }
 
-impl DumpResolutionCmd {
+/// Which hidden dump a command line asks for: the resolution dump (5.2,
+/// ADR 0053) or the HIR dump (5.3a, ADR 0055), each with
+/// [`DumpResolutionCmd`]'s arguments and refusals.
+#[derive(Clone, Copy, PartialEq, Eq)]
+pub enum DumpKind {
+    Resolution,
+    Hir,
+}
+
+impl DumpKind {
+    /// The flag, as the form's errors name it.
+    fn flag(self) -> &'static str {
+        match self {
+            DumpKind::Resolution => "--dump-resolution",
+            DumpKind::Hir => "--dump-hir",
+        }
+    }
+}
+
+/// A hidden dump's command line: which dump, and its arguments.
+pub struct DumpCmd {
+    kind: DumpKind,
+    args: DumpResolutionCmd,
+}
+
+impl DumpCmd {
     /// The hidden command, parsed from `args` (a process's arguments,
-    /// `argv[0]` first), when they are `check --dump-resolution` and more;
-    /// `None` for every other command line, which the ordinary parser
-    /// takes. A misuse after the flag exits as clap's errors do.
+    /// `argv[0]` first), when they are `check --dump-resolution` or `check
+    /// --dump-hir` and more; `None` for every other command line, which the
+    /// ordinary parser takes. A misuse after the flag exits as clap's
+    /// errors do, under the usage `nova check --dump-resolution` or `nova
+    /// check --dump-hir`.
     pub fn from_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Option<Self> {
         let mut args = args.into_iter().skip(1);
-        if args.next()? != "check" || args.next()? != "--dump-resolution" {
+        if args.next()? != "check" {
             return None;
         }
-        Some(<Self as clap::Parser>::parse_from(args))
+        let flag = args.next()?;
+        if flag == "--dump-resolution" {
+            return Some(DumpCmd {
+                kind: DumpKind::Resolution,
+                args: <DumpResolutionCmd as clap::Parser>::parse_from(args),
+            });
+        }
+        if flag != "--dump-hir" {
+            return None;
+        }
+        let name = "nova check --dump-hir";
+        let matches = <DumpResolutionCmd as clap::CommandFactory>::command()
+            .name(name)
+            .bin_name(name)
+            .get_matches_from(args);
+        let args = <DumpResolutionCmd as clap::FromArgMatches>::from_arg_matches(&matches)
+            .unwrap_or_else(|e| e.exit());
+        Some(DumpCmd {
+            kind: DumpKind::Hir,
+            args,
+        })
     }
 }
 
-/// Run [`DumpResolutionCmd`]: with `--std`, std's part for the target;
-/// otherwise, for each file, `== FILE` (the path as given), then its
-/// program's part, or `IOERR` and the error's kind for a root that cannot
-/// be read, after which the rest are dumped and the exit code is 2, as
-/// `parse --dump-ast` does; with `--test`, each as `nova test` resolves
-/// it. With `--test` and `--target wasm`, with neither a file nor `--std`,
-/// or with both, it prints one line to stderr, nothing to stdout, and
-/// exits 2, as N does.
-pub fn dump_resolution(cmd: DumpResolutionCmd) -> Result<()> {
-    let code = dump_resolution_code(&cmd)?;
+/// Run [`DumpCmd`]: with `--std`, std's part for the target; otherwise, for
+/// each file, `== FILE` (the path as given), then its program's part, or
+/// `IOERR` and the error's kind for a root that cannot be read, after
+/// which the rest are dumped and the exit code is 2, as `parse --dump-ast`
+/// does; with `--test`, each as `nova test` resolves and checks it. With
+/// `--test` and `--target wasm`, with neither a file nor `--std`, or with
+/// both, it prints one line to stderr, nothing to stdout, and exits 2, as
+/// N does.
+pub fn dump(cmd: DumpCmd) -> Result<()> {
+    let code = dump_code(&cmd)?;
     if code != 0 {
         std::process::exit(code);
     }
     Ok(())
 }
 
-fn dump_resolution_code(cmd: &DumpResolutionCmd) -> Result<i32> {
-    let target: nova_driver::Target = cmd.target.into();
+/// One program's part of a dump, from its root.
+type DumpOne = dyn Fn(&Path) -> std::io::Result<String>;
+
+fn dump_code(cmd: &DumpCmd) -> Result<i32> {
+    let flag = cmd.kind.flag();
+    let args = &cmd.args;
+    let target: nova_driver::Target = args.target.into();
     let stdout = std::io::stdout();
     let mut out = stdout.lock();
-    if cmd.test && target == nova_driver::Target::Wasm {
-        eprintln!("error: --dump-resolution --test is native only, as nova test is");
+    if args.test && target == nova_driver::Target::Wasm {
+        eprintln!("error: {flag} --test is native only, as nova test is");
         return Ok(2);
     }
-    if cmd.std {
-        if !cmd.files.is_empty() {
-            eprintln!("error: --dump-resolution --std takes no FILE");
+    if args.std {
+        if !args.files.is_empty() {
+            eprintln!("error: {flag} --std takes no FILE");
             return Ok(2);
         }
-        out.write_all(nova_driver::dump_std_resolution(target, cmd.test).as_bytes())?;
+        let std = match cmd.kind {
+            DumpKind::Resolution => nova_driver::dump_std_resolution(target, args.test),
+            DumpKind::Hir => nova_driver::dump_std_hir(target, args.test),
+        };
+        out.write_all(std.as_bytes())?;
         out.flush()?;
         return Ok(0);
     }
-    if cmd.files.is_empty() {
-        eprintln!("error: --dump-resolution needs a FILE, or --std");
+    if args.files.is_empty() {
+        eprintln!("error: {flag} needs a FILE, or --std");
         return Ok(2);
     }
-    // std is parsed once for every program, as N parses it.
-    let dumps = nova_driver::ResolutionDumps::new(target, cmd.test);
+    // std is parsed once for every program, as N parses it, and resolved
+    // and its items collected once.
+    let dumps: Box<DumpOne> = match cmd.kind {
+        DumpKind::Resolution => {
+            let dumps = nova_driver::ResolutionDumps::new(target, args.test);
+            Box::new(move |file| dumps.dump(file))
+        }
+        DumpKind::Hir => {
+            let dumps = nova_driver::HirDumps::new(target, args.test);
+            Box::new(move |file| dumps.dump(file))
+        }
+    };
     let mut code = 0;
-    for file in &cmd.files {
+    for file in &args.files {
         writeln!(out, "== {}", file.display())?;
-        match dumps.dump(file) {
+        match dumps(file) {
             Ok(dump) => out.write_all(dump.as_bytes())?,
             Err(e) => {
                 writeln!(out, "IOERR {}", crate::cmd::parse::io_kind_name(e.kind()))?;
diff --git a/crates/nova-cli/src/main.rs b/crates/nova-cli/src/main.rs
index 69d679e7..f58199b1 100644
--- a/crates/nova-cli/src/main.rs
+++ b/crates/nova-cli/src/main.rs
@@ -99,16 +99,17 @@ enum Ran {
 }
 
 /// What a command line asks for: a command [`Cli`] parses, or the hidden
-/// `nova check --dump-resolution`, which has a parser of its own, so that
-/// `check`'s is as it was ([`cmd::run::DumpResolutionCmd`]).
+/// `nova check --dump-resolution` or `nova check --dump-hir`, which have a
+/// parser of their own, so that `check`'s is as it was
+/// ([`cmd::run::DumpCmd`]).
 enum Job {
     Cli(Command),
-    DumpResolution(cmd::run::DumpResolutionCmd),
+    Dump(cmd::run::DumpCmd),
 }
 
 fn main() -> Result<()> {
-    let job = match cmd::run::DumpResolutionCmd::from_args(std::env::args_os()) {
-        Some(cmd) => Job::DumpResolution(cmd),
+    let job = match cmd::run::DumpCmd::from_args(std::env::args_os()) {
+        Some(cmd) => Job::Dump(cmd),
         None => Job::Cli(Cli::parse().command),
     };
     let filter = tracing_subscriber::EnvFilter::from_default_env()
@@ -141,7 +142,7 @@ fn main() -> Result<()> {
         .stack_size(nova_driver::PIPELINE_STACK)
         .spawn(move || match job {
             Job::Cli(command) => dispatch(command),
-            Job::DumpResolution(cmd) => Ran::Done(cmd::run::dump_resolution(cmd)),
+            Job::Dump(cmd) => Ran::Done(cmd::run::dump(cmd)),
         });
     let ran = match thread {
         Ok(thread) => thread.join(),
diff --git a/crates/nova-driver/src/lib.rs b/crates/nova-driver/src/lib.rs
index 1be3e1eb..cb9bc154 100644
--- a/crates/nova-driver/src/lib.rs
+++ b/crates/nova-driver/src/lib.rs
@@ -17,6 +17,9 @@ pub use nova_codegen_cranelift::CompiledProgram;
 /// test (spec 5.2 §3.4).
 pub use nova_resolver::dump as resolution_dump;
 pub use nova_resolver::CompileTarget as Target;
+/// The HIR dump's forms, for the self-hosted type checker's differential
+/// test (spec 5.3 §3.3).
+pub use nova_typeck::dump as hir_dump;
 pub use program::{package_of, Program, Root, RootKind, Roots};
 pub use runtime_cache::{set_embedded_runtime, EmbeddedRuntime};
 
@@ -179,6 +182,32 @@ impl ResolutionDumps {
     /// dump holds what was built and every diagnostic so far. `Err` when the
     /// root cannot be read.
     pub fn dump(&self, path: &Path) -> std::io::Result<String> {
+        let resolve = if self.shared {
+            Resolve::Shared
+        } else {
+            Resolve::Full
+        };
+        self.front(path, resolve, |front| {
+            nova_resolver::dump::dump_program(
+                self.target,
+                self.test,
+                front.sources,
+                front.labels,
+                front.resolved,
+                &front.diagnostics,
+            )
+        })
+    }
+
+    /// The pipeline [`ResolutionDumps::dump`] describes, up to the
+    /// resolver, resolving as `resolve` says, handed to `then`. `Err` when
+    /// the root cannot be read.
+    fn front<R>(
+        &self,
+        path: &Path,
+        resolve: Resolve,
+        then: impl FnOnce(Front<'_>) -> R,
+    ) -> std::io::Result<R> {
         let db = self.std_db.clone();
         let mut labels = self.std_labels.clone();
         let Program {
@@ -191,14 +220,14 @@ impl ResolutionDumps {
         labels.extend(manifest_labels(graph.as_ref(), &graph_diagnostics));
         let has_error = |ds: &[Diagnostic]| ds.iter().any(|d| d.severity == Severity::Error);
         if has_error(&graph_diagnostics) {
-            return Ok(nova_resolver::dump::dump_program(
-                self.target,
-                self.test,
-                &[],
-                &labels,
-                None,
-                &graph_diagnostics,
-            ));
+            return Ok(then(Front {
+                db: &db,
+                sources: &[],
+                labels: &labels,
+                resolved: None,
+                diagnostics: graph_diagnostics,
+                registry: &HashSet::new(),
+            }));
         }
         let load = program::load_program(graph.as_ref(), &roots, &DiskSources, &mut db)?;
         let mut modules = load.modules;
@@ -228,10 +257,12 @@ impl ResolutionDumps {
                 importable: m.importable.clone(),
             })
             .collect();
-        let mut resolved = (!failed).then(|| {
-            if self.shared {
-                nova_resolver::resolve_program_with_shared_std(&sources, &self.std)
-            } else {
+        let mut resolved = (!failed).then(|| match resolve {
+            Resolve::Shared => nova_resolver::resolve_program_with_shared_std(&sources, &self.std),
+            Resolve::SharedForCheck => {
+                nova_resolver::resolve_program_with_shared_std_for_check(&sources, &self.std)
+            }
+            Resolve::Full => {
                 nova_resolver::resolve_program_with_std(self.target, &sources, self.std.parsed())
             }
         });
@@ -239,15 +270,164 @@ impl ResolutionDumps {
             name_tests(&mut resolved.tests, &resolved.definitions, &modules);
             diagnostics.extend(shown(&resolved.diagnostics));
         }
-        Ok(nova_resolver::dump::dump_program(
-            self.target,
-            self.test,
-            &sources,
-            &labels,
-            resolved.as_ref(),
-            &diagnostics,
-        ))
+        Ok(then(Front {
+            db: &db,
+            sources: &sources,
+            labels: &labels,
+            resolved: resolved.as_ref(),
+            diagnostics,
+            registry: &registry,
+        }))
+    }
+}
+
+/// How [`ResolutionDumps::front`] resolves a program: with std resolved
+/// once, for the resolution dump or, with std's definitions, for the type
+/// checker; or in full, as `check` does.
+#[derive(Clone, Copy)]
+enum Resolve {
+    Shared,
+    SharedForCheck,
+    Full,
+}
+
+/// What [`ResolutionDumps::front`] built: the files, the modules, their
+/// labels, the resolution once it got that far, every diagnostic so far,
+/// and the registry packages' files, whose warnings are not shown.
+struct Front<'a> {
+    db: &'a FileDb,
+    sources: &'a [ModuleSource<'a>],
+    labels: &'a [(FileId, String)],
+    resolved: Option<&'a nova_resolver::ProgramResolution>,
+    diagnostics: Vec<Diagnostic>,
+    registry: &'a HashSet<FileId>,
+}
+
+/// The HIR dumps of programs for one target and mode (`nova_typeck::dump`;
+/// spec
+/// `docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
+/// §3.3 and §3.4, ADR 0055): [`ResolutionDumps`]'s pipeline, then, when it
+/// stopped on no error, the type checker's item passes with `check`'s
+/// options (the sources, no probe, no index). std is parsed, resolved and
+/// its items collected once, and each program reads std's tables rebased
+/// past its own ([`nova_typeck::check_items_shared`]); should std alone give
+/// a diagnostic or hold a test, every program is checked in full.
+pub struct HirDumps {
+    front: ResolutionDumps,
+    items: nova_typeck::SharedItems,
+    shared: bool,
+}
+
+impl HirDumps {
+    /// std's modules for `target`, parsed, resolved and their items
+    /// collected once, under test mode with `std/test` (`test`).
+    pub fn new(target: Target, test: bool) -> HirDumps {
+        let front = ResolutionDumps::new(target, test);
+        let alone = front.std.alone();
+        let items = nova_typeck::SharedItems::new(
+            &alone.file,
+            &alone.definitions,
+            &nova_typeck::CheckOptions {
+                sources: Some(&front.std_db),
+                ..Default::default()
+            },
+        );
+        let shared = front.std.shareable() && items.shareable();
+        HirDumps {
+            front,
+            items,
+            shared,
+        }
     }
+
+    /// [`HirDumps::new`], each program resolving and checking std again,
+    /// as `check` does: the dumps are the same, which a test checks.
+    pub fn unshared(target: Target, test: bool) -> HirDumps {
+        HirDumps {
+            shared: false,
+            ..HirDumps::new(target, test)
+        }
+    }
+
+    /// The HIR dump of the program whose root is `path`, taken as
+    /// [`ResolutionDumps::dump`] takes it: the type checker's item tables
+    /// and every diagnostic so far, or, for a program that stopped before
+    /// the type checker, `checked=false` and the diagnostics. `Err` when
+    /// the root cannot be read.
+    pub fn dump(&self, path: &Path) -> std::io::Result<String> {
+        let resolve = if self.shared {
+            Resolve::SharedForCheck
+        } else {
+            Resolve::Full
+        };
+        let target = self.front.target;
+        let test = self.front.test;
+        self.front.front(path, resolve, |front| {
+            let stopped = front
+                .diagnostics
+                .iter()
+                .any(|d| d.severity == Severity::Error);
+            let Some(resolved) = front.resolved.filter(|_| !stopped) else {
+                return nova_typeck::dump::dump_program(
+                    target,
+                    test,
+                    front.labels,
+                    None,
+                    &front.diagnostics,
+                );
+            };
+            let options = nova_typeck::CheckOptions {
+                sources: Some(front.db),
+                ..Default::default()
+            };
+            let tables = if self.shared {
+                nova_typeck::check_items_shared(
+                    &resolved.file,
+                    &resolved.definitions,
+                    &options,
+                    &self.items,
+                )
+            } else {
+                nova_typeck::check_items(&resolved.file, &resolved.definitions, &options)
+            };
+            let mut diagnostics = front.diagnostics;
+            diagnostics.extend(
+                tables
+                    .diagnostics
+                    .iter()
+                    .filter(|d| !program::a_dependencys_warning(d, front.registry))
+                    .cloned(),
+            );
+            nova_typeck::dump::dump_program(
+                target,
+                test,
+                front.labels,
+                Some((resolved, &tables)),
+                &diagnostics,
+            )
+        })
+    }
+}
+
+/// std's part of the HIR dump for `target`, under test mode with `std/test`
+/// (`test`): the item tables of a program of no module of its own, which
+/// are std's in every program, and every diagnostic, the resolver's and the
+/// item passes'.
+pub fn dump_std_hir(target: Target, test: bool) -> String {
+    let mut db = FileDb::new();
+    let (std_files, extra, labels) = register_std_for(&mut db, target, test);
+    let resolved = nova_resolver::resolve_program_for(target, &[], &std_files, extra);
+    let tables = nova_typeck::check_items(
+        &resolved.file,
+        &resolved.definitions,
+        &nova_typeck::CheckOptions {
+            sources: Some(&db),
+            ..Default::default()
+        },
+    );
+    let mut diagnostics = resolved.diagnostics.clone();
+    diagnostics.extend(tables.diagnostics.iter().cloned());
+    nova_typeck::dump::dump_std(target, test, &labels, &resolved, &tables, &diagnostics)
 }
 
 /// The label of each manifest a program's graph read (ADR 0054 §1):
diff --git a/crates/nova-resolver/src/dump.rs b/crates/nova-resolver/src/dump.rs
index d3cea0a3..95450147 100644
--- a/crates/nova-resolver/src/dump.rs
+++ b/crates/nova-resolver/src/dump.rs
@@ -238,6 +238,37 @@ pub fn std_part_of(
     d.out
 }
 
+/// Where std begins in `defs`, as `(StdStart …)` prints it: the first std
+/// module, the first std definition and the first item of a std module.
+/// The HIR dump repeats it (5.3a, ADR 0055).
+pub fn std_start(defs: &Definitions) -> (usize, u32, usize) {
+    let std_module = defs.std_start;
+    let std_item = defs
+        .item_module
+        .iter()
+        .position(|&m| m as usize >= std_module)
+        .unwrap_or(defs.item_module.len());
+    (std_module, first_std_def(defs, std_item), std_item)
+}
+
+/// `diagnostics` as `(Diagnostics …)` prints them, after `out`, each place
+/// under its file's label in `labels`: the HIR dump prints them so too
+/// (5.3a, ADR 0055).
+pub fn write_diagnostics(out: &mut String, labels: &Labels, diagnostics: &[Diagnostic]) {
+    let mut d = Dump::new(labels, u32::MAX);
+    d.out = std::mem::take(out);
+    d.diagnostics(diagnostics);
+    *out = d.out;
+}
+
+/// ` ` and a place, as the dump prints one, after `out`.
+pub fn write_place(out: &mut String, labels: &Labels, span: Span) {
+    let mut d = Dump::new(labels, u32::MAX);
+    d.out = std::mem::take(out);
+    d.place(span);
+    *out = d.out;
+}
+
 /// The first std definition's index: the first definition of an item of a
 /// std module. The user's definitions all come before std's, since pass 1
 /// collects the modules in order, and every kind of definition but an
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 1159c07d..3c710c94 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -2336,11 +2336,13 @@ pub fn resolve_program_with_std(
 pub struct SharedStd {
     target: CompileTarget,
     std: ParsedStd,
-    /// Each std module's exports, its definitions numbered from 0.
-    exports: Vec<Exports>,
+    /// The resolution of a program of no module of its own: std's
+    /// definitions, scopes, items and exports, numbered from 0, which the
+    /// type checker's shared path reads too (5.3a,
+    /// [`resolve_program_with_shared_std_for_check`]).
+    alone: ProgramResolution,
     /// [`std_globbed_variants`] of std.
     variants: FxHashMap<String, String>,
-    missing: FxHashMap<String, &'static str>,
     /// Whether resolving std gave no diagnostic of its own, beyond parsing
     /// it, and found no `@test` function: a diagnostic would come between
     /// a program's, and a test would be listed with a program's, which the
@@ -2355,6 +2357,18 @@ impl SharedStd {
         &self.std
     }
 
+    /// The resolution of a program of no module of its own, from which
+    /// std's item tables are built once (5.3a).
+    pub fn alone(&self) -> &ProgramResolution {
+        &self.alone
+    }
+
+    /// Whether a program reads std resolved once: resolving std gave no
+    /// diagnostic of its own and found no `@test` function.
+    pub fn shareable(&self) -> bool {
+        self.shareable
+    }
+
     /// `std`, parsed for `target`, resolved once.
     pub fn new(target: CompileTarget, std: ParsedStd) -> SharedStd {
         let alone = resolve_program_with_std(target, &[], &std);
@@ -2371,9 +2385,8 @@ impl SharedStd {
         let shareable = alone.diagnostics.len() == std.diagnostics.len() && alone.tests.is_empty();
         SharedStd {
             target,
-            exports: alone.definitions.exports,
+            alone,
             variants,
-            missing: alone.definitions.missing,
             shareable,
             std,
         }
@@ -2461,7 +2474,14 @@ pub fn resolve_program_with_shared_std(
     }
     // std's definitions come after the user's.
     let offset = definitions.defs.len() as u32;
-    exports.extend(shared.exports.iter().map(|e| e.rebased(offset)));
+    exports.extend(
+        shared
+            .alone
+            .definitions
+            .exports
+            .iter()
+            .map(|e| e.rebased(offset)),
+    );
     if target == CompileTarget::Wasm {
         for m in &all[..std_start] {
             for item in &m.file.items {
@@ -2540,7 +2560,7 @@ pub fn resolve_program_with_shared_std(
     definitions.std_start = std_start;
     definitions.exports = exports;
     definitions.target = target;
-    definitions.missing = shared.missing.clone();
+    definitions.missing = shared.alone.definitions.missing.clone();
     ProgramResolution {
         file: File {
             items: merged,
@@ -2556,6 +2576,121 @@ pub fn resolve_program_with_shared_std(
     }
 }
 
+/// [`resolve_program_with_shared_std`] for the type checker's shared path
+/// (5.3a, the hidden HIR dump alone): std's definitions, module scopes and
+/// items' modules follow the program's, rebased past them, as the full
+/// resolution numbers them, so the type checker can read std's
+/// definitions; std's items are not in `file`, since std's item tables
+/// are shared too (`nova_typeck::check_items_shared`). A program resolved
+/// in full, because std's gave a diagnostic or holds a `@test` function,
+/// is returned as [`resolve_program_with_std`] gives it.
+pub fn resolve_program_with_shared_std_for_check(
+    modules: &[ModuleSource],
+    shared: &SharedStd,
+) -> ProgramResolution {
+    let mut resolved = resolve_program_with_shared_std(modules, shared);
+    if !shared.shareable {
+        return resolved;
+    }
+    let defs = &mut resolved.definitions;
+    let def_offset = defs.defs.len() as u32;
+    let item_offset = resolved.file.items.len();
+    let module_offset = defs.std_start as u32;
+    let std = &shared.alone.definitions;
+    defs.defs
+        .extend(std.defs.iter().map(|d| d.rebased(def_offset, item_offset)));
+    defs.modules
+        .extend(std.modules.iter().map(|m| m.rebased(def_offset)));
+    defs.item_module
+        .extend(std.item_module.iter().map(|m| m + module_offset));
+    resolved
+}
+
+impl Def {
+    /// This definition with every definition's id `def` higher and every
+    /// item's index `item` higher, as std's are in a program whose own come
+    /// first.
+    fn rebased(&self, def: u32, item: usize) -> Def {
+        let kind = match &self.kind {
+            DefKind::Fn { item_index } => DefKind::Fn {
+                item_index: item_index + item,
+            },
+            DefKind::Sum {
+                item_index,
+                variants,
+            } => DefKind::Sum {
+                item_index: item_index + item,
+                variants: variants.clone(),
+            },
+            DefKind::Record { item_index } => DefKind::Record {
+                item_index: item_index + item,
+            },
+            DefKind::Const { item_index } => DefKind::Const {
+                item_index: item_index + item,
+            },
+            DefKind::Trait { item_index } => DefKind::Trait {
+                item_index: item_index + item,
+            },
+            DefKind::Method {
+                item_index,
+                method_index,
+                owner,
+            } => DefKind::Method {
+                item_index: item_index + item,
+                method_index: *method_index,
+                owner: *owner,
+            },
+            DefKind::ExternFn {
+                item_index,
+                fn_index,
+            } => DefKind::ExternFn {
+                item_index: item_index + item,
+                fn_index: *fn_index,
+            },
+            DefKind::AssocType { trait_def } => DefKind::AssocType {
+                trait_def: DefId(trait_def.0 + def),
+            },
+        };
+        Def {
+            name: self.name.clone(),
+            span: self.span,
+            kind,
+        }
+    }
+}
+
+impl ModuleScope {
+    /// This scope with every definition's id `offset` higher.
+    fn rebased(&self, offset: u32) -> ModuleScope {
+        let def = |id: DefId| DefId(id.0 + offset);
+        ModuleScope {
+            name: self.name.clone(),
+            values: self
+                .values
+                .iter()
+                .map(|(k, r)| {
+                    let r = match *r {
+                        Res::Def(id) => Res::Def(def(id)),
+                        Res::Variant(sum, index) => Res::Variant(def(sum), index),
+                        Res::Builtin(b) => Res::Builtin(b),
+                    };
+                    (k.clone(), r)
+                })
+                .collect(),
+            types: self
+                .types
+                .iter()
+                .map(|(k, id)| (k.clone(), def(*id)))
+                .collect(),
+            traits: self
+                .traits
+                .iter()
+                .map(|(k, id)| (k.clone(), def(*id)))
+                .collect(),
+        }
+    }
+}
+
 /// The variants std brings into every module, by name: each public variant
 /// of a std module whose names glob into user modules, with its sum's name.
 /// The first module to export a name wins, as std's glob does.
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 810a5861..34d317ec 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -33,20 +33,22 @@ mod tuple_tests;
 mod view;
 
 use fixes::{Binding, Need};
+pub use items::{check_items, check_items_shared, ItemTables, SharedItems};
+use items::{Rebase, Splice};
 use patterns::PatState;
 
 /// A collected function (or method) signature.
 #[derive(Debug, Clone)]
-struct FnSig {
-    generics: u32,
+pub(crate) struct FnSig {
+    pub(crate) generics: u32,
     /// Trait bounds per generic parameter.
-    bounds: Vec<Vec<DefId>>,
+    pub(crate) bounds: Vec<Vec<DefId>>,
     /// Parameter types. For a method that declares a `self` receiver,
     /// `params[0]` is that receiver. An associated function (an impl method with
     /// no `self`, tracked in `Checker::selfless`) holds only its declared
     /// parameters, so `params` stays positionally aligned with the AST's.
-    params: Vec<Ty>,
-    ret: Ty,
+    pub(crate) params: Vec<Ty>,
+    pub(crate) ret: Ty,
 }
 
 /// Outcome of resolving a method name against a receiver type.
@@ -116,7 +118,7 @@ enum TraitCallSelf<'a> {
 /// by-index bound table either. Its candidate traits are not bounds at all but
 /// the single trait the impl implements.
 #[derive(Debug, Clone)]
-struct ImplSelf {
+pub(crate) struct ImplSelf {
     /// The impl's self type, with the impl's own `Param(k)` still in it.
     ty: Ty,
     /// The trait this impl implements, or `None` for an inherent impl — which
@@ -126,10 +128,10 @@ struct ImplSelf {
 
 /// The AST location and flavor of a method to compile.
 #[derive(Debug, Clone, Copy)]
-struct MethodLoc {
-    item_index: usize,
-    method_index: usize,
-    owner: MethodOwner,
+pub(crate) struct MethodLoc {
+    pub(crate) item_index: usize,
+    pub(crate) method_index: usize,
+    pub(crate) owner: MethodOwner,
 }
 
 /// Type-check a parsed file against its resolved definitions.
@@ -139,74 +141,14 @@ pub fn check(file: &ast::File, defs: &Definitions) -> CheckResult {
 
 /// [`check`], with the language server's probe (spec
 /// `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §4.1).
+///
+/// The item stage, passes 1 to 5, is [`check_items`] (5.3a); the bodies,
+/// the constant cycles, the newline splits and the lifted functions follow
+/// it here.
 pub fn check_with(file: &ast::File, defs: &Definitions, options: &CheckOptions) -> CheckResult {
-    let mut checker = Checker {
-        file,
-        defs,
-        cur_module: ModuleId(0),
-        sigs: FxHashMap::default(),
-        method_locs: FxHashMap::default(),
-        selfless: FxHashSet::default(),
-        mut_self: FxHashSet::default(),
-        sums: Vec::new(),
-        records: Vec::new(),
-        supertraits: FxHashMap::default(),
-        traits: Vec::new(),
-        impls: Vec::new(),
-        impl_param_names: Vec::new(),
-        impl_self: None,
-        impl_selves: FxHashMap::default(),
-        extra_functions: Vec::new(),
-        next_closure_def: defs.defs().len() as u32,
-        type_arity: FxHashMap::default(),
-        externs: Vec::new(),
-        diagnostics: Vec::new(),
-        probe: options.probe,
-        probe_result: ProbeResult::default(),
-        probe_pending: None,
-        probe_locals_pending: None,
-        index: options.index.then(Index::default),
-        type_params: Vec::new(),
-        sources: options.sources,
-        bindings: FxHashMap::default(),
-        qualified_callee: None,
-        pats: PatState::default(),
-        quiet_splits: FxHashSet::default(),
-    };
-    // Before any collection pass: every later pass builds a generic scope, and
-    // one containing `Self` would give the name two meanings at once.
-    checker.note_item_declarations();
-    checker.reject_self_type_params();
-    checker.collect_type_arities();
-    // Before `collect_records`: since Task 1 of the iterator-finishing plan,
-    // `collect_records` resolves each record's bounds and calls
-    // `expand_bounds`, which folds transitive supertraits in from
-    // `Checker::supertraits` — the very table `collect_supertraits` builds.
-    // Moved ahead of `collect_records` (and `collect_sums`, which never reads
-    // it but has no reason to run first either) so that table is populated
-    // before the first `expand_bounds` call, not just before
-    // `collect_traits`'s. `collect_supertraits` itself only reads trait
-    // declarations via `self.defs` (already fully resolved before `check`
-    // runs) and `decl.supertraits`, so it has no ordering dependency on
-    // `collect_type_arities`, `collect_records`, or `collect_sums`.
-    //
-    // It did change the *order* diagnostics are rendered in, and nothing sorts
-    // `self.diagnostics` before rendering, so pass order is emission order.
-    // Measured on one program carrying an unresolvable supertrait, a duplicate
-    // record generic and a sum-type bound: `E0001` -> `E0403` -> `E0900` now,
-    // `E0403` -> `E0900` -> `E0001` with this call back in its old slot (the
-    // move was re-run in both directions to confirm it, not inferred from the
-    // pass list). Cosmetic — every diagnostic is still reported, with its own
-    // span — and recorded rather than fixed: stabilizing order would mean
-    // sorting by span, which is a separate decision affecting every diagnostic
-    // in the compiler.
-    checker.collect_supertraits();
-    checker.collect_records();
-    checker.collect_sums();
-    checker.collect_traits();
-    checker.collect_impls();
-    checker.collect_signatures();
-    checker.collect_externs();
+    let tables = check_items(file, defs, options);
+    let mut checker = Checker::new(file, defs, options, None);
+    checker.restore(tables);
 
     let mut functions = Vec::new();
     let fn_ids: Vec<(DefId, usize)> = defs.functions().collect();
@@ -349,6 +291,18 @@ struct Checker<'a> {
     /// been well typed, so E0103 is not reported there (the user's 5.0b
     /// ruling of 2026-10-27), by the split's span.
     quiet_splits: FxHashSet<Span>,
+    /// Each impl's self type's span, whether its module is std's, and its
+    /// methods' names' spans, by the index of `impls`: what the impl checks
+    /// read, kept for the HIR dump and std's shared tables (5.3a).
+    impl_spans: Vec<Span>,
+    impl_in_std: Vec<bool>,
+    impl_method_spans: Vec<Vec<Span>>,
+    /// The definitions the item passes collect, `0..own_defs`: every one,
+    /// unless std's tables are `shared`, when std's, which come after the
+    /// program's, are not collected again but read from them (5.3a,
+    /// [`check_items_shared`]).
+    own_defs: u32,
+    shared: Option<(&'a SharedItems, Rebase)>,
 }
 
 /// What the probe met, before its function's inference is finished.
@@ -593,7 +547,13 @@ impl<'a> Checker<'a> {
     }
 
     fn collect_records(&mut self) {
-        for (i, def) in self.defs.defs().iter().enumerate() {
+        for (i, def) in self
+            .defs
+            .defs()
+            .iter()
+            .enumerate()
+            .take(self.own_defs as usize)
+        {
             let DefKind::Record { item_index } = &def.kind else {
                 continue;
             };
@@ -644,7 +604,13 @@ impl<'a> Checker<'a> {
     /// last), so arity must not be read from the incrementally-populated tables.
     fn collect_type_arities(&mut self) {
         let mut arities: Vec<(DefId, u32)> = Vec::new();
-        for (i, def) in self.defs.defs().iter().enumerate() {
+        for (i, def) in self
+            .defs
+            .defs()
+            .iter()
+            .enumerate()
+            .take(self.own_defs as usize)
+        {
             let n = match &def.kind {
                 DefKind::Record { item_index } => match &self.file.items[*item_index].value {
                     ast::Item::Record(r) => r.generics.len() as u32,
@@ -664,7 +630,13 @@ impl<'a> Checker<'a> {
     }
 
     fn collect_sums(&mut self) {
-        for (i, def) in self.defs.defs().iter().enumerate() {
+        for (i, def) in self
+            .defs
+            .defs()
+            .iter()
+            .enumerate()
+            .take(self.own_defs as usize)
+        {
             let DefKind::Sum { item_index, .. } = &def.kind else {
                 continue;
             };
@@ -719,6 +691,7 @@ impl<'a> Checker<'a> {
             .defs()
             .iter()
             .enumerate()
+            .take(self.own_defs as usize)
             .filter_map(|(i, d)| match d.kind {
                 DefKind::Trait { item_index } => Some((DefId(i as u32), item_index)),
                 _ => None,
@@ -819,7 +792,13 @@ impl<'a> Checker<'a> {
             .map(|(id, ii, mi, _)| ((ii, mi), id))
             .collect();
 
-        for (i, def) in self.defs.defs().iter().enumerate() {
+        for (i, def) in self
+            .defs
+            .defs()
+            .iter()
+            .enumerate()
+            .take(self.own_defs as usize)
+        {
             let DefKind::Trait { item_index } = def.kind else {
                 continue;
             };
@@ -1048,11 +1027,13 @@ impl<'a> Checker<'a> {
 
         // Signatures for default-method function bodies: generic over Self
         // (`Param(0)`), bounded by the enclosing trait.
+        self.splice(Splice::Traits);
         let trait_items: Vec<(DefId, usize)> = self
             .defs
             .defs()
             .iter()
             .enumerate()
+            .take(self.own_defs as usize)
             .filter_map(|(i, d)| match d.kind {
                 DefKind::Trait { item_index } => Some((DefId(i as u32), item_index)),
                 _ => None,
@@ -1126,6 +1107,7 @@ impl<'a> Checker<'a> {
                 );
             }
         }
+        self.splice(Splice::TraitDefaults);
     }
 
     /// Collect impl blocks into the impl table and method signatures.
@@ -1544,6 +1526,17 @@ impl<'a> Checker<'a> {
         // No impl is in scope for `collect_signatures` / `collect_externs`,
         // which run next.
         self.impl_self = None;
+        self.impl_spans = impl_spans;
+        self.impl_in_std = impl_in_std;
+        self.impl_method_spans = impl_method_spans;
+        self.impl_param_names = impl_param_names;
+        // std's impls come after the program's, as the full check collects
+        // them; the checks below pair the two (5.3a).
+        self.splice(Splice::Impls);
+        let impl_spans = self.impl_spans.clone();
+        let impl_in_std = self.impl_in_std.clone();
+        let impl_method_spans = self.impl_method_spans.clone();
+        let impl_param_names = self.impl_param_names.clone();
 
         // Ordered first among the three post-collection passes so that an
         // `E0072` about a method signature still precedes `check_supertrait_impls`'s
@@ -1557,7 +1550,6 @@ impl<'a> Checker<'a> {
             &impl_param_names,
         );
         self.check_supertrait_impls(&impl_spans);
-        self.impl_param_names = impl_param_names;
     }
 
     /// E0105, a warning: a program's trait impl defines (or takes from the
@@ -2435,6 +2427,9 @@ impl<'a> Checker<'a> {
     fn collect_signatures(&mut self) {
         let fn_ids: Vec<(DefId, usize)> = self.defs.functions().collect();
         for (def_id, item_index) in fn_ids {
+            if !self.own(def_id) {
+                continue;
+            }
             let ast::Item::Function(f) = &self.file.items[item_index].value else {
                 continue;
             };
@@ -2480,6 +2475,9 @@ impl<'a> Checker<'a> {
         // A constant compiles to a zero-argument function returning its
         // value; register its signature here.
         for (def_id, item_index) in self.const_ids() {
+            if !self.own(def_id) {
+                continue;
+            }
             let ast::Item::Const(c) = &self.file.items[item_index].value else {
                 continue;
             };
@@ -2504,6 +2502,9 @@ impl<'a> Checker<'a> {
     fn collect_externs(&mut self) {
         let externs: Vec<(DefId, usize, usize)> = self.defs.extern_functions().collect();
         for (def_id, item_index, fn_index) in externs {
+            if !self.own(def_id) {
+                continue;
+            }
             let ast::Item::Extern(block) = &self.file.items[item_index].value else {
                 continue;
             };
@@ -14816,6 +14817,11 @@ mod tests {
             qualified_callee: None,
             pats: PatState::default(),
             quiet_splits: FxHashSet::default(),
+            impl_spans: Vec::new(),
+            impl_in_std: Vec::new(),
+            impl_method_spans: Vec::new(),
+            own_defs: u32::MAX,
+            shared: None,
         };
         let mut fcx = FnCtx {
             icx: InferCtx::default(),
@@ -14896,6 +14902,11 @@ mod tests {
             qualified_callee: None,
             pats: PatState::default(),
             quiet_splits: FxHashSet::default(),
+            impl_spans: Vec::new(),
+            impl_in_std: Vec::new(),
+            impl_method_spans: Vec::new(),
+            own_defs: u32::MAX,
+            shared: None,
         };
         let mut fcx = FnCtx {
             icx: InferCtx::default(),
diff --git a/crates/nova-typeck/src/check/items.rs b/crates/nova-typeck/src/check/items.rs
index 5e5096d1..4e08407d 100644
--- a/crates/nova-typeck/src/check/items.rs
+++ b/crates/nova-typeck/src/check/items.rs
@@ -14,6 +14,545 @@
 
 use super::*;
 
+/// What passes 1 to 5 build: the item tables, the item diagnostics in
+/// the order the passes emitted them, and the state the bodies start from.
+pub struct ItemTables {
+    pub(crate) sigs: FxHashMap<DefId, FnSig>,
+    pub(crate) method_locs: FxHashMap<DefId, MethodLoc>,
+    pub(crate) selfless: FxHashSet<DefId>,
+    pub(crate) mut_self: FxHashSet<DefId>,
+    pub(crate) sums: Vec<hir::SumType>,
+    pub(crate) records: Vec<hir::RecordType>,
+    pub(crate) supertraits: FxHashMap<DefId, Vec<DefId>>,
+    pub(crate) traits: Vec<hir::TraitDef>,
+    pub(crate) impls: Vec<hir::ImplInfo>,
+    pub(crate) impl_param_names: Vec<Vec<String>>,
+    pub(crate) impl_selves: FxHashMap<usize, ImplSelf>,
+    pub(crate) type_arity: FxHashMap<DefId, u32>,
+    pub(crate) externs: Vec<hir::ExternFn>,
+    /// Each impl's self type's span, whether it is std's, and its methods'
+    /// names' spans, by the index of `impls`: what the impl checks read.
+    pub(crate) impl_spans: Vec<Span>,
+    pub(crate) impl_in_std: Vec<bool>,
+    pub(crate) impl_method_spans: Vec<Vec<Span>>,
+    /// The item diagnostics, in the order the passes emitted them.
+    pub diagnostics: Vec<Diagnostic>,
+    index: Option<Index>,
+    cur_module: ModuleId,
+    type_params: Vec<(String, Span)>,
+    impl_self: Option<ImplSelf>,
+}
+
+/// Passes 1 to 5 of [`check_with`](super::check_with) over `file`: the
+/// item tables and the item diagnostics, which are the prefix of
+/// `check_with`'s diagnostics (5.3a).
+pub fn check_items(file: &ast::File, defs: &Definitions, options: &CheckOptions) -> ItemTables {
+    let mut checker = Checker::new(file, defs, options, None);
+    checker.collect_items();
+    checker.into_tables()
+}
+
+/// std's item tables, built once from the check of a program of no module
+/// of its own, which each program's [`check_items_shared`] reads with std's
+/// ids rebased past its own (spec 5.3 §3.4): std's items are not collected
+/// again. For the hidden HIR dump alone.
+pub struct SharedItems {
+    tables: ItemTables,
+    /// std's definitions and items, numbered from 0.
+    defs: u32,
+    /// Whether std's items gave no diagnostic: one would come between a
+    /// program's, which the shared path drops, so a program is then
+    /// checked in full.
+    shareable: bool,
+}
+
+impl SharedItems {
+    /// std's item tables, from `file` and `defs`, the resolution of a
+    /// program of no module of its own.
+    pub fn new(file: &ast::File, defs: &Definitions, options: &CheckOptions) -> SharedItems {
+        let tables = check_items(file, defs, options);
+        SharedItems {
+            shareable: tables.diagnostics.is_empty(),
+            defs: defs.defs().len() as u32,
+            tables,
+        }
+    }
+
+    /// Whether a program may read these tables ([`SharedItems::shareable`]'s
+    /// field): std's items gave no diagnostic.
+    pub fn shareable(&self) -> bool {
+        self.shareable
+    }
+
+    /// std's tables, as std's part of the dump prints them.
+    pub fn tables(&self) -> &ItemTables {
+        &self.tables
+    }
+}
+
+/// [`check_items`] reading std's tables from `shared`: `file` holds the
+/// program's own items only, and `defs` its definitions followed by std's,
+/// rebased (`nova_resolver::resolve_program_with_shared_std_for_check`).
+/// Each pass runs over the program's items, and std's part of each table is
+/// added where the full check would have collected it, after the program's;
+/// the impl checks then pair the program's impls with std's as the full
+/// check does. The caller checks in full when `shared` is not shareable.
+pub fn check_items_shared(
+    file: &ast::File,
+    defs: &Definitions,
+    options: &CheckOptions,
+    shared: &SharedItems,
+) -> ItemTables {
+    let own_defs = defs.defs().len() as u32 - shared.defs;
+    let rebase = Rebase {
+        def: own_defs,
+        item: file.items.len(),
+    };
+    let mut checker = Checker::new(file, defs, options, Some((shared, rebase)));
+    checker.own_defs = own_defs;
+    checker.collect_items();
+    checker.into_tables()
+}
+
+/// How std's ids move in a program: its definitions come after the
+/// program's `def`, its items after the program's `item`.
+#[derive(Clone, Copy)]
+pub(crate) struct Rebase {
+    def: u32,
+    item: usize,
+}
+
+impl Rebase {
+    fn id(self, id: DefId) -> DefId {
+        DefId(id.0 + self.def)
+    }
+
+    fn ids(self, ids: &[DefId]) -> Vec<DefId> {
+        ids.iter().map(|&id| self.id(id)).collect()
+    }
+
+    fn bounds(self, bounds: &[Vec<DefId>]) -> Vec<Vec<DefId>> {
+        bounds.iter().map(|b| self.ids(b)).collect()
+    }
+
+    fn tys(self, tys: &[Ty]) -> Vec<Ty> {
+        tys.iter().map(|t| self.ty(t)).collect()
+    }
+
+    fn ty(self, ty: &Ty) -> Ty {
+        match ty {
+            Ty::Fn { params, ret } => Ty::Fn {
+                params: self.tys(params),
+                ret: Box::new(self.ty(ret)),
+            },
+            Ty::Future(out) => Ty::Future(Box::new(self.ty(out))),
+            Ty::Sum { def_id, args } => Ty::Sum {
+                def_id: self.id(*def_id),
+                args: self.tys(args),
+            },
+            Ty::Record { def_id, args } => Ty::Record {
+                def_id: self.id(*def_id),
+                args: self.tys(args),
+            },
+            Ty::Array(elem) => Ty::Array(Box::new(self.ty(elem))),
+            Ty::Tuple(elems) => Ty::Tuple(self.tys(elems)),
+            Ty::Assoc { on, assoc } => Ty::Assoc {
+                on: Box::new(self.ty(on)),
+                assoc: self.id(*assoc),
+            },
+            Ty::Int
+            | Ty::Float
+            | Ty::Bool
+            | Ty::Char
+            | Ty::String
+            | Ty::Bytes
+            | Ty::Unit
+            | Ty::Param(_)
+            | Ty::Var(_)
+            | Ty::Never
+            | Ty::Error => ty.clone(),
+        }
+    }
+
+    fn head(self, head: TyHead) -> TyHead {
+        match head {
+            TyHead::Sum(id) => TyHead::Sum(self.id(id)),
+            TyHead::Record(id) => TyHead::Record(self.id(id)),
+            other => other,
+        }
+    }
+
+    fn sig(self, sig: &FnSig) -> FnSig {
+        FnSig {
+            generics: sig.generics,
+            bounds: self.bounds(&sig.bounds),
+            params: self.tys(&sig.params),
+            ret: self.ty(&sig.ret),
+        }
+    }
+
+    fn imp(self, imp: &hir::ImplInfo) -> hir::ImplInfo {
+        hir::ImplInfo {
+            trait_id: imp.trait_id.map(|t| self.id(t)),
+            self_head: self.head(imp.self_head),
+            self_ty: self.ty(&imp.self_ty),
+            generics: imp.generics,
+            bounds: self.bounds(&imp.bounds),
+            methods: imp
+                .methods
+                .iter()
+                .map(|(n, d)| (n.clone(), self.id(*d)))
+                .collect(),
+            assoc_bindings: imp
+                .assoc_bindings
+                .iter()
+                .map(|(d, t)| (self.id(*d), self.ty(t)))
+                .collect(),
+        }
+    }
+
+    fn trait_def(self, t: &hir::TraitDef) -> hir::TraitDef {
+        hir::TraitDef {
+            def_id: self.id(t.def_id),
+            name: t.name.clone(),
+            supertraits: self.ids(&t.supertraits),
+            methods: t
+                .methods
+                .iter()
+                .map(|m| hir::TraitMethod {
+                    name: m.name.clone(),
+                    params: self.tys(&m.params),
+                    ret: self.ty(&m.ret),
+                    has_self: m.has_self,
+                    mut_self: m.mut_self,
+                    generics: m.generics,
+                    bounds: self.bounds(&m.bounds),
+                    default_def: m.default_def.map(|d| self.id(d)),
+                })
+                .collect(),
+            assoc_types: t
+                .assoc_types
+                .iter()
+                .map(|(n, d)| (n.clone(), self.id(*d)))
+                .collect(),
+        }
+    }
+
+    fn impl_self(self, s: &ImplSelf) -> ImplSelf {
+        ImplSelf {
+            ty: self.ty(&s.ty),
+            trait_id: s.trait_id.map(|t| self.id(t)),
+        }
+    }
+}
+
+/// Where in the passes std's part of a table is added: where the full
+/// check would have collected std's items, at the end of a pass's loop.
+#[derive(Clone, Copy)]
+pub(super) enum Splice {
+    Arities,
+    Supertraits,
+    Records,
+    Sums,
+    Traits,
+    TraitDefaults,
+    Impls,
+    Signatures,
+    Externs,
+}
+
+impl<'a> Checker<'a> {
+    /// A checker over `file` and `defs` with empty tables, reading std's
+    /// from `shared` when given.
+    pub(super) fn new(
+        file: &'a ast::File,
+        defs: &'a Definitions,
+        options: &CheckOptions<'a>,
+        shared: Option<(&'a SharedItems, Rebase)>,
+    ) -> Checker<'a> {
+        Checker {
+            file,
+            defs,
+            cur_module: ModuleId(0),
+            sigs: FxHashMap::default(),
+            method_locs: FxHashMap::default(),
+            selfless: FxHashSet::default(),
+            mut_self: FxHashSet::default(),
+            sums: Vec::new(),
+            records: Vec::new(),
+            supertraits: FxHashMap::default(),
+            traits: Vec::new(),
+            impls: Vec::new(),
+            impl_param_names: Vec::new(),
+            impl_self: None,
+            impl_selves: FxHashMap::default(),
+            extra_functions: Vec::new(),
+            next_closure_def: defs.defs().len() as u32,
+            type_arity: FxHashMap::default(),
+            externs: Vec::new(),
+            diagnostics: Vec::new(),
+            probe: options.probe,
+            probe_result: ProbeResult::default(),
+            probe_pending: None,
+            probe_locals_pending: None,
+            index: options.index.then(Index::default),
+            type_params: Vec::new(),
+            sources: options.sources,
+            bindings: FxHashMap::default(),
+            qualified_callee: None,
+            pats: PatState::default(),
+            quiet_splits: FxHashSet::default(),
+            impl_spans: Vec::new(),
+            impl_in_std: Vec::new(),
+            impl_method_spans: Vec::new(),
+            own_defs: defs.defs().len() as u32,
+            shared,
+        }
+    }
+
+    /// Passes 1 to 5.
+    fn collect_items(&mut self) {
+        // Before any collection pass: every later pass builds a generic scope, and
+        // one containing `Self` would give the name two meanings at once.
+        self.note_item_declarations();
+        self.reject_self_type_params();
+        self.collect_type_arities();
+        self.splice(Splice::Arities);
+        // Before `collect_records`: since Task 1 of the iterator-finishing plan,
+        // `collect_records` resolves each record's bounds and calls
+        // `expand_bounds`, which folds transitive supertraits in from
+        // `Checker::supertraits` — the very table `collect_supertraits` builds.
+        // Moved ahead of `collect_records` (and `collect_sums`, which never reads
+        // it but has no reason to run first either) so that table is populated
+        // before the first `expand_bounds` call, not just before
+        // `collect_traits`'s. `collect_supertraits` itself only reads trait
+        // declarations via `self.defs` (already fully resolved before `check`
+        // runs) and `decl.supertraits`, so it has no ordering dependency on
+        // `collect_type_arities`, `collect_records`, or `collect_sums`.
+        //
+        // It did change the *order* diagnostics are rendered in, and nothing sorts
+        // `self.diagnostics` before rendering, so pass order is emission order.
+        // Measured on one program carrying an unresolvable supertrait, a duplicate
+        // record generic and a sum-type bound: `E0001` -> `E0403` -> `E0900` now,
+        // `E0403` -> `E0900` -> `E0001` with this call back in its old slot (the
+        // move was re-run in both directions to confirm it, not inferred from the
+        // pass list). Cosmetic — every diagnostic is still reported, with its own
+        // span — and recorded rather than fixed: stabilizing order would mean
+        // sorting by span, which is a separate decision affecting every diagnostic
+        // in the compiler.
+        self.collect_supertraits();
+        self.splice(Splice::Supertraits);
+        self.collect_records();
+        self.splice(Splice::Records);
+        self.collect_sums();
+        self.splice(Splice::Sums);
+        self.collect_traits();
+        self.collect_impls();
+        self.collect_signatures();
+        self.splice(Splice::Signatures);
+        self.collect_externs();
+        self.splice(Splice::Externs);
+    }
+
+    /// The tables, taken out of the checker.
+    fn into_tables(self) -> ItemTables {
+        ItemTables {
+            sigs: self.sigs,
+            method_locs: self.method_locs,
+            selfless: self.selfless,
+            mut_self: self.mut_self,
+            sums: self.sums,
+            records: self.records,
+            supertraits: self.supertraits,
+            traits: self.traits,
+            impls: self.impls,
+            impl_param_names: self.impl_param_names,
+            impl_selves: self.impl_selves,
+            type_arity: self.type_arity,
+            externs: self.externs,
+            impl_spans: self.impl_spans,
+            impl_in_std: self.impl_in_std,
+            impl_method_spans: self.impl_method_spans,
+            diagnostics: self.diagnostics,
+            index: self.index,
+            cur_module: self.cur_module,
+            type_params: self.type_params,
+            impl_self: self.impl_self,
+        }
+    }
+
+    /// The checker as [`Checker::into_tables`] left it, for the bodies.
+    pub(super) fn restore(&mut self, tables: ItemTables) {
+        self.sigs = tables.sigs;
+        self.method_locs = tables.method_locs;
+        self.selfless = tables.selfless;
+        self.mut_self = tables.mut_self;
+        self.sums = tables.sums;
+        self.records = tables.records;
+        self.supertraits = tables.supertraits;
+        self.traits = tables.traits;
+        self.impls = tables.impls;
+        self.impl_param_names = tables.impl_param_names;
+        self.impl_selves = tables.impl_selves;
+        self.type_arity = tables.type_arity;
+        self.externs = tables.externs;
+        self.impl_spans = tables.impl_spans;
+        self.impl_in_std = tables.impl_in_std;
+        self.impl_method_spans = tables.impl_method_spans;
+        self.diagnostics = tables.diagnostics;
+        self.index = tables.index;
+        self.cur_module = tables.cur_module;
+        self.type_params = tables.type_params;
+        self.impl_self = tables.impl_self;
+    }
+
+    /// Whether the definition `id` is the program's own, which the passes
+    /// collect: every one, unless std's tables are shared.
+    pub(super) fn own(&self, id: DefId) -> bool {
+        id.0 < self.own_defs
+    }
+
+    /// Add std's part of the tables `at` fills, rebased, when they are
+    /// shared; nothing otherwise.
+    pub(super) fn splice(&mut self, at: Splice) {
+        let Some((shared, r)) = self.shared else {
+            return;
+        };
+        let std = &shared.tables;
+        match at {
+            Splice::Arities => {
+                for id in sorted(&std.type_arity) {
+                    self.type_arity.insert(r.id(id), std.type_arity[&id]);
+                }
+            }
+            Splice::Supertraits => {
+                for id in sorted(&std.supertraits) {
+                    self.supertraits
+                        .insert(r.id(id), r.ids(&std.supertraits[&id]));
+                }
+            }
+            Splice::Records => {
+                self.records.extend(std.records.iter().map(|rec| {
+                    hir::RecordType {
+                        def_id: r.id(rec.def_id),
+                        name: rec.name.clone(),
+                        generics: rec.generics,
+                        fields: rec
+                            .fields
+                            .iter()
+                            .map(|f| hir::RecordField {
+                                name: f.name.clone(),
+                                ty: r.ty(&f.ty),
+                            })
+                            .collect(),
+                    }
+                }));
+            }
+            Splice::Sums => {
+                self.sums.extend(std.sums.iter().map(|s| {
+                    hir::SumType {
+                        def_id: r.id(s.def_id),
+                        name: s.name.clone(),
+                        generics: s.generics,
+                        variants: s
+                            .variants
+                            .iter()
+                            .map(|v| hir::Variant {
+                                name: v.name.clone(),
+                                fields: r.tys(&v.fields),
+                            })
+                            .collect(),
+                    }
+                }));
+            }
+            Splice::Traits => {
+                self.traits
+                    .extend(std.traits.iter().map(|t| r.trait_def(t)));
+            }
+            Splice::TraitDefaults => self.splice_methods(MethodOwner::TraitDefault),
+            Splice::Impls => {
+                self.splice_methods(MethodOwner::Impl);
+                self.impls.extend(std.impls.iter().map(|i| r.imp(i)));
+                self.impl_spans.extend(std.impl_spans.iter().copied());
+                self.impl_in_std.extend(std.impl_in_std.iter().copied());
+                self.impl_method_spans
+                    .extend(std.impl_method_spans.iter().cloned());
+                self.impl_param_names
+                    .extend(std.impl_param_names.iter().cloned());
+                for item in sorted(&std.impl_selves) {
+                    let s = r.impl_self(&std.impl_selves[&item]);
+                    self.impl_selves.insert(item + r.item, s);
+                }
+            }
+            Splice::Signatures => {
+                for id in sorted(&std.sigs) {
+                    if matches!(
+                        self.defs.def(r.id(id)).kind,
+                        DefKind::Fn { .. } | DefKind::Const { .. }
+                    ) {
+                        self.sigs.insert(r.id(id), r.sig(&std.sigs[&id]));
+                    }
+                }
+            }
+            Splice::Externs => {
+                for id in sorted(&std.sigs) {
+                    if matches!(self.defs.def(r.id(id)).kind, DefKind::ExternFn { .. }) {
+                        self.sigs.insert(r.id(id), r.sig(&std.sigs[&id]));
+                    }
+                }
+                self.externs
+                    .extend(std.externs.iter().map(|e| hir::ExternFn {
+                        def_id: r.id(e.def_id),
+                        symbol: e.symbol.clone(),
+                        abi: e.abi,
+                        params: r.tys(&e.params),
+                        ret: r.ty(&e.ret),
+                        span: e.span,
+                    }));
+            }
+        }
+    }
+
+    /// std's methods owned by `owner`: their signatures, places and
+    /// receivers.
+    fn splice_methods(&mut self, owner: MethodOwner) {
+        let Some((shared, r)) = self.shared else {
+            return;
+        };
+        let std = &shared.tables;
+        for id in sorted(&std.method_locs) {
+            let loc = std.method_locs[&id];
+            if loc.owner != owner {
+                continue;
+            }
+            let at = r.id(id);
+            if let Some(sig) = std.sigs.get(&id) {
+                self.sigs.insert(at, r.sig(sig));
+            }
+            self.method_locs.insert(
+                at,
+                MethodLoc {
+                    item_index: loc.item_index + r.item,
+                    ..loc
+                },
+            );
+            if std.selfless.contains(&id) {
+                self.selfless.insert(at);
+            }
+            if std.mut_self.contains(&id) {
+                self.mut_self.insert(at);
+            }
+        }
+    }
+}
+
+/// A map's keys sorted, so that no map's order reaches a table.
+fn sorted<K: Ord + Copy, V>(map: &FxHashMap<K, V>) -> Vec<K> {
+    let mut keys: Vec<K> = map.keys().copied().collect();
+    keys.sort();
+    keys
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/nova-typeck/src/dump.rs b/crates/nova-typeck/src/dump.rs
new file mode 100644
index 00000000..d0e40e47
--- /dev/null
+++ b/crates/nova-typeck/src/dump.rs
@@ -0,0 +1,503 @@
+//! The HIR dump: what the type checker built for one program, as text, one
+//! entry per line, which the self-hosted type checker (`selfhost/`) prints
+//! too, so that the two can be compared byte for byte (spec
+//! `docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
+//! §3.3; ADR 0055 writes the grammar out, with an example of every form).
+//! It carries no stability promise: it changes whenever the type checker
+//! does, on both sides in one pull request. `nova check --dump-hir FILE…`
+//! prints a program's part after a `== FILE` header, and `nova check
+//! --dump-hir --std` std's part; `--target wasm` and `--test` as the
+//! resolution dump takes them (ADR 0054).
+//!
+//! - It follows the resolution dump's conventions (ADR 0053 §1): a line is
+//!   its indentation, two spaces per depth, `(` and its kind, its values
+//!   and its fields `key=value`; its children follow, one level deeper,
+//!   and its `)` closes it at the end of its last line. Names and messages
+//!   are quoted as the AST dump quotes strings; a place is a file's label
+//!   quoted, `:` and a span; diagnostics print as the resolution dump's.
+//! - **5.3a's part is the items** (passes 2 to 5): records, sums, traits,
+//!   impls, every signature by `DefId`, externs, then every diagnostic so
+//!   far, the earlier stages' and the item passes'. 5.3b adds every
+//!   function, after the externs and before the diagnostics.
+//! - **Two parts.** std's tables are the same in every program for a
+//!   target and mode, so they print once, by [`dump_std`]; a program's
+//!   part, by [`dump_program`], prints only the program's own entries, and
+//!   `(StdStart module= def= item=)` as the resolution dump does.
+//! - **Ids** are the resolution dump's: `u<n>` for the program's own
+//!   definition, `s<n>` for std's, its `DefId` less the first std
+//!   definition's. An impl, which has no `DefId`, is numbered from 0 within
+//!   its part, in the table's order.
+//! - **Types** print by structure, with ids, not names: `Int`, `()`,
+//!   `[T0]`, `(Int, String)`, `(Int,)`, `fn(Int) -> Bool`, `Future<Int>`,
+//!   `T0` for a type parameter, a record or a sum as its id and its
+//!   arguments (`s3<Int>`), a projection as its type, `::` and the
+//!   associated type's id (`T0::s20`), `!`, `{error}`, and an inference
+//!   variable as `?N`, as S0's messages print it.
+//! - Tables print in the order the type checker collected them; a map's
+//!   entries only sorted, by `DefId`.
+
+use std::fmt::Write;
+
+use nova_ast::dump::quote;
+use nova_diagnostics::Diagnostic;
+use nova_hir::{Ty, TyHead};
+use nova_resolver::dump::{std_start, write_diagnostics, write_place, Labels};
+use nova_resolver::{CompileTarget, DefId, DefKind, Definitions, MethodOwner, ProgramResolution};
+
+use crate::ItemTables;
+
+/// Every form the dump prints, as [`nova_resolver::dump::forms_on`] reads
+/// a line, beyond those whose values are numbers or ids: the harness
+/// checks that std, the corpus and the cases print each of them and print
+/// nothing unlisted (`crates/nova-cli/tests/selfhost.rs`).
+pub const KINDS: &[&str] = &[
+    "(Hir",
+    "(Hir checked=true",
+    "(Hir checked=false",
+    "(HirStd",
+    "target=native",
+    "target=wasm",
+    "mode=test",
+    "(StdStart",
+    "(Record",
+    "(Field",
+    "(Sum",
+    "(Variant",
+    "(Trait",
+    "(Supertrait",
+    "(AssocType",
+    "(Method",
+    "(Method has_self=true",
+    "(Method has_self=false",
+    "(Method mut_self=true",
+    "(Method mut_self=false",
+    "default=None",
+    "(Bound",
+    "(Param",
+    "(Ret",
+    "(Impl",
+    "trait=None",
+    "head=Int",
+    "head=Float",
+    "head=Bool",
+    "head=Char",
+    "head=String",
+    "head=Bytes",
+    "head=Array",
+    "(SelfType",
+    "(AssocBinding",
+    "(Sig",
+    "(Sig selfless=true",
+    "(Sig selfless=false",
+    "(Sig mut_self=true",
+    "(Sig mut_self=false",
+    "(Extern",
+    "abi=C",
+    "abi=Js",
+    "(Diagnostics",
+    "(Error",
+    "(Warning",
+    "(Primary",
+    "(Secondary",
+    "(Note",
+    "(Fix",
+    "(Edit",
+];
+
+/// The forms no input prints: the two severities no stage gives.
+pub const LATER_KINDS: &[&str] = &["(Help", "(NoteDiagnostic"];
+
+/// A program's part (module docs) for `target`, under test mode when
+/// `test`: when the program reached the type checker, its resolution and
+/// item tables, and every diagnostic so far, the package graph's, the
+/// loader's, the resolver's and the item passes', in order. A program that
+/// stopped before the type checker prints `checked=false` and the
+/// diagnostics.
+pub fn dump_program(
+    target: CompileTarget,
+    test: bool,
+    labels: &Labels,
+    checked: Option<(&ProgramResolution, &ItemTables)>,
+    diagnostics: &[Diagnostic],
+) -> String {
+    let mut d = Dump::new(labels, u32::MAX);
+    d.open(0, "Hir");
+    d.field("target", target.name());
+    d.mode(test);
+    d.bool_field("checked", checked.is_some());
+    if let Some((resolution, tables)) = checked {
+        let defs = &resolution.definitions;
+        let (module, def, item) = std_start(defs);
+        d.std_def = def;
+        d.open(1, "StdStart");
+        d.field("module", &module.to_string());
+        d.field("def", &def.to_string());
+        d.field("item", &item.to_string());
+        d.close();
+        d.tables(defs, tables, false);
+    }
+    d.close();
+    d.out.push('\n');
+    write_diagnostics(&mut d.out, labels, diagnostics);
+    d.out
+}
+
+/// std's part (module docs) for `target`, under test mode when `test`, from
+/// the check of a program of no module of its own: its resolution, its
+/// item tables, and every diagnostic, the resolver's and the item passes'.
+pub fn dump_std(
+    target: CompileTarget,
+    test: bool,
+    labels: &Labels,
+    resolution: &ProgramResolution,
+    tables: &ItemTables,
+    diagnostics: &[Diagnostic],
+) -> String {
+    let mut out = std_part_of(target, test, labels, resolution, tables);
+    write_diagnostics(&mut out, labels, diagnostics);
+    out
+}
+
+/// std's part of any program's check, without its diagnostics: the same
+/// text as [`dump_std`]'s for a program of no module of its own, in every
+/// program for one target and mode, which a test checks.
+pub fn std_part_of(
+    target: CompileTarget,
+    test: bool,
+    labels: &Labels,
+    resolution: &ProgramResolution,
+    tables: &ItemTables,
+) -> String {
+    let defs = &resolution.definitions;
+    let (_, std_def, _) = std_start(defs);
+    let mut d = Dump::new(labels, std_def);
+    d.open(0, "HirStd");
+    d.field("target", target.name());
+    d.mode(test);
+    let std = |id: DefId| id.0 >= std_def;
+    let count = |n: usize| n.to_string();
+    d.field(
+        "records",
+        &count(tables.records.iter().filter(|r| std(r.def_id)).count()),
+    );
+    d.field(
+        "sums",
+        &count(tables.sums.iter().filter(|s| std(s.def_id)).count()),
+    );
+    d.field(
+        "traits",
+        &count(tables.traits.iter().filter(|t| std(t.def_id)).count()),
+    );
+    d.field(
+        "impls",
+        &count(tables.impl_in_std.iter().filter(|s| **s).count()),
+    );
+    d.field(
+        "sigs",
+        &count(tables.sigs.keys().filter(|id| std(**id)).count()),
+    );
+    d.field(
+        "externs",
+        &count(tables.externs.iter().filter(|e| std(e.def_id)).count()),
+    );
+    d.tables(defs, tables, true);
+    d.close();
+    d.out.push('\n');
+    d.out
+}
+
+/// The text being written, the files' labels, and the first std
+/// definition's id, from which ids print as `s<n>`.
+struct Dump<'a> {
+    out: String,
+    labels: &'a Labels,
+    std_def: u32,
+}
+
+impl<'a> Dump<'a> {
+    fn new(labels: &'a Labels, std_def: u32) -> Self {
+        Dump {
+            out: String::new(),
+            labels,
+            std_def,
+        }
+    }
+
+    // --- Lines and values ---
+
+    /// A line, up to its kind.
+    fn open(&mut self, depth: usize, kind: &str) {
+        if !self.out.is_empty() && !self.out.ends_with('\n') {
+            self.out.push('\n');
+        }
+        for _ in 0..depth {
+            self.out.push_str("  ");
+        }
+        self.out.push('(');
+        self.out.push_str(kind);
+    }
+
+    fn close(&mut self) {
+        self.out.push(')');
+    }
+
+    /// ` key=value`.
+    fn field(&mut self, key: &str, value: &str) {
+        let _ = write!(self.out, " {key}={value}");
+    }
+
+    /// ` mode=test` under test mode, and nothing under `check`'s.
+    fn mode(&mut self, test: bool) {
+        if test {
+            self.field("mode", "test");
+        }
+    }
+
+    fn bool_field(&mut self, key: &str, value: bool) {
+        let _ = write!(self.out, " {key}={value}");
+    }
+
+    /// ` ` and `text` quoted.
+    fn string(&mut self, text: &str) {
+        self.out.push(' ');
+        quote(&mut self.out, text);
+    }
+
+    /// ` ` and a value.
+    fn value(&mut self, text: &str) {
+        self.out.push(' ');
+        self.out.push_str(text);
+    }
+
+    /// A definition's id: `u<n>` for the program's own, `s<n>` for std's.
+    fn id(&self, id: DefId) -> String {
+        if id.0 >= self.std_def {
+            format!("s{}", id.0 - self.std_def)
+        } else {
+            format!("u{}", id.0)
+        }
+    }
+
+    fn opt_id(&self, id: Option<DefId>) -> String {
+        id.map_or_else(|| "None".to_string(), |id| self.id(id))
+    }
+
+    /// A type, by structure (module docs).
+    fn ty(&self, ty: &Ty) -> String {
+        let list = |tys: &[Ty]| tys.iter().map(|t| self.ty(t)).collect::<Vec<_>>();
+        match ty {
+            Ty::Int => "Int".to_string(),
+            Ty::Float => "Float".to_string(),
+            Ty::Bool => "Bool".to_string(),
+            Ty::Char => "Char".to_string(),
+            Ty::String => "String".to_string(),
+            Ty::Bytes => "Bytes".to_string(),
+            Ty::Unit => "()".to_string(),
+            Ty::Fn { params, ret } => {
+                format!("fn({}) -> {}", list(params).join(", "), self.ty(ret))
+            }
+            Ty::Future(out) => format!("Future<{}>", self.ty(out)),
+            Ty::Sum { def_id, args } | Ty::Record { def_id, args } => {
+                if args.is_empty() {
+                    self.id(*def_id)
+                } else {
+                    format!("{}<{}>", self.id(*def_id), list(args).join(", "))
+                }
+            }
+            Ty::Array(elem) => format!("[{}]", self.ty(elem)),
+            Ty::Tuple(elems) => crate::tuple_text(list(elems)),
+            Ty::Param(i) => format!("T{i}"),
+            Ty::Assoc { on, assoc } => format!("{}::{}", self.ty(on), self.id(*assoc)),
+            Ty::Var(v) => format!("?{v}"),
+            Ty::Never => "!".to_string(),
+            Ty::Error => "{error}".to_string(),
+        }
+    }
+
+    fn head(&self, head: TyHead) -> String {
+        match head {
+            TyHead::Int => "Int".to_string(),
+            TyHead::Float => "Float".to_string(),
+            TyHead::Bool => "Bool".to_string(),
+            TyHead::Char => "Char".to_string(),
+            TyHead::String => "String".to_string(),
+            TyHead::Bytes => "Bytes".to_string(),
+            TyHead::Array => "Array".to_string(),
+            TyHead::Sum(id) | TyHead::Record(id) => self.id(id),
+        }
+    }
+
+    /// `(Bound k id…)` for each generic parameter's bounds, at `depth`.
+    fn bounds(&mut self, depth: usize, bounds: &[Vec<DefId>]) {
+        for (k, slot) in bounds.iter().enumerate() {
+            self.open(depth, "Bound");
+            let _ = write!(self.out, " {k}");
+            for &id in slot {
+                let id = self.id(id);
+                self.value(&id);
+            }
+            self.close();
+        }
+    }
+
+    /// `(Param ty)` for each parameter, then `(Ret ty)`, at `depth`.
+    fn signature(&mut self, depth: usize, params: &[Ty], ret: &Ty) {
+        for p in params {
+            self.open(depth, "Param");
+            let t = self.ty(p);
+            self.value(&t);
+            self.close();
+        }
+        self.open(depth, "Ret");
+        let t = self.ty(ret);
+        self.value(&t);
+        self.close();
+    }
+
+    // --- The tables ---
+
+    /// The entries of std's part (`std`) or of the program's.
+    fn tables(&mut self, defs: &Definitions, t: &ItemTables, std: bool) {
+        let std_def = self.std_def;
+        let ours = move |id: DefId| (id.0 >= std_def) == std;
+        for r in t.records.iter().filter(|r| ours(r.def_id)) {
+            self.open(1, "Record");
+            let id = self.id(r.def_id);
+            self.value(&id);
+            self.string(&r.name);
+            self.field("generics", &r.generics.to_string());
+            for f in &r.fields {
+                self.open(2, "Field");
+                self.string(&f.name);
+                let ty = self.ty(&f.ty);
+                self.value(&ty);
+                self.close();
+            }
+            self.close();
+        }
+        for s in t.sums.iter().filter(|s| ours(s.def_id)) {
+            self.open(1, "Sum");
+            let id = self.id(s.def_id);
+            self.value(&id);
+            self.string(&s.name);
+            self.field("generics", &s.generics.to_string());
+            for v in &s.variants {
+                self.open(2, "Variant");
+                self.string(&v.name);
+                for f in &v.fields {
+                    let ty = self.ty(f);
+                    self.value(&ty);
+                }
+                self.close();
+            }
+            self.close();
+        }
+        for tr in t.traits.iter().filter(|tr| ours(tr.def_id)) {
+            self.open(1, "Trait");
+            let id = self.id(tr.def_id);
+            self.value(&id);
+            self.string(&tr.name);
+            for &s in &tr.supertraits {
+                self.open(2, "Supertrait");
+                let id = self.id(s);
+                self.value(&id);
+                self.close();
+            }
+            for (name, assoc) in &tr.assoc_types {
+                self.open(2, "AssocType");
+                self.string(name);
+                let id = self.id(*assoc);
+                self.value(&id);
+                self.close();
+            }
+            for m in &tr.methods {
+                self.open(2, "Method");
+                self.string(&m.name);
+                self.bool_field("has_self", m.has_self);
+                self.bool_field("mut_self", m.mut_self);
+                self.field("generics", &m.generics.to_string());
+                let default = self.opt_id(m.default_def);
+                self.field("default", &default);
+                self.bounds(3, &m.bounds);
+                self.signature(3, &m.params, &m.ret);
+                self.close();
+            }
+            self.close();
+        }
+        let mut index = 0;
+        for (k, imp) in t.impls.iter().enumerate() {
+            if t.impl_in_std.get(k).copied().unwrap_or(false) != std {
+                continue;
+            }
+            self.open(1, "Impl");
+            let _ = write!(self.out, " {index}");
+            index += 1;
+            let tr = self.opt_id(imp.trait_id);
+            self.field("trait", &tr);
+            let head = self.head(imp.self_head);
+            self.field("head", &head);
+            self.field("generics", &imp.generics.to_string());
+            if let Some(&span) = t.impl_spans.get(k) {
+                write_place(&mut self.out, self.labels, span);
+            }
+            self.open(2, "SelfType");
+            let ty = self.ty(&imp.self_ty);
+            self.value(&ty);
+            self.close();
+            self.bounds(2, &imp.bounds);
+            for (name, def) in &imp.methods {
+                self.open(2, "Method");
+                self.string(name);
+                let id = self.id(*def);
+                self.value(&id);
+                self.close();
+            }
+            for (assoc, ty) in &imp.assoc_bindings {
+                self.open(2, "AssocBinding");
+                let id = self.id(*assoc);
+                self.value(&id);
+                let ty = self.ty(ty);
+                self.value(&ty);
+                self.close();
+            }
+            self.close();
+        }
+        let mut sigs: Vec<DefId> = t.sigs.keys().copied().filter(|id| ours(*id)).collect();
+        sigs.sort_by_key(|id| id.0);
+        for id in sigs {
+            let sig = &t.sigs[&id];
+            self.open(1, "Sig");
+            let shown = self.id(id);
+            self.value(&shown);
+            self.field("generics", &sig.generics.to_string());
+            let impl_method = matches!(
+                defs.def(id).kind,
+                DefKind::Method {
+                    owner: MethodOwner::Impl,
+                    ..
+                }
+            );
+            if impl_method {
+                self.bool_field("selfless", t.selfless.contains(&id));
+                self.bool_field("mut_self", t.mut_self.contains(&id));
+            }
+            self.bounds(2, &sig.bounds);
+            self.signature(2, &sig.params, &sig.ret);
+            self.close();
+        }
+        for e in t.externs.iter().filter(|e| ours(e.def_id)) {
+            self.open(1, "Extern");
+            let id = self.id(e.def_id);
+            self.value(&id);
+            self.string(&e.symbol);
+            let abi = match e.abi {
+                nova_hir::ExternAbi::C => "C",
+                nova_hir::ExternAbi::Js => "Js",
+            };
+            self.field("abi", abi);
+            write_place(&mut self.out, self.labels, e.span);
+            self.signature(2, &e.params, &e.ret);
+            self.close();
+        }
+    }
+}
diff --git a/crates/nova-typeck/src/lib.rs b/crates/nova-typeck/src/lib.rs
index 989cd5db..3b92adbc 100644
--- a/crates/nova-typeck/src/lib.rs
+++ b/crates/nova-typeck/src/lib.rs
@@ -14,10 +14,11 @@
 //! rather than panicking.
 
 mod check;
+pub mod dump;
 mod infer;
 mod usefulness;
 
-pub use check::{check, check_with};
+pub use check::{check, check_items, check_items_shared, check_with, ItemTables, SharedItems};
 
 use nova_hir::Ty;
 use nova_resolver::Definitions;
````

### Task 2: The harness's HIR dump and the type fuzzer, S0's side

Spec §3.6; decisions 6, 7 and 16.

**Files:**
- Modify: `crates/nova-cli/tests/selfhost.rs`,
  `crates/nova-cli/tests/selfhost/fuzz.rs`

- [ ] **Step 1: The tests.** `patch_block t2-tests`: `selfhost.rs`'s 5.3a
  section, through S0 alone (`N_HIR` false; `agree_on_hir_dump` runs S0
  and checks its exit code until N exists): `HIR_S0`, `agree_in_parts`
  (the resolution dump's process split, for any dump), the case
  directories by mode, `ITEM_CODES`; the tests
  `the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps`,
  `the_hir_dump_has_a_header_per_root_and_std_its_own_part`,
  `every_run_of_a_hir_dump_settles_e0004s_paths_on_windows`,
  `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`,
  `the_hir_dumps_agree_on_the_cases_with_windows_line_endings` and
  `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`;
  `settled_stdout` settles the HIR dump.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. Then `SH`. Expected: `selfhost.rs` does not compile, 2
  errors: `E0425` (`fuzz::type_mutate`, `fuzz::TYPE_ITEMS`), and `error:
  could not compile nova-cli (test "selfhost")`.
- [ ] **Step 3: The code.** `patch_block t2-code`: `fuzz.rs`'s
  `TYPE_REPLACEMENTS`, `TYPE_ITEMS` (24 items, each aimed at a code),
  `type_sites` and `type_mutate`: one to three edits of a parameter's,
  return's or field's type, a generic list, a trait impl's method (dropped,
  added, retyped), an impl duplicated, or an item appended; an edit that
  does not parse is not kept.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`. Then
  `S` (255 s). Expected: `2880 1 10`, the IPv6 test the one failure;
  `selfhost.rs`'s line among them, `50 passed`: the base's 44 and the 6
  new tests, each on S0's dump alone.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `cli tests: the type fuzzer and the HIR dump,
      S0's side`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 5066139c..64f53dbd 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -525,11 +525,17 @@ fn run(program: &Path, args: &[&str], dir: &Path, files: &[String]) -> Result<Ru
 }
 
 /// `stdout` of a run with `args` as S0's and N's are compared: a
-/// resolution dump's, S0's `check --dump-resolution` or N's `resolve`,
+/// resolution dump's, S0's `check --dump-resolution` or N's `resolve`, or
+/// a HIR dump's, S0's `check --dump-hir` or N's `hir` (5.3a),
 /// with `\` read as `/` in the paths of E0004's and E0093's messages on
 /// `windows` ([`settle_paths`], ADR 0054 §7); any other as it is.
 fn settled_stdout(args: &[&str], stdout: Vec<u8>, windows: bool) -> Vec<u8> {
-    if args.starts_with(RESOLUTION_S0) || args.first() == Some(&"resolve") {
+    // The HIR dump prints the loader's diagnostics too (5.3a, spec 5.3
+    // §3.6): S0's `check --dump-hir` and N's `hir` are settled alike.
+    if args.starts_with(RESOLUTION_S0)
+        || args.starts_with(HIR_S0)
+        || matches!(args.first(), Some(&"resolve" | &"hir"))
+    {
         settle_paths(&stdout, windows)
     } else {
         stdout
@@ -2012,6 +2018,10 @@ fn nova_run_runs_the_program_on_the_main_thread() {
 /// program's root.
 const RESOLUTION_S0: &[&str] = &["check", "--dump-resolution"];
 
+/// S0's HIR dump (5.3a, ADR 0055): the hidden `check --dump-hir`, before
+/// the mode's flags and the roots.
+const HIR_S0: &[&str] = &["check", "--dump-hir"];
+
 /// A package's `nova.toml` with what the package graph requires and
 /// nothing else: the tests' packages (5.2b, ADR 0054).
 const MANIFEST: &str = "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";
@@ -2298,15 +2308,30 @@ fn agree_on_resolution_in_parts(
     files: &[String],
     code: i32,
     parts: usize,
+) -> String {
+    agree_in_parts(files, parts, |part| {
+        agree_on_resolution_in(mode, dir, part, code)
+    })
+}
+
+/// `agree` on `files` in `parts` processes a side, side by side, root `i`
+/// in part `i % parts` ([`agree_on_resolution_in_parts`]): each root's
+/// part of the outputs in the roots' order, each checked to be its root's,
+/// and none left over. The HIR dump's sets go through it too (5.3a).
+fn agree_in_parts(
+    files: &[String],
+    parts: usize,
+    agree: impl Fn(&[String]) -> String + Sync,
 ) -> String {
     let parts = parts.clamp(1, files.len().max(1));
     let dealt: Vec<Vec<String>> = (0..parts)
         .map(|k| files.iter().skip(k).step_by(parts).cloned().collect())
         .collect();
+    let agree = &agree;
     let outs: Vec<String> = std::thread::scope(|scope| {
         let runs: Vec<_> = dealt
             .iter()
-            .map(|part| scope.spawn(move || agree_on_resolution_in(mode, dir, part, code)))
+            .map(|part| scope.spawn(move || agree(part)))
             .collect();
         joined(runs)
     });
@@ -3492,3 +3517,613 @@ fn the_name_fuzzer_agrees_and_gives_every_resolver_code() {
         assert!(n >= 2, "{code} occurs {n} times, under 2, in {all:?}");
     }
 }
+
+// === 5.3a: the HIR dump (spec 5.3 §3.6, ADR 0055) ===
+//
+// Part A builds S0's side: the hidden `check --dump-hir`, its cases
+// (`cases/check/`, `cases/check-wasm/`, `cases/check-test/`) and the type
+// fuzzer's item mutations, each run through [`run`]. Until N's `selfhost
+// hir` exists ([`N_HIR`]), every set runs through S0 alone and its
+// assertions hold on S0's dump; part B ports N's side, sets `N_HIR`, and
+// every set below is then compared byte for byte, exit codes included,
+// through [`compare_batch`].
+
+/// Whether N has `selfhost hir --std DIR [--target wasm] [--test] FILE…`
+/// (or `--std-part`), the HIR dump's other side (spec 5.3 §3.3): part B of
+/// 5.3a sets it.
+const N_HIR: bool = false;
+
+/// The item codes of 5.3a (spec 5.3 §3.2): each occurs among the HIR
+/// dumps of the cases, the corpus and the fuzzer's inputs. E0092 is the
+/// WASM target's.
+const ITEM_CODES: [&str; 16] = [
+    "E0001", "E0010", "E0012", "E0015", "E0070", "E0071", "E0072", "E0073", "E0074", "E0076",
+    "E0077", "E0078", "E0092", "E0105", "E0403", "E0900",
+];
+
+/// The type checker's cases under `mode` (5.3a): `cases/check/` under
+/// `check`, `cases/check-wasm/` for the WASM target and `cases/check-test/`
+/// under test mode, laid out as the resolver's ([`case_roots`]).
+fn hir_cases_name(mode: Mode) -> &'static str {
+    match mode {
+        Mode::Check => "check",
+        Mode::Wasm => "check-wasm",
+        Mode::Test => "check-test",
+    }
+}
+
+fn hir_cases_dir(mode: Mode) -> PathBuf {
+    cases_dir().join(hir_cases_name(mode))
+}
+
+/// The HIR dump through both sides under `mode`: S0's `check --dump-hir`
+/// and N's `hir --std DIR`, each with the mode's flags and followed by the
+/// roots.
+fn hir_in(mode: Mode) -> &'static Dump {
+    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
+    DUMPS[mode as usize].get_or_init(|| hir_dump_with(mode, &[]))
+}
+
+/// std's part of the HIR dump under `mode`: S0's `check --dump-hir --std`,
+/// N's `hir --std DIR --std-part`.
+fn std_hir(mode: Mode) -> &'static Dump {
+    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
+    DUMPS[mode as usize].get_or_init(|| hir_dump_with(mode, &["--std"]))
+}
+
+/// [`dump_with`] for the HIR dump.
+fn hir_dump_with(mode: Mode, extra: &[&str]) -> Dump {
+    let s0: Vec<&str> = HIR_S0
+        .iter()
+        .chain(mode.flags())
+        .chain(extra)
+        .copied()
+        .collect();
+    let std_dir = std_dir();
+    let mut n = vec!["hir", "--std", std_dir.as_str()];
+    n.extend(mode.flags());
+    n.extend(
+        extra
+            .iter()
+            .map(|a| if *a == "--std" { "--std-part" } else { a }),
+    );
+    Dump {
+        s0: leak_args(&s0),
+        n: leak_args(&n),
+    }
+}
+
+/// `dump` on `files`, relative to `dir`, in one process a side, each side
+/// exiting with `code`: compared through [`compare_batch`] once N has the
+/// HIR dump, and through S0 alone ([`run`]) before. Returns S0's output.
+fn agree_on_hir_dump(dump: &Dump, dir: &Path, files: &[String], code: i32) -> String {
+    if N_HIR {
+        return compare_batch(dump, dir, files, code, &nova(), selfhost())
+            .unwrap_or_else(|report| panic!("{report}"));
+    }
+    let s0 = run(&nova(), dump.s0, dir, files).unwrap_or_else(|e| panic!("{e}"));
+    assert_eq!(
+        s0.code,
+        Some(code),
+        "{}: {}",
+        batch_name(&nova(), dump.s0, files),
+        s0.stderr
+    );
+    String::from_utf8(s0.stdout).expect("the HIR dump is UTF-8")
+}
+
+/// [`agree_on_hir_dump`] the HIR dump under `mode` on `files`, each a
+/// program's root.
+fn agree_on_hir_in(mode: Mode, dir: &Path, files: &[String], code: i32) -> String {
+    if files.is_empty() {
+        return String::new();
+    }
+    agree_on_hir_dump(hir_in(mode), dir, files, code)
+}
+
+/// [`agree_on_resolve_cases`] for the HIR dump: the loose programs and
+/// the packages' roots of `cases` in one process a side, then each program
+/// apart from its own directory. Returns S0's output and how many programs
+/// it holds.
+fn agree_on_hir_cases(mode: Mode, cases: &Path) -> (String, usize) {
+    let roots = case_roots(cases);
+    let count = roots.loose.len() + roots.apart.len() + roots.packages.len();
+    let from_cases: Vec<String> = roots.loose.into_iter().chain(roots.packages).collect();
+    let mut all = agree_on_hir_in(mode, cases, &from_cases, 0);
+    for dir in &roots.apart {
+        all.push_str(&agree_on_hir_in(mode, dir, &["main.nova".to_string()], 0));
+    }
+    (all, count)
+}
+
+/// The forms `dumps` print, as the resolution dump's [`forms_on`] reads
+/// them ([`nova_driver::resolution_dump::forms_on`]).
+fn hir_forms(dumps: &[&str]) -> std::collections::HashSet<String> {
+    let mut found = std::collections::HashSet::new();
+    for dump in dumps {
+        for line in dump.lines() {
+            found.extend(nova_driver::resolution_dump::forms_on(line));
+        }
+    }
+    found
+}
+
+/// `nova check`'s help names no `--dump-hir`, every `nova check` command
+/// line whose first argument after `check` is neither hidden flag is
+/// parsed as the base parses it (the resolution dump's test holds the
+/// rest), and the HIR dump's own parser refuses what the resolution dump's
+/// refuses, under its own usage line (5.3a, ADR 0055).
+#[test]
+fn the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps() {
+    let help = Command::new(nova())
+        .args(["check", "--help"])
+        .output()
+        .expect("run nova check --help");
+    let help = String::from_utf8_lossy(&help.stdout).into_owned();
+    assert!(help.contains("--target"), "{help}");
+    assert!(!help.contains("dump-hir"), "{help}");
+    let out = Command::new(nova())
+        .args(["check", "--help", "--dump-hir"])
+        .output()
+        .expect("run nova check --help --dump-hir");
+    assert_eq!(out.status.code(), Some(0));
+    assert_eq!(String::from_utf8_lossy(&out.stdout), help);
+    let (opts, file, target) = ("[OPTIONS] [FILE]", "<FILE>", "--target <TARGET> [FILE]");
+    for (args, named, usage) in [
+        (&["check", "--dump-hi", "a.nova"][..], "--dump-hi", opts),
+        (&["check", "--dump-hirr", "a.nova"], "--dump-hirr", opts),
+        (&["check", "--dump-hir=1", "a.nova"], "--dump-hir", opts),
+        (&["check", "--dump-hir=", "a.nova"], "--dump-hir", opts),
+        (&["check", "a.nova", "--dump-hir"], "--dump-hir", file),
+        (
+            &["check", "--target", "native", "--dump-hir", "a.nova"],
+            "--dump-hir",
+            target,
+        ),
+        (&["check", "--test", "--dump-hir", "a.nova"], "--test", opts),
+    ] {
+        let out = Command::new(nova())
+            .args(args)
+            .output()
+            .expect("run nova check");
+        let stderr = String::from_utf8_lossy(&out.stderr);
+        let tip = format!("  tip: to pass '{named}' as a value, use '-- {named}'\n\n");
+        assert_eq!(out.status.code(), Some(2), "{args:?}: {stderr}");
+        assert_eq!(
+            stderr,
+            format!(
+                "error: unexpected argument '{named}' found\n\n{tip}\
+                 Usage: nova{EXE_SUFFIX} check {usage}\n\n\
+                 For more information, try '--help'.\n"
+            ),
+            "{args:?}"
+        );
+        assert!(out.stdout.is_empty(), "{args:?}");
+    }
+    for args in [
+        &["--stdd"][..],
+        &["--std", "--std"],
+        &["--target", "bogus", "a.nova"],
+        &["--target=", "a.nova"],
+        &["--help"],
+        &["-h"],
+        &["--dump-hir", "a.nova"],
+        &["--dump-resolution", "a.nova"],
+        &["--test", "--test", "a.nova"],
+        &["--test=1", "a.nova"],
+        &["--tests", "a.nova"],
+        &["--test", "--target", "wasm", "a.nova"],
+        &["--target=wasm", "--test", "--std"],
+        &["--std", "a.nova"],
+        &["--test", "--std", "a.nova"],
+        &["--test"],
+        &[],
+    ] {
+        let out = Command::new(nova())
+            .args(HIR_S0)
+            .args(args)
+            .output()
+            .expect("run nova check --dump-hir");
+        let stderr = String::from_utf8_lossy(&out.stderr);
+        assert_eq!(out.status.code(), Some(2), "{args:?}: {stderr}");
+        assert!(stderr.starts_with("error: "), "{args:?}: {stderr}");
+        assert_eq!(stderr.matches("error:").count(), 1, "{args:?}: {stderr}");
+        assert_eq!(
+            stderr.matches("Usage: ").count(),
+            stderr.matches("Usage: nova check --dump-hir ").count(),
+            "{args:?}: {stderr}"
+        );
+        // The form's own refusals, which are not clap's, name its flag.
+        if !stderr.contains("Usage: ") && !stderr.contains("[possible values: ") {
+            assert!(
+                stderr.starts_with("error: --dump-hir "),
+                "{args:?}: {stderr}"
+            );
+        }
+        assert!(out.stdout.is_empty(), "{args:?}");
+    }
+}
+
+/// Each root's part follows a header with its path as given; a root that
+/// cannot be read is `IOERR` and its kind, the rest are dumped, and the
+/// exit code is 2; a program that stops before the type checker prints
+/// `checked=false` and the diagnostics so far; a package's root is a module
+/// of its package; `--std` prints std's part for the target and mode, the
+/// same as std's part of any program, and a program's part holds only its
+/// own entries (5.3a).
+#[test]
+fn the_hir_dump_has_a_header_per_root_and_std_its_own_part() {
+    let dir = fresh_dir("hir-headers");
+    std::fs::write(dir.join("a.nova"), "record P { x: Int }\nfn main() {}\n")
+        .expect("write a.nova");
+    std::fs::write(dir.join("b.nova"), "fn g( {").expect("write b.nova");
+    std::fs::write(dir.join("c.nova"), "fn f(x: Nope) {}\nfn f() {}\n").expect("write c.nova");
+    std::fs::create_dir_all(dir.join("pkg").join("src")).expect("create pkg/src");
+    std::fs::write(dir.join("pkg").join("nova.toml"), MANIFEST).expect("write nova.toml");
+    std::fs::write(
+        dir.join("pkg").join("src").join("main.nova"),
+        "import shapes\nfn main() {}\n",
+    )
+    .expect("write main.nova");
+    std::fs::write(
+        dir.join("pkg").join("src").join("shapes.nova"),
+        "pub type Shape =\n  | Dot\n",
+    )
+    .expect("write shapes.nova");
+    let files: Vec<String> = [
+        "a.nova",
+        "missing.nova",
+        "b.nova",
+        "c.nova",
+        "pkg/src/main.nova",
+    ]
+    .iter()
+    .map(|f| f.to_string())
+    .collect();
+    let out = agree_on_hir_in(Mode::Check, &dir, &files, 2);
+    let headers: Vec<&str> = out.lines().filter(|l| l.starts_with("== ")).collect();
+    assert_eq!(
+        headers,
+        files.iter().map(|f| format!("== {f}")).collect::<Vec<_>>()
+    );
+    assert!(
+        out.contains("== missing.nova\nIOERR NotFound\n== b.nova\n"),
+        "{out}"
+    );
+    assert!(
+        out.contains("== a.nova\n(Hir target=native checked=true\n  (StdStart module=1 def=2 item=2)\n  (Record u0 \"P\" generics=0\n    (Field \"x\" Int))\n  (Sig u1 generics=0\n    (Ret ())))\n(Diagnostics)\n"),
+        "{out}"
+    );
+    assert!(
+        out.contains("== b.nova\n(Hir target=native checked=false)\n(Diagnostics\n  (Error \"P"),
+        "{out}"
+    );
+    // E0002 is the resolver's: the type checker is not reached.
+    assert!(
+        out.contains(
+            "== c.nova\n(Hir target=native checked=false)\n(Diagnostics\n  (Error \"E0002\""
+        ),
+        "{out}"
+    );
+    assert!(
+        out.contains(
+            "== pkg/src/main.nova\n(Hir target=native checked=true\n  (StdStart module=2 "
+        ),
+        "{out}"
+    );
+    assert!(
+        out.contains("  (Sum u1 \"Shape\" generics=0\n    (Variant \"Dot\"))"),
+        "{out}"
+    );
+    for (mode, head) in [
+        (Mode::Check, "(HirStd target=native records="),
+        (Mode::Wasm, "(HirStd target=wasm records="),
+        (Mode::Test, "(HirStd target=native mode=test records="),
+    ] {
+        let std = agree_on_hir_dump(std_hir(mode), &dir, &[], 0);
+        assert!(std.starts_with(head), "{}", &std[..80]);
+        assert!(std.ends_with(")\n(Diagnostics)\n"), "{mode:?}");
+        assert!(
+            !std.contains(" u"),
+            "{mode:?}: a program's id in std's part"
+        );
+    }
+}
+
+/// The HIR dump's settling: S0's `check --dump-hir` and N's `hir` read `\`
+/// as `/` in E0004's and E0093's paths on Windows, as the resolution
+/// dump's are (5.3a, spec 5.3 §3.6).
+#[test]
+fn every_run_of_a_hir_dump_settles_e0004s_paths_on_windows() {
+    let windows = b"(Diagnostics\n  (Error \"E0004\" \"`a` is both: app\\x5csrc\\x5ca.nova, and the entry at app\\x5cnova.toml:6\"\n".to_vec();
+    let settled = settled_stdout(RESOLUTION_S0, windows.clone(), true);
+    assert_ne!(settled, windows);
+    let hir_args: Vec<&str> = HIR_S0.iter().chain(["a.nova"].iter()).copied().collect();
+    assert_eq!(settled_stdout(&hir_args, windows.clone(), true), settled);
+    assert_eq!(
+        settled_stdout(&["hir", "--std", "std"], windows.clone(), true),
+        settled
+    );
+    assert_eq!(settled_stdout(&hir_args, windows.clone(), false), windows);
+}
+
+/// std's three parts, every corpus root (natively, for the WASM target
+/// and under test mode) and the cases of each mode, through the HIR dump
+/// (5.3a, spec 5.3 §3.6): each root's part is there, every form the dump
+/// prints is listed and every listed form but [`nova_driver::hir_dump::LATER_KINDS`]
+/// occurs, and each of the 16 item codes occurs, E0092 only for the WASM
+/// target.
+#[test]
+fn the_hir_dumps_agree_on_std_the_corpus_and_the_cases() {
+    let root = root();
+    let std_parts: Vec<String> = std::thread::scope(|scope| {
+        let parts: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
+            .into_iter()
+            .map(|mode| {
+                let root = &root;
+                scope.spawn(move || agree_on_hir_dump(std_hir(mode), root, &[], 0))
+            })
+            .collect();
+        joined(parts)
+    });
+    let loose = loose_corpus();
+    let packages = package_corpus();
+    let wasm = wasm_corpus();
+    let tests = test_corpus();
+    let (check, wasm_dumps, test_dumps, counts) = std::thread::scope(|scope| {
+        let cases: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
+            .into_iter()
+            .map(|mode| scope.spawn(move || agree_on_hir_cases(mode, &hir_cases_dir(mode))))
+            .collect();
+        let package_dumps = scope.spawn(|| {
+            agree_in_parts(&packages, 3, |part| {
+                agree_on_hir_in(Mode::Check, &root, part, 0)
+            })
+        });
+        let wasm_dumps = scope.spawn(|| agree_on_hir_in(Mode::Wasm, &root, &wasm, 0));
+        let test_dumps = scope.spawn(|| agree_on_hir_in(Mode::Test, &root, &tests, 0));
+        let mut check = agree_on_hir_in(Mode::Check, &root, &loose, 0);
+        check.push_str(&package_dumps.join().expect("the packages' thread"));
+        let mut wasm_dumps = wasm_dumps.join().expect("the WASM programs' thread");
+        let mut test_dumps = test_dumps.join().expect("the tests' thread");
+        let mut counts = Vec::new();
+        for (mode, case) in [Mode::Check, Mode::Wasm, Mode::Test].into_iter().zip(cases) {
+            let (dumps, count) = case.join().expect("a mode's cases' thread");
+            counts.push(count);
+            match mode {
+                Mode::Check => check.push_str(&dumps),
+                Mode::Wasm => wasm_dumps.push_str(&dumps),
+                Mode::Test => test_dumps.push_str(&dumps),
+            }
+        }
+        (check, wasm_dumps, test_dumps, counts)
+    });
+    // 5.3a's cases: 164 under `check` (154 extracted, 10 by hand), 2 for
+    // the WASM target, 1 under test mode.
+    assert!(counts[0] >= 150, "only {} type-checker cases", counts[0]);
+    assert!(counts[1] >= 2, "only {} WASM cases", counts[1]);
+    assert!(counts[2] >= 1, "only {} test-mode cases", counts[2]);
+    let parts = |dumps: &str, head: &str| dumps.lines().filter(|l| l.starts_with(head)).count();
+    assert_eq!(
+        parts(&check, "(Hir target=native checked="),
+        loose.len() + packages.len() + counts[0]
+    );
+    assert_eq!(
+        parts(&wasm_dumps, "(Hir target=wasm checked="),
+        wasm.len() + counts[1]
+    );
+    assert_eq!(
+        parts(&test_dumps, "(Hir target=native mode=test checked="),
+        tests.len() + counts[2]
+    );
+    // Most programs reach the type checker.
+    let checked = parts(&check, "(Hir target=native checked=true");
+    assert!(
+        checked * 10 > parts(&check, "(Hir ") * 8,
+        "{checked} checked"
+    );
+    let found = hir_forms(&[
+        &check,
+        &wasm_dumps,
+        &test_dumps,
+        &std_parts[0],
+        &std_parts[1],
+        &std_parts[2],
+    ]);
+    let listed = |f: &String| {
+        f.ends_with("=<n>")
+            || f.ends_with("=<id>")
+            || nova_driver::hir_dump::KINDS.contains(&f.as_str())
+            || nova_driver::hir_dump::LATER_KINDS.contains(&f.as_str())
+    };
+    let unlisted: Vec<&String> = found.iter().filter(|f| !listed(f)).collect();
+    assert!(unlisted.is_empty(), "{unlisted:?} are in no list");
+    let missing: Vec<&str> = nova_driver::hir_dump::KINDS
+        .iter()
+        .copied()
+        .filter(|k| !found.contains(*k))
+        .collect();
+    assert!(missing.is_empty(), "no HIR dump holds {missing:?}");
+    let later: Vec<&str> = nova_driver::hir_dump::LATER_KINDS
+        .iter()
+        .copied()
+        .filter(|k| found.contains(*k))
+        .collect();
+    assert!(later.is_empty(), "{later:?} appear");
+    let mut all = codes_in(&check);
+    for (code, n) in codes_in(&test_dumps) {
+        *all.entry(code).or_insert(0) += n;
+    }
+    // The resolver's E0092 (a std module the target lacks, imported)
+    // occurs natively too; the item passes' only for the WASM target.
+    assert!(
+        wasm_dumps
+            .contains("which target wasm does not have\"\n    (Primary \"every_item_e0092_site\""),
+        "no item E0092"
+    );
+    for (code, n) in codes_in(&wasm_dumps) {
+        *all.entry(code).or_insert(0) += n;
+    }
+    let absent: Vec<&str> = ITEM_CODES
+        .iter()
+        .copied()
+        .filter(|c| !all.contains_key(*c))
+        .collect();
+    assert!(absent.is_empty(), "{absent:?} occur nowhere: {all:?}");
+    eprintln!(
+        "the HIR pass dumps {} loose programs, {} package files, {} WASM programs, {} files \
+         with tests and {:?} cases; codes {all:?}",
+        loose.len(),
+        packages.len(),
+        wasm.len(),
+        tests.len(),
+        counts
+    );
+}
+
+/// The type checker's cases of every mode as a Windows checkout has them
+/// ([`crlf`]), as the resolver's are compared
+/// (`the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`):
+/// a mode whose copy changes no file is not run again.
+#[test]
+fn the_hir_dumps_agree_on_the_cases_with_windows_line_endings() {
+    let dir = fresh_dir("crlf-hir");
+    let mut crlf_files = 0;
+    let mut changed_modes = Vec::new();
+    for mode in [Mode::Check, Mode::Wasm, Mode::Test] {
+        let (held, changed) = copy_crlf(&hir_cases_dir(mode), &dir.join(hir_cases_name(mode)));
+        crlf_files += held;
+        if changed > 0 {
+            changed_modes.push(mode);
+        }
+    }
+    assert!(
+        crlf_files > 150,
+        "only {crlf_files} cases have CRLF line endings"
+    );
+    std::thread::scope(|scope| {
+        let modes: Vec<_> = changed_modes
+            .into_iter()
+            .map(|mode| {
+                let cases = dir.join(hir_cases_name(mode));
+                scope.spawn(move || (mode, agree_on_hir_cases(mode, &cases)))
+            })
+            .collect();
+        for handle in modes {
+            let (mode, (dumps, count)) = handle.join().expect("a mode's thread");
+            let parts = dumps.lines().filter(|l| l.starts_with("(Hir ")).count();
+            assert_eq!(parts, count, "{mode:?}");
+        }
+    });
+}
+
+/// The type fuzzer's seeds (5.3a): its own, past the name fuzzer's.
+const TYPE_SEEDS: [u64; 3] = [401, 402, 403];
+const TYPE_COUNT: usize = 40;
+
+/// The loose corpus programs the type fuzzer starts from: those that
+/// import no module, whose HIR dump under `check` reaches the type checker
+/// with no diagnostic, so that every diagnostic of an input is its
+/// mutations'. Read from S0's dump, which is the same on every system.
+fn type_bases() -> Vec<String> {
+    let root = root();
+    let candidates: Vec<String> = loose_corpus()
+        .into_iter()
+        .filter(|f| {
+            let text = std::fs::read_to_string(root.join(f)).expect("read a corpus file");
+            !text.lines().any(|l| l.starts_with("import "))
+                && fuzz::parses(&text.replace("\r\n", "\n"))
+        })
+        .collect();
+    let s0 = run(&nova(), HIR_S0, &root, &candidates).unwrap_or_else(|e| panic!("{e}"));
+    let dump = String::from_utf8(s0.stdout).expect("the HIR dump is UTF-8");
+    let mut bases = Vec::new();
+    let mut current: Option<&str> = None;
+    let mut clean = false;
+    for line in dump.lines().chain(["== "]) {
+        if let Some(file) = line.strip_prefix("== ") {
+            if let (Some(f), true) = (current, clean) {
+                bases.push(f.to_string());
+            }
+            current = Some(file);
+            clean = false;
+        } else if line == "(Diagnostics)" {
+            clean = true;
+        } else if line.starts_with("(Hir target=native checked=false") {
+            current = None;
+        }
+    }
+    bases
+}
+
+/// `count` type-mutated programs from `seed` ([`fuzz::type_mutate`]), each
+/// one of `bases` that the seed picks, read with each `\r\n` as `\n`, so a
+/// seed gives the same inputs on every system.
+fn type_inputs(bases: &[String], seed: u64, count: usize) -> Vec<String> {
+    let root = root();
+    let mut bases = bases.to_vec();
+    bases.sort();
+    let mut rng = SplitMix64::new(seed);
+    (0..count)
+        .map(|i| {
+            let file = rng.pick(&bases);
+            let source = std::fs::read_to_string(root.join(file))
+                .expect("read a corpus file")
+                .replace("\r\n", "\n");
+            fuzz::type_mutate(&mut rng, &source, i)
+        })
+        .collect()
+}
+
+/// The type fuzzer's item mutations (5.3a, spec 5.3 §3.6): each seed's
+/// inputs reach the type checker, and each code the mutations aim at
+/// occurs at least twice over the fixed seeds.
+#[test]
+fn the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code() {
+    let bases = type_bases();
+    // 160 of the loose corpus at 5.3a.
+    assert!(bases.len() >= 100, "only {} bases", bases.len());
+    let seeded: Vec<(u64, Vec<String>)> = TYPE_SEEDS
+        .iter()
+        .map(|&seed| (seed, type_inputs(&bases, seed, TYPE_COUNT)))
+        .collect();
+    let dir = fresh_dir("types");
+    let mut roots = Vec::new();
+    for (seed, inputs) in &seeded {
+        let sub = dir.join(seed.to_string());
+        std::fs::create_dir_all(&sub).expect("create a seed's directory");
+        for (i, input) in inputs.iter().enumerate() {
+            let file = format!("f{i:05}.nova");
+            std::fs::write(sub.join(&file), input).expect("write a type-fuzz input");
+            roots.push(format!("{seed}/{file}"));
+        }
+    }
+    let dump = agree_on_hir_in(Mode::Check, &dir, &roots, 0);
+    let unchecked: Vec<&str> = dump
+        .split("== ")
+        .filter(|p| p.contains("(Hir target=native checked=false"))
+        .map(|p| p.lines().next().unwrap_or(""))
+        .collect();
+    assert!(
+        unchecked.is_empty(),
+        "inputs that stop before the type checker: {unchecked:?}"
+    );
+    let codes = codes_in(&dump);
+    let mut aimed: Vec<&str> = fuzz::TYPE_ITEMS.iter().map(|(c, _)| *c).collect();
+    aimed.sort();
+    aimed.dedup();
+    let short: Vec<(&str, usize)> = aimed
+        .iter()
+        .map(|c| (*c, codes.get(*c).copied().unwrap_or(0)))
+        .filter(|(_, n)| *n < 2)
+        .collect();
+    assert!(
+        short.is_empty(),
+        "aimed codes under two: {short:?}; all: {codes:?}"
+    );
+    for seed in TYPE_SEEDS {
+        let part = seed_parts(&dump, seed);
+        assert_eq!(part.matches("(Hir ").count(), TYPE_COUNT, "seed {seed}");
+    }
+    eprintln!("the type fuzzer's codes over seeds {TYPE_SEEDS:?}: {codes:?}");
+}
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-cli/tests/selfhost/fuzz.rs b/crates/nova-cli/tests/selfhost/fuzz.rs
index 71a01748..82d23aca 100644
--- a/crates/nova-cli/tests/selfhost/fuzz.rs
+++ b/crates/nova-cli/tests/selfhost/fuzz.rs
@@ -31,6 +31,15 @@
 //!    [`PACKAGE_FILES`] lays out, with its sibling modules, a dependency,
 //!    a dev-dependency and a module named as a dependency.
 //!
+//! 5. [`type_mutate`] (5.3a, spec
+//!    `docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
+//!    §3.6): a corpus program that checks at the item level, mutated at
+//!    the level of items, so that the item passes' codes occur among the
+//!    fixed seeds' inputs: a parameter's, a return's or a field's type, a
+//!    duplicated generic, a trait impl's method dropped, added or retyped,
+//!    an impl duplicated, and items appended that give each aimed code.
+//!    5.3b adds the body mutations.
+//!
 //! Beside them, [`edges`]: fixed inputs that no file under `cases/` can
 //! hold as it is, such as a lone `\r` or a NUL.
 
@@ -1010,3 +1019,213 @@ pub fn package_name_mutate(rng: &mut SplitMix64, source: &str, stem: &str) -> (b
 pub fn parses(source: &str) -> bool {
     named_items(source).is_some()
 }
+
+/// A type that the item passes refuse, for [`type_mutate`]'s retyping:
+/// E0001 (a type nowhere, `_`), E0012 (arities) and E0900 (the types not
+/// supported yet).
+const TYPE_REPLACEMENTS: [&str; 10] = [
+    "FzNowhere",
+    "Option<Int, Int>",
+    "Option",
+    "Int<Bool>",
+    "&Int",
+    "Int?",
+    "_",
+    "[FzMissing]",
+    "core::Int",
+    "Future<Int, Bool>",
+];
+
+/// Items [`type_mutate`] appends, `{n}` replaced by a number of the
+/// input's own so that no name repeats, each aimed at an item code: the
+/// code it gives, then the text; `{chain}` is [`binding_chain`]'s.
+pub const TYPE_ITEMS: [(&str, &str); 24] = [
+    ("E0001", "fn fz_missing{n}(x: FzNowhere{n}) {}"),
+    ("E0001", "impl FzNoTrait{n} for Int {}"),
+    ("E0001", "fn fz_bound{n}<T: FzNoBound{n}>(x: T) {}"),
+    ("E0012", "fn fz_arity{n}(x: Option) -> Result<Int> { panic(\"\") }"),
+    ("E0015", "trait FzOne{n} {\n    type Out\n}\ntrait FzTwo{n} {\n    type Out\n}\nfn fz_amb{n}<T: FzOne{n} + FzTwo{n}>(x: T::Out) {}"),
+    ("E0070", "trait FzAssoc{n} {\n    type Item\n    fn get(self) -> Int\n}\nrecord FzR{n} {}\nimpl FzAssoc{n} for FzR{n} {}"),
+    ("E0071", "record FzIn{n} {}\nimpl FzIn{n} {\n    type Item = Int\n}"),
+    ("E0071", "trait FzSmall{n} {}\nrecord FzS{n} {}\nimpl FzSmall{n} for FzS{n} {\n    fn extra(self) {}\n}"),
+    ("E0072", "trait FzSup{n} {}\ntrait FzSub{n}: FzSup{n} {}\nrecord FzT{n} {}\nimpl FzSub{n} for FzT{n} {}"),
+    ("E0072", "trait FzRet{n} {\n    fn f(self) -> Int\n}\nrecord FzU{n} {}\nimpl FzRet{n} for FzU{n} {\n    fn f(self) -> Bool { true }\n}"),
+    ("E0073", "record FzV{n} {}\nimpl<T> FzV{n} {}"),
+    ("E0074", "record FzW{n} {}\nimpl FzW{n} {\n    fn m(self) {}\n}\nimpl FzW{n} {\n    fn m(self) {}\n}"),
+    ("E0074", "record FzX{n} {}\nimpl Eq for FzX{n} {\n    fn eq(self, o: FzX{n}) -> Bool { true }\n}\nimpl Eq for FzX{n} {\n    fn eq(self, o: FzX{n}) -> Bool { false }\n}"),
+    ("E0076", "fn fz_self{n}<Self>() {}"),
+    ("E0077", "trait FzCyc{n} {\n    type A\n    type B\n}\nrecord FzY{n} {}\nimpl FzCyc{n} for FzY{n} {\n    type A = Self::B\n    type B = Self::A\n}"),
+    ("E0078", "{chain}"),
+    ("E0403", "record FzDup{n}<T, T> { t: T }"),
+    ("E0403", "trait FzBind{n} {\n    type Item\n}\nimpl FzBind{n} for Bool {\n    type Item = Int\n    type Item = Int\n}"),
+    ("E0105", "trait FzLast{n} {\n    fn last(self) -> Int\n}\nimpl<T> FzLast{n} for Vec<T> {\n    fn last(self) -> Int { 0 }\n}"),
+    ("E0010", "impl (Int, Bool) {}"),
+    ("E0010", "trait FzArr{n} {}\nimpl FzArr{n} for [Int] {}"),
+    ("E0900", "extern \"C\" {\n    fn fz_ffi{n}(x: String) -> [Int]\n}"),
+    ("E0900", "extern \"Rust\" {\n    fn fz_abi{n}()\n}"),
+    ("E0900", "extern \"C\" {\n    fn nova_fz{n}()\n}"),
+];
+
+/// A span of a source's bytes, start and end.
+type Range = (usize, usize);
+
+/// What [`type_mutate`] edits in place: the spans of the program's types
+/// (parameters', returns', fields'), of its generic lists' first
+/// parameters, of its trait impls' methods and its impls.
+struct TypeSites {
+    types: Vec<(usize, usize)>,
+    generics: Vec<(usize, usize)>,
+    /// Each trait impl's methods: (start of its line, end of its body,
+    /// end of its return type or of its parameter list's last type).
+    methods: Vec<(usize, usize, Option<Range>)>,
+    /// Each trait impl's body's last byte, its `}`.
+    trait_impl_ends: Vec<usize>,
+    impls: Vec<(usize, usize)>,
+}
+
+fn type_sites(source: &str) -> Option<TypeSites> {
+    use nova_ast::Item;
+    let (tokens, lex_errors) = nova_lexer::lex(source, FileId::DUMMY);
+    if !lex_errors.is_empty() {
+        return None;
+    }
+    let parsed = nova_parser::parse_recovering(&tokens, FileId::DUMMY);
+    if !parsed.errors.is_empty() {
+        return None;
+    }
+    let span = |s: nova_diagnostics::Span| (s.start as usize, s.end as usize);
+    let mut sites = TypeSites {
+        types: Vec::new(),
+        generics: Vec::new(),
+        methods: Vec::new(),
+        trait_impl_ends: Vec::new(),
+        impls: Vec::new(),
+    };
+    let function = |f: &nova_ast::item::Function, sites: &mut TypeSites| {
+        sites.types.extend(
+            f.params
+                .iter()
+                .filter(|p| p.name.value != "self")
+                .map(|p| span(p.ty.span)),
+        );
+        sites.types.extend(f.return_ty.iter().map(|t| span(t.span)));
+        sites
+            .generics
+            .extend(f.generics.first().map(|g| span(g.name.span)));
+    };
+    for item in &parsed.file.items {
+        match &item.value {
+            Item::Function(f) => function(f, &mut sites),
+            Item::Record(r) => {
+                sites.types.extend(r.fields.iter().map(|f| span(f.ty.span)));
+                sites
+                    .generics
+                    .extend(r.generics.first().map(|g| span(g.name.span)));
+            }
+            Item::Impl(b) => {
+                sites.impls.push(span(item.span));
+                sites
+                    .generics
+                    .extend(b.generics.first().map(|g| span(g.name.span)));
+                for f in &b.functions {
+                    function(f, &mut sites);
+                    if b.trait_.is_some() {
+                        let start = line_start(source, f.name.span.start as usize);
+                        let typed = f.return_ty.as_ref().map(|t| span(t.span));
+                        sites.methods.push((start, f.body.span.end as usize, typed));
+                    }
+                }
+                if b.trait_.is_some() {
+                    sites.trait_impl_ends.push(item.span.end as usize - 1);
+                }
+            }
+            _ => {}
+        }
+    }
+    Some(sites)
+}
+
+/// `source`, a program that checks at the item level, mutated one to
+/// three times at the level of items (module docs, 5.): a type replaced
+/// by one of [`TYPE_REPLACEMENTS`], a generic list's first parameter
+/// duplicated, a trait impl's method dropped, added or given another
+/// return type, an impl duplicated, or one of [`TYPE_ITEMS`] appended,
+/// each numbered from `n`. A mutation that would not parse is skipped;
+/// every name an appended item declares is new, so every input reaches
+/// the type checker, which the harness checks.
+pub fn type_mutate(rng: &mut SplitMix64, source: &str, n: usize) -> String {
+    let mut text = source.to_string();
+    for step in 0..1 + rng.below(3) {
+        let Some(sites) = type_sites(&text) else {
+            break;
+        };
+        let splice = |(start, end): (usize, usize), with: &str| {
+            format!("{}{with}{}", &text[..start], &text[end..])
+        };
+        let edited = match rng.below(20) {
+            0..=3 if !sites.types.is_empty() => Some(splice(
+                *rng.pick(&sites.types),
+                rng.pick(&TYPE_REPLACEMENTS),
+            )),
+            4 if !sites.generics.is_empty() => {
+                let (start, end) = *rng.pick(&sites.generics);
+                let name = text[start..end].to_string();
+                Some(splice((start, end), &format!("{name}, {name}")))
+            }
+            5 if !sites.methods.is_empty() => {
+                let (start, end, _) = *rng.pick(&sites.methods);
+                Some(splice((start, end), ""))
+            }
+            6 if !sites.trait_impl_ends.is_empty() => {
+                let at = *rng.pick(&sites.trait_impl_ends);
+                Some(splice(
+                    (at, at),
+                    &format!("    fn fz_added{n}(self) {{}}\n"),
+                ))
+            }
+            7 if sites.methods.iter().any(|m| m.2.is_some()) => {
+                let typed: Vec<(usize, usize)> = sites.methods.iter().filter_map(|m| m.2).collect();
+                Some(splice(*rng.pick(&typed), "(Int, Bool)"))
+            }
+            8 if !sites.impls.is_empty() => {
+                let (start, end) = *rng.pick(&sites.impls);
+                let copy = text[start..end].to_string();
+                Some(format!("{}\n{copy}\n", text.trim_end_matches('\n')))
+            }
+            _ => {
+                let (_, item) = rng.pick(&TYPE_ITEMS);
+                let item = item
+                    .replace("{chain}", &binding_chain())
+                    .replace("{n}", &format!("{n}x{step}"));
+                Some(format!("{}\n{item}\n", text.trim_end_matches('\n')))
+            }
+        };
+        if let Some(next) = edited {
+            if type_sites(&next).is_some() {
+                text = next;
+            }
+        }
+    }
+    text
+}
+
+/// An impl whose associated types double at each of 17 steps, so that the
+/// method signature's check meets more type nodes than normalizing allows
+/// (E0078), as the type checker's own test of the limit builds one.
+fn binding_chain() -> String {
+    let mut text = String::from("record FzPair{n}<A, B> { a: A, b: B }\ntrait FzChain{n} {\n");
+    for i in 0..17 {
+        text.push_str(&format!("    type A{i}\n"));
+    }
+    text.push_str(
+        "    fn put(self, y: Self::A0) -> Int\n}\nrecord FzW{n} {}\nimpl FzChain{n} for FzW{n} {\n",
+    );
+    for i in 0..16 {
+        let next = i + 1;
+        text.push_str(&format!(
+            "    type A{i} = FzPair{{n}}<Self::A{next}, Self::A{next}>\n"
+        ));
+    }
+    text.push_str("    type A16 = Int\n    fn put(self, y: Self::A0) -> Int { 0 }\n}");
+    text
+}
````

### Task 3: N's item passes, its HIR dump and `selfhost hir`, and `selfhost.rs` in a CI job of its own

Spec §3.1 to §3.7; decisions 5, 10, 12, 15, 17, 18 and 20.

**Files:**
- Create: `selfhost/src/hir.nova`, `selfhost/src/typeck.nova`,
  `selfhost/src/typeck_ty.nova`, `selfhost/src/typeck_traits.nova`,
  `selfhost/src/fixes.nova`, `selfhost/src/hir_dump.nova`,
  `selfhost/src/typeck_tests.nova`
- Modify: `selfhost/src/main.nova`, `selfhost/src/resolver.nova`,
  `selfhost/src/resolution_dump.nova`, `crates/nova-cli/tests/selfhost.rs`,
  `crates/nova-cli/Cargo.toml`, `.github/workflows/ci.yml`,
  `.github/PULL_REQUEST_TEMPLATE.md`, `CONTRIBUTING.md`, `agent.md`

- [ ] **Step 1: The tests.** `patch_block t3-tests`:
  - `typeck_tests.nova`, N's 12 `@test`s of the item passes' edges (HIR's
    equality, substitution, matching and overlap, the normalization limit,
    the generic-parameter checks, arities, an impl's missing items, cyclic
    bindings, coherence's labels, a method's signature against its trait's,
    an unknown type's fixes and where an import's edit goes), imported by
    `main.nova`;
  - `selfhost.rs`: `N_HIR` true and asserted in a constant, so every
    5.3a set compares both sides through `compare_batch`; the HIR dump's
    refusals (`both_sides_take_test_in_every_position_alike`, the WASM and
    test-mode refusal) compared on both sides, and an unreadable root
    (`IOERR InvalidData`) in the header test;
    `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`
    (N with a copy of std whose `core` names a type it lacks: N falls
    back to checking std for each program, which keeps std's E0001 in
    each dump; otherwise S0's; its comment says that S0's own fallback is
    unreachable today, mutant 50);
    `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`
    (spec §3.7: every written nesting of a type, arrays, a generic
    record's arguments, tuples, `fn` types' parameters and returns,
    parentheses, optionals, references, mutable references and pointers,
    at 128 levels, the parser's limit, in each item position, a parameter,
    a return, a field, a variant's payload, a constant, a trait's method,
    an impl's self type, an impl's method alike and unlike its trait's,
    an associated type's binding, a `where` bound and an extern, ten files
    under `nova-selfhost-deep-n-items` (the item passes walk arrays,
    generic arguments, tuples and `fn` types 128 levels deep; parentheses
    are gone once parsed, and the other four stop at the outermost level
    with one E0900); S0's `check --dump-hir` reaches
    the type checker in each with no P0002, and N's `selfhost hir`, under
    `ulimit -s 1024` on Unix and as it is on Windows, prints S0's dump,
    each side's output settled as `run` settles it (Review Focus 1);
    5.4 s here, N's build included, run alone).
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. Then `SH`. Expected: `18 passed; 34 failed`: N does not
  build, `error[E0001]: cannot find module` `fixes`, `hir` and `typeck`
  (imported by `typeck_tests.nova`), so every test that builds N fails,
  `the_selfhost_tests_pass`, every comparison of 5.1, 5.2 and 5.3a, and
  both 1 MB tests of N among them, the new item test too; the 18 that
  pass run S0 alone or no N.
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **N:** `hir.nova` (`Ty`, `TyHead`, the tables, `ty_eq`, `subst`,
    `match_pattern`, `normalize_ty` with its limits, `self_types_overlap`);
    `typeck.nova` (the checker's state, `check_items`, `SharedItems`,
    `share_items`, the splices and the rebasing, the generic-parameter
    checks, arities, supertraits, records, sums); `typeck_ty.nova`
    (`convert_ty`, projections, `normalize`, bounds and `where`,
    signatures, externs); `typeck_traits.nova` (traits, impls, conformance,
    method signatures, coherence, hidden methods, supertrait impls,
    associated-type cycles); `fixes.nova` (`push_fix`, `name_fixes`, the
    import fix and its edit, "did you mean"); `hir_dump.nova`; `main.nova`'s
    `selfhost hir --std DIR [--target native|wasm] [--test] (--std-part |
    FILE...)`, on `selfhost resolve`'s parser; `resolver.nova`'s and
    `resolution_dump.nova`'s helpers;
  - **the CI job:** `ci.yml`'s Selfhost job on each system
    (`NOVA_EMBED_RUNTIME=1`; `cargo test … --test selfhost`; N's own tests
    on the debug runtime through `NOVA_RUNTIME_LIB`) and the clippy job's
    `cargo clippy --locked --all-features --test selfhost -- -D warnings`;
    `crates/nova-cli/Cargo.toml`'s `[[test]] name = "selfhost" … test =
    false`; `selfhost.rs`'s note; `CONTRIBUTING.md`, `agent.md` and the
    pull request template name the two extra commands.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH` (43 to 53 s, by the day). Expected: `52 passed; 0 failed`, the 7th and 8th new
    tests Step 1's, and `the_selfhost_tests_pass`'s N run giving `44
    passed`.
  - `S` (223 s). Expected: `2830 1 10`, the IPv6 test the one failure;
    `selfhost.rs` is no longer among them (`test = false`).
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `selfhost: N's item passes, its HIR dump and
      selfhost hir, and selfhost.rs in a CI job of its own`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 64f53dbd..1f1625a9 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -948,19 +948,37 @@ fn both_sides_refuse_no_file() {
         s0: leak_args(&[wasm_test.s0, &["--std"]].concat()),
         n: leak_args(&[wasm_test.n, &["--std-part"]].concat()),
     };
+    // The HIR dump's (5.3a): S0's `check --dump-hir` shares the resolution
+    // dump's parser, N's `hir` the resolution dump's.
+    let hir_wasm_test = Dump {
+        s0: leak_args(&[HIR_S0, &["--test", "--target", "wasm"]].concat()),
+        n: leak_args(&["hir", "--std", &std_dir(), "--test", "--target", "wasm"]),
+    };
+    let hir_wasm_test_std = Dump {
+        s0: leak_args(&[hir_wasm_test.s0, &["--std"]].concat()),
+        n: leak_args(&[hir_wasm_test.n, &["--std-part"]].concat()),
+    };
     for dump in [
         &TOKENS,
         &AST,
         resolution(),
         resolution_in(Mode::Wasm),
         resolution_in(Mode::Test),
+        hir_in(Mode::Check),
+        hir_in(Mode::Wasm),
+        hir_in(Mode::Test),
     ] {
         let out = compare_batch(dump, &root(), &[], 2, &nova(), selfhost());
         assert_eq!(out, Ok(String::new()));
     }
     let main = ["tests/runtime/array_helpers.nova".to_string()];
     assert!(root().join(&main[0]).is_file());
-    for (dump, files) in [(&wasm_test, &main[..]), (&wasm_test_std, &[])] {
+    for (dump, files) in [
+        (&wasm_test, &main[..]),
+        (&wasm_test_std, &[]),
+        (&hir_wasm_test, &main[..]),
+        (&hir_wasm_test_std, &[]),
+    ] {
         let out = compare_batch(dump, &root(), files, 2, &nova(), selfhost());
         assert_eq!(out, Ok(String::new()));
     }
@@ -998,8 +1016,15 @@ fn both_sides_take_test_in_every_position_alike() {
         (&["--target=wasm", "--target", "wasm", "--std"], 2),
         (&["--target=bogus", file], 2),
     ];
-    for (args, code) in shapes {
-        let n: Vec<&str> = ["resolve", "--std", std.as_str()]
+    // Both dumps: the HIR dump's forms take what the resolution dump's
+    // take (5.3a, ADR 0055).
+    for ((args, code), (s0, command)) in shapes.iter().flat_map(|shape| {
+        [
+            (shape, (RESOLUTION_S0, "resolve")),
+            (shape, (HIR_S0, "hir")),
+        ]
+    }) {
+        let n: Vec<&str> = [command, "--std", std.as_str()]
             .into_iter()
             .chain(
                 args.iter()
@@ -1007,11 +1032,11 @@ fn both_sides_take_test_in_every_position_alike() {
             )
             .collect();
         let dump = Dump {
-            s0: leak_args(&[RESOLUTION_S0, args].concat()),
+            s0: leak_args(&[s0, *args].concat()),
             n: leak_args(&n),
         };
-        if let Err(report) = compare_batch(&dump, &root(), &[], code, &nova(), selfhost()) {
-            panic!("{args:?}: {report}");
+        if let Err(report) = compare_batch(&dump, &root(), &[], *code, &nova(), selfhost()) {
+            panic!("{command} {args:?}: {report}");
         }
     }
 }
@@ -3531,7 +3556,11 @@ fn the_name_fuzzer_agrees_and_gives_every_resolver_code() {
 /// Whether N has `selfhost hir --std DIR [--target wasm] [--test] FILE…`
 /// (or `--std-part`), the HIR dump's other side (spec 5.3 §3.3): part B of
 /// 5.3a sets it.
-const N_HIR: bool = false;
+const N_HIR: bool = true;
+
+// From part B on, N's side is compared: the HIR dump's checks hold on
+// both sides, never on S0's alone.
+const _: () = assert!(N_HIR, "5.3a's part B compares N's side");
 
 /// The item codes of 5.3a (spec 5.3 §3.2): each occurs among the HIR
 /// dumps of the cases, the corpus and the fuzzer's inputs. E0092 is the
@@ -3758,6 +3787,7 @@ fn the_hir_dump_has_a_header_per_root_and_std_its_own_part() {
         .expect("write a.nova");
     std::fs::write(dir.join("b.nova"), "fn g( {").expect("write b.nova");
     std::fs::write(dir.join("c.nova"), "fn f(x: Nope) {}\nfn f() {}\n").expect("write c.nova");
+    std::fs::write(dir.join("latin1.nova"), b"fn \xe9() {}\n").expect("write latin1.nova");
     std::fs::create_dir_all(dir.join("pkg").join("src")).expect("create pkg/src");
     std::fs::write(dir.join("pkg").join("nova.toml"), MANIFEST).expect("write nova.toml");
     std::fs::write(
@@ -3775,12 +3805,17 @@ fn the_hir_dump_has_a_header_per_root_and_std_its_own_part() {
         "missing.nova",
         "b.nova",
         "c.nova",
+        "latin1.nova",
         "pkg/src/main.nova",
     ]
     .iter()
     .map(|f| f.to_string())
     .collect();
     let out = agree_on_hir_in(Mode::Check, &dir, &files, 2);
+    assert!(
+        out.contains("== latin1.nova\nIOERR InvalidData\n== pkg/src/main.nova\n"),
+        "{out}"
+    );
     let headers: Vec<&str> = out.lines().filter(|l| l.starts_with("== ")).collect();
     assert_eq!(
         headers,
@@ -3870,6 +3905,16 @@ fn the_hir_dumps_agree_on_std_the_corpus_and_the_cases() {
     let packages = package_corpus();
     let wasm = wasm_corpus();
     let tests = test_corpus();
+    // The resolution dump's floors (5.2b): 251 loose programs, 74 package
+    // files, 39 WASM programs and 10 files with tests at 5.3a.
+    assert!(loose.len() >= 240, "only {} loose programs", loose.len());
+    assert!(
+        packages.len() >= 60,
+        "only {} package files",
+        packages.len()
+    );
+    assert!(wasm.len() >= 35, "only {} WASM programs", wasm.len());
+    assert!(tests.len() >= 8, "only {} files with tests", tests.len());
     let (check, wasm_dumps, test_dumps, counts) = std::thread::scope(|scope| {
         let cases: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
             .into_iter()
@@ -4017,6 +4062,178 @@ fn the_hir_dumps_agree_on_the_cases_with_windows_line_endings() {
     });
 }
 
+/// N shares std's item tables only when std alone gives no diagnostic, as
+/// S0 does (5.3a, spec 5.3 §3.4): with a std whose `core` names a type it
+/// lacks, an item E0001 and no resolver diagnostic, N checks std again
+/// for each program, so each program that reaches the type checker keeps
+/// std's E0001 among its diagnostics, and its dump is otherwise S0's,
+/// whose std is built in; std's part holds it too. S0's own fallback is
+/// unreachable today: its built-in std gives no item diagnostic, so no
+/// input reaches it (the 5.3a plan's mutant 50, which passes by design).
+/// The roots are the first eight loose cases of `cases/check/`.
+#[test]
+fn n_checks_std_for_each_program_when_std_gives_an_item_diagnostic() {
+    let std = std_copy("item-error-std");
+    let file = "core/lib.nova";
+    let (old, new) = ("fn mix64(x: Int) -> Int {", "fn mix64(x: Intt) -> Int {");
+    let text = std::fs::read_to_string(std.join(file))
+        .expect("read the std file")
+        .replace("\r\n", "\n");
+    assert_eq!(text.matches(old).count(), 1, "{file}: {old}");
+    std::fs::write(std.join(file), text.replacen(old, new, 1)).expect("alter the std file");
+    let mark = "(Error \"E0001\" \"cannot find type `Intt`\"";
+    let std = std.display().to_string();
+    let cases = hir_cases_dir(Mode::Check);
+    let roots: Vec<String> = case_roots(&cases).loose[..8].to_vec();
+    let s0 = run(&nova(), HIR_S0, &cases, &roots).expect("run S0");
+    let n = run(selfhost(), &["hir", "--std", &std], &cases, &roots).expect("run N");
+    assert_eq!((s0.code, n.code), (Some(0), Some(0)), "{}", n.stderr);
+    let (s0, n) = (
+        String::from_utf8_lossy(&s0.stdout).into_owned(),
+        String::from_utf8_lossy(&n.stdout).into_owned(),
+    );
+    let (s0_parts, n_parts) = (parts_of(&s0), parts_of(&n));
+    assert_eq!(n_parts.len(), roots.len(), "{n}");
+    let mut checked = 0;
+    for (s0_part, n_part) in s0_parts.iter().zip(&n_parts) {
+        if !s0_part.contains(" checked=true\n") {
+            assert_eq!(n_part, s0_part);
+            continue;
+        }
+        checked += 1;
+        assert_eq!(n_part.matches(mark).count(), 1, "{n_part}");
+        assert_eq!(
+            without_parens(&without_block(n_part, mark)),
+            without_parens(s0_part),
+            "{n_part}"
+        );
+    }
+    assert!(checked >= 6, "{checked} of {} checked", roots.len());
+    let part = run(
+        selfhost(),
+        &["hir", "--std", &std, "--std-part"],
+        &cases,
+        &[],
+    )
+    .expect("run N's std part");
+    assert_eq!(part.code, Some(0));
+    assert_eq!(
+        String::from_utf8_lossy(&part.stdout).matches(mark).count(),
+        1
+    );
+}
+
+/// N with `args` in `dir`, on a main thread of `stack` KB where `ulimit -s`
+/// can set it (Unix), or of the platform's size (Windows: 1 MB): its exit
+/// code and stdout, settled as [`run`] settles it ([`settled_stdout`]).
+fn n_args_on_stack(dir: &Path, stack: Option<u32>, args: &[&str]) -> (Option<i32>, Vec<u8>) {
+    let mut cmd = match stack {
+        Some(kb) => {
+            // `$0` is N and `$@` its arguments, so that no path is quoted.
+            let mut sh = Command::new("sh");
+            sh.arg("-c")
+                .arg(format!("ulimit -s {kb} && exec \"$0\" \"$@\""))
+                .arg(selfhost())
+                .args(args);
+            sh
+        }
+        None => {
+            let mut n = Command::new(selfhost());
+            n.args(args);
+            n
+        }
+    };
+    let out = output_within(
+        cmd.current_dir(dir).stdin(Stdio::null()),
+        CHILD_LIMIT,
+        &format!("selfhost {} on a {stack:?} KB stack", args.join(" ")),
+    )
+    .unwrap_or_else(|e| panic!("{e}"));
+    (
+        out.status.code(),
+        settled_stdout(args, out.stdout, cfg!(windows)),
+    )
+}
+
+/// N's item passes on its own stack (spec 5.3 §3.7, ADR 0055 §12): every
+/// written nesting of a type, 128 levels deep, the parser's limit, in each
+/// item position (a parameter's and a return's type, a field, a variant's
+/// payload, a constant, a trait's method, an impl's self type, an impl's
+/// method alike and unlike its trait's, an associated type's binding, a
+/// `where` bound and an extern's signature), through N's `selfhost hir` on
+/// a main thread of 1 MB, as
+/// [`n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`] runs N's
+/// parser: N's dump is S0's, which S0 checks on its 64 MB thread.
+#[test]
+fn n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread() {
+    // Each kind's level, written around `Int`: an optional's first `?` is
+    // not a level, each after it one more.
+    const KINDS: [(&str, &str, &str); 10] = [
+        ("arrays", "[", "]"),
+        ("generics", "W<", ">"),
+        ("tuples", "(", ", Int)"),
+        ("fn-parameters", "fn(", ") -> Int"),
+        ("fn-returns", "fn() -> ", ""),
+        ("parentheses", "(", ")"),
+        ("optionals", "", "?"),
+        ("references", "& ", ""),
+        ("mutable-references", "&mut ", ""),
+        ("pointers", "*", ""),
+    ];
+    // Every item position of a type, `{T}` the deep type.
+    const ITEMS: [&str; 12] = [
+        "fn f(x: {T}) {}\n",
+        "fn g() -> {T} {}\n",
+        "record R { x: {T} }\n",
+        "type S = | A({T}) | B\n",
+        "const C: {T} = 1\n",
+        "trait Tr { fn m(self, x: {T}) -> {T} }\n",
+        "trait Tr2 {}\nimpl Tr2 for {T} {}\n",
+        "trait Tr3 { fn m(self, x: {T}) -> {T} }\nrecord Q {}\nimpl Tr3 for Q { fn m(self, x: {T}) -> {T} {} }\n",
+        "trait Tr4 { fn m(self, x: Int) -> Int }\nrecord M {}\nimpl Tr4 for M { fn m(self, x: {T}) -> {T} {} }\n",
+        "trait It {\n    type Item\n    fn get(self) -> Self::Item\n}\nrecord P {}\nimpl It for P {\n    type Item = {T}\n    fn get(self) -> {T} {}\n}\n",
+        "fn h<U>(x: U) where {T}: Eq {}\n",
+        "extern \"C\" {\n    fn e(x: {T}) -> {T}\n}\n",
+    ];
+    let dir = fresh_dir("deep-n-items");
+    let mut files = Vec::new();
+    for (kind, open, close) in KINDS {
+        let extra = if kind == "optionals" { "?" } else { "" };
+        let ty = format!("{}Int{}{extra}", open.repeat(128), close.repeat(128));
+        let items: Vec<String> = ITEMS.iter().map(|item| item.replace("{T}", &ty)).collect();
+        let file = format!("{kind}-128.nova");
+        std::fs::write(
+            dir.join(&file),
+            format!("record W<T> {{ x: T }}\n\n{}", items.join("\n")),
+        )
+        .expect("write a deep input");
+        files.push(file);
+    }
+    let s0 = run(&nova(), HIR_S0, &dir, &files).expect("run S0");
+    assert_eq!(s0.code, Some(0), "{}", s0.stderr);
+    // At the parser's limit, not past it: every program reaches the type
+    // checker.
+    let text = String::from_utf8_lossy(&s0.stdout);
+    assert_eq!(
+        text.matches(" checked=true\n").count(),
+        files.len(),
+        "{text}"
+    );
+    assert!(!text.contains("\"P0002\""), "{text}");
+    let std_dir = std_dir();
+    let mut args = vec!["hir", "--std", std_dir.as_str()];
+    args.extend(files.iter().map(String::as_str));
+    let stack = if cfg!(unix) { Some(1024) } else { None };
+    let (code, n) = n_args_on_stack(&dir, stack, &args);
+    assert_eq!(code, Some(0), "N's `selfhost hir` on a {stack:?} KB stack");
+    if let Some(m) = first_difference(&s0.stdout, &n) {
+        panic!(
+            "{}",
+            report(&m, hir_in(Mode::Check), &dir, &nova(), selfhost())
+        );
+    }
+}
+
 /// The type fuzzer's seeds (5.3a): its own, past the name fuzzer's.
 const TYPE_SEEDS: [u64; 3] = [401, 402, 403];
 const TYPE_COUNT: usize = 40;
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index 3681b00d..c2c35a8c 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -43,6 +43,7 @@ import resolution_dump
 import resolver
 import resolver_tests
 import token
+import typeck_tests
 
 async fn main() {
     let argv = args()
diff --git a/selfhost/src/typeck_tests.nova b/selfhost/src/typeck_tests.nova
new file mode 100644
index 00000000..8ada8463
--- /dev/null
+++ b/selfhost/src/typeck_tests.nova
@@ -0,0 +1,359 @@
+// N's own tests of the type checker's item passes' edges (5.3a): HIR's
+// logic (equality, substitution, matching an impl, overlap, the
+// normalization limit), the generic-parameter checks, arities, impl
+// conformance, cyclic bindings, coherence's labels, a method's signature
+// against its trait's, the fixes of an unknown type and where an import's
+// edit goes. Each expected value is what S0's type checker gives for the
+// same source (its tests in crates/nova-typeck, or `nova check --dump-hir`,
+// whose std adds nothing to these); the differential compares every other
+// input, std's items included, which these leave out.
+//
+// They live in `src/`, imported by `main.nova`, for the reason
+// lexer_tests.nova gives.
+
+import ast
+import builtins
+import diagnostic
+import fixes
+import hir
+import lexer
+import loader
+import parser
+import resolver
+import token
+import typeck
+
+// The program of `sources`, each `(name, source)` a module in that order,
+// resolved natively with no std module, each module importing the others
+// by name as its imports say and able to import every other, then its
+// items checked.
+fn check_sources(sources: [(String, String)]) -> Checker {
+    let mut names: Vec<String> = Vec::new()
+    for (name, _) in sources.iter() { names.push(name) }
+    let mut modules: Vec<ModuleSource> = Vec::new()
+    let mut labels: Vec<String> = Vec::new()
+    let mut texts: Vec<Option<String>> = Vec::new()
+    let mut i = 0
+    for (name, source) in sources.iter() {
+        let (tokens, _) = lex(source)
+        let (file, errors) = parse(tokens)
+        if !errors.is_empty() { panic("a test source does not parse") }
+        let mut m = module_source(name, name, file)
+        m.text = Some(source)
+        for item in file.items.iter() {
+            match item.value {
+                ItemImport(imp) => {
+                    let first = imp.path.value.segments.at(0).value
+                    let mut j = 0
+                    while j < names.len() {
+                        if names.at(j) == first {
+                            let _ = m.imports.insert(first, ImportTargetModule(j))
+                        }
+                        j += 1
+                    }
+                }
+                _ => ()
+            }
+        }
+        let mut j = 0
+        while j < names.len() {
+            if j != i { m.importable.push((names.at(j), j)) }
+            j += 1
+        }
+        modules.push(m)
+        labels.push(name)
+        texts.push(Some(source))
+        i += 1
+    }
+    let std_set = StdSet { modules: Vec::new(), natives: Vec::new(), builtins: builtin_table() }
+    let r = resolve_program_for(CompileTargetNative, modules, std_set)
+    if !r.diagnostics.is_empty() {
+        panic("a test source does not resolve: ${r.diagnostics.at(0).message}")
+    }
+    check_items(r.items, r.definitions, labels, texts)
+}
+
+// The texts, one to a line, to compare.
+fn lines(texts: [String]) -> String { "\n".join(texts) }
+
+// Each diagnostic's code and message, in order.
+fn messages(c: Checker) -> String {
+    let mut out: Vec<String> = Vec::new()
+    for d in c.diagnostics.iter() { out.push("${d.code} ${d.message}") }
+    lines(out.to_array())
+}
+
+fn tys(list: [Ty]) -> Vec<Ty> { Vec::from_array(list) }
+
+@test
+fn types_compare_substitute_and_match_by_structure() {
+    let pair = TyRecord(4, tys([TyParam(0), TyArray(TyParam(1))]))
+    assert(ty_eq(pair, TyRecord(4, tys([TyParam(0), TyArray(TyParam(1))]))), "it holds")
+    assert(!ty_eq(pair, TySum(4, tys([TyParam(0), TyArray(TyParam(1))]))), "it holds")
+    assert(!ty_eq(TyFn(tys([TyInt]), TyUnit), TyFn(tys([]), TyUnit)), "it holds")
+    // `Ty::subst`: a parameter past the arguments is the error type.
+    let s = subst(pair, tys([TyBool]))
+    assert(ty_eq(s, TyRecord(4, tys([TyBool, TyArray(TyError)]))), "it holds")
+    // `ImplInfo::match_args`: a hole the self type never meets is the error
+    // type; a hole met twice must match the same type both times.
+    let imp = ImplInfo {
+        trait_id: -1,
+        self_head: TyHeadRecord(4),
+        self_ty: TyRecord(4, tys([TyParam(1), TyParam(1)])),
+        generics: 3,
+        bounds: Vec::new(),
+        methods: Vec::new(),
+        assoc_bindings: Vec::new(),
+    }
+    match match_args(imp, TyRecord(4, tys([TyInt, TyInt]))) {
+        Some(args) => assert(tys_eq(args, tys([TyError, TyInt, TyError])), "it holds")
+        None => assert(false, "the impl matches")
+    }
+    assert(match_args(imp, TyRecord(4, tys([TyInt, TyBool]))).is_none(), "it holds")
+    assert(mentions_param(TyAssoc(TyParam(2), 9), 2), "it holds")
+    assert(has_assoc(TyFuture(TyAssoc(TyParam(0), 9))), "it holds")
+    assert(!has_assoc(TyArray(TyParam(0))), "it holds")
+    assert(has_error(TyTuple(tys([TyInt, TyError]))), "it holds")
+}
+
+@test
+fn self_types_overlap_when_some_instance_is_both() {
+    // `[T]` and `[Int]`; `W<T, Int>` and `W<Bool, U>`; not `W<Int>` and
+    // `W<Bool>`; `W<T, T>` and `W<Int, Bool>` share no instance.
+    assert(self_types_overlap(TyArray(TyParam(0)), 1, TyArray(TyInt), 0), "it holds")
+    let w = |a: Ty, b: Ty| TyRecord(7, tys([a, b]))
+    assert(self_types_overlap(w(TyParam(0), TyInt), 1, w(TyBool, TyParam(0)), 1), "it holds")
+    assert(
+        !self_types_overlap(TyRecord(7, tys([TyInt])), 0, TyRecord(7, tys([TyBool])), 0),
+        "it holds",
+    )
+    assert(!self_types_overlap(w(TyParam(0), TyParam(0)), 1, w(TyInt, TyBool), 0), "it holds")
+    // A projection overlaps anything but a projection onto another
+    // associated type.
+    assert(self_types_overlap(TyAssoc(TyParam(0), 3), 1, TyInt, 0), "it holds")
+    assert(self_types_overlap(TyAssoc(TyParam(0), 3), 1, TyAssoc(TyInt, 4), 0), "it holds")
+}
+
+@test
+fn normalize_follows_a_chain_of_bindings_to_its_depth_limit() {
+    // `type A(k) = Self::A(k+1)`, the last bound to `Int`: 63 links resolve,
+    // 64 meet `NORMALIZE_DEPTH_LIMIT`.
+    let on = TyRecord(1, Vec::new())
+    for links in [63, 64].iter() {
+        let mut bindings: Vec<(Int, Ty)> = Vec::new()
+        for k in 0..links { bindings.push((100 + k, TyAssoc(on, 101 + k))) }
+        bindings.push((100 + links, TyInt))
+        let imp = ImplInfo {
+            trait_id: 2,
+            self_head: TyHeadRecord(1),
+            self_ty: on,
+            generics: 0,
+            bounds: Vec::new(),
+            methods: Vec::new(),
+            assoc_bindings: bindings,
+        }
+        let mut impls: Vec<ImplInfo> = Vec::new()
+        impls.push(imp)
+        match normalize_ty(TyArray(TyAssoc(on, 100)), impls) {
+            Ok(t) => {
+                assert_eq(links, 63)
+                assert(ty_eq(t, TyArray(TyInt)), "it holds")
+            }
+            Err(NormalizeLimitDepth) => assert_eq(links, 64)
+            Err(NormalizeLimitSteps) => assert(false, "the depth limit comes first")
+        }
+    }
+}
+
+@test
+fn generic_parameters_named_self_or_twice_are_reported_by_pass() {
+    // E0076 comes from pass 2, before every other; a record's duplicate
+    // from pass 3, a function's from pass 5.
+    let c = check_sources(
+        [
+            (
+                "main",
+                "fn f<T, T>(x: T) {}\nrecord R<U, U> { x: Int }\nfn g<Self>() {}\nfn main() {}\n",
+            ),
+        ],
+    )
+    assert_eq(
+        messages(c),
+        lines(
+            [
+                "E0076 a generic parameter may not be named `Self`: in a trait `Self` is the implementing type, and in an impl it is that impl's self type",
+                "E0403 the name `U` is already used for a generic parameter of this type",
+                "E0403 the name `T` is already used for a generic parameter of this function",
+            ],
+        ),
+    )
+    assert_eq(c.diagnostics.at(0).labels.at(0).span.start, 51)
+}
+
+@test
+fn a_types_arguments_are_counted() {
+    let c = check_sources(
+        [
+            (
+                "main",
+                "record P<T> { x: T }\nfn f(p: P, q: Int<Bool>, r: Future, s: P<Int>) {}\nfn main() {}\n",
+            ),
+        ],
+    )
+    assert_eq(
+        messages(c),
+        lines(
+            [
+                "E0012 type `P` expects 1 type argument(s), found 0",
+                "E0012 `Int` takes no type arguments",
+                "E0012 `Future` takes exactly one type argument, found 0",
+            ],
+        ),
+    )
+    let sig = c.sigs.at(1).unwrap()
+    assert(ty_eq(sig.params.at(1), TyInt), "it holds")
+    assert(ty_eq(sig.params.at(3), TyRecord(0, tys([TyInt]))), "it holds")
+}
+
+@test
+fn an_impl_missing_items_names_each_kind_and_a_foreign_one() {
+    let c = check_sources(
+        [
+            (
+                "main",
+                "trait Tr {\n    type Item\n    fn a(self) -> Int\n    fn b(self) -> Int { 1 }\n}\nrecord R { x: Int }\nimpl Tr for R {\n    type Other = Int\n    fn c(self) {}\n}\nfn main() {}\n",
+            ),
+        ],
+    )
+    assert_eq(
+        messages(c),
+        lines(
+            [
+                "E0071 method `c` is not a member of trait `Tr`",
+                "E0071 associated type `Other` is not a member of trait `Tr`",
+                "E0070 impl of trait `Tr` is missing method(s): `a` and associated type(s): `Item`",
+            ],
+        ),
+    )
+}
+
+@test
+fn a_binding_in_terms_of_itself_is_e0077_and_poisoned() {
+    let c = check_sources(
+        [
+            (
+                "main",
+                "trait It {\n    type A\n    type B\n}\nrecord W { x: Int }\nimpl It for W {\n    type A = [Self::B]\n    type B = Self::A\n}\nfn main() {}\n",
+            ),
+        ],
+    )
+    assert_eq(
+        messages(c),
+        lines(
+            [
+                "E0077 the associated type `A` is defined in terms of itself",
+                "E0077 the associated type `B` is defined in terms of itself",
+            ],
+        ),
+    )
+    let imp = c.impls.at(0)
+    assert_eq(imp.assoc_bindings.len(), 2)
+    let (_, a) = imp.assoc_bindings.at(0)
+    assert(ty_eq(a, TyError), "it holds")
+}
+
+@test
+fn overlapping_inherent_impls_are_labelled_at_both() {
+    let c = check_sources(
+        [
+            (
+                "main",
+                "record W<T> { x: T }\nimpl<T> W<T> {\n    fn get(self) -> Int { 1 }\n}\nimpl W<Int> {\n    fn get(self) -> Int { 2 }\n}\nfn main() {}\n",
+            ),
+        ],
+    )
+    assert_eq(messages(c), "E0074 method `get` is defined by multiple overlapping inherent impls")
+    let labels = c.diagnostics.at(0).labels
+    assert_eq(labels.len(), 2)
+    assert(labels.at(0).primary, "it holds")
+    assert_eq(labels.at(0).span.start, 73)
+    assert(!labels.at(1).primary, "it holds")
+    assert_eq(labels.at(1).message, "`get` is also defined by this impl")
+    assert_eq(labels.at(1).span.start, 29)
+}
+
+@test
+fn a_methods_signature_is_compared_with_its_traits() {
+    let c = check_sources(
+        [
+            (
+                "main",
+                "trait Tr {\n    fn m(self, x: Int) -> Self\n    fn n(mut self)\n    fn o<U>(self, u: U)\n}\nrecord R { x: Int }\nimpl Tr for R {\n    fn m(self, x: Bool) -> Int { 1 }\n    fn n(self) {}\n    fn o(self, u: Int) {}\n}\nfn main() {}\n",
+            ),
+        ],
+    )
+    assert_eq(
+        messages(c),
+        lines(
+            [
+                "E0072 parameter 1 of method `m` has type `Bool` but trait `Tr` declares `Int`",
+                "E0072 method `m` returns `Int` but trait `Tr` declares `R`",
+                "E0072 method `n` has a plain `self` receiver but trait `Tr` declares a `mut self` receiver",
+                "E0072 method `o` has 0 generic parameter(s) but trait `Tr` declares 1",
+            ],
+        ),
+    )
+}
+
+@test
+fn an_unknown_type_offers_an_import_or_the_nearest_name() {
+    let c = check_sources(
+        [
+            ("main", "// a comment\n\nfn f(s: Shape) {}\nfn g(x: Boool) {}\nfn main() {}\n"),
+            ("shapes", "pub type Shape = | Dot\n"),
+        ],
+    )
+    assert_eq(
+        messages(c),
+        lines(["E0001 cannot find type `Shape`", "E0001 cannot find type `Boool`"]),
+    )
+    let fix = c.diagnostics.at(0).fixes.at(0)
+    assert_eq(fix.title, "import `Shape` from `shapes`")
+    assert_eq(fix.edits.at(0).file, "main")
+    assert_eq(fix.edits.at(0).span.start, 14)
+    assert_eq(fix.edits.at(0).text, "import shapes::{Shape}\n\n")
+    assert_eq(c.diagnostics.at(0).fixes.len(), 1)
+    let change = c.diagnostics.at(1).fixes.at(0)
+    assert_eq(change.title, "change `Boool` to `Bool`")
+    assert_eq(change.edits.at(0).text, "Bool")
+}
+
+@test
+fn an_imports_edit_goes_after_the_last_import_or_above_the_comments() {
+    // `line_after`: past spaces and a whole block comment, in the text's
+    // own line ending, and at the end of a last line with no break.
+    let (at, text) = line_after("import a\r\nfn f() {}\r\n", 8, "import b::{x}")
+    assert_eq(at, 10)
+    assert_eq(text, "import b::{x}\r\n")
+    let (at, text) = line_after("import a /* one\ntwo */ x\nfn f() {}", 8, "import b::{x}")
+    assert_eq(at, 25)
+    assert_eq(text, "import b::{x}\n")
+    let (at, text) = line_after("import a", 8, "import b::{x}")
+    assert_eq(at, 8)
+    assert_eq(text, "\nimport b::{x}")
+    // `line_before_item`: above the comments directly over the item, below
+    // one a blank line separates from it.
+    let source = "// apart\n\n// over\n/* also */\nfn f() {}\n"
+    let (at, text) = line_before_item(source, 30, "import b::{x}")
+    assert_eq(at, 10)
+    assert_eq(text, "import b::{x}\n\n")
+}
+
+@test
+fn an_impls_type_is_shown_with_its_own_parameter_names() {
+    let c = check_sources([("main", "record W<T> { x: T }\nfn main() {}\n")])
+    let ty = TyRecord(0, tys([TyArray(TyParam(0)), TyParam(1), TyVar(3)]))
+    assert_eq(display_ty_named(ty, c.defs, Vec::from_array(["A"])), "W<[A], T1, _>")
+    assert_eq(display_ty(ty, c.defs), "W<[T0], T1, ?3>")
+    assert_eq(display_ty(TyTuple(tys([TyInt])), c.defs), "(Int,)")
+}
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/.github/PULL_REQUEST_TEMPLATE.md b/.github/PULL_REQUEST_TEMPLATE.md
index baaa5271..4f1da6c2 100644
--- a/.github/PULL_REQUEST_TEMPLATE.md
+++ b/.github/PULL_REQUEST_TEMPLATE.md
@@ -9,8 +9,10 @@
 ## Testing
 
 - [ ] `cargo test --workspace` passes
+- [ ] `cargo test --workspace --test selfhost` passes (the self-hosting differential, which the line above skips)
 - [ ] `cargo fmt --all -- --check` passes
 - [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
+- [ ] `cargo clippy --all-features --test selfhost -- -D warnings` passes (`--all-targets` skips that target)
 - [ ] New tests added for new functionality
 - [ ] CHANGELOG.md updated
 
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index a4871813..ba86bff3 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -67,7 +67,9 @@ jobs:
         run: cargo build --locked --workspace
       # On Linux, `NOVA_REQUIRE_CLANG=1` makes `release_golden_suite` run, and
       # fail if `clang-18` cannot; elsewhere it skips, and `NOVA_CLANG` is
-      # the `clang` nova uses anyway.
+      # the `clang` nova uses anyway. `tests/selfhost.rs` is not among what
+      # this runs: its target says `test = false`, and the Selfhost job
+      # below runs it.
       - name: cargo test
         env:
           NOVA_REQUIRE_NODE: "1"
@@ -118,6 +120,73 @@ jobs:
         continue-on-error: true
         run: cargo test --locked --workspace --all-features --no-fail-fast -- --ignored
 
+  # The self-hosting differential, `crates/nova-cli/tests/selfhost.rs`
+  # (ADR 0051 to ADR 0055): N, the Nova package in `selfhost/`, built by
+  # `nova build`, must print what S0 prints, byte for byte, on std, the
+  # corpus, the cases and the fuzzers' inputs. It ran in the Test job until
+  # 5.3a, when N's item stage took it, in run 42201484370, to 70.80 s on
+  # Linux, 86.85 s on macOS and 92.54 s on Windows, in Test jobs of 525,
+  # 485 and 751 s; Windows had passed the 70 s line 5.2b set. Of the
+  # options measured, the user chose, on 2026-11-06, this job and the
+  # release runtime, and no coverage cut: no fewer roots, no fewer fuzzer
+  # inputs (ADR 0055 §15).
+  #
+  # The release runtime: N's Cranelift code calls the runtime for every
+  # allocation, string operation and collection, and with the debug
+  # runtime over a third of its time went to the collector (ADR 0055
+  # §13). `NOVA_EMBED_RUNTIME=1` makes nova-cli's build script build the
+  # runtime with `--release` (a nested cargo, into its own `OUT_DIR`) and
+  # embed it in this debug `nova`, which links it into every program it
+  # builds, N included; locally that halved the file's time (33.7 s to
+  # 17.3 s, 108 s to 47 s of CPU) for about 18 s of build. `nova build
+  # --release` would not do it: that is LLVM through `clang`, with the
+  # same runtime. The variable is set for this job only: in the Test job
+  # it would move `run_tests` and every other test that links a program
+  # onto the release runtime. Swatinem/rust-cache keys its cache by the
+  # job's id, so this job's `target/`, which holds the embedded runtime,
+  # is never the Test job's; it keeps none of the workspace's own build
+  # outputs, nova-cli's build script's among them, so the nested build
+  # runs on every run.
+  #
+  # What this job does not cover: N on the debug runtime, except for its
+  # own `@test`s, which the last step runs with `NOVA_RUNTIME_LIB` naming
+  # the debug runtime `cargo build` leaves beside `nova` (that variable
+  # outranks the embedded runtime): the runtime's debug assertions and
+  # overflow checks still run under N, and every other test that links a
+  # program still runs on the debug runtime in the Test job. The two 1 MB
+  # stack tests, of N's parser and of its item passes, run on release
+  # frames; N's depth there is the debug runtime's within 4 KB (ADR 0055
+  # §12). Clippy lints the file in its
+  # own job, by name, since `--all-targets` skips a `test = false` target.
+  selfhost:
+    name: Selfhost (${{ matrix.os }})
+    runs-on: ${{ matrix.os }}
+    strategy:
+      fail-fast: false
+      matrix:
+        os: [ubuntu-24.04, windows-latest, macos-latest]
+        include:
+          - os: ubuntu-24.04
+            runtime_lib: libnova_runtime.a
+          - os: macos-latest
+            runtime_lib: libnova_runtime.a
+          - os: windows-latest
+            runtime_lib: nova_runtime.lib
+    env:
+      NOVA_EMBED_RUNTIME: "1"
+    steps:
+      - uses: actions/checkout@v4
+      - uses: dtolnay/rust-toolchain@stable
+      - uses: Swatinem/rust-cache@v2
+      - name: cargo build (a release runtime embedded in nova)
+        run: cargo build --locked --workspace
+      - name: cargo test --test selfhost (N on the release runtime)
+        run: cargo test --locked --workspace --all-features --no-fail-fast --test selfhost
+      - name: N's own tests on the debug runtime
+        env:
+          NOVA_RUNTIME_LIB: ${{ github.workspace }}/target/debug/${{ matrix.runtime_lib }}
+        run: cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --exact the_selfhost_tests_pass
+
   # 4.2's and 4.3's gates in a real browser (spec
   # docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md
   # §10.3, and 4.3's §9.4): the counters `cargo test` drives under the fake
@@ -269,6 +338,11 @@ jobs:
           components: clippy
       - uses: Swatinem/rust-cache@v2
       - run: cargo clippy --locked --all-targets --all-features -- -D warnings
+      # `--all-targets` takes only the targets `cargo test` runs, so it
+      # skips `tests/selfhost.rs`, whose target says `test = false` (the
+      # Selfhost job runs it); naming it lints it. Passing both flags in
+      # one command would not: `--all-targets` wins.
+      - run: cargo clippy --locked --all-features --test selfhost -- -D warnings
 
   msrv:
     name: MSRV (1.78)
diff --git a/CONTRIBUTING.md b/CONTRIBUTING.md
index 3822bc85..74a7bb39 100644
--- a/CONTRIBUTING.md
+++ b/CONTRIBUTING.md
@@ -8,6 +8,10 @@ Thank you for your interest in contributing!
 2. Clone the repo
 3. `cargo build --workspace`
 4. `cargo test --workspace`
+5. `cargo test --workspace --test selfhost`: the self-hosting
+   differential, which `cargo test --workspace` skips (its target says
+   `test = false`) and CI runs in a job of its own
+   (docs/adr/0055-the-self-hosted-item-collection-and-the-hir-dump.md §15)
 
 ## Commit Style
 
@@ -15,7 +19,9 @@ Use conventional commits: `feat:`, `fix:`, `chore:`, `docs:`, `refactor:`, `test
 
 ## Code Style
 
-- `cargo fmt` and `cargo clippy --all-targets --all-features -- -D warnings` must pass
+- `cargo fmt` and `cargo clippy --all-targets --all-features -- -D warnings` must pass, and
+  `cargo clippy --all-features --test selfhost -- -D warnings`, since
+  `--all-targets` skips that target
 - No `unwrap()` outside tests
 - All public items have rustdoc
 
diff --git a/agent.md b/agent.md
index 971a1605..7c27aab6 100644
--- a/agent.md
+++ b/agent.md
@@ -45,11 +45,14 @@ See `nova-spec/00-MASTER-SPEC.md` §3 for subsequent phase order.
 # Build everything
 cargo build --workspace
 
-# Run all tests
+# Run all tests: the second runs the self-hosting differential, which
+# the first skips (`test = false`; CI runs it in a job of its own, ADR 0055 §15)
 cargo test --workspace
+cargo test --workspace --test selfhost
 
-# Lint (must pass — CI enforces this)
+# Lint (must pass — CI enforces this); `--all-targets` skips the selfhost target
 cargo clippy --all-targets --all-features -- -D warnings
+cargo clippy --all-features --test selfhost -- -D warnings
 
 # Format check
 cargo fmt --check
diff --git a/crates/nova-cli/Cargo.toml b/crates/nova-cli/Cargo.toml
index 703c9cbf..eb9dd375 100644
--- a/crates/nova-cli/Cargo.toml
+++ b/crates/nova-cli/Cargo.toml
@@ -19,6 +19,16 @@ path = "src/main.rs"
 # workspace's only benchmark is `nova-lexer/benches/lex.rs`.
 bench = false
 
+# The self-hosting differential (ADR 0051 to 0055) runs in a CI job of its
+# own on each system, N linked against a release runtime, and not in the
+# Test job (ADR 0055 §15). `test = false` keeps it out of
+# `cargo test --workspace`; `cargo test --workspace --test selfhost` runs it,
+# here as in that job. Its source stays where cargo finds it.
+[[test]]
+name = "selfhost"
+path = "tests/selfhost.rs"
+test = false
+
 [dependencies]
 nova-diagnostics = { workspace = true }
 nova-lexer = { workspace = true }
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 1f1625a9..f2e163e2 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -60,6 +60,11 @@
 //! directory, so the source tree gets no `target/`, and writes its inputs to
 //! fixed directories there too. `nova build`, `nova test` and each side on
 //! each batch are killed if they run past [`CHILD_LIMIT`].
+//!
+//! The target says `test = false`: `cargo test --workspace` skips this
+//! file and `cargo test --workspace --test selfhost` runs it. CI runs it in
+//! a job of its own on each system, N linked against a release runtime,
+//! and N's own tests once more on the debug runtime (ADR 0055 §15).
 
 #[path = "selfhost/compare.rs"]
 mod compare;
diff --git a/selfhost/src/fixes.nova b/selfhost/src/fixes.nova
new file mode 100644
index 00000000..b83cf7bc
--- /dev/null
+++ b/selfhost/src/fixes.nova
@@ -0,0 +1,346 @@
+// The fixes the item passes attach to an unknown type's or trait's E0001:
+// a port of crates/nova-typeck/src/check/fixes.rs's `push_fix`,
+// `name_fixes`, `import_fix`, `import_edit`, `fits`, `did_you_mean` and
+// `offers` for a type and a trait, and of nova-diagnostics' lines.rs
+// (`ending`, `line_start`, `next_line_start`, `line_after`,
+// `line_before_item`, `comments_before`), which place the import fix's
+// edit by the source text, function by function (spec
+// docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md
+// §3.2, ADR 0055). The rest of fixes.rs, for the bodies' errors, comes in
+// 5.3b.
+
+import ast
+import diagnostic
+import hir
+import resolver
+import token
+import typeck
+
+/// What an unknown name must be at its site (`Need`): the item passes
+/// meet a type and a trait.
+pub type Need = | NeedType | NeedTrait
+
+/// The primitive types' names, which "did you mean" offers for a type
+/// (`PRIMITIVES`).
+const PRIMITIVES: [String] = ["Bool", "Bytes", "Char", "Float", "Future", "Int", "String"]
+
+impl Checker {
+    /// `fix`, if there is one, added to the diagnostic just pushed, unless
+    /// the module being checked is not this project's (`push_fix`).
+    pub fn push_fix(mut self, fix: Option<Fix>) {
+        if !self.owned(self.cur_module) { return }
+        match fix {
+            Some(f) => {
+                if !self.diagnostics.is_empty() {
+                    let mut d = self.diagnostics.last()
+                    d.fixes.push(f)
+                }
+            }
+            None => ()
+        }
+    }
+
+    /// The import and "did you mean" fixes for the unknown `name`, added to
+    /// the diagnostic just pushed (`name_fixes`); `nearby` holds the type
+    /// parameters in scope.
+    pub fn name_fixes(mut self, name: Spanned<String>, need: Need, nearby: Vec<String>) {
+        let import_fix = self.import_fix(name, need)
+        let change = self.did_you_mean(name, need, nearby)
+        self.push_fix(import_fix)
+        self.push_fix(change)
+    }
+
+    // "import `x` from `m`": the one module this one could import that
+    // exports an `x` of the kind `need` names, when importing it binds `x`
+    // nowhere this module already has it (`import_fix`).
+    fn import_fix(self, name: Spanned<String>, need: Need) -> Option<Fix> {
+        let x = name.value
+        let here = self.cur_module
+        if here < 0 || here >= self.defs.importable.len() { return None }
+        let mut found: Vec<(String, Int)> = Vec::new()
+        for (imp, m) in self.defs.importable.at(here).iter() {
+            if self.fits(m, x, need) { found.push((imp, m)) }
+        }
+        if found.len() != 1 { return None }
+        let (imp, owner) = found.at(0)
+        if owner < 0 || owner >= self.defs.exports.len() { return None }
+        let e = self.defs.exports.at(owner)
+        if here >= self.defs.modules.len() { return None }
+        let scope = self.defs.modules.at(here)
+        let clash = (e.values.contains_key(x) && self.bound_outside_std_value(scope, x))
+            || (e.types.contains_key(x) && self.bound_outside_std_type(scope, x))
+            || (e.traits.contains_key(x) && self.bound_outside_std_trait(scope, x))
+        if clash { return None }
+        let edit = self.import_edit(imp, x)?
+        let mut edits: Vec<Edit> = Vec::new()
+        edits.push(edit)
+        Some(Fix { title: "import `${x}` from `${imp}`", edits: edits })
+    }
+
+    // Whether module `m` exports an `x` of the kind `need` names (`fits`).
+    fn fits(self, m: Int, x: String, need: Need) -> Bool {
+        if m < 0 || m >= self.defs.exports.len() { return false }
+        let e = self.defs.exports.at(m)
+        match need {
+            NeedType => {
+                match e.types.get(x) {
+                    Some(id) => {
+                        match self.kind_of(id) {
+                            DefKindRecord(_) => true
+                            DefKindSum(_, _) => true
+                            _ => false
+                        }
+                    }
+                    None => false
+                }
+            }
+            NeedTrait => e.traits.contains_key(x)
+        }
+    }
+
+    // What `scope` binds to `x` that no std module exports under it, in
+    // each namespace (`bound_outside_std`).
+    fn bound_outside_std_value(self, scope: ModuleScope, x: String) -> Bool {
+        let r = match scope.values.get(x) {
+            Some(r) => r
+            None => return false
+        }
+        let mut m = self.defs.std_start
+        while m < self.defs.exports.len() {
+            match self.defs.exports.at(m).values.get(x) {
+                Some(s) => if res_eq(s, r) { return false }
+                None => ()
+            }
+            m += 1
+        }
+        true
+    }
+
+    fn bound_outside_std_type(self, scope: ModuleScope, x: String) -> Bool {
+        let d = match scope.types.get(x) {
+            Some(d) => d
+            None => return false
+        }
+        let mut m = self.defs.std_start
+        while m < self.defs.exports.len() {
+            match self.defs.exports.at(m).types.get(x) {
+                Some(s) => if s == d { return false }
+                None => ()
+            }
+            m += 1
+        }
+        true
+    }
+
+    fn bound_outside_std_trait(self, scope: ModuleScope, x: String) -> Bool {
+        let d = match scope.traits.get(x) {
+            Some(d) => d
+            None => return false
+        }
+        let mut m = self.defs.std_start
+        while m < self.defs.exports.len() {
+            match self.defs.exports.at(m).traits.get(x) {
+                Some(s) => if s == d { return false }
+                None => ()
+            }
+            m += 1
+        }
+        true
+    }
+
+    // Where `x` goes in this module's file: into an `import m::{…}`, or a
+    // new line after the last import, or above the first item
+    // (`import_edit`).
+    fn import_edit(self, imp: String, x: String) -> Option<Edit> {
+        let here = self.cur_module
+        let file = self.cur_file
+        let mut items: Vec<Spanned<Item>> = Vec::new()
+        for i in 0..self.items.len() {
+            if self.module_of(i) == here { items.push(self.items.at(i)) }
+        }
+        let mut k = items.len()
+        while k > 0 {
+            k -= 1
+            match items.at(k).value {
+                ItemImport(decl) => {
+                    match decl.kind {
+                        ImportKindList(names) => {
+                            let segs = decl.path.value.segments
+                            if segs.len() == 1 && segs.at(0).value == imp && !names.is_empty() {
+                                return Some(Edit::insert(names.last().span.end, file, ", ${x}"))
+                            }
+                        }
+                        _ => ()
+                    }
+                }
+                _ => ()
+            }
+        }
+        let text = if here >= 0 && here < self.texts.len() {
+            self.texts.at(here)?
+        } else {
+            return None
+        }
+        let line = "import ${imp}::{${x}}"
+        let mut last_import = -1
+        for it in items.iter() {
+            match it.value {
+                ItemImport(_) => last_import = it.span.end
+                _ => ()
+            }
+        }
+        let (at, inserted) = if last_import >= 0 {
+            line_after(text, last_import, line)
+        } else if !items.is_empty() {
+            line_before_item(text, items.at(0).span.start, line)
+        } else {
+            (text.byte_len(), "${line}${ending(text)}")
+        }
+        Some(Edit::insert(at, file, inserted))
+    }
+
+    // "change `x` to `y`", `y` a name of the kind `need` says, in scope
+    // here or in `nearby` (`did_you_mean`).
+    fn did_you_mean(self, name: Spanned<String>, need: Need, nearby: Vec<String>) -> Option<Fix> {
+        let mut candidates: Vec<String> = Vec::new()
+        for n in nearby.iter() { candidates.push(n) }
+        if self.cur_module >= 0 && self.cur_module < self.defs.modules.len() {
+            let scope = self.defs.modules.at(self.cur_module)
+            match need {
+                NeedType => for n in scope.types.keys().iter() { candidates.push(n) }
+                NeedTrait => for n in scope.traits.keys().iter() { candidates.push(n) }
+            }
+        }
+        match need {
+            NeedType => for p in PRIMITIVES.iter() { candidates.push(p) }
+            NeedTrait => ()
+        }
+        change_to(self.cur_file, name, candidates.to_array())
+    }
+}
+
+/// `a == b` on two resolutions (`Res`'s derived `PartialEq`).
+pub fn res_eq(a: Res, b: Res) -> Bool {
+    match (a, b) {
+        (ResDef(x), ResDef(y)) => x == y
+        (ResVariant(s1, i1), ResVariant(s2, i2)) => s1 == s2 && i1 == i2
+        (ResBuiltin(x), ResBuiltin(y)) => x == y
+        _ => false
+    }
+}
+
+// ---------------------------------------------------------------------------
+// lines.rs
+// ---------------------------------------------------------------------------
+
+/// The text's line ending: `\r\n` when it holds one (`ending`).
+pub fn ending(text: String) -> String { if text.contains("\r\n") { "\r\n" } else { "\n" } }
+
+// Where the line holding byte `at` starts (`line_start`).
+fn line_start(text: String, at: Int) -> Int {
+    let mut i = at
+    while i > 0 {
+        if text.byte_at(i - 1) == 10 { return i }
+        i -= 1
+    }
+    0
+}
+
+// Where the line after the one holding byte `at` starts, or the text's
+// end (`next_line_start`).
+fn next_line_start(text: String, at: Int) -> Int {
+    let n = text.byte_len()
+    let mut i = at
+    while i < n {
+        if text.byte_at(i) == 10 { return i + 1 }
+        i += 1
+    }
+    n
+}
+
+// Whether the bytes of `text` from `at` begin with `/` and `*`.
+fn block_comment_at(text: String, at: Int) -> Bool {
+    at + 1 < text.byte_len() && text.byte_at(at) == 47 && text.byte_at(at + 1) == 42
+}
+
+// Past the `*/` that closes a block comment whose body starts at `from`,
+// or the text's end.
+fn past_block_end(text: String, from: Int) -> Int {
+    let n = text.byte_len()
+    let mut i = from
+    while i + 1 < n {
+        if text.byte_at(i) == 42 && text.byte_at(i + 1) == 47 { return i + 2 }
+        i += 1
+    }
+    n
+}
+
+/// The edit adding `line` on a line of its own after the line holding byte
+/// `at`: where it goes, and what it inserts, in the text's own ending
+/// (`line_after`).
+pub fn line_after(text: String, at: Int, line: String) -> (Int, String) {
+    let nl = ending(text)
+    let n = text.byte_len()
+    let mut i = at
+    while true {
+        while i < n && (text.byte_at(i) == 32 || text.byte_at(i) == 9) { i += 1 }
+        if !block_comment_at(text, i) { break }
+        i = past_block_end(text, i + 2)
+    }
+    let next = next_line_start(text, i)
+    if next == n && !text.ends_with("\n") { (next, "${nl}${line}") } else { (next, "${line}${nl}") }
+}
+
+/// The edit adding `line` and a blank line before the item starting at
+/// byte `item_start`, above the comment lines directly over it
+/// (`line_before_item`).
+pub fn line_before_item(text: String, item_start: Int, line: String) -> (Int, String) {
+    let nl = ending(text)
+    let mut at = line_start(text, item_start)
+    let comments = comments_before(text, at)
+    let mut k = comments.len()
+    while k > 0 {
+        k -= 1
+        let (start, end) = comments.at(k)
+        if end > at {
+            let s = line_start(text, start)
+            if s < at { at = s }
+            continue
+        }
+        let mut breaks = 0
+        let mut j = end
+        while j < at {
+            if text.byte_at(j) == 10 { breaks += 1 }
+            j += 1
+        }
+        if breaks > 1 { break }
+        at = line_start(text, start)
+    }
+    (at, "${line}${nl}${nl}")
+}
+
+// The comments before byte `end` of `text`, which holds only comments and
+// blank space there (`comments_before`); block comments do not nest.
+fn comments_before(text: String, end: Int) -> Vec<(Int, Int)> {
+    let n = text.byte_len()
+    let mut out: Vec<(Int, Int)> = Vec::new()
+    let mut i = 0
+    while i < end {
+        let b = text.byte_at(i)
+        if b == 32 || b == 9 || b == 13 || b == 10 {
+            i += 1
+        } else if b == 47 && i + 1 < n && text.byte_at(i + 1) == 47 {
+            let mut e = i
+            while e < n && text.byte_at(e) != 10 { e += 1 }
+            out.push((i, e))
+            i = e
+        } else if b == 47 && i + 1 < n && text.byte_at(i + 1) == 42 {
+            let e = past_block_end(text, i + 2)
+            out.push((i, e))
+            i = e
+        } else {
+            break
+        }
+    }
+    out
+}
diff --git a/selfhost/src/hir.nova b/selfhost/src/hir.nova
new file mode 100644
index 00000000..e74e0cfa
--- /dev/null
+++ b/selfhost/src/hir.nova
@@ -0,0 +1,585 @@
+// HIR's types and the item tables: a port of crates/nova-hir/src/lib.rs's
+// `Ty`, `TyHead`, `TraitDef`, `TraitMethod`, `ImplInfo`, `RecordType`,
+// `RecordField`, `SumType`, `Variant`, `ExternFn` and `ExternAbi`, and of
+// its logic (`subst`, the queries, `match_pattern`, `match_args`,
+// `normalize_ty` with its limits, `self_types_overlap`), function by
+// function and with the same names (spec
+// docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md
+// §3.1, ADR 0055). The bodies' nodes (`Function`, `Expr`, `Pattern`) come
+// in 5.3b.
+//
+// The Rust's idioms, translated one way everywhere (ADR 0055 lists them):
+//
+// - A `DefId` is an `Int`, and an `Option<DefId>` an `Int`, -1 for none.
+// - `Ty` derives `PartialEq`, and a user sum has no `==` in Nova (E0013), so
+//   two types are compared by `ty_eq`, two heads by `head_eq`.
+// - A variant whose name a module of N already holds carries `Hir`
+//   (`HirVariant`, `HirField`), as one module may not import two of a name.
+// - A span carries no file in N (ADR 0052), so a span kept in a table,
+//   which a diagnostic or the dump prints with its file, is a `FileSpan`.
+
+import token
+
+/// A type (`Ty`): the 18 variants, `Var` included, which the bodies use.
+pub type Ty =
+    | TyInt
+    | TyFloat
+    | TyBool
+    | TyChar
+    | TyString
+    | TyBytes
+    | TyUnit
+    // params, ret
+    | TyFn(Vec<Ty>, Ty)
+    | TyFuture(Ty)
+    // def_id, args
+    | TySum(Int, Vec<Ty>)
+    // def_id, args
+    | TyRecord(Int, Vec<Ty>)
+    | TyArray(Ty)
+    | TyTuple(Vec<Ty>)
+    | TyParam(Int)
+    // on, assoc
+    | TyAssoc(Ty, Int)
+    | TyVar(Int)
+    | TyNever
+    | TyError
+
+/// The outermost constructor of a type an impl can be written for
+/// (`TyHead`).
+pub type TyHead =
+    | TyHeadInt
+    | TyHeadFloat
+    | TyHeadBool
+    | TyHeadChar
+    | TyHeadString
+    | TyHeadBytes
+    | TyHeadArray
+    | TyHeadSum(Int)
+    | TyHeadRecord(Int)
+
+/// A span of the file labelled `file`.
+pub record FileSpan { pub file: String, pub span: Span }
+
+/// An `extern` function's ABI (`ExternAbi`).
+pub type ExternAbi = | ExternAbiC | ExternAbiJs
+
+/// An `extern` function (`ExternFn`): its symbol, ABI, signature and the
+/// span of its name.
+pub record ExternFn {
+    pub def_id: Int,
+    pub symbol: String,
+    pub abi: ExternAbi,
+    pub params: Vec<Ty>,
+    pub ret: Ty,
+    pub span: FileSpan,
+}
+
+/// A trait (`TraitDef`).
+pub record TraitDef {
+    pub def_id: Int,
+    pub name: String,
+    pub supertraits: Vec<Int>,
+    pub methods: Vec<TraitMethod>,
+    pub assoc_types: Vec<(String, Int)>,
+}
+
+/// A trait's method (`TraitMethod`): `Self` is `TyParam(0)`, its own
+/// generics `TyParam(1..)`; `params` holds no `self`.
+pub record TraitMethod {
+    pub name: String,
+    pub params: Vec<Ty>,
+    pub ret: Ty,
+    pub has_self: Bool,
+    pub mut_self: Bool,
+    pub generics: Int,
+    pub bounds: Vec<Vec<Int>>,
+    pub default_def: Int,
+}
+
+/// An impl (`ImplInfo`); `trait_id` is -1 for an inherent impl.
+pub record ImplInfo {
+    pub trait_id: Int,
+    pub self_head: TyHead,
+    pub self_ty: Ty,
+    pub generics: Int,
+    pub bounds: Vec<Vec<Int>>,
+    pub methods: Vec<(String, Int)>,
+    pub assoc_bindings: Vec<(Int, Ty)>,
+}
+
+/// A record type (`RecordType`).
+pub record RecordType {
+    pub def_id: Int,
+    pub name: String,
+    pub generics: Int,
+    pub fields: Vec<HirField>,
+}
+
+/// A record's field (`RecordField`).
+pub record HirField { pub name: String, pub ty: Ty }
+
+/// A sum type (`SumType`).
+pub record SumType {
+    pub def_id: Int,
+    pub name: String,
+    pub generics: Int,
+    pub variants: Vec<HirVariant>,
+}
+
+/// A sum's variant (`Variant`).
+pub record HirVariant { pub name: String, pub fields: Vec<Ty> }
+
+// ---------------------------------------------------------------------------
+// Equality
+// ---------------------------------------------------------------------------
+
+/// `a == b` on two types (`Ty`'s derived `PartialEq`).
+pub fn ty_eq(a: Ty, b: Ty) -> Bool {
+    match (a, b) {
+        (TyInt, TyInt) => true
+        (TyFloat, TyFloat) => true
+        (TyBool, TyBool) => true
+        (TyChar, TyChar) => true
+        (TyString, TyString) => true
+        (TyBytes, TyBytes) => true
+        (TyUnit, TyUnit) => true
+        (TyFn(p1, r1), TyFn(p2, r2)) => tys_eq(p1, p2) && ty_eq(r1, r2)
+        (TyFuture(o1), TyFuture(o2)) => ty_eq(o1, o2)
+        (TySum(d1, a1), TySum(d2, a2)) => d1 == d2 && tys_eq(a1, a2)
+        (TyRecord(d1, a1), TyRecord(d2, a2)) => d1 == d2 && tys_eq(a1, a2)
+        (TyArray(e1), TyArray(e2)) => ty_eq(e1, e2)
+        (TyTuple(t1), TyTuple(t2)) => tys_eq(t1, t2)
+        (TyParam(i), TyParam(j)) => i == j
+        (TyAssoc(o1, x), TyAssoc(o2, y)) => x == y && ty_eq(o1, o2)
+        (TyVar(i), TyVar(j)) => i == j
+        (TyNever, TyNever) => true
+        (TyError, TyError) => true
+        _ => false
+    }
+}
+
+/// Element-wise `ty_eq` of two lists of types.
+pub fn tys_eq(a: Vec<Ty>, b: Vec<Ty>) -> Bool {
+    if a.len() != b.len() { return false }
+    let mut i = 0
+    while i < a.len() {
+        if !ty_eq(a.at(i), b.at(i)) { return false }
+        i += 1
+    }
+    true
+}
+
+/// `a == b` on two heads (`TyHead`'s derived `PartialEq`).
+pub fn head_eq(a: TyHead, b: TyHead) -> Bool {
+    match (a, b) {
+        (TyHeadInt, TyHeadInt) => true
+        (TyHeadFloat, TyHeadFloat) => true
+        (TyHeadBool, TyHeadBool) => true
+        (TyHeadChar, TyHeadChar) => true
+        (TyHeadString, TyHeadString) => true
+        (TyHeadBytes, TyHeadBytes) => true
+        (TyHeadArray, TyHeadArray) => true
+        (TyHeadSum(x), TyHeadSum(y)) => x == y
+        (TyHeadRecord(x), TyHeadRecord(y)) => x == y
+        _ => false
+    }
+}
+
+/// Whether `head` is `TyHead::Array`.
+pub fn is_array_head(head: TyHead) -> Bool {
+    match head {
+        TyHeadArray => true
+        _ => false
+    }
+}
+
+// ---------------------------------------------------------------------------
+// Ty's logic
+// ---------------------------------------------------------------------------
+
+/// `ty` with each `TyParam(i)` replaced by `args[i]`, `TyError` past
+/// `args`' end (`Ty::subst`).
+pub fn subst(ty: Ty, args: Vec<Ty>) -> Ty {
+    match ty {
+        TyParam(i) => if i >= 0 && i < args.len() { args.at(i) } else { TyError }
+        TyFn(params, ret) => TyFn(subst_all(params, args), subst(ret, args))
+        TySum(d, a) => TySum(d, subst_all(a, args))
+        TyRecord(d, a) => TyRecord(d, subst_all(a, args))
+        TyArray(e) => TyArray(subst(e, args))
+        TyTuple(es) => TyTuple(subst_all(es, args))
+        TyFuture(o) => TyFuture(subst(o, args))
+        TyAssoc(on, assoc) => TyAssoc(subst(on, args), assoc)
+        other => other
+    }
+}
+
+/// `subst` of each of `tys`.
+pub fn subst_all(tys: Vec<Ty>, args: Vec<Ty>) -> Vec<Ty> {
+    let mut out: Vec<Ty> = Vec::new()
+    for t in tys.iter() { out.push(subst(t, args)) }
+    out
+}
+
+/// Whether `ty` mentions `TyParam(idx)` (`Ty::mentions_param`).
+pub fn mentions_param(ty: Ty, idx: Int) -> Bool {
+    match ty {
+        TyParam(k) => k == idx
+        TyFn(params, ret) => {
+            for p in params.iter() { if mentions_param(p, idx) { return true } }
+            mentions_param(ret, idx)
+        }
+        TySum(_, args) => any_mentions(args, idx)
+        TyRecord(_, args) => any_mentions(args, idx)
+        TyArray(e) => mentions_param(e, idx)
+        TyTuple(es) => any_mentions(es, idx)
+        TyFuture(o) => mentions_param(o, idx)
+        TyAssoc(on, _) => mentions_param(on, idx)
+        _ => false
+    }
+}
+
+fn any_mentions(tys: Vec<Ty>, idx: Int) -> Bool {
+    for t in tys.iter() { if mentions_param(t, idx) { return true } }
+    false
+}
+
+/// Whether `ty` holds a projection anywhere (`Ty::has_assoc`).
+pub fn has_assoc(ty: Ty) -> Bool {
+    match ty {
+        TyAssoc(_, _) => true
+        TyFn(params, ret) => {
+            for p in params.iter() { if has_assoc(p) { return true } }
+            has_assoc(ret)
+        }
+        TySum(_, args) => any_assoc(args)
+        TyRecord(_, args) => any_assoc(args)
+        TyArray(e) => has_assoc(e)
+        TyTuple(es) => any_assoc(es)
+        TyFuture(o) => has_assoc(o)
+        _ => false
+    }
+}
+
+fn any_assoc(tys: Vec<Ty>) -> Bool {
+    for t in tys.iter() { if has_assoc(t) { return true } }
+    false
+}
+
+/// Whether `ty` holds the error type anywhere (`Ty::has_error`).
+pub fn has_error(ty: Ty) -> Bool {
+    match ty {
+        TyError => true
+        TyFn(params, ret) => {
+            for p in params.iter() { if has_error(p) { return true } }
+            has_error(ret)
+        }
+        TySum(_, args) => any_error(args)
+        TyRecord(_, args) => any_error(args)
+        TyArray(e) => has_error(e)
+        TyTuple(es) => any_error(es)
+        TyFuture(o) => has_error(o)
+        TyAssoc(on, _) => has_error(on)
+        _ => false
+    }
+}
+
+fn any_error(tys: Vec<Ty>) -> Bool {
+    for t in tys.iter() { if has_error(t) { return true } }
+    false
+}
+
+/// The head of a type an impl can be written for (`Ty::head`).
+pub fn head(ty: Ty) -> Option<TyHead> {
+    match ty {
+        TyInt => Some(TyHeadInt)
+        TyFloat => Some(TyHeadFloat)
+        TyBool => Some(TyHeadBool)
+        TyChar => Some(TyHeadChar)
+        TyString => Some(TyHeadString)
+        TyBytes => Some(TyHeadBytes)
+        TyArray(_) => Some(TyHeadArray)
+        TySum(d, _) => Some(TyHeadSum(d))
+        TyRecord(d, _) => Some(TyHeadRecord(d))
+        _ => None
+    }
+}
+
+/// Match `pattern`, whose `TyParam`s are holes, against `ground`, filling
+/// `out[k]` with what hole `k` matched (`Ty::match_pattern`); `out` holds
+/// `None` for a hole not met yet.
+pub fn match_pattern(pattern: Ty, ground: Ty, mut out: Vec<Option<Ty>>) -> Bool {
+    match (pattern, ground) {
+        (TyParam(k), g) => {
+            while out.len() <= k { out.push(None) }
+            match out.at(k) {
+                Some(existing) => ty_eq(existing, g)
+                None => {
+                    out.set(k, Some(g))
+                    true
+                }
+            }
+        }
+        (TyRecord(d1, a1), TyRecord(d2, a2)) => d1 == d2 && match_all(a1, a2, out)
+        (TySum(d1, a1), TySum(d2, a2)) => d1 == d2 && match_all(a1, a2, out)
+        (TyArray(e1), TyArray(e2)) => match_pattern(e1, e2, out)
+        (TyTuple(t1), TyTuple(t2)) => match_all(t1, t2, out)
+        (TyFuture(o1), TyFuture(o2)) => match_pattern(o1, o2, out)
+        (TyFn(p1, r1), TyFn(p2, r2)) => match_all(p1, p2, out) && match_pattern(r1, r2, out)
+        (TyAssoc(p, pa), TyAssoc(g, ga)) => pa == ga && match_pattern(p, g, out)
+        (a, b) => ty_eq(a, b)
+    }
+}
+
+// Equal lengths and each pair matching, stopping at the first that does
+// not, as `zip(…).all(…)` does.
+fn match_all(ps: Vec<Ty>, gs: Vec<Ty>, out: Vec<Option<Ty>>) -> Bool {
+    if ps.len() != gs.len() { return false }
+    let mut i = 0
+    while i < ps.len() {
+        if !match_pattern(ps.at(i), gs.at(i), out) { return false }
+        i += 1
+    }
+    true
+}
+
+/// The impl's type arguments that make its self type `concrete`, a hole
+/// not met `TyError` (`ImplInfo::match_args`).
+pub fn match_args(imp: ImplInfo, concrete: Ty) -> Option<Vec<Ty>> {
+    let out: Vec<Option<Ty>> = Vec::new()
+    if !match_pattern(imp.self_ty, concrete, out) { return None }
+    let mut args: Vec<Ty> = Vec::new()
+    let mut i = 0
+    while i < imp.generics {
+        let t = if i < out.len() {
+            match out.at(i) {
+                Some(t) => t
+                None => TyError
+            }
+        } else {
+            TyError
+        }
+        args.push(t)
+        i += 1
+    }
+    Some(args)
+}
+
+// ---------------------------------------------------------------------------
+// normalize_ty
+// ---------------------------------------------------------------------------
+
+/// How deep a chain of associated-type bindings may go
+/// (`NORMALIZE_DEPTH_LIMIT`).
+pub const NORMALIZE_DEPTH_LIMIT: Int = 64
+
+/// How many type nodes one normalization may visit
+/// (`NORMALIZE_STEP_LIMIT`).
+pub const NORMALIZE_STEP_LIMIT: Int = 10000
+
+/// Which limit a normalization ran out on (`NormalizeLimit`).
+pub type NormalizeLimit = | NormalizeLimitDepth | NormalizeLimitSteps
+
+// The steps a normalization has left, shared by its recursion.
+record Steps { left: Int }
+
+/// `ty` with every projection that an impl of `impls` binds replaced by
+/// the binding, its arguments substituted, again until none is left
+/// (`normalize_ty`); `Err` with the limit it ran out on, for which the
+/// caller names `ty`.
+pub fn normalize_ty(ty: Ty, impls: Vec<ImplInfo>) -> Result<Ty, NormalizeLimit> {
+    let steps = Steps { left: NORMALIZE_STEP_LIMIT }
+    normalize_within(ty, impls, NORMALIZE_DEPTH_LIMIT, steps)
+}
+
+fn normalize_within(
+    ty: Ty,
+    impls: Vec<ImplInfo>,
+    depth: Int,
+    mut steps: Steps,
+) -> Result<Ty, NormalizeLimit> {
+    if steps.left == 0 { return Err(NormalizeLimitSteps) }
+    steps.left -= 1
+    match ty {
+        TyAssoc(on, assoc) => {
+            if depth == 0 { return Err(NormalizeLimitDepth) }
+            let depth = depth - 1
+            let on = normalize_within(on, impls, depth, steps)?
+            let h = match head(on) {
+                Some(h) => h
+                None => return Ok(TyAssoc(on, assoc))
+            }
+            let mut bound: Option<Ty> = None
+            for i in impls.iter() {
+                if !head_eq(i.self_head, h) { continue }
+                let mut binding: Option<Ty> = None
+                for (d, t) in i.assoc_bindings.iter() {
+                    if d == assoc {
+                        binding = Some(t)
+                        break
+                    }
+                }
+                let b = match binding {
+                    Some(b) => b
+                    None => continue
+                }
+                match match_args(i, on) {
+                    Some(args) => {
+                        bound = Some(subst(b, args))
+                        break
+                    }
+                    None => continue
+                }
+            }
+            match bound {
+                Some(b) => normalize_within(b, impls, depth, steps)
+                None => Ok(TyAssoc(on, assoc))
+            }
+        }
+        TyFn(params, ret) => {
+            let ps = normalize_all(params, impls, depth, steps)?
+            let r = normalize_within(ret, impls, depth, steps)?
+            Ok(TyFn(ps, r))
+        }
+        TySum(d, args) => Ok(TySum(d, normalize_all(args, impls, depth, steps)?))
+        TyRecord(d, args) => Ok(TyRecord(d, normalize_all(args, impls, depth, steps)?))
+        TyArray(e) => Ok(TyArray(normalize_within(e, impls, depth, steps)?))
+        TyTuple(es) => Ok(TyTuple(normalize_all(es, impls, depth, steps)?))
+        TyFuture(o) => Ok(TyFuture(normalize_within(o, impls, depth, steps)?))
+        other => Ok(other)
+    }
+}
+
+fn normalize_all(
+    tys: Vec<Ty>,
+    impls: Vec<ImplInfo>,
+    depth: Int,
+    steps: Steps,
+) -> Result<Vec<Ty>, NormalizeLimit> {
+    let mut out: Vec<Ty> = Vec::new()
+    for t in tys.iter() { out.push(normalize_within(t, impls, depth, steps)?) }
+    Ok(out)
+}
+
+// ---------------------------------------------------------------------------
+// self_types_overlap
+// ---------------------------------------------------------------------------
+
+/// Whether two impls' self types, with `a_generics` and `b_generics`
+/// parameters, share a ground instance (`self_types_overlap`); a
+/// projection is taken to overlap anything but a projection onto another
+/// associated type.
+pub fn self_types_overlap(a: Ty, a_generics: Int, b: Ty, b_generics: Int) -> Bool {
+    match (a, b) {
+        (TyAssoc(a1, x), TyAssoc(b1, y)) => x != y
+            || self_types_overlap(a1, a_generics, b1, b_generics)
+        (TyAssoc(_, _), _) => true
+        (_, TyAssoc(_, _)) => true
+        _ => {
+            let shifted = shift_params(b, a_generics)
+            let mut s: Vec<Option<Ty>> = Vec::new()
+            let mut i = 0
+            while i < a_generics + b_generics {
+                s.push(None)
+                i += 1
+            }
+            unify_patterns(a, shifted, s)
+        }
+    }
+}
+
+fn shift_params(t: Ty, by: Int) -> Ty {
+    match t {
+        TyParam(k) => TyParam(k + by)
+        TyFn(params, ret) => TyFn(shift_all(params, by), shift_params(ret, by))
+        TySum(d, args) => TySum(d, shift_all(args, by))
+        TyRecord(d, args) => TyRecord(d, shift_all(args, by))
+        TyArray(e) => TyArray(shift_params(e, by))
+        TyTuple(es) => TyTuple(shift_all(es, by))
+        TyFuture(o) => TyFuture(shift_params(o, by))
+        other => other
+    }
+}
+
+fn shift_all(tys: Vec<Ty>, by: Int) -> Vec<Ty> {
+    let mut out: Vec<Ty> = Vec::new()
+    for t in tys.iter() { out.push(shift_params(t, by)) }
+    out
+}
+
+fn walk_param(t: Ty, s: Vec<Option<Ty>>) -> Ty {
+    match t {
+        TyParam(k) => {
+            if k >= 0 && k < s.len() {
+                match s.at(k) {
+                    Some(bound) => walk_param(bound, s)
+                    None => t
+                }
+            } else {
+                t
+            }
+        }
+        _ => t
+    }
+}
+
+fn occurs(k: Int, t: Ty, s: Vec<Option<Ty>>) -> Bool {
+    match walk_param(t, s) {
+        TyParam(j) => j == k
+        TyFn(params, ret) => {
+            for p in params.iter() { if occurs(k, p, s) { return true } }
+            occurs(k, ret, s)
+        }
+        TySum(_, args) => any_occurs(k, args, s)
+        TyRecord(_, args) => any_occurs(k, args, s)
+        TyArray(e) => occurs(k, e, s)
+        TyTuple(es) => any_occurs(k, es, s)
+        TyFuture(o) => occurs(k, o, s)
+        _ => false
+    }
+}
+
+fn any_occurs(k: Int, tys: Vec<Ty>, s: Vec<Option<Ty>>) -> Bool {
+    for t in tys.iter() { if occurs(k, t, s) { return true } }
+    false
+}
+
+fn unify_patterns(a: Ty, b: Ty, mut s: Vec<Option<Ty>>) -> Bool {
+    let a = walk_param(a, s)
+    let b = walk_param(b, s)
+    match (a, b) {
+        (TyParam(i), TyParam(j)) => {
+            if i == j { return true }
+            if occurs(i, b, s) { return false }
+            s.set(i, Some(b))
+            true
+        }
+        (TyParam(i), _) => {
+            if occurs(i, b, s) { return false }
+            s.set(i, Some(b))
+            true
+        }
+        (_, TyParam(j)) => {
+            if occurs(j, a, s) { return false }
+            s.set(j, Some(a))
+            true
+        }
+        (TyRecord(d1, a1), TyRecord(d2, a2)) => d1 == d2 && unify_all(a1, a2, s)
+        (TySum(d1, a1), TySum(d2, a2)) => d1 == d2 && unify_all(a1, a2, s)
+        (TyArray(e1), TyArray(e2)) => unify_patterns(e1, e2, s)
+        (TyTuple(t1), TyTuple(t2)) => unify_all(t1, t2, s)
+        (TyFuture(o1), TyFuture(o2)) => unify_patterns(o1, o2, s)
+        (TyFn(p1, r1), TyFn(p2, r2)) => unify_all(p1, p2, s) && unify_patterns(r1, r2, s)
+        (x, y) => ty_eq(x, y)
+    }
+}
+
+fn unify_all(xs: Vec<Ty>, ys: Vec<Ty>, s: Vec<Option<Ty>>) -> Bool {
+    if xs.len() != ys.len() { return false }
+    let mut i = 0
+    while i < xs.len() {
+        if !unify_patterns(xs.at(i), ys.at(i), s) { return false }
+        i += 1
+    }
+    true
+}
diff --git a/selfhost/src/hir_dump.nova b/selfhost/src/hir_dump.nova
new file mode 100644
index 00000000..19ff6fd0
--- /dev/null
+++ b/selfhost/src/hir_dump.nova
@@ -0,0 +1,354 @@
+// The HIR dump: what the type checker built for one program, as text, one
+// entry per line, exactly as S0's `nova check --dump-hir` prints it: a
+// port of crates/nova-typeck/src/dump.rs, function by function (spec
+// docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md
+// §3.3). ADR 0055 writes the grammar out, with an example of every form.
+//
+// - Two parts: std's, printed once (`dump_hir_std`), and a program's
+//   (`dump_hir_program`), which prints only the program's own entries and
+//   `(StdStart …)` as the resolution dump does.
+// - Ids are the resolution dump's, `u<n>` and `s<n>`; an impl is numbered
+//   from 0 within its part.
+// - Types print by structure, with ids (`ty_text`).
+// - Tables print in the order the type checker collected them; the
+//   signatures, a map in S0, by id.
+
+import builtins
+import diagnostic
+import dump
+import hir
+import resolution_dump
+import resolver
+import token
+import typeck
+
+/// A program's part for `target`, under test mode when `test`: when the
+/// program reached the type checker, its resolution and item tables, and
+/// every diagnostic so far, in order (`dump_program`).
+pub fn dump_hir_program(
+    target: CompileTarget,
+    test: Bool,
+    checked: Option<(ProgramResolution, Checker)>,
+    diagnostics: Vec<Diagnostic>,
+) -> String {
+    let mut d = HirDump { out: StringBuilder::new(), std_def: 9223372036854775807, fresh: true }
+    d.open(0, "Hir")
+    d.field("target", compile_target_name(target))
+    d.mode(test)
+    d.bool_field("checked", checked.is_some())
+    match checked {
+        Some((resolution, tables)) => {
+            let (std_module, def, item) = std_start_of(resolution.definitions)
+            d.std_def = def
+            d.open(1, "StdStart")
+            d.field("module", "${std_module}")
+            d.field("def", "${def}")
+            d.field("item", "${item}")
+            d.close()
+            d.tables(resolution.definitions, tables, false)
+        }
+        None => ()
+    }
+    d.close()
+    d.out.push("\n")
+    d.out.push(diagnostics_text(diagnostics))
+    d.out.to_string()
+}
+
+/// std's part for `target`, under test mode when `test`, from the check
+/// of a program of no module of its own: its tables and every diagnostic,
+/// the resolver's and the item passes' (`dump_std`).
+pub fn dump_hir_std(
+    target: CompileTarget,
+    test: Bool,
+    resolution: ProgramResolution,
+    tables: Checker,
+    diagnostics: Vec<Diagnostic>,
+) -> String {
+    let defs = resolution.definitions
+    let (_, std_def, _) = std_start_of(defs)
+    let mut d = HirDump { out: StringBuilder::new(), std_def: std_def, fresh: true }
+    d.open(0, "HirStd")
+    d.field("target", compile_target_name(target))
+    d.mode(test)
+    let mut records = 0
+    for r in tables.records.iter() { if r.def_id >= std_def { records += 1 } }
+    let mut sums = 0
+    for s in tables.sums.iter() { if s.def_id >= std_def { sums += 1 } }
+    let mut traits = 0
+    for t in tables.traits.iter() { if t.def_id >= std_def { traits += 1 } }
+    let mut impls = 0
+    for s in tables.impl_in_std.iter() { if s { impls += 1 } }
+    let mut sigs = 0
+    for id in 0..tables.sigs.len() {
+        if id >= std_def && tables.sigs.at(id).is_some() { sigs += 1 }
+    }
+    let mut externs = 0
+    for e in tables.externs.iter() { if e.def_id >= std_def { externs += 1 } }
+    d.field("records", "${records}")
+    d.field("sums", "${sums}")
+    d.field("traits", "${traits}")
+    d.field("impls", "${impls}")
+    d.field("sigs", "${sigs}")
+    d.field("externs", "${externs}")
+    d.tables(defs, tables, true)
+    d.close()
+    d.out.push("\n")
+    d.out.push(diagnostics_text(diagnostics))
+    d.out.to_string()
+}
+
+// The text being written, the first std definition's id, from which ids
+// print as `s<n>`, and whether the text is empty or ends a line (`Dump`).
+record HirDump { out: StringBuilder, std_def: Int, fresh: Bool }
+
+impl HirDump {
+    // A line, up to its kind (`open`).
+    fn open(mut self, depth: Int, kind: String) {
+        if !self.fresh { self.out.push("\n") }
+        self.fresh = false
+        for _ in 0..depth { self.out.push("  ") }
+        self.out.push("(")
+        self.out.push(kind)
+    }
+
+    fn close(mut self) { self.out.push(")") }
+
+    // ` key=value`.
+    fn field(mut self, key: String, value: String) { self.out.push(" ${key}=${value}") }
+
+    // ` mode=test` under test mode (`mode`).
+    fn mode(mut self, test: Bool) { if test { self.out.push(" mode=test") } }
+
+    fn bool_field(mut self, key: String, value: Bool) { self.out.push(" ${key}=${value}") }
+
+    // ` ` and `text` quoted.
+    fn string(mut self, text: String) {
+        self.out.push(" \"")
+        push_escaped(self.out, text, true)
+        self.out.push("\"")
+    }
+
+    // ` ` and a value.
+    fn value(mut self, text: String) {
+        self.out.push(" ")
+        self.out.push(text)
+    }
+
+    // ` ` and a place: the file's label quoted, `:`, the span
+    // (`write_place`).
+    fn place(mut self, at: FileSpan) {
+        self.string(at.file)
+        self.out.push(":${at.span.start}..${at.span.end}")
+    }
+
+    // A definition's id: `u<n>` for the program's own, `s<n>` for std's.
+    fn id(self, id: Int) -> String {
+        if id >= self.std_def { "s${id - self.std_def}" } else { "u${id}" }
+    }
+
+    fn opt_id(self, id: Int) -> String { if id < 0 { "None" } else { self.id(id) } }
+
+    // A type, by structure (`ty`).
+    fn ty_text(self, ty: Ty) -> String {
+        match ty {
+            TyInt => "Int"
+            TyFloat => "Float"
+            TyBool => "Bool"
+            TyChar => "Char"
+            TyString => "String"
+            TyBytes => "Bytes"
+            TyUnit => "()"
+            TyFn(
+                params,
+                ret,
+            ) => "fn(${", ".join(self.list(params).to_array())}) -> ${self.ty_text(ret)}"
+            TyFuture(o) => "Future<${self.ty_text(o)}>"
+            TySum(d, args) => self.named(d, args)
+            TyRecord(d, args) => self.named(d, args)
+            TyArray(e) => "[${self.ty_text(e)}]"
+            TyTuple(es) => tuple_text(self.list(es))
+            TyParam(i) => "T${i}"
+            TyAssoc(on, assoc) => "${self.ty_text(on)}::${self.id(assoc)}"
+            TyVar(v) => "?${v}"
+            TyNever => "!"
+            TyError => "{error}"
+        }
+    }
+
+    fn list(self, tys: Vec<Ty>) -> Vec<String> {
+        let mut out: Vec<String> = Vec::new()
+        for t in tys.iter() { out.push(self.ty_text(t)) }
+        out
+    }
+
+    fn named(self, d: Int, args: Vec<Ty>) -> String {
+        if args.is_empty() {
+            self.id(d)
+        } else {
+            "${self.id(d)}<${", ".join(self.list(args).to_array())}>"
+        }
+    }
+
+    fn head_text(self, h: TyHead) -> String {
+        match h {
+            TyHeadInt => "Int"
+            TyHeadFloat => "Float"
+            TyHeadBool => "Bool"
+            TyHeadChar => "Char"
+            TyHeadString => "String"
+            TyHeadBytes => "Bytes"
+            TyHeadArray => "Array"
+            TyHeadSum(d) => self.id(d)
+            TyHeadRecord(d) => self.id(d)
+        }
+    }
+
+    // `(Bound k id…)` for each generic parameter's bounds, at `depth`.
+    fn bounds(mut self, depth: Int, bounds: Vec<Vec<Int>>) {
+        for k in 0..bounds.len() {
+            self.open(depth, "Bound")
+            self.out.push(" ${k}")
+            for id in bounds.at(k).iter() { self.value(self.id(id)) }
+            self.close()
+        }
+    }
+
+    // `(Param ty)` for each parameter, then `(Ret ty)`, at `depth`.
+    fn signature(mut self, depth: Int, params: Vec<Ty>, ret: Ty) {
+        for p in params.iter() {
+            self.open(depth, "Param")
+            self.value(self.ty_text(p))
+            self.close()
+        }
+        self.open(depth, "Ret")
+        self.value(self.ty_text(ret))
+        self.close()
+    }
+
+    // The entries of std's part (`std`) or of the program's (`tables`).
+    fn tables(mut self, defs: Definitions, t: Checker, std: Bool) {
+        for r in t.records.iter() {
+            if (r.def_id >= self.std_def) != std { continue }
+            self.open(1, "Record")
+            self.value(self.id(r.def_id))
+            self.string(r.name)
+            self.field("generics", "${r.generics}")
+            for f in r.fields.iter() {
+                self.open(2, "Field")
+                self.string(f.name)
+                self.value(self.ty_text(f.ty))
+                self.close()
+            }
+            self.close()
+        }
+        for s in t.sums.iter() {
+            if (s.def_id >= self.std_def) != std { continue }
+            self.open(1, "Sum")
+            self.value(self.id(s.def_id))
+            self.string(s.name)
+            self.field("generics", "${s.generics}")
+            for v in s.variants.iter() {
+                self.open(2, "Variant")
+                self.string(v.name)
+                for f in v.fields.iter() { self.value(self.ty_text(f)) }
+                self.close()
+            }
+            self.close()
+        }
+        for tr in t.traits.iter() {
+            if (tr.def_id >= self.std_def) != std { continue }
+            self.open(1, "Trait")
+            self.value(self.id(tr.def_id))
+            self.string(tr.name)
+            for s in tr.supertraits.iter() {
+                self.open(2, "Supertrait")
+                self.value(self.id(s))
+                self.close()
+            }
+            for (name, assoc) in tr.assoc_types.iter() {
+                self.open(2, "AssocType")
+                self.string(name)
+                self.value(self.id(assoc))
+                self.close()
+            }
+            for m in tr.methods.iter() {
+                self.open(2, "Method")
+                self.string(m.name)
+                self.bool_field("has_self", m.has_self)
+                self.bool_field("mut_self", m.mut_self)
+                self.field("generics", "${m.generics}")
+                self.field("default", self.opt_id(m.default_def))
+                self.bounds(3, m.bounds)
+                self.signature(3, m.params, m.ret)
+                self.close()
+            }
+            self.close()
+        }
+        let mut index = 0
+        for k in 0..t.impls.len() {
+            let in_std = k < t.impl_in_std.len() && t.impl_in_std.at(k)
+            if in_std != std { continue }
+            let imp = t.impls.at(k)
+            self.open(1, "Impl")
+            self.out.push(" ${index}")
+            index += 1
+            self.field("trait", self.opt_id(imp.trait_id))
+            self.field("head", self.head_text(imp.self_head))
+            self.field("generics", "${imp.generics}")
+            if k < t.impl_spans.len() { self.place(t.impl_spans.at(k)) }
+            self.open(2, "SelfType")
+            self.value(self.ty_text(imp.self_ty))
+            self.close()
+            self.bounds(2, imp.bounds)
+            for (name, def) in imp.methods.iter() {
+                self.open(2, "Method")
+                self.string(name)
+                self.value(self.id(def))
+                self.close()
+            }
+            for (assoc, ty) in imp.assoc_bindings.iter() {
+                self.open(2, "AssocBinding")
+                self.value(self.id(assoc))
+                self.value(self.ty_text(ty))
+                self.close()
+            }
+            self.close()
+        }
+        for id in 0..t.sigs.len() {
+            if (id >= self.std_def) != std { continue }
+            let sig = match t.sigs.at(id) {
+                Some(s) => s
+                None => continue
+            }
+            self.open(1, "Sig")
+            self.value(self.id(id))
+            self.field("generics", "${sig.generics}")
+            let impl_method = match defs.defs.at(id).kind {
+                DefKindMethod(_, _, MethodOwnerImpl) => true
+                _ => false
+            }
+            if impl_method {
+                self.bool_field("selfless", t.selfless.at(id))
+                self.bool_field("mut_self", t.mut_self.at(id))
+            }
+            self.bounds(2, sig.bounds)
+            self.signature(2, sig.params, sig.ret)
+            self.close()
+        }
+        for e in t.externs.iter() {
+            if (e.def_id >= self.std_def) != std { continue }
+            self.open(1, "Extern")
+            self.value(self.id(e.def_id))
+            self.string(e.symbol)
+            let abi = match e.abi {
+                ExternAbiC => "C"
+                ExternAbiJs => "Js"
+            }
+            self.field("abi", abi)
+            self.place(e.span)
+            self.signature(2, e.params, e.ret)
+            self.close()
+        }
+    }
+}
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index c2c35a8c..489b7dee 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -1,5 +1,6 @@
 // N, the self-hosted Nova compiler, so far its lexer, its parser, its
-// loader of programs and packages and its resolver (spec
+// loader of programs and packages, its resolver and its type checker's
+// item stage (spec
 // docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md
 // §3.2 and §4.3, ADR 0051 and ADR 0052; spec
 // docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
@@ -23,6 +24,14 @@
 //   selfhost resolve --std DIR [--target native|wasm] [--test] --std-part
 //                             prints std's part, as `nova check
 //                             --dump-resolution --std` does
+//   selfhost hir --std DIR [--target native|wasm] [--test] FILE...
+//   selfhost hir --std DIR [--target native|wasm] [--test] --std-part
+//                             the same, with the HIR dump of the type
+//                             checker's item stage, as `nova check
+//                             --dump-hir` prints it, std's item tables
+//                             collected once (spec
+//                             docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md
+//                             §3.3, ADR 0055)
 //
 // A file with lex or parse errors is dumped with them, and the exit code
 // stays 0. A file that cannot be read prints `IOERR` and its kind after its
@@ -33,6 +42,9 @@ import ast_dump
 import builtins
 import diagnostic
 import dump
+import fixes
+import hir
+import hir_dump
 import lexer
 import lexer_tests
 import loader
@@ -43,13 +55,16 @@ import resolution_dump
 import resolver
 import resolver_tests
 import token
+import typeck
 import typeck_tests
+import typeck_traits
+import typeck_ty
 
 async fn main() {
     let argv = args()
     let command = if argv.len() < 2 { "" } else { argv.at(1) }
-    if command == "resolve" {
-        resolve_command(argv).await
+    if command == "resolve" || command == "hir" {
+        dump_command(argv, command).await
         return
     }
     // No file is refused, as S0's clap refuses it.
@@ -91,8 +106,12 @@ async fn main() {
 // package's manifest N's reader refuses, or whose package graph would
 // give an M-code, S0's alone, stops the run: N says why on stderr and
 // exits 1, printing nothing of that root's part.
-async fn resolve_command(argv: Vec<String>) {
-    let usage = "usage: selfhost resolve --std DIR [--target native|wasm] [--test] (--std-part | FILE...)"
+//
+// `selfhost hir` takes the same arguments and refuses the same shapes, as
+// S0's `check --dump-hir` shares `check --dump-resolution`'s parser, and
+// prints the HIR dump in place of the resolution dump.
+async fn dump_command(argv: Vec<String>, command: String) {
+    let usage = "usage: selfhost ${command} --std DIR [--target native|wasm] [--test] (--std-part | FILE...)"
     let mut std_dir = ""
     let mut target_name = ""
     let mut target_given = false
@@ -151,7 +170,7 @@ async fn resolve_command(argv: Vec<String>) {
         return
     }
     if test && target_name == "wasm" {
-        eprintln("error: selfhost resolve --test is native only, as nova test is")
+        eprintln("error: selfhost ${command} --test is native only, as nova test is")
         exit(2)
         return
     }
@@ -163,23 +182,47 @@ async fn resolve_command(argv: Vec<String>) {
             return
         }
     }
+    let hir = command == "hir"
     if std_part {
         let resolved = resolve_program_for(target, Vec::new(), std_set)
-        print(dump_std(target, test, resolved, std_set.builtins))
+        if hir {
+            let (labels, texts) = labels_and_texts(Vec::new(), std_set)
+            let tables = check_items(resolved.items, resolved.definitions, labels, texts)
+            let mut diagnostics: Vec<Diagnostic> = Vec::new()
+            for d in resolved.diagnostics.iter() { diagnostics.push(d) }
+            for d in tables.diagnostics.iter() { diagnostics.push(d) }
+            print(dump_hir_std(target, test, resolved, tables, diagnostics))
+        } else {
+            print(dump_std(target, test, resolved, std_set.builtins))
+        }
         return
     }
-    // std is resolved once, and every root reads it (ADR 0054 §9).
+    // std is resolved once, and every root reads it (ADR 0054 §9); for the
+    // HIR dump std's items are collected once too, from the resolution of
+    // a program of no module of its own (ADR 0055).
     let shared = share_std(target, std_set)
+    let mut items: Option<SharedItems> = None
+    if hir {
+        let (labels, texts) = labels_and_texts(Vec::new(), std_set)
+        let alone = shared.alone
+        let s = share_items(alone.items, alone.definitions, labels, texts)
+        if shared.shareable && s.shareable { items = Some(s) }
+    }
     let mut failed = false
     for path in files.iter() {
-        match dump_resolution(path, target, test, shared).await {
+        let dumped = if hir {
+            dump_hir(path, target, test, shared, items).await
+        } else {
+            dump_resolution(path, target, test, shared).await
+        }
+        match dumped {
             DumpedText(out) => print("== ${path}\n${out}")
             DumpedIoError(e) => {
                 print("== ${path}\nIOERR ${io_kind_name(e.kind)}\n")
                 failed = true
             }
             DumpedRefused(why) => {
-                eprintln("error: selfhost resolve cannot dump ${path}: ${why}")
+                eprintln("error: selfhost ${command} cannot dump ${path}: ${why}")
                 exit(1)
                 return
             }
@@ -306,6 +349,100 @@ async fn dump_resolution(
     DumpedText(dump_program(target, test, sources, Some(resolved), diagnostics, std_set.builtins))
 }
 
+// The HIR dump of the program whose root is `path` (nova-driver's
+// `HirDumps::dump`): `dump_resolution`'s pipeline, then, when it stopped
+// on no error, the type checker's item passes. With std's tables `items`,
+// the program is resolved with std's definitions after its own and reads
+// std's tables rebased past them; without, because std alone gave a
+// diagnostic or holds a test, it is resolved and checked in full, as
+// `check` does. The dump holds the item tables and every diagnostic so
+// far, or, for a program that stopped before the type checker,
+// `checked=false` and the diagnostics.
+async fn dump_hir(
+    path: String,
+    target: CompileTarget,
+    test: Bool,
+    shared: SharedStd,
+    items: Option<SharedItems>,
+) -> Dumped {
+    let std_set = shared.std_set
+    let mut graph: Option<Graph> = None
+    let mut kind = RootLoose
+    match package_of(path).await {
+        Some((dir, k)) => {
+            match read_graph(dir).await {
+                Ok(g) => graph = Some(g)
+                Err(why) => return DumpedRefused(why)
+            }
+            kind = k
+        }
+        None => ()
+    }
+    let load = match load_program(graph, path, kind).await {
+        Ok(l) => l
+        Err(e) => return DumpedIoError(e)
+    }
+    let modules = load.modules
+    let mut diagnostics = load.diagnostics
+    let mut failed = false
+    for d in diagnostics.iter() { if d.is_error() { failed = true } }
+    if !failed { strip_test_functions(modules, test) }
+    let mut sources: Vec<ModuleSource> = Vec::new()
+    for m in modules.iter() {
+        sources.push(
+            ModuleSource {
+                name: m.name,
+                label: m.name,
+                file: m.ast,
+                imports: m.imports,
+                text: Some(m.text),
+                package: m.package,
+                importable: m.importable,
+            },
+        )
+    }
+    if failed { return DumpedText(dump_hir_program(target, test, None, diagnostics)) }
+    let resolved = match items {
+        Some(_) => resolve_program_shared_for_check(sources, shared)
+        None => resolve_program_for(target, sources, std_set)
+    }
+    name_tests(resolved.tests, resolved.definitions, modules)
+    for d in resolved.diagnostics.iter() {
+        diagnostics.push(d)
+        if d.is_error() { failed = true }
+    }
+    if failed { return DumpedText(dump_hir_program(target, test, None, diagnostics)) }
+    let (labels, texts) = labels_and_texts(sources, std_set)
+    let tables = match items {
+        Some(s) => check_items_shared(resolved.items, resolved.definitions, labels, texts, s)
+        None => check_items(resolved.items, resolved.definitions, labels, texts)
+    }
+    for d in tables.diagnostics.iter() { diagnostics.push(d) }
+    DumpedText(dump_hir_program(target, test, Some((resolved, tables)), diagnostics))
+}
+
+// Each module's file label, the program's `sources` then std's modules
+// that parsed, as the resolver numbers them, and each program module's
+// source text, for a fix's edit; std's have none.
+fn labels_and_texts(
+    sources: Vec<ModuleSource>,
+    std_set: StdSet,
+) -> (Vec<String>, Vec<Option<String>>) {
+    let mut labels: Vec<String> = Vec::new()
+    let mut texts: Vec<Option<String>> = Vec::new()
+    for m in sources.iter() {
+        labels.push(m.label)
+        texts.push(m.text)
+    }
+    for s in std_set.modules.iter() {
+        if s.file.is_some() {
+            labels.push(s.label)
+            texts.push(None)
+        }
+    }
+    (labels, texts)
+}
+
 // Name each test of a module of a package's `tests/`
 // `<file stem>::<function>` (the driver's `name_tests`), as `nova test`'s
 // inventory names it.
diff --git a/selfhost/src/resolution_dump.nova b/selfhost/src/resolution_dump.nova
index cb2c9921..b254951f 100644
--- a/selfhost/src/resolution_dump.nova
+++ b/selfhost/src/resolution_dump.nova
@@ -127,6 +127,29 @@ pub fn dump_std(
     d.out.to_string()
 }
 
+/// Where std begins in `defs`, as `(StdStart …)` prints it: the first std
+/// module, the first std definition and the first item of a std module
+/// (`std_start`). The HIR dump repeats it (5.3a, ADR 0055).
+pub fn std_start_of(defs: Definitions) -> (Int, Int, Int) {
+    let std_module = defs.std_start
+    let std_item = first_item_of(defs, std_module)
+    (std_module, first_std_def(defs, std_item), std_item)
+}
+
+/// `diagnostics` as `(Diagnostics …)` prints them, each on lines of its
+/// own (`write_diagnostics`): the HIR dump prints them so too (5.3a, ADR
+/// 0055).
+pub fn diagnostics_text(diagnostics: Vec<Diagnostic>) -> String {
+    let mut d = ResDump {
+        out: StringBuilder::new(),
+        std_def: 9223372036854775807,
+        fresh: true,
+        variants: BUILTIN_VARIANTS,
+    }
+    d.diagnostics(diagnostics)
+    d.out.to_string()
+}
+
 // The index of the first merged item of a module from `std_module` on.
 fn first_item_of(defs: Definitions, std_module: Int) -> Int {
     let mut i = 0
diff --git a/selfhost/src/resolver.nova b/selfhost/src/resolver.nova
index cd64d6a6..318a48e2 100644
--- a/selfhost/src/resolver.nova
+++ b/selfhost/src/resolver.nova
@@ -527,6 +527,11 @@ pub record SharedStd {
     // no `@test` function, which the shared path would drop; else a program
     // is resolved in full.
     pub shareable: Bool,
+    // The resolution of a program of no module of its own: std's
+    // definitions, scopes, items and exports, numbered from 0, which the
+    // type checker's shared path reads too (5.3a,
+    // `resolve_program_shared_for_check`).
+    pub alone: ProgramResolution,
 }
 
 /// std for `target`, resolved once (`SharedStd::new`).
@@ -560,6 +565,7 @@ pub fn share_std(target: CompileTarget, std_set: StdSet) -> SharedStd {
         missing: alone.definitions.missing,
         globs: globs,
         shareable: alone.diagnostics.len() == parsed && alone.tests.is_empty(),
+        alone: alone,
     }
 }
 
@@ -753,6 +759,51 @@ pub fn resolve_program_shared(modules: Vec<ModuleSource>, shared: SharedStd) ->
     }
 }
 
+/// `resolve_program_shared` for the type checker's shared path (5.3a,
+/// the HIR dump alone; nova-resolver's
+/// `resolve_program_with_shared_std_for_check`): std's definitions and its
+/// items' modules follow the program's, rebased past them, as the full
+/// resolution numbers them, so that the type checker can read std's
+/// definitions; std's items are not in `items`, since std's item tables
+/// are shared too (typeck.nova's `check_items_shared`). std's module
+/// scopes follow the program's as they are, their ids std's own: the type
+/// checker adds the offset where it reads one, rather than every program
+/// copying them. A program resolved in full, because std's resolution gave
+/// a diagnostic or holds a `@test` function, is returned as
+/// `resolve_program_for` gives it.
+pub fn resolve_program_shared_for_check(
+    modules: Vec<ModuleSource>,
+    shared: SharedStd,
+) -> ProgramResolution {
+    let resolved = resolve_program_shared(modules, shared)
+    if !shared.shareable { return resolved }
+    let mut defs = resolved.definitions
+    let def_offset = defs.defs.len()
+    let item_offset = resolved.items.len()
+    let module_offset = defs.std_start
+    let std = shared.alone.definitions
+    for d in std.defs.iter() { defs.defs.push(rebased_def(d, def_offset, item_offset)) }
+    for m in std.modules.iter() { defs.modules.push(m) }
+    for m in std.item_module.iter() { defs.item_module.push(m + module_offset) }
+    resolved
+}
+
+// `d` with every definition's id `def` higher and every item's index
+// `item` higher (`Def::rebased`).
+fn rebased_def(d: Def, def: Int, item: Int) -> Def {
+    let kind = match d.kind {
+        DefKindFn(i) => DefKindFn(i + item)
+        DefKindSum(i, variants) => DefKindSum(i + item, variants)
+        DefKindRecord(i) => DefKindRecord(i + item)
+        DefKindConst(i) => DefKindConst(i + item)
+        DefKindTrait(i) => DefKindTrait(i + item)
+        DefKindMethod(i, method, owner) => DefKindMethod(i + item, method, owner)
+        DefKindExternFn(i, f) => DefKindExternFn(i + item, f)
+        DefKindAssocType(t) => DefKindAssocType(t + def)
+    }
+    Def { name: d.name, file: d.file, span: d.span, kind: kind }
+}
+
 // Bind each builtin of `builtins` by its name in `scope`'s values.
 fn seed(mut scope: ModuleScope, table: BuiltinTable, builtins: Vec<Int>) {
     for b in builtins.iter() { let _ = scope.values.insert(table.names[b], ResBuiltin(b)) }
diff --git a/selfhost/src/typeck.nova b/selfhost/src/typeck.nova
new file mode 100644
index 00000000..17c59f91
--- /dev/null
+++ b/selfhost/src/typeck.nova
@@ -0,0 +1,1071 @@
+// The type checker's item stage: a port of crates/nova-typeck's
+// `check_items` (check/items.rs) and of passes 2 to 5 of `check_with`
+// (check.rs), function by function and with the same names, so that the
+// two read side by side (spec
+// docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md
+// §3.1 and §3.2, ADR 0055). It builds what S0's builds, the item tables
+// (records, sums, traits, impls, every signature, externs), and gives every
+// item diagnostic in S0's order, fixes included; the HIR dump
+// (hir_dump.nova) prints them, and the differential in
+// crates/nova-cli/tests/selfhost.rs holds them equal to S0's. A change to
+// how items are collected changes both type checkers in one pull request.
+//
+// This file holds the state, `check_items`, std's shared tables, the
+// generic-parameter checks, the arities, supertraits, records and sums,
+// and the message helpers; typeck_traits.nova the traits, the impls and
+// their checks; typeck_ty.nova `convert_ty`, projections, `normalize`,
+// bounds, `where`, the signatures and the externs; fixes.nova the fixes.
+// The Rust's idioms, beyond hir.nova's:
+//
+// - The checker's maps keyed by `DefId` (`sigs`, `method_locs`,
+//   `selfless`, `mut_self`, `supertraits`, `type_arity`) are lists indexed
+//   by the id, as long as the definitions: looked up as the maps are, and
+//   read in id order where the dump sorts the map's keys.
+// - A composite key, `(item, method)`, is one `Int`, `item * 2^20 + method`.
+// - A span carries no file in N, so the checker keeps the label of the
+//   module whose item it is checking (`cur_file`) beside `cur_module`, and a
+//   span kept in a table is a `FileSpan`. A check that reads a table's spans
+//   says which file each diagnostic is in.
+// - The language server's index (`note_*`, `enter_type_params`) is not
+//   built: nothing the dump prints reads it.
+
+import ast
+import builtins
+import diagnostic
+import fixes
+import hir
+import resolver
+import token
+
+// ---------------------------------------------------------------------------
+// The state
+// ---------------------------------------------------------------------------
+
+/// A collected function's or method's signature (`FnSig`).
+pub record FnSig { pub generics: Int, pub bounds: Vec<Vec<Int>>, pub params: Vec<Ty>, pub ret: Ty }
+
+/// Where a method's body is (`MethodLoc`).
+pub record MethodLoc { pub item_index: Int, pub method_index: Int, pub owner: MethodOwner }
+
+/// The `Self` of an impl block (`ImplSelf`); `trait_id` -1 for an inherent
+/// impl.
+pub record ImplSelf { pub ty: Ty, pub trait_id: Int }
+
+/// The checker (`Checker`), and what passes 2 to 5 leave in it, which is
+/// what S0's `ItemTables` holds: the item tables, the item diagnostics in
+/// the order the passes emitted them, and the state the bodies start from.
+pub record Checker {
+    pub items: Vec<Spanned<Item>>,
+    pub defs: Definitions,
+    // Each module's file label and, for a program's own, its source text.
+    pub labels: Vec<String>,
+    pub texts: Vec<Option<String>>,
+    pub cur_module: Int,
+    pub cur_file: String,
+    pub sigs: Vec<Option<FnSig>>,
+    pub method_locs: Vec<Option<MethodLoc>>,
+    pub selfless: Vec<Bool>,
+    pub mut_self: Vec<Bool>,
+    pub sums: Vec<SumType>,
+    pub records: Vec<RecordType>,
+    pub supertraits: Vec<Option<Vec<Int>>>,
+    pub traits: Vec<TraitDef>,
+    pub impls: Vec<ImplInfo>,
+    pub impl_param_names: Vec<Vec<String>>,
+    pub impl_self: Option<ImplSelf>,
+    pub impl_selves: Map<Int, ImplSelf>,
+    pub type_arity: Vec<Int>,
+    pub externs: Vec<ExternFn>,
+    pub diagnostics: Vec<Diagnostic>,
+    pub impl_spans: Vec<FileSpan>,
+    pub impl_in_std: Vec<Bool>,
+    pub impl_method_spans: Vec<Vec<FileSpan>>,
+    // Each trait's associated types, in definition order, by the trait's id.
+    pub assoc_of: Map<Int, Vec<Int>>,
+    // The definitions the passes collect, `0..own_defs`; std's, after them,
+    // are read from `shared` when it is given, rebased by `rebase_def` and
+    // `rebase_item`. std's module scopes are then the shared resolution's,
+    // whose ids are std's own: a lookup in one adds `scope_offset`.
+    pub own_defs: Int,
+    pub shared: Option<SharedItems>,
+    pub rebase_def: Int,
+    pub rebase_item: Int,
+    pub scope_offset: Int,
+}
+
+/// std's item tables, built once from the check of a program of no module
+/// of its own (`SharedItems`): `defs` is how many definitions std has, and
+/// `shareable` whether its items gave no diagnostic.
+pub record SharedItems { pub tables: Checker, pub defs: Int, pub shareable: Bool }
+
+/// Passes 2 to 5 over `items` and `defs` (`check_items`), each module's
+/// file label in `labels` and a program module's source in `texts`.
+pub fn check_items(
+    items: Vec<Spanned<Item>>,
+    defs: Definitions,
+    labels: Vec<String>,
+    texts: Vec<Option<String>>,
+) -> Checker {
+    let mut c = new_checker(items, defs, labels, texts, None, 0)
+    c.collect_items()
+    c
+}
+
+/// std's item tables from the resolution of a program of no module of its
+/// own (`SharedItems::new`).
+pub fn share_items(
+    items: Vec<Spanned<Item>>,
+    defs: Definitions,
+    labels: Vec<String>,
+    texts: Vec<Option<String>>,
+) -> SharedItems {
+    let tables = check_items(items, defs, labels, texts)
+    SharedItems { tables: tables, defs: defs.defs.len(), shareable: tables.diagnostics.is_empty() }
+}
+
+/// `check_items` reading std's tables from `shared` (`check_items_shared`):
+/// `items` holds the program's own items only, and `defs` its definitions
+/// followed by std's, rebased (resolver.nova's
+/// `resolve_program_shared_for_check`), whose module scopes are std's as
+/// they are. Each pass runs over the program's items, and std's part of
+/// each table is added where the full check would have collected it.
+pub fn check_items_shared(
+    items: Vec<Spanned<Item>>,
+    defs: Definitions,
+    labels: Vec<String>,
+    texts: Vec<Option<String>>,
+    shared: SharedItems,
+) -> Checker {
+    let own = defs.defs.len() - shared.defs
+    let mut c = new_checker(items, defs, labels, texts, Some(shared), own)
+    c.collect_items()
+    c
+}
+
+// A checker with empty tables (`Checker::new`).
+fn new_checker(
+    items: Vec<Spanned<Item>>,
+    defs: Definitions,
+    labels: Vec<String>,
+    texts: Vec<Option<String>>,
+    shared: Option<SharedItems>,
+    own: Int,
+) -> Checker {
+    let n = defs.defs.len()
+    let mut sigs: Vec<Option<FnSig>> = Vec::new()
+    let mut locs: Vec<Option<MethodLoc>> = Vec::new()
+    let mut selfless: Vec<Bool> = Vec::new()
+    let mut mut_self: Vec<Bool> = Vec::new()
+    let mut supers: Vec<Option<Vec<Int>>> = Vec::new()
+    let mut arity: Vec<Int> = Vec::new()
+    let mut assoc_of: Map<Int, Vec<Int>> = Map::new()
+    for i in 0..n {
+        sigs.push(None)
+        locs.push(None)
+        selfless.push(false)
+        mut_self.push(false)
+        supers.push(None)
+        arity.push(0)
+        match defs.defs.at(i).kind {
+            DefKindAssocType(t) => {
+                match assoc_of.get(t) {
+                    Some(list) => {
+                        let mut l = list
+                        l.push(i)
+                    }
+                    None => {
+                        let mut l: Vec<Int> = Vec::new()
+                        l.push(i)
+                        let _ = assoc_of.insert(t, l)
+                    }
+                }
+            }
+            _ => ()
+        }
+    }
+    let is_shared = shared.is_some()
+    Checker {
+        items: items,
+        defs: defs,
+        labels: labels,
+        texts: texts,
+        cur_module: 0,
+        cur_file: if labels.is_empty() { "" } else { labels.at(0) },
+        sigs: sigs,
+        method_locs: locs,
+        selfless: selfless,
+        mut_self: mut_self,
+        sums: Vec::new(),
+        records: Vec::new(),
+        supertraits: supers,
+        traits: Vec::new(),
+        impls: Vec::new(),
+        impl_param_names: Vec::new(),
+        impl_self: None,
+        impl_selves: Map::new(),
+        type_arity: arity,
+        externs: Vec::new(),
+        diagnostics: Vec::new(),
+        impl_spans: Vec::new(),
+        impl_in_std: Vec::new(),
+        impl_method_spans: Vec::new(),
+        assoc_of: assoc_of,
+        own_defs: if is_shared { own } else { n },
+        shared: shared,
+        rebase_def: own,
+        rebase_item: items.len(),
+        scope_offset: if is_shared { own } else { 0 },
+    }
+}
+
+impl Checker {
+    // Passes 2 to 5 (`collect_items`); pass 1 is the language server's
+    // index alone.
+    fn collect_items(mut self) {
+        self.reject_self_type_params()
+        self.collect_type_arities()
+        self.splice_arities()
+        self.collect_supertraits()
+        self.splice_supertraits()
+        self.collect_records()
+        self.splice_records()
+        self.collect_sums()
+        self.splice_sums()
+        self.collect_traits()
+        self.collect_impls()
+        self.collect_signatures()
+        self.splice_signatures()
+        self.collect_externs()
+        self.splice_externs()
+    }
+
+    /// Whether the definition `id` is the program's own, which the passes
+    /// collect (`own`).
+    pub fn own(self, id: Int) -> Bool { id < self.own_defs }
+
+    /// Make the item at `item_index`'s module the current one.
+    pub fn enter_item(mut self, item_index: Int) {
+        self.cur_module = self.module_of(item_index)
+        self.cur_file = self.label_of(self.cur_module)
+    }
+
+    /// The file label of module `m`.
+    pub fn label_of(self, m: Int) -> String {
+        if m >= 0 && m < self.labels.len() { self.labels.at(m) } else { "" }
+    }
+
+    // --- The resolution, as `Definitions`' methods read it ---
+
+    /// The module of the item at `item_index` (`module_of`).
+    pub fn module_of(self, item_index: Int) -> Int {
+        if item_index >= 0 && item_index < self.defs.item_module.len() {
+            self.defs.item_module.at(item_index)
+        } else {
+            0
+        }
+    }
+
+    /// What definition `id` is.
+    pub fn kind_of(self, id: Int) -> DefKind { self.defs.defs.at(id).kind }
+
+    /// Definition `id`'s name.
+    pub fn name_of(self, id: Int) -> String { self.defs.defs.at(id).name }
+
+    /// The type `name` in module `m`'s scope, or -1 (`resolve_type`).
+    pub fn resolve_type_in(self, m: Int, name: String) -> Int {
+        if m < 0 || m >= self.defs.modules.len() { return -1 }
+        match self.defs.modules.at(m).types.get(name) {
+            Some(id) => id + self.offset_of(m)
+            None => -1
+        }
+    }
+
+    /// The trait `name` in module `m`'s scope, or -1 (`resolve_trait`).
+    pub fn resolve_trait_in(self, m: Int, name: String) -> Int {
+        if m < 0 || m >= self.defs.modules.len() { return -1 }
+        match self.defs.modules.at(m).traits.get(name) {
+            Some(id) => id + self.offset_of(m)
+            None => -1
+        }
+    }
+
+    /// What the value `name` in module `m`'s scope resolves to
+    /// (`resolve_value`).
+    pub fn resolve_value_in(self, m: Int, name: String) -> Option<Res> {
+        if m < 0 || m >= self.defs.modules.len() { return None }
+        match self.defs.modules.at(m).values.get(name) {
+            Some(r) => Some(shift_res(r, self.offset_of(m)))
+            None => None
+        }
+    }
+
+    // What a lookup in module `m`'s scope adds to an id: std's scopes are
+    // the shared resolution's on the shared path.
+    pub fn offset_of(self, m: Int) -> Int {
+        if m >= self.defs.std_start { self.scope_offset } else { 0 }
+    }
+
+    /// Whether module `m` is one of std's (`is_std_module`).
+    pub fn is_std_module(self, m: Int) -> Bool {
+        m >= self.defs.std_start && m < self.defs.modules.len()
+    }
+
+    /// Whether a fix may edit module `m` (`owned`): the root package's or
+    /// a loose program's, and not std's.
+    pub fn owned(self, m: Int) -> Bool {
+        if self.is_std_module(m) || m < 0 || m >= self.defs.packages.len() { return false }
+        let p = self.defs.packages.at(m)
+        p == -1 || p == 0
+    }
+
+    /// The definitions of each kind, as `(id, item_index)`
+    /// (`functions`, `const_ids`).
+    pub fn defs_of_kind(self, want_const: Bool) -> Vec<(Int, Int)> {
+        let mut out: Vec<(Int, Int)> = Vec::new()
+        for i in 0..self.defs.defs.len() {
+            match self.defs.defs.at(i).kind {
+                DefKindFn(item) => if !want_const { out.push((i, item)) }
+                DefKindConst(item) => if want_const { out.push((i, item)) }
+                _ => ()
+            }
+        }
+        out
+    }
+
+    // --- Diagnostics ---
+
+    /// An error labelled "here" at `span` of the current file (`error`).
+    pub fn error(mut self, code: String, message: String, span: Span) {
+        self.diagnostics.push(error_at(code, message, self.cur_file, span, "here"))
+    }
+
+    /// An error labelled "here" at `span` of `file`.
+    pub fn error_in(mut self, code: String, message: String, file: String, span: Span) {
+        self.diagnostics.push(error_at(code, message, file, span, "here"))
+    }
+
+    /// E0092 at `span`, when `name` is exported by a std module the target
+    /// does not compile (`missing_on_target`); whether it reported one.
+    pub fn missing_on_target(mut self, name: String, span: Span) -> Bool {
+        let owner = match self.defs.missing.get(name) {
+            Some(m) => if m.starts_with("$std.") {
+                "std/${m.byte_slice(5, m.byte_len())}"
+            } else {
+                m
+            }
+            None => return false
+        }
+        let target = compile_target_name(self.defs.target)
+        self.diagnostics.push(
+            error_at(
+                "E0092",
+                "`${name}` is in `${owner}`, which target ${target} does not have",
+                self.cur_file,
+                span,
+                "not on target ${target}",
+            ),
+        )
+        true
+    }
+
+    /// E0900 for `what`, a plural phrase, at `span` (`unsupported`).
+    pub fn unsupported(mut self, span: Span, what: String) {
+        let mut d = error_at(
+            "E0900",
+            "${what} are not supported yet",
+            self.cur_file,
+            span,
+            "not supported yet",
+        )
+        d.add_note(MVP_NOTE)
+        self.diagnostics.push(d)
+    }
+
+    // --- Pass 2: the generic parameters ---
+
+    // E0076 for every generic parameter named `Self`, over every item's
+    // six kinds of generic list (`reject_self_type_params`).
+    fn reject_self_type_params(mut self) {
+        for i in 0..self.items.len() {
+            let file = self.label_of(self.module_of(i))
+            match self.items.at(i).value {
+                ItemFunction(f) => self.reject_self_generic_name(f.generics, file)
+                ItemRecord(r) => self.reject_self_generic_name(r.generics, file)
+                ItemType(t) => self.reject_self_generic_name(t.generics, file)
+                ItemTrait(t) => {
+                    self.reject_self_generic_name(t.generics, file)
+                    for ti in t.items.iter() {
+                        match ti {
+                            TraitItemRequired(sig) => self.reject_self_generic_name(
+                                sig.generics,
+                                file,
+                            )
+                            TraitItemProvided(f) => self.reject_self_generic_name(f.generics, file)
+                            TraitItemAssocType(_, _, _) => ()
+                        }
+                    }
+                }
+                ItemImpl(b) => {
+                    self.reject_self_generic_name(b.generics, file)
+                    for f in b.functions.iter() { self.reject_self_generic_name(f.generics, file) }
+                }
+                ItemExtern(b) => {
+                    for ei in b.items.iter() {
+                        match ei {
+                            ExternItemFn(sig) => self.reject_self_generic_name(sig.generics, file)
+                        }
+                    }
+                }
+                _ => ()
+            }
+        }
+    }
+
+    fn reject_self_generic_name(mut self, generics: Vec<TypeParam>, file: String) {
+        for g in generics.iter() {
+            if g.name.value == "Self" {
+                self.error_in(
+                    "E0076",
+                    "a generic parameter may not be named `Self`: in a trait `Self` is the implementing type, and in an impl it is that impl's self type",
+                    file,
+                    g.name.span,
+                )
+            }
+        }
+    }
+
+    /// E0403 for each generic parameter whose name repeats in one
+    /// declaration (`check_duplicate_generics`).
+    pub fn check_duplicate_generics(mut self, generics: Vec<TypeParam>, owner: String) {
+        let mut seen: Vec<String> = Vec::new()
+        for g in generics.iter() {
+            let name = g.name.value
+            if holds(seen.to_array(), name) {
+                self.error(
+                    "E0403",
+                    "the name `${name}` is already used for a generic parameter of this ${owner}",
+                    g.name.span,
+                )
+            } else {
+                seen.push(name)
+            }
+        }
+    }
+
+    // E0900 for a bound on a sum type's own parameter
+    // (`reject_type_param_bounds`).
+    fn reject_type_param_bounds(mut self, generics: Vec<TypeParam>, owner: String) {
+        for g in generics.iter() {
+            if g.bounds.is_empty() { continue }
+            let mut span = g.name.span
+            for b in g.bounds.iter() { span = span.merge(b.span) }
+            let mut d = error_at(
+                "E0900",
+                "trait bounds on ${owner} are not supported yet",
+                self.cur_file,
+                span,
+                "bound on the type parameter `${g.name.value}`",
+            )
+            d.add_note(
+                "this bound is not enforced, so it is rejected rather than silently ignored; write it on an `impl` block instead, where it is enforced",
+            )
+            self.diagnostics.push(d)
+        }
+    }
+
+    // --- Pass 3: arities, supertraits, records, sums ---
+
+    fn collect_records(mut self) {
+        for i in 0..self.own_defs {
+            let def = self.defs.defs.at(i)
+            let item_index = match def.kind {
+                DefKindRecord(item) => item
+                _ => continue
+            }
+            let decl = match self.items.at(item_index).value {
+                ItemRecord(r) => r
+                _ => continue
+            }
+            self.enter_item(item_index)
+            self.check_duplicate_generics(decl.generics, "type")
+            let generics = generic_scope(decl.generics)
+            let bounds = self.resolve_bounds(decl.generics)
+            self.expand_bounds(bounds)
+            let mut fields: Vec<HirField> = Vec::new()
+            for f in decl.fields.iter() {
+                fields.push(
+                    HirField { name: f.name.value, ty: self.convert_ty(f.ty, generics, bounds) },
+                )
+            }
+            self.records.push(
+                RecordType {
+                    def_id: i,
+                    name: def.name,
+                    generics: decl.generics.len(),
+                    fields: fields,
+                },
+            )
+        }
+    }
+
+    // Every record's and sum's arity, from the tree, before any type is
+    // converted (`collect_type_arities`).
+    fn collect_type_arities(mut self) {
+        for i in 0..self.own_defs {
+            let n = match self.defs.defs.at(i).kind {
+                DefKindRecord(item) => {
+                    match self.items.at(item).value {
+                        ItemRecord(r) => r.generics.len()
+                        _ => continue
+                    }
+                }
+                DefKindSum(item, _) => {
+                    match self.items.at(item).value {
+                        ItemType(t) => t.generics.len()
+                        _ => continue
+                    }
+                }
+                _ => continue
+            }
+            self.type_arity.set(i, n)
+        }
+    }
+
+    fn collect_sums(mut self) {
+        for i in 0..self.own_defs {
+            let def = self.defs.defs.at(i)
+            let item_index = match def.kind {
+                DefKindSum(item, _) => item
+                _ => continue
+            }
+            let decl = match self.items.at(item_index).value {
+                ItemType(t) => t
+                _ => continue
+            }
+            let variants = match decl.def {
+                TypeDefSum(vs) => vs
+                _ => continue
+            }
+            self.enter_item(item_index)
+            self.check_duplicate_generics(decl.generics, "type")
+            self.reject_type_param_bounds(decl.generics, "sum type parameters")
+            let generics = generic_scope(decl.generics)
+            let mut out: Vec<HirVariant> = Vec::new()
+            for v in variants.iter() {
+                let mut fields: Vec<Ty> = Vec::new()
+                for t in v.fields.iter() { fields.push(self.convert_ty(t, generics, Vec::new())) }
+                out.push(HirVariant { name: v.name.value, fields: fields })
+            }
+            self.sums.push(
+                SumType { def_id: i, name: def.name, generics: decl.generics.len(), variants: out },
+            )
+        }
+    }
+
+    // Every trait's declared supertraits, before any bound list is
+    // expanded (`collect_supertraits`), each once.
+    fn collect_supertraits(mut self) {
+        for trait_id in 0..self.own_defs {
+            let item_index = match self.defs.defs.at(trait_id).kind {
+                DefKindTrait(item) => item
+                _ => continue
+            }
+            if item_index >= self.items.len() { continue }
+            let decl = match self.items.at(item_index).value {
+                ItemTrait(t) => t
+                _ => continue
+            }
+            self.enter_item(item_index)
+            let mut ids: Vec<Int> = Vec::new()
+            for path in decl.supertraits.iter() {
+                let segs = path.value.segments
+                let name = if segs.is_empty() { "" } else { segs.last().value }
+                let id = self.resolve_trait_in(self.cur_module, name)
+                if id >= 0 {
+                    if !int_in(ids, id) { ids.push(id) }
+                } else if !self.missing_on_target(name, path.span) {
+                    self.error("E0001", "cannot find trait `${name}`", path.span)
+                    if !segs.is_empty() { self.name_fixes(segs.last(), NeedTrait, Vec::new()) }
+                }
+            }
+            self.supertraits.set(trait_id, Some(ids))
+        }
+    }
+
+    /// `bounds` and each one's transitive supertraits, each once, the
+    /// declared first in order (`with_supertraits`).
+    pub fn with_supertraits(self, bounds: Vec<Int>) -> Vec<Int> {
+        let mut out: Vec<Int> = Vec::new()
+        for id in bounds.iter() { if !int_in(out, id) { out.push(id) } }
+        let mut i = 0
+        while i < out.len() {
+            let id = out.at(i)
+            i += 1
+            let supers = if id >= 0 && id < self.supertraits.len() {
+                match self.supertraits.at(id) {
+                    Some(s) => s
+                    None => continue
+                }
+            } else {
+                continue
+            }
+            for s in supers.iter() { if !int_in(out, s) { out.push(s) } }
+        }
+        out
+    }
+
+    /// `with_supertraits` of every slot, in place (`expand_bounds`).
+    pub fn expand_bounds(self, mut bounds: Vec<Vec<Int>>) {
+        for k in 0..bounds.len() { bounds.set(k, self.with_supertraits(bounds.at(k))) }
+    }
+
+    // --- std's shared tables, spliced in (`splice`) ---
+
+    // The tables of std's shared check, when given.
+    fn std_tables(self) -> Option<Checker> {
+        match self.shared {
+            Some(s) => Some(s.tables)
+            None => None
+        }
+    }
+
+    fn splice_arities(mut self) {
+        let std = match self.std_tables() {
+            Some(t) => t
+            None => return
+        }
+        for id in 0..std.type_arity.len() {
+            if std.type_arity.at(id) != 0 {
+                self.type_arity.set(id + self.rebase_def, std.type_arity.at(id))
+            }
+        }
+    }
+
+    fn splice_supertraits(mut self) {
+        let std = match self.std_tables() {
+            Some(t) => t
+            None => return
+        }
+        for id in 0..std.supertraits.len() {
+            match std.supertraits.at(id) {
+                Some(ids) => self.supertraits.set(
+                    id + self.rebase_def,
+                    Some(rebase_ids(ids, self.rebase_def)),
+                )
+                None => ()
+            }
+        }
+    }
+
+    fn splice_records(mut self) {
+        let std = match self.std_tables() {
+            Some(t) => t
+            None => return
+        }
+        let r = self.rebase_def
+        for rec in std.records.iter() {
+            let mut fields: Vec<HirField> = Vec::new()
+            for f in rec.fields.iter() {
+                fields.push(HirField { name: f.name, ty: rebase_ty(f.ty, r) })
+            }
+            self.records.push(
+                RecordType {
+                    def_id: rec.def_id + r,
+                    name: rec.name,
+                    generics: rec.generics,
+                    fields: fields,
+                },
+            )
+        }
+    }
+
+    fn splice_sums(mut self) {
+        let std = match self.std_tables() {
+            Some(t) => t
+            None => return
+        }
+        let r = self.rebase_def
+        for s in std.sums.iter() {
+            let mut variants: Vec<HirVariant> = Vec::new()
+            for v in s.variants.iter() {
+                variants.push(HirVariant { name: v.name, fields: rebase_tys(v.fields, r) })
+            }
+            self.sums.push(
+                SumType {
+                    def_id: s.def_id + r,
+                    name: s.name,
+                    generics: s.generics,
+                    variants: variants,
+                },
+            )
+        }
+    }
+
+    /// std's traits, rebased (`Splice::Traits`).
+    pub fn splice_traits(mut self) {
+        let std = match self.std_tables() {
+            Some(t) => t
+            None => return
+        }
+        let r = self.rebase_def
+        for t in std.traits.iter() {
+            let mut methods: Vec<TraitMethod> = Vec::new()
+            for m in t.methods.iter() {
+                methods.push(
+                    TraitMethod {
+                        name: m.name,
+                        params: rebase_tys(m.params, r),
+                        ret: rebase_ty(m.ret, r),
+                        has_self: m.has_self,
+                        mut_self: m.mut_self,
+                        generics: m.generics,
+                        bounds: rebase_bounds(m.bounds, r),
+                        default_def: if m.default_def < 0 { -1 } else { m.default_def + r },
+                    },
+                )
+            }
+            let mut assoc: Vec<(String, Int)> = Vec::new()
+            for (n, d) in t.assoc_types.iter() { assoc.push((n, d + r)) }
+            self.traits.push(
+                TraitDef {
+                    def_id: t.def_id + r,
+                    name: t.name,
+                    supertraits: rebase_ids(t.supertraits, r),
+                    methods: methods,
+                    assoc_types: assoc,
+                },
+            )
+        }
+    }
+
+    /// std's methods whose bodies `trait_default` says, their signatures,
+    /// places and receivers (`splice_methods`).
+    pub fn splice_methods(mut self, trait_default: Bool) {
+        let std = match self.std_tables() {
+            Some(t) => t
+            None => return
+        }
+        let r = self.rebase_def
+        for id in 0..std.method_locs.len() {
+            let loc = match std.method_locs.at(id) {
+                Some(l) => l
+                None => continue
+            }
+            let is_default = match loc.owner {
+                MethodOwnerTraitDefault => true
+                MethodOwnerImpl => false
+            }
+            if is_default != trait_default { continue }
+            let at = id + r
+            match std.sigs.at(id) {
+                Some(sig) => self.sigs.set(at, Some(rebase_sig(sig, r)))
+                None => ()
+            }
+            self.method_locs.set(
+                at,
+                Some(
+                    MethodLoc {
+                        item_index: loc.item_index + self.rebase_item,
+                        method_index: loc.method_index,
+                        owner: loc.owner,
+                    },
+                ),
+            )
+            if std.selfless.at(id) { self.selfless.set(at, true) }
+            if std.mut_self.at(id) { self.mut_self.set(at, true) }
+        }
+    }
+
+    /// std's impls and their methods (`Splice::Impls`).
+    pub fn splice_impls(mut self) {
+        let std = match self.std_tables() {
+            Some(t) => t
+            None => return
+        }
+        self.splice_methods(false)
+        let r = self.rebase_def
+        for i in std.impls.iter() {
+            let mut methods: Vec<(String, Int)> = Vec::new()
+            for (n, d) in i.methods.iter() { methods.push((n, d + r)) }
+            let mut bindings: Vec<(Int, Ty)> = Vec::new()
+            for (d, t) in i.assoc_bindings.iter() { bindings.push((d + r, rebase_ty(t, r))) }
+            self.impls.push(
+                ImplInfo {
+                    trait_id: if i.trait_id < 0 { -1 } else { i.trait_id + r },
+                    self_head: rebase_head(i.self_head, r),
+                    self_ty: rebase_ty(i.self_ty, r),
+                    generics: i.generics,
+                    bounds: rebase_bounds(i.bounds, r),
+                    methods: methods,
+                    assoc_bindings: bindings,
+                },
+            )
+        }
+        for s in std.impl_spans.iter() { self.impl_spans.push(s) }
+        for s in std.impl_in_std.iter() { self.impl_in_std.push(s) }
+        for s in std.impl_method_spans.iter() { self.impl_method_spans.push(s) }
+        for s in std.impl_param_names.iter() { self.impl_param_names.push(s) }
+        for item in std.impl_selves.keys().iter() {
+            let s = std.impl_selves.get(item).unwrap()
+            let tid = if s.trait_id < 0 { -1 } else { s.trait_id + r }
+            let _ = self.impl_selves.insert(
+                item + self.rebase_item,
+                ImplSelf { ty: rebase_ty(s.ty, r), trait_id: tid },
+            )
+        }
+    }
+
+    fn splice_signatures(mut self) {
+        let std = match self.std_tables() {
+            Some(t) => t
+            None => return
+        }
+        let r = self.rebase_def
+        for id in 0..std.sigs.len() {
+            let sig = match std.sigs.at(id) {
+                Some(s) => s
+                None => continue
+            }
+            match self.kind_of(id + r) {
+                DefKindFn(_) => self.sigs.set(id + r, Some(rebase_sig(sig, r)))
+                DefKindConst(_) => self.sigs.set(id + r, Some(rebase_sig(sig, r)))
+                _ => ()
+            }
+        }
+    }
+
+    fn splice_externs(mut self) {
+        let std = match self.std_tables() {
+            Some(t) => t
+            None => return
+        }
+        let r = self.rebase_def
+        for id in 0..std.sigs.len() {
+            let sig = match std.sigs.at(id) {
+                Some(s) => s
+                None => continue
+            }
+            match self.kind_of(id + r) {
+                DefKindExternFn(_, _) => self.sigs.set(id + r, Some(rebase_sig(sig, r)))
+                _ => ()
+            }
+        }
+        for e in std.externs.iter() {
+            self.externs.push(
+                ExternFn {
+                    def_id: e.def_id + r,
+                    symbol: e.symbol,
+                    abi: e.abi,
+                    params: rebase_tys(e.params, r),
+                    ret: rebase_ty(e.ret, r),
+                    span: e.span,
+                },
+            )
+        }
+    }
+}
+
+// ---------------------------------------------------------------------------
+// Rebasing std's tables (`Rebase`)
+// ---------------------------------------------------------------------------
+
+/// `ty` with every definition's id `by` higher (`Rebase::ty`).
+pub fn rebase_ty(ty: Ty, by: Int) -> Ty {
+    match ty {
+        TyFn(params, ret) => TyFn(rebase_tys(params, by), rebase_ty(ret, by))
+        TyFuture(o) => TyFuture(rebase_ty(o, by))
+        TySum(d, args) => TySum(d + by, rebase_tys(args, by))
+        TyRecord(d, args) => TyRecord(d + by, rebase_tys(args, by))
+        TyArray(e) => TyArray(rebase_ty(e, by))
+        TyTuple(es) => TyTuple(rebase_tys(es, by))
+        TyAssoc(on, assoc) => TyAssoc(rebase_ty(on, by), assoc + by)
+        other => other
+    }
+}
+
+/// `rebase_ty` of each of `tys`.
+pub fn rebase_tys(tys: Vec<Ty>, by: Int) -> Vec<Ty> {
+    let mut out: Vec<Ty> = Vec::new()
+    for t in tys.iter() { out.push(rebase_ty(t, by)) }
+    out
+}
+
+fn rebase_ids(ids: Vec<Int>, by: Int) -> Vec<Int> {
+    let mut out: Vec<Int> = Vec::new()
+    for id in ids.iter() { out.push(id + by) }
+    out
+}
+
+fn rebase_bounds(bounds: Vec<Vec<Int>>, by: Int) -> Vec<Vec<Int>> {
+    let mut out: Vec<Vec<Int>> = Vec::new()
+    for b in bounds.iter() { out.push(rebase_ids(b, by)) }
+    out
+}
+
+fn rebase_head(h: TyHead, by: Int) -> TyHead {
+    match h {
+        TyHeadSum(d) => TyHeadSum(d + by)
+        TyHeadRecord(d) => TyHeadRecord(d + by)
+        other => other
+    }
+}
+
+fn rebase_sig(sig: FnSig, by: Int) -> FnSig {
+    FnSig {
+        generics: sig.generics,
+        bounds: rebase_bounds(sig.bounds, by),
+        params: rebase_tys(sig.params, by),
+        ret: rebase_ty(sig.ret, by),
+    }
+}
+
+/// `res` with its definition's id `by` higher.
+pub fn shift_res(res: Res, by: Int) -> Res {
+    match res {
+        ResDef(id) => ResDef(id + by)
+        ResVariant(sum, index) => ResVariant(sum + by, index)
+        ResBuiltin(b) => ResBuiltin(b)
+    }
+}
+
+// ---------------------------------------------------------------------------
+// Helpers
+// ---------------------------------------------------------------------------
+
+/// E0900's note for a feature not supported yet.
+pub const MVP_NOTE: String = "the Phase 1 MVP compiler supports a subset of Nova; this feature arrives in a later milestone"
+
+/// Each generic parameter's index by its name, a repeated name its last
+/// (`generic_scope`).
+pub fn generic_scope(generics: Vec<TypeParam>) -> Map<String, Int> {
+    let mut m: Map<String, Int> = Map::new()
+    for i in 0..generics.len() { let _ = m.insert(generics.at(i).name.value, i) }
+    m
+}
+
+/// The scope of a trait body: `Self` at 0 (`self_generic_scope`).
+pub fn self_generic_scope() -> Map<String, Int> {
+    let mut m: Map<String, Int> = Map::new()
+    let _ = m.insert("Self", 0)
+    m
+}
+
+/// A copy of `m`.
+pub fn copy_scope(m: Map<String, Int>) -> Map<String, Int> {
+    let mut out: Map<String, Int> = Map::new()
+    for k in m.keys().iter() { let _ = out.insert(k, m.get(k).unwrap()) }
+    out
+}
+
+/// A copy of `bounds`, each slot copied too, so that adding to one leaves
+/// the other as it was (`clone`).
+pub fn copy_bounds(bounds: Vec<Vec<Int>>) -> Vec<Vec<Int>> {
+    let mut out: Vec<Vec<Int>> = Vec::new()
+    for b in bounds.iter() {
+        let mut slot: Vec<Int> = Vec::new()
+        for id in b.iter() { slot.push(id) }
+        out.push(slot)
+    }
+    out
+}
+
+/// Whether `id` is in `ids`.
+pub fn int_in(ids: Vec<Int>, id: Int) -> Bool {
+    for x in ids.iter() { if x == id { return true } }
+    false
+}
+
+/// Set equality on two bound lists (`same_bound_set`).
+pub fn same_bound_set(a: Vec<Int>, b: Vec<Int>) -> Bool {
+    for d in a.iter() { if !int_in(b, d) { return false } }
+    for d in b.iter() { if !int_in(a, d) { return false } }
+    true
+}
+
+/// Whether some parameter is named `self`, and whether a `mut` one is.
+pub fn self_param(params: Vec<Param>, need_mut: Bool) -> Bool {
+    for p in params.iter() { if p.name.value == "self" && (!need_mut || p.is_mut) { return true } }
+    false
+}
+
+/// A type as a diagnostic shows it, with its definitions' names
+/// (`display_ty`).
+pub fn display_ty(ty: Ty, defs: Definitions) -> String {
+    match ty {
+        TyInt => "Int"
+        TyFloat => "Float"
+        TyBool => "Bool"
+        TyChar => "Char"
+        TyString => "String"
+        TyBytes => "Bytes"
+        TyUnit => "()"
+        TyFn(params, ret) => "fn(${join_tys(params, defs)}) -> ${display_ty(ret, defs)}"
+        TySum(d, args) => named_ty(defs.defs.at(d).name, args, defs)
+        TyRecord(d, args) => named_ty(defs.defs.at(d).name, args, defs)
+        TyArray(e) => "[${display_ty(e, defs)}]"
+        TyTuple(es) => {
+            let mut parts: Vec<String> = Vec::new()
+            for e in es.iter() { parts.push(display_ty(e, defs)) }
+            tuple_text(parts)
+        }
+        TyFuture(o) => "Future<${display_ty(o, defs)}>"
+        TyParam(i) => "T${i}"
+        TyAssoc(on, assoc) => "${display_ty(on, defs)}::${defs.defs.at(assoc).name}"
+        TyVar(v) => "?${v}"
+        TyNever => "!"
+        TyError => "{error}"
+    }
+}
+
+fn join_tys(tys: Vec<Ty>, defs: Definitions) -> String {
+    let mut parts: Vec<String> = Vec::new()
+    for t in tys.iter() { parts.push(display_ty(t, defs)) }
+    ", ".join(parts.to_array())
+}
+
+fn named_ty(name: String, args: Vec<Ty>, defs: Definitions) -> String {
+    if args.is_empty() { name } else { "${name}<${join_tys(args, defs)}>" }
+}
+
+/// A tuple type's text from its elements' texts, `(Int,)` for one
+/// (`tuple_text`).
+pub fn tuple_text(elems: Vec<String>) -> String {
+    if elems.len() == 1 { "(${elems.at(0)},)" } else { "(${", ".join(elems.to_array())})" }
+}
+
+/// `display_ty` with type parameter `i` shown as `names[i]`, and an
+/// unsolved variable or an error as `_` (`display_ty_named`).
+pub fn display_ty_named(ty: Ty, defs: Definitions, names: Vec<String>) -> String {
+    match ty {
+        TyFn(
+            params,
+            ret,
+        ) => "fn(${join_named(params, defs, names)}) -> ${display_ty_named(ret, defs, names)}"
+        TySum(d, args) => named_ty_named(defs.defs.at(d).name, args, defs, names)
+        TyRecord(d, args) => named_ty_named(defs.defs.at(d).name, args, defs, names)
+        TyArray(e) => "[${display_ty_named(e, defs, names)}]"
+        TyTuple(es) => {
+            let mut parts: Vec<String> = Vec::new()
+            for e in es.iter() { parts.push(display_ty_named(e, defs, names)) }
+            tuple_text(parts)
+        }
+        TyFuture(o) => "Future<${display_ty_named(o, defs, names)}>"
+        TyParam(i) => {
+            if i >= 0 && i < names.len() && names.at(i) != "" { names.at(i) } else { "T${i}" }
+        }
+        TyAssoc(on, assoc) => "${display_ty_named(on, defs, names)}::${defs.defs.at(assoc).name}"
+        TyVar(_) => "_"
+        TyError => "_"
+        other => display_ty(other, defs)
+    }
+}
+
+fn join_named(tys: Vec<Ty>, defs: Definitions, names: Vec<String>) -> String {
+    let mut parts: Vec<String> = Vec::new()
+    for t in tys.iter() { parts.push(display_ty_named(t, defs, names)) }
+    ", ".join(parts.to_array())
+}
+
+fn named_ty_named(name: String, args: Vec<Ty>, defs: Definitions, names: Vec<String>) -> String {
+    if args.is_empty() { name } else { "${name}<${join_named(args, defs, names)}>" }
+}
diff --git a/selfhost/src/typeck_traits.nova b/selfhost/src/typeck_traits.nova
new file mode 100644
index 00000000..6fe30d8c
--- /dev/null
+++ b/selfhost/src/typeck_traits.nova
@@ -0,0 +1,1094 @@
+// The type checker's traits and impls: a port of check.rs's
+// `collect_traits`, `collect_impls` and the impl checks it runs
+// (`check_impl_conformance`, `check_assoc_binding_cycles`,
+// `check_impl_method_signatures`, `check_impl_coherence`,
+// `check_hidden_trait_methods`, `check_supertrait_impls`,
+// `render_bound_set`), function by function (spec
+// docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md
+// §3.1, ADR 0055). typeck.nova gives the idioms.
+
+import ast
+import builtins
+import diagnostic
+import fixes
+import hir
+import resolver
+import token
+import typeck
+
+// A composite key `(item, method)` as one `Int`.
+fn item_method_key(item: Int, method: Int) -> Int { item * 1048576 + method }
+
+impl Checker {
+    // The methods owned by `trait_default`'s kind, by `(item, method)`:
+    // the default bodies' or the impls' method `DefId`s.
+    fn methods_by_place(self, trait_default: Bool) -> Map<Int, Int> {
+        let mut out: Map<Int, Int> = Map::new()
+        for i in 0..self.defs.defs.len() {
+            match self.defs.defs.at(i).kind {
+                DefKindMethod(item, method, owner) => {
+                    let is_default = match owner {
+                        MethodOwnerTraitDefault => true
+                        MethodOwnerImpl => false
+                    }
+                    if is_default == trait_default {
+                        let _ = out.insert(item_method_key(item, method), i)
+                    }
+                }
+                _ => ()
+            }
+        }
+        out
+    }
+
+    // The program's traits, as `(id, item_index)`.
+    fn own_traits(self) -> Vec<(Int, Int)> {
+        let mut out: Vec<(Int, Int)> = Vec::new()
+        for i in 0..self.own_defs {
+            match self.defs.defs.at(i).kind {
+                DefKindTrait(item) => out.push((i, item))
+                _ => ()
+            }
+        }
+        out
+    }
+
+    /// The traits and their methods' signatures, `Self` as `TyParam(0)`,
+    /// then the default bodies' signatures (`collect_traits`).
+    pub fn collect_traits(mut self) {
+        let default_defs = self.methods_by_place(true)
+        for (def_id, item_index) in self.own_traits().iter() {
+            let decl = match self.items.at(item_index).value {
+                ItemTrait(t) => t
+                _ => continue
+            }
+            self.enter_item(item_index)
+            if !decl.generics.is_empty() { self.unsupported(decl.name.span, "generic traits") }
+            if !decl.where_clause.is_empty() {
+                self.unsupported(decl.name.span, "`where` clauses on trait declarations")
+            }
+            let self_scope = self_generic_scope()
+            let mut methods: Vec<TraitMethod> = Vec::new()
+            // This trait's associated types' ids, in the order the resolver
+            // made them, drained one per associated type.
+            let assoc_ids = match self.assoc_of.get(def_id) {
+                Some(l) => l
+                None => Vec::new()
+            }
+            let mut next_assoc = 0
+            let mut assoc_types: Vec<(String, Int)> = Vec::new()
+            for mi in 0..decl.items.len() {
+                let item = decl.items.at(mi)
+                let mut name = Spanned::new("", Span::point(0))
+                let mut generics: Vec<TypeParam> = Vec::new()
+                let mut where_clause: Vec<WhereBound> = Vec::new()
+                let mut params: Vec<Param> = Vec::new()
+                let mut ret: Option<Spanned<Type>> = None
+                let mut is_default = false
+                let mut is_async = false
+                match item {
+                    TraitItemRequired(sig) => {
+                        name = sig.name
+                        generics = sig.generics
+                        where_clause = sig.where_clause
+                        params = sig.params
+                        ret = sig.return_ty
+                        is_async = sig.is_async
+                    }
+                    TraitItemProvided(f) => {
+                        name = f.name
+                        generics = f.generics
+                        where_clause = f.where_clause
+                        params = f.params
+                        ret = f.return_ty
+                        is_default = true
+                        is_async = f.is_async
+                    }
+                    TraitItemAssocType(_, aname, bounds) => {
+                        if !bounds.is_empty() {
+                            self.unsupported(aname.span, "trait bounds on an associated type")
+                        }
+                        let assoc_def_id = if next_assoc < assoc_ids.len() {
+                            assoc_ids.at(next_assoc)
+                        } else {
+                            -1
+                        }
+                        next_assoc += 1
+                        let mut dup = false
+                        for (n, _) in assoc_types.iter() { if n == aname.value { dup = true } }
+                        if dup {
+                            self.error(
+                                "E0403",
+                                "the name `${aname.value}` is already used for an associated type of this trait",
+                                aname.span,
+                            )
+                            continue
+                        }
+                        if assoc_def_id >= 0 { assoc_types.push((aname.value, assoc_def_id)) }
+                        continue
+                    }
+                }
+                let mut dup = false
+                for m in methods.iter() { if m.name == name.value { dup = true } }
+                if dup {
+                    self.error(
+                        "E0403",
+                        "the name `${name.value}` is already used for a method of this trait",
+                        name.span,
+                    )
+                    continue
+                }
+                if is_async { self.unsupported(name.span, "async methods") }
+                if !where_clause.is_empty() {
+                    self.error(
+                        "E0900",
+                        "`where` clauses on trait methods are not supported yet",
+                        name.span,
+                    )
+                }
+                self.check_duplicate_generics(generics, "method")
+                let mut m_scope = copy_scope(self_scope)
+                for j in 0..generics.len() {
+                    let _ = m_scope.insert(generics.at(j).name.value, 1 + j)
+                }
+                let mut sig_bounds: Vec<Vec<Int>> = Vec::new()
+                let mut first: Vec<Int> = Vec::new()
+                first.push(def_id)
+                sig_bounds.push(first)
+                for b in self.resolve_bounds(generics).iter() { sig_bounds.push(b) }
+                self.expand_bounds(sig_bounds)
+                let (m_params, m_ret) = self.method_sig_parts(params, ret, m_scope, sig_bounds)
+                let mut m_bounds: Vec<Vec<Int>> = Vec::new()
+                for k in 1..sig_bounds.len() { m_bounds.push(sig_bounds.at(k)) }
+                let mut default_def = -1
+                if is_default {
+                    match default_defs.get(item_method_key(item_index, mi)) {
+                        Some(d) => default_def = d
+                        None => ()
+                    }
+                }
+                methods.push(
+                    TraitMethod {
+                        name: name.value,
+                        params: m_params,
+                        ret: m_ret,
+                        has_self: self_param(params, false),
+                        mut_self: self_param(params, true),
+                        generics: generics.len(),
+                        bounds: m_bounds,
+                        default_def: default_def,
+                    },
+                )
+            }
+            let supers = match self.supertraits.at(def_id) {
+                Some(s) => s
+                None => Vec::new()
+            }
+            self.traits.push(
+                TraitDef {
+                    def_id: def_id,
+                    name: self.name_of(def_id),
+                    supertraits: supers,
+                    methods: methods,
+                    assoc_types: assoc_types,
+                },
+            )
+        }
+        self.splice_traits()
+        // The default bodies' signatures: generic over `Self`, bounded by
+        // the trait.
+        for (trait_id, item_index) in self.own_traits().iter() {
+            let decl = match self.items.at(item_index).value {
+                ItemTrait(t) => t
+                _ => continue
+            }
+            self.enter_item(item_index)
+            let self_scope = self_generic_scope()
+            for mi in 0..decl.items.len() {
+                let f = match decl.items.at(mi) {
+                    TraitItemProvided(f) => f
+                    _ => continue
+                }
+                let def_id = match default_defs.get(item_method_key(item_index, mi)) {
+                    Some(d) => d
+                    None => continue
+                }
+                if f.is_async { continue }
+                if !f.where_clause.is_empty() { continue }
+                let mut scope = copy_scope(self_scope)
+                for j in 0..f.generics.len() {
+                    let _ = scope.insert(f.generics.at(j).name.value, 1 + j)
+                }
+                let mut bounds: Vec<Vec<Int>> = Vec::new()
+                let mut first: Vec<Int> = Vec::new()
+                first.push(trait_id)
+                bounds.push(first)
+                for b in self.resolve_bounds(f.generics).iter() { bounds.push(b) }
+                self.expand_bounds(bounds)
+                let (ps, ret) = self.method_sig_parts(f.params, f.return_ty, scope, bounds)
+                let mut params: Vec<Ty> = Vec::new()
+                if self_param(f.params, false) { params.push(TyParam(0)) }
+                for p in ps.iter() { params.push(p) }
+                self.sigs.set(
+                    def_id,
+                    Some(
+                        FnSig {
+                            generics: 1 + f.generics.len(),
+                            bounds: bounds,
+                            params: params,
+                            ret: ret,
+                        },
+                    ),
+                )
+                self.method_locs.set(
+                    def_id,
+                    Some(
+                        MethodLoc {
+                            item_index: item_index,
+                            method_index: mi,
+                            owner: MethodOwnerTraitDefault,
+                        },
+                    ),
+                )
+            }
+        }
+        self.splice_methods(true)
+    }
+
+    /// The impl blocks into the impl table and their methods' signatures,
+    /// then the checks that need the whole table (`collect_impls`).
+    pub fn collect_impls(mut self) {
+        let impl_methods = self.methods_by_place(false)
+        let mut impl_spans: Vec<FileSpan> = Vec::new()
+        let mut impl_in_std: Vec<Bool> = Vec::new()
+        let mut impl_method_spans: Vec<Vec<FileSpan>> = Vec::new()
+        let mut impl_param_names: Vec<Vec<String>> = Vec::new()
+        for item_index in 0..self.items.len() {
+            let block = match self.items.at(item_index).value {
+                ItemImpl(b) => b
+                _ => continue
+            }
+            self.enter_item(item_index)
+            self.impl_self = None
+            self.check_duplicate_generics(block.generics, "impl")
+            let impl_generics = generic_scope(block.generics)
+            let impl_bounds = self.resolve_bounds(block.generics)
+            self.apply_where(impl_bounds, block.where_clause, impl_generics)
+            self.expand_bounds(impl_bounds)
+            let self_ty = self.convert_ty(block.ty, impl_generics, impl_bounds)
+            if has_assoc(self_ty) {
+                let tystr = display_ty(self_ty, self.defs)
+                let mut d = error_at(
+                    "E0900",
+                    "an associated-type projection in an impl's self type (`${tystr}`) is not supported yet",
+                    self.cur_file,
+                    block.ty.span,
+                    "not supported yet",
+                )
+                d.add_note(
+                    "an impl's type arguments are recovered by matching its self type against a concrete one, which cannot invert a projection, so this impl could never be selected — and overlap checking cannot see through the projection either, so it would not conflict with an impl that does apply",
+                )
+                d.add_note(MVP_NOTE)
+                self.diagnostics.push(d)
+                continue
+            }
+            let self_head = match head(self_ty) {
+                Some(h) => h
+                None => {
+                    self.error(
+                        "E0010",
+                        "impl blocks are only supported on named types",
+                        block.ty.span,
+                    )
+                    continue
+                }
+            }
+            let mut has_unused = false
+            for i in 0..block.generics.len() {
+                if !mentions_param(self_ty, i) {
+                    has_unused = true
+                    let g = block.generics.at(i)
+                    self.error(
+                        "E0073",
+                        "impl type parameter `${g.name.value}` is not used in the self type `${display_ty(self_ty, self.defs)}`",
+                        g.name.span,
+                    )
+                }
+            }
+            if has_unused { continue }
+            let mut trait_id = -1
+            match block.trait_ {
+                Some(tr) => {
+                    let segs = tr.value.segments
+                    let name = if segs.is_empty() { "" } else { segs.last().value }
+                    let id = self.resolve_trait_in(self.cur_module, name)
+                    if id >= 0 {
+                        trait_id = id
+                    } else {
+                        if !self.missing_on_target(name, tr.span) {
+                            self.error("E0001", "cannot find trait `${name}`", tr.span)
+                            if !segs.is_empty() {
+                                self.name_fixes(segs.last(), NeedTrait, Vec::new())
+                            }
+                        }
+                        continue
+                    }
+                }
+                None => ()
+            }
+            if trait_id >= 0 && is_array_head(self_head) {
+                let mut d = error_at(
+                    "E0010",
+                    "a trait cannot be implemented for an array",
+                    self.cur_file,
+                    block.ty.span,
+                    "an array type",
+                )
+                d.add_note(
+                    "an array takes inherent impls only, `impl<T> [T] { … }` or `impl [Int] { … }`; wrap the array in a record to implement a trait for it",
+                )
+                self.diagnostics.push(d)
+                continue
+            }
+            let imp_self = ImplSelf { ty: self_ty, trait_id: trait_id }
+            let _ = self.impl_selves.insert(item_index, imp_self)
+            self.impl_self = Some(imp_self)
+            let impl_count = block.generics.len()
+            let mut assoc_bindings: Vec<(Int, Ty)> = Vec::new()
+            let mut assoc_spans: Vec<Span> = Vec::new()
+            for c in block.consts.iter() {
+                self.unsupported(c.name.span, "associated constants like `${c.name.value}`")
+            }
+            for b in block.assoc_types.iter() {
+                let ty = self.convert_ty(b.ty, impl_generics, impl_bounds)
+                let assoc = if trait_id >= 0 {
+                    self.find_assoc_type(trait_id, b.name.value)
+                } else {
+                    -1
+                }
+                if assoc < 0 {
+                    if trait_id < 0 {
+                        self.error(
+                            "E0071",
+                            "associated type `${b.name.value}` cannot be bound by an inherent impl, which implements no trait",
+                            b.name.span,
+                        )
+                    }
+                    continue
+                }
+                let mut dup = false
+                for (d, _) in assoc_bindings.iter() { if d == assoc { dup = true } }
+                if dup {
+                    self.error(
+                        "E0403",
+                        "the associated type `${b.name.value}` is already bound by this impl",
+                        b.name.span,
+                    )
+                    continue
+                }
+                assoc_bindings.push((assoc, ty))
+                assoc_spans.push(b.name.span)
+            }
+            self.check_assoc_binding_cycles(assoc_bindings, assoc_spans, self_ty)
+            let mut methods: Vec<(String, Int)> = Vec::new()
+            let mut method_spans: Vec<FileSpan> = Vec::new()
+            for mi in 0..block.functions.len() {
+                let f = block.functions.at(mi)
+                let def_id = match impl_methods.get(item_method_key(item_index, mi)) {
+                    Some(d) => d
+                    None => continue
+                }
+                let mut scope = copy_scope(impl_generics)
+                let mut bounds = copy_bounds(impl_bounds)
+                let mut seen: Vec<String> = Vec::new()
+                for j in 0..f.generics.len() {
+                    let g = f.generics.at(j)
+                    let gname = g.name.value
+                    if impl_generics.contains_key(gname) {
+                        self.error(
+                            "E0403",
+                            "the name `${gname}` shadows the impl's generic parameter",
+                            g.name.span,
+                        )
+                    } else if holds(seen.to_array(), gname) {
+                        self.error(
+                            "E0403",
+                            "the name `${gname}` is already used for a generic parameter of this method",
+                            g.name.span,
+                        )
+                    }
+                    seen.push(gname)
+                    let _ = scope.insert(gname, impl_count + j)
+                }
+                for b in self.resolve_bounds(f.generics).iter() { bounds.push(b) }
+                let method_generic_count = f.generics.len()
+                self.apply_where(bounds, f.where_clause, scope)
+                self.expand_bounds(bounds)
+                let (ps, r) = self.method_sig_parts(f.params, f.return_ty, scope, bounds)
+                let ret = if f.is_async { TyFuture(r) } else { r }
+                let mut params: Vec<Ty> = Vec::new()
+                if self_param(f.params, false) {
+                    params.push(self_ty)
+                } else {
+                    self.selfless.set(def_id, true)
+                }
+                for p in ps.iter() { params.push(p) }
+                if self_param(f.params, true) { self.mut_self.set(def_id, true) }
+                self.sigs.set(
+                    def_id,
+                    Some(
+                        FnSig {
+                            generics: impl_count + method_generic_count,
+                            bounds: bounds,
+                            params: params,
+                            ret: ret,
+                        },
+                    ),
+                )
+                self.method_locs.set(
+                    def_id,
+                    Some(
+                        MethodLoc {
+                            item_index: item_index,
+                            method_index: mi,
+                            owner: MethodOwnerImpl,
+                        },
+                    ),
+                )
+                methods.push((f.name.value, def_id))
+                method_spans.push(FileSpan { file: self.cur_file, span: f.name.span })
+            }
+            if trait_id >= 0 {
+                self.check_impl_conformance(trait_id, methods, block.assoc_types, block.ty.span)
+            }
+            self.impls.push(
+                ImplInfo {
+                    trait_id: trait_id,
+                    self_head: self_head,
+                    self_ty: self_ty,
+                    generics: block.generics.len(),
+                    bounds: impl_bounds,
+                    methods: methods,
+                    assoc_bindings: assoc_bindings,
+                },
+            )
+            impl_spans.push(FileSpan { file: self.cur_file, span: block.ty.span })
+            impl_in_std.push(self.is_std_module(self.cur_module))
+            impl_method_spans.push(method_spans)
+            let mut names: Vec<String> = Vec::new()
+            for g in block.generics.iter() { names.push(g.name.value) }
+            impl_param_names.push(names)
+        }
+        self.impl_self = None
+        self.impl_spans = impl_spans
+        self.impl_in_std = impl_in_std
+        self.impl_method_spans = impl_method_spans
+        self.impl_param_names = impl_param_names
+        self.splice_impls()
+        self.check_impl_method_signatures()
+        self.check_impl_coherence()
+        self.check_hidden_trait_methods()
+        self.check_supertrait_impls()
+    }
+
+    // The trait whose id is `tid` in the table.
+    fn trait_by_id(self, tid: Int) -> Option<TraitDef> {
+        for t in self.traits.iter() { if t.def_id == tid { return Some(t) } }
+        None
+    }
+
+    // E0105, a warning: a program's trait impl's method that one of std's
+    // inherent impls on an overlapping type also defines, which a call
+    // reaches first (`check_hidden_trait_methods`).
+    fn check_hidden_trait_methods(mut self) {
+        let mut found: Vec<Diagnostic> = Vec::new()
+        let spans = self.impl_spans
+        let in_std = self.impl_in_std
+        for i in 0..self.impls.len() {
+            let a = self.impls.at(i)
+            if a.trait_id < 0 { continue }
+            if i >= in_std.len() || in_std.at(i) { continue }
+            let tr = match self.trait_by_id(a.trait_id) {
+                Some(t) => t
+                None => continue
+            }
+            let mut names: Vec<(String, FileSpan, Bool)> = Vec::new()
+            for k in 0..a.methods.len() {
+                let (n, d) = a.methods.at(k)
+                let span = if i < self.impl_method_spans.len()
+                    && k < self.impl_method_spans.at(i).len() {
+                    self.impl_method_spans.at(i).at(k)
+                } else {
+                    spans.at(i)
+                }
+                names.push((n, span, !self.selfless.at(d)))
+            }
+            for m in tr.methods.iter() {
+                let mut overridden = false
+                for (n, _) in a.methods.iter() { if n == m.name { overridden = true } }
+                if m.default_def >= 0 && !overridden {
+                    names.push((m.name, spans.at(i), m.has_self))
+                }
+            }
+            let own: Vec<String> = if i < self.impl_param_names.len() {
+                self.impl_param_names.at(i)
+            } else {
+                Vec::new()
+            }
+            for j in 0..self.impls.len() {
+                let b = self.impls.at(j)
+                if b.trait_id >= 0 || !(j < in_std.len() && in_std.at(j)) { continue }
+                if !head_eq(a.self_head, b.self_head)
+                    || !self_types_overlap(a.self_ty, a.generics, b.self_ty, b.generics) {
+                    continue
+                }
+                for (name, span, has_self) in names.iter() {
+                    let mut std_def = -1
+                    for (n, d) in b.methods.iter() { if n == name && std_def < 0 { std_def = d } }
+                    if std_def < 0 { continue }
+                    let std_has_self = !self.selfless.at(std_def)
+                    if std_has_self != has_self { continue }
+                    let ty = display_ty_named(a.self_ty, self.defs, own)
+                    let declared = match self.sigs.at(std_def) {
+                        Some(s) => s.params.len()
+                        None => 0
+                    }
+                    let args = if declared > (if std_has_self { 1 } else { 0 }) {
+                        "(…)"
+                    } else {
+                        "()"
+                    }
+                    let trait_name = tr.name
+                    let mut trait_extra = 0
+                    let mut met = false
+                    for m in tr.methods.iter() {
+                        if !met && m.name == name {
+                            trait_extra = m.params.len()
+                            met = true
+                        }
+                    }
+                    let mut trait_args = "()"
+                    let mut path_args = "()"
+                    if has_self {
+                        if trait_extra == 0 {
+                            trait_args = "()"
+                            path_args = "(x)"
+                        } else {
+                            trait_args = "(…)"
+                            path_args = "(x, …)"
+                        }
+                    } else if trait_extra != 0 {
+                        trait_args = "(…)"
+                        path_args = "(…)"
+                    }
+                    let param = fresh_param_name(own)
+                    if has_self {
+                        let mut d = Diagnostic::warning(
+                            "E0105",
+                            "std's inherent method `${name}` on `${ty}` hides `${trait_name}::${name}`",
+                        )
+                        d.add_primary(
+                            span.file,
+                            span.span,
+                            "`x.${name}${args}` on a `${ty}` runs std's method, not this one",
+                        )
+                        d.add_note(
+                            "a method call takes an inherent method before a trait's; Nova has no `${trait_name}::${name}${path_args}` call, so reach this one through a function generic over `${param}: ${trait_name}`, whose `x.${name}${trait_args}` is the trait's, or rename it",
+                        )
+                        found.push(d)
+                    } else {
+                        let head_name = ty.split("<")[0]
+                        let mut d = Diagnostic::warning(
+                            "E0105",
+                            "std's inherent function `${name}` on `${ty}` hides `${trait_name}::${name}`",
+                        )
+                        d.add_primary(
+                            span.file,
+                            span.span,
+                            "`${head_name}::${name}${args}` runs std's function, not this one",
+                        )
+                        d.add_note(
+                            "a path takes an inherent function before a trait's, so `${head_name}::${name}${args}` is std's; reach this one through a function generic over `${param}: ${trait_name}`, whose `${param}::${name}${path_args}` is the trait's, or rename it",
+                        )
+                        found.push(d)
+                    }
+                }
+            }
+        }
+        for d in found.iter() { self.diagnostics.push(d) }
+    }
+
+    // E0077 for an associated type this impl binds in terms of itself, the
+    // binding poisoned to `TyError` (`check_assoc_binding_cycles`).
+    fn check_assoc_binding_cycles(
+        mut self,
+        mut bindings: Vec<(Int, Ty)>,
+        spans: Vec<Span>,
+        self_ty: Ty,
+    ) {
+        let mut edges: Map<Int, Vec<Int>> = Map::new()
+        for (d, ty) in bindings.iter() {
+            let refs: Vec<Int> = Vec::new()
+            collect_self_projections(ty, self_ty, refs)
+            let _ = edges.insert(d, refs)
+        }
+        let mut k = 0
+        while k < bindings.len() && k < spans.len() {
+            let (d, _) = bindings.at(k)
+            let span = spans.at(k)
+            k += 1
+            if !reaches_self(d, edges) { continue }
+            let name = self.name_of(d)
+            bindings.set(k - 1, (d, TyError))
+            let mut diag = error_at(
+                "E0077",
+                "the associated type `${name}` is defined in terms of itself",
+                self.cur_file,
+                span,
+                "cyclic associated type",
+            )
+            diag.add_note(
+                "an associated type's binding may name another of this impl's associated types, but the references must bottom out in a concrete type",
+            )
+            self.diagnostics.push(diag)
+        }
+    }
+
+    // E0072 for each trait impl whose self type lacks an impl of one of
+    // the trait's direct supertraits (`check_supertrait_impls`).
+    fn check_supertrait_impls(mut self) {
+        let mut errors: Vec<(String, FileSpan)> = Vec::new()
+        let mut i = 0
+        while i < self.impls.len() && i < self.impl_spans.len() {
+            let imp = self.impls.at(i)
+            let span = self.impl_spans.at(i)
+            i += 1
+            if imp.trait_id < 0 { continue }
+            let tr = match self.trait_by_id(imp.trait_id) {
+                Some(t) => t
+                None => continue
+            }
+            for super_id in tr.supertraits.iter() {
+                let mut satisfied = false
+                for other in self.impls.iter() {
+                    if !satisfied
+                        && other.trait_id == super_id
+                        && match_args(other, imp.self_ty).is_some() {
+                        satisfied = true
+                    }
+                }
+                if satisfied { continue }
+                let sname = match self.trait_by_id(super_id) {
+                    Some(t) => t.name
+                    None => "?"
+                }
+                errors.push(
+                    (
+                        "the trait `${tr.name}` requires `${sname}`, which `${display_ty(imp.self_ty, self.defs)}` does not implement",
+                        span,
+                    ),
+                )
+            }
+        }
+        for (msg, span) in errors.iter() { self.error_in("E0072", msg, span.file, span.span) }
+    }
+
+    // E0074 for overlapping impls: two of one trait on types that share an
+    // instance, two inherent ones defining one method, and a program's
+    // `len` on arrays (`check_impl_coherence`).
+    fn check_impl_coherence(mut self) {
+        let mut conflicts: Vec<Diagnostic> = Vec::new()
+        let spans = self.impl_spans
+        let in_std = self.impl_in_std
+        for i in 0..self.impls.len() {
+            let a = self.impls.at(i)
+            let mut j = i + 1
+            while j < self.impls.len() && j < spans.len() {
+                let b = self.impls.at(j)
+                let b_span = spans.at(j)
+                j += 1
+                if !head_eq(a.self_head, b.self_head) || a.trait_id != b.trait_id { continue }
+                if !self_types_overlap(a.self_ty, a.generics, b.self_ty, b.generics) { continue }
+                if a.trait_id >= 0 {
+                    let tname = match self.trait_by_id(a.trait_id) {
+                        Some(t) => t.name
+                        None => "?"
+                    }
+                    let mut d = Diagnostic::error(
+                        "E0074",
+                        "conflicting implementations of trait `${tname}` for overlapping types",
+                    )
+                    d.add_primary(b_span.file, b_span.span, "here")
+                    conflicts.push(d)
+                } else {
+                    let mut dup = ""
+                    let mut has_dup = false
+                    for (n, _) in a.methods.iter() {
+                        if !has_dup {
+                            for (m, _) in b.methods.iter() { if m == n { has_dup = true } }
+                            if has_dup { dup = n }
+                        }
+                    }
+                    if has_dup {
+                        let jj = j - 1
+                        let std_b = (jj < in_std.len() && in_std.at(jj))
+                            && !(i < in_std.len() && in_std.at(i))
+                        let a_known = i < spans.len()
+                        let mut d = Diagnostic::error(
+                            "E0074",
+                            "method `${dup}` is defined by multiple overlapping inherent impls",
+                        )
+                        if a_known && std_b {
+                            let a_span = spans.at(i)
+                            d.add_primary(a_span.file, a_span.span, "here")
+                            d.add_secondary(
+                                b_span.file,
+                                b_span.span,
+                                "`${dup}` is also defined by std's impl",
+                            )
+                        } else {
+                            d.add_primary(b_span.file, b_span.span, "here")
+                            if a_known {
+                                let a_span = spans.at(i)
+                                d.add_secondary(
+                                    a_span.file,
+                                    a_span.span,
+                                    "`${dup}` is also defined by this impl",
+                                )
+                            }
+                        }
+                        conflicts.push(d)
+                    }
+                }
+            }
+        }
+        let mut std_arrays: Option<FileSpan> = None
+        let mut k = 0
+        while k < self.impls.len() && k < spans.len() && k < in_std.len() {
+            let imp = self.impls.at(k)
+            if std_arrays.is_none()
+                && in_std.at(k)
+                && imp.trait_id < 0
+                && is_array_head(imp.self_head) {
+                std_arrays = Some(spans.at(k))
+            }
+            k += 1
+        }
+        k = 0
+        while k < self.impls.len() && k < spans.len() && k < in_std.len() {
+            let imp = self.impls.at(k)
+            let span = spans.at(k)
+            let std = in_std.at(k)
+            k += 1
+            if std { continue }
+            let mut has_len = false
+            for (n, _) in imp.methods.iter() { if n == "len" { has_len = true } }
+            if imp.trait_id >= 0 || !is_array_head(imp.self_head) || !has_len { continue }
+            let mut d = Diagnostic::error(
+                "E0074",
+                "method `len` is defined by multiple overlapping inherent impls",
+            )
+            d.add_primary(span.file, span.span, "here")
+            match std_arrays {
+                Some(s) => d.add_secondary(
+                    s.file,
+                    s.span,
+                    "`len` is also defined by std, for every array",
+                )
+                None => ()
+            }
+            conflicts.push(d)
+        }
+        for d in conflicts.iter() { self.diagnostics.push(d) }
+    }
+
+    // A set of bounds for a diagnostic, `` `Show + Ord` ``, or `(none)`
+    // (`render_bound_set`).
+    fn render_bound_set(self, bounds: Vec<Int>) -> String {
+        if bounds.is_empty() { return "(none)" }
+        let mut names: Vec<String> = Vec::new()
+        for d in bounds.iter() {
+            match self.trait_by_id(d) {
+                Some(t) => names.push(t.name)
+                None => names.push("DefId(${d})")
+            }
+        }
+        "`${" + ".join(names.to_array())}`"
+    }
+
+    // E0070 and E0071: a trait impl must provide exactly the trait's items
+    // that lack a default, and nothing foreign (`check_impl_conformance`).
+    fn check_impl_conformance(
+        mut self,
+        trait_id: Int,
+        provided: Vec<(String, Int)>,
+        provided_assoc: Vec<AssocTypeBinding>,
+        span: Span,
+    ) {
+        let tr = match self.trait_by_id(trait_id) {
+            Some(t) => t
+            None => return
+        }
+        for (name, _) in provided.iter() {
+            let mut member = false
+            for m in tr.methods.iter() { if m.name == name { member = true } }
+            if !member {
+                self.error("E0071", "method `${name}` is not a member of trait `${tr.name}`", span)
+            }
+        }
+        for b in provided_assoc.iter() {
+            let mut member = false
+            for (n, _) in tr.assoc_types.iter() { if n == b.name.value { member = true } }
+            if !member {
+                self.error(
+                    "E0071",
+                    "associated type `${b.name.value}` is not a member of trait `${tr.name}`",
+                    b.name.span,
+                )
+            }
+        }
+        let mut missing: Vec<String> = Vec::new()
+        for m in tr.methods.iter() {
+            let mut given = false
+            for (n, _) in provided.iter() { if n == m.name { given = true } }
+            if m.default_def < 0 && !given { missing.push("`${m.name}`") }
+        }
+        let mut missing_assoc: Vec<String> = Vec::new()
+        for (n, _) in tr.assoc_types.iter() {
+            let mut given = false
+            for b in provided_assoc.iter() { if b.name.value == n { given = true } }
+            if !given { missing_assoc.push("`${n}`") }
+        }
+        let mut kinds: Vec<String> = Vec::new()
+        if !missing.is_empty() { kinds.push("method(s): ${", ".join(missing.to_array())}") }
+        if !missing_assoc.is_empty() {
+            kinds.push("associated type(s): ${", ".join(missing_assoc.to_array())}")
+        }
+        if !kinds.is_empty() {
+            self.error(
+                "E0070",
+                "impl of trait `${tr.name}` is missing ${" and ".join(kinds.to_array())}",
+                span,
+            )
+        }
+    }
+
+    // E0072: each impl method's signature against the trait's, `Self` the
+    // impl's self type and both sides normalized, once the whole table is
+    // built (`check_impl_method_signatures`).
+    fn check_impl_method_signatures(mut self) {
+        let impls = self.impls
+        let spans = self.impl_spans
+        let mut i = 0
+        while i < impls.len() && i < spans.len() {
+            let imp = impls.at(i)
+            let place = spans.at(i)
+            i += 1
+            if imp.trait_id < 0 { continue }
+            let tr = match self.trait_by_id(imp.trait_id) {
+                Some(t) => t
+                None => continue
+            }
+            for (name, def_id) in imp.methods.iter() {
+                let mut found: Option<TraitMethod> = None
+                for m in tr.methods.iter() {
+                    if found.is_none() && m.name == name { found = Some(m) }
+                }
+                let trait_method = match found {
+                    Some(m) => m
+                    None => continue
+                }
+                let impl_sig = match self.sigs.at(def_id) {
+                    Some(s) => s
+                    None => continue
+                }
+                self.check_impl_method(imp, tr, name, def_id, trait_method, impl_sig, place)
+            }
+        }
+    }
+
+    // One method of `check_impl_method_signatures`.
+    fn check_impl_method(
+        mut self,
+        imp: ImplInfo,
+        tr: TraitDef,
+        name: String,
+        def_id: Int,
+        trait_method: TraitMethod,
+        impl_sig: FnSig,
+        place: FileSpan,
+    ) {
+        let file = place.file
+        let span = place.span
+        let impl_count = imp.generics
+        let impl_has_self = !self.selfless.at(def_id)
+        if impl_has_self != trait_method.has_self {
+            let want = if trait_method.has_self {
+                "a `self` receiver"
+            } else {
+                "no `self` receiver"
+            }
+            let got = if trait_method.has_self { "none" } else { "one" }
+            self.error_in(
+                "E0072",
+                "method `${name}` has ${got} but trait `${tr.name}` declares ${want}",
+                file,
+                span,
+            )
+            return
+        }
+        let impl_mut_self = self.mut_self.at(def_id)
+        if impl_mut_self != trait_method.mut_self {
+            let want = if trait_method.mut_self {
+                "a `mut self` receiver"
+            } else {
+                "a plain `self` receiver"
+            }
+            let got = if trait_method.mut_self {
+                "a plain `self` receiver"
+            } else {
+                "a `mut self` receiver"
+            }
+            self.error_in(
+                "E0072",
+                "method `${name}` has ${got} but trait `${tr.name}` declares ${want}",
+                file,
+                span,
+            )
+        }
+        let impl_method_generics = if impl_sig.generics > impl_count {
+            impl_sig.generics - impl_count
+        } else {
+            0
+        }
+        if impl_method_generics != trait_method.generics {
+            self.error_in(
+                "E0072",
+                "method `${name}` has ${impl_method_generics} generic parameter(s) but trait `${tr.name}` declares ${trait_method.generics}",
+                file,
+                span,
+            )
+            return
+        }
+        for k in 0..trait_method.generics {
+            let want = trait_method.bounds.at(k)
+            let got: Vec<Int> = if impl_count + k < impl_sig.bounds.len() {
+                impl_sig.bounds.at(impl_count + k)
+            } else {
+                Vec::new()
+            }
+            if !same_bound_set(want, got) {
+                let want_str = self.render_bound_set(want)
+                let got_str = self.render_bound_set(got)
+                self.error_in(
+                    "E0072",
+                    "generic parameter ${k + 1} of method `${name}` has bound(s) ${got_str} but trait `${tr.name}` declares ${want_str}",
+                    file,
+                    span,
+                )
+            }
+        }
+        let mut s: Vec<Ty> = Vec::new()
+        s.push(imp.self_ty)
+        for k in 0..trait_method.generics { s.push(TyParam(impl_count + k)) }
+        let mut impl_params: Vec<Ty> = Vec::new()
+        if self.selfless.at(def_id) {
+            impl_params = impl_sig.params
+        } else {
+            for k in 1..impl_sig.params.len() { impl_params.push(impl_sig.params.at(k)) }
+        }
+        if impl_params.len() != trait_method.params.len() {
+            self.error_in(
+                "E0072",
+                "method `${name}` has ${impl_params.len()} parameter(s) but trait `${tr.name}` declares ${trait_method.params.len()}",
+                file,
+                span,
+            )
+            return
+        }
+        let mut expected: Vec<Ty> = Vec::new()
+        for p in trait_method.params.iter() {
+            expected.push(self.normalize(subst(p, s), file, span))
+        }
+        let mut got_params: Vec<Ty> = Vec::new()
+        for p in impl_params.iter() { got_params.push(self.normalize(p, file, span)) }
+        for k in 0..got_params.len() {
+            let got = got_params.at(k)
+            let want = expected.at(k)
+            if has_error(got) || has_error(want) { continue }
+            if !ty_eq(got, want) {
+                self.error_in(
+                    "E0072",
+                    "parameter ${k + 1} of method `${name}` has type `${display_ty(got, self.defs)}` but trait `${tr.name}` declares `${display_ty(want, self.defs)}`",
+                    file,
+                    span,
+                )
+            }
+        }
+        let expected_ret = self.normalize(subst(trait_method.ret, s), file, span)
+        let impl_ret = self.normalize(impl_sig.ret, file, span)
+        if has_error(impl_ret) || has_error(expected_ret) { return }
+        if !ty_eq(impl_ret, expected_ret) {
+            self.error_in(
+                "E0072",
+                "method `${name}` returns `${display_ty(impl_ret, self.defs)}` but trait `${tr.name}` declares `${display_ty(expected_ret, self.defs)}`",
+                file,
+                span,
+            )
+        }
+    }
+}
+
+// The generic function's type parameter in E0105's note: the first of
+// `T`, `U`, `V`, `W`, `T1`, `T2`, … the impl does not use.
+fn fresh_param_name(own: Vec<String>) -> String {
+    for p in ["T", "U", "V", "W"].iter() { if !holds(own.to_array(), p) { return p } }
+    let mut k = 1
+    while true {
+        let p = "T${k}"
+        if !holds(own.to_array(), p) { return p }
+        k += 1
+    }
+    ""
+}
+
+// The associated types `ty` projects onto `self_ty`'s, each once
+// (`collect_self_projections`).
+fn collect_self_projections(ty: Ty, self_ty: Ty, mut out: Vec<Int>) {
+    match ty {
+        TyAssoc(on, assoc) => {
+            if ty_eq(on, self_ty) && !int_in(out, assoc) { out.push(assoc) }
+            collect_self_projections(on, self_ty, out)
+        }
+        TyFn(params, ret) => {
+            for p in params.iter() { collect_self_projections(p, self_ty, out) }
+            collect_self_projections(ret, self_ty, out)
+        }
+        TySum(_, args) => for a in args.iter() { collect_self_projections(a, self_ty, out) }
+        TyRecord(_, args) => for a in args.iter() { collect_self_projections(a, self_ty, out) }
+        TyArray(e) => collect_self_projections(e, self_ty, out)
+        TyTuple(es) => for e in es.iter() { collect_self_projections(e, self_ty, out) }
+        TyFuture(o) => collect_self_projections(o, self_ty, out)
+        _ => ()
+    }
+}
+
+// Whether `start` reaches itself through `edges` (`reaches_self`).
+fn reaches_self(start: Int, edges: Map<Int, Vec<Int>>) -> Bool {
+    let mut stack: Vec<Int> = Vec::new()
+    match edges.get(start) {
+        Some(l) => for n in l.iter() { stack.push(n) }
+        None => ()
+    }
+    let mut seen: Set<Int> = Set::new()
+    while !stack.is_empty() {
+        let n = stack.pop().unwrap()
+        if n == start { return true }
+        if !seen.insert(n) { continue }
+        match edges.get(n) {
+            Some(succ) => for s in succ.iter() { stack.push(s) }
+            None => ()
+        }
+    }
+    false
+}
diff --git a/selfhost/src/typeck_ty.nova b/selfhost/src/typeck_ty.nova
new file mode 100644
index 00000000..e2b01542
--- /dev/null
+++ b/selfhost/src/typeck_ty.nova
@@ -0,0 +1,511 @@
+// The type checker's types: a port of check.rs's `convert_ty`,
+// `resolve_projection`, `find_assoc_type`, `normalize`, `resolve_bounds`,
+// `apply_where`, `method_sig_parts`, `collect_signatures`,
+// `collect_externs`, `require_ffi_safe` and view.rs's `component_ret`,
+// function by function (spec
+// docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md
+// §3.1, ADR 0055). typeck.nova gives the idioms.
+
+import ast
+import builtins
+import diagnostic
+import fixes
+import hir
+import resolver
+import token
+import typeck
+
+impl Checker {
+    /// An AST type as a `Ty`, its names resolved (`convert_ty`): `generics`
+    /// maps the parameters in scope to their indices, and `bounds[i]` is
+    /// parameter `i`'s bounds, for a projection on it.
+    pub fn convert_ty(
+        mut self,
+        ty: Spanned<Type>,
+        generics: Map<String, Int>,
+        bounds: Vec<Vec<Int>>,
+    ) -> Ty {
+        match ty.value {
+            TypePath(path, args) => self.convert_path(ty.span, path, args, generics, bounds)
+            TypeTuple(items) => {
+                if items.is_empty() { return TyUnit }
+                let mut out: Vec<Ty> = Vec::new()
+                for t in items.iter() { out.push(self.convert_ty(t, generics, bounds)) }
+                TyTuple(out)
+            }
+            TypeFn(params, ret) => {
+                let mut ps: Vec<Ty> = Vec::new()
+                for p in params.iter() { ps.push(self.convert_ty(p, generics, bounds)) }
+                let r = self.convert_ty(ret, generics, bounds)
+                TyFn(ps, r)
+            }
+            TypeRef(_, _) => {
+                self.unsupported(ty.span, "reference and pointer types")
+                TyError
+            }
+            TypePtr(_, _) => {
+                self.unsupported(ty.span, "reference and pointer types")
+                TyError
+            }
+            TypeArray(elem) => TyArray(self.convert_ty(elem, generics, bounds))
+            TypeOptional(_) => {
+                self.unsupported(ty.span, "the `T?` optional sugar")
+                TyError
+            }
+            TypeInfer => {
+                self.unsupported(ty.span, "`_` type placeholders")
+                TyError
+            }
+        }
+    }
+
+    // `convert_ty` of a path type.
+    fn convert_path(
+        mut self,
+        span: Span,
+        path: Path,
+        args: Vec<Spanned<Type>>,
+        generics: Map<String, Int>,
+        bounds: Vec<Vec<Int>>,
+    ) -> Ty {
+        let segs = path.segments
+        if segs.len() == 2 {
+            let base = segs.at(0).value
+            let assoc_name = segs.at(1).value
+            // A projection on a parameter in scope, or on `Self` in an impl;
+            // a module-qualified path otherwise.
+            let by_index = generics.get(base)
+            let in_impl = by_index.is_none() && base == "Self"
+            if by_index.is_some() || (in_impl && self.impl_self.is_some()) {
+                if !args.is_empty() {
+                    self.error("E0012", "`${base}::${assoc_name}` takes no type arguments", span)
+                }
+                let mut on = TyError
+                let mut candidates: Vec<Int> = Vec::new()
+                match by_index {
+                    Some(idx) => {
+                        on = TyParam(idx)
+                        if idx < bounds.len() { candidates = bounds.at(idx) }
+                    }
+                    None => {
+                        let imp = match self.impl_self {
+                            Some(imp) => imp
+                            None => return TyError
+                        }
+                        if imp.trait_id < 0 {
+                            let shown = display_ty(imp.ty, self.defs)
+                            self.error(
+                                "E0001",
+                                "no associated type `${assoc_name}` on `${shown}`: an inherent impl implements no trait, so nothing declares one",
+                                span,
+                            )
+                            return TyError
+                        }
+                        on = imp.ty
+                        let mut one: Vec<Int> = Vec::new()
+                        one.push(imp.trait_id)
+                        candidates = self.with_supertraits(one)
+                    }
+                }
+                return self.resolve_projection(on, base, assoc_name, candidates, span)
+            }
+            self.unsupported(span, "module-qualified type paths")
+            return TyError
+        }
+        if segs.len() != 1 {
+            self.unsupported(span, "module-qualified type paths")
+            return TyError
+        }
+        let name = segs.at(0).value
+        match generics.get(name) {
+            Some(idx) => {
+                if !args.is_empty() {
+                    self.error("E0012", "generic parameter `${name}` takes no type arguments", span)
+                }
+                return TyParam(idx)
+            }
+            None => ()
+        }
+        if name == "Future" {
+            if args.len() != 1 {
+                self.error(
+                    "E0012",
+                    "`Future` takes exactly one type argument, found ${args.len()}",
+                    span,
+                )
+                return TyError
+            }
+            return TyFuture(self.convert_ty(args.at(0), generics, bounds))
+        }
+        let prim = match name {
+            "Int" => Some(TyInt)
+            "Float" => Some(TyFloat)
+            "Bool" => Some(TyBool)
+            "Char" => Some(TyChar)
+            "String" => Some(TyString)
+            "Bytes" => Some(TyBytes)
+            _ => None
+        }
+        match prim {
+            Some(p) => {
+                if !args.is_empty() {
+                    self.error("E0012", "`${name}` takes no type arguments", span)
+                }
+                return p
+            }
+            None => ()
+        }
+        let def_id = self.resolve_type_in(self.cur_module, name)
+        if def_id >= 0 {
+            let is_record = match self.kind_of(def_id) {
+                DefKindRecord(_) => true
+                _ => false
+            }
+            let expected = if def_id < self.type_arity.len() {
+                self.type_arity.at(def_id)
+            } else {
+                0
+            }
+            let mut converted: Vec<Ty> = Vec::new()
+            for a in args.iter() { converted.push(self.convert_ty(a, generics, bounds)) }
+            if converted.len() != expected {
+                self.error(
+                    "E0012",
+                    "type `${name}` expects ${expected} type argument(s), found ${converted.len()}",
+                    span,
+                )
+                return TyError
+            }
+            return if is_record { TyRecord(def_id, converted) } else { TySum(def_id, converted) }
+        }
+        if !self.missing_on_target(name, span) {
+            self.error("E0001", "cannot find type `${name}`", span)
+            self.name_fixes(segs.at(0), NeedType, Vec::from_array(generics.keys()))
+        }
+        TyError
+    }
+
+    // `<base>::<assoc_name>` as a projection on `on`, the one of
+    // `candidate_traits` that declares it (`resolve_projection`).
+    fn resolve_projection(
+        mut self,
+        on: Ty,
+        base: String,
+        assoc_name: String,
+        candidate_traits: Vec<Int>,
+        span: Span,
+    ) -> Ty {
+        let mut found: Vec<Int> = Vec::new()
+        for t in candidate_traits.iter() {
+            let a = self.find_assoc_type(t, assoc_name)
+            if a >= 0 { found.push(a) }
+        }
+        if found.len() == 1 { return TyAssoc(on, found.at(0)) }
+        if found.is_empty() {
+            self.error(
+                "E0001",
+                "no associated type `${assoc_name}` on any bound of `${base}`",
+                span,
+            )
+        } else {
+            self.error(
+                "E0015",
+                "ambiguous associated type `${base}::${assoc_name}`: more than one bound of `${base}` declares it",
+                span,
+            )
+        }
+        TyError
+    }
+
+    /// The associated type `name` that trait `trait_id` declares, or -1
+    /// (`find_assoc_type`), read from the resolution, so that a trait's own
+    /// signatures can name it before its table entry exists.
+    pub fn find_assoc_type(self, trait_id: Int, name: String) -> Int {
+        match self.assoc_of.get(trait_id) {
+            Some(list) => {
+                for id in list.iter() { if self.defs.defs.at(id).name == name { return id } }
+                -1
+            }
+            None => -1
+        }
+    }
+
+    /// `ty` with its projections resolved through the impl table
+    /// (`normalize`); E0078 at `span` of `file` when a limit is met.
+    pub fn normalize(mut self, ty: Ty, file: String, span: Span) -> Ty {
+        match normalize_ty(ty, self.impls) {
+            Ok(t) => t
+            Err(limit) => {
+                let detail = match limit {
+                    NormalizeLimitDepth => "the chain of bindings is more than ${NORMALIZE_DEPTH_LIMIT} deep, or does not terminate"
+                    NormalizeLimitSteps => "it resolves to more than ${NORMALIZE_STEP_LIMIT} type nodes"
+                }
+                self.error_in(
+                    "E0078",
+                    "could not resolve the associated types in `${display_ty(ty, self.defs)}`: ${detail}",
+                    file,
+                    span,
+                )
+                TyError
+            }
+        }
+    }
+
+    /// Each generic parameter's bounds as trait ids, each once
+    /// (`resolve_bounds`).
+    pub fn resolve_bounds(mut self, generics: Vec<TypeParam>) -> Vec<Vec<Int>> {
+        let mut out: Vec<Vec<Int>> = Vec::new()
+        for g in generics.iter() {
+            let mut ids: Vec<Int> = Vec::new()
+            for b in g.bounds.iter() { self.add_bound(ids, b) }
+            out.push(ids)
+        }
+        out
+    }
+
+    // The trait `b` names added to `slot`, once, or E0092 or E0001 with
+    // its fixes.
+    fn add_bound(mut self, mut slot: Vec<Int>, b: Spanned<Path>) {
+        let segs = b.value.segments
+        let name = if segs.is_empty() { "" } else { segs.last().value }
+        let id = self.resolve_trait_in(self.cur_module, name)
+        if id >= 0 {
+            if !int_in(slot, id) { slot.push(id) }
+        } else if !self.missing_on_target(name, b.span) {
+            self.error("E0001", "cannot find trait `${name}`", b.span)
+            if !segs.is_empty() { self.name_fixes(segs.last(), NeedTrait, Vec::new()) }
+        }
+    }
+
+    /// A `where` clause folded into `bounds`, each bound on one of the
+    /// item's own parameters (`apply_where`).
+    pub fn apply_where(
+        mut self,
+        bounds: Vec<Vec<Int>>,
+        where_clause: Vec<WhereBound>,
+        scope: Map<String, Int>,
+    ) {
+        for wb in where_clause.iter() {
+            let idx = match self.convert_ty(wb.ty, scope, bounds) {
+                TyParam(i) => i
+                TyError => continue
+                _ => {
+                    self.error(
+                        "E0900",
+                        "a `where` clause may only constrain a type parameter",
+                        wb.ty.span,
+                    )
+                    continue
+                }
+            }
+            if idx < 0 || idx >= bounds.len() { continue }
+            let slot = bounds.at(idx)
+            for b in wb.bounds.iter() { self.add_bound(slot, b) }
+        }
+    }
+
+    /// A method's parameter types but `self`'s, and its return type
+    /// (`method_sig_parts`).
+    pub fn method_sig_parts(
+        mut self,
+        params: Vec<Param>,
+        ret: Option<Spanned<Type>>,
+        scope: Map<String, Int>,
+        bounds: Vec<Vec<Int>>,
+    ) -> (Vec<Ty>, Ty) {
+        let mut converted: Vec<Ty> = Vec::new()
+        for p in params.iter() {
+            if p.name.value != "self" { converted.push(self.convert_ty(p.ty, scope, bounds)) }
+        }
+        let r = match ret {
+            Some(t) => self.convert_ty(t, scope, bounds)
+            None => TyUnit
+        }
+        (converted, r)
+    }
+
+    // --- Pass 5: signatures and externs ---
+
+    /// Every function's and constant's signature (`collect_signatures`);
+    /// an async function's returns `Future<T>`.
+    pub fn collect_signatures(mut self) {
+        for (def_id, item_index) in self.defs_of_kind(false).iter() {
+            if !self.own(def_id) { continue }
+            let f = match self.items.at(item_index).value {
+                ItemFunction(f) => f
+                _ => continue
+            }
+            self.enter_item(item_index)
+            self.check_duplicate_generics(f.generics, "function")
+            let generics = generic_scope(f.generics)
+            let bounds = self.resolve_bounds(f.generics)
+            self.apply_where(bounds, f.where_clause, generics)
+            self.expand_bounds(bounds)
+            let mut params: Vec<Ty> = Vec::new()
+            for p in f.params.iter() { params.push(self.convert_ty(p.ty, generics, bounds)) }
+            let mut ret = if f.is_component {
+                self.component_ret()
+            } else {
+                match f.return_ty {
+                    Some(t) => self.convert_ty(t, generics, bounds)
+                    None => TyUnit
+                }
+            }
+            if f.is_async { ret = TyFuture(ret) }
+            self.sigs.set(
+                def_id,
+                Some(
+                    FnSig { generics: f.generics.len(), bounds: bounds, params: params, ret: ret },
+                ),
+            )
+        }
+        for (def_id, item_index) in self.defs_of_kind(true).iter() {
+            if !self.own(def_id) { continue }
+            let c = match self.items.at(item_index).value {
+                ItemConst(c) => c
+                _ => continue
+            }
+            self.enter_item(item_index)
+            let ret = self.convert_ty(c.ty, Map::new(), Vec::new())
+            self.sigs.set(
+                def_id,
+                Some(FnSig { generics: 0, bounds: Vec::new(), params: Vec::new(), ret: ret }),
+            )
+        }
+    }
+
+    /// `std/ui`'s `View`, a component's return type (`component_ret`).
+    pub fn component_ret(self) -> Ty {
+        let ui = self.module_named("$std.ui")
+        let html = self.module_named("$std.ui.html")
+        if ui < 0 || html < 0 { return TyError }
+        let view = self.resolve_type_in(ui, "View")
+        let attr = self.resolve_type_in(ui, "Attr")
+        if view < 0 || attr < 0 { return TyError }
+        if !self.is_def_value(html, "on") || !self.is_def_value(html, "text") { return TyError }
+        TySum(view, Vec::new())
+    }
+
+    // The first module named `name`, or -1.
+    fn module_named(self, name: String) -> Int {
+        for m in 0..self.defs.modules.len() { if self.defs.modules.at(m).name == name { return m } }
+        -1
+    }
+
+    // Whether the value `name` in module `m` is a definition.
+    fn is_def_value(self, m: Int, name: String) -> Bool {
+        match self.resolve_value_in(m, name) {
+            Some(ResDef(_)) => true
+            _ => false
+        }
+    }
+
+    /// Every `extern` function's signature and table entry, the C ABI and
+    /// FFI-safe types only, or `"js"` in a std module of a WASM build
+    /// (`collect_externs`).
+    pub fn collect_externs(mut self) {
+        for i in 0..self.defs.defs.len() {
+            let (item_index, fn_index) = match self.defs.defs.at(i).kind {
+                DefKindExternFn(item, f) => (item, f)
+                _ => continue
+            }
+            let def_id = i
+            if !self.own(def_id) { continue }
+            let block = match self.items.at(item_index).value {
+                ItemExtern(b) => b
+                _ => continue
+            }
+            let sig = match block.items.at(fn_index) {
+                ExternItemFn(s) => s
+            }
+            self.enter_item(item_index)
+            let name = sig.name.value
+            if name == "main" || name.starts_with("nova_") {
+                self.error(
+                    "E0900",
+                    "extern symbol `${name}` is reserved by the compiler",
+                    sig.name.span,
+                )
+                continue
+            }
+            let abi = match block.abi {
+                Some(a) => a
+                None => ""
+            }
+            let wasm = match self.defs.target {
+                CompileTargetWasm => true
+                CompileTargetNative => false
+            }
+            let js = block.abi.is_some()
+                && abi == "js"
+                && self.is_std_module(self.cur_module)
+                && wasm
+            if !js && block.abi.is_some() && abi != "C" {
+                self.error(
+                    "E0900",
+                    "extern ABI `\"${abi}\"` is not supported; only the C ABI is supported",
+                    sig.name.span,
+                )
+            }
+            if sig.is_async { self.unsupported(sig.name.span, "async extern functions") }
+            if !sig.where_clause.is_empty() {
+                self.unsupported(sig.name.span, "`where` clauses on extern functions")
+            }
+            if !sig.generics.is_empty() {
+                self.unsupported(sig.name.span, "generic extern functions")
+                continue
+            }
+            let empty: Map<String, Int> = Map::new()
+            let mut params: Vec<Ty> = Vec::new()
+            for p in sig.params.iter() {
+                let t = self.convert_ty(p.ty, empty, Vec::new())
+                self.require_ffi_safe(t, p.ty.span, false, js)
+                params.push(t)
+            }
+            let ret = match sig.return_ty {
+                Some(t) => {
+                    let r = self.convert_ty(t, empty, Vec::new())
+                    self.require_ffi_safe(r, t.span, true, js)
+                    r
+                }
+                None => TyUnit
+            }
+            self.sigs.set(
+                def_id,
+                Some(FnSig { generics: 0, bounds: Vec::new(), params: params, ret: ret }),
+            )
+            self.externs.push(
+                ExternFn {
+                    def_id: def_id,
+                    symbol: name,
+                    abi: if js { ExternAbiJs } else { ExternAbiC },
+                    params: params,
+                    ret: ret,
+                    span: FileSpan { file: self.cur_file, span: sig.name.span },
+                },
+            )
+        }
+    }
+
+    // E0900 for a type an extern signature cannot carry
+    // (`require_ffi_safe`).
+    fn require_ffi_safe(mut self, ty: Ty, span: Span, is_return: Bool, js: Bool) {
+        let ok = match ty {
+            TyInt => true
+            TyFloat => true
+            TyBool => true
+            TyString => js
+            TyUnit => is_return
+            TyError => true
+            _ => false
+        }
+        if !ok {
+            let extra = if js { ", and String for extern \"js\"" } else { "" }
+            self.error(
+                "E0900",
+                "`${display_ty(ty, self.defs)}` is not FFI-safe in an extern signature yet; only Int, Float, and Bool (and a unit return) are supported${extra}",
+                span,
+            )
+        }
+    }
+}
````

### Task 4: ADR 0055 and the records

Spec §8; decisions 1 to 22.

**Files:**
- Create:
  `docs/adr/0055-the-self-hosted-item-collection-and-the-hir-dump.md`
- Modify: `CHANGELOG.md`, `docs/phase-5-plan.md`,
  `nova-spec/00-MASTER-SPEC.md`, `nova-spec/12-TYPESYSTEM.md`

- [ ] **Step 1: The records.** `patch_block t4-code`:
  - **ADR 0055,** the self-hosted item collection and the HIR dump:
    its Status `Accepted (2026-11-11).` and this branch,
    `claude/loving-archimedes-frtf5q-53a`, as ADR 0054's names 5.2b's,
    written in three parts; §1 `check_items`; §2 the dump's grammar and one example of every
    form; §3 the hidden flag; §4 std's item tables shared, the fallback and
    the guard; §5 the cases; §6 the type fuzzer; §7 part A's decisions; §8
    N's code and idioms; §9 the differential; §10 part B's decisions; §11
    the time and CI's projection; §12 N's stack and §13 N's memory, each
    measured again here (decision 22); §14 the rule that a change to item
    collection changes both compilers; §15 the CI job, its figures, the
    options, the user's choice and CI run 42774093193's figures; then its
    Consequences, as ADRs 0050 to 0054 end.
  - **`CHANGELOG.md`,** Unreleased: the hidden `nova check --dump-hir`
    and N's item collection; nothing a user runs changes.
  - **Dated notes:** `docs/phase-5-plan.md`'s 5.3 split by stage, with the
    corrected sizes, and the CI job; `00-MASTER-SPEC.md`'s Phase 5;
    `12-TYPESYSTEM.md` where it describes the checker's passes. Each note
    names this branch, `claude/loving-archimedes-frtf5q-53a`, as 5.2b's
    name theirs.
- [ ] **Step 2: Read them against the code.** Each record's claim against
  the tree: the ADR's decisions against Tasks 1 to 3 and this plan's
  decisions; its counts (251 loose and 74 package files natively, 39
  WASM, 10 with tests, as `SH --nocapture --exact
  the_hir_dumps_agree_on_std_the_corpus_and_the_cases` prints them (`SH`
  already ends with `--`), `the
  HIR pass dumps 251 loose programs, 74 package files, 39 WASM programs,
  10 files with tests and [164, 2, 1] cases`; 154, 16) and the CI
  figures against decisions 11 and 14; its examples against `nova check
  --dump-hir` of their sources, by a script that writes each example's
  sources, runs the command and finds every line of the example in S0's
  output, in order (std's example shows part of std's part):

```sh
mkdir -p "$(dirname "$P/plan53a/adrcheck.py")"
cat > "$P/plan53a/adrcheck.py" <<'EOF'
#!/usr/bin/env python3
"""adrcheck.py: run from the repository's root, after the build, with $P set.
ADR 0055 §2's examples against S0's `nova check --dump-hir` of their
sources. Each example in the block after "One example of every form" is
its `;;` lines (the command, `;; NAME.nova: text` and `;;   text` going
on, with `\\n` for a line break) and its dump lines; the sources are
written into a fresh directory, the command run there (`--dump-hir` and the
files in order when the example names none), and every dump line must be a
line of the output, in order, so an example that shows part of a dump
(std's) is checked as far as it goes. Prints each example's verdict, then
the count."""
import os
import re
import shutil
import subprocess
import sys

P = os.environ["P"]
ADR = "docs/adr/0055-the-self-hosted-item-collection-and-the-hir-dump.md"
NOVA = os.path.abspath("target/debug/nova")
text = open(ADR, encoding="utf-8").read().split("\n")
start = next(i for i, l in enumerate(text) if l.startswith("**One example of every form**"))
a = next(i for i in range(start, len(text)) if text[i] == "```")
b = next(i for i in range(a + 1, len(text)) if text[i] == "```")
block = text[a + 1 : b]
examples, cur = [], []
for line in block + [""]:
    if line == "":
        if cur:
            examples.append(cur)
        cur = []
    else:
        cur.append(line)
bad = 0
for k, ex in enumerate(examples, 1):
    files, last, args = [], None, None
    body = [l for l in ex if not l.startswith(";;")]
    for l in ex:
        if not l.startswith(";;"):
            continue
        m = re.search(r"nova check (--dump-hir[^:,]*?)(?::|,|$)", l)
        if m:
            args = m.group(1).split()
            args = [x for x in args if not x.endswith(".nova")]
        m = re.match(r";; (\S+\.nova): (.*)$", l)
        if m:
            files.append([m.group(1), m.group(2)])
            continue
        m = re.match(r";;   (.*)$", l)
        if m and files:
            files[-1][1] += m.group(1)
    work = f"{P}/plan53a/adr-ex-{k}"
    shutil.rmtree(work, ignore_errors=True)
    os.makedirs(work)
    for name, src in files:
        open(os.path.join(work, name), "w", encoding="utf-8").write(src.replace("\\n", "\n"))
    argv = [NOVA, "check"] + (args or ["--dump-hir"]) + [n for n, _ in files]
    out = subprocess.run(argv, cwd=work, capture_output=True, text=True).stdout.split("\n")
    at, missing = 0, None
    for line in body:
        while at < len(out) and out[at] != line:
            at += 1
        if at == len(out):
            missing = line
            break
        at += 1
    verdict = "ok" if missing is None else f"MISSING {missing!r}"
    bad += missing is not None
    print(f"example {k}: {' '.join(argv[2:])}: {len(body)} lines, {verdict}")
print(f"adr examples: {len(examples) - bad} of {len(examples)} found in S0's dump, in order")
sys.exit(1 if bad else 0)
EOF
```

  After the build, `python3 "$P/plan53a/adrcheck.py"`. Expected, as the
  replay printed:

  ```
  example 1: --dump-hir shapes.nova: 58 lines, ok
  example 2: --dump-hir errs.nova: 8 lines, ok
  example 3: --dump-hir c.nova: 5 lines, ok
  example 4: --dump-hir --std --target wasm: 4 lines, ok
  adr examples: 4 of 4 found in S0's dump, in order
  ```

- [ ] **Step 3: The checks,** as the conventions list them.
- [ ] **Step 4: Commit**
      `docs: ADR 0055, the self-hosted item collection and the HIR dump,
      and the notes for 5.3a`.

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index a6967561..0ca3f7f2 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -19,7 +19,10 @@ self-hosted lexer and the differential (spec
 depth limit (§4; ADR 0052). Phase 5.2a, the self-hosted loader and
 resolver for loose programs and the resolution dump (spec
 `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
-§3; ADR 0053).
+§3; ADR 0053). Phase 5.3a, the self-hosted item collection and the HIR
+dump (spec
+`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
+§3; ADR 0055).
 
 ### Added
 
@@ -227,6 +230,30 @@ resolver for loose programs and the resolution dump (spec
   each mode and the fuzzer's (ADR 0054). Nothing a user sees changes with
   5.2b: `nova check`, `nova test` and `nova build` give what they gave,
   byte for byte, on every file and package of the repository.
+- **`nova check --dump-hir FILE…` and `--std`, hidden:** what the type
+  checker's item stage built for each program whose root is FILE, after
+  the resolution dump's pipeline: its records, sums, traits, impls, every
+  signature and its externs, by structure and with the resolution dump's
+  ids, then every diagnostic so far, fixes included; `--std` prints std's
+  part once, and `--target wasm` and `--test` as `--dump-resolution` takes
+  them. std's item tables are built once for all the roots. It is absent
+  from `--help`, carries no stability promise and is taken only when
+  `--dump-hir` comes first after `check`, by the resolution dump's own
+  parser (5.3a, ADR 0055).
+- **The self-hosted item collection, in `selfhost/`:** N collects a
+  program's items as `nova-typeck`'s item stage does, function for
+  function: the generic-parameter checks, arities, supertraits, records,
+  sums, traits, impls with their checks, signatures, externs, `convert_ty`
+  and the normalization of associated types, and the 16 item codes in
+  S0's order, word for word, with their import and "did you mean" fixes.
+  `selfhost hir --std DIR FILE…` prints the same dump as `nova check
+  --dump-hir`, and `--std-part` std's; the differential compares them on
+  std's three parts, every corpus root natively, for the WASM target and
+  under test mode, the type checker's cases and a type-level fuzzer's
+  programs. A change to how items are collected now changes both
+  compilers in one pull request (ADR 0055). Nothing a user sees changes
+  with 5.3a: `nova check`, `nova test` and `nova build` give what they
+  gave, byte for byte, on every file of the repository.
 
 ### Changed
 
diff --git a/docs/adr/0055-the-self-hosted-item-collection-and-the-hir-dump.md b/docs/adr/0055-the-self-hosted-item-collection-and-the-hir-dump.md
new file mode 100644
index 00000000..49ec6d74
--- /dev/null
+++ b/docs/adr/0055-the-self-hosted-item-collection-and-the-hir-dump.md
@@ -0,0 +1,791 @@
+# ADR 0055 — The self-hosted item collection, and the HIR dump
+
+## Status
+
+Accepted (2026-11-11). Branch `claude/loving-archimedes-frtf5q-53a`
+(`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
+§3), written in three parts: part A (S0's side) wrote §1 to §7, part B
+(N's side, the comparison, the figures) §8 to §14. §1 is S0's item stage
+as a function; §2 the HIR dump's grammar, with one example of every
+form; §3 the hidden flag; §4 std's item tables shared, its fallback and
+its guard; §5 the cases; §6 the type fuzzer's item mutations; §7 the
+decisions part A took; §8 N's code and idioms; §9 the differential; §10
+the decisions part B took; §11 the time, and CI's projection; §12 N's
+stack; §13 N's memory; §14 the rule that a change to item collection
+changes both compilers. Part C wrote §15, CI's figures and the user's
+choice: `selfhost.rs` in a job of its own on each system, N on a release
+runtime; and §12's release-runtime figures. The 5.3a plan's step 0,
+before the plan split the work into tasks, wrote §9's count of 164
+cases, the "Measured again" tables of §12 and §13, §12's account of the
+harness's 1 MB tests, §15's figures of CI run 42774093193, and the
+Consequences. 5.3b (the bodies, the full dump) writes ADR 0056.
+
+## Decision
+
+### 1. `check_items`: S0's item stage, as a function
+
+`nova_typeck::check_items(file, defs, options)` runs passes 1 to 5 of
+`check_with` and returns `ItemTables`: the records, sums, traits, impls,
+signatures and externs, the tables the bodies read (receivers, method
+places, impl selves, arities, supertraits, the impls' spans and owners),
+and the item diagnostics in the order the passes emitted them.
+`check_with` calls it first and restores its checker from the tables: a
+pure refactor, `nova check` over every base-tracked `.nova` file printing
+what the base prints (1,078 checks, `checkcmp.sh`), and a unit test that
+`check_items`' diagnostics are the prefix of `check_with`'s and its
+tables the module's.
+
+**Passes 1 to 5, and 2 to 5.** The spec says both (§3.1). They are one
+boundary seen from two sides: pass 1 is the language server's index
+(`note_item_declarations`), which records nothing unless asked and gives
+no diagnostic. S0's `check_items` keeps it, so that `check_with` calls one
+function and its order is as it was; N ports passes 2 to 5 (decision 8),
+and the dump, which asks for no index, shows passes 2 to 5 alone.
+
+### 2. The HIR dump
+
+`nova_typeck::dump::dump_program` writes a program's part,
+`nova_typeck::dump::dump_std` std's part; `nova_driver::HirDumps` runs the
+resolution dump's pipeline for each root (ADR 0053 §1, ADR 0054 §1), then,
+when no error stopped it, the item passes with `check`'s options (the
+sources, no probe, no index), and dumps their output as returned, before
+the driver's `keep_the_entry_main`. `nova_driver::dump_std_hir` checks a
+program of no module of its own and dumps std's part.
+
+**Conventions:** the resolution dump's (ADR 0053 §1): two spaces a depth,
+`(Kind`, values, fields `key=value`, children a level deeper, `)` at the
+end of the last line; names, symbols and messages quoted as the AST dump
+quotes strings; a place is a file's label quoted, `:` and a span.
+
+**Ids:** the resolution dump's: `u<n>` for a program's own definition,
+`s<n>` for std's, its `DefId` less the first std definition's. An impl has
+no `DefId`; it is numbered from 0 within its part, in the table's order.
+
+**Types,** by structure, with ids, not names (spec §3.3):
+
+```
+ty = "Int" | "Float" | "Bool" | "Char" | "String" | "Bytes" | "()"
+   | "fn(" [ty {", " ty}] ") -> " ty
+   | "Future<" ty ">" | "[" ty "]"
+   | "(" ty "," ")" | "(" ty ", " ty {", " ty} ")"     -- a tuple
+   | id ["<" ty {", " ty} ">"]                           -- a record or a sum
+   | "T" uint                                            -- a type parameter
+   | ty "::" id                                          -- a projection
+   | "?" uint | "!" | "{error}"                          -- a variable, never, error
+```
+
+A record and a sum print alike, as their id: which one it is, the
+resolution dump's `(Def.Record` or `(Def.Sum` says, and `convert_ty`
+decides it from the definition's kind alone.
+
+**The grammar** (5.3a's part; `I(d)` is `d` levels of indentation):
+
+```
+part      = "IOERR " kind "\n" | program diagnostics
+program   = "(Hir target=" target [" mode=test"] " checked=" bool
+            [ "\n" I(1) "(StdStart module=" uint " def=" uint " item=" uint ")"
+              entries ] ")\n"
+std       = "(HirStd target=" target [" mode=test"] " records=" uint " sums=" uint
+            " traits=" uint " impls=" uint " sigs=" uint " externs=" uint
+            entries ")\n" diagnostics
+entries   = { record } { sum } { trait } { impl } { sig } { extern }
+            -- 5.3b adds every function here, after the externs
+record    = I(1) "(Record " id " " string " generics=" uint { I(2) "(Field " string " " ty ")" } ")"
+sum       = I(1) "(Sum " id " " string " generics=" uint
+            { I(2) "(Variant " string { " " ty } ")" } ")"
+trait     = I(1) "(Trait " id " " string { I(2) "(Supertrait " id ")" }
+            { I(2) "(AssocType " string " " id ")" } { method } ")"
+method    = I(2) "(Method " string " has_self=" bool " mut_self=" bool " generics=" uint
+            " default=" (id | "None") bounds(3) signature(3) ")"
+impl      = I(1) "(Impl " uint " trait=" (id | "None") " head=" head " generics=" uint " " place
+            I(2) "(SelfType " ty ")" bounds(2)
+            { I(2) "(Method " string " " id ")" } { I(2) "(AssocBinding " id " " ty ")" } ")"
+head      = "Int" | "Float" | "Bool" | "Char" | "String" | "Bytes" | "Array" | id
+sig       = I(1) "(Sig " id " generics=" uint [" selfless=" bool " mut_self=" bool]
+            bounds(2) signature(2) ")"     -- the two flags on an impl's method only
+extern    = I(1) "(Extern " id " " string " abi=" ("C" | "Js") " " place signature(2) ")"
+bounds(d) = { I(d) "(Bound " uint { " " id } ")" }    -- every generic parameter's, empty or not
+signature(d) = { I(d) "(Param " ty ")" } I(d) "(Ret " ty ")"
+diagnostics = -- the resolution dump's (ADR 0053 §1), every diagnostic so far
+```
+
+Tables print in the order the type checker collected them: records, then
+sums, traits and impls in `DefId` or item order, each with the program's
+before std's; signatures, a map, sorted by `DefId`: functions, methods
+(an impl's and a trait's default), constants (no parameters, the declared
+type as `Ret`) and externs. A method's signature holds its receiver's type
+as its first parameter when it has one (`selfless=false`), as `FnSig`
+does; a trait's method, which has no `FnSig`, holds no receiver, as
+`TraitMethod` does, and its bounds exclude `Self`'s. A program that stops
+before the type checker prints `checked=false` and the diagnostics so far;
+its part repeats only `(StdStart …)` and those of the resolution dump.
+
+**One example of every form**, S0's dump of the source after it:
+
+```
+;; nova check --dump-hir shapes.nova
+;; shapes.nova: record Pair<T> { a: T, b: Int }\ntype Shape = | Dot | Line(Int, Float)\n
+;;   trait Area: Eq {\n    type Unit\n    fn area(self) -> Self::Unit\n
+;;       fn scale<U: Ord>(mut self, by: U) -> Bool { true }\n}\n
+;;   impl<T: Eq> Area for Pair<T> {\n    type Unit = [T]\n    fn area(self) -> [T] { [] }\n}\n
+;;   impl Eq for Pair<Int> {\n    fn eq(self, o: Pair<Int>) -> Bool { true }\n}\n
+;;   impl Shape {\n    fn origin() -> Shape { Dot }\n}\n
+;;   extern "C" {\n    fn abs(x: Int) -> Int\n}\nconst LIMIT: Int = 3\n
+;;   async fn fetch(t: (Int, String)) -> Option<Int> { None }\n
+(Hir target=native checked=true
+  (StdStart module=1 def=11 item=9)
+  (Record u0 "Pair" generics=1
+    (Field "a" T0)
+    (Field "b" Int))
+  (Sum u1 "Shape" generics=0
+    (Variant "Dot")
+    (Variant "Line" Int Float))
+  (Trait u2 "Area"
+    (Supertrait s19)
+    (AssocType "Unit" u3)
+    (Method "area" has_self=true mut_self=false generics=0 default=None
+      (Ret T0::u3))
+    (Method "scale" has_self=true mut_self=true generics=1 default=u4
+      (Bound 0 s21 s19)
+      (Param T1)
+      (Ret Bool)))
+  (Impl 0 trait=u2 head=u0 generics=1 "shapes":211..218
+    (SelfType u0<T0>)
+    (Bound 0 s19)
+    (Method "area" u5)
+    (AssocBinding u3 [T0]))
+  (Impl 1 trait=s19 head=u0 generics=0 "shapes":287..296
+    (SelfType u0<Int>)
+    (Method "eq" u6))
+  (Impl 2 trait=None head=u1 generics=0 "shapes":353..358
+    (SelfType u1)
+    (Method "origin" u7))
+  (Sig u4 generics=2
+    (Bound 0 u2 s19)
+    (Bound 1 s21 s19)
+    (Param T0)
+    (Param T1)
+    (Ret Bool))
+  (Sig u5 generics=1 selfless=false mut_self=false
+    (Bound 0 s19)
+    (Param u0<T0>)
+    (Ret [T0]))
+  (Sig u6 generics=0 selfless=false mut_self=false
+    (Param u0<Int>)
+    (Param u0<Int>)
+    (Ret Bool))
+  (Sig u7 generics=0 selfless=true mut_self=false
+    (Ret u1))
+  (Sig u8 generics=0
+    (Param Int)
+    (Ret Int))
+  (Sig u9 generics=0
+    (Ret Int))
+  (Sig u10 generics=0
+    (Param (Int, String))
+    (Ret Future<s0<Int>>))
+  (Extern u8 "abs" abi=C "shapes":416..419
+    (Param Int)
+    (Ret Int)))
+(Diagnostics
+  (Error "E0072" "the trait `Area` requires `Eq`, which `Pair<T0>` does not implement"
+    (Primary "shapes":211..218 "here")))
+
+;; A note and a fix, after the program's part:
+;; nova check --dump-hir errs.nova
+;; errs.nova: impl Eq for [Int] {}\nfn f(x: Strin) {}\n
+(Diagnostics
+  (Error "E0010" "a trait cannot be implemented for an array"
+    (Primary "errs":12..17 "an array type")
+    (Note "an array takes inherent impls only, `impl<T> [T] { … }` or `impl [Int] { … }`; wrap the array in a record to implement a trait for it"))
+  (Error "E0001" "cannot find type `Strin`"
+    (Primary "errs":29..34 "here")
+    (Fix "change `Strin` to `Stdin`"
+      (Edit "errs":29..34 "Stdin"))))
+
+;; checked=false: the resolver's E0002 stops the program:
+;; c.nova: fn f(x: Nope) {}\nfn f() {}\n
+(Hir target=native checked=false)
+(Diagnostics
+  (Error "E0002" "duplicate definition of `f`"
+    (Primary "c":20..21 "redefined here")
+    (Secondary "c":3..4 "first defined here")))
+
+;; std's part, and abi=Js, from nova check --dump-hir --std --target wasm:
+(HirStd target=wasm records=49 sums=10 traits=10 impls=82 sigs=711 externs=61
+  (Extern s307 "dom_body" abi=Js "<std/dom>":922..930
+    (Ret Int)))
+(Diagnostics)
+```
+
+`nova_typeck::dump::KINDS` lists every form, `LATER_KINDS` the two
+severities no stage gives; the harness checks that std, the corpus and
+the cases print every listed form and nothing unlisted. **Sizes:** std's
+native part is 3,107 lines (79,476 bytes), its WASM part 3,436 (83,762),
+its test-mode part 3,121 (79,737): 63 records, 14 sums, 12 traits, 95
+impls and 623 signatures natively.
+
+### 3. The hidden flag
+
+Taken only when `--dump-hir` is the first argument after `check`, by the
+resolution dump's own parser (ADR 0053 §1), generalised to either flag
+(`DumpCmd`): the same arguments (`--std`, `--target native|wasm`,
+`--test`, files, each option once), the same refusals in clap's words
+under the usage `nova check --dump-hir`, and the same three errors of its
+own, naming `--dump-hir`. Every other command line, `--help` and argv[0]
+included, is the base's, byte for byte (`helpcmp.sh`, `usagecmp.sh` with
+`--dump-hir`'s shapes, `argv0.py`). Both dumps' outputs are settled in the
+harness's `run`, so E0004's and E0093's Windows paths compare alike
+(ADR 0054 §7).
+
+### 4. std's item tables, shared
+
+`nova_typeck::SharedItems` holds std's `ItemTables`, built once per
+process and mode from the check of a program of no module of its own
+(`SharedStd::alone`, which 5.2b's shared resolution now keeps whole).
+Each program is resolved by `resolve_program_with_shared_std_for_check`:
+5.2b's shared path, then std's definitions, module scopes and items'
+modules appended, rebased past the program's (`Def::rebased`,
+`ModuleScope::rebased`), so the type checker reads std's definitions as
+the full resolution numbers them; std's items are not in the merged file.
+`check_items_shared` runs each pass over the program's own definitions
+(`Checker::own_defs`) and splices std's part of each table, rebased,
+where the full check would have collected std's items: at the end of each
+pass's loop, after the program's (`Splice`), so tables keep the full
+check's order and no pass sees a std entry the full check had not yet
+collected. The impl checks (signatures, coherence, hidden methods,
+supertrait impls) then run over the program's impls and std's together,
+as the full check runs them. A program never writes the shared state.
+
+**The fallback:** should std's items give a diagnostic, or 5.2b's shared
+resolution not apply (std's diagnostic or `@test`), every program is
+resolved and checked in full. A unit test builds std with one more module
+and checks that the shared path gives a program what the full check gives
+it (E0074 and E0105 against std's impls among it) and that a std whose
+items give a diagnostic is not shareable.
+
+**The guard:** the driver's test
+`std_checked_once_dumps_what_std_checked_for_each_program_dumps` compares
+the shared and unshared HIR dumps byte for byte over every corpus root and
+every case (`resolve*` and `check*`) of each mode, in order and reversed;
+another checks that std's entries of a two-module program's check print as
+std's part. Only the dump shares: `nova check`, `build`, `run`, `test` and
+the language server keep `check_with` on the full resolution.
+
+### 5. The cases
+
+`cases/check/` (under `check`), `cases/check-wasm/` (the WASM target) and
+`cases/check-test/` (test mode), laid out and named as the resolver's
+(ADR 0054 §2): the directory names the mode. The item-level tests'
+sources come from a temporary instrumentation of the type checker's test
+helpers (each source a test lexes, written under the test's name; a
+script, not committed). A test is item-level when one of its sources
+gives an item diagnostic (the HIR dump's diagnostics beyond the
+resolution dump's) or its body reads the item tables: 114 of the 550
+tests that lex a source, whose 158 sources give 154 cases (one dropped as
+a 5.2 case's text, three as repeats), the one two-module test a directory
+rooted at `main.nova`. The other 436 tests are body-level and wait for
+5.3b, which re-runs the same instrumentation and rule. Hand cases (names
+without `__`): every item E0001 site, every E0012 form, the unsupported
+types and an extern's `where`, the array `len` and inherent overlaps, the
+import fix (both edits) and did-you-mean, did-you-mean a primitive or a
+type parameter, an import fix held back by a clash, none when two
+modules export the name, no fix in a dependency, std's supertraits in
+a program's bounds (the last five,
+before the split, for mutants that passed); every item E0092 site and a
+component on WASM; tests beside items and an item error under test mode.
+No case needs `fn main`. The harness runs them again with `\r\n`.
+
+### 6. The type fuzzer's item mutations
+
+`fuzz::type_mutate`: a loose corpus program that imports nothing and whose
+HIR dump reaches the type checker with no diagnostic (S0 picks them, the
+same on every system), mutated one to three times: a parameter's,
+return's or field's type replaced (E0001, E0012, E0900), a generic list's
+first parameter duplicated (E0403), a trait impl's method dropped (E0070),
+added (E0071) or retyped (E0072), an impl duplicated (E0074), or one of
+`fuzz::TYPE_ITEMS` appended, each aimed at a code: E0001, E0010, E0012,
+E0015, E0070 to E0074, E0076 to E0078, E0105, E0403 and E0900. Seeds 401
+to 403, 40 inputs each. Every input reaches the type checker, and each
+aimed code occurs at least twice, which the harness asserts; on S0's dump
+(part A): E0001 30, E0010 16, E0012 50, E0015 11, E0070 10, E0071 14,
+E0072 21, E0073 18, E0074 23, E0076 5, E0077 20, E0078 19, E0105 4, E0403
+17, E0900 46. E0092 is the WASM target's and occurs in the cases.
+
+### 7. The decisions part A took
+
+1. **`check_items` covers passes 1 to 5** (§1); N ports 2 to 5.
+2. **The dump lives in `nova-typeck`** (`dump.rs`), since it reads the
+   checker's private tables; it reuses the resolution dump's diagnostics
+   printer, places and `StdStart` (`write_diagnostics`, `write_place`,
+   `std_start`), made public for it.
+3. **A record and a sum print as their id** (§2), as the spec writes them.
+4. **An impl's place** is its self type's span, the one its diagnostics
+   name; an impl's method's receiver flags are its `Sig`'s.
+5. **Every generic parameter's bounds print, empty or not**, so a slot's
+   count is in the dump.
+6. **The sharing splices per pass** (§4) rather than seeding std's tables
+   first, which would change the tables' order and what a pass sees.
+7. **Item-level tests** are chosen by S0's dumps, a rule 5.3b can re-run.
+8. **A record prints no bounds:** a record's bounds are a resolution
+   scope for its fields' projections, not a constraint, and `RecordType`
+   keeps none; its fields' types show what they resolved to.
+9. **`abi=Js` occurs only in std's WASM part:** a program's `extern "js"`
+   is E0900, so no program part prints it.
+10. **E0099's note is a body diagnostic,** 5.3b's: test mode changes no
+    item diagnostic, and `cases/check-test/` holds tests beside items.
+
+### 8. N's code
+
+| File | Lines | Code | What it ports |
+|---|---:|---:|---|
+| `selfhost/src/hir.nova` | 585 | 455 | `nova-hir`'s `Ty` (18 variants), `TyHead`, the tables, `subst`, the queries, `match_pattern`, `match_args`, `normalize_ty` with its two limits, `self_types_overlap`; `ty_eq`, `head_eq` |
+| `selfhost/src/typeck.nova` | 1,071 | 860 | the state (`Checker`), `check_items`, `share_items`, `check_items_shared` and the splices, pass 2, arities, supertraits, records, sums, `display_ty`, `display_ty_named`, the message helpers |
+| `selfhost/src/typeck_traits.nova` | 1,094 | 1,032 | `collect_traits`, `collect_impls` and its six checks |
+| `selfhost/src/typeck_ty.nova` | 511 | 455 | `convert_ty`, projections, `find_assoc_type`, `normalize`, bounds, `where`, signatures, `component_ret`, externs and FFI |
+| `selfhost/src/fixes.nova` | 346 | 275 | `push_fix`, `name_fixes` for a type and a trait, the import fix and its edit, did-you-mean; lines.rs's placing |
+| `selfhost/src/hir_dump.nova` | 354 | 299 | `dump.rs` |
+| `selfhost/src/typeck_tests.nova` | 359 | 309 | N's 12 tests of the item passes' edges |
+
+Beside them `main.nova` gains `selfhost hir` (the resolution dump's
+command, generalised: `dump_command`, `dump_hir`, `labels_and_texts`),
+`resolver.nova` keeps std's resolution whole in `SharedStd` (`alone`) and
+gains `resolve_program_shared_for_check`, and `resolution_dump.nova`
+shares `std_start_of` and `diagnostics_text`. Code lines as 5.1 and 5.2
+counted them, no blank and no comment line: 3,376 without the tests,
+about the 2,900 the spec projected plus the dump and the command.
+
+**The idioms,** beyond ADR 0053's (an id an `Int`, -1 for none; a record
+a reference):
+
+- **Equality:** `ty_eq`, `tys_eq`, `head_eq` and `res_eq` where Rust
+  compares with the derived `==`, which a user sum lacks (E0013).
+- **Maps by `DefId`** (`sigs`, `method_locs`, `selfless`, `mut_self`,
+  `supertraits`, `type_arity`) are lists as long as the definitions,
+  indexed by the id: looked up as the maps are, and read in id order
+  where the dump sorts a map's keys, so no map's order reaches output.
+  `impl_selves` stays a `Map` (looked up only); each trait's associated
+  types are listed once (`assoc_of`), which `find_assoc_type` reads in
+  place of a scan of every definition.
+- **Composite keys:** `(item, method)` is `item * 2^20 + method`.
+- **Files:** a span carries no file in N, so the checker keeps the
+  current module's label (`cur_file`) beside `cur_module`, a span kept in
+  a table is a `FileSpan`, and a check that reads a table's spans (the
+  signature, coherence, hidden-method and supertrait checks, `normalize`
+  from them, pass 2) names the file of each diagnostic (`error_in`).
+- **Copies:** a record is a reference, so where Rust clones a scope or a
+  bound list before adding to it (`collect_impls`' methods, a trait
+  method's scope) N copies it (`copy_scope`, `copy_bounds`), and a
+  binding poisoned to `{error}` is set in the impl's own list.
+- **Not ported:** the language server's index (`note_*`,
+  `enter_type_params`, `type_param_target`) and probe; the fixes for
+  needs other than a type and a trait (5.3b's).
+- **std shared:** `share_items` checks std alone once per process and
+  mode; each program is resolved by `resolve_program_shared_for_check`
+  (std's definitions rebased and appended, `rebased_def`; std's module
+  scopes appended as they are, read with std's offset, `scope_offset`,
+  §10.1) and checked by `check_items_shared`, whose splices add std's
+  part of each table where S0's do. When std's resolution or items give a
+  diagnostic, every program is resolved and checked in full, as S0 does.
+
+### 9. The differential
+
+`N_HIR` is set: every HIR set of `crates/nova-cli/tests/selfhost.rs` is
+compared byte for byte, exit codes included, through `compare_batch`,
+and every comparison goes through `run`:
+
+- std's part natively, for the WASM target and under test mode;
+- the corpus: 251 loose programs and 74 package files natively, 39 WASM
+  programs for the WASM target, 10 files with tests under test mode, held
+  to 5.2b's floors (240, 60, 35, 8);
+- the cases: 164 under `check` (154 extracted, 10 by hand), 2 for the
+  WASM target, 1 under test mode, and all of them again with `\r\n`;
+- the type fuzzer's 120 inputs (seeds 401 to 403), part A's floors held
+  on N's dumps, which are S0's: E0001 30, E0010 16, E0012 50, E0015 11,
+  E0070 10, E0071 14, E0072 21, E0073 18, E0074 23, E0076 5, E0077 20,
+  E0078 19, E0105 4, E0403 17, E0900 46;
+- the refusals: no root under each mode, test mode for the WASM target
+  with a root or std's part, and the 15 argument shapes of
+  `both_sides_take_test_in_every_position_alike`, each through both
+  dumps; an unreadable root (`IOERR NotFound`, `IOERR InvalidData`);
+- the fallback: with a std copy whose `core` names a missing type (an
+  item E0001, no resolver diagnostic), N checks std for each program, and
+  each checked program's dump is S0's plus std's E0001.
+
+**What the comparison found:** nothing. N's first build gave S0's bytes
+on std's three parts, the cases, the corpus and the fuzzer's inputs; the
+port's only faults were compile errors (`import` and `module` are
+keywords, Nova has no `loop`). Each of the 16 item codes occurs in the
+inputs. N's own 44 tests pass (`the_selfhost_tests_pass`), 12 of them the
+item passes'.
+
+### 10. The decisions part B took
+
+The spec does not settle these; each with its cost if wrong.
+
+1. **std's module scopes are not rebased per program.** S0 appends std's
+   scopes rebased (`ModuleScope::rebased`); N appends them as they are and
+   adds std's offset where it reads one (`resolve_type_in`,
+   `resolve_trait_in`, `resolve_value_in`). Rebasing would copy std's 25
+   scopes of about a thousand names each for every program. If wrong: a
+   read of a std scope by another path would miss the offset; the
+   differential shows it at once (`component_ret` is the only reader of a
+   std scope today).
+2. **Lists by `DefId` in place of maps** (§8). If wrong: memory of a few
+   lists as long as the definitions (about 1,500 entries) per program.
+3. **`selfhost hir` shares `selfhost resolve`'s parser and refusals**, as
+   S0's two hidden flags share theirs, so the shapes compare alike.
+4. **No unshared mode in N's command line:** N's sharing was measured
+   (§11) from a temporary build that never shares, not from a flag.
+5. **The files** follow spec §3.5 (`typeck`, `typeck_traits`,
+   `typeck_ty`, `fixes`, `hir_dump`), none past 1,100 lines.
+6. **N's tests resolve their sources without std** (`check_sources`), so
+   their expected values are S0's for the same source wherever std adds
+   nothing; the import fix's test makes every module importable, which a
+   loose program's loader does not.
+
+### 11. The time, and CI's projection
+
+Measured locally on 2026-11-06, debug S0 and N with the debug runtime,
+one process a side over the loose corpus (251 roots), three runs:
+
+| Run | Wall | User |
+|---|---:|---:|
+| S0 `check --dump-hir` | 1.17-1.25 s | 1.14-1.18 s |
+| N `hir` (std shared) | 3.12-3.29 s | 3.06-3.14 s |
+| N `hir`, std resolved and checked per program | 7.47-7.72 s | 7.39-7.65 s |
+| S0 `check --dump-resolution` | 0.35-0.39 s | 0.32-0.34 s |
+| N `resolve` | 1.78-1.94 s | 1.72-1.83 s |
+
+N's HIR dump runs **2.6×** as long as S0's (the resolver's ratio is 5.3×
+here, as the research measured): the item stage itself costs N about
+1.3 s over the corpus (5 ms a root) and S0 about 0.85 s (3.4 ms a root).
+On the 74 package files N takes 9.8 s against S0's 1.6 s, as its
+resolution dump does (9.7 s against 1.4 s): 30 of them are N's own
+modules, each loading all of N, and parsing dominates.
+
+**Sharing, both sides:** N 3.1-3.3 s shared against 7.5-7.7 s unshared
+(2.4×); S0 1.25 s against 3.03 s (part A, in process, before the splice
+fix that brought shared to 1.21 s). Unshared, N spends about 17 ms more
+a root, resolving std and checking its items again; checking std's items
+once costs N about 0.02 s a process (one root's HIR dump 0.305 s against
+its resolution dump's 0.289 s). On both sides most of the saving is the
+resolution's (5.2b's): S0 measured item-level sharing alone, std's
+resolution shared either way, as roughly break-even (part A).
+
+**`selfhost.rs`, three runs each:** before part B (`819b9224`, 50 tests)
+22.87, 22.48, 23.17 s, user 72.8-73.5 s; after (51 tests) 33.77, 33.98,
+33.82 s, user 109.2-109.9 s: **+11 s wall (+48%), +36 s CPU**. Run alone,
+the HIR corpus test went from 4.8 s CPU to 27.9 s, the CRLF cases from
+0.8 to 6.6 s, the type fuzzer from 1.9 to 7.4 s, the header test from
+0.2 to 4.9 s (N's build in it); the rest is unchanged.
+
+**CI, projected (not measured):** today's CI-to-local factors, from run
+42171538582 (52.6 / 53.3 / 61.6 s against 20.5 s), are 2.57 on Linux,
+2.60 on macOS and 3.00 on Windows, which put `selfhost.rs` after 5.3a at
+about **87 s on Linux, 88 s on macOS and 102 s on Windows**, Windows past
+5.2b's 70 s line. 5.2b's lesson holds: its local projection undershot CI
+(Linux 24 to 25 s projected, 37.90 s measured), so these are a floor.
+The real figures came from the measurement branch on the user's word,
+and the user was asked with them (spec §3.7): §15.
+
+### 12. N's stack
+
+N runs on its main thread, 1 MB on Windows. Part B measured it on Linux
+(2026-11-06) under `ulimit -s` with inputs built for it (a script, not
+committed, lost with its container), each compared with S0's dump:
+
+- **Every written nesting at 128 levels** (the parser's limit): arrays,
+  a generic record's arguments, `fn` types, tuples, an E0072 between two
+  128-level types and an E0073 naming a 127-level self type: identical to
+  S0's on 1 MB, and still on 256 KB.
+- **A type built by normalization,** deeper than any written one: 64
+  chained associated types, each binding `k` arrays around the next
+  (64 × `k` levels, within the 10,000 steps), which E0072 then shows. On
+  1 MB part B's N survived `k` up to 126 (8,064 levels) and overflowed
+  (SIGSEGV, exit 139) at `k` = 127 (8,128 levels, the deepest the limits
+  admit) when its output went to a file or a pipe, as the harness's
+  does; to `/dev/null` it survived, a margin of a few frames. On 512 KB
+  it failed from 4,096 levels, on 256 KB from 2,048. S0 checks all of
+  them on its 64 MB thread. These figures are part B's, kept as its
+  record: the construct rebuilt since overflows a link earlier on both
+  runtimes (the tables below), and the latest table is the depth this
+  ADR records.
+- **A 2,000-trait supertrait chain** (`with_supertraits` is a loop): on
+  256 KB.
+
+So N overflows where S0 does not, but only on that construct: as last
+measured (the second table below), N survives 64 linked associated
+types each nesting 124 arrays (7,936 levels) on both runtimes; on the
+debug runtime it overflows at 125 (8,000 levels), and on the release
+runtime at 125 or at 126 (8,064 levels), 125 being borderline there:
+N is within a few hundred bytes of the 1 MB limit; nothing
+in the corpus, the cases or the fuzzer comes near. The user ruled
+(2026-11-06): a departure from S0, recorded here with the depth N
+survives; 5.5's driver runs N on a large-stack thread, and nothing in
+the language or the runtime changes now.
+
+**On the release runtime** (part C, 2026-11-10), since the Selfhost job
+links N against one (§15): measured on Linux, 1 MB, output to a file,
+with the construct rebuilt (a script, not committed; part B's was lost
+with its container) on a new container, so both runtimes were measured
+again side by side:
+
+| N linked against | Survives | Overflows | Least stack, `k` = 64 (4,096 levels) | Least stack, `k` = 124 (7,936 levels) |
+|---|---:|---:|---:|---:|
+| the debug runtime | `k` = 124 (7,936 levels) | `k` = 125 (8,000) | 540 KB | 1,020 KB |
+| the release runtime | `k` = 124 (7,936 levels) | `k` = 125 (8,000) | 540 KB | 1,024 KB |
+
+The least stack is found to 4 KB, each run's dump compared with S0's.
+The depth is the same: the frames are N's own, Cranelift's code for
+`normalize_ty` and `display_ty`, at about 128 bytes a level; the
+runtime's are leaves above them, the release runtime's at most 4 KB
+more on this input. The rebuilt construct overflows two arrays a link
+earlier than part B's (124 against 126 on the debug runtime), a
+difference of input or container, not of runtime.
+
+**The written nestings in the harness,** on 1 MB on every system (under
+`ulimit -s 1024` on Linux and macOS, on Windows's main thread as it
+is), on both runtimes: `n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`
+runs N's parser (`selfhost ast`) on every nesting the deep generator
+writes, at 128 levels and at 129; `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`
+runs N's item passes (`selfhost hir`) on every written nesting of a type
+(arrays, a generic record's arguments, tuples, `fn` types' parameters
+and returns, parentheses, optionals, references, mutable references,
+pointers) at 128 levels in each item position (a parameter, a return,
+a field, a variant's payload, a constant, a trait's method, an impl's
+self type, an impl's method alike and unlike its trait's, an associated
+type's binding, a `where` bound, an extern), its dump compared with
+S0's. Neither comes near the limit: N's dumps of the item test's
+inputs are S0's on 64 KB, on either runtime.
+
+**Measured again** (2026-11-10 at `4d0eca73` and 2026-11-11 at
+`5bda68ce`, earlier splits of this branch for its plan; `nova` and N
+are this branch's, byte for byte, at both; on a slower container with
+stable Rust 1.97), the same way (a script, not committed; its source is
+in the 5.3a plan, Task 5, Step 7), three runs on each day, and a
+third measurement on 2026-11-11 by the plan's second fact-check, at a
+later split whose `nova` and N are this branch's too:
+
+| N linked against | Survives | Overflows | Least stack, `k` = 64 | Least stack, `k` = 124 |
+|---|---:|---:|---:|---:|
+| the debug runtime | `k` = 124 (7,936 levels) | `k` = 125 (8,000) | 544 KB | 1,016 KB, then 1,020 KB |
+| the release runtime | `k` = 124; 125 on the first day and in the third measurement | `k` = 125 on the second day; 126 | 540 KB | 1,020 KB, then 1,016 KB |
+
+Within a link and 4 KB of the figures above, and of each other: at
+`k` = 124 and 125 N is within a few hundred bytes of the 1 MB limit,
+where a run's environment, which shares the main thread's stack, moves
+the outcome (the release runtime took `k` = 125 three times on the
+first day, failed it three times on the second, and took it again in
+the third measurement). This table is the latest measurement: N
+survives `k` = 124 on both runtimes; the debug runtime overflows at
+125, the release runtime at 125 or 126, borderline at 125.
+### 13. N's memory
+
+ADR 0054 §10's method (part B, 2026-11-06; a script, not committed,
+after 5.2b's): peak RSS and wall best of three, GC pauses from
+`nova_gc_collect_roots` to the end of the sweep under gdb, and
+`NOVA_GC_DEBUG`'s largest live heap:
+
+| Runtime | Input | Wall | Peak RSS | Pauses (median / p95 / max, total) | Max live |
+|---|---|---:|---:|---|---:|
+| debug | `resolve` N's root | 1.08 s | 45.6 MB | 12.9 / 69.3 / 83.1 ms, 376 ms | 13.3 MB |
+| debug | `hir` N's root | 1.07 s | 40.1 MB | 13.5 / 69.8 / 92.0 ms, 392 ms | 13.6 MB |
+| debug | `hir --std-part` | 0.29 s | 15.0 MB | 5.9 / 24.3 / 24.3 ms, 102 ms | 3.9 MB |
+| debug | `hir --target wasm --std-part` | 0.37 s | 18.0 MB | 7.9 / 24.1 / 32.3 ms, 130 ms | 5.6 MB |
+| debug | `hir` the loose corpus | 3.12 s | 26.2 MB | 28.3 / 32.9 / 41.8 ms, 1,140 ms | 6.8 MB |
+| release | `resolve` N's root | 0.25 s | 44.6 MB | 4.0 / 21.1 / 25.0 ms, 113 ms | 13.3 MB |
+| release | `hir` N's root | 0.26 s | 39.2 MB | 3.7 / 20.1 / 23.8 ms, 107 ms | 13.6 MB |
+| release | `hir --std-part` | 0.07 s | 14.1 MB | 1.7 / 6.8 / 6.8 ms, 27 ms | 3.9 MB |
+| release | `hir --target wasm --std-part` | 0.09 s | 17.1 MB | 2.0 / 6.5 / 9.1 ms, 35 ms | 5.6 MB |
+| release | `hir` the loose corpus | 0.76 s | 25.2 MB | 8.2 / 9.1 / 11.7 ms, 325 ms | 6.8 MB |
+
+std's item tables add little to the live heap (N's root: 13.6 MB against
+the resolution's 13.3 MB); over the corpus, std's shared state alive
+throughout, the live heap stays at 6.8 MB, but the debug runtime spends
+over a third of the run (1.14 of 3.12 s) in 46 collections. Recorded, no
+threshold (spec §3.7).
+
+**Measured again** (2026-11-10, for the plan, at `4d0eca73`, an
+earlier split of this branch whose `nova` and N are this branch's, byte
+for byte, on a slower container, the same way: a script, not committed,
+its source in the 5.3a plan, Task 5, Step 7): peak RSS within 0.2 MB of
+the table's, the same collections and live heaps, and the times longer
+by about half on this container:
+
+| Runtime | Input | Wall | Peak RSS | Pauses (median / p95 / max, total) | Max live |
+|---|---|---:|---:|---|---:|
+| debug | `resolve` N's root | 1.64 s | 45.6 MB | 20.6 / 115.5 / 124.8 ms, 601 ms | 13.3 MB |
+| debug | `hir` N's root | 1.54 s | 40.1 MB | 19.2 / 106.9 / 125.9 ms, 582 ms | 13.6 MB |
+| debug | `hir --std-part` | 0.43 s | 14.9 MB | 10.0 / 33.2 / 33.2 ms, 148 ms | 3.9 MB |
+| debug | `hir --target wasm --std-part` | 0.58 s | 18.0 MB | 12.9 / 34.5 / 48.4 ms, 211 ms | 5.6 MB |
+| debug | `hir` the loose corpus | 4.85 s | 26.2 MB | 41.5 / 54.0 / 61.2 ms, 1,709 ms | 6.8 MB |
+| release | `resolve` N's root | 0.38 s | 44.6 MB | 4.7 / 29.1 / 38.5 ms, 159 ms | 13.3 MB |
+| release | `hir` N's root | 0.37 s | 39.1 MB | 6.1 / 34.1 / 40.8 ms, 175 ms | 13.6 MB |
+| release | `hir --std-part` | 0.11 s | 14.2 MB | 2.4 / 8.6 / 8.6 ms, 37 ms | 3.9 MB |
+| release | `hir --target wasm --std-part` | 0.15 s | 17.1 MB | 3.2 / 8.1 / 12.6 ms, 50 ms | 5.6 MB |
+| release | `hir` the loose corpus | 1.08 s | 25.0 MB | 9.5 / 14.4 / 15.5 ms, 405 ms | 6.8 MB |
+
+### 14. The rule
+
+From 5.3a item collection exists twice, in `nova-typeck` (`check_items`,
+passes 2 to 5) and in `selfhost/` (§8): a change to either (a pass, a
+table, a diagnostic's words, order or fixes, `convert_ty`, normalization)
+changes both in one pull request, and the HIR dump's differential (§9)
+holds them equal (phase plan decision 6). The dump's grammar (§2) changes
+with them, on both sides.
+
+### 15. CI: `selfhost.rs` in a job of its own, on a release runtime
+
+**Measured:** CI run 42201484370 (draft PR #136, head `d8016169`,
+closed unmerged; every check green) put `selfhost.rs` at **70.80 s on
+Linux (51 tests), 86.85 s on macOS (49) and 92.54 s on Windows (45)**,
+against PR #135's 52.60, 53.25 and 61.58 s and §11's projection of 87,
+88 and 102 s: Windows past the 70 s line 5.2b set. The Test jobs took
+525, 485 and 751 s (82, 73 and 167 s of them `cargo build`), the
+Browser job 752 s.
+
+**The options measured** (2026-11-06): (a) fewer corpus roots, those
+that cover the rest; (b) fewer fuzzer inputs; (c) `selfhost.rs` in a CI
+job of its own on each system, out of the Test job; (d) N linked against
+a release runtime. For (d), `NOVA_EMBED_RUNTIME=1` makes nova-cli's
+build script build the runtime with `--release` in a nested cargo and
+embed it in the debug `nova`, which links every program it builds
+against it (`nova build --release` is LLVM through `clang`, with the
+same runtime, so it would not do): locally `selfhost.rs` went from
+33.7 s to 17.3 s (108 s to 47 s of CPU), for about 18 s more of a cold
+build (8 s the nested build, 8 s gzip of the 28.8 MB library, 2 s
+relinking `nova`), with nothing CI lacks and no change to `Cargo.lock`.
+(c) and (d) together were projected at about **36, 45 and 48 s** for
+the file in its own job, and a Test job shorter by 70 to 93 s. The
+figures of (a) and (b) were in the measurement's scratch files, lost
+with the container; neither was chosen.
+
+**The user's choice** (2026-11-06): (c) and (d), and no coverage cut:
+every root, every fuzzer input and every test of the file still run, on
+each system.
+
+**The job** (`.github/workflows/ci.yml`, `selfhost`): the Test job's
+three systems, toolchain and cache; `NOVA_EMBED_RUNTIME=1` set for the
+job alone, since in the Test job it would move `run_tests` and every
+other test that links a program onto the release runtime; `cargo build
+--locked --workspace`, then `cargo test --locked --workspace
+--all-features --no-fail-fast --test selfhost`, then N's own tests on
+the debug runtime (below). Swatinem/rust-cache keys its cache by the
+job's id, so this job's `target/`, which holds the embedded runtime,
+never reaches the Test job's.
+
+**Out of the Test job:** the target says `test = false`
+(`crates/nova-cli/Cargo.toml`), so `cargo test --workspace` skips it and
+`cargo test --workspace --test selfhost` runs it, with `Cargo.lock`
+unchanged. The other ways were worse: cargo has no way to exclude one
+test target from `cargo test --workspace` (`--exclude` drops a whole
+package, all of nova-cli's tests); libtest's `--skip` filters names
+across every binary, still builds and starts this one, and would skip
+a namesake elsewhere; and a variable that returns early would make the
+file pass without running. The cost is local: `cargo test --workspace`
+no longer runs the file, so `CONTRIBUTING.md`, `agent.md` and the pull
+request template add `cargo test --workspace --test selfhost`, and
+`cargo clippy --all-features --test selfhost -- -D warnings` beside
+their clippy command. A
+`test = false` target is also skipped by `--all-targets`, so the clippy
+job lints it by name in a second command (one command with both flags
+does not: `--all-targets` wins). The ignored-tests step loses nothing:
+the file has no `#[ignore]`d test.
+
+**Totals:** `cargo test --locked --workspace --all-features -- --list`
+on Linux lists **2,841 tests in 111 binaries** at this branch's last
+commit, where `selfhost.rs` among them would make 2,893 in 112 (part C
+counted 2,840 and 2,891, before a resolver test and the harness's
+128-level item test, §12, were added); the binary gone is nova-cli's
+`tests/selfhost.rs`, with its 52 tests (50 on macOS, 46 on Windows). A
+plan that counts the suite's totals (`S`) counts them without it from
+here, and counts `selfhost.rs`'s (`SH`, `cargo test --locked
+--workspace --test selfhost`) apart; a list compared binary by binary,
+as 5.2b's `names.py` compares them, shows the binary removed with its
+tests and nothing else.
+
+**N on the debug runtime:** the Selfhost job runs N on the release
+runtime only, and every other test that links a program runs on the
+debug runtime in the Test job; what the move alone would lose is N, a
+large program, under the debug runtime's assertions (5 `debug_assert`s,
+the collector's pages and the executor's among them) and overflow
+checks. One cheap check keeps it: the job's last step runs
+`the_selfhost_tests_pass` (`nova test` builds N with its 44 tests and
+runs them) with `NOVA_RUNTIME_LIB` naming the debug runtime that
+`cargo build --workspace` leaves beside `nova` (`libnova_runtime.a`,
+`nova_runtime.lib` on Windows), a variable that outranks the embedded
+runtime (a path that does not exist fails the test, which shows it is
+read). Measured locally (2026-11-10, debug runtime):
+
+| Check | Where | Cost |
+|---|---|---:|
+| `the_selfhost_tests_pass` | the Test job: `selfhost.rs` built there again (2.3 s) and the test | 8.2 s |
+| `the_corpus_dumps_agree` | the Test job, the same | 13.7 s |
+| both | the Test job, the same | 13.9 s |
+| `the_selfhost_tests_pass` | the Selfhost job, after its test step: nothing to build | 5.8 s |
+
+In the Test job each would put back part of what the choice took out of
+it (about 20 to 40 s on CI at §11's factors), so it runs in the Selfhost
+job, which builds nothing more for it. Not chosen: a corpus comparison
+on the debug runtime, which would build N a second time and double the
+step. If wrong: a fault only N's corpus-sized heaps reach under the
+debug runtime's checks goes unseen in CI; the release runtime runs
+those heaps every time.
+
+**The 1 MB stack tests** (`n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread`,
+N's parser, and `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`,
+N's item passes) now run on the release runtime's frames; N's depth
+there is the debug runtime's (§12).
+
+**Locally** (2026-11-10, a new container, slower than 11-06's): `SH`
+before the change 57.25 and 62.54 s (182.4 and 202.0 s of CPU); the
+job's command after it, without `NOVA_EMBED_RUNTIME`, 63.00 and 60.49 s
+(198.2 and 195.7 s), and with it 31.36, 29.04 and 28.49 s (86.5, 80.6
+and 78.5 s), **0.48 to 0.50×**, as on 11-06 (0.51×). A cold build of the
+job's test, a fresh target directory, the nested build included: 73.3 s
+(212 s of CPU).
+
+**Measured on CI** (2026-11-10, run 42774093193, draft PR #137, head
+`cc38ac37`, part C, closed unmerged; every check green): the Selfhost
+job ran `selfhost.rs` in **37.10 s on Linux (51 tests), 27.53 s on macOS
+(49) and 55.38 s on Windows (45)**, against the Test job's 70.80, 86.85
+and 92.54 s in run 42201484370 and (c) and (d)'s projection of 36, 45
+and 48 s; the job took 2m48s, 2m13s and 4m57s, its build 1m20s,
+59s and 2m19s and the test target's build 27, 21 and 54 s, and its last
+step, `the_selfhost_tests_pass` on the debug runtime, 5.49, 3.89 and
+7.35 s. The Test job took 9m38s, 7m00s and 9m02s (Windows had taken
+12m31s), and its sums were `2830 passed, 10 ignored` on Linux and `2807
+passed, 9 ignored` on Windows. Rust's cache said "No cache found" on
+that first run. Every figure is under its line (70 s on Windows, 60 s
+on Linux and macOS), so the user was not asked again. The release
+workflow ran too, as a dry run ("Publish the release" skipped). This
+branch adds two tests to these: a resolver test to the Test job's
+suite, and the 128-level item test (§12) to `selfhost.rs`.
+
+## Consequences
+
+- S0's type checker has an item stage of its own, `check_items`, which
+  `check_with` calls first (§1); `nova check`, `build`, `run`, `test` and
+  the language server give what they gave. The hidden `nova check
+  --dump-hir` prints the item stage's tables, signatures and
+  diagnostics (§2, §3), and is in no help or usage line.
+- N collects a program's items as S0 does, passes 2 to 5 function for
+  function, and prints the same dump, byte for byte, on std's three
+  parts, every corpus root in the three modes, the cases with both line
+  endings and the type fuzzer's inputs (§8, §9). 5.3b ports the bodies
+  on top of it and extends the dump to the full HIR (ADR 0056).
+- Both sides collect std's items once per process and mode, and check
+  std for each program when std alone gives an item diagnostic (§4,
+  §8).
+- Item collection exists twice from here: a change to it changes both
+  compilers in one pull request (§14).
+- `selfhost.rs` runs in a CI job of its own on each system, N linked
+  against a release runtime, and no longer in `cargo test --workspace`
+  (`test = false`): contributors run `cargo test --workspace --test
+  selfhost` and a second clippy command (§15). N's own tests still run
+  once on the debug runtime there.
+- N overflows its 1 MB main thread on a normalized type about 8,000
+  levels deep (8,000 on the debug runtime, 8,000 or 8,064 on the
+  release one), where S0 does not; 5.5's driver runs N on a large-stack thread
+  (§12). N's heap is recorded, with no threshold (§13).
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index 0a55065f..90efe1d5 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -470,6 +470,45 @@ The largest part, 7,404 code lines; its spec may split it, for example:
 - **5.3b:** traits and associated types, generics and bounds, and async
   signatures.
 
+**Amended 2026-11-06 (5.3a; spec
+`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
+§1, ADR 0055):** 5.3 is split **by stage**, not by feature, as the user
+decided on 2026-11-06. The split above cannot check one corpus program:
+every program type-checks all of std, std's bodies need the trait tables,
+impl lookup and `Iterator`, and even a user body needs std's typed impls
+(`for`, method calls, `?` on core's `Option`), so its first half would need
+a std-less mode in both compilers and a case set of its own. Item
+collection depends on nothing in bodies, so a split by stage checks all of
+std and every root at each step. The sizes are 9,246 code lines with 585
+inline tests in `nova-typeck` (not 7,404 and 394, which were v0.4.0's),
+and 810 with 30 in `nova-hir`.
+- **5.3a, the items:** passes 2 to 5 of `check_with` (S0's
+  `nova_typeck::check_items` runs passes 1 to 5, pass 1 being the language
+  server's index, which N does not port): the generic-parameter checks,
+  arities, supertraits, records, sums, traits, impls with their checks,
+  signatures, externs, bounds, `convert_ty` and `normalize`, the item
+  diagnostics (16 codes) with their fixes; HIR's types; compared through
+  the hidden `nova check --dump-hir`, items only, with std's item tables
+  built once per process and mode on both sides.
+- **5.3b, the bodies:** inference, expressions, patterns, exhaustiveness,
+  closures and the rest of the 38 codes; the full HIR dump; S0 checks
+  methods in `DefId` order.
+- **5.4's input** is the HIR the dump shows, before the driver's
+  `keep_the_entry_main`.
+
+**Done 2026-11-11 (5.3a, branch `claude/loving-archimedes-frtf5q-53a`, ADR
+0055):** N's
+item stage (`selfhost/src/hir.nova`, `typeck*.nova`, `fixes.nova`,
+`hir_dump.nova`, `selfhost hir`) gives S0's HIR dump byte for byte on
+std's three parts, every corpus root in the three modes, the cases (both
+line endings) and the type fuzzer's 120 inputs. `selfhost.rs` went from
+22.5-23.2 s to 33.8-34.0 s locally; on CI (run 42201484370) to 70.80 s on
+Linux, 86.85 s on macOS and 92.54 s on Windows. The user chose
+(2026-11-06) to run it in a CI job of its own on each system, N linked
+against a release runtime, with no coverage cut: `cargo test --workspace`
+skips it (`test = false`), `cargo test --workspace --test selfhost` runs
+it (ADR 0055 §15).
+
 - **Needs first:** 5.2; tuples and nested patterns throughout.
 - **Gate:**
   - N type-checks all of std and the corpus;
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 5e4ffb62..e7eeeebe 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -808,6 +808,19 @@ with tests, the cases of each mode and the name fuzzer's package, WASM and
 test-mode programs. The live heap of N resolving its largest inputs is
 recorded in ADR 0054 §10.
 
+**Amended 2026-11-11 (branch `claude/loving-archimedes-frtf5q-53a`, ADR
+0055):** step 4's 5.3 is split by stage
+(`docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`).
+5.3a: `selfhost/` collects the items as `nova-typeck`'s item stage does
+(records, sums, traits, impls and their checks, signatures, externs, the
+16 item codes with their fixes) and `selfhost hir` prints the HIR dump
+S0's hidden `nova check --dump-hir` prints, items only, with std's item
+tables built once per process and mode on both sides; the differential
+compares them on std's three parts, every corpus root in each mode, the
+type checker's cases and a type-level fuzzer's programs. From here a
+change to item collection changes both compilers in one pull request.
+5.3b ports the bodies and the full HIR.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/nova-spec/12-TYPESYSTEM.md b/nova-spec/12-TYPESYSTEM.md
index 8da5104b..5f34598f 100644
--- a/nova-spec/12-TYPESYSTEM.md
+++ b/nova-spec/12-TYPESYSTEM.md
@@ -49,6 +49,18 @@ ADR 0053) and HIR with the type checker (5.3).
 - Resolves trait method calls
 - Outputs: `TypedHir` where every expression has a `Type`
 
+**Amended 2026-11-11 (branch `claude/loving-archimedes-frtf5q-53a`, ADR
+0055):** the checker runs in two stages. The item stage
+(`nova_typeck::check_items`) rejects a generic parameter named `Self`,
+then collects the record and sum arities, the supertraits, records, sums,
+traits, impls (with conformance, the method signatures against the
+trait's, coherence, std's methods hiding a trait's, supertrait impls and
+cyclic associated types), every function's, method's and constant's
+signature and the externs, in that order, which is the order of its
+diagnostics; the bodies follow. The self-hosted compiler ports the item
+stage first (5.3a, `selfhost hir`, compared with the hidden `nova check
+--dump-hir`) and the bodies in 5.3b.
+
 ---
 
 ## 3. Core Type Representation
````

### Task 5: Verification, the mutants, the review, the PR, and the merge

Every step runs in a shell set up as the conventions say, the process
limit included.

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 6G
  free; if not, remove older duplicate test executables.

  ```sh
  cargo build --locked --workspace
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo clippy --locked --all-features --test selfhost -- -D warnings
  target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-53a.txt" 2>&1
  grep '^test result' "$P/full-53a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-53a.txt"
  grep -c 'Running tests/selfhost.rs' "$P/full-53a.txt"
  cargo test --locked --workspace -- --list > "$P/final-list-53a.txt" 2>&1
  cargo test --locked --workspace --test selfhost -- --list > "$P/final-list-sh-53a.txt" 2>&1
  python3 "$P/plan53a/names.py" "$P/base-list-53a.txt" "$P/final-list-53a.txt"
  comm -3 <(grep ': test$' "$P/base-list-sh-53a.txt" | sort) <(grep ': test$' "$P/final-list-sh-53a.txt" | sort)
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
mkdir -p "$(dirname "$P/plan53a/names.py")"
cat > "$P/plan53a/names.py" <<'EOF'
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
  - both formatters print nothing, and both clippy commands end with
    `Finished`, with no warning or error;
  - the suite, in 244 s: `2830 1 10`; the one failure is Task 0's IPv6
    test; `0` binaries named `tests/selfhost.rs`: the Test job's suite;
  - `names.py` prints `binaries: base 112, now 111` and `tests: base 2880,
    now 2841`, `binary removed: nova: tests/selfhost.rs (selfhost)` (its
    44, run by `SH` now), and `net added: 5`:
    - `nova_driver (unittests src/lib.rs)`, 39 to 41:
      `resolution_dumps::std_checked_once_dumps_what_std_checked_for_each_program_dumps`;
      `resolution_dumps::std_part_of_a_programs_check_is_std_part`;
    - `nova_resolver (unittests src/lib.rs)`, 69 to 70:
      `tests::std_shared_for_check_defines_what_the_full_path_does`;
    - `nova_typeck (unittests src/lib.rs)`, 585 to 587:
      `check::items::tests::check_items_is_the_prefix_of_check_with`;
      `check::items::tests::shared_items_check_what_the_full_check_does`;
  - `comm` prints the 8 tests `selfhost.rs` gains, 44 to 52, none gone:
    `every_run_of_a_hir_dump_settles_e0004s_paths_on_windows`,
    `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`,
    `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`,
    `the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps`,
    `the_hir_dump_has_a_header_per_root_and_std_its_own_part`,
    `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`,
    `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`,
    `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
- [ ] **Step 2: CI's Linux leg, as the Test job runs it.** The `test`
  job's `ubuntu-24.04` leg runs the whole suite with the golden suite
  required, with CI's own flags, `--locked --workspace --all-features
  --no-fail-fast`, `selfhost.rs` not among it:

  ```sh
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast > "$P/ci-53a.txt" 2>&1
  grep '^test result' "$P/ci-53a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/ci-53a.txt"
  grep -c 'release_golden_suite ... ok' "$P/ci-53a.txt"
  ```

  Expected, in 253 s: `2830 1 10`, Step 1's figures, the one failure the
  IPv6 test, and `1`: the golden suite ran and passed inside the whole run,
  as on CI, where Step 1's run skips it. (CI run 42774093193's Linux Test
  job, at `cc38ac37`, summed `2830 passed, 10 ignored`: one test fewer than
  here, step 0's resolver test, and the IPv6 test passing there; the PR's
  Linux Test job should sum `2831 passed, 10 ignored`, decision 23.)
- [ ] **Step 3: The golden suite.** `time G`. Expected, in 21 s of wall
  time: `release golden: 147 passed, 9 excluded, Ubuntu clang version
  18.1.3 (1ubuntu1)` and `1 passed`: unchanged, since 5.3a adds no runtime
  program.
- [ ] **Step 4: Parsing and checking every file the base tracks.** Two
  scripts, as the replay ran them, compare the base's `nova` (Task 0's
  copy, `$P/nova-base-53a`) with the tree's on every `.nova` file
  `git ls-tree -r --name-only 28f49f2e` lists, each run's output, both
  streams, and exit status compared byte for byte. 5.3a changes neither
  the lexer, the parser, nor any command's path (`check_with` calls
  `check_items`, the same passes in the same order), so nothing here
  changes. A third runs `nova test` and `nova build` on every package. All
  run from the repository's root after the build, and read `$P`.
  - `parsecmp.sh`: `nova parse --check` and `nova parse --ast` of each
    file, both on the file as `28f49f2e` has it, from a
    `git archive 28f49f2e` unpacked in `$P/pc-src-53a`, each passed the
    path relative to that root:

```sh
mkdir -p "$(dirname "$P/plan53a/parsecmp.sh")"
cat > "$P/plan53a/parsecmp.sh" <<'EOF'
#!/bin/bash
# parsecmp.sh: run from the repository's root, after the build, with $P set.
# For every `.nova` file `git ls-tree -r --name-only 28f49f2e` lists (the
# base's tracked files, so none 5.3a adds), `nova parse --check` and
# `nova parse --ast`, by the base's `nova` ($P/nova-base-53a, Task 0) and
# by the tree's `target/debug/nova`, both on the file as 28f49f2e has it,
# from a `git archive 28f49f2e` unpacked in $P/pc-src-53a: 5.3a adds
# seven files to N's sources and changes three, which the base's text keeps
# out of the comparison. Each run passes the path relative to that root and
# keeps its output, both streams, and its exit status; every pair must be
# byte-identical, since 5.3a changes neither the lexer nor the parser.
# Prints each file that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
src="$P/pc-src-53a"
rm -rf "$src"; mkdir -p "$src"
git archive 28f49f2e | tar -x -C "$src"
n=0; differ=0; nz_base=0; nz_new=0
while read -r f; do
  for mode in --check --ast; do
    a=$( (cd "$src" && "$P/nova-base-53a" parse $mode "$f") 2>&1; echo "exit=$?")
    b=$( (cd "$src" && "$nova" parse $mode "$f") 2>&1; echo "exit=$?")
    n=$((n + 1))
    if [ "$mode" = --check ]; then
      [ "$(tail -1 <<< "$a")" != exit=0 ] && nz_base=$((nz_base + 1))
      [ "$(tail -1 <<< "$b")" != exit=0 ] && nz_new=$((nz_new + 1))
    fi
    if [ "$a" != "$b" ]; then differ=$((differ + 1)); echo "differs: $mode $f"; fi
  done
done < <(git ls-tree -r --name-only 28f49f2e | grep '\.nova$')
echo "parse: $n runs over $((n / 2)) files, $differ differ; --check exits nonzero on $nz_base at the base, $nz_new now"
rm -rf "$src"
[ "$differ" = 0 ]
EOF
```

  - `checkcmp.sh`: `nova check` of each file, or of its package once from
    the package's root, and each package file alone too:

```sh
mkdir -p "$(dirname "$P/plan53a/checkcmp.sh")"
cat > "$P/plan53a/checkcmp.sh" <<'EOF'
#!/bin/bash
# checkcmp.sh (5.3a, from 5.2b's): run from the repository's root, after the build, with $P set.
# `nova check` by the base's `nova` ($P/nova-base-53a, Task 0) and by the
# tree's `target/debug/nova`, both on the tree's files, for every `.nova`
# file `git ls-tree -r --name-only 28f49f2e` lists. A file's package root
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
    # Since 5.2b: each file of a package by `nova check FILE` too, from the
    # repository's root, as the dump takes a package's root.
    a=$("$P/nova-base-53a" check "$f" 2>&1; echo "exit=$?")
    b=$("$nova" check "$f" 2>&1; echo "exit=$?")
    n=$((n + 1))
    if [ "$a" != "$b" ]; then
      differ=$((differ + 1)); echo "differs: $f (nova check FILE)"
    fi
    [ -n "${seen[$pkg]:-}" ] && continue
    seen[$pkg]=1
    a=$( (cd "$pkg" && "$P/nova-base-53a" check) 2>&1; echo "exit=$?")
    b=$( (cd "$pkg" && "$nova" check) 2>&1; echo "exit=$?")
    what="$pkg/ (package)"
  else
    a=$("$P/nova-base-53a" check "$f" 2>&1; echo "exit=$?")
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
done < <(git ls-tree -r --name-only 28f49f2e | grep '\.nova$')
echo "check: $n checks, $differ outside std differ, $stddiff std files differ"
[ "$differ" = 0 ] && [ "$stddiff" = 0 ]
EOF
```

  - `pkgcmp.sh`: `nova test` and `nova build` in a fresh copy of every
    package, by both:

```sh
mkdir -p "$(dirname "$P/plan53a/pkgcmp.sh")"
cat > "$P/plan53a/pkgcmp.sh" <<'EOF'
#!/bin/bash
# pkgcmp.sh: run from the repository's root, after the build, with $P set. For every package of the
# base's tree (a directory holding a nova.toml, of `git ls-tree -r --name-only 28f49f2e`), a copy in a
# fresh directory for each side, then `nova test` and `nova build -o OUT` in it by the base's `nova`
# ($P/nova-base-53a) and by the tree's: both streams and the exit status must be byte-identical
# (the built program's bytes are not compared), both with the tree's runtime library, which 5.3a does not change. Prints each
# that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
nova="$PWD/target/debug/nova"; repo=$PWD
work=$P/plan53a/pkgcmp; rm -rf "$work"; mkdir -p "$work"
n=0; differ=0
for m in $(git ls-tree -r --name-only 28f49f2e | grep '/nova.toml$' | grep -v '^crates/'); do
  pkg=$(dirname "$m")
  for side in base tree; do
    rm -rf "$work/$side"; mkdir -p "$work/$side"; cp -r "$pkg" "$work/$side/p"; rm -rf "$work/$side/p/target"
  done
  for cmd in test "build -o $work/OUT"; do
    a=$(cd "$work/base/p" && NOVA_RUNTIME_LIB=$repo/target/debug/libnova_runtime.a NOVA_HOME=$work/home-base "$P/nova-base-53a" $cmd 2>&1; echo "exit=$?")
    b=$(cd "$work/tree/p" && NOVA_RUNTIME_LIB=$repo/target/debug/libnova_runtime.a NOVA_HOME=$work/home-tree "$nova" $cmd 2>&1; echo "exit=$?")
    a=${a//$work\/base/W}; b=${b//$work\/tree/W}
    n=$((n + 1))
    if [ "$a" != "$b" ]; then differ=$((differ + 1)); echo "differs: $pkg: nova $cmd"; diff <(echo "$a") <(echo "$b") | head -8; fi
  done
done
echo "packages: $n runs, $differ differ"
[ "$differ" = 0 ]
EOF
```

  Then `bash "$P/plan53a/parsecmp.sh"`, `bash "$P/plan53a/checkcmp.sh"`
  and `bash "$P/plan53a/pkgcmp.sh"`. Expected, as the replay printed, in
  94, 193 and 37 s:
  - `parse: 1948 runs over 974 files, 0 differ; --check exits nonzero on
    123 at the base, 123 now`;
  - `check: 1078 checks, 0 outside std differ, 0 std files differ`;
  - `packages: 22 runs, 0 differ`.
- [ ] **Step 5: The hidden flags stay hidden.**
  - `helpcmp.sh`: `--help` of `nova` and of every command, both sides:

```sh
mkdir -p "$(dirname "$P/plan53a/helpcmp.sh")"
cat > "$P/plan53a/helpcmp.sh" <<'EOF'
#!/bin/bash
# helpcmp.sh: run from the repository's root, after the build, with $P set.
# `--help` of `nova` and of every command, by the base's `nova`
# ($P/nova-base-53a, Task 0) and the tree's `target/debug/nova`, both run
# through a link named `nova` so the usage lines agree: both streams and the
# exit status must be byte-identical, the hidden flags and `more` absent
# from `parse --help` and `check --help`.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
d="$P/help-53a"
rm -rf "$d"; mkdir -p "$d/base" "$d/tree"
ln -s "$P/nova-base-53a" "$d/base/nova"
ln -s "$PWD/target/debug/nova" "$d/tree/nova"
n=0; same=0
for cmd in "" parse run build bundle dev check test fmt doc lsp new init add remove fetch update login package publish version help; do
  a=$("$d/base/nova" $cmd --help 2>&1; echo "exit=$?")
  b=$("$d/tree/nova" $cmd --help 2>&1; echo "exit=$?")
  n=$((n + 1))
  if [ "$a" == "$b" ]; then same=$((same + 1)); else echo "differs: nova $cmd --help"; fi
done
hidden=$( ("$d/tree/nova" parse --help; "$d/tree/nova" check --help) | grep -c 'dump-\|MORE\|--std\|--test')
echo "help: $same of $n identical; hidden flags shown: $hidden"
rm -rf "$d"
[ "$same" = "$n" ] && [ "$hidden" = 0 ]
EOF
```

  - `usagecmp.sh`, with `argv0.py`: the errors `parse` and `check` give for
    the arguments they refuse, and the shapes beside them, both sides, by
    six first arguments, then in a terminal, then `--dump-ast` against
    `--dump-tokens`, then the shapes the two hidden dumps refuse:

```sh
mkdir -p "$(dirname "$P/plan53a/argv0.py")"
cat > "$P/plan53a/argv0.py" <<'EOF'
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
mkdir -p "$(dirname "$P/plan53a/usagecmp.sh")"
cat > "$P/plan53a/usagecmp.sh" <<'EOF'
#!/bin/bash
# usagecmp.sh (5.3a, from 5.2b's, with `--dump-hir`'s shapes beside `--dump-resolution`'s): run from the repository's root, after the build, with $P set
# and $P/plan53a/argv0.py written. The errors `parse` and `check` give for
# arguments they refuse, and what they print for the shapes beside them,
# from the base's `nova` ($P/nova-base-53a, Task 0) and the tree's
# `target/debug/nova`, each run by execv with the same first argument:
# `nova` (a link's name), `othernova` (a renamed binary), and four with no
# file name in UTF-8, "", "/", ".." and the bytes ff fe. Both streams and
# the exit status must be byte-identical, tips, usage lines and `--help`
# included: every `check` command line whose first argument after `check`
# is neither `--dump-resolution` nor `--dump-hir` is parsed as the base
# parses it. Then the
# same in a terminal, colours included, through `script`'s pseudo-terminal
# with NO_COLOR unset. Then 5.1's `--dump-ast` against the tree's own
# `--dump-tokens`. Then the shapes the hidden `check --dump-resolution`
# and `check --dump-hir` refuse, which the base lacks: each must exit 2
# with nothing on stdout and one error on stderr, no panic, and a refusal
# of `--dump-hir` names `--dump-hir` (in its usage line, or first in its
# own error) and never `--dump-resolution` but as the argument refused.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
work=$(mktemp -d) && cd "$work" || exit 1
shapes=("parse a b" "parse --ast a b" "parse --check a b" "parse -- a b"
  "check a b" "check a ''" "check --std" "check --std a" "check -- a b"
  "check --target wasm a b" "check a --std"
  "check --st a.nova" "check --stdd a.nova" "check --std-part a.nova" "check --dump a.nova"
  "check --dump-res a.nova" "check --dump-resolutio a.nova" "check --std=1 a.nova"
  "check --dump-resolution=1 a.nova" "check --std --std a.nova" "check a.nova b.nova --target bogus"
  "check --std --help" "check --std -h" "check a b --bogus" "check a.nova --dump-resolution"
  "check --target native --dump-resolution a.nova" "check -- --dump-resolution"
  "check --help --dump-resolution" "check -h --std"
  "check --test a" "check --test" "check a --test" "check --test --dump-resolution a" "check --tests a"
  "check --test=1 a" "check -- --test" "check --test ''" "check --test -- a" "check '' --test"
  "check --test --test a" "check a --test --dump-resolution"
  "check --dump-hi a.nova" "check --dump-hirr a.nova" "check --dump-hir=1 a.nova" "check --dump-hir= a.nova"
  "check a.nova --dump-hir" "check --target native --dump-hir a.nova" "check -- --dump-hir"
  "check --help --dump-hir" "check --test --dump-hir a" "check a --test --dump-hir")
runs=0; same=0
for arg0 in nova othernova "" / .. '\xff\xfe'; do
  for args in "${shapes[@]}"; do
    eval "python3 -I '$P/plan53a/argv0.py' '$arg0' '$P/nova-base-53a' $args" > "$P/usage-base.txt" 2>&1
    echo "exit $?" >> "$P/usage-base.txt"
    eval "python3 -I '$P/plan53a/argv0.py' '$arg0' '$nova' $args" > "$P/usage-tree.txt" 2>&1
    echo "exit $?" >> "$P/usage-tree.txt"
    runs=$((runs + 1))
    if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then same=$((same + 1)); else
      echo "differs: [$arg0] $args"; diff "$P/usage-base.txt" "$P/usage-tree.txt" | head -12; fi
  done
  echo "[$arg0] $(grep -a '^Usage:' "$P/usage-tree.txt" | head -1), $(tail -1 "$P/usage-tree.txt")"
done
tty=0; ttys=("parse a b" "parse -- a b" "check a b" "check --std a" "check --st a.nova"
  "check --std=1 a.nova" "check --std --help" "check a b --bogus" "check --test a" "check a --test")
for args in "${ttys[@]}"; do
  env -u NO_COLOR script -qc "python3 -I '$P/plan53a/argv0.py' othernova '$P/nova-base-53a' $args" /dev/null > "$P/usage-base.txt"
  env -u NO_COLOR script -qc "python3 -I '$P/plan53a/argv0.py' othernova '$nova' $args" /dev/null > "$P/usage-tree.txt"
  if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then tty=$((tty + 1)); else echo "differs in a terminal: $args"; fi
done
grep -qa $'\e\\[' "$P/usage-tree.txt" || echo "no colour in the terminal's output"
asts=0; astsame=0
for arg0 in othernova "" / .. '\xff\xfe'; do
  for args in "parse --dump-ast --ast a b" "parse --dump-ast --check a" "parse --dump-ast" "parse --dump-ast ''" "parse --dump-ast=1 a"; do
    tok=${args//dump-ast/dump-tokens}
    eval "python3 -I '$P/plan53a/argv0.py' '$arg0' '$nova' $args" > "$P/usage-base.txt" 2>&1
    echo "exit $?" >> "$P/usage-base.txt"
    eval "python3 -I '$P/plan53a/argv0.py' '$arg0' '$nova' $tok" 2>&1 | sed 's/dump-tokens/dump-ast/g' > "$P/usage-tree.txt"
    echo "exit ${PIPESTATUS[0]}" >> "$P/usage-tree.txt"
    asts=$((asts + 1))
    if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then astsame=$((astsame + 1)); else
      echo "--dump-ast differs: [$arg0] $args"; diff "$P/usage-base.txt" "$P/usage-tree.txt"; fi
  done
done
res=0; resok=0; hir=0; hirok=0
for arg0 in nova othernova "" / .. '\xff\xfe'; do
  for flag in --dump-hir; do
  for rest in "" "--std a" "--dump-hir a" "--dump-resolution a" "--target bogus a" "--std --std" "--stdd" "--help" "-h" \
              "--test --test a" "--test=1 a" "--tests a" "--test --target wasm a" "--target=wasm --test --std" \
              "--test --std a" "--test" "--test -h" "--test --std --target native a" "--target wasm --test" \
              "--test --std --test" "--target= a"; do
    eval "python3 -I '$P/plan53a/argv0.py' '$arg0' '$nova' check $flag $rest" > "$P/usage-tree.txt" 2> "$P/usage-err.txt"
    code=$?
    hir=$((hir + 1))
    named=$( (grep -q '^Usage: ' "$P/usage-err.txt" && grep -q '^Usage: nova check --dump-hir ' "$P/usage-err.txt") \
      || grep -q '^error: --dump-hir ' "$P/usage-err.txt" \
      || (grep -q "possible values" "$P/usage-err.txt" && ! grep -q '^Usage: ' "$P/usage-err.txt"); echo $?)
    stray=$(grep -v "'--dump-resolution'" "$P/usage-err.txt" | grep -c -- '--dump-resolution')
    if [ "$code" = 2 ] && [ ! -s "$P/usage-tree.txt" ] && [ "$(grep -c '^error' "$P/usage-err.txt")" = 1 ] \
       && ! grep -q panicked "$P/usage-err.txt" && [ "$named" = 0 ] && [ "$stray" = 0 ]; then hirok=$((hirok + 1)); else
      echo "--dump-hir: [$arg0] $rest: exit $code"; cat "$P/usage-tree.txt" "$P/usage-err.txt"; fi
  done
  done
  for args in "check --dump-resolution" "check --dump-resolution --std a" "check --dump-resolution --dump-resolution a" \
              "check --dump-resolution --target bogus a" "check --dump-resolution --std --std" \
              "check --dump-resolution --stdd" "check --dump-resolution --help" "check --dump-resolution -h" \
              "check --dump-resolution --test --test a" "check --dump-resolution --test=1 a" \
              "check --dump-resolution --tests a" "check --dump-resolution --test --target wasm a" \
              "check --dump-resolution --target=wasm --test --std" "check --dump-resolution --test --std a" \
              "check --dump-resolution --test" "check --dump-resolution --test -h" \
              "check --dump-resolution --test --std --target native a" "check --dump-resolution --target wasm --test" \
              "check --dump-resolution --test --std --test"; do
    eval "python3 -I '$P/plan53a/argv0.py' '$arg0' '$nova' $args" > "$P/usage-tree.txt" 2> "$P/usage-err.txt"
    code=$?
    res=$((res + 1))
    if [ "$code" = 2 ] && [ ! -s "$P/usage-tree.txt" ] && [ "$(grep -c '^error' "$P/usage-err.txt")" = 1 ] \
       && ! grep -q panicked "$P/usage-err.txt"; then resok=$((resok + 1)); else
      echo "--dump-resolution: [$arg0] $args: exit $code"; cat "$P/usage-tree.txt" "$P/usage-err.txt"; fi
  done
done
echo "usage: $same of $runs runs identical; in a terminal, $tty of ${#ttys[@]}; --dump-ast as --dump-tokens: $astsame of $asts; --dump-resolution refused: $resok of $res; --dump-hir refused: $hirok of $hir"
rm -f "$P/usage-base.txt" "$P/usage-tree.txt" "$P/usage-err.txt"; cd / && rmdir "$work"
[ "$same" = "$runs" ] && [ "$tty" = "${#ttys[@]}" ] && [ "$astsame" = "$asts" ] && [ "$resok" = "$res" ] && [ "$hirok" = "$hir" ]
EOF
```

  Then `bash "$P/plan53a/helpcmp.sh"` and `bash "$P/plan53a/usagecmp.sh"`.
  Expected, as the replay printed: `help: 22 of 22 identical; hidden flags
  shown: 0`; and, in 19 s, six lines of the usage each first argument
  gives (`[nova] Usage: nova check <FILE>, exit 2`, and so on), then
  `usage: 306 of 306 runs identical; in a terminal, 10 of 10; --dump-ast as
  --dump-tokens: 25 of 25; --dump-resolution refused: 114 of 114;
  --dump-hir refused: 126 of 126`.
- [ ] **Step 6: The lockfile and Rust 1.78.**
  - `git diff --numstat 28f49f2e -- Cargo.lock`, and `git diff --stat
    28f49f2e -- '*Cargo.toml'`. Expected: nothing, then
    ` crates/nova-cli/Cargo.toml | 10 ++++++++++` (the `[[test]]` table,
    decision 18) and ` 1 file changed, 10 insertions(+)`.
  - Then:

    ```sh
    RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-53a cargo check --locked --workspace
    rm -rf /dev/shm/msrv-53a
    ```

    Expected: clean, `Finished` in 45 s. It checks the libraries and
    binaries, as CI's MSRV job does, not the tests: `cargo check
    --workspace` compiles no test target, so the tests' Rust meets 1.78 by
    review alone (Global Constraints). Its target directory is in
    `/dev/shm`, memory, so the disk stays above 6G.
- [ ] **Step 7: The differential's time, N's tests, and N's heap and
  stack.**
  - Without the release runtime, as the build stands: `cargo test
    --locked --workspace --test selfhost --no-run` (which builds the
    target and runs nothing; `SH --no-run` would hand `--no-run` to the
    test binary, which refuses it), then three times `time SH`, then `(cd
    selfhost && ../target/debug/nova test) 2>&1 | tail -n 1`, N's tests on
    the debug runtime, the one this `nova` links against. Expected: `52
    passed; 0 failed` three times, in 41.4, 42.1 and 40.9 s (user CPU
    130.2, 132.4 and 129.9 s), and `test result: ok. 44 passed; 0 failed;
    0 trapped; 44 total`.
  - Then with it, as the Selfhost job runs, in one shell: `export
    NOVA_EMBED_RUNTIME=1`, `cargo build --locked --workspace` (the release
    runtime built and embedded, 12.5 s here, the cache warm), `cargo test
    --locked --workspace --test selfhost --no-run`, then three times `time
    SH`; then N's tests on each runtime: `(cd selfhost &&
    NOVA_RUNTIME_LIB=$PWD/../target/debug/libnova_runtime.a ../target/debug/nova
    test) 2>&1 | tail -n 1`, the debug runtime, which the variable names,
    and `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 1`, the
    release runtime embedded in this `nova`; then the job's last step,
    `time NOVA_RUNTIME_LIB=$PWD/target/debug/libnova_runtime.a cargo test
    --locked --workspace --all-features --no-fail-fast --test selfhost --
    --exact the_selfhost_tests_pass`. Expected: `52 passed; 0 failed`
    three times, in 21.9, 21.6 and 21.2 s (user CPU 58.6, 57.3 and 57.7
    s); `test result: ok. 44 passed; 0 failed; 0 trapped; 44 total` on
    each runtime; `1 passed` in 4.43 s (the test 4.30 s). The replay kept
    each run's output and its `time` in logs named by date
    (`2026-11-11-sh-noembed-1.log` and `.time`, and so on).
  - Then, in the same shell, N's live heap (ADR 0055 §13, after ADR 0054
    §10's method) and stack (§12). The scripts, as the replay ran them:

```sh
mkdir -p "$(dirname "$P/plan53a/meas.py")"
cat > "$P/plan53a/meas.py" <<'EOF'
import os, sys, time, subprocess, resource
# usage: meas.py N cmd...  -> best-of-N wall, max rss (MB) of child
n=int(sys.argv[1]); cmd=sys.argv[2:]
best=None; rss=0
for i in range(n):
    t=time.perf_counter()
    p=subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    _,status,ru=os.wait4(p.pid,0)
    w=time.perf_counter()-t
    best=w if best is None else min(best,w); rss=max(rss,ru.ru_maxrss)
print(f"{best:.3f}s {rss/1024:.1f}MB exit={os.waitstatus_to_exitcode(status)}  {' '.join(os.path.basename(c) if i==0 else c for i,c in enumerate(cmd))[:150]}")
EOF
```

```sh
mkdir -p "$(dirname "$P/plan53a/gcpause.py")"
cat > "$P/plan53a/gcpause.py" <<'EOF'
# gdb -batch -nx -x gcpause.py --args PROGRAM ARGS...: each GC pause, timed as the research
# timed them (facts-52 §6, ADR 0054 §10): from the entry of nova_gc_collect_roots (the stack scan, the
# collection's start) to the return of its caller, the runtime's `collect`, which ends with
# the mark and Pages::sweep (in the release runtime `collect` is inlined into the allocation
# that triggers it, whose return follows the sweep at once). Prints count, median, p95, max
# and total, in ms.
import gdb, time
gdb.execute("set pagination off")
times = []
class Fin(gdb.FinishBreakpoint):
    def __init__(self, frame, t0):
        super().__init__(frame, internal=True)
        self.silent = True
        self.t0 = t0
    def stop(self):
        times.append(time.perf_counter() - self.t0)
        return False
    def out_of_scope(self):
        pass
class Start(gdb.Breakpoint):
    def stop(self):
        t0 = time.perf_counter()
        Fin(gdb.newest_frame().older(), t0)
        return False
Start("*nova_gc_collect_roots", internal=True)
def report(ev):
    ms = sorted(t * 1000 for t in times)
    if not ms:
        print("GCPAUSE n=0")
        return
    p = lambda q: ms[min(len(ms) - 1, int(round(q * (len(ms) - 1))))]
    print("GCPAUSE n=%d median=%.1fms p95=%.1fms max=%.1fms total=%.0fms" % (len(ms), p(0.5), p(0.95), ms[-1], sum(ms)))
gdb.events.exited.connect(report)
gdb.execute("run %s > /dev/null" % gdb.parameter("args"))
EOF
```

```sh
mkdir -p "$(dirname "$P/plan53a/heap.sh")"
cat > "$P/plan53a/heap.sh" <<'EOF'
#!/bin/bash
# heap.sh (5.3a; 5.2b's heap.sh with ADR 0055 §13's inputs): run from the repository's root,
# after a build with NOVA_EMBED_RUNTIME=1, with $P set and $P/plan53a/meas.py and
# $P/plan53a/gcpause.py written. The live heap of N on its largest inputs, with the debug runtime
# and a release one: peak RSS and wall (best of 3, wait4, meas.py), GC pauses (gdb, gcpause.py:
# from nova_gc_collect_roots to the end of Pages::sweep), and NOVA_GC_DEBUG's largest live heap.
#   N release: nova build -o heap/selfhost-release        (the runtime NOVA_EMBED_RUNTIME embedded)
#   N debug:   NOVA_RUNTIME_LIB=target/debug/libnova_runtime.a nova build -o heap/selfhost-debug
# The loose corpus is the harness's: every tracked .nova file outside selfhost/ (but N's src/ and
# tests/) and the cases, and not directly in a package's src/ or tests/. Results in heap.txt.
: "${P:?set P to the scratch directory}"
H=$P/plan53a/heap; mkdir -p $H
repo=$PWD
(cd selfhost && ../target/debug/nova build -o $H/selfhost-release > /dev/null \
  && NOVA_RUNTIME_LIB=$repo/target/debug/libnova_runtime.a ../target/debug/nova build -o $H/selfhost-debug > /dev/null) || exit 1
git ls-files '*.nova' | python3 -c '
import os, sys
for f in sys.stdin.read().split():
    if f.startswith("crates/nova-cli/tests/selfhost/cases/"): continue
    if f.startswith("selfhost/") and not f.startswith(("selfhost/src/", "selfhost/tests/")): continue
    d = os.path.dirname(f)
    if os.path.basename(d) in ("src", "tests") and os.path.isfile(os.path.join(os.path.dirname(d), "nova.toml")): continue
    print(f)' > $H/loose.txt
echo "loose corpus: $(wc -l < $H/loose.txt) programs"
out=$P/plan53a/heap.txt; : > $out
for rt in debug release; do
  N=$H/selfhost-$rt
  for input in "resolve|selfhost/src/main.nova" "hir|selfhost/src/main.nova" "hir|--std-part" "hir|--target wasm --std-part" "hir|LOOSE"; do
    cmd=${input%%|*}; rest=${input#*|}
    [ "$rest" = LOOSE ] && rest=$(tr '\n' ' ' < $H/loose.txt)
    args="$cmd --std std $rest"
    m=$(python3 $P/plan53a/meas.py 3 $N $args | cut -d' ' -f1-3)
    g=$(gdb -batch -nx -x $P/plan53a/gcpause.py --args $N $args 2>/dev/null | grep GCPAUSE)
    live=$(NOVA_GC_DEBUG=1 $N $args 2>&1 >/dev/null | awk -F'[(]' '/nova-gc: collection/ {n++; split($2,a," "); if (a[1]>m) m=a[1]} END {printf "%d collections, max live %.1f MB", n, m/1048576}')
    echo "$rt | $cmd ${input#*|} | $m | $g | $live" | tee -a $out
  done
done
EOF
```

```sh
mkdir -p "$(dirname "$P/plan53a/stack.py")"
cat > "$P/plan53a/stack.py" <<'EOF'
"""stack.py (the prototype's c-stack.py): N's stack on a type normalization builds (ADR 0055 §12):
a trait of 64 associated types, an impl binding each to `k` arrays around
the next (`Self::A{i+1}`), the last to `k` arrays around `Int`, and a
method whose return type `Self::A0` the impl's method gets wrong, so
E0072 prints the normalized type, 64 * k levels deep.

usage: stack.py OUT_DIR N_EXE NOVA_EXE STACK_KB K...
Writes deep-K.nova, runs `N hir` under `ulimit -s STACK_KB` with stdout to
a file, and S0's `nova check --dump-hir`, and prints each K's outcome."""
import os
import subprocess
import sys

LINKS = 64


def source(k):
    lines = ["trait Deep {"]
    for i in range(LINKS):
        lines.append(f"    type A{i}")
    lines.append("    fn f(self) -> Self::A0")
    lines.append("}")
    lines.append("record W {}")
    lines.append("impl Deep for W {")
    for i in range(LINKS):
        inner = f"Self::A{i + 1}" if i + 1 < LINKS else "Int"
        lines.append(f"    type A{i} = " + "[" * k + inner + "]" * k)
    lines.append("    fn f(self) -> Bool { true }")
    lines.append("}")
    return "\n".join(lines) + "\n"


def main():
    out, n, nova, kb = sys.argv[1:5]
    std = os.path.abspath(os.path.join(os.path.dirname(nova), '..', '..', 'std'))
    for k in map(int, sys.argv[5:]):
        name = f"deep-{k}.nova"
        with open(os.path.join(out, name), "w", newline="\n") as f:
            f.write(source(k))
        n_out = os.path.join(out, f"n-{k}.txt")
        s0_out = os.path.join(out, f"s0-{k}.txt")
        with open(n_out, "wb") as f:
            n_run = subprocess.run(
                ["sh", "-c", f"ulimit -s {kb} && exec '{n}' hir --std '{std}' '{name}'"],
                cwd=out, stdout=f, stderr=subprocess.DEVNULL, stdin=subprocess.DEVNULL)
        with open(s0_out, "wb") as f:
            s0_run = subprocess.run(
                [nova, "check", "--dump-hir", name], cwd=out, stdout=f,
                stderr=subprocess.DEVNULL, stdin=subprocess.DEVNULL,
                env=dict(os.environ, NOVA_HOME=os.path.join(out, "home")))
        same = open(n_out, "rb").read() == open(s0_out, "rb").read()
        e0072 = b'"E0072"' in open(s0_out, "rb").read()
        print(f"k={k} levels={LINKS * k} stack={kb}KB N exit {n_run.returncode} "
              f"S0 exit {s0_run.returncode} E0072 {e0072} same {same}", flush=True)


main()
EOF
```

```sh
mkdir -p "$(dirname "$P/plan53a/leaststack.sh")"
cat > "$P/plan53a/leaststack.sh" <<'EOF'
#!/bin/bash
# leaststack.sh RT K: the least stack, to 4 KB, on which N (heap/selfhost-RT) prints S0's dump
# of stack.py's input for K, by bisection between 256 KB and 2048 KB. Run from the repository's root.
rt=$1; k=$2; lo=256; hi=2048
d=$P/plan53a/stack/$rt; mkdir -p "$d"
while [ $((hi - lo)) -gt 4 ]; do
  mid=$(( (lo + hi) / 2 / 4 * 4 ))
  if python3 $P/plan53a/stack.py $d $P/plan53a/heap/selfhost-$rt $PWD/target/debug/nova $mid $k | grep -q 'same True'; then hi=$mid; else lo=$mid; fi
done
echo "$rt k=$k least stack ${hi} KB"
EOF
```

    Then `bash "$P/plan53a/heap.sh"`, which builds N against each
    runtime into `$P/plan53a/heap/`; then, for each runtime `RT` of
    `debug` and `release`, `mkdir -p "$P/plan53a/stack/RT"` and `python3
    "$P/plan53a/stack.py" "$P/plan53a/stack/RT"
    "$P/plan53a/heap/selfhost-RT" "$PWD/target/debug/nova" 1024 124 125
    126`, and `bash "$P/plan53a/leaststack.sh" RT 64` and `… RT 124`.
    Expected, as the replay printed at `4d0eca73` (decision 22), which
    ADR 0055's tables "Measured again" hold: `loose corpus: 251 programs`;
    peaks 45.6, 40.1, 14.9, 18.0 and 26.2 MB on the debug runtime and
    44.6, 39.1, 14.2, 17.1 and 25.0 MB on the release one; live heaps
    13.3, 13.6, 3.9, 5.6 and 6.8 MB, 16, 16, 11, 12 and 46 collections on
    both; walls and pauses as the table, give or take this machine's
    state. On 1 MB each runtime's N prints S0's dump (`same True`) at `k`
    = 124 and overflows (`N exit -11`) at 126; at 125 the debug runtime's
    overflows, and the release runtime's either overflows or prints
    S0's dump, borderline (the replay's first day and the second
    fact-check took 125, the replay's second day did not: decision 22); the least stacks `debug k=64 least stack 544 KB`,
    `release k=64 least stack 540 KB`, and at `k` = 124 1016 or 1020 KB on
    either (the replay printed both, a day apart). A change past 4 KB or a
    link is recorded in the ledger; it is no threshold (spec §3.7).
  - Then `unset NOVA_EMBED_RUNTIME` and `cargo build --locked
    --workspace`, which ends with `Finished`: the debug-runtime `nova`
    again.
- [ ] **Step 8: The mutants.** One at a time, each edit is made, its
  command run, and the file restored with `git checkout -- <file>`, all on
  the last commit, in a shell under the process limit, with
  `NOVA_EMBED_RUNTIME=1` exported for the whole step, as the Selfhost job
  runs `selfhost.rs` (decision 21).
  - Each edit replaces one exact text, which occurs once in its file
    unless the mutant names which occurrence ("the second of two"); "the
    line goes" removes the whole line holding it, "the lines holding …
    go" every line the text touches.
  - Each command starts with `cargo build --locked --workspace`. A mutant
    of N needs no Rust rebuilt: `selfhost.rs` builds N from `selfhost/`
    each run.
  - The replay's last run took 98 minutes; a mutant of S0 about 60 s,
    one of N about 24 s.
  - Each caught mutant must fail as written: the tests named; a name
    after `the_selfhost_tests_pass` in parentheses is one of N's `@test`s,
    which that test runs. Eight pass by design, each with its reason
    below: mutants 11, 12, 50, 52, 53, 132, 136 and 143. Any other mutant
    that passes is a gap: record it in the ledger, name it in the PR, and
    give it to the review (Step 10), which says whether a check is added.
  - At the end, `git status --short` prints nothing, then `unset
    NOVA_EMBED_RUNTIME` and `cargo build --locked --workspace`, which
    rebuilds the last commit's binaries with the debug runtime.

  1. **StdStart's module printed as its def.** `crates/nova-typeck/src/dump.rs`: `d.field("module", &module.to_string());` becomes `d.field("module", &def.to_string());`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
     - Expected: fails:
       `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  2. **` mode=test` never printed.** `crates/nova-typeck/src/dump.rs`: replace

     ```
     fn mode(&mut self, test: bool) {
             if test {
     ```

     with

     ```
     fn mode(&mut self, test: bool) {
             if false {
     ```

     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
     - Expected: fails:
       `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  3. **`checked=` always true.** `crates/nova-typeck/src/dump.rs`: `d.bool_field("checked", checked.is_some());` becomes `d.bool_field("checked", true);`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
     - Expected: fails:
       `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  4. **A std id printed as the program's.** `crates/nova-typeck/src/dump.rs`: `format!("s{}", id.0 - self.std_def)` becomes `format!("u{}", id.0)`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
     - Expected: fails:
       `both_sides_take_test_in_every_position_alike`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `resolution_dumps::std_part_of_a_programs_check_is_std_part`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  5. **A type parameter numbered from 1.** `crates/nova-typeck/src/dump.rs`: `Ty::Param(i) => format!("T{i}"),` becomes `Ty::Param(i) => format!("T{}", i + 1),`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
     - Expected: fails:
       `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  6. **A projection without its associated type.** `crates/nova-typeck/src/dump.rs`: `Ty::Assoc { on, assoc } => format!("{}::{}", self.ty(on), self.id(*assoc)),` becomes `Ty::Assoc { on, assoc: _ } => self.ty(on),`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
     - Expected: fails:
       `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  7. **`Future<T>` printed as T.** `crates/nova-typeck/src/dump.rs`: `Ty::Future(out) => format!("Future<{}>", self.ty(out)),` becomes `Ty::Future(out) => self.ty(out),`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
     - Expected: fails:
       `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  8. **An array printed as its element.** `crates/nova-typeck/src/dump.rs`: `Ty::Array(elem) => format!("[{}]", self.ty(elem)),` becomes `Ty::Array(elem) => self.ty(elem),`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
     - Expected: fails:
       `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  9. **A tuple printed as ().** `crates/nova-typeck/src/dump.rs`: `Ty::Tuple(elems) => crate::tuple_text(list(elems)),` becomes `Ty::Tuple(_) => "()".to_string(),`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
     - Expected: fails:
       `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  10. **A function type's parameters joined without the space.** `crates/nova-typeck/src/dump.rs`: `format!("fn({}) -> {}", list(params).join(", "), self.ty(ret))` becomes `format!("fn({}) -> {}", list(params).join(","), self.ty(ret))`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  11. **An inference variable printed as `?`.** `crates/nova-typeck/src/dump.rs`: `Ty::Var(v) => format!("?{v}"),` becomes `Ty::Var(_) => "?".to_string(),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: passes, by design: unreachable by any input: no item pass makes an inference variable (`Ty::Var` arises only in the bodies, `check.rs` from `check_function` on, 5.3b's), so no item dump holds one.
  12. **Never printed as `never`.** `crates/nova-typeck/src/dump.rs`: `Ty::Never => "!".to_string(),` becomes `Ty::Never => "never".to_string(),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: passes, by design: unreachable by any input: `convert_ty` never gives `Ty::Never`; only a body's expression is typed `!` (5.3b's).
  13. **The error type printed as `{err}`.** `crates/nova-typeck/src/dump.rs`: `Ty::Error => "{error}".to_string(),` becomes `Ty::Error => "{err}".to_string(),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  14. **A bound's parameter numbered from 1.** `crates/nova-typeck/src/dump.rs`: `let _ = write!(self.out, " {k}");` becomes `let _ = write!(self.out, " {}", k + 1);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  15. **A record's generics printed as 0.** `crates/nova-typeck/src/dump.rs`: `self.field("generics", &r.generics.to_string());` becomes `self.field("generics", "0");`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  16. **A variant's fields after the first dropped.** `crates/nova-typeck/src/dump.rs`: `for f in &v.fields {` becomes `for f in v.fields.iter().take(1) {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  17. **Supertraits never printed.** `crates/nova-typeck/src/dump.rs`: `for &s in &tr.supertraits {` becomes `for &s in tr.supertraits.iter().take(0) {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  18. **Associated types never printed.** `crates/nova-typeck/src/dump.rs`: `for (name, assoc) in &tr.assoc_types {` becomes `for (name, assoc) in tr.assoc_types.iter().take(0) {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  19. **A trait method's has_self always true.** `crates/nova-typeck/src/dump.rs`: `self.bool_field("has_self", m.has_self);` becomes `self.bool_field("has_self", true);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  20. **A trait method's mut_self always false.** `crates/nova-typeck/src/dump.rs`: `self.bool_field("mut_self", m.mut_self);` becomes `self.bool_field("mut_self", false);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  21. **A trait method's default never printed.** `crates/nova-typeck/src/dump.rs`: `let default = self.opt_id(m.default_def);` becomes `let default = self.opt_id(None);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  22. **Every impl numbered 0.** `crates/nova-typeck/src/dump.rs`: the line holding `index += 1;` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  23. **An impl's trait printed as None.** `crates/nova-typeck/src/dump.rs`: `let tr = self.opt_id(imp.trait_id);` becomes `let tr = self.opt_id(None);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  24. **An impl's head printed as Int.** `crates/nova-typeck/src/dump.rs`: `let head = self.head(imp.self_head);` becomes `let head = self.head(TyHead::Int);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  25. **An impl's place not printed.** `crates/nova-typeck/src/dump.rs`: the line holding `write_place(&mut self.out, self.labels, span);` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  26. **An impl's methods never printed.** `crates/nova-typeck/src/dump.rs`: `for (name, def) in &imp.methods {` becomes `for (name, def) in imp.methods.iter().take(0) {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  27. **An impl's associated-type bindings never printed.** `crates/nova-typeck/src/dump.rs`: `for (assoc, ty) in &imp.assoc_bindings {` becomes `for (assoc, ty) in imp.assoc_bindings.iter().take(0) {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  28. **Signatures in the map's order.** `crates/nova-typeck/src/dump.rs`: the line holding `sigs.sort_by_key(|id| id.0);` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `resolution_dumps::std_part_of_a_programs_check_is_std_part`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  29. **`selfless=` always false.** `crates/nova-typeck/src/dump.rs`: `self.bool_field("selfless", t.selfless.contains(&id));` becomes `self.bool_field("selfless", false);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  30. **A signature's `mut_self=` always false.** `crates/nova-typeck/src/dump.rs`: `self.bool_field("mut_self", t.mut_self.contains(&id));` becomes `self.bool_field("mut_self", false);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  31. **An extern's ABI always C.** `crates/nova-typeck/src/dump.rs`: `nova_hir::ExternAbi::Js => "Js",` becomes `nova_hir::ExternAbi::Js => "C",`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  32. **An extern's symbol printed empty.** `crates/nova-typeck/src/dump.rs`: `self.string(&e.symbol);` becomes `self.string("");`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  33. **std's part counts the program's impls.** `crates/nova-typeck/src/dump.rs`: `&count(tables.impl_in_std.iter().filter(|s| **s).count()),` becomes `&count(tables.impl_in_std.iter().filter(|s| !**s).count()),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `resolution_dumps::std_part_of_a_programs_check_is_std_part`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  34. **A program's part holds std's entries too.** `crates/nova-typeck/src/dump.rs`: `let ours = move |id: DefId| (id.0 >= std_def) == std;` becomes `let ours = move |id: DefId| id.0 >= std_def || !std;`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  35. **check_with drops the item diagnostics.** `crates/nova-typeck/src/check/items.rs`: `self.diagnostics = tables.diagnostics;` becomes `self.diagnostics = Vec::new();`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::check_items_is_the_prefix_of_check_with`.
  36. **check_items skips the generic-parameter checks.** `crates/nova-typeck/src/check/items.rs`: replace

      ```
              self.reject_self_type_params();
              self.collect_type_arities();
      ```

      with

      ```
              self.collect_type_arities();
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  37. **std's arities not spliced.** `crates/nova-typeck/src/check/items.rs`: the line holding `self.splice(Splice::Arities);` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `resolution_dumps::std_checked_once_dumps_what_std_checked_for_each_program_dumps`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  38. **std's supertraits not spliced.** `crates/nova-typeck/src/check/items.rs`: the line holding `self.splice(Splice::Supertraits);` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `resolution_dumps::std_checked_once_dumps_what_std_checked_for_each_program_dumps`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  39. **std's records not spliced.** `crates/nova-typeck/src/check/items.rs`: the line holding `self.splice(Splice::Records);` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::shared_items_check_what_the_full_check_does`.
  40. **std's sums not spliced.** `crates/nova-typeck/src/check/items.rs`: the line holding `self.splice(Splice::Sums);` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::shared_items_check_what_the_full_check_does`.
  41. **std's signatures not spliced.** `crates/nova-typeck/src/check/items.rs`: the line holding `self.splice(Splice::Signatures);` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::shared_items_check_what_the_full_check_does`.
  42. **std's externs not spliced.** `crates/nova-typeck/src/check/items.rs`: the line holding `self.splice(Splice::Externs);` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::shared_items_check_what_the_full_check_does`.
  43. **std's impls not spliced.** `crates/nova-typeck/src/check/items.rs`: the line holding `self.impls.extend(std.impls.iter().map(|i| r.imp(i)));` goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `resolution_dumps::std_checked_once_dumps_what_std_checked_for_each_program_dumps`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  44. **std's ids not rebased.** `crates/nova-typeck/src/check/items.rs`: `DefId(id.0 + self.def)` becomes `DefId(id.0)`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `resolution_dumps::std_checked_once_dumps_what_std_checked_for_each_program_dumps`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  45. **A std impl's head not rebased.** `crates/nova-typeck/src/check/items.rs`: `TyHead::Sum(id) => TyHead::Sum(self.id(id)),` becomes `TyHead::Sum(id) => TyHead::Sum(id),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::shared_items_check_what_the_full_check_does`.
  46. **A std method's item not rebased.** `crates/nova-typeck/src/check/items.rs`: `item_index: loc.item_index + r.item,` becomes `item_index: loc.item_index,`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::shared_items_check_what_the_full_check_does`.
  47. **A std impl's self not rebased by item.** `crates/nova-typeck/src/check/items.rs`: `self.impl_selves.insert(item + r.item, s);` becomes `self.impl_selves.insert(item, s);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::shared_items_check_what_the_full_check_does`.
  48. **std's items shareable despite a diagnostic.** `crates/nova-typeck/src/check/items.rs`: `shareable: tables.diagnostics.is_empty(),` becomes `shareable: true,`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::shared_items_check_what_the_full_check_does`.
  49. **A std trait's methods not rebased.** `crates/nova-typeck/src/check/items.rs`: `default_def: m.default_def.map(|d| self.id(d)),` becomes `default_def: m.default_def,`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `check::items::tests::shared_items_check_what_the_full_check_does`.
  50. **HirDumps shares std's items whatever they gave.** `crates/nova-driver/src/lib.rs`: `let shared = front.std.shareable() && items.shareable();` becomes `let shared = front.std.shareable();`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: passes, by design: unreachable by any input: S0's std is built into `nova`, and today's std gives no item diagnostic (std's three parts end in an empty `(Diagnostics)`), so `items.shareable()` is true for every run and never decides the conjunction. `SharedItems::shareable` itself is pinned by `shared_items_check_what_the_full_check_does` (mutant 48), and N's fallback by the harness (mutants 134 and 135).
  51. **A program with an error still checked.** `crates/nova-driver/src/lib.rs`: `let Some(resolved) = front.resolved.filter(|_| !stopped) else {` becomes `let Some(resolved) = front.resolved.filter(|_| true) else {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  52. **A dependency's item warnings kept.** `crates/nova-driver/src/lib.rs`: replace

      ```
                          .filter(|d| !program::a_dependencys_warning(d, front.registry))
      ```

      with nothing.

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: passes, by design: unreachable by any input: no case, corpus file or fuzz input holds a registry package (every package depends by path, and tests never touch the internet); the resolution dump's same filter, 5.2b's, is unreached alike.
  53. **std's part drops the item diagnostics.** `crates/nova-driver/src/lib.rs`: replace

      ```
          diagnostics.extend(tables.diagnostics.iter().cloned());
          nova_typeck::dump::dump_std(
      ```

      with

      ```
          nova_typeck::dump::dump_std(
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: passes, by design: unreachable by any input: today's std gives no item diagnostic, so std's part has none to drop (as mutant 50); N's twin, mutant 138, is caught with an altered std, which S0's built-in std cannot be given.
  54. **HirDumps checks std in full always.** `crates/nova-driver/src/lib.rs`: replace

      ```
              let resolve = if self.shared {
                  Resolve::SharedForCheck
      ```

      with

      ```
              let resolve = if false {
                  Resolve::SharedForCheck
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `resolution_dumps::std_checked_once_dumps_what_std_checked_for_each_program_dumps`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  55. **An associated type's trait not rebased.** `crates/nova-resolver/src/lib.rs`: `trait_def: DefId(trait_def.0 + def),` becomes `trait_def: DefId(trait_def.0),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `resolution_dumps::std_checked_once_dumps_what_std_checked_for_each_program_dumps`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  56. **A std scope's variant not rebased.** `crates/nova-resolver/src/lib.rs` (the second of 2): `Res::Variant(sum, index) => Res::Variant(def(sum), index),` becomes `Res::Variant(sum, index) => Res::Variant(sum, index),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `tests::std_shared_for_check_defines_what_the_full_path_does`.
  57. **std's item modules not rebased.** `crates/nova-resolver/src/lib.rs`: `.extend(std.item_module.iter().map(|m| m + module_offset));` becomes `.extend(std.item_module.iter().copied());`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `resolution_dumps::std_checked_once_dumps_what_std_checked_for_each_program_dumps`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  58. **StdStart's item counted from the program's first item.** `crates/nova-resolver/src/dump.rs` (the first of 3): `.position(|&m| m as usize >= std_module)` becomes `.position(|&m| m as usize >= std_module + 1)`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `resolution_dumps::std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`.
  59. **The HIR dump's refusals name the resolution flag.** `crates/nova-cli/src/cmd/run.rs`: `DumpKind::Hir => "--dump-hir",` becomes `DumpKind::Hir => "--dump-resolution",`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps`.
  60. **The HIR dump's usage line is the resolution dump's.** `crates/nova-cli/src/cmd/run.rs`: `let name = "nova check --dump-hir";` becomes `let name = "nova check --dump-resolution";`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps`.
  61. **`--dump-hir --std` prints std's resolution part.** `crates/nova-cli/src/cmd/run.rs`: `DumpKind::Hir => nova_driver::dump_std_hir(target, args.test),` becomes `DumpKind::Hir => nova_driver::dump_std_resolution(target, args.test),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  62. **`--dump-hir` not taken.** `crates/nova-cli/src/cmd/run.rs`: `if flag != "--dump-hir" {` becomes `if flag != "--dump-hirx" {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  63. **`--dump-hir --test --target wasm` taken.** `crates/nova-cli/src/cmd/run.rs`: `if args.test && target == nova_driver::Target::Wasm {` becomes `if args.test && target == nova_driver::Target::Wasm && cmd.kind == DumpKind::Resolution {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- items::tests resolution_dumps dump::tests std_shared_for_check` and `SH`, each to its end.
      - Expected: fails:
        `both_sides_refuse_no_file`, `both_sides_take_test_in_every_position_alike`, `the_hir_dump_flag_is_hidden_and_refused_as_the_resolution_dumps`.
  64. **N's E0001 site gives E0999.** `selfhost/src/typeck.nova`: `"E0001"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  65. **N's E0076 site gives E0999.** `selfhost/src/typeck.nova`: `"E0076"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `generic_parameters_named_self_or_twice_are_reported_by_pass`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  66. **N's E0092 site gives E0999.** `selfhost/src/typeck.nova`: `"E0092"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  67. **N's E0403 site gives E0999.** `selfhost/src/typeck.nova`: `"E0403"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `generic_parameters_named_self_or_twice_are_reported_by_pass`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  68. **N's E0900 site gives E0999.** `selfhost/src/typeck.nova` (the first of 2): `"E0900"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  69. **N's E0900 site gives E0999.** `selfhost/src/typeck.nova` (the second of 2): `"E0900"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  70. **N's E0001 site gives E0999.** `selfhost/src/typeck_ty.nova` (the first of 4): `"E0001"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  71. **N's E0001 site gives E0999.** `selfhost/src/typeck_ty.nova` (the second of 4): `"E0001"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `an_unknown_type_offers_an_import_or_the_nearest_name`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  72. **N's E0001 site gives E0999.** `selfhost/src/typeck_ty.nova` (the third of 4): `"E0001"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  73. **N's E0001 site gives E0999.** `selfhost/src/typeck_ty.nova` (the fourth of 4): `"E0001"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  74. **N's E0012 site gives E0999.** `selfhost/src/typeck_ty.nova` (the first of 5): `"E0012"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  75. **N's E0012 site gives E0999.** `selfhost/src/typeck_ty.nova` (the second of 5): `"E0012"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  76. **N's E0012 site gives E0999.** `selfhost/src/typeck_ty.nova` (the third of 5): `"E0012"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `a_types_arguments_are_counted`).
  77. **N's E0012 site gives E0999.** `selfhost/src/typeck_ty.nova` (the fourth of 5): `"E0012"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `a_types_arguments_are_counted`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  78. **N's E0012 site gives E0999.** `selfhost/src/typeck_ty.nova` (the fifth of 5): `"E0012"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `a_types_arguments_are_counted`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  79. **N's E0015 site gives E0999.** `selfhost/src/typeck_ty.nova`: `"E0015"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  80. **N's E0078 site gives E0999.** `selfhost/src/typeck_ty.nova`: `"E0078"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  81. **N's E0900 site gives E0999.** `selfhost/src/typeck_ty.nova` (the first of 4): `"E0900"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  82. **N's E0900 site gives E0999.** `selfhost/src/typeck_ty.nova` (the second of 4): `"E0900"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  83. **N's E0900 site gives E0999.** `selfhost/src/typeck_ty.nova` (the third of 4): `"E0900"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  84. **N's E0900 site gives E0999.** `selfhost/src/typeck_ty.nova` (the fourth of 4): `"E0900"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  85. **N's E0001 site gives E0999.** `selfhost/src/typeck_traits.nova`: `"E0001"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  86. **N's E0010 site gives E0999.** `selfhost/src/typeck_traits.nova` (the first of 2): `"E0010"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  87. **N's E0010 site gives E0999.** `selfhost/src/typeck_traits.nova` (the second of 2): `"E0010"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  88. **N's E0070 site gives E0999.** `selfhost/src/typeck_traits.nova`: `"E0070"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `an_impl_missing_items_names_each_kind_and_a_foreign_one`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  89. **N's E0071 site gives E0999.** `selfhost/src/typeck_traits.nova` (the first of 3): `"E0071"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  90. **N's E0071 site gives E0999.** `selfhost/src/typeck_traits.nova` (the second of 3): `"E0071"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `an_impl_missing_items_names_each_kind_and_a_foreign_one`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  91. **N's E0071 site gives E0999.** `selfhost/src/typeck_traits.nova` (the third of 3): `"E0071"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `an_impl_missing_items_names_each_kind_and_a_foreign_one`).
  92. **N's E0072 site gives E0999.** `selfhost/src/typeck_traits.nova` (the first of 8): `"E0072"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  93. **N's E0072 site gives E0999.** `selfhost/src/typeck_traits.nova` (the second of 8): `"E0072"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  94. **N's E0072 site gives E0999.** `selfhost/src/typeck_traits.nova` (the third of 8): `"E0072"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `a_methods_signature_is_compared_with_its_traits`).
  95. **N's E0072 site gives E0999.** `selfhost/src/typeck_traits.nova` (the fourth of 8): `"E0072"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `a_methods_signature_is_compared_with_its_traits`).
  96. **N's E0072 site gives E0999.** `selfhost/src/typeck_traits.nova` (the fifth of 8): `"E0072"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  97. **N's E0072 site gives E0999.** `selfhost/src/typeck_traits.nova` (the sixth of 8): `"E0072"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  98. **N's E0072 site gives E0999.** `selfhost/src/typeck_traits.nova` (the seventh of 8): `"E0072"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `a_methods_signature_is_compared_with_its_traits`).
  99. **N's E0072 site gives E0999.** `selfhost/src/typeck_traits.nova` (the eighth of 8): `"E0072"` becomes `"E0999"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `a_methods_signature_is_compared_with_its_traits`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  100. **N's E0073 site gives E0999.** `selfhost/src/typeck_traits.nova`: `"E0073"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  101. **N's E0074 site gives E0999.** `selfhost/src/typeck_traits.nova` (the first of 3): `"E0074"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  102. **N's E0074 site gives E0999.** `selfhost/src/typeck_traits.nova` (the second of 3): `"E0074"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `overlapping_inherent_impls_are_labelled_at_both`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  103. **N's E0074 site gives E0999.** `selfhost/src/typeck_traits.nova` (the third of 3): `"E0074"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  104. **N's E0077 site gives E0999.** `selfhost/src/typeck_traits.nova`: `"E0077"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `a_binding_in_terms_of_itself_is_e0077_and_poisoned`), `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  105. **N's E0105 site gives E0999.** `selfhost/src/typeck_traits.nova` (the first of 2): `"E0105"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  106. **N's E0105 site gives E0999.** `selfhost/src/typeck_traits.nova` (the second of 2): `"E0105"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  107. **N's E0403 site gives E0999.** `selfhost/src/typeck_traits.nova` (the first of 5): `"E0403"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  108. **N's E0403 site gives E0999.** `selfhost/src/typeck_traits.nova` (the second of 5): `"E0403"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  109. **N's E0403 site gives E0999.** `selfhost/src/typeck_traits.nova` (the third of 5): `"E0403"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  110. **N's E0403 site gives E0999.** `selfhost/src/typeck_traits.nova` (the fourth of 5): `"E0403"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  111. **N's E0403 site gives E0999.** `selfhost/src/typeck_traits.nova` (the fifth of 5): `"E0403"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  112. **N's E0900 site gives E0999.** `selfhost/src/typeck_traits.nova` (the first of 2): `"E0900"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  113. **N's E0900 site gives E0999.** `selfhost/src/typeck_traits.nova` (the second of 2): `"E0900"` becomes `"E0999"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  114. **N's import fix titled otherwise.** `selfhost/src/fixes.nova`: replace

       ```
       Some(Fix { title: "import `${x}` from `${imp}`", edits: edits })
       ```

       with

       ```
       Some(Fix { title: "import `${x}`", edits: edits })
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `an_unknown_type_offers_an_import_or_the_nearest_name`).
  115. **N's import edit into a list without the comma.** `selfhost/src/fixes.nova`: `return Some(Edit::insert(names.last().span.end, file, ", ${x}"))` becomes `return Some(Edit::insert(names.last().span.end, file, " ${x}"))`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  116. **N's new import line without braces.** `selfhost/src/fixes.nova`: `let line = "import ${imp}::{${x}}"` becomes `let line = "import ${imp}::${x}"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `an_unknown_type_offers_an_import_or_the_nearest_name`).
  117. **N's line after the last import, the break on the wrong side.** `selfhost/src/fixes.nova`: `if next == n && !text.ends_with("\n") { (next, "${nl}${line}") } else { (next, "${line}${nl}") }` becomes `if next == n && !text.ends_with("\n") { (next, "${line}${nl}") } else { (next, "${line}${nl}") }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_selfhost_tests_pass` (N's `@test` `an_imports_edit_goes_after_the_last_import_or_above_the_comments`).
  118. **N's line before the first item without the blank line.** `selfhost/src/fixes.nova`: `(at, "${line}${nl}${nl}")` becomes `(at, "${line}${nl}")`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_selfhost_tests_pass` (N's `@test` `an_imports_edit_goes_after_the_last_import_or_above_the_comments`, `an_unknown_type_offers_an_import_or_the_nearest_name`).
  119. **N's line before the first item not above its comments.** `selfhost/src/fixes.nova`: replace

       ```
               at = line_start(text, start)
           }
           (at,
       ```

       with

       ```
           }
           (at,
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_selfhost_tests_pass` (N's `@test` `an_imports_edit_goes_after_the_last_import_or_above_the_comments`).
  120. **N's fixes always end lines with \n.** `selfhost/src/fixes.nova`: `pub fn ending(text: String) -> String { if text.contains("\r\n") { "\r\n" } else { "\n" } }` becomes `pub fn ending(text: String) -> String { "\n" }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `an_imports_edit_goes_after_the_last_import_or_above_the_comments`).
  121. **N's did-you-mean offers no primitive type.** `selfhost/src/fixes.nova`: `NeedType => for p in PRIMITIVES.iter() { candidates.push(p) }` becomes `NeedType => ()`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass` (N's `@test` `an_unknown_type_offers_an_import_or_the_nearest_name`).
  122. **N's did-you-mean ignores the type parameters in scope.** `selfhost/src/fixes.nova`: the line holding `for n in nearby.iter() { candidates.push(n) }` goes.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  123. **N's fixes kept in a module not the project's.** `selfhost/src/fixes.nova`: the line holding `if !self.owned(self.cur_module) { return }` goes.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  124. **N's import fix offered when the name clashes.** `selfhost/src/fixes.nova`: the line holding `if clash { return None }` goes.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  125. **N's import fix offered among two modules.** `selfhost/src/fixes.nova`: `if found.len() != 1 { return None }` becomes `if found.is_empty() { return None }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  126. **N prints a type parameter from 1.** `selfhost/src/hir_dump.nova`: `TyParam(i) => "T${i}"` becomes `TyParam(i) => "T${i + 1}"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  127. **N prints a std id as the program's.** `selfhost/src/hir_dump.nova`: `if id >= self.std_def { "s${id - self.std_def}" } else { "u${id}" }` becomes `"u${id}"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `both_sides_take_test_in_every_position_alike`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  128. **N's trait methods' has_self always true.** `selfhost/src/hir_dump.nova`: `self.bool_field("has_self", m.has_self)` becomes `self.bool_field("has_self", true)`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  129. **N's signatures' selfless always false.** `selfhost/src/hir_dump.nova`: `self.bool_field("selfless", t.selfless.at(id))` becomes `self.bool_field("selfless", false)`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  130. **N's impl's head printed as Int.** `selfhost/src/hir_dump.nova`: `self.field("head", self.head_text(imp.self_head))` becomes `self.field("head", "Int")`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  131. **N's std part counts no externs.** `selfhost/src/hir_dump.nova`: `d.field("externs", "${externs}")` becomes `d.field("externs", "0")`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `both_sides_take_test_in_every_position_alike`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  132. **N's never printed as `never`.** `selfhost/src/hir_dump.nova`: `TyNever => "!"` becomes `TyNever => "never"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: passes, by design: unreachable by any input, as mutant 12: N's item passes never make `TyNever`.
  133. **N's StdStart item printed as its def.** `selfhost/src/hir_dump.nova`: `d.field("item", "${item}")` becomes `d.field("item", "${def}")`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  134. **N shares std's items whatever they gave.** `selfhost/src/main.nova`: `if shared.shareable && s.shareable { items = Some(s) }` becomes `if shared.shareable { items = Some(s) }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`.
  135. **N's std items shareable despite a diagnostic.** `selfhost/src/typeck.nova`: `shareable: tables.diagnostics.is_empty() }` becomes `shareable: true }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`.
  136. **N's std method items not rebased.** `selfhost/src/typeck.nova`: `item_index: loc.item_index + self.rebase_item,` becomes `item_index: loc.item_index,`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: passes, by design: unreachable by any input: a std method's place (`method_locs`' item index) is read only by the bodies (5.3b's), so no item dump shows it; S0's twin, mutant 46, is caught by `shared_items_check_what_the_full_check_does`'s comparison of every table, which N's tests, checking no std, have no counterpart for. 5.3b's differential reads it.
  137. **N's std scope lookups not offset.** `selfhost/src/typeck.nova`: `if m >= self.defs.std_start { self.scope_offset } else { 0 }` becomes `0`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  138. **N's std part drops the item diagnostics.** `selfhost/src/main.nova`: replace

       ```
                   for d in tables.diagnostics.iter() { diagnostics.push(d) }
                   print(dump_hir_std(
       ```

       with

       ```
                   print(dump_hir_std(
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`.
  139. **N's hir command prints the resolution part for --std-part.** `selfhost/src/main.nova`: replace

       ```
           let hir = command == "hir"
           if std_part {
       ```

       with

       ```
           let hir = false
           if std_part {
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `both_sides_take_test_in_every_position_alike`, `n_checks_items_holding_128_levels_of_every_kind_on_a_1_mb_main_thread`, `n_checks_std_for_each_program_when_std_gives_an_item_diagnostic`, `the_hir_dump_has_a_header_per_root_and_std_its_own_part`, `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  140. **N's side not compared (N_HIR false).** `crates/nova-cli/tests/selfhost.rs`: `const N_HIR: bool = true;` becomes `const N_HIR: bool = false;`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `selfhost.rs` does not compile (`error[E0080]`: the constant's assertion, `5.3a's part B compares N's side`).
  141. **The WASM mode reads the check cases.** `crates/nova-cli/tests/selfhost.rs`: `Mode::Wasm => "check-wasm",` becomes `Mode::Wasm => "check",`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  142. **The HIR dump not settled on Windows.** `crates/nova-cli/tests/selfhost.rs`: replace

       ```
               || args.starts_with(HIR_S0)
       ```

       with nothing.

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `every_run_of_a_hir_dump_settles_e0004s_paths_on_windows`.
  143. **The WASM dumps' codes not counted among the item codes.** `crates/nova-cli/tests/selfhost.rs`: `for (code, n) in codes_in(&wasm_dumps) {` becomes `for (code, n) in codes_in("") {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: passes, by design: equivalent: E0092 also occurs in the dumps under `check` (the resolver's, a std module the native target lacks), so the codes' floor holds without the WASM dumps' counts; the item passes' E0092 is asserted on a line of its own (`no item E0092`).
  144. **The package corpus left out of the HIR pass.** `crates/nova-cli/tests/selfhost.rs` (the second of 2): replace

       ```
               check.push_str(&package_dumps.join().expect("the packages' thread"));
       ```

       with nothing.

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`.
  145. **The type fuzzer's E0076 item no longer aims at E0076.** `crates/nova-cli/tests/selfhost/fuzz.rs`: `("E0076", "fn fz_self{n}<Self>() {}"),` becomes `("E0076", "fn fz_self{n}<T>() {}"),`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  146. **A type-fuzz item that stops the program in the resolver.** `crates/nova-cli/tests/selfhost/fuzz.rs`: `("E0403", "record FzDup{n}<T, T> { t: T }"),` becomes `("E0403", "record FzDup{n}<T, T> { t: T }\nfn main() {}"),`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  147. **The package cases not compared.** `crates/nova-cli/tests/selfhost.rs`: replace

       ```
           let from_cases: Vec<String> = roots.loose.into_iter().chain(roots.packages).collect();
           let mut all = agree_on_hir_in(
       ```

       with

       ```
           let from_cases: Vec<String> = roots.loose.into_iter().collect();
           let mut all = agree_on_hir_in(
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_hir_dumps_agree_on_std_the_corpus_and_the_cases`, `the_hir_dumps_agree_on_the_cases_with_windows_line_endings`.
  148. **A seed's inputs counted short.** `crates/nova-cli/tests/selfhost.rs`: `.map(|&seed| (seed, type_inputs(&bases, seed, TYPE_COUNT)))` becomes `.map(|&seed| (seed, type_inputs(&bases, seed, TYPE_COUNT - 1)))`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  149. **The type fuzzer makes one edit an input.** `crates/nova-cli/tests/selfhost/fuzz.rs`: `for step in 0..1 + rng.below(3) {` becomes `for step in 0..1 {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_type_fuzzers_item_inputs_agree_and_hold_each_aimed_code`.
  150. **selfhost.rs back in `cargo test --workspace`.** `crates/nova-cli/Cargo.toml`: the `[[test]]` table's line `test = false` goes (the table's line, the last of the table; the comment above the table, which also holds `test = false`, stays).
       - Run `cargo build --locked --workspace`, then `cargo test --locked --workspace -- --list | grep -c the_hir_dumps_agree_on_std`.
       - Expected: `1`, where the last commit gives `0`: `selfhost.rs` is back among the targets `cargo test --workspace` runs, which Step 1's `names.py` shows too (the `selfhost` binary listed again).
- [ ] **Step 9: The review's inputs.** The full run's figures, the new
  tests by binary and `selfhost.rs`'s apart, the golden suite's line and
  time, the parse, check, help and usage comparisons, the differential's
  times, the heap and the stack, and the mutants' outcome, with each gap,
  go in the ledger.
- [ ] **Step 10: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0055 and this plan, and is told to hunt for:
    - a Windows-only difference: a path that reaches a HIR dump or a
      message N compares beyond E0004's and E0093's, or a comparison of
      S0's and N's output not taken from `run` (Review Focus 1);
    - N's messages and fixes against S0's, word for word, at every site of
      the 16 codes, the import fix's edit and its line ending, "did you
      mean" (Review Focus 2);
    - std's sharing: anything `check_items_shared` or N's shared path does
      that the full one does not, a splice in the wrong place or order, a
      rebasing offset, a shared state written by a program, the fallback
      when std gives an item diagnostic (Review Focus 3);
    - anything `check_items` changes for `check`, `build`, `test` or the
      language server (Review Focus 4);
    - argument handling: the hidden flag shown anywhere, a refusal that
      names the wrong flag, any command's `--help` changed (Review Focus
      5);
    - a way for the harness to pass when the two sides differ: a mode's
      comparison skipped, a root or a case directory not compared, a part
      not counted, a floor dropped, a code asserted absent, the fuzzer's
      inputs stopping before the type checker (Review Focus 6);
    - the CI job: a system or step missing, `NOVA_EMBED_RUNTIME` leaking
      into the Test job, N's tests not run on the debug runtime, clippy
      not linting `selfhost.rs`, a document naming the wrong command
      (Review Focus 7; decisions 12, 17 and 18);
    - N's stack and memory figures against ADR 0055 §§12 and 13 (Review
      Focus 8; decision 22);
    - a commit between Task 1 and Task 4 that builds wrong code or fails
      the suite, and the one version the head does not have (decision 16);
    - the records: ADR 0055, the CHANGELOG, the dated notes, every count
      (251 loose and 74 package files natively, 39 WASM, 10 with tests,
      164, 154, 16; the spec's 318, 67 and 9 are the base's) and figure
      against the tree, the
      CI figures of runs 42201484370 and 42774093193 (decisions 11 and 14);
    - each mutant of Step 8 that passes, the accepted ones included, and
      whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock` or to a manifest
      beyond decision 18's table, a test that touches the internet, a
      temporary name that is not fixed or missing from the Global
      Constraints' list.
  - Each finding is fixed in one commit, `5.3a: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Steps 1 and 2 run again, and `SH`.
- [ ] **Step 11: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-53a`.
  - Open the PR to `main` with the repository's template headings: its
    Testing list now names `cargo test --workspace --test selfhost` and
    the second clippy command (Task 3). The body names no local path, no
    person and no model ID. It ends with this line, a blank line, and the
    session's address, exactly:

    ```
    🤖 Generated with [Claude Code](https://claude.com/claude-code)

    https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
    ```

    Every commit the PR carries ends with the two trailer lines of the
    Conventions (`Co-Authored-By: Claude Opus 5.5
    <noreply@anthropic.com>` and `Claude-Session:
    https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN`): check with
    `git log --format=%B 28f49f2e..HEAD` before the push.
  - It holds:
    - that its first commit is the approved spec's (`28f49f2e`), then the
      plan's (decision 19);
    - what 5.3a adds: S0's `check_items`, the hidden `nova check
      --dump-hir` and std's item tables shared; N's item passes, its dump
      and `selfhost hir`; the cases, the type fuzzer's item mutations and
      the differential on both sides; `selfhost.rs` in a CI job of its
      own, N on a release runtime; ADR 0055 and the notes;
    - that nothing a user runs changes: every `.nova` file the base
      tracks parses and checks as before, every package tests and builds
      as before, every `--help` is unchanged, and every `nova check`
      command line but the hidden forms' gives what it gave;
    - the departures, marked **(differs)** here;
    - the figures: the Test job's suite `2830 1 10` and `selfhost.rs`'s 52
      apart, the golden suite's `147 passed, 9 excluded`, `selfhost.rs`'s
      times here with and without the release runtime and CI's from runs
      42201484370 and 42774093193, N's heap and stack, and that
      `Cargo.lock` is unchanged;
    - the user's decisions, and this plan's decisions, numbered as here,
      with decisions 10, 12, 15, 16, 17, 18 and 19 marked as differing from the
      spec;
    - the mutants, those that pass by design and the seventeen gaps step 0
      closed,
      and the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems; on the
    Test job's `ubuntu-24.04` leg, `cargo test`'s log holds `test
    release_golden_suite ... ok` and no `tests/selfhost.rs`; on each
    system the Selfhost job passes `selfhost.rs` and then
    `the_selfhost_tests_pass` on the debug runtime; the counts are
    decision 23's: `selfhost.rs` 52 tests on Linux, 50 on macOS, 46 on
    Windows, and the Test job's sums `2831 passed, 10 ignored` on Linux
    and `2808 passed, 9 ignored` on Windows, each one more than run
    42774093193's; record each system's time for `selfhost.rs`, the job's
    and the Test job's, and the run's address, in the ledger, and compare
    each with run 42774093193's (decision 14) in the PR's body.
  - **If the differential fails on CI and not here,** record its report,
    stop and ask the user: a Windows path or line ending is the likeliest
    (Review Focus 1). **If `selfhost.rs` takes more than 70 s on
    Windows,** or more than 60 s on Linux or macOS, ask the user
    (decision 23). Never lower a count.
- [ ] **Step 12: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 11's.
