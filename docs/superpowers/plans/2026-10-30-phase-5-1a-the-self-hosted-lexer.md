# Phase 5.1a, "The self-hosted lexer", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** The first of Phase 5.1's two parts: N's lexer. A Nova package,
`selfhost/`, whose `selfhost tokens FILE…` lexes each file exactly as
`nova-lexer` does, quirks included, and prints a token dump; S0 prints the
same dump behind a hidden flag, `nova parse --dump-tokens FILE…`; and a
differential test, `crates/nova-cli/tests/selfhost.rs`, builds N with
`nova build` and compares the two dumps byte for byte on every `.nova` file
of the repository, on the same corpus with a Windows checkout's line
endings, on fixed cases and edge inputs, and on a fuzzer's soup and mutated
corpus files from fixed seeds, on Linux, macOS and Windows. No language
change: every `.nova` file the base tracks parses and checks as before, and
`nova parse --help` is byte for byte the base's. ADR 0051 records the
departures.

**Architecture:**
- **S0:** `crates/nova-lexer/src/dump.rs` (`dump_tokens`, exported from
  `lib.rs`), next to the lexer it describes; `crates/nova-cli/src/cmd/parse.rs`
  (the hidden `--dump-tokens`, the hidden list `more` of further files, the
  `== FILE` headers, `IOERR` and the exit code 2).
- **N:** `selfhost/nova.toml` and `selfhost/src/`: `token.nova` (the 92
  kinds, `Span`, `Spanned`, `description()`), `lexer.nova` (`lex`,
  `lex_with_comments`, `Tokens` and its marks, the scanner with logos's
  rules written out, the tuple-index split, comments, `LexError`),
  `dump.nova`, `main.nova` (the command line) and `lexer_tests.nova` (N's
  own `@test`s).
- **The differential:** `crates/nova-cli/tests/selfhost.rs`, with
  `tests/selfhost/compare.rs` (the comparison and its report),
  `tests/selfhost/fuzz.rs` (splitmix64, the soup, the mutations, the edge
  inputs) and `tests/selfhost/cases/` (twelve fixed cases); `nova-fmt`'s
  corpus gate skips the cases; CI's `nova fmt --check` covers `selfhost/`.
- **Records:** ADR 0051; `CHANGELOG.md`; dated notes in
  `docs/phase-5-plan.md`, `nova-spec/00-MASTER-SPEC.md` and
  `nova-spec/11-PARSER.md`.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §6.1).
- N is built by `nova build` through Cranelift: no `clang` for the
  differential. `clang-18` (Ubuntu's 18.1.3, CI's pin) for the golden
  suite, and Node 22 for the WASM tests, as before.

**Spec:**
`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`,
approved by the user on 2026-10-30, the branch's first two commits.
- Read its header, §3 (5.1a), §5 to §9 before Task 1: it is the authority
  this plan argues from.
- Its header holds the user's decisions (decisions 1 to 3 here, the 5.1a
  ones) and its "not asked" recommendations (decision 4 is 5.1a's).

**Checked on 2026-10-30, before execution:**
- **A prototype built 5.1a whole** on a local branch on the spec commits,
  in this tree: `8ec57240` (S0's dump and the flag), `ea4e4036` (N's lexer
  and its tests), `53c315ad` (the harness and the fuzzer), `78813665` (the
  harness compares a named pair of dumps, for 5.1b), `ff98d0d7` (ADR 0051
  and the records) and `be3e4329` (`selfhost/target` ignored); then
  `f8da9f7b`, step 0, `ab446e43`, the first fact-check's fixes, and
  `e8d10c4f`, the second's (all three below). No difference between the two lexers was found while building
  it: the corpus agreed at once, and so did 40,000 inputs of a local
  search.
- **Step 0, before the split,** `f8da9f7b 5.1a prototype: before the
  split` (decisions 20 to 26):
  - Windows' line endings: the corpus runs a second time with each `\n`
    written `\r\n`, on every system; the soup gains `\r` inside each kind
    of token and tails that end a comment in `\r`;
  - the edge inputs, which no file under `cases/` can hold, and eleven
    fixed cases from the probes, every probe agreeing;
  - a side that fails is reported with both exit codes and its stderr, and
    the file it never reached is named;
  - S0's dump pinned on its own: `nova-lexer`'s dump of every field, the
    flag's conflicts, the headers and `IOERR`;
  - the patches cleaned of backslash-u escapes; ADR 0051 and the CHANGELOG
    record each departure.
- **The first fact-check,** read-only, of the prototype at `f8da9f7b` and
  this plan's draft (decisions 33 and 35 to 41): about 50,000 inputs of
  its own, and no fault in the code, the two lexers agreeing on each. It
  found gaps in the tests, records that said more than was true, and
  nits, all fixed in `ab446e43 5.1a prototype: the first fact-check's
  fixes`:
  - `the_selfhost_tests_pass` passed when no test ran: it now requires as
    many passed as N's sources hold `@test`s, eleven with the new one,
    `descriptions_are_the_rust_ones`, which checks `token.nova`'s
    descriptions, run by nothing before;
  - no process had a time limit: each has two minutes now, read on
    threads and killed past it, the test naming its batch;
  - a seed's mutations differed by system, the corpus being `\r\n` on
    Windows: they read `\r\n` as `\n` now, picked from a sorted list,
    and an untracked local file changes them, as the records now say;
  - the edge inputs lacked U+2001 to U+2009, so "each White_Space
    character" was not true; S0's own dump test did not pin the hex's
    case; the outputs were compared lossily; a failed build of N ran
    again for each test;
  - the second file's usage line always said `nova`: it names the binary
    as run now, byte for byte the base's through a renamed link;
  - two differences went unrecorded, a file name that is not UTF-8 and
    `selfhost tokens` with no file: N now refuses no file as S0 does, and
    ADR 0051 records the other;
  - the Runs block gave every user root's cgroup check.

  Each new or strengthened test was run with its fix taken out, and
  failed; the old test, where one was strengthened, passed there.
- **The second fact-check,** read-only, of the prototype at `ab446e43`
  and this plan (decisions 42 to 48), found a record error, the old S0
  bug behind it, a plan error and nits, all fixed or recorded in
  `e8d10c4f 5.1a prototype: the second fact-check's fixes`:
  - ADR 0051 and decision 20 said both lexers were loops, so Windows'
    1 MB stack could not limit them; but `logos`'s code for a decimal
    number recurses once per character, and a decimal literal of about
    4,000 characters aborts debug S0 under 1 MB (16,000 pass and 32,000
    abort under 8 MB), where N lexes it. Both texts are corrected, and
    the ADR records it as a known S0 bug for 5.1b's big-stack thread; no
    generator makes such a number;
  - Task 5 gave figures with no command that produced them: the three
    scripts are in its Steps 1 and 4 now, verbatim;
  - with no file name in UTF-8 in its first argument, the second file's
    error said `nova parse` where the base says `parse`: it says `parse`
    now, and, built as clap builds it, shows the base's colours in a
    terminal too;
  - the two lexers' speed with the debug runtime, quadratic on a run of
    overflowing digits and N about 10 to 15 times slower than S0
    otherwise, is in the ADR;
  - nothing failed if `selfhost()` or `run()` stopped using their tested
    helpers: `selfhost()` is pinned now, and four bypasses that no cheap
    test can catch are mutants that pass, accepted (decision 46);
  - `output_within` reads its pipes within the limit after an exit, and
    kills a child it cannot wait for; a line that differs only in its
    final line break is marked in a report.

  Each new or strengthened test was run with its fix taken out, and
  failed: four runs (decisions 44, 46 to 48).
- **The mutants, first at the prototype's head** (decision 32): all 56
  caught, each mutant of N's lexer by at least one of the differential's
  inputs, not only by N's own tests; none left a gap to close, so no
  commit followed step 0. One is caught only by step 0's work: a Unicode
  escape's value let wrap (mutant 59) passed at
  `be3e4329`, and fails on step 0's seventeen-digit edge input.
- **A replay split the prototype into this plan's tasks,** on the local
  branch `split/5-1a`, by a script, and ran each task's steps as written:
  - the two spec commits, cherry-picked unchanged (the same `f3bd8fe0` and
    `3bef7004`), then seven commits: a tests commit and a code commit for
    each of Tasks 1 to 3, then Task 4, each the patch of its task below.
    The last commit's tree is the prototype head's
    (`git diff --stat proto/5-1a-whole split/5-1a` prints nothing). Each
    patch applies with plain `git apply`, in order, on `3bef7004`, with no
    warning;
  - each tests commit failed as its step says before its code, Task 1's
    and Task 3's by not compiling (decision 29), and passed after it; the
    whole suite, but the IPv6 test, `cargo fmt`, clippy with
    `-D warnings` and `nova fmt --check` passed after every task:
    `2799 1 10` after Task 1, `2800 1 10` after Task 2 and
    `2816 1 10` after Task 3; Task 4 changes no code;
  - every "Expected:" below is the replay's, from a fresh `NOVA_HOME`, but
    Task 0's suite figures, which are 5.0c's last full run on `25be9f3c`.
- **The final checks ran on the split's last commit:**
  - the full suite gave `2816 1 10` (6 min 20 s): the base's `2795 1 10`
    and 21 tests more, which a comparison by binary finds as
    21 added and 0 gone (Task 5, Step 1); the one failure is the IPv6 test;
  - CI's Linux leg, the whole suite under
    `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 … --all-features`:
    `2816 1 10`, the golden suite among them (Task 5, Step 2);
  - the golden suite, in 42 s of wall time:
    `release golden: 147 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)`;
  - `nova parse --check` and `nova parse --ast` over every `.nova` file
    `25be9f3c` tracks, by the base's `nova` and the tree's:
    `parse: 590 runs over 295 files, 0 differ; --check exits nonzero on 2 at the base, 2 now`;
    `nova check` over the same files (Task 5, Step 4):
    `check: 261 checks, 0 outside std differ, 0 std files differ`;
  - `nova parse --help` byte for byte the base's, and the second file's
    error too, under five first arguments, four commands each, and twice
    in a terminal (Task 5, Step 5);
  - Rust 1.78's check was clean, in 51.28 s, and `Cargo.lock`
    did not change (Task 5, Step 6);
  - `selfhost.rs` in three runs: 11.7 s, 12.6 s and 12.4 s (Task 5, Step 7).
- **The mutants.** 77 breakages of what 5.1a adds, each a text
  replaced in one file, run one at a time at the split's last commit,
  under the process limit (Task 5, Step 8): S0's dump, each field of it
  (17); the command, its headers, `IOERR`, exit code and flag (8); the
  harness, its time limit and its pipes' deadline, its kept build, the
  mutations' sameness, the line-break mark, the fuzzer's assertion and
  four callers bypassing their helpers (18); and N, each quirk, the float split, the marks,
  the doc comment's trim, the escapes, holes, keywords, punctuation, `\r`,
  its descriptions, its dump and command line and its tests' count
  (34). The first run, at `f8da9f7b`, caught all 56 in 16 min 49 s
  (decision 32); the first fact-check's fixes added twelve, the second's
  nine. This run, on the split of `e8d10c4f`, caught 73 of
  77, in 21 min 22 s; the four bypasses of decision 46
  passed, as accepted, no other passed, and none was left unrun.
- **What the checks left standing,** each recorded or handed on:
  - macOS and Windows are reasoned from the code, not run here; Linux runs
    the Windows checkout's line endings (decision 20);
  - `IOERR` names std/io's eight kinds; a failure of another kind reads
    `Other` on both sides (decision 18);
  - an empty path is refused by S0's `clap` and read as `IOERR NotFound`
    by N; the harness never passes one (decision 26);
  - a file name that is not UTF-8 is dumped by S0 and `IOERR NotFound` to
    N; the harness's names are UTF-8 (decision 40);
  - an untracked local `.nova` file changes a seed's mutations
    (decision 37);
  - a local `.nova` file that is not UTF-8 joins the corpus and fails it,
    since the corpus is a walk of the tree (decision 14);
  - two runs of the differential at once share its fixed directories
    (decision 31);
  - Rust 1.78 checks the libraries and binaries, not the tests, as CI's
    MSRV job does;
  - each system's time for `selfhost.rs`, and so how far below the two
    minutes a process stays on Windows, is CI's to give (Task 5, Step 11);
  - a decimal literal of about 4,000 characters overflows debug S0's stack
    under 1 MB, Windows' main thread's, and one of 32,000 under Linux's
    8 MB: a known S0 bug, recorded in ADR 0051 for 5.1b's big-stack
    thread (decision 42);
  - with the debug runtime, N is about 10 to 15 times slower than debug
    S0, and both are quadratic on a run of overflowing digits
    (decision 45);
  - four mutants, callers bypassing a tested helper, pass by design, and
    `try_wait`'s failure has no test (decisions 46 and 47).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` under `RUSTUP_TOOLCHAIN=1.78.0`
  with `RUSTFLAGS=-D warnings`. No `std::sync::LazyLock` (1.80), no
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79). 5.1a's Rust
  uses `std::sync::OnceLock` (1.70), `Option::is_some_and` (1.70),
  `let … else` (1.65), `JoinHandle::is_finished` (1.61), arrays' `map`
  (1.55) and slices' `split_inclusive` and `strip_prefix` (1.51), nothing
  newer.
- **No new crate, and `Cargo.lock` does not change** (spec §6.1). The
  fuzzer's PRNG is written in the test; the mutations lex through
  `nova-lexer`, already a dependency of `nova-cli`. Any change to the lock
  stops the task: ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`. 5.1a's Rust outside tests has none; N's `unwrap()`s
  are on values its own code has checked (`char_from_int` of a code point
  it decoded or tested).
- **Tests never touch the internet,** bind no port, and need no `clang`
  (spec §6.1). The differential runs `nova` and the N it builds, nothing
  else: `nova lsp` stands in for a process that never ends, and a hard
  link to `nova` (a copy where the temporary directory is on another
  file system) for a renamed one, and on Unix `Command::arg0` for a first
  argument with no file name.
- **Temporary names are fixed** (spec §6.1): under
  `std::env::temp_dir()`, `nova-selfhost-<name>`, one per use: `build`
  (N's executable), `home` and `test-home` (fresh `NOVA_HOME`s for
  `nova build` and `nova test`), `renamed`, `headers`, `crlf`,
  `crlf-mutations`, `edges`, `soup-1` to `soup-3`, `mutated-101` to
  `mutated-103`, `search-soup`, `search-mutated`, `unreadable`,
  `mismatch` and `broken`. Each is removed
  and made afresh by the test that uses it. A test that needs `$NOVA_HOME`
  sets a fresh one.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and the golden suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the suite here with `NOVA_REQUIRE_NODE=1`: Node 22 is installed. No test
  of 5.1a needs Node.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and
  passes. 5.1a adds no runtime program, so it stays at 147.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary, which only a build
  of the workspace writes. `nova build` of `selfhost/` links against that
  runtime. Every run step starts with `cargo build --locked --workspace`
  (Conventions).
- **The process limit.** 5.1a starts no process that runs itself, but its
  differential starts two processes a batch, and the suite thousands; every
  test, mutant and suite runs under the limit of the Conventions, as
  5.0c's did, so that a mistake that forks without end fails to fork
  rather than filling the machine.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other. Never build in another worktree.
- **Keep `df -h /` above 6G.** Older duplicate test executables in
  `target/debug/deps` may be removed between commands, never while one
  runs. Every `cargo` command here selects the whole workspace
  (Conventions), so cargo keeps one copy of each test executable. Task
  5's Rust 1.78 check uses its own target directory, in `/dev/shm`,
  removed after.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.1a's
  patches holds one: N's tests spell a Unicode escape `u("{41}")`, a
  helper that joins a backslash and `u`, and the fuzzer builds its escapes
  from `BS`. Apply the patches with `patch_block`, from the committed plan.
- **No patch adds a CR, a NUL or trailing whitespace.** The inputs that
  need them, a lone `\r`, `\r\n`, NUL, are written by the tests at run
  time (`fuzz::edges`, the Windows-endings corpus), not kept as files.
- **Nothing writes into the repository but the patches.** `nova build -o`
  writes N under the system's temporary directory and `nova test` writes
  nothing into the package; `git status --short` is empty after every run.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.1a.
- **This machine is Linux.** macOS and Windows results come from CI; what
  differs there is reasoned from the code (decision 20).

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-51a.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-51a-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-51a-<task>.txt"`. A task with a tests
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
  mutants ran under it on 4 CPUs, and peaked at 64 tasks. Test threads
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
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-51a"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG
  mkdir -p "$NOVA_HOME"
  SH() { cargo test --locked --workspace --test selfhost -- "$@"; }
  S() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test run_tests -- release_golden_suite --nocapture "$@"; }
  ```

  So `SH` runs `selfhost.rs`, the differential and the flag's tests, `S`
  the whole suite, and `G` the golden suite through LLVM as CI's Linux leg
  runs it. `NOVA_HOME` keeps a run by hand off `~/.nova`; every test that
  needs one sets its own.
- **Every `cargo` command selects the whole workspace,** `--workspace`
  with `--test NAME` or `--lib` to narrow it, never `-p`. Cargo unifies
  features over the packages a command selects, so each `-p` selection
  builds and links its own copy of every test executable it needs, which
  the disk does not have room for; a `--workspace` command reuses one
  copy. `S` gives each code step the whole suite, about seven
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
  PLAN=docs/superpowers/plans/2026-10-30-phase-5-1a-the-self-hosted-lexer.md   # this file, committed
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
  nothing: no 5.1a patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. The non-ASCII
  characters the patches add are `§`, `…`, `–` and `—` in prose, and `é`, `€`
  and `😀` in the tests' sources. A block that does not apply
  means the tree differs from the plan's: stop, and record why in the
  ledger.
- **Each task** but Task 4 is test first: its tests' patch, their run and
  the expected failure, then its code's patch, its run and the expected
  pass. Task 4's patch is the records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy,
    `cargo clippy --locked --all-targets --all-features -- -D warnings`, as
    CI runs it, which ends with `Finished` and no warning;
  - `target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr`,
    with `selfhost` added from Task 2's code on, with the `nova` the step's
    build made, which prints nothing.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost
  if wrong.

## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
check in the task that owns the code:

1. **N's lexer against S0's** (`selfhost/src/lexer.nova` against
   `crates/nova-lexer/src/{lib,tokens,error}.rs`): logos's rules written by
   hand, the longest match, a keyword only as a whole match, and a refused
   match as `UnexpectedCharacter` of its first character, one character
   on; an integer past `Int`, in any radix, refused that way, and `r#`
   with no `"` the one `InvalidNumberLiteral`; `'\q'` three errors; the
   char literal's classes, any character but `'` and `\`, and after `\`
   any but `\n`; a doc comment's end trimmed by Unicode's White_Space; a
   Unicode escape's hex as `u32::from_str_radix` reads it, a `+` taken,
   past `u32` refused before an `Int` could wrap; the float split after a
   `.`, by the text; the `NL` mark through comments, `\r` alone included;
   `PD` by the text (decisions 5, 7, 9 and 21) → the differential on the
   corpus, the Windows-endings corpus, the cases, the edge inputs, the soup
   and the mutations, and N's own `@test`s (Tasks 2 and 3) (mutants
   44 to 77). A difference no
   generator makes is the risk the spec names first (spec §9 risk 1): the reviewer reads
   `raw_token`, `lex_number`, `lex_char`, `hex_u32`, `trim_end` and
   `marked` against the Rust.
2. **Windows and macOS** (`selfhost.rs`'s corpus, `run`, `selfhost`; N's
   `main.nova`): `\r` in every position, as a Windows checkout gives it;
   paths passed relative, joined by `/`, printed as given by both sides,
   canonicalized by neither; `selfhost.exe` named by `EXE_SUFFIX` and
   written by `nova build -o` as given; a file that is not UTF-8 `IOERR
   InvalidData` on both; the mutations read with `\r\n` as `\n`, so a
   seed's are the same from a Windows checkout; a slow runner within the
   two minutes each process has; N's lexer without recursion, so Windows'
   1 MB stack does not bind it, and S0's decimal numbers recursing once
   per character, so that one of about 4,000 characters overflows debug
   S0 there, a known bug for 5.1b's big-stack thread (decisions 20, 37
   and 42) →
   `the_corpus_dumps_agree_with_windows_line_endings`,
   `the_mutations_are_the_same_with_windows_line_endings`, the edge inputs,
   `unreadable_files_are_reported_alike` (Task 3) (mutants
   65, 66, 49,
   19 and 32). CI's three systems are the
   only run of the other two; the reviewer reads every place a path, a line ending or an
   exit code crosses a process.
3. **The dump, on both sides** (`dump.rs`, `dump.nova`): the spans, the
   kinds' names, `Int` in decimal, `Float` as 16 lowercase hex digits of
   its bits, `Char` as its code point; text quoted, each byte below 0x20,
   0x7F, `"` and `\` as `\xHH`; a message's control bytes escaped and its
   `"` and `\` kept; `NL` then `PD`; `Eof` last, then errors, then comments;
   the header, `IOERR` and the exit code 2 after the rest; no file refused
   by both (decisions 7, 16, 18 and 26) → `nova-lexer`'s
   `the_token_dump_writes_every_field`, whose `0.1` pins the hex's case on
   S0's side alone, the headers' test (Task 1), the differential and
   `both_sides_refuse_no_file` (Task 3) (mutants 1 to
   20, 70 to 74).
4. **The harness agreeing falsely, or hanging** (`compare.rs`,
   `compare_batch`, `agree_on`, `output_within`, `once`): two outputs
   compared whole, as bytes, with their exit codes; a side that prints
   nothing, crashes or stops early reported with its exit code and stderr,
   and the file it never reached named; every batch compared, not only the
   first; the corpus walk finding every file, N's own included; as many of
   N's `@test`s passing as its sources hold, so a run of none fails; each
   process killed at two minutes, its batch named, and its pipes read
   within the limit after it exits, so a process it started cannot hold
   the harness; a line that differs only in its final line break marked;
   a failed build of N made once and reported to every test, and
   `selfhost()`, `run()` and N's build each through the helper that has
   the test; for a seed, the soup the same on
   every system, and the mutations the same for the same corpus files,
   picked from their sorted list and read with `\r\n` as `\n`, though an
   untracked local `.nova` file changes them (decisions 8, 9, 13, 14, 22,
   35 to 38 and 46 to 48) → `the_mismatch_report_names_the_file_and_line`,
   `a_missing_final_line_break_is_marked`,
   `a_pipe_still_open_at_the_deadline_is_reported`,
   `a_side_that_fails_is_reported_with_its_exit_code_and_stderr`,
   `a_process_past_its_limit_is_killed_and_its_batch_named`,
   `a_failed_build_is_made_once_and_reported_to_each_test`,
   `the_mutations_are_the_same_with_windows_line_endings`,
   `the_selfhost_tests_pass` and the soup's assertion (Tasks 2 and 3)
   (mutants 26 to 43, and
   77; 39 to
   42 pass by design, decision 46, and the reviewer
   reads those four calls).
5. **The hidden flag** (`parse.rs`): absent from `--help`, whose text is
   the base's byte for byte; conflicting with `--ast` and `--check`; a
   second file without the flag refused in clap's words and colours as
   before, the usage line naming the binary by the name it was run by, or
   `parse` alone when that has no file name in UTF-8; the further files a
   hidden list only the flag reads (decisions 7, 17, 39 and 44) →
   `the_token_dump_flag_is_hidden` (Task 1), Task 5's Step 5
   (mutants 21 to 25).
6. **The records:** ADR 0051 against the spec's §3 and §8 and this plan's
   decisions; the CHANGELOG's Added and Changed; the dated notes of
   `docs/phase-5-plan.md`, `00-MASTER-SPEC.md` and `11-PARSER.md`, each
   naming the branch `claude/loving-archimedes-frtf5q-51a` → Task 4,
   Step 2.

## Decisions

Made while writing the spec (its header and §3), prototyping, probing,
splitting and replaying, each as built and each read against the code.
Where one departs from the spec, it says so and why, and is marked
**(differs)**; one that fills in what the spec left to the plan says so,
unmarked. ADR 0051 records 5.1a's departures and the points it settles
(spec §8).

**The user's decisions, as built**

1. **5.1 is two pull requests, and 5.1a is the first** (the user's
   decision 1): the package `selfhost/`, the lexer, a token dump on both
   sides, the fuzzer and the differential harness. The AST, the parser,
   the AST dump, the comparison of parse errors, the depth limit and the
   speed are 5.1b's; so are the user's decisions 2, 3, 4 and 6, which 5.1a
   does not touch. `docs/phase-5-plan.md`'s dated note says so.
2. **The fuzzer is Rust, in `cargo test`, with fixed seeds** (the user's
   decision 5): its own splitmix64 PRNG and no new crate; random soup and
   corpus files mutated token by token; the fixed seeds on every system;
   longer runs locally through `NOVA_SELFHOST_FUZZ` (decisions 9 to 11).
3. **The differential runs in `cargo test` on all three systems** (the
   user's decision 7): `selfhost.rs` builds N with `nova build`, Cranelift,
   no `clang`, on Linux, macOS and Windows (decision 20).

**The spec's recommendation, not asked, taken**

4. **Floats are dumped by their bits**, the 16 lowercase hex digits of
   `f64::to_bits` on S0's side and of `Float.to_bits` on N's, so no
   decimal rendering has to agree (`the_token_dump_writes_every_field`;
   mutant 4, 71).

**The spec's design (§3), as built**

5. **The package** (spec §3.1): `selfhost` 0.1.0, edition 2026, no
   dependencies; `token.nova` (the 92 kinds as `Token`'s variants, `Span`
   and `Spanned` in byte offsets with no `FileId`, and `description()`
   word for word), `lexer.nova`, `dump.nova` and `main.nova`. The lexer is
   `lib.rs`, `tokens.rs` and `error.rs` function for function, each
   function naming the Rust it ports. logos becomes a hand-written
   scanner with its rules: the longest match; a keyword only when the
   whole identifier is one; where no regex matches or a callback refuses,
   `UnexpectedCharacter` of the first character, then one character on.
   Its classes are ASCII but two, the char literal's `[^'\\]` and the `.`
   after its `\`, which take any character but `'` and `\`, and any but
   `\n`; and the doc comment's `trim_end`, Unicode's White_Space. Keywords
   are found by first byte and length before any comparison, since a
   string literal allocates each time it is evaluated. N is 989 code lines
   for what the Rust's 1,002 do, and 176 for the dump. No dump prints a
   description: N's `@test` `descriptions_are_the_rust_ones` checks
   thirteen kinds' against `token.rs`'s, a keyword, punctuation, the `${`
   of a hole and the literals among them, and 5.1b's parser messages,
   which hold them, exercise every one through the differential
   (decision 41; mutants 75 and 76).
6. **The command line** (spec §3.2): `selfhost tokens FILE…`; each file
   after a header line `== FILE`, the path as given; lex errors are part of
   the dump and leave the exit code 0; a file that cannot be read gives
   `== FILE` then `IOERR KIND`, and the files after it are dumped before
   the exit code 2; no file at all is refused with a usage line and the
   exit code 2, as S0's clap refuses it (decision 40). `main` is
   `async fn`, since `std/fs` reads only so
   (`unreadable_files_are_reported_alike`, `both_sides_refuse_no_file`;
   mutants 72, 73 and 74).
7. **The dump, and S0's hidden flag** (spec §3.3):
   `nova_lexer::dump_tokens(source)` writes one file's dump next to the
   lexer it describes, and `nova parse --dump-tokens FILE…` adds the
   headers and `IOERR`, hidden from `--help` (`hide = true`), with no
   stability promise, conflicting with `--ast` and `--check`. Per file:
   a line per token, `Eof` last, `start end Kind[ payload][ NL][ PD]`;
   then `ERR start end message` per lex error, in the order found; then
   `COMMENT start end Line|Block` per comment. `Int` in decimal, `Float`
   by its bits (decision 4), `Char` as its code point, the text of
   `StrPart`, `RawStr`, `Ident` and `DocComment` in double quotes with each
   byte below 0x20, 0x7F, `"` and `\` written `\xHH`. The tokens and the
   marks are `lex_with_comments`', which are `lex`'s by construction
   (`the_token_dump_writes_every_field`,
   `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`,
   `the_token_dump_flag_is_hidden`; mutants 1 to
   24).
8. **The harness** (spec §3.4): N built once per test binary through a
   `OnceLock`, which holds the build's result, by `nova build -o` into
   `nova-selfhost-build`; the corpus, every `.nova` file outside
   `selfhost/` and the cases, sorted, then N's own sources, 300 files; 50
   files to a process, 100 for the fuzzer's inputs, each process within
   two minutes; both outputs identical byte for byte, compared as bytes,
   and both exit codes the one expected. On a difference the report names
   the file, the line of its dump and of its source, both sides' lines,
   and the commands that reproduce it
   (`the_mismatch_report_names_the_file_and_line`; mutants
   26 to 30; decisions 36 and 38).
9. **The fuzzer** (spec §3.5): soup of up to 200 atoms, the spike's and
   5.1a's, once in five ending in an unterminated tail, weighted so that
   all seven lex errors occur among the fixed seeds' inputs, which the test
   asserts (mutant 31); corpus files lexed by S0 into
   their tokens' spans, then one to eight edits that delete, duplicate,
   swap or insert whole tokens, then up to three ASCII bytes replaced, so
   the text stays UTF-8; the files picked from their sorted list and read
   with `\r\n` as `\n` (decision 37). Deep nesting is 5.1b's and plugs
   in beside them:
   any inputs go through `agree_on_inputs`, any pair of dumps through a
   `Dump` (`TOKENS` now).
10. **The seeds and counts,** measured (spec §3.4): soup seeds 1, 2 and 3,
    2,000 inputs each; mutation seeds 101, 102 and 103, 250 each. The whole
    of `selfhost.rs`, building N included, took 11.7 s, 12.6 s and 12.4 s
    here, on 4 CPUs (Task 5, Step 7); CI's slowest system is Windows, whose
    time the PR's run records.
11. **`NOVA_SELFHOST_FUZZ=SEED:COUNT`** runs COUNT inputs of soup and COUNT
    mutated corpus files from SEED, the mutations 500 to a directory; unset,
    as in CI, the test returns at once. The prototype's local search,
    `1000:20000`, found no difference.
12. **`nova fmt --check` covers `selfhost/`** in CI's step with `std` and
    `examples` (spec §3.1), from Task 3's code; the conventions' check
    names it from Task 2's code, when the package exists.

**Where the prototype departs from the spec, or settles what it left open**

13. **N's own `@test`s are in `selfhost/src/lexer_tests.nova`,** imported
    by `main.nova`. **(differs)** The spec puts them in `selfhost/tests/`;
    a module there sees only the files beside it and the package's
    `src/lib.nova`, and Nova has no re-exports, so a test there cannot
    reach `lexer.nova`. So `nova build` compiles the tests into `selfhost`
    too, unused; `nova test` runs them, and `the_selfhost_tests_pass`
    requires as many to pass as the package's sources hold `@test` lines,
    eleven, since a module `main.nova` does not import runs none
    (decision 35).
14. **The corpus is a walk of the tree, untracked files included.**
    **(differs)** The spec says every tracked file, "listed from the source
    tree, not from `git`"; a walk cannot tell tracked from untracked, so a
    local `.nova` file joins it, and one that is not UTF-8 fails it, both
    sides giving `IOERR InvalidData` where exit code 0 is expected. The
    walk skips `target/`, `node_modules/`, directories whose names begin
    with `.`, symbolic links, `selfhost/` and the cases. A local file is
    also one more for the mutations to pick, so it changes a seed's
    mutated inputs (decision 37). CI's checkout has no untracked file.
15. **`nova-fmt`'s corpus gate skips `crates/nova-cli/tests/selfhost/cases/`.**
    **(differs)** The spec does not name it. The gate formats every
    `.nova` file and expects the two known failures alone; many cases are
    not Nova on purpose.
16. **An error's message escapes only control bytes and 0x7F.**
    **(differs)** The spec says the message is the error's `Display` text;
    a message can hold the character it names, a line break among them
    (`unexpected character` of a NUL, `invalid escape sequence` of a
    `\n`), which would break the dump's lines, so those are `\xHH`; `"`
    and `\` stay as they are, since the message is not quoted. No message
    can hold a `"` (mutant 13).
17. **`ParseCmd` gains a hidden list, `more`, of the files after the
    first.** **(differs)** The spec names the flag alone. Only the flag
    reads the list; without it, a second file is refused with clap's own
    error, `unexpected argument 'b.nova' found`, as before the list
    existed, built from clap's command so it never names the hidden flag,
    its usage line naming the binary by the file name it was run by, as
    clap does (decision 39); `--help` is byte for byte the base's (Task 5,
    Step 5; mutants 23 and 24).
18. **`IOERR`'s kinds are std/io's eight names.** **(differs)** The spec
    says "the error kind"; N's reads fail with `IoErrorKind`, whose eight
    are `NotFound`, `PermissionDenied`, `AlreadyExists`, `InvalidData`,
    `Interrupted`, `TimedOut`, `ConnectionRefused` and `Other`, so S0 maps
    Rust's `io::ErrorKind` to those words and every other to `Other`, as
    N's runtime does. A directory is `IOERR Other` on both on Linux
    (mutants 19, 72).
19. **N is built with `nova build -o` in place, and tested in place.**
    `-o` writes the executable where it is told, under the temporary
    directory, so the package gets no `target/`; `nova test` writes
    nothing into the package either; `.gitignore` names `/selfhost/target`
    for a build by hand.

**Step 0's decisions, before the split**

20. **Windows and macOS, reasoned from the code, and pinned on Linux where
    a test can.**
    - **Line endings:** GitHub's Windows runners check out with
      `core.autocrlf`, so the corpus there holds `\r\n`. Both lexers treat
      `\r` alike everywhere: whitespace between tokens; a line break for
      `NL`; text in strings and raw strings; dropped by a doc comment's
      `trim_end`; left out of a line comment's span when it is the last
      byte before the `\n` or the end, one `\r` only. Step 0 added
      `the_corpus_dumps_agree_with_windows_line_endings`, the corpus again
      with each `\n` written `\r\n` (a file that holds a `\r` already left
      as it is, as git leaves it), on every system; soup atoms with `\r`
      and `\r\n` inside strings, raw strings, chars and block comments and
      ending doc and line comments; two tails ending a comment in `\r` at
      the end of the file; and the edge inputs (decision 21). Mutants
      65 and 66 are caught by the
      Windows-endings corpus, and 49 by the lone `\r`s.
    - **Paths:** the harness passes each file relative to the directory
      it runs both sides in, its parts joined by `/`, and both print it as
      given, S0 through `Path::display` and N its argument; neither
      canonicalizes, so no `\\?\` prefix can appear on one side only.
    - **The executable:** `selfhost` plus `EXE_SUFFIX`, `selfhost.exe` on
      Windows; `nova build -o` writes the path it is given and adds no
      suffix.
    - **Reads:** a file that is not UTF-8 is `IOERR InvalidData` on both
      (Rust's `read_to_string` and std/fs's, the same check); both keep a
      byte order mark, which both lexers report as an unexpected
      character; an empty file is `0 0 Eof`.
    - **Output:** both write the dump's bytes to standard output with no
      translation of `\n`, S0 through a locked `Stdout`, N through the
      runtime's `print`, which writes and flushes; N's `exit(2)` comes after
      the last `print`.
    - **The stack:** N's lexer is loops, not recursion, so Windows' 1 MB
      main-thread stack does not bound it. S0's is not, as the tests build
      it: `logos`'s generated code for a decimal number recurses once per
      character, so a decimal literal of about 4,000 characters aborts
      debug S0 under 1 MB (decision 42, which corrects what this said
      before the second fact-check). No generator makes one; 5.1b's
      big-stack thread will cover it, and the depth limit is 5.1b's.
    - **The fuzzer's inputs:** the soup needs no file, so a seed's is the
      same everywhere; the mutations read corpus files, which a Windows
      checkout writes with `\r\n`, so they read each `\r\n` as `\n` and
      pick from the sorted list (decision 37).
    - **Time:** each process has two minutes, which a slow Windows runner
      should not reach; past it the process is killed and the test fails,
      naming the batch (decision 36). `Child::kill` is `TerminateProcess`
      there, and the pipes' readers are not waited for after a kill
      (decision 47).
21. **The probes, the fixed cases and the edge inputs.** Before the split,
    each edge the brief named was run through both lexers: `\r` alone,
    `\r\n` in strings, raw strings, holes, chars and block comments, doc
    comments ending in `\r`, a `//` comment at the end of the file ending in
    `\r`; an empty file, a byte order mark alone and before code, no final
    line break; a file that is not UTF-8, a directory and a missing file;
    `9223372036854775808`, prefixes overflowing and with `_`, floats with
    exponents and `_`, `1.e5`, `1._0`, `.5`, `5.`, `t.0.1.2`, `t.1e5`, holes
    fifty deep, `${` in raw strings, Unicode escapes at `10FFFF`, `110000`
    and `D800`, an open `/*` holding `"`, `'''`, `'\''`, NUL inside and
    outside tokens, and a doc comment ending in each White_Space
    character, but `\n`, which ends it, and `\r`, among the `\r` edges,
    and in five characters that are not White_Space: U+180E, U+200B,
    U+FEFF, 0x7F and 0x01. All agreed. (Step 0's edges held U+2000 and
    U+200A but not U+2001 to U+2009; the first fact-check found it, and
    they are in now: decision 41, mutant 55.) The
    printable ones are eleven new fixed cases, beside the
    spike's: `integers_at_the_edges`, `floats_and_exponents`,
    `tuple_indexes`, `nested_holes`, `raw_strings_and_holes`,
    `chars_and_quotes`, `punctuation_longest_match`,
    `keywords_and_identifiers`, `dollars_and_braces`,
    `an_open_block_comment_with_a_quote` and `slashes`. The rest are
    `fuzz::edges`, written by `the_edge_inputs_agree`, since a file could
    not keep them: a Windows checkout would rewrite their line endings,
    and the repository's text holds no line ending in `\r`, no NUL and no
    backslash-u. Among the edges, a Unicode escape of seventeen hex digits
    whose value wraps an `Int` to `A` (mutant 59), and every
    escape.
22. **The harness reports a failing side as such.** At the prototype's
    head, a side that printed nothing or crashed showed only a first
    differing line, often blamed on the file before the one it died in.
    Now `compare_batch` gives every report both exit codes and the first
    40 lines of each side's stderr, reports agreeing dumps with differing
    exit codes, and `first_difference` names the file whose header one
    side never printed, at its header
    (`a_side_that_fails_is_reported_with_its_exit_code_and_stderr`, and
    the mismatch test's short side; mutants 27 and
    29). A side's stdout is compared as bytes and shown
    lossily, so output that is not UTF-8 differs where its bytes do and
    cannot panic the report (decision 38; mutant 30).
23. **S0's dump is pinned on its own,** beside the differential, which
    would pass a format both sides got wrong together: `nova-lexer`'s
    `the_token_dump_writes_every_field`, one source whose dump has every
    field, written out and checked by hand, `0.1`'s bits among them, so
    that the lowercase hex is pinned on S0's side alone (decision 41;
    mutant 5), and
    `the_token_dump_of_nothing_is_eof`; the flag's conflicts with `--ast`
    and `--check`; and S0's headers, `IOERR` for a missing file between two
    others, and the exit code 2. The spec's §6.2 lists the hidden flag's
    test; these are more of it.
24. **A token swapped with itself stays.** The mutation's swap took the
    first text before reading the second, so `i == j` deleted the token,
    a fourth delete; now it swaps, and a self-swap changes nothing.
25. **No patch line holds a backslash-u escape.** N's escapes test spells
    each Unicode escape `u("{41}")`, a helper joining `"\\"` and `"u"`;
    `lexer.nova`'s comment says "A Unicode escape"; the fuzzer builds every
    escape from `BS`, and its byte order mark from `char::from_u32`.
26. **An empty path differs, and is never passed.** S0's `clap` refuses an
    empty `FILE`, "a value is required"; N reads it and prints `IOERR
    NotFound`. The harness's paths are never empty. Decision 40 records
    two more such edges, a file name that is not UTF-8 and no file at
    all.

**The split's decisions**

27. **Four tasks, each tests then code, then the records:** S0's dump and
    the hidden flag; N's lexer and its own tests; the differential, its
    comparator and its fuzzer; ADR 0051. S0's dump stands alone and is
    tested alone (decision 23). N needs nothing of the harness, and its own
    `@test`s, each an S0 dump written out, test it before the differential
    exists. The differential's tests need both, and its comparator and
    generators are the code they call, so they are Task 3's code, with CI's
    `nova fmt` step. The brief's natural split, kept.
28. **`selfhost.rs` is cut by task.** Task 1's tests make it, with S0's
    tests and its doc's first bullet; Task 2 adds `root()`, the limit,
    `output_within`, N's bullet, `selfhost_test_count` and
    `the_selfhost_tests_pass`; Task 3 the modules, constants and the
    differential. Before Task 3, std's `use`s are the ones each stage's
    code needs (Task 1 `PathBuf`, `Command` and `EXE_SUFFIX`; Task 2 all
    but `OnceLock`), each stage's last bullet ends with a full stop, and
    its last paragraph says only that tests write to fixed directories.
29. **Tests that cannot stand first, and tests that pass first.**
    `nova-lexer`'s dump tests name `dump_tokens`, so Task 1's tests do not
    compile before its code; `selfhost.rs` names `compare` and `fuzz`, so
    Task 3's do not either. In Task 1, `the_token_dump_flag_is_hidden`'s
    first half, `--help` without the flag and clap's error for a second
    file, the renamed binary's usage line and, on Unix, the four first
    arguments with no file name among them, passes at the base,
    as it must: those are the base's behaviour kept; its conflicts fail.
    Task 2's `the_selfhost_tests_pass` fails before the package exists. Every other new test runs first at its code
    step.
30. **Every commit builds correct code and passes the suite.** Each code
    step ran the whole suite, `cargo fmt`, clippy and `nova fmt --check`
    (`2799 1 10` after Task 1, `2800 1 10` after Task 2 and
    `2816 1 10` after Task 3; Task 4 changes no code). A tests commit is not a commit of the plan: each
    task commits once, after both patches.
31. **Fixed directories, and two runs at once.** The temporary names are
    fixed (spec §6.1), so two runs of `selfhost.rs` at once, two
    `cargo test`s in one machine, clobber each other's inputs and N; one
    run at a time, as CI and this plan run it, cannot. Within one run, each
    test has a directory of its own, and N is built once, behind the
    `OnceLock`, before any test reads it.

**The mutants**

32. **The mutants' gaps: none,** and four that pass by design
    (decision 46). The first run, at the prototype's head
    `f8da9f7b`, caught all 56, and every mutant of N's lexer failed at
    least one of the differential's tests, not N's own `@test`s alone: the
    corpus, its Windows endings, the cases, the edge inputs, the soup or
    the mutations. One relies on step 0: letting a Unicode escape's value
    wrap (mutant 59) passed at `be3e4329`, whose soup's
    longest escape, nine hex digits, cannot wrap an `Int`; step 0's edge
    input of seventeen catches it, and nothing else does. Three others
    were run against `be3e4329` too, the `\r` mutants
    66 and 65 and a doc comment left
    untrimmed, and its soup already caught them; the Windows-endings
    corpus catches them now as well. The first fact-check's fixes added
    twelve, one for each new check (decision 41); the second's, nine:
    five caught, one or two for each new check, and the four bypasses of
    decision 46, which pass. The whole set, 77, ran again at
    the split's last commit: 73 caught, and the four accepted
    passed.
33. **The process limit** (Conventions): 600 tasks, set by a pids control
    group for root, since Linux never applies `ulimit -u` to root, on
    cgroup v1 or v2, and by a systemd scope's `TasksMax` or a soft
    `ulimit -Su` for any other user. Every shell checks that it is under
    the cap and stops if not, each way by its own check: root's by
    `/proc/self/cgroup` naming the group, a systemd scope's by its
    `pids.max`, `ulimit`'s by `ulimit -Su`; a user who is not root never
    runs the group's lines, which only root may (the first fact-check
    found the Runs block gave every user root's check). The replay ran as
    root on cgroup v1 and 4 CPUs, every step in the group `nova-limit`.
34. **The mutants leave the tree built:** Task 5, Step 8 ends with
    `cargo build --locked --workspace`, so `target/debug` holds the last
    commit's binaries for the steps after it.

**The first fact-check's fixes**

A read-only fact-check of the prototype and this plan, after step 0,
searched with about 50,000 inputs of its own and found no fault in the
code: the two lexers agreed on every one. It found gaps in the tests,
records that said more than was true, and nits. Each is fixed in one
prototype commit, `ab446e43 5.1a prototype: the first fact-check's
fixes`, and in the split's tasks; every new or strengthened test was seen
to fail with its fix taken out.

35. **N's `@test`s are counted** (Task 2). `the_selfhost_tests_pass` asked
    only for exit 0 and ` 0 failed`, which a run of no test gives, as when
    `main.nova` stops importing `lexer_tests`. It now counts the lines that
    are `@test` in `selfhost/src/` and `selfhost/tests/`, eleven, and
    requires ` 11 passed; 0 failed;`, so it stays right as tests are added
    (mutant 77). The eleventh is new:
    `descriptions_are_the_rust_ones` (decision 5).
36. **Every process has two minutes** (Tasks 2 and 3). The harness waited
    on `Command::output`, so an N that loops forever would hold CI for
    hours. `output_within` spawns the process, reads its stdout and stderr
    on threads of their own, so a full pipe cannot stall it, and polls
    `try_wait` against `CHILD_LIMIT`, 120 s, each of `nova build`,
    `nova test` and each side's batch taking a second or two here; past
    it, the process is killed, and the test fails naming the batch, its
    size and its first file (and, after the second fact-check, a process
    that cannot be waited for is killed, and its pipes are read within
    the limit after it exits: decision 47). No crate. The test runs
    `nova lsp`, which
    reads its stdin until it ends, with the stdin held open and a 300 ms
    limit, on a thread, and fails if no answer comes within a minute
    (`a_process_past_its_limit_is_killed_and_its_batch_named`; mutant
    34). On Windows `nova lsp` blocks on its pipe the same
    way, and `Child::kill` is `TerminateProcess`.
37. **A seed's mutations are the same on every system, for the same
    files** (Task 3). The soup reads no file, so "the same inputs on every
    system" held for it alone: the mutations read corpus files, which a
    Windows checkout writes with `\r\n`, and pick by position in the walk's
    list. `mutations_of` now reads each `\r\n` as `\n` and picks from the
    sorted list. An untracked local `.nova` file is still one more to pick,
    so it changes the mutated inputs, as `fuzz.rs`'s module doc says
    (`the_mutations_are_the_same_with_windows_line_endings`, the corpus as
    a Windows checkout writes it, listed backwards, mutated as the corpus
    is; mutants 32 and 33).
38. **The outputs are compared as bytes, and a failed build is kept**
    (Task 3). `first_difference` takes bytes, so two outputs that differ
    only in bytes that are not UTF-8, which both read as the replacement
    character, differ (the mismatch test's last case; mutant
    30). The `OnceLock` holds the build's `Result`, so a
    failed build is made once and every test that needs N reports it,
    where a panic inside `get_or_init` left the lock empty and each test
    built again (`a_failed_build_is_made_once_and_reported_to_each_test`;
    mutant 43).
39. **The second file's usage line names the binary as run** (Task 1).
    The error built from clap's command said `Usage: nova parse …`
    whatever the binary's name; clap names it by the file name of the
    first argument, so a renamed `nova` printed the base's error with
    another name. The command now sets that name, as clap does. Through a
    link named `othernova`, `parse a b`, `parse --ast a b`,
    `parse --check a b` and `parse -- a b` print the base's bytes and exit
    code (Task 5, Step 5); the flag's test runs a hard link named
    `nova-renamed` (mutant 24). A first argument with no
    file name was still named `nova`: decision 44.
40. **No file, and a file name that is not UTF-8** (Tasks 2 and 4). Two
    differences between S0 and N were not recorded. `selfhost tokens` with
    no file printed nothing and exited 0, where S0's clap refuses it with
    the exit code 2: N now refuses it too, with its usage line, which is
    safe since the harness never runs a batch of none, and
    `both_sides_refuse_no_file` checks both (mutant 74). A
    file name that is not UTF-8 is read and dumped by S0, its header
    showing the name lossily, while N, whose arguments are strings, gets
    the lossy name and prints `IOERR NotFound`; that stays, recorded in
    ADR 0051's consequences beside decision 26: the harness's names are
    its own, and UTF-8.
41. **The rest** (Tasks 1 to 4): the edge inputs gain U+2001 to U+2009,
    so a doc comment ends in each White_Space character, and ADR 0051
    names the five that are not (decision 21; mutant
    55, which the old edges let pass); S0's own dump
    test gains `0.1`, `3fb999999999999a`, whose hex has letters (decision
    23; mutant 5, which the old test let pass); N's descriptions
    gain their `@test`, and ADR 0051 says 5.1b's parser messages exercise
    every one (decision 5); decision 22's "not UTF-8" is made true
    (decision 38); and the Runs block gives each way of the process limit
    its own check (decision 33).

**The second fact-check's fixes**

A second read-only fact-check, of the prototype at `ab446e43` and this
plan, found a record error and the old S0 bug behind it, a plan error,
and nits. Each is fixed in one prototype commit, `e8d10c4f 5.1a
prototype: the second fact-check's fixes`, and in the split's tasks;
each new or strengthened test was seen to fail with its fix taken out.

42. **S0's stack, recorded** (Task 4; decision 20). ADR 0051 and decision
    20 said both lexers were loops, so Windows' 1 MB main-thread stack
    could not limit them. That is false for S0 as the tests build it:
    `logos`'s generated code for a decimal number recurses once per
    character. Measured again here: under `ulimit -s 1024`, a decimal
    literal of 4,000 characters, digits, digits with `_`, or digits
    before `.5`, aborts debug S0 with `thread 'main' has overflowed its
    stack`, exit code 134, where 3,000 pass, and N lexes all three; under
    8 MB, S0 lexes 16,000 and aborts at 32,000; hexadecimal, binary and
    octal numbers, identifiers, strings and comments of 40,000 characters
    pass under 1 MB. No generator makes such a number. Both texts are
    corrected, and ADR 0051's consequences record it, with these figures,
    as a known S0 bug that 5.1b's big-stack thread (spec §4.4) will
    cover; no fixed case is added now.
43. **The figures' scripts are in the plan** (Task 5). Steps 1 and 4 gave
    figures, 2806 tests in 110 binaries before and 2825 in 111 after,
    590 parse runs and 261 checks, with no command that produced them.
    The three scripts the replay ran are now in those steps, verbatim, as
    commands that write them to `$P`: `names.py`, which compares the two
    lists binary by binary; `parsecmp.sh`, which finds its files by
    `git ls-tree -r --name-only 25be9f3c` and the base's text by
    `git archive 25be9f3c`; and `checkcmp.sh`, which finds a file's
    package root as the nearest directory above it holding a `nova.toml`.
    The replay ran these files, and its figures are theirs.
44. **The usage line with no file name** (Task 1; decision 39). When the
    first argument has no file name in UTF-8, `""`, `/`, `..` or the
    bytes ff fe, the base's clap prints `Usage: parse [OPTIONS] <FILE>`,
    and the prototype printed `Usage: nova parse …`, since clap's usage
    name falls back on the command's name, `nova`, once the whole command
    is built. The error is now built from a copy of the `parse`
    subcommand alone, named by `parse_bin_name`, `{name} parse` or `parse`
    alone, and, as clap builds it, from the error's context, its argument
    and usage, so that a terminal shows the base's colours as well, where
    the old message, plain text, left the argument uncoloured.
    `usagecmp.sh` (Task 5, Step 5) runs both binaries by `execv` with
    each such first argument and `othernova`: twenty runs identical, and
    two in a pseudo-terminal. The flag's test runs the four on Unix,
    where `Command::arg0` is (mutants 24 and
    25).
45. **The speed, recorded** (Task 4). Measured with the debug runtime, as
    the tests build N, not the optimized runtime of 5.1b's measurement:
    both lexers are quadratic on a run of overflowing decimal digits,
    since `logos`'s rule moves one character on and lexes the rest again:
    N takes 5.7 s on 16,000 digits, 23 s on 32,000 and 89 s on 64,000,
    debug S0 7.3 s on 16,000. Otherwise both are linear, but N is about
    10 to 15 times slower than debug S0: 5 MB of `lexer.nova` repeated,
    11.8 s against 1.1 s; 1 MB of `{`, 10.2 s against 0.83 s; 1 MB of bad
    escapes, 2.2 s against 0.12 s. Spot-checked here: 16,000 digits, N
    5.7 s and S0 6.9 s; 1 MB of `{`, N 10.2 s and S0 0.84 s. ADR 0051's
    consequences record them.
46. **A caller that stops using a tested helper** (Task 3). `once` and
    `output_within` have tests, but nothing failed if `selfhost()` or
    `run()` stopped using them. `selfhost()` is pinned now:
    `a_failed_build_is_made_once_and_reported_to_each_test` also asks
    that two calls give one path (`std::ptr::eq`), which a `selfhost()`
    that builds N each time fails (mutant 38). Four
    bypasses run as mutants, pass, and are accepted:
    - 39, `selfhost()` with a cache of its own that
      panics inside, as before the first fact-check: it differs only when
      N's build fails, which no test can cause in the real `selfhost()`
      short of breaking `selfhost/`;
    - 40 and 41, `run()` or N's
      build by `Command::output`, with no limit: they differ only when a
      side or the build runs past two minutes, which no test can make
      happen cheaply, on every system, without a broken N;
    - 42, `output_within` joining its readers with no
      deadline: it differs only when a process the child started holds
      its pipes, which needs a second program, and the tests run only
      `nova` and N (Global Constraints).

    Each is one call a reader sees, and the review reads them (Task 5,
    Step 10). Making each bypass fail a test would take a test hook in
    each caller, or a slow test: more code, or more time, than the risk.
47. **`output_within` after an exit, and when it cannot wait** (Task 2).
    After a normal exit it joined its readers with no deadline, so a
    process the child started that held a pipe would hang the harness.
    It now reads them within what is left of the limit, by `read_by`, and
    reports a pipe still open then
    (`a_pipe_still_open_at_the_deadline_is_reported`: two readers, one
    held for two seconds, and a deadline of 100 ms; mutant
    35). When `try_wait` fails, the child is
    killed before the error. A comment and ADR 0051 say that `kill`
    reaches the direct child alone: an N `@test` that loops leaves its
    test process running, orphaned, when `nova test` is killed, but that
    process holds none of the harness's pipes, since `nova test` reads
    its output itself. `try_wait`'s failure has no test: nothing here
    makes it fail.
48. **A missing final line break is marked** (Task 3). Two lines that
    differ only because one side lacks the final `\n` read the same in a
    report. `first_difference` now marks the one without it
    `(no line break)` (`a_missing_final_line_break_is_marked`; mutants
    36 and 37).

**What the replay met**

49. **Nothing unexpected in the code.** Every step gave its "Expected:"
    on its first run, and no test failed but the IPv6 one, in four whole
    suites and CI's leg; the group's peak was 64 tasks of 600, on 4 CPUs.
    The suite took 6 min 20 s to 6 min 52 s a run. Free disk fell to 5.9G
    after the golden suite, under the 6G floor, and was 6.1G again at the
    next step, once older duplicate test executables were removed: keep the
    floor before the golden suite too.

---
### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-51a`: from `25be9f3c`,
`main` after 5.0c, the spec's two commits, `f3bd8fe0` (the spec, for
approval) and `3bef7004` (approved), then this plan's commit, which adds
this file alone, so the patches apply to it as to `3bef7004`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-51a`.
  - `git log --format='%h %s' 25be9f3c..HEAD~1` prints the two spec
    commits, `3bef7004 docs: the Phase 5.1 spec is approved` and
    `f3bd8fe0 docs: the Phase 5.1 spec (the lexer, the parser and the AST), for approval`.
  - `git diff --stat 3bef7004 HEAD` lists only
    `docs/superpowers/plans/2026-10-30-phase-5-1a-the-self-hosted-lexer.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 5's review reads from it.
  - `df -h /` shows at least 6G free; if not, remove older duplicate test
    executables in `target/debug/deps` first. The replay began with
    6.6G free.
- [ ] **Step 2: The tools and the process limit.**
  - `clang-18 --version | head -n 1` prints
    `Ubuntu clang version 18.1.3 (1ubuntu1)`: CI's pin, for the golden
    suite.
  - `node --version` prints `v22.22.0`, for the WASM tests.
  - `RUSTUP_TOOLCHAIN=1.78.0 cargo --version` prints `cargo 1.78.0`, for
    Task 5.
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
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-51a.txt" 2>&1
  grep '^test result' "$P/base-51a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-51a.txt"
  cp target/debug/nova "$P/nova-base-51a"
  target/debug/nova parse --help > "$P/base-help-51a.txt"
  cargo test --locked --workspace -- --list > "$P/base-list-51a.txt" 2>&1
  grep -c ': test$' "$P/base-list-51a.txt"
  ```

  Expected, from 5.0c's last full run on `25be9f3c` and not the replay's:
  `2795 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. The replay built `25be9f3c` and listed its tests, and
  found `2806` names, the run's 2795 + 1 + 10. The copy of `nova`
  is the base's, and the help text its `nova parse --help`, for Task 5's
  comparisons; the list is the base's names, for Task 5's comparison.
  Task 5 expects `2816 1 10`: 5.1a's tests make 21 more, 21
  added and 0 gone.
- [ ] **Step 4: The ledger.** Start `$P/ledger-51a.md`, headed with the
  base commit, the baseline's figures and the process limit's form.

### Task 1: S0's token dump and the hidden flag

Spec §3.3; decisions 4, 7, 16 to 18, 23, 29, 39, 41 and 44.

**Files:**
- Create: `crates/nova-lexer/src/dump.rs` (`dump_tokens`)
- Modify: `crates/nova-lexer/src/lib.rs` (`mod dump`, the export),
  `crates/nova-cli/src/cmd/parse.rs` (`--dump-tokens`, `more`, the
  headers, `IOERR`, the exit code)
- Create: `crates/nova-cli/tests/selfhost.rs` (Task 1's part: S0's tests)
- Test: `crates/nova-lexer/tests/lexer_tests.rs` (two), `selfhost.rs`
  (two)

- [ ] **Step 1: The tests.** `patch_block t1-tests`. The patch holds:
  - `lexer_tests.rs`: `the_token_dump_writes_every_field`, one source
    whose dump has each field, its lines written out: the spans, keywords
    and punctuation by name, `Int`s plain and not (`07`, `1_0`, `0x1`,
    `12`), `2.5`'s and `0.1`'s bits, the second's hex with letters, `'é'` as
    233, a string holding a control byte, DEL, `"`, `\` and `é`, a raw string, a doc comment trimmed, the
    tuple-index split, `NL` after a comment, an unexpected control byte and
    a bad escape in order, a line and a block comment; and
    `the_token_dump_of_nothing_is_eof`;
  - `selfhost.rs`, its doc's first bullet, `nova()`, `fresh_dir()`, and
    `the_token_dump_flag_is_hidden` (absent from `--help`; a second file
    without the flag refused in clap's words, which name no hidden flag,
    and by a hard link named `nova-renamed` with that name in its usage
    line, and on Unix with each first argument that has no file name in
    UTF-8, `""`, `/`, `..` and the bytes ff fe, with `parse` alone in it;
    the flag's conflicts with `--ast` and `--check`) and
    `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`
    (`a.nova`, `missing.nova`, `b.nova`: two dumps around `IOERR NotFound`,
    exit code 2).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which
  ends with `Finished`.
  - `cargo test --locked --workspace --test lexer_tests -- the_token_dump`.
    Expected: it does not compile,
    ``error[E0432]: unresolved import `nova_lexer::dump_tokens` `` and
    ``error: could not compile `nova-lexer` (test "lexer_tests") due to 1 previous error``
    (decision 29).
  - `SH`. Expected: `0 passed; 2 failed`:
    `the_token_dump_flag_is_hidden` at its first conflict, `--ast: error: unexpected argument '--dump-tokens' found`,
    after its first half, the renamed and nameless first arguments' usage
    lines included, passed as at the base (decision 29); and
    `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`,
    its output empty (`left: ""`).
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`dump.rs`:** `dump_tokens(source)`, from `lex_with_comments`: each
    token's line, its payload by kind, `NL` then `PD`; each error's line,
    its message's control bytes escaped; each comment's line. Its module
    doc is the format's statement.
  - **`lib.rs`:** `mod dump;` and `pub use dump::dump_tokens;`.
  - **`parse.rs`:** `--dump-tokens` (`hide = true`,
    `conflicts_with_all = ["ast", "check"]`) and the hidden `more`; with the
    flag, `dump_tokens` writes each file's header and dump, or `IOERR` and
    its kind by `io_kind_name`, then exits 2 if any file failed; without
    it, a file in `more` is refused with clap's `UnknownArgument` error,
    built as clap builds it from a copy of the `parse` subcommand, so it
    reads as before, colours included, with the command named as clap
    names it by `parse_bin_name`: the file name of the first argument and
    `parse`, or `parse` alone when it has none in UTF-8 (decisions 39 and
    44).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --test lexer_tests -- the_token_dump`.
    Expected: `2 passed; 0 failed`.
  - `SH`. Expected: `2 passed; 0 failed`.
  - `S` (6 min 52 s). Expected: `2799 1 10`: the base's figures and
    Task 1's 4 tests, the one failure the IPv6 test.
- [ ] **Step 5: The checks,** as the conventions list them, without
  `selfhost`.
- [ ] **Step 6: Commit**
  `lexer, cli: the token dump, and the hidden nova parse --dump-tokens`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
new file mode 100644
index 0000000000000000000000000000000000000000..ab4a074f843ebeade5684d424b2deef7c2fd882b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -0,0 +1,115 @@
+//! The tests of the self-hosted lexer (spec
+//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+//! §3 and §6.2, ADR 0051):
+//!
+//! - S0's hidden `nova parse --dump-tokens FILE…`: absent from `--help`, a
+//!   header per file, and `IOERR` with the exit code 2 for a file it cannot
+//!   read.
+//!
+//! Each test writes its inputs to fixed directories under the system temp
+//! directory.
+
+use std::env::consts::EXE_SUFFIX;
+use std::path::PathBuf;
+use std::process::Command;
+
+fn nova() -> PathBuf {
+    PathBuf::from(env!("CARGO_BIN_EXE_nova"))
+}
+
+/// A fresh, empty directory under the system temp dir. Its name is fixed,
+/// so each run replaces the last run's.
+fn fresh_dir(name: &str) -> PathBuf {
+    let dir = std::env::temp_dir().join(format!("nova-selfhost-{name}"));
+    let _ = std::fs::remove_dir_all(&dir);
+    std::fs::create_dir_all(&dir).expect("create the test directory");
+    dir
+}
+
+#[test]
+fn the_token_dump_flag_is_hidden() {
+    let help = Command::new(nova())
+        .args(["parse", "--help"])
+        .output()
+        .expect("run nova parse --help");
+    let help = String::from_utf8_lossy(&help.stdout);
+    assert!(help.contains("--check"), "{help}");
+    assert!(!help.contains("dump-tokens"), "{help}");
+    // A second file without the flag is refused as it was before the flag.
+    let extra = Command::new(nova())
+        .args(["parse", "a.nova", "b.nova"])
+        .output()
+        .expect("run nova parse");
+    let stderr = String::from_utf8_lossy(&extra.stderr);
+    assert_eq!(extra.status.code(), Some(2), "{stderr}");
+    assert!(
+        stderr.contains("unexpected argument 'b.nova' found"),
+        "{stderr}"
+    );
+    assert!(!stderr.contains("dump-tokens"), "{stderr}");
+    // Its usage line names the binary by the file name it was run by, as
+    // clap's own errors do: here a link to `nova` under another name.
+    let renamed = fresh_dir("renamed").join(format!("nova-renamed{EXE_SUFFIX}"));
+    if std::fs::hard_link(nova(), &renamed).is_err() {
+        std::fs::copy(nova(), &renamed).expect("copy nova");
+    }
+    let extra = Command::new(&renamed)
+        .args(["parse", "a.nova", "b.nova"])
+        .output()
+        .expect("run nova under another name");
+    let stderr = String::from_utf8_lossy(&extra.stderr);
+    let usage = format!("\nUsage: nova-renamed{EXE_SUFFIX} parse [OPTIONS] <FILE>\n");
+    assert!(stderr.contains(&usage), "{stderr}");
+    // A first argument with no file name in UTF-8 gives clap no name, and
+    // its usage line says `parse` alone. Only Unix's `Command` sets it.
+    #[cfg(unix)]
+    {
+        use std::ffi::OsStr;
+        use std::os::unix::ffi::OsStrExt;
+        use std::os::unix::process::CommandExt;
+        for arg0 in [&b""[..], b"/", b"..", b"\xff\xfe"] {
+            let extra = Command::new(nova())
+                .arg0(OsStr::from_bytes(arg0))
+                .args(["parse", "a.nova", "b.nova"])
+                .output()
+                .expect("run nova with another first argument");
+            let stderr = String::from_utf8_lossy(&extra.stderr);
+            assert!(
+                stderr.contains("\nUsage: parse [OPTIONS] <FILE>\n"),
+                "{arg0:?}: {stderr}"
+            );
+        }
+    }
+    // The flag takes neither `--ast` nor `--check`.
+    for other in ["--ast", "--check"] {
+        let both = Command::new(nova())
+            .args(["parse", "--dump-tokens", other, "a.nova"])
+            .output()
+            .expect("run nova parse");
+        let stderr = String::from_utf8_lossy(&both.stderr);
+        assert_eq!(both.status.code(), Some(2), "{other}: {stderr}");
+        assert!(stderr.contains("cannot be used with"), "{other}: {stderr}");
+    }
+}
+
+/// Each file's dump follows a header with its path as given; a file that
+/// cannot be read is `IOERR` and its kind, the files after it are dumped,
+/// and the exit code is 2.
+#[test]
+fn the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one() {
+    let dir = fresh_dir("headers");
+    std::fs::write(dir.join("a.nova"), "let a = 1\n").expect("write a.nova");
+    std::fs::write(dir.join("b.nova"), "b").expect("write b.nova");
+    let out = Command::new(nova())
+        .current_dir(&dir)
+        .args(["parse", "--dump-tokens", "a.nova", "missing.nova", "b.nova"])
+        .output()
+        .expect("run nova parse --dump-tokens");
+    assert_eq!(
+        String::from_utf8_lossy(&out.stdout),
+        "== a.nova\n0 3 Let\n4 5 Ident \"a\"\n6 7 Eq\n8 9 Int 1 PD\n10 10 Eof NL\n\
+         == missing.nova\nIOERR NotFound\n\
+         == b.nova\n0 1 Ident \"b\"\n1 1 Eof\n"
+    );
+    assert_eq!(out.status.code(), Some(2));
+}
diff --git a/crates/nova-lexer/tests/lexer_tests.rs b/crates/nova-lexer/tests/lexer_tests.rs
index 27debe408d896fe458f44ab2054a0fe21edef943..22775046d9e775effc92c4a521896c3ee307dee7 100644
--- a/crates/nova-lexer/tests/lexer_tests.rs
+++ b/crates/nova-lexer/tests/lexer_tests.rs
@@ -1,5 +1,5 @@
 use nova_diagnostics::FileDb;
-use nova_lexer::{lex, lex_with_comments, CommentKind, Token};
+use nova_lexer::{dump_tokens, lex, lex_with_comments, CommentKind, Token};
 use std::path::{Path, PathBuf};
 
 fn tokens(source: &str) -> Vec<Token> {
@@ -850,3 +850,55 @@ fn only_plain_decimal_integers_are_marked_as_tuple_indexes() {
     assert!(!tokens.plain_decimal(tokens.len() - 2));
     assert!(!tokens.plain_decimal(tokens.len() + 3));
 }
+
+// --- The token dump (5.1 spec §3.3, ADR 0051) ---
+
+/// Every field of the dump: the spans; the kinds' names; `Int` in decimal,
+/// `Float` as its bits in lowercase hex, `Char` as its code point, text in
+/// quotes with its control bytes, DEL, `"` and `\` escaped and non-ASCII
+/// kept; the `NL` and `PD` marks; `Eof` last; then the errors in order, a
+/// control byte in a message escaped and a `\` kept; then the comments.
+#[test]
+fn the_token_dump_writes_every_field() {
+    let source = "let s = \"a\x01\x7f\\\"\\\\é\" r#\"q\\\"# // c\n\
+                  /* b */ 07 1_0 0x1 12 2.5 'é' t.1.2 \x02 \"\\q\"\n/// doc \nfn 0.1";
+    let want = [
+        "0 3 Let",
+        r#"4 5 Ident "s""#,
+        "6 7 Eq",
+        "8 9 StrStart",
+        r#"9 18 StrPart "a\x01\x7f\x22\x5cé""#,
+        "18 19 StrEnd",
+        r#"20 27 RawStr "q\x5c""#,
+        "41 43 Int 7 NL",
+        "44 47 Int 10",
+        "48 51 Int 1",
+        "52 54 Int 12 PD",
+        "55 58 Float 4004000000000000",
+        "59 63 Char 233",
+        r#"64 65 Ident "t""#,
+        "65 66 Dot",
+        "66 67 Int 1 PD",
+        "67 68 Dot",
+        "68 69 Int 2 PD",
+        "72 73 StrStart",
+        r#"74 75 StrPart "q""#,
+        "75 76 StrEnd",
+        r#"77 85 DocComment " doc" NL"#,
+        "86 88 Fn NL",
+        "89 92 Float 3fb999999999999a",
+        "92 92 Eof",
+        r"ERR 70 71 unexpected character `\x02`",
+        r"ERR 73 74 invalid escape sequence `\q`",
+        "COMMENT 28 32 Line",
+        "COMMENT 33 40 Block",
+    ];
+    assert_eq!(dump_tokens(source), want.join("\n") + "\n");
+}
+
+/// An empty source is `Eof` alone, and every line ends with `\n`.
+#[test]
+fn the_token_dump_of_nothing_is_eof() {
+    assert_eq!(dump_tokens(""), "0 0 Eof\n");
+    assert_eq!(dump_tokens("\r\n1"), "2 3 Int 1 NL PD\n3 3 Eof\n");
+}
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-cli/src/cmd/parse.rs b/crates/nova-cli/src/cmd/parse.rs
index 674d601be9e4d24ad93fb5ba609babefc0d9c57a..1283bb95b9f37b7b2f5591f9653aa5a4aaf90e04 100644
--- a/crates/nova-cli/src/cmd/parse.rs
+++ b/crates/nova-cli/src/cmd/parse.rs
@@ -1,6 +1,13 @@
 //! `nova parse <file>` — parse a Nova source file and print the AST.
+//!
+//! The hidden `--dump-tokens FILE…` prints each file's token dump instead,
+//! what the self-hosted lexer's `selfhost tokens` prints (spec
+//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+//! §3.3, ADR 0051).
 
-use std::path::PathBuf;
+use std::ffi::{OsStr, OsString};
+use std::io::Write;
+use std::path::{Path, PathBuf};
 
 use anyhow::{Context, Result};
 use clap::Args;
@@ -20,9 +27,50 @@ pub struct ParseCmd {
     /// Print only errors (exit 1 if any).
     #[arg(long)]
     check: bool,
+
+    /// Print the token dump of FILE and of every further file given, for
+    /// the self-hosted lexer's differential test. Hidden: it carries no
+    /// stability promise.
+    #[arg(long, hide = true, conflicts_with_all = ["ast", "check"])]
+    dump_tokens: bool,
+
+    /// The files after FILE, which only `--dump-tokens` takes.
+    #[arg(hide = true)]
+    more: Vec<PathBuf>,
 }
 
 pub fn run(cmd: ParseCmd) -> Result<()> {
+    if cmd.dump_tokens {
+        let files: Vec<&Path> = std::iter::once(cmd.file.as_path())
+            .chain(cmd.more.iter().map(PathBuf::as_path))
+            .collect();
+        let code = dump_tokens(&files)?;
+        if code != 0 {
+            std::process::exit(code);
+        }
+        return Ok(());
+    }
+    if let Some(extra) = cmd.more.first() {
+        // What clap said before `more` existed, without naming the hidden
+        // flag: its "unexpected argument" error, built as clap builds it,
+        // so a terminal shows it in the same colours, with its usage line
+        // naming the command as clap does (`parse_bin_name`).
+        use clap::error::{ContextKind, ContextValue, ErrorKind};
+        let cli = <crate::Cli as clap::CommandFactory>::command();
+        let mut parse = cli
+            .find_subcommand("parse")
+            .expect("parse is a subcommand")
+            .clone()
+            .bin_name(parse_bin_name(std::env::args_os().next()));
+        let usage = parse.render_usage();
+        let mut error = clap::Error::new(ErrorKind::UnknownArgument).with_cmd(&parse);
+        error.insert(
+            ContextKind::InvalidArg,
+            ContextValue::String(extra.display().to_string()),
+        );
+        error.insert(ContextKind::Usage, ContextValue::StyledStr(usage));
+        error.exit();
+    }
     let source = std::fs::read_to_string(&cmd.file)
         .with_context(|| format!("failed to read {}", cmd.file.display()))?;
 
@@ -87,3 +135,57 @@ pub fn run(cmd: ParseCmd) -> Result<()> {
         )
     }
 }
+
+/// The name clap's usage line gives `nova parse`, from the binary's first
+/// argument `arg0`, as clap names it: that argument's file name, then
+/// `parse`, so a renamed `nova` prints its own; or `parse` alone when the
+/// argument has no file name in UTF-8 (`""`, `/`, `..`, bytes that are not
+/// UTF-8) or there is none.
+fn parse_bin_name(arg0: Option<OsString>) -> String {
+    let name = arg0
+        .as_deref()
+        .and_then(|arg0| Path::new(arg0).file_name())
+        .and_then(OsStr::to_str);
+    match name {
+        Some(name) => format!("{name} parse"),
+        None => "parse".to_string(),
+    }
+}
+
+/// `--dump-tokens`: for each file, `== FILE` (the path as given), then its
+/// token dump; for a file that cannot be read, `== FILE` then `IOERR` and
+/// its kind, after which the rest are dumped and the exit code is 2.
+fn dump_tokens(files: &[&Path]) -> Result<i32> {
+    let stdout = std::io::stdout();
+    let mut out = stdout.lock();
+    let mut code = 0;
+    for file in files {
+        writeln!(out, "== {}", file.display())?;
+        match std::fs::read_to_string(file) {
+            Ok(source) => out.write_all(nova_lexer::dump_tokens(&source).as_bytes())?,
+            Err(e) => {
+                writeln!(out, "IOERR {}", io_kind_name(e.kind()))?;
+                code = 2;
+            }
+        }
+    }
+    out.flush()?;
+    Ok(code)
+}
+
+/// The kind as std/io's `IoErrorKind` names it, so N's dump, whose reads
+/// fail with those kinds, prints the same word: the seven kinds it has, and
+/// `Other` for every other.
+fn io_kind_name(kind: std::io::ErrorKind) -> &'static str {
+    use std::io::ErrorKind;
+    match kind {
+        ErrorKind::NotFound => "NotFound",
+        ErrorKind::PermissionDenied => "PermissionDenied",
+        ErrorKind::AlreadyExists => "AlreadyExists",
+        ErrorKind::InvalidData => "InvalidData",
+        ErrorKind::Interrupted => "Interrupted",
+        ErrorKind::TimedOut => "TimedOut",
+        ErrorKind::ConnectionRefused => "ConnectionRefused",
+        _ => "Other",
+    }
+}
diff --git a/crates/nova-lexer/src/dump.rs b/crates/nova-lexer/src/dump.rs
new file mode 100644
index 0000000000000000000000000000000000000000..8a60345d7c1bcc00a33291a98c0b9cfc82219bb1
--- /dev/null
+++ b/crates/nova-lexer/src/dump.rs
@@ -0,0 +1,120 @@
+//! The token dump: one source's tokens, lex errors and comments as text, one
+//! item per line, which the self-hosted lexer (`selfhost/`) prints too, so
+//! the two lexers can be compared byte for byte (spec
+//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+//! §3.3, ADR 0051). It carries no stability promise: it changes whenever
+//! the tokens do, on both sides in one pull request.
+//!
+//! Per source, in this order:
+//!
+//! - one line per token, `Eof` last: `start end Kind[ payload][ NL][ PD]`.
+//!   - `start` and `end` are the span's byte offsets, in decimal.
+//!   - `Kind` is the [`Token`] variant's name, as its `Debug` prints it.
+//!   - The payload: `Int` in decimal; `Float` as the 16 lowercase hex digits
+//!     of `f64::to_bits`; `Char` as its code point in decimal; `StrPart`,
+//!     `RawStr`, `Ident` and `DocComment` as their text in double quotes,
+//!     with each byte below 0x20, 0x7F, `"` and `\` written `\xHH` (two
+//!     lowercase hex digits) and every other byte as it is. No other kind
+//!     has one.
+//!   - `NL` when [`Tokens::line_break_before`] holds, then `PD` when
+//!     [`Tokens::plain_decimal`] does.
+//! - one line per lex error, in the order the lexer found them:
+//!   `ERR start end message`, the message being the error's `Display`, with
+//!   each byte below 0x20 and 0x7F written `\xHH`; `"` and `\` stay as they
+//!   are, since the message is not quoted.
+//! - one line per comment, in source order: `COMMENT start end Line` or
+//!   `COMMENT start end Block`.
+//!
+//! Every line ends with `\n`, the last one included. The `== FILE` header of
+//! `nova parse --dump-tokens` is the command's, not this module's.
+
+use std::fmt::Write;
+
+use nova_diagnostics::FileId;
+
+use crate::{lex_with_comments, CommentKind, Token, Tokens};
+
+/// The token dump of `source` (module docs).
+pub fn dump_tokens(source: &str) -> String {
+    let (tokens, comments, errors) = lex_with_comments(source, FileId::DUMMY);
+    let mut out = String::with_capacity(source.len() * 4);
+    dump_token_lines(&mut out, &tokens);
+    for e in &errors {
+        let span = e.span();
+        let _ = write!(out, "ERR {} {} ", span.start, span.end);
+        escape_message(&mut out, &e.to_string());
+        out.push('\n');
+    }
+    for c in &comments {
+        let kind = match c.kind {
+            CommentKind::Line => "Line",
+            CommentKind::Block => "Block",
+        };
+        let _ = writeln!(out, "COMMENT {} {} {kind}", c.span.start, c.span.end);
+    }
+    out
+}
+
+fn dump_token_lines(out: &mut String, tokens: &Tokens) {
+    for (i, t) in tokens.iter().enumerate() {
+        let _ = write!(out, "{} {} ", t.span.start, t.span.end);
+        match &t.value {
+            Token::Int(n) => {
+                let _ = write!(out, "Int {n}");
+            }
+            Token::Float(f) => {
+                let _ = write!(out, "Float {:016x}", f.to_bits());
+            }
+            Token::Char(c) => {
+                let _ = write!(out, "Char {}", *c as u32);
+            }
+            Token::StrPart(s) => quoted(out, "StrPart", s),
+            Token::RawStr(s) => quoted(out, "RawStr", s),
+            Token::Ident(s) => quoted(out, "Ident", s),
+            Token::DocComment(s) => quoted(out, "DocComment", s),
+            other => {
+                let _ = write!(out, "{other:?}");
+            }
+        }
+        if tokens.line_break_before(i) {
+            out.push_str(" NL");
+        }
+        if tokens.plain_decimal(i) {
+            out.push_str(" PD");
+        }
+        out.push('\n');
+    }
+}
+
+fn quoted(out: &mut String, kind: &str, text: &str) {
+    out.push_str(kind);
+    out.push_str(" \"");
+    escape_text(out, text);
+    out.push('"');
+}
+
+/// A string payload's text: each byte below 0x20, 0x7F, `"` and `\` as
+/// `\xHH`, two lowercase hex digits; every other byte, so every non-ASCII
+/// character, as it is.
+fn escape_text(out: &mut String, text: &str) {
+    for c in text.chars() {
+        if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {
+            let _ = write!(out, "\\x{:02x}", c as u32);
+        } else {
+            out.push(c);
+        }
+    }
+}
+
+/// An error's message: each byte below 0x20 and 0x7F as `\xHH`, so the
+/// message stays on its line; `"` and `\` as they are, since the message is
+/// not quoted.
+fn escape_message(out: &mut String, text: &str) {
+    for c in text.chars() {
+        if (c as u32) < 0x20 || c == '\x7f' {
+            let _ = write!(out, "\\x{:02x}", c as u32);
+        } else {
+            out.push(c);
+        }
+    }
+}
diff --git a/crates/nova-lexer/src/lib.rs b/crates/nova-lexer/src/lib.rs
index 9e6d237c391c0306ffec17fe3f97ad095a040414..b137e0e0b62909aa210a691afdc53bf87c2c7b89 100644
--- a/crates/nova-lexer/src/lib.rs
+++ b/crates/nova-lexer/src/lib.rs
@@ -23,10 +23,12 @@
 //! assert!(errors.is_empty());
 //! ```
 
+mod dump;
 mod error;
 mod token;
 mod tokens;
 
+pub use dump::dump_tokens;
 pub use error::LexError;
 pub use token::Token;
 pub use tokens::Tokens;
````

### Task 2: The self-hosted lexer and its own tests

Spec §3.1, §3.2; decisions 5, 6, 13, 19, 25, 29, 35, 36, 40 and 47.

**Files:**
- Create: `selfhost/nova.toml`, `selfhost/src/main.nova`,
  `selfhost/src/token.nova`, `selfhost/src/lexer.nova`,
  `selfhost/src/dump.nova`, `selfhost/src/lexer_tests.nova`
- Modify: `.gitignore` (`/selfhost/target`), `selfhost.rs`
  (`root()`, `CHILD_LIMIT`, `output_within`, `read_by`, `selfhost_test_count`,
  `the_selfhost_tests_pass`)
- Test: N's eleven `@test`s in `lexer_tests.nova`, run by
  `the_selfhost_tests_pass`

- [ ] **Step 1: The tests.** `patch_block t2-tests`. The patch holds:
  - `selfhost/src/lexer_tests.nova`: eleven `@test`s, ten of them a source
    and the dump S0 prints for it, written out: the escapes and bad ones; raw
    strings and the one `InvalidNumberLiteral`; holes nesting and counting
    their braces, one left open; an overflowing integer as an unexpected
    first digit; the float split after a dot; line breaks marked through
    comments; chars and `'\q'`'s three errors; doc comments as exactly three
    slashes; keywords as whole identifiers; unexpected characters and an
    open block comment; the eleventh checks thirteen kinds' descriptions
    against `token.rs`'s. A Unicode escape is spelled `u("{41}")`
    (decision 25);
  - `selfhost.rs`: N's bullet, `root()`, `CHILD_LIMIT` and
    `output_within`, which kills a process past its limit, or one it
    cannot wait for, and reads its pipes after it exits within what is
    left of the limit by `read_by` (decisions 36 and 47),
    `selfhost_test_count`, which counts the lines that are `@test` in N's
    sources, and `the_selfhost_tests_pass`, which runs `nova test` in
    `selfhost/` with a fresh `NOVA_HOME`, within the limit, and asks for
    that many passed and ` 0 failed` (decision 35).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which
  ends with `Finished`.
  - `SH`. Expected: `2 passed; 1 failed`: Task 1's two pass, and
    `the_selfhost_tests_pass` fails, `` ` 11 passed; 0 failed;` is not in: ``,
    `nova test` in a directory with no package saying
    `Error: failed to open src/main.nova` and
    `No such file or directory (os error 2)`.
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`nova.toml`:** `selfhost` 0.1.0, edition 2026.
  - **`token.nova`:** `Token`'s 92 variants, `Span`, `Spanned`,
    `spanned`, and `description()` word for word.
  - **`lexer.nova`:** `LexError` and its `Display`; `Comment`; `lex` and
    `lex_with_comments`; `Tokens`, `marked` and the two marks; the lexer:
    `tokenize`, `split_tuple_indexes`, `skip_trivia`, `next_token`, the
    strings' and raw strings' scanners, `hex_u32`, `maybe_close_interp`,
    `lex_logos` and `raw_token` with `punct`, `lex_number`, `int_value`,
    `lex_float` and `lex_char`; `utf8_char_width`, `keyword_or_ident`,
    `is_white_space`.
  - **`dump.nova`:** `dump_tokens`, the 92-arm `kind_name`, the escapes,
    `hex16`.
  - **`main.nova`:** `selfhost tokens FILE…`, no file refused with exit
    code 2, the headers, `IOERR` by `io_kind_name`, and `exit(2)` after
    the rest; `import lexer_tests`.
  - **`.gitignore`:** `/selfhost/target`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH`. Expected: `3 passed; 0 failed`.
  - `(cd selfhost && ../target/debug/nova test) | tail -n 3`. Expected:
    `test result: ok. 11 passed; 0 failed; 0 trapped; 11 total`.
  - `git status --short`. Expected: nothing: neither command wrote into
    the package (decision 19).
  - `S` (6 min 22 s). Expected: `2800 1 10`: Task 1's figures and
    `the_selfhost_tests_pass`.
- [ ] **Step 5: The checks,** as the conventions list them, with
  `selfhost` from here on.
- [ ] **Step 6: Commit** `selfhost: the self-hosted lexer and its own tests`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index ab4a074f843ebeade5684d424b2deef7c2fd882b..9b9fbe615fe4c9e09bcb3215a87b57e61b4de677 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -4,14 +4,103 @@
 //!
 //! - S0's hidden `nova parse --dump-tokens FILE…`: absent from `--help`, a
 //!   header per file, and `IOERR` with the exit code 2 for a file it cannot
-//!   read.
+//!   read;
+//! - N's own `@test`s, which `nova test` runs in `selfhost/`: as many must
+//!   pass as its sources hold.
 //!
 //! Each test writes its inputs to fixed directories under the system temp
 //! directory.
 
 use std::env::consts::EXE_SUFFIX;
-use std::path::PathBuf;
-use std::process::Command;
+use std::io::Read;
+use std::path::{Path, PathBuf};
+use std::process::{Command, Output, Stdio};
+use std::thread::JoinHandle;
+use std::time::{Duration, Instant};
+
+/// The repository, two levels above this crate.
+fn root() -> PathBuf {
+    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
+}
+
+/// How long one process a test starts may run: `nova test`, `nova build`,
+/// or one side on one batch. Each takes a second or two here, and CI's
+/// Windows runners are several times slower; a lexer that loops forever
+/// fails its test at this limit rather than holding CI for hours.
+const CHILD_LIMIT: Duration = Duration::from_secs(120);
+
+/// `cmd`'s output, as `Command::output` gives it, if it exits and its
+/// stdout and stderr close within `limit`; otherwise an error that names it
+/// by `what`. Its stdout and stderr are read on threads of their own, so a
+/// child that fills a pipe cannot stall the wait; its stdin is what `cmd`
+/// sets.
+/// - Past the limit, or if it cannot be waited for, it is killed. `kill`
+///   reaches the child alone, not a process it started: an N `@test` that
+///   looped would leave its test process running, orphaned, when
+///   `nova test` is killed, but that process holds none of these pipes,
+///   since `nova test` reads its output itself.
+/// - After it exits, its pipes are read to their end within what is left
+///   of the limit ([`read_by`]): a process it started may still hold them
+///   open, and that is reported rather than waited for. After a kill the
+///   readers are not waited for at all.
+fn output_within(cmd: &mut Command, limit: Duration, what: &str) -> Result<Output, String> {
+    fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> JoinHandle<Vec<u8>> {
+        std::thread::spawn(move || {
+            let mut bytes = Vec::new();
+            if let Some(mut pipe) = pipe {
+                let _ = pipe.read_to_end(&mut bytes);
+            }
+            bytes
+        })
+    }
+    let mut child = cmd
+        .stdout(Stdio::piped())
+        .stderr(Stdio::piped())
+        .spawn()
+        .map_err(|e| format!("{what} could not start: {e}"))?;
+    let stdout = drain(child.stdout.take());
+    let stderr = drain(child.stderr.take());
+    let deadline = Instant::now() + limit;
+    let status = loop {
+        match child.try_wait() {
+            Ok(Some(status)) => break status,
+            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
+            Ok(None) => {
+                let _ = child.kill();
+                let _ = child.wait();
+                return Err(format!("{what} did not finish within {limit:?}: killed"));
+            }
+            Err(e) => {
+                let _ = child.kill();
+                let _ = child.wait();
+                return Err(format!("{what} could not be waited for: {e}: killed"));
+            }
+        }
+    };
+    let [stdout, stderr] = read_by([stdout, stderr], deadline).ok_or_else(|| {
+        format!(
+            "{what} exited ({status}), but its stdout or stderr was still open at \
+             {limit:?}: a process it started holds it"
+        )
+    })?;
+    Ok(Output {
+        status,
+        stdout,
+        stderr,
+    })
+}
+
+/// What each of `readers` read, once both have finished, if they finish by
+/// `deadline`; `None` if either is still reading then.
+fn read_by(readers: [JoinHandle<Vec<u8>>; 2], deadline: Instant) -> Option<[Vec<u8>; 2]> {
+    while readers.iter().any(|reader| !reader.is_finished()) {
+        if Instant::now() >= deadline {
+            return None;
+        }
+        std::thread::sleep(Duration::from_millis(5));
+    }
+    Some(readers.map(|reader| reader.join().expect("read a pipe")))
+}
 
 fn nova() -> PathBuf {
     PathBuf::from(env!("CARGO_BIN_EXE_nova"))
@@ -113,3 +202,48 @@ fn the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one() {
     );
     assert_eq!(out.status.code(), Some(2));
 }
+
+/// How many `@test` functions N's sources hold: the lines that are `@test`
+/// in the `.nova` files of `selfhost/src/` and `selfhost/tests/`.
+fn selfhost_test_count() -> usize {
+    let mut count = 0;
+    for sub in ["src", "tests"] {
+        let Ok(entries) = std::fs::read_dir(root().join("selfhost").join(sub)) else {
+            continue;
+        };
+        for entry in entries {
+            let path = entry.expect("read a directory entry").path();
+            if path.extension().is_some_and(|e| e == "nova") {
+                let text = std::fs::read_to_string(&path).expect("read an N source");
+                count += text.lines().filter(|l| l.trim() == "@test").count();
+            }
+        }
+    }
+    assert!(count > 0, "no @test in selfhost/");
+    count
+}
+
+/// N's own `@test`s. `nova test` writes nothing into the package, so it
+/// runs in place. Each one must pass, so as many must pass as the sources
+/// hold: a run of no test at all fails.
+#[test]
+fn the_selfhost_tests_pass() {
+    let home = fresh_dir("test-home");
+    let out = output_within(
+        Command::new(nova())
+            .current_dir(root().join("selfhost"))
+            .env("NOVA_HOME", &home)
+            .arg("test")
+            .stdin(Stdio::null()),
+        CHILD_LIMIT,
+        "nova test in selfhost/",
+    )
+    .unwrap_or_else(|e| panic!("{e}"));
+    let stdout = String::from_utf8_lossy(&out.stdout);
+    let all = format!(" {} passed; 0 failed;", selfhost_test_count());
+    assert!(
+        out.status.success() && stdout.contains(&all),
+        "`{all}` is not in:\n{stdout}{}",
+        String::from_utf8_lossy(&out.stderr)
+    );
+}
diff --git a/selfhost/src/lexer_tests.nova b/selfhost/src/lexer_tests.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f722ced1626260e110bbb9cc891235c1fb8d9557
--- /dev/null
+++ b/selfhost/src/lexer_tests.nova
@@ -0,0 +1,329 @@
+// N's own tests of the lexer's edges: escapes, raw strings, interpolation
+// holes, the overflow quirk, the float split and the line-break marks. Each
+// expected dump is S0's (`nova parse --dump-tokens`) for the same source.
+// The last test checks some kinds' descriptions against token.rs's, which
+// no dump prints; 5.1b's parser messages compare every one.
+//
+// They live in `src/`, imported by `main.nova`, rather than in `tests/`: a
+// module in a package's `tests/` sees only the files beside it and the
+// package's `src/lib.nova`, and Nova has no re-exports, so it could not
+// reach `lexer.nova` (ADR 0051).
+
+import dump
+import token
+
+// The lines, each ended by a line break, as a dump ends each.
+fn dump_lines(lines: [String]) -> String { "\n".join(lines) + "\n" }
+
+// A backslash, `u` and `rest`: a Unicode escape, or a broken one, in a
+// source, spelled so that no escape of this file's own is one.
+fn u(rest: String) -> String { "\\" + "u" + rest }
+
+@test
+fn escapes_are_decoded_and_bad_ones_reported() {
+    // Each escape decodes into the segment's text; `\q` is an error that drops
+    // the text before it, after which the string goes on (lib.rs's
+    // `lex_string_content`).
+    let source = "\"a\\n\\t\\r\\\"\\\\\\0b\" \""
+        + u("{41}")
+        + u("{1F600}")
+        + "\" \"x\\qy\" \""
+        + u("{D800}")
+        + "\" \""
+        + u("Z")
+        + "\" \""
+        + u("{+41}")
+        + "\""
+    let expected = dump_lines(
+        [
+            "0 1 StrStart",
+            "1 15 StrPart \"a\\x0a\\x09\\x0d\\x22\\x5c\\x00b\"",
+            "15 16 StrEnd",
+            "17 18 StrStart",
+            "18 33 StrPart \"A😀\"",
+            "33 34 StrEnd",
+            "35 36 StrStart",
+            "38 40 StrPart \"qy\"",
+            "40 41 StrEnd",
+            "42 43 StrStart",
+            "51 52 StrEnd",
+            "53 54 StrStart",
+            "56 57 StrPart \"Z\"",
+            "57 58 StrEnd",
+            "59 60 StrStart",
+            "60 67 StrPart \"A\"",
+            "67 68 StrEnd",
+            "68 68 Eof",
+            "ERR 36 38 invalid escape sequence `\\q`",
+            "ERR 43 51 invalid unicode escape sequence",
+            "ERR 54 56 invalid escape sequence `\\u`",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn raw_strings_and_the_one_invalid_number_literal() {
+    // A raw string ends at a `"` with as many `#`s as opened it. `r#` with no
+    // `"` is `InvalidNumberLiteral`, the only source of it; an unterminated
+    // raw string runs to the end.
+    let source = "r\"a\\nb\" r#\"say \"hi\"\"# r##\"x\"#y\"## r#abc r\"open"
+    let expected = dump_lines(
+        [
+            "0 7 RawStr \"a\\x5cnb\"",
+            "8 21 RawStr \"say \\x22hi\\x22\"",
+            "22 33 RawStr \"x\\x22#y\"",
+            "36 39 Ident \"abc\"",
+            "46 46 Eof",
+            "ERR 34 36 invalid numeric literal",
+            "ERR 40 46 unterminated string literal",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn interpolation_holes_nest_and_count_their_braces() {
+    // A hole ends at the `}` that matches its `${`, through record braces and
+    // nested strings; an open hole at the end is reported at its `${`.
+    let source = "\"a$" + "{f(R { v: 1 })}b$" + "{\"c$" + "{d}\"}\" \"x$" + "{ y"
+    let expected = dump_lines(
+        [
+            "0 1 StrStart",
+            "1 2 StrPart \"a\"",
+            "2 4 InterpOpen",
+            "4 5 Ident \"f\"",
+            "5 6 LParen",
+            "6 7 Ident \"R\"",
+            "8 9 LBrace",
+            "10 11 Ident \"v\"",
+            "11 12 Colon",
+            "13 14 Int 1 PD",
+            "15 16 RBrace",
+            "16 17 RParen",
+            "17 18 InterpClose",
+            "18 19 StrPart \"b\"",
+            "19 21 InterpOpen",
+            "21 22 StrStart",
+            "22 23 StrPart \"c\"",
+            "23 25 InterpOpen",
+            "25 26 Ident \"d\"",
+            "26 27 InterpClose",
+            "27 28 StrEnd",
+            "28 29 InterpClose",
+            "29 30 StrEnd",
+            "31 32 StrStart",
+            "32 33 StrPart \"x\"",
+            "33 35 InterpOpen",
+            "36 37 Ident \"y\"",
+            "37 37 Eof",
+            "ERR 33 35 unterminated string interpolation: this `$" + "{` has no matching `}`",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn an_overflowing_integer_is_an_unexpected_first_digit() {
+    // logos's callback refuses an integer past `Int`, which is an
+    // `UnexpectedCharacter` at its first digit; lexing then moves on one
+    // character, so 20 nines give two errors and an integer of 18.
+    let source = "99999999999999999999 0xFFFFFFFFFFFFFFFFF 9223372036854775807 0b2 0x_1"
+    let expected = dump_lines(
+        [
+            "2 20 Int 999999999999999999 PD",
+            "22 40 Ident \"xFFFFFFFFFFFFFFFFF\"",
+            "41 60 Int 9223372036854775807 PD",
+            "61 62 Int 0 PD",
+            "62 64 Ident \"b2\"",
+            "65 66 Int 0 PD",
+            "66 69 Ident \"x_1\"",
+            "69 69 Eof",
+            "ERR 0 1 unexpected character `9`",
+            "ERR 1 2 unexpected character `9`",
+            "ERR 21 22 unexpected character `0`",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn a_float_after_a_dot_splits_into_tuple_indexes() {
+    // After a `.`, `digits.digits` becomes `Int . Int` with exact spans; an
+    // `_`, an exponent, `..` before it, or a part past `Int` keeps the float.
+    let source = "t.1.0 t.0.1.2 t.1.10 t..1.5 1.5 t.1_0.5 t.1e5 x.007.5 t.1.99999999999999999999 0_81.5"
+    let expected = dump_lines(
+        [
+            "0 1 Ident \"t\"",
+            "1 2 Dot",
+            "2 3 Int 1 PD",
+            "3 4 Dot",
+            "4 5 Int 0 PD",
+            "6 7 Ident \"t\"",
+            "7 8 Dot",
+            "8 9 Int 0 PD",
+            "9 10 Dot",
+            "10 11 Int 1 PD",
+            "11 12 Dot",
+            "12 13 Int 2 PD",
+            "14 15 Ident \"t\"",
+            "15 16 Dot",
+            "16 17 Int 1 PD",
+            "17 18 Dot",
+            "18 20 Int 10 PD",
+            "21 22 Ident \"t\"",
+            "22 24 DotDot",
+            "24 27 Float 3ff8000000000000",
+            "28 31 Float 3ff8000000000000",
+            "32 33 Ident \"t\"",
+            "33 34 Dot",
+            "34 39 Float 4025000000000000",
+            "40 41 Ident \"t\"",
+            "41 42 Dot",
+            "42 45 Float 40f86a0000000000",
+            "46 47 Ident \"x\"",
+            "47 48 Dot",
+            "48 51 Int 7",
+            "51 52 Dot",
+            "52 53 Int 5 PD",
+            "54 55 Ident \"t\"",
+            "55 56 Dot",
+            "56 78 Float 4000000000000000",
+            "79 85 Float 4054600000000000",
+            "85 85 Eof",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn line_breaks_are_marked_through_comments() {
+    // A `\n` or a lone `\r` between two tokens marks the second, also inside
+    // a comment; a line comment's span stops before a `\r\n`.
+    let source = "a\n// c\nb /* x\n */ c\r d /* e */ f // g\r\nh \"i\nj\" k"
+    let expected = dump_lines(
+        [
+            "0 1 Ident \"a\"",
+            "7 8 Ident \"b\" NL",
+            "18 19 Ident \"c\" NL",
+            "21 22 Ident \"d\" NL",
+            "31 32 Ident \"f\"",
+            "39 40 Ident \"h\" NL",
+            "41 42 StrStart",
+            "42 45 StrPart \"i\\x0aj\"",
+            "45 46 StrEnd",
+            "47 48 Ident \"k\"",
+            "48 48 Eof",
+            "COMMENT 2 6 Line",
+            "COMMENT 9 17 Block",
+            "COMMENT 23 30 Block",
+            "COMMENT 33 37 Line",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn char_literals_and_the_three_errors_of_a_bad_escape() {
+    // `'\q'` matches the char regex, but its callback refuses it: three
+    // `UnexpectedCharacter`s, at `'`, `\` and `'`, around an identifier.
+    let source = "'a' '\\n' '\\'' '\\q' 'é' '\\\\' ''' "
+    let expected = dump_lines(
+        [
+            "0 3 Char 97",
+            "4 8 Char 10",
+            "9 13 Char 39",
+            "16 17 Ident \"q\"",
+            "17 20 Char 32",
+            "22 25 Char 32",
+            "27 30 Char 32",
+            "33 33 Eof",
+            "ERR 14 15 unexpected character `'`",
+            "ERR 15 16 unexpected character `\\`",
+            "ERR 20 22 unexpected character `é`",
+            "ERR 25 26 unexpected character `\\`",
+            "ERR 26 27 unexpected character `\\`",
+            "ERR 30 31 unexpected character `'`",
+            "ERR 31 32 unexpected character `'`",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn doc_comments_are_exactly_three_slashes() {
+    // `///` is a `DocComment`, its leading space kept and its end trimmed;
+    // `////` is a plain comment.
+    let source = "/// doc  \n//// not a doc\n///x\r\nfn"
+    let expected = dump_lines(
+        [
+            "0 9 DocComment \" doc\"",
+            "25 30 DocComment \"x\" NL",
+            "31 33 Fn NL",
+            "33 33 Eof",
+            "COMMENT 10 24 Line",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn keywords_are_whole_identifiers() {
+    // A keyword is a whole match: `lets` and `self_` are identifiers.
+    let source = "let lets Self self_ _ r rx fn fnx continue where with"
+    let expected = dump_lines(
+        [
+            "0 3 Let",
+            "4 8 Ident \"lets\"",
+            "9 13 SelfUpper",
+            "14 19 Ident \"self_\"",
+            "20 21 Ident \"_\"",
+            "22 23 Ident \"r\"",
+            "24 26 Ident \"rx\"",
+            "27 29 Fn",
+            "30 33 Ident \"fnx\"",
+            "34 42 Continue",
+            "43 48 Where",
+            "49 53 With",
+            "53 53 Eof",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn unexpected_characters_and_an_open_block_comment() {
+    // Non-ASCII text outside strings, and a `/*` with no `*/`.
+    let source = "a é € # ` /* open"
+    let expected = dump_lines(
+        [
+            "0 1 Ident \"a\"",
+            "20 20 Eof",
+            "ERR 2 4 unexpected character `é`",
+            "ERR 5 8 unexpected character `€`",
+            "ERR 9 10 unexpected character `#`",
+            "ERR 11 12 unexpected character ```",
+            "ERR 13 20 unterminated block comment",
+        ],
+    )
+    assert_eq(dump_tokens(source), expected)
+}
+
+@test
+fn descriptions_are_the_rust_ones() {
+    // token.rs's `description`, word for word: keywords, punctuation, a
+    // string's quote, a hole's opening, literals and the end of the file.
+    assert_eq(Let.description(), "`let`")
+    assert_eq(Continue.description(), "`continue`")
+    assert_eq(FatArrow.description(), "`=>`")
+    assert_eq(LtLtEq.description(), "`<<=`")
+    assert_eq(StrStart.description(), "`\"`")
+    assert_eq(InterpOpen.description(), "`$" + "{`")
+    assert_eq(InterpClose.description(), "`}`")
+    assert_eq(Int(7).description(), "integer literal")
+    assert_eq(Float(2.5).description(), "float literal")
+    assert_eq(StrPart("a").description(), "string segment")
+    assert_eq(RawStr("a").description(), "raw string literal")
+    assert_eq(Ident("a").description(), "identifier")
+    assert_eq(Eof.description(), "end of file")
+}
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/.gitignore b/.gitignore
index 4a7a5870952cc91b46903a7b6b7c424559fce292..449cf0d956ca7921f9394087256f632e6752a244 100644
--- a/.gitignore
+++ b/.gitignore
@@ -13,6 +13,7 @@
 /examples/07-fullstack-blog/data/users.json
 /examples/07-fullstack-blog/data/users.json.tmp
 /examples/07-fullstack-blog/data/secret
+/selfhost/target
 /tools/vscode-nova/node_modules
 /tools/vscode-nova/out
 /tools/vscode-nova/.vscode-test
diff --git a/selfhost/nova.toml b/selfhost/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..a9fd6812f50a45038b3973ea1430f54e76818fa9
--- /dev/null
+++ b/selfhost/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "selfhost"
+version = "0.1.0"
+edition = "2026"
diff --git a/selfhost/src/dump.nova b/selfhost/src/dump.nova
new file mode 100644
index 0000000000000000000000000000000000000000..57e1f35c4138b94b605c99bfc313ba64f3532120
--- /dev/null
+++ b/selfhost/src/dump.nova
@@ -0,0 +1,202 @@
+// The token dump: one source's tokens, lex errors and comments as text, one
+// item per line, exactly as S0's `nova parse --dump-tokens` prints them
+// (crates/nova-lexer/src/dump.rs; spec §3.3 and ADR 0051 give the format).
+//
+// Per source:
+// - one line per token, `Eof` last: `start end Kind[ payload][ NL][ PD]`;
+// - one line per lex error: `ERR start end message`;
+// - one line per comment: `COMMENT start end Line|Block`.
+
+import lexer
+import token
+
+/// The token dump of `source` (dump.rs's `dump_tokens`).
+pub fn dump_tokens(source: String) -> String {
+    let (tokens, comments, errors) = lex_with_comments(source)
+    let mut out = StringBuilder::new()
+    let mut i = 0
+    while i < tokens.len() {
+        let t = tokens.at(i)
+        out.push("${t.span.start} ${t.span.end} ")
+        push_kind_and_payload(out, t.value)
+        if tokens.line_break_before(i) { out.push(" NL") }
+        if tokens.plain_decimal(i) { out.push(" PD") }
+        out.push("\n")
+        i += 1
+    }
+    for e in errors.iter() {
+        let span = e.span()
+        out.push("ERR ${span.start} ${span.end} ")
+        push_escaped(out, "${e}", false)
+        out.push("\n")
+    }
+    for c in comments.iter() {
+        let kind = match c.kind {
+            Line => "Line"
+            Block => "Block"
+        }
+        out.push("COMMENT ${c.span.start} ${c.span.end} ${kind}\n")
+    }
+    out.to_string()
+}
+
+// The token's kind, the Rust variant's name, and its payload: `Int` in
+// decimal, `Float` as the 16 hex digits of its bits, `Char` as its code
+// point, and text in double quotes, escaped.
+fn push_kind_and_payload(mut out: StringBuilder, t: Token) {
+    match t {
+        Int(n) => out.push("Int ${n}")
+        Float(f) => {
+            out.push("Float ")
+            out.push(hex16(f.to_bits()))
+        }
+        Char(c) => out.push("Char ${c.to_int()}")
+        StrPart(s) => push_quoted(out, "StrPart", s)
+        RawStr(s) => push_quoted(out, "RawStr", s)
+        Ident(s) => push_quoted(out, "Ident", s)
+        DocComment(s) => push_quoted(out, "DocComment", s)
+        _ => out.push(kind_name(t))
+    }
+}
+
+fn push_quoted(mut out: StringBuilder, kind: String, text: String) {
+    out.push(kind)
+    out.push(" \"")
+    push_escaped(out, text, true)
+    out.push("\"")
+}
+
+// `text`, with each byte below 0x20 and 0x7F written `\xHH`, two lowercase
+// hex digits, and, when `quoted`, `"` and `\` too. Every other byte is kept,
+// so a non-ASCII character prints as it is. The bytes escaped are all ASCII,
+// so the runs between them are whole characters.
+fn push_escaped(mut out: StringBuilder, text: String, quoted: Bool) {
+    let n = text.byte_len()
+    let mut run = 0
+    let mut i = 0
+    while i < n {
+        let b = text.byte_at(i)
+        if b < 0x20 || b == 0x7F || (quoted && (b == 34 || b == 92)) {
+            if i > run { out.push(text.byte_slice(run, i)) }
+            out.push("\\x")
+            out.push(hex_digit(b >> 4))
+            out.push(hex_digit(b & 15))
+            run = i + 1
+        }
+        i += 1
+    }
+    if n > run { out.push(text.byte_slice(run, n)) }
+}
+
+// The 64 bits of `bits` as 16 lowercase hex digits, the sign bit first.
+fn hex16(bits: Int) -> String {
+    let mut out = StringBuilder::new()
+    let mut shift = 60
+    while shift >= 0 {
+        out.push(hex_digit((bits >> shift) & 15))
+        shift -= 4
+    }
+    out.to_string()
+}
+
+fn hex_digit(d: Int) -> String {
+    if d < 10 { "${d}" } else { string_from_chars([char_from_int(87 + d).unwrap()]) }
+}
+
+// The name of a token kind with no payload: its Rust variant's name.
+fn kind_name(t: Token) -> String {
+    match t {
+        Let => "Let"
+        Mut => "Mut"
+        Const => "Const"
+        Fn => "Fn"
+        Return => "Return"
+        If => "If"
+        Else => "Else"
+        While => "While"
+        For => "For"
+        In => "In"
+        Break => "Break"
+        Continue => "Continue"
+        Match => "Match"
+        Type => "Type"
+        Record => "Record"
+        Trait => "Trait"
+        Impl => "Impl"
+        Import => "Import"
+        Module => "Module"
+        Pub => "Pub"
+        Async => "Async"
+        Await => "Await"
+        Extern => "Extern"
+        Unsafe => "Unsafe"
+        True => "True"
+        False => "False"
+        As => "As"
+        Is => "Is"
+        Where => "Where"
+        With => "With"
+        SelfLower => "SelfLower"
+        SelfUpper => "SelfUpper"
+        LParen => "LParen"
+        RParen => "RParen"
+        LBracket => "LBracket"
+        RBracket => "RBracket"
+        LBrace => "LBrace"
+        RBrace => "RBrace"
+        Comma => "Comma"
+        Semicolon => "Semicolon"
+        Colon => "Colon"
+        ColonColon => "ColonColon"
+        Arrow => "Arrow"
+        FatArrow => "FatArrow"
+        Dot => "Dot"
+        DotDot => "DotDot"
+        DotDotEq => "DotDotEq"
+        At => "At"
+        Question => "Question"
+        Eq => "Eq"
+        EqEq => "EqEq"
+        BangEq => "BangEq"
+        Lt => "Lt"
+        LtEq => "LtEq"
+        Gt => "Gt"
+        GtEq => "GtEq"
+        AmpAmp => "AmpAmp"
+        PipePipe => "PipePipe"
+        Bang => "Bang"
+        Plus => "Plus"
+        Minus => "Minus"
+        Star => "Star"
+        Slash => "Slash"
+        Percent => "Percent"
+        PlusEq => "PlusEq"
+        MinusEq => "MinusEq"
+        StarEq => "StarEq"
+        SlashEq => "SlashEq"
+        PercentEq => "PercentEq"
+        Amp => "Amp"
+        Pipe => "Pipe"
+        Caret => "Caret"
+        Tilde => "Tilde"
+        LtLt => "LtLt"
+        GtGt => "GtGt"
+        AmpEq => "AmpEq"
+        PipeEq => "PipeEq"
+        CaretEq => "CaretEq"
+        LtLtEq => "LtLtEq"
+        GtGtEq => "GtGtEq"
+        StrStart => "StrStart"
+        StrEnd => "StrEnd"
+        InterpOpen => "InterpOpen"
+        InterpClose => "InterpClose"
+        Eof => "Eof"
+        Int(_) => "Int"
+        Float(_) => "Float"
+        Char(_) => "Char"
+        StrPart(_) => "StrPart"
+        RawStr(_) => "RawStr"
+        Ident(_) => "Ident"
+        DocComment(_) => "DocComment"
+    }
+}
diff --git a/selfhost/src/lexer.nova b/selfhost/src/lexer.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a41bc7563cbad7622869bf12549cd1cee7ef5fba
--- /dev/null
+++ b/selfhost/src/lexer.nova
@@ -0,0 +1,1048 @@
+// The Nova lexer: a port of crates/nova-lexer/src/lib.rs, tokens.rs and
+// error.rs, function by function, so that it gives the same tokens, spans,
+// line-break marks, errors and comments as the Rust lexer on every input
+// (spec
+// docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md
+// §3, ADR 0051). Each function names the Rust it ports. A change to the
+// language's tokens changes both lexers in one pull request; the
+// differential test in crates/nova-cli/tests/selfhost.rs holds them equal.
+//
+// The Rust lexer hands most tokens to `logos` 0.14. Here `lex_logos` and
+// the functions after it are that recognizer written by hand, with logos's
+// rules: the longest match wins, a keyword beats an identifier of the same
+// text, and a match that no regex makes, or whose callback refuses it (an
+// integer that overflows), is `UnexpectedCharacter` of its first character,
+// after which lexing moves on one character.
+//
+// The source is read a byte at a time with `String.byte_at`, and every span
+// is a byte offset, as in Rust.
+
+import token
+
+// The bytes the lexer tests for, by name.
+const TAB: Int = 9
+
+const NEWLINE: Int = 10
+
+const CR: Int = 13
+
+const SPACE: Int = 32
+
+const QUOTE: Int = 34
+
+const HASH: Int = 35
+
+const DOLLAR: Int = 36
+
+const APOSTROPHE: Int = 39
+
+const STAR: Int = 42
+
+const PLUS: Int = 43
+
+const MINUS: Int = 45
+
+const DOT: Int = 46
+
+const SLASH: Int = 47
+
+const ZERO: Int = 48
+
+const NINE: Int = 57
+
+const UPPER_E: Int = 69
+
+const BACKSLASH: Int = 92
+
+const UNDERSCORE: Int = 95
+
+const LOWER_E: Int = 101
+
+const LOWER_R: Int = 114
+
+const LBRACE: Int = 123
+
+const RBRACE: Int = 125
+
+// ---------------------------------------------------------------------------
+// error.rs
+// ---------------------------------------------------------------------------
+
+/// A non-fatal error the lexer reports (error.rs's `LexError`). Lexing goes
+/// on after each one.
+pub type LexError =
+    | UnterminatedString(Span)
+    | UnterminatedInterpolation(Span)
+    | UnterminatedBlockComment(Span)
+    | InvalidEscapeSequence(Char, Span)
+    | InvalidUnicodeEscape(Span)
+    | InvalidNumberLiteral(Span)
+    | UnexpectedCharacter(Char, Span)
+
+impl LexError {
+    /// Where the error is (error.rs's `LexError::span`).
+    pub fn span(self) -> Span {
+        match self {
+            UnterminatedString(s) => s
+            UnterminatedInterpolation(s) => s
+            UnterminatedBlockComment(s) => s
+            InvalidEscapeSequence(_, s) => s
+            InvalidUnicodeEscape(s) => s
+            InvalidNumberLiteral(s) => s
+            UnexpectedCharacter(_, s) => s
+        }
+    }
+}
+
+// The `#[error(…)]` texts, word for word.
+impl Display for LexError {
+    fn fmt(self) -> String {
+        match self {
+            UnterminatedString(_) => "unterminated string literal"
+            UnterminatedInterpolation(_) => {
+                "unterminated string interpolation: this `$" + "{` has no matching `}`"
+            }
+            UnterminatedBlockComment(_) => "unterminated block comment"
+            InvalidEscapeSequence(c, _) => "invalid escape sequence `\\${c}`"
+            InvalidUnicodeEscape(_) => "invalid unicode escape sequence"
+            InvalidNumberLiteral(_) => "invalid numeric literal"
+            UnexpectedCharacter(c, _) => "unexpected character `${c}`"
+        }
+    }
+}
+
+// ---------------------------------------------------------------------------
+// lib.rs: comments, and the lexer's entry points
+// ---------------------------------------------------------------------------
+
+/// Which kind of comment (lib.rs's `CommentKind`).
+pub type CommentKind =
+    /// `// …`, including `//// …`.
+    | Line
+    /// `/* … */`, which does not nest.
+    | Block
+
+/// A comment the lexer skipped (lib.rs's `Comment`): from `//` to the end
+/// of its line, without the line break or a `\r` before it; or from `/*`
+/// through `*/`.
+pub record Comment { pub kind: CommentKind, pub span: Span }
+
+/// The tokens of `source` and the errors found (lib.rs's `lex`).
+pub fn lex(source: String) -> (Tokens, Vec<LexError>) {
+    let mut lexer = new_lexer(source)
+    let (tokens, errors) = lexer.tokenize()
+    (marked(tokens, source), errors)
+}
+
+/// `lex`, plus every comment skipped, in source order (lib.rs's
+/// `lex_with_comments`). The tokens and errors are exactly `lex`'s.
+pub fn lex_with_comments(source: String) -> (Tokens, Vec<Comment>, Vec<LexError>) {
+    let mut lexer = new_lexer(source)
+    let (tokens, errors) = lexer.tokenize()
+    (marked(tokens, source), lexer.comments, errors)
+}
+
+// ---------------------------------------------------------------------------
+// tokens.rs
+// ---------------------------------------------------------------------------
+
+/// The tokens of one source, ending with `Eof`, each marked with whether a
+/// line break comes before it, and whether it is an integer in plain decimal
+/// digits (tokens.rs's `Tokens`).
+pub record Tokens { pub tokens: Vec<Spanned>, breaks: Vec<Bool>, plain: Vec<Bool> }
+
+/// `tokens`, lexed from `source`, each marked by the source text between it
+/// and the token before it, or the start of the source (tokens.rs's
+/// `Tokens::marked`). A line break is a `\n` or a `\r`, and one inside a
+/// comment counts.
+pub fn marked(tokens: Vec<Spanned>, source: String) -> Tokens {
+    let n = source.byte_len()
+    let mut prev_end = 0
+    let mut breaks: Vec<Bool> = Vec::new()
+    let mut plain: Vec<Bool> = Vec::new()
+    for t in tokens.iter() {
+        let start = min(t.span.start, n)
+        let mut i = min(prev_end, start)
+        let mut line_break = false
+        while i < start {
+            let c = source.byte_at(i)
+            if c == NEWLINE || c == CR {
+                line_break = true
+                break
+            }
+            i += 1
+        }
+        prev_end = max(prev_end, t.span.end)
+        breaks.push(line_break)
+        plain.push(is_plain_decimal(t, source))
+    }
+    Tokens { tokens: tokens, breaks: breaks, plain: plain }
+}
+
+// An `Int` whose text is ASCII digits with no leading zero, unless it is
+// exactly `0`.
+fn is_plain_decimal(t: Spanned, source: String) -> Bool {
+    let is_int = match t.value {
+        Int(_) => true
+        _ => false
+    }
+    if !is_int { return false }
+    let start = t.span.start
+    let end = t.span.end
+    if end <= start { return false }
+    let mut i = start
+    while i < end {
+        if !is_digit(source.byte_at(i)) { return false }
+        i += 1
+    }
+    end - start == 1 || source.byte_at(start) != ZERO
+}
+
+impl Tokens {
+    /// How many tokens, `Eof` included.
+    pub fn len(self) -> Int { self.tokens.len() }
+
+    /// The token at `index`, which must be less than `len()`.
+    pub fn at(self, index: Int) -> Spanned { self.tokens.at(index) }
+
+    /// Whether a line break comes before the token at `index`; `false` past
+    /// the end (tokens.rs's `line_break_before`).
+    pub fn line_break_before(self, index: Int) -> Bool {
+        if index < 0 || index >= self.breaks.len() { return false }
+        self.breaks.at(index)
+    }
+
+    /// Whether the token at `index` is an integer in plain decimal digits
+    /// with no leading zero; `false` past the end (tokens.rs's
+    /// `plain_decimal`).
+    pub fn plain_decimal(self, index: Int) -> Bool {
+        if index < 0 || index >= self.plain.len() { return false }
+        self.plain.at(index)
+    }
+}
+
+// ---------------------------------------------------------------------------
+// lib.rs: the lexer
+// ---------------------------------------------------------------------------
+
+// One open `${…}` hole (lib.rs's `InterpFrame`): the offset of its `$`, for
+// the unterminated-hole error, and how many `{` opened inside it are still
+// unmatched. A `}` ends the hole only when that count is zero.
+record InterpFrame { open: Int, braces: Int }
+
+/// The lexer's state (lib.rs's `Lexer`).
+pub record Lexer {
+    source: String,
+    // The source's length in bytes.
+    len: Int,
+    // The current byte offset.
+    pos: Int,
+    // The open `${…}` holes, innermost last.
+    interp: Vec<InterpFrame>,
+    // Whether the lexer is between a string's delimiters.
+    in_string: Bool,
+    // Every comment skipped so far, in source order.
+    comments: Vec<Comment>,
+}
+
+/// A lexer at the start of `source` (lib.rs's `Lexer::new`).
+pub fn new_lexer(source: String) -> Lexer {
+    Lexer {
+        source: source,
+        len: source.byte_len(),
+        pos: 0,
+        interp: Vec::new(),
+        in_string: false,
+        comments: Vec::new(),
+    }
+}
+
+impl Lexer {
+    /// Every token, `Eof` last, and every error (lib.rs's `tokenize`).
+    pub fn tokenize(mut self) -> (Vec<Spanned>, Vec<LexError>) {
+        let mut tokens: Vec<Spanned> = Vec::new()
+        let mut errors: Vec<LexError> = Vec::new()
+        while self.pos < self.len {
+            match self.next_token() {
+                Ok(Some(tok)) => tokens.push(tok)
+                Ok(None) => ()
+                Err(e) => errors.push(e)
+            }
+        }
+        // A hole still open at the end never found its `}`: report the
+        // innermost one.
+        if !self.interp.is_empty() {
+            let frame = self.interp.last()
+            let end = min(frame.open + 2, self.len)
+            errors.push(UnterminatedInterpolation(Span { start: frame.open, end: end }))
+        }
+        let mut out = self.split_tuple_indexes(tokens)
+        out.push(spanned(Eof, self.len, self.len))
+        (out, errors)
+    }
+
+    // After a `.`, a float whose text is digits, `.`, digits, both parts
+    // fitting an `Int`, becomes an integer, a `.` and an integer, so `t.1.0`
+    // reads as `(t.1).0` (lib.rs's `split_tuple_indexes`). It splits by the
+    // text, not the value.
+    fn split_tuple_indexes(self, tokens: Vec<Spanned>) -> Vec<Spanned> {
+        let mut out: Vec<Spanned> = Vec::new()
+        for tok in tokens.iter() {
+            let after_dot = !out.is_empty() && is_dot(out.last().value)
+            if after_dot && is_float(tok.value) {
+                let start = tok.span.start
+                let end = tok.span.end
+                let dot = self.find_dot(start, end)
+                if dot >= 0 && self.all_digits(start, dot) && self.all_digits(dot + 1, end) {
+                    let int_part = self.source.byte_slice(start, dot).parse_int()
+                    let frac_part = self.source.byte_slice(dot + 1, end).parse_int()
+                    match (int_part, frac_part) {
+                        (Some(a), Some(b)) => {
+                            out.push(spanned(Int(a), start, dot))
+                            out.push(spanned(Dot, dot, dot + 1))
+                            out.push(spanned(Int(b), dot + 1, end))
+                            continue
+                        }
+                        _ => ()
+                    }
+                }
+            }
+            out.push(tok)
+        }
+        out
+    }
+
+    // The offset of the first `.` in `start..end`, or -1 (`split_once('.')`).
+    fn find_dot(self, start: Int, end: Int) -> Int {
+        let mut i = start
+        while i < end {
+            if self.source.byte_at(i) == DOT { return i }
+            i += 1
+        }
+        0 - 1
+    }
+
+    // Whether `start..end` is one or more ASCII digits.
+    fn all_digits(self, start: Int, end: Int) -> Bool {
+        if end <= start { return false }
+        let mut i = start
+        while i < end {
+            if !is_digit(self.source.byte_at(i)) { return false }
+            i += 1
+        }
+        true
+    }
+
+    // The byte at `i`, or -1 past the end (`b.get(i)`).
+    fn byte_or_none(self, i: Int) -> Int {
+        if i < self.len { self.source.byte_at(i) } else { 0 - 1 }
+    }
+
+    // Advance past whitespace, `//` line comments and `/* */` block
+    // comments, recording each comment (lib.rs's `skip_trivia`). A `///` doc
+    // comment, exactly three slashes, is left for the tokenizer; four or
+    // more make a plain comment. A `/*` with no `*/` is an error.
+    fn skip_trivia(mut self) -> Result<(), LexError> {
+        let b = self.source
+        let n = self.len
+        while true {
+            while self.pos < n {
+                let c = b.byte_at(self.pos)
+                if c == SPACE || c == TAB || c == NEWLINE || c == CR {
+                    self.pos += 1
+                } else {
+                    break
+                }
+            }
+            if self.pos + 1 < n
+                && b.byte_at(self.pos) == SLASH
+                && b.byte_at(self.pos + 1) == SLASH {
+                let doc = self.byte_or_none(self.pos + 2) == SLASH
+                    && self.byte_or_none(self.pos + 3) != SLASH
+                if doc { break }
+                let start = self.pos
+                while self.pos < n && b.byte_at(self.pos) != NEWLINE { self.pos += 1 }
+                let end = if self.pos > start && b.byte_at(self.pos - 1) == CR {
+                    self.pos - 1
+                } else {
+                    self.pos
+                }
+                self.comments.push(Comment { kind: Line, span: Span { start: start, end: end } })
+                continue
+            }
+            if self.pos + 1 < n && b.byte_at(self.pos) == SLASH && b.byte_at(self.pos + 1) == STAR {
+                let start = self.pos
+                self.pos += 2
+                while self.pos + 1 < n
+                    && !(b.byte_at(self.pos) == STAR && b.byte_at(self.pos + 1) == SLASH) {
+                    self.pos += 1
+                }
+                if self.pos + 1 >= n {
+                    self.pos = n
+                    return Err(UnterminatedBlockComment(Span { start: start, end: n }))
+                }
+                self.pos += 2
+                self.comments.push(
+                    Comment { kind: Block, span: Span { start: start, end: self.pos } },
+                )
+                continue
+            }
+            break
+        }
+        Ok(())
+    }
+
+    // The next token, nothing, or an error (lib.rs's `next_token`).
+    fn next_token(mut self) -> Result<Option<Spanned>, LexError> {
+        // Inside a string, whitespace is text: lex the content before any
+        // skipping.
+        if self.in_string {
+            if self.pos >= self.len { return Ok(None) }
+            return self.lex_string_content()
+        }
+        self.skip_trivia()?
+        if self.pos >= self.len { return Ok(None) }
+        let b = self.source
+        if b.byte_at(self.pos) == QUOTE { return self.lex_string_start() }
+        if self.pos + 1 < self.len && b.byte_at(self.pos) == LOWER_R {
+            let next = b.byte_at(self.pos + 1)
+            if next == QUOTE || next == HASH { return self.lex_raw_string() }
+        }
+        self.lex_logos()
+    }
+
+    // lib.rs's `lex_string_start`.
+    fn lex_string_start(mut self) -> Result<Option<Spanned>, LexError> {
+        let start = self.pos
+        self.pos += 1
+        self.in_string = true
+        Ok(Some(spanned(StrStart, start, self.pos)))
+    }
+
+    // The string's end, an interpolation's `${`, or a segment of text with
+    // its escapes decoded (lib.rs's `lex_string_content`). Runs of plain text
+    // are copied a run at a time rather than a character at a time, which
+    // gives the same text.
+    fn lex_string_content(mut self) -> Result<Option<Spanned>, LexError> {
+        let src = self.source
+        let n = self.len
+        let start = self.pos
+        if src.byte_at(self.pos) == QUOTE {
+            self.pos += 1
+            self.in_string = false
+            return Ok(Some(spanned(StrEnd, start, self.pos)))
+        }
+        if self.pos + 1 < n
+            && src.byte_at(self.pos) == DOLLAR
+            && src.byte_at(self.pos + 1) == LBRACE {
+            self.pos += 2
+            self.in_string = false
+            self.interp.push(InterpFrame { open: start, braces: 0 })
+            return Ok(Some(spanned(InterpOpen, start, self.pos)))
+        }
+        let mut buf = StringBuilder::new()
+        let mut run = self.pos
+        while self.pos < n && src.byte_at(self.pos) != QUOTE {
+            if self.pos + 1 < n
+                && src.byte_at(self.pos) == DOLLAR
+                && src.byte_at(self.pos + 1) == LBRACE {
+                break
+            }
+            if src.byte_at(self.pos) == BACKSLASH {
+                if self.pos > run { buf.push(src.byte_slice(run, self.pos)) }
+                self.pos += 1
+                if self.pos >= n {
+                    return Err(UnterminatedString(Span { start: start, end: self.pos }))
+                }
+                let c = src.byte_at(self.pos)
+                if c == 110 {
+                    // `\n`
+                    buf.push_char('\n')
+                    self.pos += 1
+                } else if c == 116 {
+                    // `\t`
+                    buf.push_char('\t')
+                    self.pos += 1
+                } else if c == 114 {
+                    // `\r`
+                    buf.push_char('\r')
+                    self.pos += 1
+                } else if c == QUOTE {
+                    buf.push_char('"')
+                    self.pos += 1
+                } else if c == BACKSLASH {
+                    buf.push_char('\\')
+                    self.pos += 1
+                } else if c == ZERO {
+                    buf.push_char('\0')
+                    self.pos += 1
+                } else if c == 117 {
+                    // A Unicode escape: the hex runs to the next `}`, wherever it is.
+                    self.pos += 1
+                    if self.pos < n && src.byte_at(self.pos) == LBRACE {
+                        self.pos += 1
+                        let hex_start = self.pos
+                        while self.pos < n && src.byte_at(self.pos) != RBRACE { self.pos += 1 }
+                        let hex_end = self.pos
+                        if self.pos < n { self.pos += 1 }
+                        match char_from_int(self.hex_u32(hex_start, hex_end)) {
+                            Some(ch) => buf.push_char(ch)
+                            None => return Err(
+                                InvalidUnicodeEscape(Span { start: start, end: self.pos }),
+                            )
+                        }
+                    } else {
+                        return Err(InvalidEscapeSequence('u', Span { start: start, end: self.pos }))
+                    }
+                } else {
+                    // Rust's `c as char` of the byte, even one that starts
+                    // a longer character.
+                    return Err(
+                        InvalidEscapeSequence(
+                            char_from_int(c).unwrap(),
+                            Span { start: start, end: self.pos },
+                        ),
+                    )
+                }
+                run = self.pos
+            } else {
+                self.pos += utf8_char_width(src.byte_at(self.pos))
+            }
+        }
+        if self.pos > run { buf.push(src.byte_slice(run, self.pos)) }
+        if self.pos >= n && !buf.is_empty() {
+            return Err(UnterminatedString(Span { start: start, end: self.pos }))
+        }
+        Ok(Some(spanned(StrPart(buf.to_string()), start, self.pos)))
+    }
+
+    // `u32::from_str_radix(hex, 16)`: the value of the text `start..end`,
+    // an optional `+` then one or more hex digits; or -1 if it is anything
+    // else or past `u32::MAX`. `char_from_int` then refuses what
+    // `char::from_u32` refuses.
+    fn hex_u32(self, start: Int, end: Int) -> Int {
+        let mut i = start
+        if i < end && self.source.byte_at(i) == PLUS { i += 1 }
+        if i >= end { return 0 - 1 }
+        let mut v = 0
+        while i < end {
+            let d = digit_value(self.source.byte_at(i))
+            if d >= 16 { return 0 - 1 }
+            v = v * 16 + d
+            if v > 0xFFFFFFFF { return 0 - 1 }
+            i += 1
+        }
+        v
+    }
+
+    // `r"…"` and `r#"…"#`, with any number of `#`s (lib.rs's
+    // `lex_raw_string`). An `r#` with no `"` after its `#`s is the one
+    // source of `InvalidNumberLiteral`.
+    fn lex_raw_string(mut self) -> Result<Option<Spanned>, LexError> {
+        let src = self.source
+        let n = self.len
+        let start = self.pos
+        self.pos += 1
+        let mut hash_count = 0
+        while self.pos < n && src.byte_at(self.pos) == HASH {
+            hash_count += 1
+            self.pos += 1
+        }
+        if self.pos >= n || src.byte_at(self.pos) != QUOTE {
+            return Err(InvalidNumberLiteral(Span { start: start, end: self.pos }))
+        }
+        self.pos += 1
+        let content_start = self.pos
+        while true {
+            if self.pos >= n {
+                return Err(UnterminatedString(Span { start: start, end: self.pos }))
+            }
+            if src.byte_at(self.pos) == QUOTE {
+                // A `"` followed by as many `#`s as opened the string.
+                let mut end_hashes = 0
+                let mut look = self.pos + 1
+                while look < n && src.byte_at(look) == HASH && end_hashes < hash_count {
+                    end_hashes += 1
+                    look += 1
+                }
+                if end_hashes == hash_count {
+                    let content = src.byte_slice(content_start, self.pos)
+                    self.pos = look
+                    return Ok(Some(spanned(RawStr(content), start, self.pos)))
+                }
+            }
+            self.pos += utf8_char_width(src.byte_at(self.pos))
+        }
+        Ok(None)
+    }
+
+    // A `}` closes the innermost hole only when that hole has no unmatched
+    // `{` of its own; otherwise it is an ordinary `RBrace` (lib.rs's
+    // `maybe_close_interp`).
+    fn maybe_close_interp(mut self, start: Int) -> Spanned {
+        if self.interp.is_empty() { return spanned(RBrace, start, self.pos) }
+        let mut frame = self.interp.last()
+        if frame.braces > 0 {
+            frame.braces -= 1
+            return spanned(RBrace, start, self.pos)
+        }
+        let _ = self.interp.pop()
+        self.in_string = true
+        spanned(InterpClose, start, self.pos)
+    }
+
+    // What logos recognizes, at `pos` (lib.rs's `lex_logos`). Braces are
+    // the one pair that interacts with interpolation: a `{` opened inside a
+    // hole must be matched before the hole's own `}`.
+    fn lex_logos(mut self) -> Result<Option<Spanned>, LexError> {
+        let start = self.pos
+        match self.raw_token(start) {
+            Some((tok, end)) => {
+                self.pos = end
+                match tok {
+                    LBrace => {
+                        if !self.interp.is_empty() {
+                            let mut frame = self.interp.last()
+                            frame.braces += 1
+                        }
+                    }
+                    RBrace => return Ok(Some(self.maybe_close_interp(start)))
+                    _ => ()
+                }
+                Ok(Some(spanned(tok, start, end)))
+            }
+            None => {
+                // logos's error: the span is the one character at `start`.
+                let w = utf8_char_width(self.source.byte_at(start))
+                let ch = self.char_at(start, w)
+                self.pos = start + w
+                Err(UnexpectedCharacter(ch, Span { start: start, end: self.pos }))
+            }
+        }
+    }
+
+    // logos's longest match at `start`, after its callback: the token and
+    // its end, or `None` where logos reports an error (lib.rs's `RawToken`,
+    // and `raw_to_token`).
+    fn raw_token(self, start: Int) -> Option<(Token, Int)> {
+        let b = self.source
+        let c = b.byte_at(start)
+        if is_ident_start(c) {
+            // `[a-zA-Z_][a-zA-Z0-9_]*`, unless the whole match is a keyword.
+            let mut i = start + 1
+            while i < self.len && is_ident_continue(b.byte_at(i)) { i += 1 }
+            return Some((keyword_or_ident(b.byte_slice(start, i), c, i - start), i))
+        }
+        if is_digit(c) { return self.lex_number(start) }
+        if c == APOSTROPHE { return self.lex_char(start) }
+        if c == SLASH
+            && self.byte_or_none(start + 1) == SLASH
+            && self.byte_or_none(start + 2) == SLASH {
+            // `///[^\n]*`. Leading whitespace kept, so indentation inside a
+            // doc's Markdown survives; trailing whitespace, `\r` included,
+            // dropped (`trim_end`).
+            let mut i = start + 3
+            while i < self.len && b.byte_at(i) != NEWLINE { i += 1 }
+            let text_end = self.trim_end(start + 3, i)
+            return Some((DocComment(b.byte_slice(start + 3, text_end)), i))
+        }
+        self.punct(start)
+    }
+
+    // The character of width `w` at `i`.
+    fn char_at(self, i: Int, w: Int) -> Char {
+        let b = self.source
+        let cp = if w == 1 {
+            b.byte_at(i)
+        } else if w == 2 {
+            ((b.byte_at(i) & 31) << 6) | (b.byte_at(i + 1) & 63)
+        } else if w == 3 {
+            ((b.byte_at(i) & 15) << 12) | ((b.byte_at(i + 1) & 63) << 6) | (b.byte_at(i + 2) & 63)
+        } else {
+            ((b.byte_at(i) & 7) << 18)
+                | ((b.byte_at(i + 1) & 63) << 12)
+                | ((b.byte_at(i + 2) & 63) << 6)
+                | (b.byte_at(i + 3) & 63)
+        }
+        char_from_int(cp).unwrap()
+    }
+
+    // The end of `start..end` once trailing whitespace is dropped: Rust's
+    // `str::trim_end`, which drops every character with Unicode's
+    // White_Space property.
+    fn trim_end(self, start: Int, end: Int) -> Int {
+        let mut e = end
+        while e > start {
+            // Step back over one UTF-8 character.
+            let mut s = e - 1
+            while s > start && (self.source.byte_at(s) & 0xC0) == 0x80 { s -= 1 }
+            if !is_white_space(self.char_at(s, e - s).to_int()) { return e }
+            e = s
+        }
+        e
+    }
+
+    // Punctuation, by longest match (the `#[token(…)]` variants).
+    fn punct(self, start: Int) -> Option<(Token, Int)> {
+        let c = self.source.byte_at(start)
+        let c1 = self.byte_or_none(start + 1)
+        let c2 = self.byte_or_none(start + 2)
+        // `(` `)` `[` `]` `{` `}` `,` `;` `@` `?` `~`
+        if c == 40 { return Some((LParen, start + 1)) }
+        if c == 41 { return Some((RParen, start + 1)) }
+        if c == 91 { return Some((LBracket, start + 1)) }
+        if c == 93 { return Some((RBracket, start + 1)) }
+        if c == LBRACE { return Some((LBrace, start + 1)) }
+        if c == RBRACE { return Some((RBrace, start + 1)) }
+        if c == 44 { return Some((Comma, start + 1)) }
+        if c == 59 { return Some((Semicolon, start + 1)) }
+        if c == 64 { return Some((At, start + 1)) }
+        if c == 63 { return Some((Question, start + 1)) }
+        if c == 126 { return Some((Tilde, start + 1)) }
+        if c == 58 {
+            // `:` `::`
+            if c1 == 58 { return Some((ColonColon, start + 2)) }
+            return Some((Colon, start + 1))
+        }
+        if c == MINUS {
+            // `-` `->` `-=`
+            if c1 == 62 { return Some((Arrow, start + 2)) }
+            if c1 == 61 { return Some((MinusEq, start + 2)) }
+            return Some((Minus, start + 1))
+        }
+        if c == 61 {
+            // `=` `=>` `==`
+            if c1 == 62 { return Some((FatArrow, start + 2)) }
+            if c1 == 61 { return Some((EqEq, start + 2)) }
+            return Some((Eq, start + 1))
+        }
+        if c == DOT {
+            // `.` `..` `..=`
+            if c1 == DOT {
+                if c2 == 61 { return Some((DotDotEq, start + 3)) }
+                return Some((DotDot, start + 2))
+            }
+            return Some((Dot, start + 1))
+        }
+        if c == 33 {
+            // `!` `!=`
+            if c1 == 61 { return Some((BangEq, start + 2)) }
+            return Some((Bang, start + 1))
+        }
+        if c == 60 {
+            // `<` `<=` `<<` `<<=`
+            if c1 == 61 { return Some((LtEq, start + 2)) }
+            if c1 == 60 {
+                if c2 == 61 { return Some((LtLtEq, start + 3)) }
+                return Some((LtLt, start + 2))
+            }
+            return Some((Lt, start + 1))
+        }
+        if c == 62 {
+            // `>` `>=` `>>` `>>=`
+            if c1 == 61 { return Some((GtEq, start + 2)) }
+            if c1 == 62 {
+                if c2 == 61 { return Some((GtGtEq, start + 3)) }
+                return Some((GtGt, start + 2))
+            }
+            return Some((Gt, start + 1))
+        }
+        if c == 38 {
+            // `&` `&&` `&=`
+            if c1 == 38 { return Some((AmpAmp, start + 2)) }
+            if c1 == 61 { return Some((AmpEq, start + 2)) }
+            return Some((Amp, start + 1))
+        }
+        if c == 124 {
+            // `|` `||` `|=`
+            if c1 == 124 { return Some((PipePipe, start + 2)) }
+            if c1 == 61 { return Some((PipeEq, start + 2)) }
+            return Some((Pipe, start + 1))
+        }
+        // `+` `*` `/` `%` `^`, each alone or before `=`
+        if c == PLUS {
+            if c1 == 61 { return Some((PlusEq, start + 2)) }
+            return Some((Plus, start + 1))
+        }
+        if c == STAR {
+            if c1 == 61 { return Some((StarEq, start + 2)) }
+            return Some((Star, start + 1))
+        }
+        if c == SLASH {
+            if c1 == 61 { return Some((SlashEq, start + 2)) }
+            return Some((Slash, start + 1))
+        }
+        if c == 37 {
+            if c1 == 61 { return Some((PercentEq, start + 2)) }
+            return Some((Percent, start + 1))
+        }
+        if c == 94 {
+            if c1 == 61 { return Some((CaretEq, start + 2)) }
+            return Some((Caret, start + 1))
+        }
+        None
+    }
+
+    // The end of an exponent `[eE][+-]?[0-9]+` at `j`, or `j` if there is
+    // none there.
+    fn exponent_end(self, j: Int) -> Int {
+        let b = self.source
+        if j < self.len && (b.byte_at(j) == LOWER_E || b.byte_at(j) == UPPER_E) {
+            let mut k = j + 1
+            if k < self.len && (b.byte_at(k) == PLUS || b.byte_at(k) == MINUS) { k += 1 }
+            if k < self.len && is_digit(b.byte_at(k)) {
+                while k < self.len && is_digit(b.byte_at(k)) { k += 1 }
+                return k
+            }
+        }
+        j
+    }
+
+    // The number literals' regexes, by longest match, then their callbacks:
+    //   `0x[0-9a-fA-F][0-9a-fA-F_]*`, `0b[01][01_]*`, `0o[0-7][0-7_]*`;
+    //   `[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9]+)?` and
+    //   `[0-9][0-9_]*[eE][+-]?[0-9]+`, floats;
+    //   `[0-9][0-9_]*`, a decimal integer.
+    // A prefixed integer, when its regex matches, is always the longest.
+    fn lex_number(self, start: Int) -> Option<(Token, Int)> {
+        let b = self.source
+        let n = self.len
+        if b.byte_at(start) == ZERO && start + 2 < n {
+            let p = b.byte_at(start + 1)
+            // `x`, `b`, `o`
+            let radix = if p == 120 { 16 } else if p == 98 { 2 } else if p == 111 { 8 } else { 0 }
+            if radix > 0 && digit_value(b.byte_at(start + 2)) < radix {
+                let mut i = start + 3
+                while i < n && (digit_value(b.byte_at(i)) < radix || b.byte_at(i) == UNDERSCORE) {
+                    i += 1
+                }
+                // lex_hex_int, lex_bin_int, lex_oct_int
+                return match self.int_value(start + 2, i, radix) {
+                    Some(v) => Some((Int(v), i))
+                    None => None
+                }
+            }
+        }
+        let mut i = start + 1
+        while i < n && (is_digit(b.byte_at(i)) || b.byte_at(i) == UNDERSCORE) { i += 1 }
+        if i + 1 < n && b.byte_at(i) == DOT && is_digit(b.byte_at(i + 1)) {
+            let mut j = i + 2
+            while j < n && (is_digit(b.byte_at(j)) || b.byte_at(j) == UNDERSCORE) { j += 1 }
+            return self.lex_float(start, self.exponent_end(j))
+        }
+        let e = self.exponent_end(i)
+        if e > i { return self.lex_float(start, e) }
+        // lex_dec_int
+        match self.int_value(start, i, 10) {
+            Some(v) => Some((Int(v), i))
+            None => None
+        }
+    }
+
+    // The digits `start..end` in `radix`, underscores skipped, or `None`
+    // past `Int`'s range (`i64::from_str_radix`, `str::parse::<i64>`).
+    fn int_value(self, start: Int, end: Int, radix: Int) -> Option<Int> {
+        let mut v = 0
+        let mut i = start
+        while i < end {
+            let c = self.source.byte_at(i)
+            if c != UNDERSCORE {
+                let d = digit_value(c)
+                if v > (9223372036854775807 - d) / radix { return None }
+                v = v * radix + d
+            }
+            i += 1
+        }
+        Some(v)
+    }
+
+    // lib.rs's `lex_float`: the text without its underscores, read as Rust
+    // reads an `f64`.
+    fn lex_float(self, start: Int, end: Int) -> Option<(Token, Int)> {
+        let mut text = StringBuilder::new()
+        let mut run = start
+        let mut i = start
+        while i < end {
+            if self.source.byte_at(i) == UNDERSCORE {
+                text.push(self.source.byte_slice(run, i))
+                run = i + 1
+            }
+            i += 1
+        }
+        text.push(self.source.byte_slice(run, end))
+        match text.to_string().parse_float() {
+            Some(f) => Some((Float(f), end))
+            None => None
+        }
+    }
+
+    // `'([^'\\]|\\.)'`, where `[^'\\]` is any character but those two, and
+    // `.` any character but `\n`; then lib.rs's `lex_char`, whose escapes
+    // are `\n \t \r \\ \' \0`.
+    fn lex_char(self, start: Int) -> Option<(Token, Int)> {
+        let b = self.source
+        let n = self.len
+        let i = start + 1
+        if i >= n { return None }
+        let c = b.byte_at(i)
+        if c == BACKSLASH {
+            if i + 1 >= n || b.byte_at(i + 1) == NEWLINE { return None }
+            let q = i + 1 + utf8_char_width(b.byte_at(i + 1))
+            if q >= n || b.byte_at(q) != APOSTROPHE { return None }
+            let e = b.byte_at(i + 1)
+            let cp = if e == 110 {
+                10
+            } else if e == 116 {
+                9
+            } else if e == 114 {
+                13
+            } else if e == BACKSLASH {
+                92
+            } else if e == APOSTROPHE {
+                39
+            } else if e == ZERO {
+                0
+            } else {
+                0 - 1
+            }
+            if cp < 0 { return None }
+            return Some((Char(char_from_int(cp).unwrap()), q + 1))
+        }
+        if c == APOSTROPHE { return None }
+        let w = utf8_char_width(c)
+        let q = i + w
+        if q >= n || b.byte_at(q) != APOSTROPHE { return None }
+        Some((Char(self.char_at(i, w)), q + 1))
+    }
+}
+
+// lib.rs's `utf8_char_width`: a character's length from its first byte.
+fn utf8_char_width(b: Int) -> Int {
+    if b <= 0x7F { return 1 }
+    if b >= 0xC0 && b <= 0xDF { return 2 }
+    if b >= 0xE0 && b <= 0xEF { return 3 }
+    if b >= 0xF0 { return 4 }
+    1
+}
+
+// A keyword's token, or an identifier. A string literal allocates each time
+// it is evaluated, so this dispatches on the first byte `c` and the length
+// first, and compares the text only with the keywords that could match.
+fn keyword_or_ident(s: String, c: Int, len: Int) -> Token {
+    if len < 2 || len > 8 { return Ident(s) }
+    if c == 83 {
+        // `S`
+        if len == 4 && s == "Self" { return SelfUpper }
+    } else if c == 97 {
+        // `a`
+        if len == 2 && s == "as" { return As }
+        if len == 5 && s == "async" { return Async }
+        if len == 5 && s == "await" { return Await }
+    } else if c == 98 {
+        // `b`
+        if len == 5 && s == "break" { return Break }
+    } else if c == 99 {
+        // `c`
+        if len == 5 && s == "const" { return Const }
+        if len == 8 && s == "continue" { return Continue }
+    } else if c == 101 {
+        // `e`
+        if len == 4 && s == "else" { return Else }
+        if len == 6 && s == "extern" { return Extern }
+    } else if c == 102 {
+        // `f`
+        if len == 2 && s == "fn" { return Fn }
+        if len == 3 && s == "for" { return For }
+        if len == 5 && s == "false" { return False }
+    } else if c == 105 {
+        // `i`
+        if len == 2 && s == "if" { return If }
+        if len == 2 && s == "in" { return In }
+        if len == 2 && s == "is" { return Is }
+        if len == 4 && s == "impl" { return Impl }
+        if len == 6 && s == "import" { return Import }
+    } else if c == 108 {
+        // `l`
+        if len == 3 && s == "let" { return Let }
+    } else if c == 109 {
+        // `m`
+        if len == 3 && s == "mut" { return Mut }
+        if len == 5 && s == "match" { return Match }
+        if len == 6 && s == "module" { return Module }
+    } else if c == 112 {
+        // `p`
+        if len == 3 && s == "pub" { return Pub }
+    } else if c == 114 {
+        // `r`
+        if len == 6 && s == "return" { return Return }
+        if len == 6 && s == "record" { return Record }
+    } else if c == 115 {
+        // `s`
+        if len == 4 && s == "self" { return SelfLower }
+    } else if c == 116 {
+        // `t`
+        if len == 4 && s == "type" { return Type }
+        if len == 4 && s == "true" { return True }
+        if len == 5 && s == "trait" { return Trait }
+    } else if c == 117 {
+        // `u`
+        if len == 6 && s == "unsafe" { return Unsafe }
+    } else if c == 119 {
+        // `w`
+        if len == 4 && s == "with" { return With }
+        if len == 5 && s == "while" { return While }
+        if len == 5 && s == "where" { return Where }
+    }
+    Ident(s)
+}
+
+fn is_digit(c: Int) -> Bool { c >= ZERO && c <= NINE }
+
+// `[a-zA-Z_]`
+fn is_ident_start(c: Int) -> Bool {
+    (c >= 97 && c <= 122) || (c >= 65 && c <= 90) || c == UNDERSCORE
+}
+
+// `[a-zA-Z0-9_]`
+fn is_ident_continue(c: Int) -> Bool { is_ident_start(c) || is_digit(c) }
+
+// A digit's value in any radix up to 16, or 99 for a byte that is none.
+fn digit_value(c: Int) -> Int {
+    if c >= ZERO && c <= NINE { return c - ZERO }
+    if c >= 97 && c <= 102 { return c - 87 }
+    if c >= 65 && c <= 70 { return c - 55 }
+    99
+}
+
+// Unicode's White_Space property, which Rust's `char::is_whitespace` tests.
+fn is_white_space(cp: Int) -> Bool {
+    if cp == 32 || (cp >= 9 && cp <= 13) { return true }
+    if cp < 0x85 { return false }
+    cp == 0x85
+        || cp == 0xA0
+        || cp == 0x1680
+        || (cp >= 0x2000 && cp <= 0x200A)
+        || cp == 0x2028
+        || cp == 0x2029
+        || cp == 0x202F
+        || cp == 0x205F
+        || cp == 0x3000
+}
+
+fn is_dot(t: Token) -> Bool {
+    match t {
+        Dot => true
+        _ => false
+    }
+}
+
+fn is_float(t: Token) -> Bool {
+    match t {
+        Float(_) => true
+        _ => false
+    }
+}
+
+fn min(a: Int, b: Int) -> Int { if a < b { a } else { b } }
+
+fn max(a: Int, b: Int) -> Int { if a > b { a } else { b } }
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ba9719d0dc41784071ab650ef59fe5017eeb0e47
--- /dev/null
+++ b/selfhost/src/main.nova
@@ -0,0 +1,54 @@
+// N, the self-hosted Nova compiler, so far its lexer (spec
+// docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md
+// §3.2, ADR 0051).
+//
+//   selfhost tokens FILE...   prints each file's token dump, after a header
+//                             line `== FILE`, as `nova parse --dump-tokens`
+//                             does
+//
+// A file with lex errors is dumped with them, and the exit code stays 0. A
+// file that cannot be read prints `IOERR` and its kind after its header;
+// the rest are dumped, then the exit code is 2.
+
+import dump
+import lexer
+import lexer_tests
+import token
+
+async fn main() {
+    let argv = args()
+    // No file is refused, as S0's clap refuses it.
+    if argv.len() < 3 || argv.at(1) != "tokens" {
+        eprintln("usage: selfhost tokens FILE...")
+        exit(2)
+        return
+    }
+    let mut failed = false
+    let mut i = 2
+    while i < argv.len() {
+        let path = argv.at(i)
+        match read_to_string(path).await {
+            Ok(source) => print("== ${path}\n${dump_tokens(source)}")
+            Err(e) => {
+                print("== ${path}\nIOERR ${io_kind_name(e.kind)}\n")
+                failed = true
+            }
+        }
+        i += 1
+    }
+    if failed { exit(2) }
+}
+
+// The kind's name, as S0's dump prints it.
+fn io_kind_name(kind: IoErrorKind) -> String {
+    match kind {
+        NotFound => "NotFound"
+        PermissionDenied => "PermissionDenied"
+        AlreadyExists => "AlreadyExists"
+        InvalidData => "InvalidData"
+        Interrupted => "Interrupted"
+        TimedOut => "TimedOut"
+        ConnectionRefused => "ConnectionRefused"
+        Other => "Other"
+    }
+}
diff --git a/selfhost/src/token.nova b/selfhost/src/token.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ed3ee1da376f910766cd9cab1483f381c5dfb694
--- /dev/null
+++ b/selfhost/src/token.nova
@@ -0,0 +1,221 @@
+// The token kinds of the Nova language, and each kind's description: a
+// port of crates/nova-lexer/src/token.rs, kind for kind and word for word,
+// and of the `Spanned` and `Span` the Rust lexer takes from
+// nova-diagnostics. A span is the byte offsets `start..end`; N lexes one
+// file at a time, so it keeps no `FileId`.
+
+/// Every token produced by the lexer (token.rs's `Token`).
+pub type Token =
+    // Keywords
+    | Let
+    | Mut
+    | Const
+    | Fn
+    | Return
+    | If
+    | Else
+    | While
+    | For
+    | In
+    | Break
+    | Continue
+    | Match
+    | Type
+    | Record
+    | Trait
+    | Impl
+    | Import
+    | Module
+    | Pub
+    | Async
+    | Await
+    | Extern
+    | Unsafe
+    | True
+    | False
+    | As
+    | Is
+    | Where
+    | With
+    | SelfLower
+    | SelfUpper
+    // Punctuation
+    | LParen
+    | RParen
+    | LBracket
+    | RBracket
+    | LBrace
+    | RBrace
+    | Comma
+    | Semicolon
+    | Colon
+    | ColonColon
+    | Arrow
+    | FatArrow
+    | Dot
+    | DotDot
+    | DotDotEq
+    | At
+    | Question
+    // Comparison and assignment
+    | Eq
+    | EqEq
+    | BangEq
+    | Lt
+    | LtEq
+    | Gt
+    | GtEq
+    | AmpAmp
+    | PipePipe
+    | Bang
+    // Arithmetic
+    | Plus
+    | Minus
+    | Star
+    | Slash
+    | Percent
+    | PlusEq
+    | MinusEq
+    | StarEq
+    | SlashEq
+    | PercentEq
+    // Bitwise
+    | Amp
+    | Pipe
+    | Caret
+    | Tilde
+    | LtLt
+    | GtGt
+    | AmpEq
+    | PipeEq
+    | CaretEq
+    | LtLtEq
+    | GtGtEq
+    // Literals
+    | Int(Int)
+    | Float(Float)
+    | Char(Char)
+    // String tokens (the state machine's output)
+    | StrStart
+    | StrPart(String)
+    | StrEnd
+    | InterpOpen
+    | InterpClose
+    | RawStr(String)
+    // Identifiers and doc comments
+    | Ident(String)
+    | DocComment(String)
+    // End of file
+    | Eof
+
+/// A byte range in the source: `start..end`.
+pub record Span { pub start: Int, pub end: Int }
+
+/// A token and its span (nova-diagnostics' `Spanned<Token>`).
+pub record Spanned { pub value: Token, pub span: Span }
+
+/// A token spanning `start..end`.
+pub fn spanned(value: Token, start: Int, end: Int) -> Spanned {
+    Spanned { value: value, span: Span { start: start, end: end } }
+}
+
+impl Token {
+    /// The text the parser uses for this token in its messages
+    /// (token.rs's `description`).
+    pub fn description(self) -> String {
+        match self {
+            Let => "`let`"
+            Mut => "`mut`"
+            Const => "`const`"
+            Fn => "`fn`"
+            Return => "`return`"
+            If => "`if`"
+            Else => "`else`"
+            While => "`while`"
+            For => "`for`"
+            In => "`in`"
+            Break => "`break`"
+            Continue => "`continue`"
+            Match => "`match`"
+            Type => "`type`"
+            Record => "`record`"
+            Trait => "`trait`"
+            Impl => "`impl`"
+            Import => "`import`"
+            Module => "`module`"
+            Pub => "`pub`"
+            Async => "`async`"
+            Await => "`await`"
+            Extern => "`extern`"
+            Unsafe => "`unsafe`"
+            True => "`true`"
+            False => "`false`"
+            As => "`as`"
+            Is => "`is`"
+            Where => "`where`"
+            With => "`with`"
+            SelfLower => "`self`"
+            SelfUpper => "`Self`"
+            LParen => "`(`"
+            RParen => "`)`"
+            LBracket => "`[`"
+            RBracket => "`]`"
+            LBrace => "`{`"
+            RBrace => "`}`"
+            Comma => "`,`"
+            Semicolon => "`;`"
+            Colon => "`:`"
+            ColonColon => "`::`"
+            Arrow => "`->`"
+            FatArrow => "`=>`"
+            Dot => "`.`"
+            DotDot => "`..`"
+            DotDotEq => "`..=`"
+            At => "`@`"
+            Question => "`?`"
+            Eq => "`=`"
+            EqEq => "`==`"
+            BangEq => "`!=`"
+            Lt => "`<`"
+            LtEq => "`<=`"
+            Gt => "`>`"
+            GtEq => "`>=`"
+            AmpAmp => "`&&`"
+            PipePipe => "`||`"
+            Bang => "`!`"
+            Plus => "`+`"
+            Minus => "`-`"
+            Star => "`*`"
+            Slash => "`/`"
+            Percent => "`%`"
+            PlusEq => "`+=`"
+            MinusEq => "`-=`"
+            StarEq => "`*=`"
+            SlashEq => "`/=`"
+            PercentEq => "`%=`"
+            Amp => "`&`"
+            Pipe => "`|`"
+            Caret => "`^`"
+            Tilde => "`~`"
+            LtLt => "`<<`"
+            GtGt => "`>>`"
+            AmpEq => "`&=`"
+            PipeEq => "`|=`"
+            CaretEq => "`^=`"
+            LtLtEq => "`<<=`"
+            GtGtEq => "`>>=`"
+            Int(_) => "integer literal"
+            Float(_) => "float literal"
+            Char(_) => "char literal"
+            StrStart => "`\"`"
+            StrPart(_) => "string segment"
+            StrEnd => "`\"`"
+            InterpOpen => "`$" + "{`"
+            InterpClose => "`}`"
+            RawStr(_) => "raw string literal"
+            Ident(_) => "identifier"
+            DocComment(_) => "doc comment"
+            Eof => "end of file"
+        }
+    }
+}
````

### Task 3: The differential, its comparator and its fuzzer

Spec §3.4, §3.5, §3.6; the user's decisions 5 and 7; decisions 2, 3, 8
to 12, 14, 15, 20 to 22, 24, 29, 31, 36 to 41 and 46 to 48.

**Files:**
- Create: `crates/nova-cli/tests/selfhost/compare.rs`,
  `crates/nova-cli/tests/selfhost/fuzz.rs`, the twelve files of
  `crates/nova-cli/tests/selfhost/cases/`
- Modify: `selfhost.rs` (the differential), `crates/nova-fmt/tests/corpus.rs`
  (the cases skipped), `.github/workflows/ci.yml` (`nova fmt --check`
  over `selfhost`)
- Test: `selfhost.rs` (sixteen more), `nova-fmt`'s corpus gate

- [ ] **Step 1: The tests.** `patch_block t3-tests`. The patch holds:
  - `selfhost.rs`: the whole doc, the modules, the constants (the batch,
    the seeds and counts, the seven errors' messages), `selfhost()` with
    `once` and `build_selfhost`, `nova_files`, `corpus`, `cases_dir`,
    `batch_name`, `run`, `compare_batch`, `agree_on`, `agree`,
    `agree_on_inputs`, `mutations_of`, `mutations`, `windows_corpus`, and
    sixteen tests:
    `the_corpus_dumps_agree`,
    `the_corpus_dumps_agree_with_windows_line_endings`,
    `the_mutations_are_the_same_with_windows_line_endings`,
    `the_fixed_cases_agree`, `the_edge_inputs_agree`,
    `the_soup_agrees_and_holds_every_lex_error`,
    `the_mutated_corpus_agrees`, `a_longer_search_from_the_environment`,
    `unreadable_files_are_reported_alike`, `both_sides_refuse_no_file`,
    `the_mismatch_report_names_the_file_and_line`,
    `a_side_that_fails_is_reported_with_its_exit_code_and_stderr`,
    `a_process_past_its_limit_is_killed_and_its_batch_named`,
    `a_failed_build_is_made_once_and_reported_to_each_test`, which also
    asks that `selfhost()` give one path however often asked,
    `a_pipe_still_open_at_the_deadline_is_reported` and
    `a_missing_final_line_break_is_marked`;
  - the twelve cases: the spike's `float_underscore_after_zero` and step
    0's eleven (decision 21);
  - `corpus.rs`: `CASES`, skipped by the gate (decision 15).
- [ ] **Step 2: Run, and see the tests fail.** First the build, which
  ends with `Finished`.
  - `SH`. Expected: it does not compile,
    ``error: couldn't read `crates/nova-cli/tests/selfhost/compare.rs`: No such file or directory (os error 2)``
    and
    ``error: could not compile `nova-cli` (test "selfhost") due to 1 previous error``
    (decision 29).
  - `cargo test --locked --workspace --test corpus`. Expected:
    `1 passed; 0 failed`: the cases are skipped.
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`compare.rs`:** `Dump` and `TOKENS`; `Mismatch`;
    `first_difference`, over bytes (decision 38), which names the file
    whose header a side never printed and marks a line that differs only
    in lacking its final line break `(no line break)` (decision 48);
    `report`, with the file, the dump's
    line, the source's, both lines and the commands that reproduce it.
  - **`fuzz.rs`:** `SplitMix64`; the soup's atoms, `\r` in each kind of
    token among them, and its twelve tails; `soup`, `soups`; `mutate`, the
    self-swap fixed (decision 24); `edges`, a doc comment ending in each
    White_Space character among them (decisions 21 and 41).
  - **`ci.yml`:** the step `nova fmt --check std examples selfhost`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH`. Expected: `19 passed; 0 failed`, in about 10 s.
  - `cargo test --locked --workspace --test corpus`. Expected:
    `1 passed; 0 failed`.
  - `S` (6 min 39 s). Expected: `2816 1 10`: Task 2's figures and the
    sixteen.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `selfhost: the differential test, its comparator and its fuzzer`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 9b9fbe615fe4c9e09bcb3215a87b57e61b4de677..7949d6763b62b10098527918da80d92f366e2c62 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -6,18 +6,61 @@
 //!   header per file, and `IOERR` with the exit code 2 for a file it cannot
 //!   read;
 //! - N's own `@test`s, which `nova test` runs in `selfhost/`: as many must
-//!   pass as its sources hold.
+//!   pass as its sources hold;
+//! - the differential: N, the Nova package in `selfhost/`, built once by
+//!   `nova build`, must print the same token dump as S0, byte for byte, on
+//!   every `.nova` file of the repository and N's own sources, also with a
+//!   Windows checkout's line endings; on the fixed cases in
+//!   `tests/selfhost/cases/`; on the edge inputs; and on the fuzzer's soup
+//!   and mutated corpus files from fixed seeds (`tests/selfhost/fuzz.rs`).
 //!
-//! Each test writes its inputs to fixed directories under the system temp
-//! directory.
+//! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
+//! CI. Every input that finds a difference becomes a fixed case.
+//!
+//! Each run builds N into a fixed directory under the system temp
+//! directory, so the source tree gets no `target/`, and writes its inputs to
+//! fixed directories there too. `nova build`, `nova test` and each side on
+//! each batch are killed if they run past [`CHILD_LIMIT`].
+
+#[path = "selfhost/compare.rs"]
+mod compare;
+#[path = "selfhost/fuzz.rs"]
+mod fuzz;
 
 use std::env::consts::EXE_SUFFIX;
 use std::io::Read;
 use std::path::{Path, PathBuf};
 use std::process::{Command, Output, Stdio};
+use std::sync::OnceLock;
 use std::thread::JoinHandle;
 use std::time::{Duration, Instant};
 
+use compare::{first_difference, report, Dump, TOKENS};
+use fuzz::SplitMix64;
+
+/// How many files one process lexes.
+const BATCH: usize = 50;
+
+/// The fixed seeds and counts every run checks: 6,000 inputs of soup and
+/// 750 mutated corpus files. Measured on a 4-core Linux machine, debug
+/// build: the whole file runs in about 10 s, building N included.
+const SOUP_SEEDS: [u64; 3] = [1, 2, 3];
+const SOUP_COUNT: usize = 2000;
+const MUTATION_SEEDS: [u64; 3] = [101, 102, 103];
+const MUTATION_COUNT: usize = 250;
+
+/// The beginning of each of the seven lex errors' messages
+/// (nova-lexer's `error.rs`).
+const LEX_ERRORS: [&str; 7] = [
+    "unterminated string literal",
+    "unterminated string interpolation",
+    "unterminated block comment",
+    "invalid escape sequence",
+    "invalid unicode escape sequence",
+    "invalid numeric literal",
+    "unexpected character",
+];
+
 /// The repository, two levels above this crate.
 fn root() -> PathBuf {
     Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
@@ -247,3 +290,572 @@ fn the_selfhost_tests_pass() {
         String::from_utf8_lossy(&out.stderr)
     );
 }
+
+/// N, built once per test binary by `nova build -o` into a fixed temporary
+/// directory, from `selfhost/` in place: `-o` keeps the build out of the
+/// source tree.
+fn selfhost() -> &'static Path {
+    static BUILT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
+    once(&BUILT, build_selfhost)
+}
+
+/// What `cell` holds, made by `make` the first time any test asks. A
+/// failure is kept too, so it is made once and every test that asks
+/// reports it, rather than each running the failed build again.
+fn once(
+    cell: &OnceLock<Result<PathBuf, String>>,
+    make: impl FnOnce() -> Result<PathBuf, String>,
+) -> &Path {
+    match cell.get_or_init(make) {
+        Ok(path) => path,
+        Err(e) => panic!("{e}"),
+    }
+}
+
+fn build_selfhost() -> Result<PathBuf, String> {
+    let out = fresh_dir("build");
+    let home = fresh_dir("home");
+    let exe = out.join(format!("selfhost{EXE_SUFFIX}"));
+    let built = output_within(
+        Command::new(nova())
+            .current_dir(root().join("selfhost"))
+            .env("NOVA_HOME", &home)
+            .arg("build")
+            .arg("-o")
+            .arg(&exe)
+            .stdin(Stdio::null()),
+        CHILD_LIMIT,
+        "nova build of selfhost/",
+    )?;
+    if !built.status.success() {
+        return Err(format!(
+            "nova build of selfhost/ failed:\n{}{}",
+            String::from_utf8_lossy(&built.stdout),
+            String::from_utf8_lossy(&built.stderr)
+        ));
+    }
+    Ok(exe)
+}
+
+/// Every `.nova` file under `dir`, as paths relative to `root` with `/`
+/// between their parts, skipping directories named `target` or
+/// `node_modules`, directories whose names begin with `.`, symbolic links,
+/// and the directories in `skip`.
+fn nova_files(root: &Path, dir: &Path, skip: &[PathBuf], out: &mut Vec<String>) {
+    let mut entries: Vec<_> = std::fs::read_dir(dir)
+        .expect("read a directory")
+        .map(|e| e.expect("read a directory entry"))
+        .collect();
+    entries.sort_by_key(|e| e.file_name());
+    for e in entries {
+        let name = e.file_name().to_string_lossy().into_owned();
+        let kind = e.file_type().expect("a file type");
+        let path = e.path();
+        if kind.is_dir() {
+            let skipped = name == "target"
+                || name == "node_modules"
+                || name.starts_with('.')
+                || skip.contains(&path);
+            if !skipped {
+                nova_files(root, &path, skip, out);
+            }
+        } else if kind.is_file() && name.ends_with(".nova") {
+            let rel = path.strip_prefix(root).expect("under the root");
+            let parts: Vec<String> = rel
+                .components()
+                .map(|c| c.as_os_str().to_string_lossy().into_owned())
+                .collect();
+            out.push(parts.join("/"));
+        }
+    }
+}
+
+/// The corpus: every `.nova` file of the repository outside `selfhost/` and
+/// the fixed cases (which their own test runs), sorted, then N's own
+/// sources.
+fn corpus() -> Vec<String> {
+    let root = root();
+    let mut files = Vec::new();
+    let skip = [root.join("selfhost"), cases_dir()];
+    nova_files(&root, &root, &skip, &mut files);
+    let mut own = Vec::new();
+    for sub in ["src", "tests"] {
+        let dir = root.join("selfhost").join(sub);
+        if dir.is_dir() {
+            nova_files(&root, &dir, &[], &mut own);
+        }
+    }
+    files.extend(own);
+    files
+}
+
+fn cases_dir() -> PathBuf {
+    root()
+        .join("crates")
+        .join("nova-cli")
+        .join("tests")
+        .join("selfhost")
+        .join("cases")
+}
+
+/// One side's run over one batch: its output, as bytes, and exit code.
+struct Run {
+    stdout: Vec<u8>,
+    code: Option<i32>,
+    stderr: String,
+}
+
+/// A batch, named for a report: the command, and how many files from which.
+fn batch_name(program: &Path, args: &[&str], files: &[String]) -> String {
+    format!(
+        "{} {} on the batch of {} files from {}",
+        program.display(),
+        args.join(" "),
+        files.len(),
+        files.first().map_or("(none)", String::as_str)
+    )
+}
+
+/// `program` with `args` on `files`, within [`CHILD_LIMIT`].
+fn run(program: &Path, args: &[&str], dir: &Path, files: &[String]) -> Result<Run, String> {
+    let out = output_within(
+        Command::new(program)
+            .current_dir(dir)
+            .args(args)
+            .args(files)
+            .stdin(Stdio::null()),
+        CHILD_LIMIT,
+        &batch_name(program, args, files),
+    )?;
+    Ok(Run {
+        stdout: out.stdout,
+        code: out.status.code(),
+        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
+    })
+}
+
+/// One batch, `files` relative to `dir`, through `nova` and `selfhost` with
+/// `dump`'s arguments: S0's output, or the report of where the two part,
+/// in their dumps or in their exit codes, both of which must be `code`. The
+/// report gives both exit codes and the start of anything either side
+/// wrote to stderr, so a side that crashed or printed nothing says so; a
+/// side that runs past the limit is killed and reported with the batch.
+fn compare_batch(
+    dump: &Dump,
+    dir: &Path,
+    files: &[String],
+    code: i32,
+    nova: &Path,
+    selfhost: &Path,
+) -> Result<String, String> {
+    let s0 = run(nova, dump.s0, dir, files)?;
+    let n = run(selfhost, dump.n, dir, files)?;
+    let mismatch = first_difference(&s0.stdout, &n.stdout);
+    if mismatch.is_none() && s0.code == Some(code) && n.code == Some(code) {
+        return Ok(String::from_utf8_lossy(&s0.stdout).into_owned());
+    }
+    let mut text = match &mismatch {
+        Some(m) => report(m, dump, dir, nova, selfhost),
+        None => format!("the dumps agree on {files:?}, but not the exit codes"),
+    };
+    let shown =
+        |c: Option<i32>| c.map_or("none (killed by a signal)".to_string(), |c| c.to_string());
+    text.push_str(&format!(
+        "\nexit codes, each must be {code}: S0 {}, N {}",
+        shown(s0.code),
+        shown(n.code)
+    ));
+    for (side, stderr) in [("S0", &s0.stderr), ("N", &n.stderr)] {
+        if !stderr.trim().is_empty() {
+            let start: Vec<&str> = stderr.lines().take(40).collect();
+            text.push_str(&format!("\n{side}'s stderr:\n{}", start.join("\n")));
+        }
+    }
+    Err(text)
+}
+
+/// Run `files`, relative to `dir`, through S0's and N's `dump`, `batch`
+/// files to a process, and fail with the report of the first difference.
+/// Both sides must exit with `code`. Returns S0's output.
+fn agree_on(dump: &Dump, dir: &Path, files: &[String], batch: usize, code: i32) -> String {
+    let (nova, selfhost) = (nova(), selfhost());
+    let mut all = String::new();
+    for chunk in files.chunks(batch) {
+        match compare_batch(dump, dir, chunk, code, &nova, selfhost) {
+            Ok(out) => all.push_str(&out),
+            Err(report) => panic!("{report}"),
+        }
+    }
+    all
+}
+
+/// [`agree_on`] the token dump.
+fn agree(dir: &Path, files: &[String], batch: usize, code: i32) -> String {
+    agree_on(&TOKENS, dir, files, batch, code)
+}
+
+/// Write `inputs` to a fixed directory named after `name`, as
+/// `f00000.nova` and on, and check that both sides agree on them. Returns
+/// S0's output. The fuzzer's generators all come through here, 5.1b's deep
+/// nesting included.
+fn agree_on_inputs(name: &str, inputs: &[String]) -> String {
+    let dir = fresh_dir(name);
+    let mut files = Vec::with_capacity(inputs.len());
+    for (i, input) in inputs.iter().enumerate() {
+        let file = format!("f{i:05}.nova");
+        std::fs::write(dir.join(&file), input).expect("write a fuzz input");
+        files.push(file);
+    }
+    agree(&dir, &files, 100, 0)
+}
+
+/// `count` mutated files from `seed`, each one of `files`, under `root`,
+/// that the seed picks. The files are picked from their sorted list and
+/// read with each `\r\n` as `\n`, so a seed gives the same inputs on every
+/// system, a Windows checkout's line endings included, for the same files;
+/// an untracked local `.nova` file is one more to pick, and changes them.
+fn mutations_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<String> {
+    let mut files = files.to_vec();
+    files.sort();
+    let mut rng = SplitMix64::new(seed);
+    (0..count)
+        .map(|_| {
+            let file = rng.pick(&files);
+            let source = std::fs::read_to_string(root.join(file))
+                .expect("read a corpus file")
+                .replace("\r\n", "\n");
+            fuzz::mutate(&mut rng, &source)
+        })
+        .collect()
+}
+
+/// The mutated corpus: [`mutations_of`] the corpus.
+fn mutations(seed: u64, count: usize) -> Vec<String> {
+    mutations_of(&root(), &corpus(), seed, count)
+}
+
+/// The corpus as a Windows checkout has it, written to a fixed directory
+/// named after `name`: git's `core.autocrlf`, which GitHub's Windows
+/// runners set, writes each `\n` of a text file as `\r\n`, and leaves a
+/// file that already holds a `\r` as it is. Returns the directory and the
+/// files, relative to it.
+fn windows_corpus(name: &str) -> (PathBuf, Vec<String>) {
+    let root = root();
+    let dir = fresh_dir(name);
+    let files = corpus();
+    for file in &files {
+        let text = std::fs::read(root.join(file)).expect("read a corpus file");
+        let crlf = if text.contains(&b'\r') {
+            text
+        } else {
+            let mut out = Vec::with_capacity(text.len() + text.len() / 16);
+            for &b in &text {
+                if b == b'\n' {
+                    out.push(b'\r');
+                }
+                out.push(b);
+            }
+            out
+        };
+        let path = dir.join(file);
+        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
+        std::fs::write(&path, crlf).expect("write a corpus file");
+    }
+    (dir, files)
+}
+
+#[test]
+fn the_corpus_dumps_agree() {
+    let files = corpus();
+    // 281 under std/, tests/ and examples/ alone at 5.0c.
+    assert!(files.len() >= 290, "only {} corpus files", files.len());
+    assert!(files.iter().any(|f| f == "selfhost/src/lexer.nova"));
+    agree(&root(), &files, BATCH, 0);
+}
+
+/// Both lexers must agree on the corpus as a Windows checkout has it
+/// ([`windows_corpus`]) on every system, not on Windows alone.
+#[test]
+fn the_corpus_dumps_agree_with_windows_line_endings() {
+    let (dir, files) = windows_corpus("crlf");
+    let dumps = agree(&dir, &files, BATCH, 0);
+    assert!(dumps.contains(" NL\n"), "the corpus has line breaks");
+}
+
+/// A seed's mutations are the same on every system: from the corpus as a
+/// Windows checkout has it, listed in another order, they are the
+/// corpus's.
+#[test]
+fn the_mutations_are_the_same_with_windows_line_endings() {
+    let (dir, files) = windows_corpus("crlf-mutations");
+    let mut reversed = files.clone();
+    reversed.reverse();
+    for seed in MUTATION_SEEDS {
+        assert!(
+            mutations_of(&dir, &reversed, seed, 50) == mutations_of(&root(), &files, seed, 50),
+            "seed {seed} mutates the Windows checkout otherwise"
+        );
+    }
+}
+
+#[test]
+fn the_fixed_cases_agree() {
+    let root = root();
+    let mut files = Vec::new();
+    nova_files(&root, &cases_dir(), &[], &mut files);
+    assert!(files.len() >= 10, "only {} fixed cases", files.len());
+    agree(&root, &files, BATCH, 0);
+}
+
+#[test]
+fn the_edge_inputs_agree() {
+    agree_on_inputs("edges", &fuzz::edges());
+}
+
+#[test]
+fn the_soup_agrees_and_holds_every_lex_error() {
+    let mut dumps = String::new();
+    for seed in SOUP_SEEDS {
+        dumps.push_str(&agree_on_inputs(
+            &format!("soup-{seed}"),
+            &fuzz::soups(seed, SOUP_COUNT),
+        ));
+    }
+    for error in LEX_ERRORS {
+        let found = dumps.lines().any(|line| {
+            line.starts_with("ERR ")
+                && line
+                    .splitn(4, ' ')
+                    .nth(3)
+                    .is_some_and(|m| m.starts_with(error))
+        });
+        assert!(found, "no `{error}` in the soup of seeds {SOUP_SEEDS:?}");
+    }
+}
+
+#[test]
+fn the_mutated_corpus_agrees() {
+    for seed in MUTATION_SEEDS {
+        agree_on_inputs(&format!("mutated-{seed}"), &mutations(seed, MUTATION_COUNT));
+    }
+}
+
+/// `NOVA_SELFHOST_FUZZ=SEED:COUNT`: COUNT inputs of soup and COUNT mutated
+/// corpus files from SEED. Unset, as in CI, it does nothing.
+#[test]
+fn a_longer_search_from_the_environment() {
+    let Ok(spec) = std::env::var("NOVA_SELFHOST_FUZZ") else {
+        return;
+    };
+    let (seed, count) = spec
+        .split_once(':')
+        .and_then(|(s, c)| Some((s.parse::<u64>().ok()?, c.parse::<usize>().ok()?)))
+        .expect("NOVA_SELFHOST_FUZZ is SEED:COUNT");
+    agree_on_inputs("search-soup", &fuzz::soups(seed, count));
+    // Mutations go in batches of 500, each from a seed of its own, so a long
+    // search keeps few inputs on disk at a time.
+    let mut done = 0;
+    while done < count {
+        let n = (count - done).min(500);
+        agree_on_inputs(
+            "search-mutated",
+            &mutations(seed.wrapping_add(done as u64), n),
+        );
+        done += n;
+    }
+}
+
+#[test]
+fn unreadable_files_are_reported_alike() {
+    let dir = fresh_dir("unreadable");
+    std::fs::write(dir.join("ok.nova"), "let x = 1\n").expect("write ok.nova");
+    std::fs::write(dir.join("latin1.nova"), b"let \xe9 = 1\n").expect("write latin1.nova");
+    let files = ["missing.nova", "ok.nova", "latin1.nova"].map(String::from);
+    let out = agree(&dir, &files, BATCH, 2);
+    assert!(
+        out.starts_with("== missing.nova\nIOERR NotFound\n== ok.nova\n0 3 Let\n"),
+        "{out}"
+    );
+    assert!(
+        out.ends_with("== latin1.nova\nIOERR InvalidData\n"),
+        "{out}"
+    );
+}
+
+/// No file at all: both sides refuse it, print nothing and exit 2.
+#[test]
+fn both_sides_refuse_no_file() {
+    let out = compare_batch(&TOKENS, &root(), &[], 2, &nova(), selfhost());
+    assert_eq!(out, Ok(String::new()));
+}
+
+#[test]
+fn the_mismatch_report_names_the_file_and_line() {
+    let dir = fresh_dir("mismatch");
+    std::fs::write(dir.join("a.nova"), "let a = 1\n").expect("write a.nova");
+    std::fs::write(dir.join("b.nova"), "fn f() {}\n\nlet y = 2.5\n").expect("write b.nova");
+    let files = ["a.nova", "b.nova"].map(String::from);
+    let s0 = run(&nova(), TOKENS.s0, &dir, &files)
+        .expect("run S0")
+        .stdout;
+    let s0 = String::from_utf8(s0).expect("a dump of UTF-8 files is UTF-8");
+    let wrong = s0.replace("Float 4004000000000000", "Float 4004000000000001");
+    assert_ne!(s0, wrong, "the dump holds 2.5's bits:\n{s0}");
+    let m = first_difference(s0.as_bytes(), wrong.as_bytes()).expect("a difference");
+    assert_eq!(m.file.as_deref(), Some("b.nova"));
+    assert_eq!(m.dump_line, 10);
+    let text = report(&m, &TOKENS, &dir, &nova(), Path::new("selfhost"));
+    for part in [
+        "in b.nova, at line 10 of its dump, source line 3",
+        "S0 (nova parse --dump-tokens): 19 22 Float 4004000000000000",
+        "N  (selfhost tokens):          19 22 Float 4004000000000001",
+        "parse --dump-tokens b.nova > s0.txt",
+        "selfhost tokens b.nova > n.txt",
+    ] {
+        assert!(text.contains(part), "{part:?} is not in:\n{text}");
+    }
+    // A side that stops before a file names that file.
+    let short = &s0[..s0.rfind("== b.nova").expect("b's header")];
+    let m = first_difference(s0.as_bytes(), short.as_bytes()).expect("a difference");
+    assert_eq!((m.file.as_deref(), m.dump_line), (Some("b.nova"), 0));
+    assert_eq!(m.n, None);
+    let text = report(&m, &TOKENS, &dir, &nova(), Path::new("selfhost"));
+    assert!(
+        text.contains("in b.nova, at its header: one side's output ended before it"),
+        "{text}"
+    );
+    // Bytes that are not UTF-8 differ as bytes, though each reads as the
+    // same replacement character.
+    let m = first_difference(
+        b"== a.nova\n0 1 Ident \"\xff\"\n",
+        b"== a.nova\n0 1 Ident \"\xfe\"\n",
+    )
+    .expect("a difference");
+    assert_eq!((m.file.as_deref(), m.dump_line), (Some("a.nova"), 1));
+}
+
+/// A side that fails, here a stand-in for N that is `nova` given an
+/// argument it refuses, so that it prints nothing: the report names the
+/// file it never reached, both exit codes and its stderr. Two sides whose
+/// dumps agree and whose exit codes do not are reported too.
+#[test]
+fn a_side_that_fails_is_reported_with_its_exit_code_and_stderr() {
+    const BROKEN: Dump = Dump {
+        s0: &["parse", "--dump-tokens"],
+        n: &["parse", "--no-such-flag"],
+    };
+    const SAME: Dump = Dump {
+        s0: &["parse", "--dump-tokens"],
+        n: &["parse", "--dump-tokens"],
+    };
+    let dir = fresh_dir("broken");
+    std::fs::write(dir.join("a.nova"), "let a = 1\n").expect("write a.nova");
+    let nova = nova();
+    let files = ["a.nova".to_string()];
+    let text = compare_batch(&BROKEN, &dir, &files, 0, &nova, &nova).expect_err("N fails");
+    for part in [
+        "the dumps differ in a.nova, at its header: one side's output ended before it",
+        "(selfhost parse --no-such-flag): (nothing: the output ended)",
+        "exit codes, each must be 0: S0 0, N 2",
+        "N's stderr:\nerror: unexpected argument '--no-such-flag' found",
+    ] {
+        assert!(text.contains(part), "{part:?} is not in:\n{text}");
+    }
+    let files = ["missing.nova".to_string()];
+    let text = compare_batch(&SAME, &dir, &files, 0, &nova, &nova).expect_err("both exit 2");
+    for part in [
+        "the dumps agree on [\"missing.nova\"], but not the exit codes",
+        "exit codes, each must be 0: S0 2, N 2",
+    ] {
+        assert!(text.contains(part), "{part:?} is not in:\n{text}");
+    }
+    assert!(compare_batch(&SAME, &dir, &files, 2, &nova, &nova).is_ok());
+}
+
+/// A process past its limit is killed, and the error names its batch: here
+/// `nova lsp`, which reads its stdin, held open and empty, until it ends.
+/// A limit that never fired would leave this test to fail at its own
+/// deadline, a minute on.
+#[test]
+fn a_process_past_its_limit_is_killed_and_its_batch_named() {
+    let (send, receive) = std::sync::mpsc::channel();
+    std::thread::spawn(move || {
+        let files = ["a.nova", "b.nova"].map(String::from);
+        let what = batch_name(&nova(), &["lsp"], &files);
+        let mut lsp = Command::new(nova());
+        lsp.arg("lsp").stdin(Stdio::piped());
+        let _ = send.send(output_within(&mut lsp, Duration::from_millis(300), &what));
+    });
+    let out = receive
+        .recv_timeout(Duration::from_secs(60))
+        .expect("nova lsp was not killed at its limit");
+    let error = out.expect_err("nova lsp does not end while its stdin is open");
+    assert!(
+        error.ends_with(
+            " lsp on the batch of 2 files from a.nova did not finish within 300ms: killed"
+        ),
+        "{error}"
+    );
+}
+
+/// A failed build of N is made once and reported by each test that asks.
+#[test]
+fn a_failed_build_is_made_once_and_reported_to_each_test() {
+    let cell = OnceLock::new();
+    let builds = std::cell::Cell::new(0);
+    for _ in 0..2 {
+        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
+            once(&cell, || {
+                builds.set(builds.get() + 1);
+                Err("nova build of selfhost/ failed".to_string())
+            });
+        }))
+        .expect_err("a failed build fails the test that asks");
+        assert_eq!(
+            failed.downcast_ref::<String>().map(String::as_str),
+            Some("nova build of selfhost/ failed")
+        );
+    }
+    assert_eq!(builds.get(), 1, "the failed build ran again");
+    // `selfhost()` asks `once`: however often a test asks, N is one.
+    assert!(std::ptr::eq(selfhost(), selfhost()), "N was built again");
+}
+
+/// A pipe still open when the time is up, as when a process the child
+/// started holds it after the child exits, is reported, not waited for:
+/// here a reader that would finish two seconds on.
+#[test]
+fn a_pipe_still_open_at_the_deadline_is_reported() {
+    let (stop, stopped) = std::sync::mpsc::channel::<()>();
+    let held = std::thread::spawn(move || {
+        let _ = stopped.recv_timeout(Duration::from_secs(2));
+        b"late".to_vec()
+    });
+    let done = std::thread::spawn(|| b"out".to_vec());
+    let start = Instant::now();
+    let read = read_by([done, held], start + Duration::from_millis(100));
+    assert!(read.is_none(), "a reader still open was waited for");
+    assert!(start.elapsed() < Duration::from_secs(2));
+    drop(stop);
+    let done = std::thread::spawn(|| b"out".to_vec());
+    let err = std::thread::spawn(Vec::new);
+    let read = read_by([done, err], Instant::now() + Duration::from_secs(60));
+    assert_eq!(read, Some([b"out".to_vec(), Vec::new()]));
+}
+
+/// Two lines that differ only in a final line break, which one side lacks,
+/// read the same; the report marks the one without it.
+#[test]
+fn a_missing_final_line_break_is_marked() {
+    let m = first_difference(b"== a.nova\n0 0 Eof\n", b"== a.nova\n0 0 Eof").expect("a difference");
+    assert_eq!(m.s0.as_deref(), Some("0 0 Eof"));
+    assert_eq!(m.n.as_deref(), Some("0 0 Eof (no line break)"));
+    let text = report(&m, &TOKENS, Path::new("."), &nova(), Path::new("selfhost"));
+    assert!(text.contains("0 0 Eof (no line break)"), "{text}");
+    let m = first_difference(b"== a.nova\n1 2 Eof", b"== a.nova\n1 2 Eof\n").expect("a difference");
+    assert_eq!(m.s0.as_deref(), Some("1 2 Eof (no line break)"));
+    assert_eq!(m.n.as_deref(), Some("1 2 Eof"));
+    // Lines that differ otherwise are shown as they are.
+    let m = first_difference(b"== a.nova\n0 0 Eof\n", b"== a.nova\n0 1 Eof").expect("a difference");
+    assert_eq!(m.n.as_deref(), Some("0 1 Eof"));
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/an_open_block_comment_with_a_quote.nova b/crates/nova-cli/tests/selfhost/cases/an_open_block_comment_with_a_quote.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f4138dceb1d31e383583c699820e5437625d8e7f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/an_open_block_comment_with_a_quote.nova
@@ -0,0 +1,3 @@
+// A block comment that never closes runs to the end, a `"` inside it
+// included.
+let a = 1 /* " never closed
diff --git a/crates/nova-cli/tests/selfhost/cases/chars_and_quotes.nova b/crates/nova-cli/tests/selfhost/cases/chars_and_quotes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..14a9bfb1724e4a17a732042559dbdea20c5f1024
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/chars_and_quotes.nova
@@ -0,0 +1,5 @@
+// Char literals: each escape, a quote, three quotes, an empty pair, two
+// characters, a bad escape and a non-ASCII escape.
+let a = 'a' '\n' '\t' '\r' '\\' '\'' '\0' '"' 'é' '😀'
+let b = ''' '' 'ab' '\q' '\é' '\
+'
diff --git a/crates/nova-cli/tests/selfhost/cases/dollars_and_braces.nova b/crates/nova-cli/tests/selfhost/cases/dollars_and_braces.nova
new file mode 100644
index 0000000000000000000000000000000000000000..22b7822fb8b5ddef8e68042df700d3f960cfcaac
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/dollars_and_braces.nova
@@ -0,0 +1,4 @@
+// `$` without `{` is text; braces inside a hole are counted, and a `}`
+// outside every hole is a brace.
+let a = "$" "$x" "$$" "a$" "${}" "${ { } }" "${ {a: 1} }" "x${"${"y"}"}z"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/float_underscore_after_zero.nova b/crates/nova-cli/tests/selfhost/cases/float_underscore_after_zero.nova
new file mode 100644
index 0000000000000000000000000000000000000000..846f06ec74530eabaf1f4b8798985436bc25f0d0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/float_underscore_after_zero.nova
@@ -0,0 +1,5 @@
+// The 5.0 spike's one difference: its float workaround trimmed the leading
+// zeros before it removed the `_`, and misread `0_81.5`.
+let a = 0_81.5
+let b = 00_1.25e1_0
+let c = 0_0.5
diff --git a/crates/nova-cli/tests/selfhost/cases/floats_and_exponents.nova b/crates/nova-cli/tests/selfhost/cases/floats_and_exponents.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9359dc1819210accc94457063c101246b3f0c47f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/floats_and_exponents.nova
@@ -0,0 +1,4 @@
+// Floats: exponents with and without a sign, `_` in each part, and the
+// texts that are not one float: `1.e5`, `1._0`, `.5`, `5.`, `1.5e`.
+let a = 1e5 1E+5 1e-5 1_000.5e1_0 1.0e10 1e400 1.0e-400 00.5 1_.5
+let b = 1.e5 1._0 .5 5. 5.. 5..6 1.5.6 0.1e 1.5e 1.5e+ 1e_5 1e+_5
diff --git a/crates/nova-cli/tests/selfhost/cases/integers_at_the_edges.nova b/crates/nova-cli/tests/selfhost/cases/integers_at_the_edges.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f9aa7bb7d89e1cedf11c8a7b99b2758ed5a49bb7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/integers_at_the_edges.nova
@@ -0,0 +1,10 @@
+// Integers at Int's edges: one past the largest overflows, in each radix,
+// so its first digit is an unexpected character and lexing moves on one
+// character; `_` after a prefix, and a prefix with no digit, are no integer.
+let a = 9223372036854775807
+let b = 9223372036854775808
+let c = -9223372036854775808
+let d = 0x7FFF_FFFF_FFFF_FFFF 0x8000000000000000 0xFFFFFFFFFFFFFFFFF
+let e = 0b111111111111111111111111111111111111111111111111111111111111111 0b1000000000000000000000000000000000000000000000000000000000000000
+let f = 0o777777777777777777777 0o1000000000000000000000
+let g = 0x_1 0b_1 0o_7 0x 0b 0o 0b2 0o8 0xg 0x1_ 1_ 1__2 0_0
diff --git a/crates/nova-cli/tests/selfhost/cases/keywords_and_identifiers.nova b/crates/nova-cli/tests/selfhost/cases/keywords_and_identifiers.nova
new file mode 100644
index 0000000000000000000000000000000000000000..50f5cd0dc927180156200f7f6f53ee1d4f8e9edd
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/keywords_and_identifiers.nova
@@ -0,0 +1,6 @@
+// A keyword is a whole match; every keyword, then words that only start
+// as one, and identifiers that start as a raw string would.
+let mut const fn return if else while for in break continue match type record trait impl
+import module pub async await extern unsafe true false as is where with self Self
+lets mutable constant fnord returns iffy elsewhere fork inner breaks selfish Selfie asy
+async_ awaits _ __ _1 A_1 r ra rx br bar
diff --git a/crates/nova-cli/tests/selfhost/cases/nested_holes.nova b/crates/nova-cli/tests/selfhost/cases/nested_holes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1fb1577980040a4af897e21e80cc670c49ae61e6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/nested_holes.nova
@@ -0,0 +1,3 @@
+// Holes nested fifty deep, closed, and then left open at the end.
+let a = "${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"x"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"}"
+let b = "${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"${"x
diff --git a/crates/nova-cli/tests/selfhost/cases/punctuation_longest_match.nova b/crates/nova-cli/tests/selfhost/cases/punctuation_longest_match.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0baa018015918d23cb1b41c3eb9665ba6a9604ea
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/punctuation_longest_match.nova
@@ -0,0 +1,3 @@
+// Punctuation by longest match: three, then two, then one character.
+a ..= ... ..== <<= <<<= >>= >>>= ->= => =>= ::: :: !== &&= ||= &| ^= %= ~= @? #
+b += -= *= /= %= &= |= ^= == != <= >= && || << >> -> => :: .. ..= + - * / % & | ^ ~ ! = < > . , ; : @ ?
diff --git a/crates/nova-cli/tests/selfhost/cases/raw_strings_and_holes.nova b/crates/nova-cli/tests/selfhost/cases/raw_strings_and_holes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..84e41b1998e6c43e20392d436ffe3a3e39ab5d46
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/raw_strings_and_holes.nova
@@ -0,0 +1,4 @@
+// A `${` in a raw string is text; hashes close a raw string only when as
+// many follow its `"`.
+let a = r"${x}" r#"${"# r##"a"#b"## r#"one"## r"" r#""#
+let b = r#x r## r
diff --git a/crates/nova-cli/tests/selfhost/cases/slashes.nova b/crates/nova-cli/tests/selfhost/cases/slashes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d5c3021d069f9cba60a0bea517ff44cbd4f3e16a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/slashes.nova
@@ -0,0 +1,7 @@
+//// four
+///// five
+/// three
+///
+//
+let a = 1 / 2 /= 3 //x
+/**/ /***/ /* * / */ /* a **/ b
diff --git a/crates/nova-cli/tests/selfhost/cases/tuple_indexes.nova b/crates/nova-cli/tests/selfhost/cases/tuple_indexes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4e2ee26a6436fbb938435ee7a19bb21474d6c215
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/tuple_indexes.nova
@@ -0,0 +1,5 @@
+// The float split after a `.`: plain digits on both sides split; a leading
+// zero, an `_`, an exponent, `..` before it or a part past Int do not.
+let a = t.0.1.2 x.1.2.3.4 a.0.0 t.1.10
+let b = t.1e5 t.01.2 t.1.02 t.1_0.2 t..1.2 t.1.5e3
+let c = t.99999999999999999999.1 t.1.99999999999999999999 t.9223372036854775807.9223372036854775808
diff --git a/crates/nova-fmt/tests/corpus.rs b/crates/nova-fmt/tests/corpus.rs
index b7d523c1ae3072b7d1ec878ba55a0e01931c8a4a..930894b6ffb284efab1689c7c093f8a3c5e8f2fe 100644
--- a/crates/nova-fmt/tests/corpus.rs
+++ b/crates/nova-fmt/tests/corpus.rs
@@ -23,6 +23,10 @@ const EXPECTED_SKIP: [&str; 2] = [
     "tools/vscode-nova/test/fixture/src/complete.nova",
 ];
 
+/// The self-hosted lexer's fixed cases, inputs kept because they once made
+/// the two lexers disagree, many of them on purpose not Nova (ADR 0051).
+const CASES: &str = "crates/nova-cli/tests/selfhost/cases/";
+
 /// The gate's floor, set in 3.1: the 168 tracked files then, less the one
 /// that did not parse.
 const AT_LEAST: usize = 167;
@@ -166,6 +170,9 @@ fn every_corpus_file_formats_to_the_same_program_and_comments() {
             .unwrap()
             .to_string_lossy()
             .replace('\\', "/");
+        if rel.starts_with(CASES) {
+            continue;
+        }
         let text = std::fs::read_to_string(path).unwrap().replace("\r\n", "\n");
         let Some(before) = summary(&text) else {
             skipped.push(rel);
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 6984c22cdd4db6d5ca7467e6476b25f18271a580..a48718132be798b7096b8f589394d7771b388831 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -75,11 +75,12 @@ jobs:
           NOVA_CLANG: ${{ matrix.clang || 'clang' }}
         run: cargo test --locked --workspace --all-features --no-fail-fast
 
-      # std/ and examples/ stay in nova fmt's layout (spec
-      # docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §8).
-      # On Windows the checkout may hold CRLF files, which nova fmt keeps.
-      - name: nova fmt --check std examples
-        run: cargo run --locked -q -p nova-cli -- fmt --check std examples
+      # std/, examples/ and selfhost/ stay in nova fmt's layout (spec
+      # docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md §8;
+      # selfhost/ since 5.1a, ADR 0051). On Windows the checkout may hold
+      # CRLF files, which nova fmt keeps.
+      - name: nova fmt --check std examples selfhost
+        run: cargo run --locked -q -p nova-cli -- fmt --check std examples selfhost
 
       # The eight `#[ignore]`d GC root tests are the only coverage of `PINNED`
       # actually being HONOURED by a real collection -- the registry could be
diff --git a/crates/nova-cli/tests/selfhost/compare.rs b/crates/nova-cli/tests/selfhost/compare.rs
new file mode 100644
index 0000000000000000000000000000000000000000..3da8f66042ef637c20206a60886d4ea16124bede
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/compare.rs
@@ -0,0 +1,154 @@
+//! The comparison of S0's and N's dumps, and the report of the first place
+//! they differ: the file, the line of its dump and of its source, both
+//! sides' lines, and how to reproduce it.
+
+use std::path::Path;
+
+/// A pair of commands whose outputs must agree: S0's arguments to `nova`,
+/// and N's to `selfhost`, each followed by the files.
+pub struct Dump {
+    pub s0: &'static [&'static str],
+    pub n: &'static [&'static str],
+}
+
+/// 5.1a's token dump. 5.1b's AST dump is a second `Dump` beside it.
+pub const TOKENS: Dump = Dump {
+    s0: &["parse", "--dump-tokens"],
+    n: &["tokens"],
+};
+
+/// Where two dumps first differ.
+#[derive(Debug, PartialEq, Eq)]
+pub struct Mismatch {
+    /// The file whose dump differs: its `== FILE` header's path, or `None`
+    /// before the first header.
+    pub file: Option<String>,
+    /// The line within that file's dump, 1 for the line after its header;
+    /// 0 for the header itself, where one side's output ended before it.
+    pub dump_line: usize,
+    /// S0's line there, or `None` past the end of its output. A line that
+    /// differs from N's only in lacking the final line break ends
+    /// ` (no line break)`.
+    pub s0: Option<String>,
+    /// N's line there, or `None` past the end of its output, marked as S0's.
+    pub n: Option<String>,
+}
+
+/// The first line where `s0` and `n` differ, or `None` if they are equal.
+/// They are compared as bytes, so output that is not UTF-8 differs where
+/// its bytes do; the lines are shown lossily.
+pub fn first_difference(s0: &[u8], n: &[u8]) -> Option<Mismatch> {
+    if s0 == n {
+        return None;
+    }
+    let mut left = s0.split_inclusive(|&b| b == b'\n');
+    let mut right = n.split_inclusive(|&b| b == b'\n');
+    let mut file = None;
+    let mut dump_line = 0;
+    loop {
+        let (a, b) = (left.next(), right.next());
+        if a != b {
+            // Lines that differ only in a final line break, which one side
+            // lacks, would read the same: the one without it says so.
+            let unbroken = |s: &[u8], other: Option<&[u8]>| {
+                !s.ends_with(b"\n") && other.is_some_and(|o| o.strip_suffix(b"\n") == Some(s))
+            };
+            let line = |s: Option<&[u8]>, other: Option<&[u8]>| {
+                s.map(|s| {
+                    if unbroken(s, other) {
+                        format!("{} (no line break)", text(s))
+                    } else {
+                        text(s.strip_suffix(b"\n").unwrap_or(s))
+                    }
+                })
+            };
+            // One side ended where the other begins a file: that file is
+            // the one the ended side never reached, so it is named.
+            let header = match (a, b) {
+                (None, Some(h)) | (Some(h), None) => h.strip_prefix(b"== "),
+                _ => None,
+            };
+            return Some(match header {
+                Some(path) => Mismatch {
+                    file: Some(text(path).trim_end_matches('\n').to_string()),
+                    dump_line: 0,
+                    s0: line(a, b),
+                    n: line(b, a),
+                },
+                None => Mismatch {
+                    file,
+                    dump_line: dump_line + 1,
+                    s0: line(a, b),
+                    n: line(b, a),
+                },
+            });
+        }
+        let a = a.expect("equal outputs were returned above");
+        match a.strip_prefix(b"== ") {
+            Some(path) => {
+                file = Some(text(path).trim_end_matches('\n').to_string());
+                dump_line = 0;
+            }
+            None => dump_line += 1,
+        }
+    }
+}
+
+/// `bytes` as text, each run that is not UTF-8 as a replacement character.
+fn text(bytes: &[u8]) -> String {
+    String::from_utf8_lossy(bytes).into_owned()
+}
+
+/// The 1-based source line of the offset a dump line starts with (a token's
+/// start, or an error's or comment's), if `file` can be read from `dir`.
+fn source_line(dir: &Path, file: &str, line: &str) -> Option<usize> {
+    let mut fields = line.split(' ');
+    let first = fields.next()?;
+    let offset: usize = match first {
+        "ERR" | "COMMENT" => fields.next()?.parse().ok()?,
+        _ => first.parse().ok()?,
+    };
+    let source = std::fs::read(dir.join(file)).ok()?;
+    let before = source.get(..offset.min(source.len()))?;
+    Some(1 + before.iter().filter(|&&b| b == b'\n').count())
+}
+
+/// The report of `m`, a difference in `dump` on files read from `dir`, with
+/// the commands that reproduce it.
+pub fn report(m: &Mismatch, dump: &Dump, dir: &Path, nova: &Path, selfhost: &Path) -> String {
+    let file = m.file.as_deref().unwrap_or("(before the first file)");
+    let shown = |s: &Option<String>| match s {
+        Some(s) => s.clone(),
+        None => "(nothing: the output ended)".to_string(),
+    };
+    let at = if m.dump_line == 0 {
+        "at its header: one side's output ended before it".to_string()
+    } else {
+        let at_source =
+            m.s0.as_deref()
+                .or(m.n.as_deref())
+                .and_then(|line| source_line(dir, file, line))
+                .map(|l| format!(", source line {l}"))
+                .unwrap_or_default();
+        format!("at line {} of its dump{at_source}", m.dump_line)
+    };
+    let (s0_args, n_args) = (dump.s0.join(" "), dump.n.join(" "));
+    let s0_label = format!("S0 (nova {s0_args}):");
+    let n_label = format!("N  (selfhost {n_args}):");
+    let width = s0_label.len().max(n_label.len());
+    format!(
+        "the dumps differ in {file}, {at}\n\
+         \x20 {s0_label:<width$} {s0}\n\
+         \x20 {n_label:<width$} {n}\n\
+         reproduce, in {dir}:\n\
+         \x20 {nova} {s0_args} {file} > s0.txt\n\
+         \x20 {selfhost} {n_args} {file} > n.txt\n\
+         \x20 diff s0.txt n.txt\n\
+         and add the file to crates/nova-cli/tests/selfhost/cases/ once it is fixed",
+        s0 = shown(&m.s0),
+        n = shown(&m.n),
+        dir = dir.display(),
+        nova = nova.display(),
+        selfhost = selfhost.display(),
+    )
+}
diff --git a/crates/nova-cli/tests/selfhost/fuzz.rs b/crates/nova-cli/tests/selfhost/fuzz.rs
new file mode 100644
index 0000000000000000000000000000000000000000..d1f41aa4085677007dd32740029d4892d79a631b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/fuzz.rs
@@ -0,0 +1,418 @@
+//! The differential fuzzer's inputs (spec
+//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+//! §3.5, ADR 0051): a splitmix64 PRNG, so that no crate is needed and a
+//! seed names the same soup on every system, and the generators. A seed's
+//! mutations are the same on every system too for the same corpus files,
+//! which `selfhost.rs` picks from their sorted list and reads with each
+//! `\r\n` as `\n`; an untracked local `.nova` file joins the corpus, and
+//! changes them.
+//!
+//! 1. [`soup`]: lexical soup, as the 5.0 spike's `gen.py` made it, with
+//!    more atoms and an unterminated tail now and then, weighted so that
+//!    all seven lex errors occur among the fixed seeds' inputs.
+//! 2. [`mutate`]: a corpus file, lexed by S0's lexer into its tokens'
+//!    spans, then edited a few times: a whole token deleted, duplicated,
+//!    swapped with another or inserted elsewhere, or one ASCII byte
+//!    replaced. Most of the result stays well formed, so a lexer, and in
+//!    5.1b a parser, gets far into it.
+//! 3. Deep nesting is 5.1b's (spec §3.5 item 3, §4.4): a `deep(rng, kind,
+//!    depth) -> String` here, beside these two, whose inputs `selfhost.rs`
+//!    hands to `agree_on_inputs` as it does theirs.
+//!
+//! Beside them, [`edges`]: fixed inputs that no file under `cases/` can
+//! hold as it is, such as a lone `\r` or a NUL.
+
+use nova_diagnostics::FileId;
+
+/// splitmix64 (Steele, Lea and Flood, 2014): one `u64` of state, and an
+/// output that passes BigCrush, which is plenty for choosing atoms.
+pub struct SplitMix64(u64);
+
+impl SplitMix64 {
+    pub fn new(seed: u64) -> Self {
+        Self(seed)
+    }
+
+    pub fn next_u64(&mut self) -> u64 {
+        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
+        let mut z = self.0;
+        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
+        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
+        z ^ (z >> 31)
+    }
+
+    /// A number in `0..n`; `n` must not be 0.
+    pub fn below(&mut self, n: usize) -> usize {
+        (self.next_u64() % n as u64) as usize
+    }
+
+    /// `true` once in `n` times, on average.
+    pub fn one_in(&mut self, n: usize) -> bool {
+        self.below(n) == 0
+    }
+
+    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
+        &items[self.below(items.len())]
+    }
+}
+
+/// A backslash, so that no escape spelled with one is written in this file
+/// as a Rust escape by mistake.
+const BS: &str = "\\";
+
+/// The soup's atoms: `gen.py`'s, then the ones 5.1a adds.
+fn atoms() -> Vec<String> {
+    let mut atoms: Vec<String> = "\"${}'r# \n\t/*.eExXbBoO_u+-=<>&|!^%~@?:;,()[]0123456789aZ"
+        .chars()
+        .map(String::from)
+        .collect();
+    atoms.push(BS.to_string());
+    let words: &[&str] = &[
+        // gen.py's
+        "é",
+        "€",
+        "😀",
+        "\r",
+        "\0",
+        "\x7f",
+        "`",
+        "0x",
+        "0b",
+        "0o",
+        "1e",
+        "1.5",
+        "1..2",
+        "0x7FFFFFFFFFFFFFFF",
+        "9223372036854775807",
+        "9223372036854775808",
+        "0xFFFFFFFFFFFFFFFFF",
+        "///",
+        "////",
+        "/*",
+        "*/",
+        "//",
+        "r#\"",
+        "\"#",
+        "r\"",
+        "'é'",
+        "let",
+        "fn",
+        "Self",
+        "self",
+        "match",
+        "${",
+        "\"a\"",
+        "1_000",
+        "1e+5",
+        "1.5e-3",
+        "00.5",
+        "007",
+        "1_.5",
+        "1._5",
+        "0.1e",
+        "..=",
+        "<<=",
+        ">>=",
+        "\"${f(R { v: 1 })}\"",
+        "{",
+        "}",
+        // 5.1a's: line endings, numbers, tuple indexes, raw strings, holes
+        "\r\n",
+        "\r\n",
+        "0_81.5",
+        "t.1.2",
+        ".0.1",
+        "x.007.5",
+        "t.1.99999999999999999999",
+        "1e400",
+        "1_0.5",
+        ".5",
+        "1.0e10",
+        "0b2",
+        "0o9",
+        "0x_1",
+        "99999999999999999999",
+        "0b11111111111111111111111111111111111111111111111111111111111111111",
+        "0o7777777777777777777777",
+        "r##\"",
+        "\"##",
+        "r#x",
+        "r#",
+        "r##",
+        "\"${\"${",
+        "\"${x}\"",
+        "'''",
+        "'a",
+        "async",
+        "await",
+        "record",
+        "where",
+        "with",
+        "is",
+        "/// doc ",
+        "/* a\nb */",
+        // A Windows checkout's line endings inside every kind of token.
+        "\"a\r\nb\rc\"",
+        "r#\"a\r\nb\r\"#",
+        "/// doc\r",
+        "/// doc \r\r\n",
+        "// c\r",
+        "/* \r\n */",
+        "'\r'",
+    ];
+    atoms.extend(words.iter().map(|s| s.to_string()));
+    // Escapes, built from `BS`.
+    for tail in [
+        "u{",
+        "u{41}",
+        "u{D800}",
+        "u{110000}",
+        "u{+41}",
+        "u{}",
+        "u{10FFFF}",
+        "u{0000000041}",
+        "u{-1}",
+        "u{fffffffff}",
+        "u",
+        "0",
+        "\"",
+        "é",
+        "q",
+        "n",
+        "$",
+        "\n",
+    ] {
+        atoms.push(format!("{BS}{tail}"));
+    }
+    atoms.push(BS.repeat(2));
+    for tail in ["n", "x", "q", "'", "u{41}", "\n"] {
+        atoms.push(format!("'{BS}{tail}'"));
+    }
+    // Characters that are whitespace to Unicode but not to Nova: no-break
+    // space, ideographic space, next line, line separator, and a byte order
+    // mark.
+    for code in [0xA0, 0x3000, 0x85, 0x2028, 0xFEFF] {
+        let c = char::from_u32(code).expect("a character");
+        atoms.push(c.to_string());
+        atoms.push(format!("///x{c}"));
+    }
+    atoms
+}
+
+/// Endings that leave something open at the end of the file: what
+/// `UnterminatedInterpolation`, `UnterminatedBlockComment`,
+/// `UnterminatedString` and `InvalidNumberLiteral` need, and random soup
+/// rarely makes.
+const TAILS: [&str; 12] = [
+    "\"${",
+    "\"${ x",
+    "\"a${\"${",
+    "\"${ { }",
+    "/* never closed",
+    "\"open",
+    "r#\"open",
+    "r#",
+    "'",
+    "\"esc\\",
+    "// a line comment ending in a carriage return\r",
+    "/// a doc comment ending in a carriage return\r",
+];
+
+/// One input of lexical soup: up to 200 atoms, and once in five an
+/// unterminated tail.
+pub fn soup(rng: &mut SplitMix64, atoms: &[String]) -> String {
+    let mut out = String::new();
+    for _ in 0..rng.below(201) {
+        out.push_str(rng.pick(atoms));
+    }
+    if rng.one_in(5) {
+        out.push_str(rng.pick(&TAILS));
+    }
+    out
+}
+
+/// `count` inputs of soup from `seed`.
+pub fn soups(seed: u64, count: usize) -> Vec<String> {
+    let atoms = atoms();
+    let mut rng = SplitMix64::new(seed);
+    (0..count).map(|_| soup(&mut rng, &atoms)).collect()
+}
+
+/// Bytes a byte flip writes: ones that open, close or change a token.
+const FLIPS: &[u8] = b"\"'{}$\\/*\nr#0123456789._eEx+-=<>()[];,: \t\r\0";
+
+/// Tokens an insertion can write when the file has none of its own.
+const SPARE: [&str; 8] = ["(", "}", "\"", "${", "let", "0x", "1.5", "/*"];
+
+/// `source`, edited by token and by byte (module docs).
+pub fn mutate(rng: &mut SplitMix64, source: &str) -> String {
+    let (tokens, _) = nova_lexer::lex(source, FileId::DUMMY);
+    // Each token, but `Eof`, with the text before it; then the text after
+    // the last one.
+    let mut pieces: Vec<(String, String)> = Vec::new();
+    let mut prev = 0usize;
+    for t in tokens.iter() {
+        if t.value == nova_lexer::Token::Eof {
+            break;
+        }
+        let (start, end) = (t.span.start as usize, t.span.end as usize);
+        let start = start.max(prev);
+        let end = end.max(start);
+        pieces.push((
+            source[prev..start].to_string(),
+            source[start..end].to_string(),
+        ));
+        prev = end;
+    }
+    let tail = source[prev..].to_string();
+
+    for _ in 0..1 + rng.below(8) {
+        let n = pieces.len();
+        match rng.below(5) {
+            0 if n > 0 => {
+                // Delete a token, keeping the space before it.
+                let i = rng.below(n);
+                pieces[i].1.clear();
+            }
+            1 if n > 0 => {
+                let i = rng.below(n);
+                let copy = (" ".to_string(), pieces[i].1.clone());
+                pieces.insert(i + 1, copy);
+            }
+            2 if n > 1 => {
+                // Swap two tokens' texts; a token swapped with itself stays.
+                let (i, j) = (rng.below(n), rng.below(n));
+                let text = pieces[i].1.clone();
+                pieces[i].1 = std::mem::replace(&mut pieces[j].1, text);
+            }
+            3 => {
+                let text = if n > 0 {
+                    pieces[rng.below(n)].1.clone()
+                } else {
+                    rng.pick(&SPARE).to_string()
+                };
+                let gap = if rng.one_in(2) { "" } else { " " };
+                pieces.insert(rng.below(n + 1), (gap.to_string(), text));
+            }
+            _ => {}
+        }
+    }
+
+    let mut out = String::with_capacity(source.len() + 64);
+    for (gap, text) in &pieces {
+        out.push_str(gap);
+        out.push_str(text);
+    }
+    out.push_str(&tail);
+
+    // A few single-byte replacements, of ASCII bytes only, so the text stays
+    // UTF-8.
+    let mut bytes = out.into_bytes();
+    if !bytes.is_empty() {
+        for _ in 0..rng.below(4) {
+            let at = rng.below(bytes.len());
+            if bytes[at].is_ascii() {
+                bytes[at] = *rng.pick(FLIPS);
+            }
+        }
+    }
+    String::from_utf8(bytes).expect("only ASCII bytes were replaced")
+}
+
+/// The edge inputs: hand-made, each a place where the two lexers could
+/// part, made here rather than kept under `cases/` because a file there
+/// could not hold it as it is: a checkout on Windows turns `\n` into
+/// `\r\n`, and the repository's text holds no line ending in `\r`, no NUL
+/// and no backslash-u escape.
+pub fn edges() -> Vec<String> {
+    let c = |code: u32| char::from_u32(code).expect("a character").to_string();
+    let mut out: Vec<String> = [
+        // No text, a byte order mark alone and before code, no final line
+        // break.
+        "",
+        "let a = 1",
+        // A lone `\r` and `\r\n` between tokens, in strings, raw strings,
+        // holes, chars, block comments, and ending doc and line comments,
+        // the last at the end of the file.
+        "let a = 1\rlet b = 2\r",
+        "let a = 1\r\nlet b = 2\r\n",
+        "\r",
+        "\r\n\r\n",
+        "let s = \"a\r\nb\rc\"\r\n",
+        "let s = r#\"a\r\nb\r\"#\r\n",
+        "\"${\r\nx\r}\"",
+        "'\r' '\n'",
+        "/* a\r\nb */ x\r",
+        "/// doc\r\nfn f() {}\r\n/// tail \r\r\n///\r\n",
+        "/// doc\r",
+        "let a = 1 // end\r",
+        "// x\r\r",
+        "x //\r",
+        // NUL, outside and inside every kind of token.
+        "let a\0 = \"x\0y\" // c\0\n/* \0 */ 'a\0 '\0' r\"\0\" /// \0\n",
+        // Endings that leave something open at the last byte.
+        "0x",
+        "0b",
+        "0o",
+        "0x1",
+        "1e",
+        "1e+",
+        "1.",
+        "1.5e",
+        "'",
+        "'a",
+        "'\\",
+        "r",
+        "r#",
+        "r##",
+        "\"",
+        "\"$",
+        "\"${",
+        "\"${ {",
+        "/*",
+        "/",
+        "//",
+        "///",
+        "\"esc\\",
+    ]
+    .iter()
+    .map(|s| s.to_string())
+    .collect();
+    out.push(c(0xFEFF));
+    out.push(format!("{}let a = 1\n", c(0xFEFF)));
+    // Every escape, then each kind of Unicode escape, built from `BS`: the
+    // largest, past it, the surrogates, a leading `+` and `-`, empty, past
+    // `u32`, seventeen digits whose value wraps an `Int` into a character,
+    // and a `"` inside the braces.
+    out.push(format!(
+        "\"{BS}n{BS}t{BS}r{BS}\"{BS}{BS}{BS}0{BS}q{BS}$\" '{BS}n' '{BS}t' '{BS}r' '{BS}{BS}' '{BS}'' '{BS}0' '{BS}\"'"
+    ));
+    for hex in [
+        "10FFFF",
+        "110000",
+        "D800",
+        "DFFF",
+        "0",
+        "+10",
+        "-1",
+        "",
+        "ffffffff",
+        "100000000",
+        "10000000000000041",
+        "e9\"x",
+        "41",
+    ] {
+        out.push(format!("\"{BS}u{{{hex}}}\" '{BS}u{{{hex}}}'"));
+    }
+    // A doc comment ending in each character Rust's `trim_end` drops,
+    // Unicode's White_Space, but `\n`, which ends the comment, and `\r`,
+    // above; then in five it keeps: U+180E, U+200B, U+FEFF, DEL and U+0001.
+    let white_space = [0x09, 0x0B, 0x0C, 0x20, 0x85, 0xA0, 0x1680]
+        .into_iter()
+        .chain(0x2000..=0x200A)
+        .chain([0x2028, 0x2029, 0x202F, 0x205F, 0x3000]);
+    for code in white_space.chain([0x180E, 0x200B, 0xFEFF, 0x7F, 0x01]) {
+        out.push(format!("/// d{}\n///{}", c(code), c(code)));
+    }
+    out
+}
````

### Task 4: ADR 0051 and the records

Spec §8; decisions 13 to 26 and 35 to 48.

**Files:**
- Create: `docs/adr/0051-the-self-hosted-lexer-and-the-differential.md`
- Modify: `CHANGELOG.md`, `docs/phase-5-plan.md`,
  `nova-spec/00-MASTER-SPEC.md`, `nova-spec/11-PARSER.md`

- [ ] **Step 1: The records.** `patch_block t4-code`:
  - **ADR 0051,** the self-hosted lexer and the differential: its context;
    its decisions, `selfhost/`'s layout with the tests in `src/`, the
    command line, S0's hidden flag and the dump's format with `IOERR`'s
    kinds, the harness with its corpus walk, Windows-endings corpus, cases
    and edge inputs, its reports, its kept build and its time limit, the
    fuzzer with mutations the same on every system for the same files, the
    rule that a change to the tokens changes both compilers in one pull request, `nova fmt` over
    `selfhost/`, and Windows and macOS as reasoned, S0's stack among
    them; its consequences, the empty path, a file name that is not UTF-8
    and no file, S0's stack overflow on a long decimal number, a known
    bug for 5.1b's big-stack thread, and the two lexers' speed with the
    debug runtime among them; its references.
  - **`CHANGELOG.md`,** Unreleased: the heading names 5.1a; Added (the
    self-hosted lexer and the differential; the hidden
    `nova parse --dump-tokens`, and `nova parse` taking one file without
    it as before); Changed (CI's `nova fmt --check` over `selfhost/`).
  - **Dated notes,** each dated 2026-10-30, naming the branch
    `claude/loving-archimedes-frtf5q-51a` and citing the spec:
    `docs/phase-5-plan.md`'s 5.1 (two parts, and its three questions
    answered); `00-MASTER-SPEC.md`'s Phase 5 (step 1 ported);
    `11-PARSER.md` §4 (the token dump).
- [ ] **Step 2: Read them against the code.** Each record's claim against
  the tree: the ADR's decisions against Tasks 1 to 3 and this plan's
  decisions; the CHANGELOG's entries against the patches; every figure
  (92 kinds, 300 corpus files, twelve cases, the seeds and counts, about
  10 s) against this plan's; the branch name in each note. Any difference
  goes in the ledger and to Task 5's review, which fixes it in its one
  commit, so that this task's tree stays the replay's.
- [ ] **Step 3: The checks,** as the conventions list them, after the
  build.
- [ ] **Step 4: Commit**
  `docs: ADR 0051, the self-hosted lexer, and the notes for 5.1a`.
- [ ] **Step 5: Run,** on the commit.
  - `git diff --check HEAD~1 HEAD`. Expected: nothing.
  - `git diff --stat e8d10c4f HEAD -- . ':!docs/superpowers/plans'`.
    Expected: nothing: but for this plan's file, which the prototype does
    not hold, the tree is the prototype's last, `e8d10c4f`. A difference
    stops the task: record it in the ledger.

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 0628fc27cc297968fae8f374443080a992a89c46..873aeeff83fe3c93a71987d6e96633f398ca271c 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -12,7 +12,10 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
 Phase 5.0a, the small language and CI (spec
 `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
 §3; ADR 0048, Phase 5's scope and backend), 5.0b, tuples and
-patterns (§4; ADR 0049), and 5.0c, std (§5; ADR 0050).
+patterns (§4; ADR 0049), and 5.0c, std (§5; ADR 0050). Phase 5.1a, the
+self-hosted lexer and the differential (spec
+`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+§3; ADR 0051).
 
 ### Added
 
@@ -147,6 +150,24 @@ patterns (§4; ADR 0049), and 5.0c, std (§5; ADR 0050).
   path, `nova` itself under `nova run`. Native only: on WASM, naming one
   is E0092. The four names are in every module's scope, and a program's
   own shadows them (ADR 0050).
+- **The self-hosted lexer, `selfhost/`:** N's first module, a Nova
+  package whose `selfhost tokens FILE…` lexes each file exactly as
+  `nova-lexer` does, quirks included, and prints its token dump. A
+  differential test, `crates/nova-cli/tests/selfhost.rs`, builds it with
+  `nova build` and compares its dumps with S0's, byte for byte, on every
+  `.nova` file of the repository, on fixed cases, and on a fuzzer's soup
+  and mutated corpus files from fixed seeds, on every system, the corpus
+  also with a Windows checkout's `\r\n` line endings;
+  `NOVA_SELFHOST_FUZZ=SEED:COUNT` searches longer. A change to the tokens
+  now changes both lexers in one pull request (ADR 0051).
+- **`nova parse --dump-tokens FILE…`, hidden:** each file's tokens with
+  their spans, line-break and plain-decimal marks, then its lex errors and
+  comments, one per line, after a header `== FILE`; a file that cannot be
+  read is `IOERR` and its kind, and the exit code 2. The text comes from
+  `nova_lexer::dump_tokens`. It is absent from `--help` and carries no
+  stability promise. `nova parse` without it still takes one file, and
+  refuses a second in clap's words as before; the further files are a
+  hidden argument only the flag reads (ADR 0051).
 
 ### Changed
 
@@ -204,6 +225,8 @@ patterns (§4; ADR 0049), and 5.0c, std (§5; ADR 0050).
   `0`, by the source text, so `t.1.10` is `(t.1).10`. A float anywhere
   else, `0..1.5` included, is unchanged. `Tokens::plain_decimal` says
   whether an integer is written in plain decimal digits.
+- **CI:** `nova fmt --check` covers `selfhost/` with `std/` and
+  `examples/` (ADR 0051).
 - **CI:** the `test` job's Linux leg runs on `ubuntu-24.04`, installs
   Ubuntu's `clang-18` and asserts that it is 18.1.3. With
   `NOVA_REQUIRE_CLANG=1` and `NOVA_CLANG=clang-18`, a new test,
diff --git a/docs/adr/0051-the-self-hosted-lexer-and-the-differential.md b/docs/adr/0051-the-self-hosted-lexer-and-the-differential.md
new file mode 100644
index 0000000000000000000000000000000000000000..b918b802e0b63288b6105f38e4810adb07a2c9f5
--- /dev/null
+++ b/docs/adr/0051-the-self-hosted-lexer-and-the-differential.md
@@ -0,0 +1,313 @@
+# ADR 0051 — The self-hosted lexer and the differential
+
+## Status
+
+Accepted (2026-10-30). Branch `claude/loving-archimedes-frtf5q-51a`
+(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+§3). The two parts of 5.1, the Rust fuzzer with fixed seeds, floats dumped
+by their bits, and the differential on all three systems are the user's
+decisions of 2026-10-30 (the spec's decisions 1, 5 and 7, and its
+recommendation on floats); the rest is what the spec left to the plan.
+
+## Context
+
+Phase 5 writes N, the Nova compiler, in Nova, in `selfhost/`
+(`docs/phase-5-plan.md`; ADR 0048). Its first module is the lexer. S0's
+lexer, `nova-lexer`, is 1,002 code lines of Rust over `logos` 0.14, whose
+longest-match rules decide some of its behaviour: an integer that
+overflows is an unexpected character, and `r#` with no `"` is the only
+invalid number literal. From 5.1a on the lexer exists twice, and nothing
+but a test can keep the two equal. The 5.0 spike compared a port with S0
+by a dump on each side and a random-input generator, and the generator
+found the one bug the corpus could not.
+
+## Decision
+
+1. **`selfhost/` is a Nova package,** `selfhost` 0.1.0, edition 2026, no
+   dependencies, with flat modules in `src/`:
+   - `token.nova`: the 92 token kinds, named as `Token`'s variants, the
+     `Span` and `Spanned` records (byte offsets, no `FileId`), and each
+     kind's `description()`, word for word. No dump prints a description:
+     an `@test` checks thirteen kinds' against `token.rs`'s, and 5.1b's
+     parser messages, which hold them, exercise every one through the
+     differential;
+   - `lexer.nova`: `lex`, `lex_with_comments`, `Tokens` with its two
+     marks, the lexer, the tuple-index split, comments, and `LexError`
+     with its `Display`, function for function with `lib.rs`, `tokens.rs`
+     and `error.rs`, each function naming the Rust it ports. `logos`
+     becomes a hand-written scanner with its rules: the longest match, a
+     keyword over an identifier of the same text, and, where no regex
+     matches or a callback refuses, `UnexpectedCharacter` of the first
+     character, then one character on. Its classes are ASCII but two: the
+     char literal's `[^'\\]`, any character but those two, and `.`, any
+     but `\n`; and the doc comment's `trim_end`, which drops Unicode's
+     White_Space. Keywords are found by first byte and length before any
+     comparison, since a string literal allocates each time it is
+     evaluated;
+   - `dump.nova`: the token dump (decision 3);
+   - `main.nova`: the command line (decision 2);
+   - `lexer_tests.nova`: N's own `@test`s of the lexer's edges, each
+     against S0's dump of the same source, and of the descriptions. They
+     are in `src/`, imported by
+     `main.nova`, not in `tests/` as the spec planned: a `tests/` module
+     sees only the files beside it and the package's `src/lib.nova`, and
+     Nova has no re-exports, so a test there cannot reach `lexer.nova`.
+
+   N is 989 code lines for what the Rust's 1,002 do (`token.nova` and
+   `lexer.nova`), and 176 for the dump, whose 92-arm kind name the Rust
+   takes from `Debug`.
+2. **The command line is `selfhost tokens FILE…`.** For each file, in
+   order, a header line `== FILE`, the path as given, then the file's
+   dump. Lex errors are part of the dump and leave the exit code 0. A file
+   that cannot be read gives `== FILE` then `IOERR KIND`, and the rest
+   are dumped before the exit code 2. No file at all is refused with a
+   usage line and the exit code 2, as S0's clap refuses it. `main` is an
+   `async fn`, since `std/fs` reads only so.
+3. **S0 prints the same dump, by a hidden flag: `nova parse
+   --dump-tokens FILE…`** (`hide = true`; no stability promise).
+   `nova_lexer::dump_tokens(source)` writes one file's dump, next to the
+   lexer it describes; the command adds the headers and `IOERR`. Without
+   the flag, `nova parse` takes one file as before: the further files are
+   a hidden list, `more`, that only the flag reads, and a second file
+   without it is refused with clap's own "unexpected argument" error, in
+   the words clap gave before the list existed and built as clap builds
+   it, so a terminal shows the same colours. Its usage line names the
+   command as clap does: the file name of the first argument, then
+   `parse`, so a renamed `nova` names itself; or `parse` alone when that
+   argument has no file name in UTF-8 (`""`, `/`, `..`, bytes that are
+   not UTF-8). `--help` is unchanged.
+   The flag conflicts with `--ast` and `--check`. Per file, every line
+   ending in `\n`:
+   - **a line per token, `Eof` last:** `start end Kind[ payload][ NL][ PD]`.
+     `start` and `end` are the span's byte offsets in decimal. `Kind` is
+     the Rust `Token` variant's name. The payloads:
+     - `Int`: the value in decimal;
+     - `Float`: the 16 lowercase hex digits of `f64::to_bits`
+       (`Float.to_bits` in N), so no decimal rendering has to agree;
+     - `Char`: the code point in decimal;
+     - `StrPart`, `RawStr`, `Ident`, `DocComment`: the text in double
+       quotes, each byte below 0x20, 0x7F, `"` and `\` written `\xHH`
+       with two lowercase hex digits, every other byte, so every
+       non-ASCII character, as it is;
+     - no other kind has one.
+
+     `NL` follows when `Tokens::line_break_before` holds, then `PD` when
+     `Tokens::plain_decimal` does;
+   - **a line per lex error, in the order found:** `ERR start end
+     message`, the message being the error's `Display`, with each byte
+     below 0x20 and 0x7F written `\xHH`, so that an escape of a line
+     break stays on its line; `"` and `\` stay as they are, since the
+     message is not quoted;
+   - **a line per comment, in source order:** `COMMENT start end Line` or
+     `COMMENT start end Block`;
+   - **`IOERR KIND`,** in place of all three for a file that cannot be
+     read: the kind as `std/io`'s `IoErrorKind` names it, `NotFound`,
+     `PermissionDenied`, `AlreadyExists`, `InvalidData`, `Interrupted`,
+     `TimedOut`, `ConnectionRefused` or `Other`. S0 maps Rust's
+     `io::ErrorKind` to those words, every other kind to `Other`, as N's
+     runtime does.
+
+   The errors and comments come from `lex_with_comments`, whose tokens and
+   marks are `lex`'s by construction: both run `tokenize` and
+   `Tokens::marked`.
+4. **The differential is `crates/nova-cli/tests/selfhost.rs`,** with
+   `tests/selfhost/compare.rs` and `tests/selfhost/fuzz.rs`, a `cargo
+   test` on Linux, macOS and Windows:
+   - N is built once per test binary, through a `OnceLock`, by `nova
+     build -o` into a fixed directory under the system temp directory,
+     from `selfhost/` in place: `-o` writes nothing into the package, and
+     neither does `nova test`. No `clang` is needed. The `OnceLock` holds
+     the build's result, so a failed build is made once and reported by
+     every test that needs N.
+   - Every process a test starts, `nova build`, `nova test` or one side
+     on one batch, has two minutes. Its stdout and stderr are read on
+     threads of their own, so a full pipe cannot stall it; past the limit
+     it is killed, and the test fails naming the batch and its first file,
+     so a lexer that loops forever cannot hold CI for hours. After a
+     process exits, its pipes are read to their end within what is left of
+     the two minutes: a process it started that still holds them is
+     reported, not waited for. A process that cannot be waited for is
+     killed before it is reported. `Child::kill` reaches the process
+     alone, not one it started: an N `@test` that looped would leave its
+     test process running, orphaned, when `nova test` is killed, though
+     that process holds none of the harness's pipes, since `nova test`
+     reads its output itself. Each takes a second or two on Linux; CI's
+     Windows runners are several times slower.
+   - **The corpus** is every `.nova` file under the repository, walked
+     from the source tree, not listed by `git`, so an untracked file is
+     lexed too, skipping `target/`, `node_modules/`, directories whose
+     names begin with `.`, symbolic links, `selfhost/` and the fixed cases,
+     sorted; then N's own sources. 300 files at 5.1a, the two that do not
+     parse among them. The corpus runs a second time as a Windows checkout
+     writes it, each `\n` as `\r\n`, on every system.
+   - Files go 50 to a process through S0 and N, and the two outputs must be
+     identical byte for byte, compared as bytes, so output that is not
+     UTF-8 differs where its bytes do, with the same exit code. On a
+     difference the
+     test names the file, the line of its dump and of its source, both
+     lines, and the two commands that reproduce it; where one side's
+     output ends before a file's header, it names that file; where the
+     two lines differ only in a final line break, the one without it is
+     marked `(no line break)`. Every report
+     gives both exit codes and the start of each side's stderr, so a side
+     that crashed or printed nothing says so rather than showing a
+     confusing difference.
+   - **The fixed cases** in `tests/selfhost/cases/` run the same way:
+     every input that ever found a difference, starting with the spike's
+     `0_81.5`, then the probes of numbers, tuple indexes, nested holes,
+     raw strings, chars, punctuation, keywords, `$` and braces, slashes and
+     an open block comment. They are `.nova` files, and `nova-fmt`'s corpus
+     gate skips their directory, since many are not Nova on purpose.
+   - **The edge inputs,** in `fuzz.rs`, are the cases no file there can
+     hold as it is: a lone `\r` or `\r\n` in every kind of token and
+     ending a doc or line comment at the end of the file, NUL inside and
+     outside tokens, an empty file, a byte order mark, a file without a
+     final line break, an opening left at the last byte, every escape, the
+     Unicode escapes at and past their limits (seventeen hex digits among
+     them, which wrap an `Int`), and a doc comment ending in each
+     White_Space character, but `\n`, which ends it, and `\r`, above, and
+     in five that are not: U+180E, U+200B, U+FEFF, 0x7F and 0x01. A
+     Windows checkout
+     would rewrite such a file's line endings, and the repository's text
+     holds no line ending in `\r`, no NUL and no backslash-u escape.
+   - Other tests: the flag is absent from `nova parse --help` and from the
+     error for a second file, and conflicts with `--ast` and `--check`;
+     S0's dump has a header per file and `IOERR` for a missing one, the
+     rest dumped, exit code 2; unreadable files, a missing one and one
+     that is not UTF-8, are reported alike; the report names the file and
+     line, run on a deliberately wrong dump, and a side that fails is
+     reported with its exit code and stderr; both sides refuse no file; a
+     process past its limit is killed and its batch named, and a pipe
+     still open at the limit is reported; a failed build is made once, and
+     N is one however often asked; a missing final line break is marked;
+     a seed's mutations are the same from a Windows checkout; a renamed
+     `nova` names itself in the second file's error, and on Unix a first
+     argument with no file name gives `parse` alone;
+     `nova-lexer`'s tests hold one dump with every field, a float's bits
+     among them in lowercase hex; and `nova test` in `selfhost/` passes as
+     many tests as N's sources hold, so a run of none fails.
+5. **The fuzzer is Rust, in the test, with a splitmix64 PRNG** and no new
+   crate:
+   - **soup:** the spike's atoms, with `\r` and `\r\n` alone and inside
+     strings, raw strings, chars, block comments and ending doc and line
+     comments, NUL, overflowing numbers in every radix, non-ASCII text and
+     Unicode spaces, tuple indexes, raw strings, nested holes and bad
+     escapes, and once in five an unterminated tail (a hole, a string, a
+     raw string, a block comment, `r#`, a doc or line comment ending in
+     `\r`). The test asserts that all seven lex errors occur among the
+     fixed seeds' inputs;
+   - **mutated corpus files:** a corpus file is lexed by `nova-lexer`,
+     already a dependency of `nova-cli`, into its tokens' spans; one to
+     eight edits delete, duplicate, swap or insert whole tokens, then up
+     to three ASCII bytes are replaced by bytes that open or close
+     tokens. The files are picked from their sorted list and read with
+     each `\r\n` as `\n`, so a seed gives the same inputs on every
+     system, a Windows checkout included; the soup's need no files at
+     all. An untracked local `.nova` file joins the corpus, and changes
+     the mutations;
+   - **deep nesting** is 5.1b's, and plugs in beside them: the harness
+     compares any inputs through `agree_on_inputs`, and any pair of dumps
+     through a `Dump` (`TOKENS` now; the AST dump in 5.1b).
+
+   CI runs soup seeds 1, 2 and 3, 2,000 inputs each, and mutation seeds
+   101, 102 and 103, 250 inputs each: the whole file takes about 10 s on a
+   4-core Linux machine, building N included. `NOVA_SELFHOST_FUZZ=SEED:COUNT`
+   runs COUNT more of each from SEED, locally.
+6. **A change to the tokens changes both compilers in one pull request**
+   (phase plan decision 6): `nova-lexer`, `dump.rs`, and `selfhost/`'s
+   `token.nova`, `lexer.nova` and `dump.nova`, with the differential
+   green. The dump changes with the tokens and promises nothing else.
+7. **`nova fmt --check` covers `selfhost/`** in CI's step with `std` and
+   `examples`.
+8. **Windows and macOS are reasoned from the code, and pinned on Linux
+   where a test can.**
+   - **Line endings:** GitHub's Windows runners check out with
+     `core.autocrlf`, so the corpus there holds `\r\n`. Both lexers treat
+     `\r` alike everywhere: whitespace between tokens; a line break for
+     `NL`; text in strings and raw strings; dropped by a doc comment's
+     `trim_end`; and left out of a line comment's span when it is the last
+     byte before the `\n` or the end of the file, one `\r` only. The
+     corpus's second run with `\r\n`, the edge inputs and the soup check
+     it on every system.
+   - **Paths:** the harness passes each file relative to the directory it
+     runs both sides in, its parts joined by `/`, and both print the path
+     as given: S0 through `Path::display`, N its argument. Neither
+     canonicalizes, so no `\\?\` prefix appears on one side only.
+   - **The executable:** `nova build -o` writes the path it is given, and
+     the harness names it `selfhost` plus `EXE_SUFFIX`, `selfhost.exe` on
+     Windows.
+   - **Files that cannot be read:** a file that is not UTF-8 is `IOERR
+     InvalidData` on both sides (Rust's `read_to_string` and `std/fs`'s);
+     a directory is `IOERR Other` on Linux, whose `IsADirectory` neither
+     side names, and the same kind on both elsewhere, since both map one
+     Rust error. A byte order mark is kept by both reads, and is an
+     unexpected character to both lexers.
+   - **The stack:** N's lexer is loops, not recursion, so Windows' 1 MB
+     main-thread stack does not bound it. S0's is not, as the tests build
+     it: `logos`'s generated code for a decimal number recurses once per
+     character, so a long one overflows debug S0's stack, at about 4,000
+     characters under 1 MB (Consequences). No generator makes such a
+     number. The depth limit, and the big-stack thread for S0, are 5.1b's.
+
+## Consequences
+
+- No difference was found while building: the corpus agreed at once, and
+  so did 40,000 inputs of a local search (`NOVA_SELFHOST_FUZZ=1000:20000`)
+  and the fixed seeds. Nor did the probes before the split, of `\r` in
+  every position, files with no text, a byte order mark or no final line
+  break, files that cannot be read, and the edges of numbers, tuple
+  indexes, escapes, holes, chars and comments; the interesting ones became
+  the fixed cases and the edge inputs. A later check's own search, about
+  50,000 inputs, found none either.
+- An empty path is refused by S0's clap and read as `IOERR NotFound` by N;
+  the harness never passes one.
+- A file name that is not UTF-8 differs too: S0 reads the file and dumps
+  it, under a header that shows the name lossily, while N, whose `args()`
+  are strings, gets the lossy name and prints `IOERR NotFound`. The
+  harness's names are its own, and UTF-8.
+- **A known S0 bug, S0's stack:** `logos`'s generated code for a decimal
+  number, digits, digits with `_`, or digits before `.5`, recurses once
+  per character, so debug S0, as the tests build it, overflows its stack
+  on a long one and aborts (`thread 'main' has overflowed its stack`,
+  exit code 134 on Linux). Under `ulimit -s 1024`, Windows' main-thread
+  size, a decimal literal of about 4,000 characters aborts it; under
+  Linux's 8 MB it survives 16,000 and aborts at 32,000. N lexes them all.
+  Hexadecimal, binary and octal numbers, identifiers, strings and
+  comments of 40,000 characters are fine on S0 under 1 MB. No generator
+  makes such a number and no fixed case holds one: 5.1b's big-stack
+  thread for S0's pipeline (spec §4.4), which `nova parse` and so its
+  `--dump-tokens` will run on, will cover it.
+- **Speed, measured with the debug runtime,** as the tests build N (not
+  5.1b's measurement with an optimized runtime): both lexers are
+  quadratic on a run of decimal digits that overflows, since `logos`'s
+  rule moves one character on and lexes the rest again; N takes 5.7 s on
+  16,000 digits, 23 s on 32,000 and 89 s on 64,000, and debug S0 7.3 s on
+  16,000. Otherwise both are linear, but N is about 10 to 15 times slower
+  than debug S0: 5 MB of `lexer.nova` repeated, 11.8 s against 1.1 s;
+  1 MB of `{`, 10.2 s against 0.83 s; 1 MB of bad escapes, 2.2 s against
+  0.12 s. The differential's inputs are small, and take about 10 s in
+  all.
+- `selfhost tokens` with no file printed nothing and exited 0, where S0's
+  clap refuses it with the exit code 2; N now refuses it too, with its
+  usage line, and the harness checks that both do.
+- N's tests run inside N's binary's module graph, so `nova build`
+  compiles them into `selfhost` too, unused.
+- `IOERR` cannot name a kind `std/io` lacks; such a failure reads `Other`
+  on both sides.
+- CI time grows by the build of N and about 10 s of comparison per system,
+  more on Windows. The counts are the knob.
+- 5.1b adds the AST dump, `selfhost ast` and `nova parse --dump-ast`, the
+  deep-nesting generator and the error cases (spec §4).
+
+## References
+
+- `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+  §§1–3, 6–9.
+- `docs/phase-5-plan.md` §4 (5.1, amended 2026-10-30) and decision 6.
+- `nova-spec/11-PARSER.md` §4 and `00-MASTER-SPEC.md`'s Phase 5, and their
+  notes of 2026-10-30.
+- `crates/nova-lexer/src/{lib,token,tokens,error,dump}.rs`;
+  `crates/nova-cli/src/cmd/parse.rs`; `selfhost/`;
+  `crates/nova-cli/tests/selfhost.rs` and `tests/selfhost/`.
+- ADR 0048, Phase 5's scope and backend; ADR 0050, the std a compiler
+  needs.
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index 9081bebd0f6c749c56b3d171e07eed384c6c7b6b..865e49112d5a7e682c6ea685949dd640b9ee983f 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -361,6 +361,25 @@ decision 1; ADR 0048), in place of the 5.0a and 5.0b below:
   - whether the shadowing of std's variant names gets a warning.
 
 ### 5.1 — The lexer, the parser and the AST
+
+**Amended 2026-10-30 (branch `claude/loving-archimedes-frtf5q-51a`):** 5.1
+is built as **two parts, in order,** by the user's decision of 2026-10-30
+(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+decision 1):
+- **5.1a, the lexer and the harness** (ADR 0051): the package
+  `selfhost/` and its lexer; a token dump on both sides, S0's behind the
+  hidden `nova parse --dump-tokens`; the differential test, with a Rust
+  fuzzer of soup and mutated corpus files on fixed seeds, on all three
+  systems;
+- **5.1b, the parser and the AST:** the AST and the parser in Nova; an AST
+  dump on both sides; parse errors compared completely, notes and newline
+  splits included; the depth limit, 128 levels, P0002, in both parsers;
+  the fuzzer's deep inputs; and the speed measured against S0.
+
+The three questions below are answered by that spec: the AST dump is an
+indented S-expression with spans (§4.3); the fuzzer mutates corpus files
+token by token (§3.5); and the depth is capped at 128 in both parsers, with
+S0's pipeline on a 64 MB stack (§4.4).
 - **N's first modules:** the tokens, a hand-written scanner, the AST,
   recursive descent with error recovery, and the P-codes.
 - **The dumps:** a token dump on both sides (the spike's format), and a
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 21164213079096c22026af442813af6d080f4568..9009339d9363b883f90b194cefa0115987fb3d5a 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -763,6 +763,16 @@ arrays, `OrderedMap`, whose order makes a stage's output the same on every
 run, and `std/process`'s `run`, `status`, `env` and `current_exe`, for the
 driver that runs `clang`.
 
+**Amended 2026-10-30 (branch `claude/loving-archimedes-frtf5q-51a`):** step
+1, the lexer, is ported, as 5.1a
+(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+§3; ADR 0051): the package `selfhost/`, whose `selfhost tokens FILE…` prints
+the token dump S0's hidden `nova parse --dump-tokens` prints, and a
+differential test that compares the two on every `.nova` file of the
+repository and on fuzzed inputs, on all three systems. From here a change to
+the tokens changes both compilers in one pull request. Step 2, the parser
+and the AST, is 5.1b.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/nova-spec/11-PARSER.md b/nova-spec/11-PARSER.md
index e015d7e47c39ffc0ae75288b807917fe8c5b4dcc..8e9a88217e4e13f918fd7c99bc929a6327f1b4f0 100644
--- a/nova-spec/11-PARSER.md
+++ b/nova-spec/11-PARSER.md
@@ -461,6 +461,21 @@ pub fn parse(tokens: &[Spanned<Token>], file: FileId)
 - Returns `Some(File)` even with errors when recovery succeeds
 - Returns `None` only on catastrophic failure (rare with chumsky)
 
+**Amended 2026-10-30 (branch `claude/loving-archimedes-frtf5q-51a`):** **the
+token dump**
+(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
+§3.3; ADR 0051, which writes the format out). `nova_lexer::dump_tokens(source)`
+renders one source's tokens, lex errors and comments as text, one per line:
+`start end Kind[ payload][ NL][ PD]` for each token, `Eof` last, with the
+`Token` variant's name, decimal `Int`s, a `Float`'s 16 hex digits of bits, a
+`Char`'s code point, and text in double quotes with control bytes, `"` and
+`\` written `\xHH`; then `ERR start end message` for each lex error; then
+`COMMENT start end Line|Block`. The hidden `nova parse --dump-tokens FILE…`
+prints it after a `== FILE` header per file, and the self-hosted lexer's
+`selfhost tokens FILE…` prints the same bytes. It carries no stability
+promise: it changes with the tokens, in both compilers at once. 5.1b adds an
+AST dump beside it.
+
 ---
 
 ## 5. Error Recovery
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
  target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-51a.txt" 2>&1
  grep '^test result' "$P/full-51a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-51a.txt"
  cargo test --locked --workspace -- --list > "$P/final-list-51a.txt" 2>&1
  python3 "$P/names.py" "$P/base-list-51a.txt" "$P/final-list-51a.txt"
  ```

  `names.py` compares Task 0's list with this one binary by binary, by
  name: a binary is its `Running` or `Doc-tests` line, its hash stripped,
  an integration test's prefixed with the crate whose unit tests cargo
  listed just before it, so two crates' `tests/tuples.rs` stay apart; a
  test is a line ending `: test`. It prints each list's binaries and
  tests, each binary added or removed with its tests, each binary whose
  tests changed with the names added and gone, and the net. Write it
  before the run, as the replay ran it (decision 43):

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
  - the suite, in 6 min 20 s: `2816 1 10`, the baseline's figures and
    21 more tests;
  - the one failure is Task 0's IPv6 test;
  - `names.py` prints `binaries: base 110, now 111` and
    `tests: base 2806, now 2827`: 2806 tests in 110 binaries before and 2827 in 111 after, one binary new, and
    `net added: 21`, 21 added and 0 gone:

    - `nova_lexer: tests/lexer_tests.rs (lexer_tests)`, 49 to 51:
      `the_token_dump_of_nothing_is_eof`;
      `the_token_dump_writes_every_field`;
    - `nova: tests/selfhost.rs (selfhost)`, new, 19 tests:
      `a_failed_build_is_made_once_and_reported_to_each_test`;
      `a_longer_search_from_the_environment`;
      `a_missing_final_line_break_is_marked`;
      `a_pipe_still_open_at_the_deadline_is_reported`;
      `a_process_past_its_limit_is_killed_and_its_batch_named`;
      `a_side_that_fails_is_reported_with_its_exit_code_and_stderr`;
      `both_sides_refuse_no_file`;
      `the_corpus_dumps_agree`;
      `the_corpus_dumps_agree_with_windows_line_endings`;
      `the_edge_inputs_agree`;
      `the_fixed_cases_agree`;
      `the_mismatch_report_names_the_file_and_line`;
      `the_mutated_corpus_agrees`;
      `the_mutations_are_the_same_with_windows_line_endings`;
      `the_selfhost_tests_pass`;
      `the_soup_agrees_and_holds_every_lex_error`;
      `the_token_dump_flag_is_hidden`;
      `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`;
      `unreadable_files_are_reported_alike`.
- [ ] **Step 2: CI's Linux leg, as CI runs it.** The `test` job's
  `ubuntu-24.04` leg runs the whole suite with the golden suite required,
  with CI's own flags, `--locked --workspace --all-features --no-fail-fast`:

  ```sh
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast > "$P/ci-51a.txt" 2>&1
  grep '^test result' "$P/ci-51a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/ci-51a.txt"
  grep -c 'release_golden_suite ... ok' "$P/ci-51a.txt"
  ```

  Expected, in 7 min 11 s: `2816 1 10`, Step 1's figures, the one failure the
  IPv6 test, and `1`: the golden suite ran and passed inside the whole run,
  as on CI, where Step 1's run skips it.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in
  42 s of wall time:
  `release golden: 147 passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)` and `1 passed`: unchanged, since 5.1a adds no runtime
  program.
- [ ] **Step 4: Parsing and checking every file the base tracks.** Two
  scripts, as the replay ran them (decision 43), compare the base's `nova`
  (Task 0's copy, `$P/nova-base-51a`) with the tree's on every `.nova`
  file `git ls-tree -r --name-only 25be9f3c` lists, 295 files, each run's
  output, both streams, and exit status compared byte for byte. Both run
  from the repository's root after the build, and read `$P`.
  - `parsecmp.sh`: `nova parse --check` and `nova parse --ast` of each
    file, the base's on the file as `25be9f3c` has it, from a
    `git archive 25be9f3c` unpacked in `$P/pc-src-51a`, the tree's on the
    tree's, each passed the path relative to its root:

```sh
cat > "$P/parsecmp.sh" <<'EOF'
#!/bin/bash
# parsecmp.sh: run from the repository's root, after the build, with $P set.
# For every `.nova` file `git ls-tree -r --name-only 25be9f3c` lists (the
# base's tracked files, so none 5.1a adds), `nova parse --check` and
# `nova parse --ast`: by the base's `nova` ($P/nova-base-51a, Task 0) on the
# file as 25be9f3c has it, from a `git archive 25be9f3c` unpacked in
# $P/pc-src-51a, and by the tree's `target/debug/nova` on the tree's file.
# Each run passes the path relative to its root and keeps its output, both
# streams, and its exit status; every pair must be byte-identical, since
# 5.1a changes no language. Prints each file that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
src="$P/pc-src-51a"
rm -rf "$src"; mkdir -p "$src"
git archive 25be9f3c | tar -x -C "$src"
n=0; differ=0; nz_base=0; nz_new=0
while read -r f; do
  for mode in --check --ast; do
    a=$( (cd "$src" && "$P/nova-base-51a" parse $mode "$f") 2>&1; echo "exit=$?")
    b=$(target/debug/nova parse $mode "$f" 2>&1; echo "exit=$?")
    n=$((n + 1))
    if [ "$mode" = --check ]; then
      [ "$(tail -1 <<< "$a")" != exit=0 ] && nz_base=$((nz_base + 1))
      [ "$(tail -1 <<< "$b")" != exit=0 ] && nz_new=$((nz_new + 1))
    fi
    if [ "$a" != "$b" ]; then differ=$((differ + 1)); echo "differs: $mode $f"; fi
  done
done < <(git ls-tree -r --name-only 25be9f3c | grep '\.nova$')
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
# `nova check` by the base's `nova` ($P/nova-base-51a, Task 0) and by the
# tree's `target/debug/nova`, both on the tree's files, for every `.nova`
# file `git ls-tree -r --name-only 25be9f3c` lists. A file's package root
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
    a=$( (cd "$pkg" && "$P/nova-base-51a" check) 2>&1; echo "exit=$?")
    b=$( (cd "$pkg" && "$nova" check) 2>&1; echo "exit=$?")
    what="$pkg/ (package)"
  else
    a=$("$P/nova-base-51a" check "$f" 2>&1; echo "exit=$?")
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
done < <(git ls-tree -r --name-only 25be9f3c | grep '\.nova$')
echo "check: $n checks, $differ outside std differ, $stddiff std files differ"
[ "$differ" = 0 ] && [ "$stddiff" = 0 ]
EOF
```

  - Then:

    ```sh
    P="$P" bash "$P/parsecmp.sh"
    P="$P" bash "$P/checkcmp.sh"
    ```

    Expected: `parse: 590 runs over 295 files, 0 differ; --check exits nonzero on 2 at the base, 2 now`: 5.1a changes no language and no existing
    `.nova` file; the two that exit nonzero are the fixtures that do not
    parse, `crates/nova-parser/tests/fixtures/async.nova` and
    `tools/vscode-nova/test/fixture/src/complete.nova`. Then
    `check: 261 checks, 0 outside std differ, 0 std files differ`. Both exit 0.
- [ ] **Step 5: The hidden flag stays hidden.**
  - `target/debug/nova parse --help > "$P/final-help-51a.txt"`, then
    `cmp "$P/base-help-51a.txt" "$P/final-help-51a.txt"`. Expected: no
    output: the help is the base's byte for byte, `more` and
    `--dump-tokens` both hidden (decision 17).
  - The second file's error, from the base's `nova` and the tree's, each
    run with the same first argument (decisions 39 and 44): `othernova`, a
    renamed binary, and four with no file name in UTF-8, `""`, `/`, `..`
    and the bytes ff fe. `argv0.py` runs a program by `execv` with the
    first argument it is given, empty included, which Python's `os.execv`
    refuses; `usagecmp.sh` runs the four commands of each, then two in a
    terminal, through `script`'s pseudo-terminal, colours included:

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
# from the base's `nova` ($P/nova-base-51a, Task 0) and the tree's
# `target/debug/nova`, each run by execv with the same first argument:
# `othernova` (a renamed binary), and four with no file name in UTF-8,
# "", "/", ".." and the bytes ff fe. Both streams and the exit status must
# be byte-identical. Then the same in a terminal, colours included, through
# `script`'s pseudo-terminal with NO_COLOR unset.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
runs=0; same=0
for arg0 in othernova "" / .. '\xff\xfe'; do
  for args in "parse a b" "parse --ast a b" "parse --check a b" "parse -- a b"; do
    python3 -I "$P/argv0.py" "$arg0" "$P/nova-base-51a" $args > "$P/usage-base.txt" 2>&1
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
  env -u NO_COLOR script -qc "python3 -I '$P/argv0.py' othernova '$P/nova-base-51a' $args" /dev/null > "$P/usage-base.txt"
  env -u NO_COLOR script -qc "python3 -I '$P/argv0.py' othernova '$nova' $args" /dev/null > "$P/usage-tree.txt"
  if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then tty=$((tty + 1)); else echo "differs in a terminal: $args"; fi
done
grep -qa $'\e\\[' "$P/usage-tree.txt" || echo "no colour in the terminal's output"
echo "usage: $same of $runs runs identical; in a terminal, $tty of 2"
rm -f "$P/usage-base.txt" "$P/usage-tree.txt"
[ "$same" = "$runs" ] && [ "$tty" = 2 ]
EOF
```

    ```sh
    P="$P" bash "$P/usagecmp.sh"
    ```

    Expected:

    ```
    [othernova] Usage: othernova parse [OPTIONS] <FILE>, exit 2
    [] Usage: parse [OPTIONS] <FILE>, exit 2
    [/] Usage: parse [OPTIONS] <FILE>, exit 2
    [..] Usage: parse [OPTIONS] <FILE>, exit 2
    [\xff\xfe] Usage: parse [OPTIONS] <FILE>, exit 2
    usage: 20 of 20 runs identical; in a terminal, 2 of 2
    ```

    and exit 0: each error names `othernova` or `parse` alone in its usage
    line and exits 2, as the base's clap does.
- [ ] **Step 6: The lockfile and Rust 1.78.**
  - `git diff --numstat 25be9f3c -- Cargo.lock`. Expected: nothing.
  - Then:

    ```sh
    RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-51a cargo check --locked --workspace
    rm -rf /dev/shm/msrv-51a
    ```

    Expected: clean, `Finished` in 51.28 s. It checks the libraries
    and binaries, as CI's MSRV job does, not the tests. Its target
    directory is in `/dev/shm`, memory, so the disk stays above 6G.
- [ ] **Step 7: The differential's time.** `SH --no-run`, then `SH` three
  times, each timed. Expected: `19 passed; 0 failed` each time, in
  11.7 s, 12.6 s and 12.4 s of wall time, building N included (decision 10).
  Record them in the ledger beside CI's, once the PR's run gives each system's.
- [ ] **Step 8: The mutants.** One at a time, each edit is made, its
  command run, and the file restored with `git checkout -- <file>`, all on
  the last commit, in a shell under the process limit.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - Each command starts with `cargo build --locked --workspace`. A mutant
    of N needs no Rust rebuilt: `selfhost.rs` builds N from `selfhost/`
    each run.
  - The replay's run took 21 min 22 s in all; a mutant of S0 about 40
    seconds, one of N about 15, and mutant 34 a little
    over a minute, since its test waits a minute for an answer that never
    comes.
  - Each caught mutant must fail as written: the tests named. Four
    mutants pass by design, each a caller bypassing a helper whose own
    test it cannot reach cheaply (mutants 39 to
    42, decision 46): each must pass, and is named in
    the PR. Any other mutant that passes is a gap: record it in the
    ledger, name it in the PR, and give it to the review (Step 10), which
    says whether a check is added.
  - At the end, `git status --short` prints nothing, and
    `cargo build --locked --workspace` rebuilds the last commit's binaries
    (decision 34).

  1. **S0's token lines print the start twice.** `crates/nova-lexer/src/dump.rs`:
     `let _ = write!(out, "{} {} ", t.span.start, t.span.end);` becomes `let _ = write!(out, "{} {} ", t.span.start, t.span.start);`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
     - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mismatch_report_names_the_file_and_line`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`, `the_token_dump_of_nothing_is_eof`, `the_token_dump_writes_every_field`, `unreadable_files_are_reported_alike`.
  2. **S0 adds a mark to each payload-less kind's name.** `crates/nova-lexer/src/dump.rs`:
     `let _ = write!(out, "{other:?}");` becomes `let _ = write!(out, "{other:?}_");`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
     - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`, `the_token_dump_of_nothing_is_eof`, `the_token_dump_writes_every_field`, `unreadable_files_are_reported_alike`.
  3. **S0 prints an `Int` in hex.** `crates/nova-lexer/src/dump.rs`:
     `let _ = write!(out, "Int {n}");` becomes `let _ = write!(out, "Int {n:x}");`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
     - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  4. **S0 prints a `Float`'s bits in upper case.** `crates/nova-lexer/src/dump.rs`:
     `let _ = write!(out, "Float {:016x}", f.to_bits());` becomes `let _ = write!(out, "Float {:016X}", f.to_bits());`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
     - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  5. **S0 prints a `Float`'s bits in upper case, against its own test alone.** `crates/nova-lexer/src/dump.rs`:
     `let _ = write!(out, "Float {:016x}", f.to_bits());` becomes `let _ = write!(out, "Float {:016X}", f.to_bits());`.
     - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --test lexer_tests -- the_token_dump`, each to its end.
     - Expected: fails: `the_token_dump_writes_every_field`.
  6. **S0 prints a `Char` one past its code point.** `crates/nova-lexer/src/dump.rs`:
     `let _ = write!(out, "Char {}", *c as u32);` becomes `let _ = write!(out, "Char {}", *c as u32 + 1);`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
     - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  7. **S0 names a raw string `StrPart`.** `crates/nova-lexer/src/dump.rs`:
     `Token::RawStr(s) => quoted(out, "RawStr", s),` becomes `Token::RawStr(s) => quoted(out, "StrPart", s),`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
     - Expected: fails: `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  8. **S0 trims a doc comment's leading space.** `crates/nova-lexer/src/dump.rs`:
     `Token::DocComment(s) => quoted(out, "DocComment", s),` becomes `Token::DocComment(s) => quoted(out, "DocComment", s.trim_start()),`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
     - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  9. **S0 leaves `"` in a payload unescaped.** `crates/nova-lexer/src/dump.rs`:
     `if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {` becomes `if (c as u32) < 0x20 || c == '\x7f' || c == '\\' {`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
     - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  10. **S0 leaves `\` in a payload unescaped.** `crates/nova-lexer/src/dump.rs`:
      `if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {` becomes `if (c as u32) < 0x20 || c == '\x7f' || c == '"' {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  11. **S0 leaves DEL in a payload unescaped.** `crates/nova-lexer/src/dump.rs`:
      `if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {` becomes `if (c as u32) < 0x20 || c == '"' || c == '\\' {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  12. **S0 writes a payload's escapes in upper-case hex.** `crates/nova-lexer/src/dump.rs`:
      replace

      ```
              if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {
                  let _ = write!(out, "\\x{:02x}", c as u32);
      ```

      with

      ```
              if (c as u32) < 0x20 || c == '\x7f' || c == '"' || c == '\\' {
                  let _ = write!(out, "\\x{:02X}", c as u32);
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  13. **S0 leaves control bytes in an error's message.** `crates/nova-lexer/src/dump.rs`:
      `if (c as u32) < 0x20 || c == '\x7f' {` becomes `if c == '\x7f' {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  14. **S0 writes the line-break mark as `N`.** `crates/nova-lexer/src/dump.rs`:
      `out.push_str(" NL");` becomes `out.push_str(" N");`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`, `the_token_dump_of_nothing_is_eof`, `the_token_dump_writes_every_field`, `unreadable_files_are_reported_alike`.
  15. **S0 reads the plain-decimal mark of the next token.** `crates/nova-lexer/src/dump.rs`:
      `if tokens.plain_decimal(i) {` becomes `if tokens.plain_decimal(i + 1) {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`, `the_token_dump_of_nothing_is_eof`, `the_token_dump_writes_every_field`, `unreadable_files_are_reported_alike`.
  16. **S0's error lines print the start twice.** `crates/nova-lexer/src/dump.rs`:
      `let _ = write!(out, "ERR {} {} ", span.start, span.end);` becomes `let _ = write!(out, "ERR {} {} ", span.start, span.start);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  17. **S0 calls a block comment a line comment.** `crates/nova-lexer/src/dump.rs`:
      `CommentKind::Block => "Block",` becomes `CommentKind::Block => "Line",`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --test lexer_tests -- the_token_dump` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_writes_every_field`.
  18. **S0's header has three `=`.** `crates/nova-cli/src/cmd/parse.rs`:
      `writeln!(out, "== {}", file.display())?;` becomes `writeln!(out, "=== {}", file.display())?;`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_side_that_fails_is_reported_with_its_exit_code_and_stderr`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mismatch_report_names_the_file_and_line`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`, `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`, `unreadable_files_are_reported_alike`.
  19. **S0 calls a file that is not UTF-8 `Other`.** `crates/nova-cli/src/cmd/parse.rs`:
      `ErrorKind::InvalidData => "InvalidData",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `unreadable_files_are_reported_alike`.
  20. **S0 exits 0 after a file it cannot read.** `crates/nova-cli/src/cmd/parse.rs`:
      `code = 2;` becomes `code = 0;`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_side_that_fails_is_reported_with_its_exit_code_and_stderr`, `the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one`, `unreadable_files_are_reported_alike`.
  21. **The flag shows in `--help`.** `crates/nova-cli/src/cmd/parse.rs`:
      `#[arg(long, hide = true, conflicts_with_all = ["ast", "check"])]` becomes `#[arg(long, conflicts_with_all = ["ast", "check"])]`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_token_dump_flag_is_hidden`.
  22. **The flag takes `--check`.** `crates/nova-cli/src/cmd/parse.rs`:
      `conflicts_with_all = ["ast", "check"]` becomes `conflicts_with_all = ["ast"]`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_token_dump_flag_is_hidden`.
  23. **A second file without the flag is read as the first's.** `crates/nova-cli/src/cmd/parse.rs`:
      `if let Some(extra) = cmd.more.first() {` becomes `if let Some(extra) = None::<&PathBuf> {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_token_dump_flag_is_hidden`.
  24. **The second file's usage line always says `nova`.** `crates/nova-cli/src/cmd/parse.rs`:
      `Some(name) => format!("{name} parse"),` becomes `Some(_) => "nova parse".to_string(),`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_token_dump_flag_is_hidden`.
  25. **With no file name to take, the usage line says `nova parse`.** `crates/nova-cli/src/cmd/parse.rs`:
      `None => "parse".to_string(),` becomes `None => "nova parse".to_string(),`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_token_dump_flag_is_hidden`.
  26. **The comparator takes outputs of one length as equal.** `crates/nova-cli/tests/selfhost/compare.rs`:
      `if s0 == n {` becomes `if s0.len() == n.len() {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mismatch_report_names_the_file_and_line`.
  27. **The report names no file.** `crates/nova-cli/tests/selfhost/compare.rs`:
      `file = Some(text(path).trim_end_matches('\n').to_string());` becomes `file = None;`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mismatch_report_names_the_file_and_line`.
  28. **The report counts two lines a line.** `crates/nova-cli/tests/selfhost/compare.rs`:
      `None => dump_line += 1,` becomes `None => dump_line += 2,`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mismatch_report_names_the_file_and_line`.
  29. **Agreeing dumps pass whatever the exit codes.** `crates/nova-cli/tests/selfhost.rs`:
      `if mismatch.is_none() && s0.code == Some(code) && n.code == Some(code) {` becomes `if mismatch.is_none() {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_side_that_fails_is_reported_with_its_exit_code_and_stderr`.
  30. **The comparator reads both outputs lossily first.** `crates/nova-cli/tests/selfhost/compare.rs`:
      replace

      ```
          if s0 == n {
              return None;
          }
      ```

      with

      ```
          let (s0, n) = (text(s0), text(n));
          let (s0, n) = (s0.as_bytes(), n.as_bytes());
          if s0 == n {
              return None;
          }
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mismatch_report_names_the_file_and_line`.
  31. **The soup is its tails alone, so two errors never occur.** `crates/nova-cli/tests/selfhost/fuzz.rs`:
      `for _ in 0..rng.below(201) {` becomes `for _ in 0..rng.below(1) {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_soup_agrees_and_holds_every_lex_error`.
  32. **The mutations read a corpus file's `\r\n` as it is.** `crates/nova-cli/tests/selfhost.rs`:
      replace

      ```
      .expect("read a corpus file")
                      .replace("\r\n", "\n");
      ```

      with

      ```
      .expect("read a corpus file");
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mutations_are_the_same_with_windows_line_endings`.
  33. **The mutations pick from the files in the order given.** `crates/nova-cli/tests/selfhost.rs`:
      `files.sort();`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_mutations_are_the_same_with_windows_line_endings`.
  34. **A process past its limit is never killed.** `crates/nova-cli/tests/selfhost.rs`:
      `Ok(None) if Instant::now() < deadline =>` becomes `Ok(None) if Instant::now() < deadline || true =>`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_process_past_its_limit_is_killed_and_its_batch_named`.
  35. **The pipes' readers are waited for with no deadline.** `crates/nova-cli/tests/selfhost.rs`:
      replace

      ```
              if Instant::now() >= deadline {
                  return None;
              }
      ```

      with

      ```

      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_pipe_still_open_at_the_deadline_is_reported`.
  36. **A line without its final line break is not marked.** `crates/nova-cli/tests/selfhost/compare.rs`:
      `if unbroken(s, other) {` becomes `if false {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_missing_final_line_break_is_marked`.
  37. **Any last line without a line break is marked, whatever the other side's.** `crates/nova-cli/tests/selfhost/compare.rs`:
      `!s.ends_with(b"\n") && other.is_some_and(|o| o.strip_suffix(b"\n") == Some(s))` becomes `!s.ends_with(b"\n")`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_missing_final_line_break_is_marked`.
  38. **`selfhost()` builds N each time it is asked, not through `once`.** `crates/nova-cli/tests/selfhost.rs`:
      replace

      ```
          once(&BUILT, build_selfhost)
      ```

      with

      ```
          let _ = &BUILT;
          Box::leak(build_selfhost().unwrap_or_else(|e| panic!("{e}")).into_boxed_path())
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_failed_build_is_made_once_and_reported_to_each_test`, `both_sides_refuse_no_file`, `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_mutated_corpus_agrees`, `unreadable_files_are_reported_alike`.
  39. **`selfhost()` keeps N's path in a cache of its own and panics inside it, as before the first fact-check.** `crates/nova-cli/tests/selfhost.rs`:
      replace

      ```
          once(&BUILT, build_selfhost)
      ```

      with

      ```
          static PATH: OnceLock<PathBuf> = OnceLock::new();
          let _ = &BUILT;
          PATH.get_or_init(|| build_selfhost().unwrap_or_else(|e| panic!("{e}")))
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, an accepted survivor (decision 46).
  40. **`run()` starts each side by `Command::output`, with no limit.** `crates/nova-cli/tests/selfhost.rs`:
      replace

      ```
          let out = output_within(
              Command::new(program)
      ```

      with

      ```
          let out = (|c: &mut Command, _: Duration, _: &str| c.output().map_err(|e| e.to_string()))(
              Command::new(program)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, an accepted survivor (decision 46).
  41. **N's build runs by `Command::output`, with no limit.** `crates/nova-cli/tests/selfhost.rs`:
      `let built = output_within(` becomes `let built = (|c: &mut Command, _: Duration, _: &str| c.output().map_err(|e| e.to_string()))(`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, an accepted survivor (decision 46).
  42. **`output_within` joins its readers with no deadline, not through `read_by`.** `crates/nova-cli/tests/selfhost.rs`:
      `read_by([stdout, stderr], deadline)` becomes `Some([stdout.join().expect("read a pipe"), stderr.join().expect("read a pipe")])`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, an accepted survivor (decision 46).
  43. **A failed build panics inside the `OnceLock`, so each test builds again.** `crates/nova-cli/tests/selfhost.rs`:
      `match cell.get_or_init(make) {` becomes `match cell.get_or_init(|| Ok(make().unwrap_or_else(|e| panic!("{e}")))) {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_failed_build_is_made_once_and_reported_to_each_test`.
  44. **N wraps an overflowing integer.** `selfhost/src/lexer.nova`:
      `if v > (9223372036854775807 - d) / radix { return None }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `an_overflowing_integer_is_an_unexpected_first_digit`.
  45. **N's `r#` with no `"` is an unterminated string.** `selfhost/src/lexer.nova`:
      `return Err(InvalidNumberLiteral(Span { start: start, end: self.pos }))` becomes `return Err(UnterminatedString(Span { start: start, end: self.pos }))`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `raw_strings_and_the_one_invalid_number_literal`.
  46. **N takes `'\q'` as the char `q`.** `selfhost/src/lexer.nova`:
      replace

      ```
                  } else {
                      0 - 1
                  }
                  if cp < 0 { return None }
      ```

      with

      ```
                  } else {
                      e
                  }
                  if cp < 0 { return None }
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `char_literals_and_the_three_errors_of_a_bad_escape`.
  47. **N splits a float after any token.** `selfhost/src/lexer.nova`:
      `let after_dot = !out.is_empty() && is_dot(out.last().value)` becomes `let after_dot = !out.is_empty()`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `a_float_after_a_dot_splits_into_tuple_indexes`.
  48. **N's split makes a `..` of the dot.** `selfhost/src/lexer.nova`:
      `out.push(spanned(Dot, dot, dot + 1))` becomes `out.push(spanned(DotDot, dot, dot + 1))`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `a_float_after_a_dot_splits_into_tuple_indexes`.
  49. **N's line-break mark ignores a lone `\r`.** `selfhost/src/lexer.nova`:
      replace

      ```
      if c == NEWLINE || c == CR {
                      line_break = true
      ```

      with

      ```
      if c == NEWLINE {
                      line_break = true
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `line_breaks_are_marked_through_comments`.
  50. **N's line-break mark reads only the byte before a token.** `selfhost/src/lexer.nova`:
      `let mut i = min(prev_end, start)` becomes `let mut i = max(min(prev_end, start), start - 1)`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `line_breaks_are_marked_through_comments`.
  51. **N marks a leading zero plain decimal.** `selfhost/src/lexer.nova`:
      `end - start == 1 || source.byte_at(start) != ZERO` becomes `true`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `a_float_after_a_dot_splits_into_tuple_indexes`.
  52. **N keeps a doc comment's trailing whitespace.** `selfhost/src/lexer.nova`:
      `let text_end = self.trim_end(start + 3, i)` becomes `let text_end = i`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `doc_comments_are_exactly_three_slashes`.
  53. **N's White_Space lacks the no-break space.** `selfhost/src/lexer.nova`:
      `|| cp == 0xA0`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_soup_agrees_and_holds_every_lex_error`.
  54. **N's White_Space lacks the ideographic space.** `selfhost/src/lexer.nova`:
      `|| cp == 0x3000`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_soup_agrees_and_holds_every_lex_error`.
  55. **N's White_Space has U+2000 and U+200A but none between.** `selfhost/src/lexer.nova`:
      `|| (cp >= 0x2000 && cp <= 0x200A)` becomes `|| cp == 0x2000 || cp == 0x200A`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`.
  56. **N decodes `\t` as `t`.** `selfhost/src/lexer.nova`:
      `buf.push_char('\t')` becomes `buf.push_char('t')`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `escapes_are_decoded_and_bad_ones_reported`.
  57. **N decodes `\0` as `0`.** `selfhost/src/lexer.nova`:
      `buf.push_char('\0')` becomes `buf.push_char('0')`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `escapes_are_decoded_and_bad_ones_reported`.
  58. **N refuses a `+` in a Unicode escape.** `selfhost/src/lexer.nova`:
      `if i < end && self.source.byte_at(i) == PLUS { i += 1 }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `escapes_are_decoded_and_bad_ones_reported`.
  59. **N lets a Unicode escape's value wrap.** `selfhost/src/lexer.nova`:
      `if v > 0xFFFFFFFF { return 0 - 1 }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`.
  60. **N does not count a hole's braces.** `selfhost/src/lexer.nova`:
      `frame.braces += 1`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `interpolation_holes_nest_and_count_their_braces`.
  61. **N's open hole is one byte wide.** `selfhost/src/lexer.nova`:
      `let end = min(frame.open + 2, self.len)` becomes `let end = min(frame.open + 1, self.len)`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `interpolation_holes_nest_and_count_their_braces`.
  62. **N has no keyword `with`.** `selfhost/src/lexer.nova`:
      `if len == 4 && s == "with" { return With }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `keywords_are_whole_identifiers`.
  63. **N has no `<<=`.** `selfhost/src/lexer.nova`:
      `if c2 == 61 { return Some((LtLtEq, start + 3)) }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  64. **N has no `->`.** `selfhost/src/lexer.nova`:
      `if c1 == 62 { return Some((Arrow, start + 2)) }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  65. **N's whitespace lacks `\r`.** `selfhost/src/lexer.nova`:
      `if c == SPACE || c == TAB || c == NEWLINE || c == CR {` becomes `if c == SPACE || c == TAB || c == NEWLINE {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `line_breaks_are_marked_through_comments`.
  66. **N's line comment keeps a `\r` before its line break.** `selfhost/src/lexer.nova`:
      `let end = if self.pos > start && b.byte_at(self.pos - 1) == CR {` becomes `let end = if false && b.byte_at(self.pos - 1) == CR {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `line_breaks_are_marked_through_comments`.
  67. **N's exponent takes no `-`.** `selfhost/src/lexer.nova`:
      `if k < self.len && (b.byte_at(k) == PLUS || b.byte_at(k) == MINUS) { k += 1 }` becomes `if k < self.len && b.byte_at(k) == PLUS { k += 1 }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  68. **N reads a prefix's digit past the end.** `selfhost/src/lexer.nova`:
      `if b.byte_at(start) == ZERO && start + 2 < n {` becomes `if b.byte_at(start) == ZERO && start + 1 < n {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_edge_inputs_agree`, `the_soup_agrees_and_holds_every_lex_error`.
  69. **N steps three bytes over a four-byte character.** `selfhost/src/lexer.nova`:
      `if b >= 0xF0 { return 4 }` becomes `if b >= 0xF0 { return 3 }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_soup_agrees_and_holds_every_lex_error`.
  70. **N leaves `"` in a payload unescaped.** `selfhost/src/dump.nova`:
      `(quoted && (b == 34 || b == 92))` becomes `(quoted && b == 92)`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test`s `escapes_are_decoded_and_bad_ones_reported`, `raw_strings_and_the_one_invalid_number_literal`.
  71. **N prints fifteen hex digits of a float's bits.** `selfhost/src/dump.nova`:
      `let mut shift = 60` becomes `let mut shift = 56`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_corpus_dumps_agree`, `the_corpus_dumps_agree_with_windows_line_endings`, `the_edge_inputs_agree`, `the_fixed_cases_agree`, `the_mutated_corpus_agrees`, `the_selfhost_tests_pass`, `the_soup_agrees_and_holds_every_lex_error`; inside `the_selfhost_tests_pass`, N's `@test` `a_float_after_a_dot_splits_into_tuple_indexes`.
  72. **N calls a file that is not UTF-8 `Other`.** `selfhost/src/main.nova`:
      `InvalidData => "InvalidData"` becomes `InvalidData => "Other"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `unreadable_files_are_reported_alike`.
  73. **N exits 0 after a file it cannot read.** `selfhost/src/main.nova`:
      `if failed { exit(2) }` becomes `if failed { exit(0) }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `unreadable_files_are_reported_alike`.
  74. **N takes no file, prints nothing and exits 0.** `selfhost/src/main.nova`:
      `if argv.len() < 3 || argv.at(1) != "tokens" {` becomes `if argv.len() < 2 || argv.at(1) != "tokens" {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_refuse_no_file`.
  75. **N describes a hole's opening as `{`.** `selfhost/src/token.nova`:
      `` InterpOpen => "`$" + "{`" `` becomes `` InterpOpen => "`{`" ``.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `descriptions_are_the_rust_ones`.
  76. **N describes an `Int` as `integer`.** `selfhost/src/token.nova`:
      `Int(_) => "integer literal"` becomes `Int(_) => "integer"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `descriptions_are_the_rust_ones`.
  77. **`main.nova` imports no `@test`s, so `nova test` runs none.** `selfhost/src/main.nova`:
      `import lexer_tests`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`.
- [ ] **Step 9: The review's inputs.** The full run's figures, the new
  tests by binary, the golden suite's line and time, the parse, check and
  help comparisons, the differential's times, and the mutants' outcome,
  with each gap, go in the ledger.
- [ ] **Step 10: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0051 and this plan, and is told to hunt for:
    - an input on which N's lexer and S0's part that no generator, case or
      edge input makes: a quirk of logos's written wrongly (the longest
      match, a refused callback, the char literal's classes), an overflow
      in any radix, a Unicode escape `u32::from_str_radix` reads otherwise,
      `trim_end` other than Unicode's White_Space, the float split by value
      rather than text, a mark wrong through a comment (decisions 5, 9 and
      21);
    - anything that differs only on Windows or macOS: a line ending, a
      path printed or passed differently by the two sides, the
      executable's name, a read's error kind, output translated, a stack:
      S0's decimal numbers recurse once per character, recorded for
      5.1b's big-stack thread (decisions 20 and 42);
    - a way for the harness to pass when the two sides differ, or to hang:
      a batch not compared, an exit code not checked, an empty or crashed
      side taken as agreement, a corpus that silently loses files, N's
      tests not run, a process not killed at its limit or its pipes left
      to fill or held by a process the child started, a report that blames
      the wrong file or hides a missing line break, a caller that bypasses
      `once`, `output_within` or `read_by` (decisions 8, 14, 22, 35, 36,
      38 and 46 to 48);
    - something that breaks only under repetition: two runs sharing the
      fixed directories, a stale N reused, a failed build run again, the
      fuzzer's inputs differing by system for one seed, state kept between
      batches (decisions 31, 37 and 38);
    - the dump's format on either side against spec §3.3 and decisions 7,
      16 and 18: an escape, a mark, a payload, the order;
    - the hidden flag shown anywhere, `--help` changed, a second file read
      without it or refused in other words or colours than the base's,
      under any first argument, its conflicts (decisions 17, 39 and 44);
    - a commit between Task 1 and Task 4 that builds wrong code or fails
      the suite (decisions 27 to 30);
    - the records: ADR 0051, the CHANGELOG, the dated notes, the branch
      name in each;
    - each mutant of Step 8 that passes, the four of decision 46
      included, and whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock`, a test that touches
      the internet, a temporary name that is not fixed.
  - Each finding is fixed in one commit, `5.1a: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Steps 1 and 2 run again.
- [ ] **Step 11: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-51a`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 5.1a adds: `selfhost/`, N's lexer and its `@test`s; S0's
      `nova_lexer::dump_tokens` and the hidden `nova parse --dump-tokens`;
      the differential with its corpus, Windows-endings corpus, cases, edge
      inputs, soup and mutations; CI's `nova fmt --check` over
      `selfhost/`; ADR 0051;
    - that no language changes: every `.nova` file the base tracks parses
      and checks as before, and `nova parse --help` is unchanged;
    - the figures: the suite's `2816 1 10`, the golden suite's
      `147 passed, 9 excluded`, the differential's time here, and that
      `Cargo.lock` is unchanged;
    - the user's decisions, and this plan's decisions, numbered as here,
      with 13 to 18 marked as differing from the spec;
    - the mutants, the four that pass by design (decision 46) and any gap
      they found, and the review's findings and what came of each;
    - the known S0 bug ADR 0051 records, a long decimal number overflowing
      debug S0's stack, for 5.1b (decision 42).
  - Read CI once it ends. Expected: green on all three systems; on the
    `test` job's `ubuntu-24.04` leg, `cargo test`'s log holds
    `test release_golden_suite ... ok`, and on each system `selfhost.rs`'s
    19 tests pass; record each system's time for it, and the run's
    address, in the ledger.
  - **If the differential fails on CI and not here,** record its report,
    stop and ask the user: a Windows or macOS difference is the likeliest
    (decision 20), and its report names the file, the line and both
    sides' exit codes and stderr. **If `selfhost.rs` takes more than a
    minute on a system,** ask the user before lowering the counts (spec
    §9 risk 5).
- [ ] **Step 12: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 11's.
