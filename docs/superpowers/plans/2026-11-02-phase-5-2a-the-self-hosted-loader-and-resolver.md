# Phase 5.2a, "The self-hosted loader and resolver", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** The first of Phase 5.2's two parts: N's third and fourth stages
for loose programs. `selfhost/` gains a loader that reads a root and the
sibling files it imports, breadth first as the driver does, and a
resolver that follows `nova-resolver` function for function, with std
read from `--std DIR`. `selfhost resolve --std DIR FILE…` prints the same
resolution dump as S0's hidden `nova check --dump-resolution FILE…`: the
modules, the definitions with their ids and mangled names, each module's
namespaces, the exports, the items and splits, then every diagnostic with
its labels, notes and fixes; std's part, the same in every native
program, once. The differential (`crates/nova-cli/tests/selfhost.rs`)
compares the two on std's part, the 251 loose corpus programs, 103
resolver cases (with Windows line endings too) and a name-level fuzzer's
inputs, on Linux, macOS and Windows. One change to S0's output: a glob
import's E0002s for the names it clashes with come out in declaration
order. ADR 0053 records the dump's grammar, the idioms, the departures and
the figures.

**Architecture:**
- **S0, the order:** `crates/nova-resolver/src/lib.rs`'s `Exports` keeps
  each namespace's names in a `Vec` beside its map; `resolve_import`'s
  glob follows them.
- **S0, the dump:** `crates/nova-resolver/src/dump.rs` (`dump_program`,
  `dump_std`, `KINDS`, `LATER_KINDS`, `forms_on`, `listed`);
  `nova_resolver::parse_std` and `resolve_program_with_std`, which
  `resolve_program_for` now calls; `nova_driver::ResolutionDumps` and
  `dump_std_resolution`; `nova check --dump-resolution [--std]`, hidden,
  `crates/nova-cli/src/cmd/run.rs`'s `DumpResolutionCmd`, which
  `crates/nova-cli/src/main.rs` takes before clap parses, only when it
  comes first after `check`, so `CheckCmd` stays the base's.
- **N:** `selfhost/src/diagnostic.nova`, `builtins.nova`, `loader.nova`,
  `resolver.nova`, `resolution_dump.nova`, and `loader_tests.nova`,
  `resolver_tests.nova` (N's own `@test`s); `main.nova`'s `selfhost
  resolve`.
- **The differential:** `selfhost.rs`'s resolution pass, its
  every-form check, the headers, the root's spellings and the CRLF cases;
  `tests/selfhost/fuzz.rs`'s name fuzzer; 139 new files under
  `tests/selfhost/cases/resolve/`.
- **Records:** ADR 0053; `CHANGELOG.md`; dated notes in
  `docs/phase-5-plan.md`, `nova-spec/00-MASTER-SPEC.md`,
  `nova-spec/11-PARSER.md` and `nova-spec/12-TYPESYSTEM.md`.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §6).
- N is built by `nova build` through Cranelift: no `clang` for the
  differential. `clang-18` (Ubuntu's 18.1.3, CI's pin) for the golden
  suite, and Node 22 for the WASM tests, as before.

**Spec:**
`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`,
approved by the user on 2026-11-02, on this branch's base.
- Read its header, §3 (5.2a), §§1, 2 and 5 to 10 before Task 1: it is the
  authority this plan argues from.
- Its header holds the user's decisions of 2026-11-02 (decisions 1 to 7
  here) and its two "not asked" recommendations (decisions 9 and 10);
  the user's ruling of 2026-11-03 on the three codes is decision 8.

**Checked on 2026-11-02 and 2026-11-03, before execution:**
- **A prototype built 5.2a whole** on a local branch from `7878b9bb`
  (`main` after 5.1b, `1fa7cd3f`, and the approved spec), in this tree, in
  two parts: S0's side, `04b44070` (the order), `99486460` (the cases),
  `98bbcbe3` (the dump and the flags) and `27a90a38` (the name fuzzer);
  then N's, `b997d4f8` (S0 parses std once), `82793aef` (N's loader,
  resolver and dump), `9a6dad02` (the comparison) and `2706b1bb` (the
  records); then step 0, `a8ce1570`, "5.2a prototype: before the split"
  (decisions 14, 16 and 26 to 33); then `f2fe29d6`, "5.2a prototype: the
  first fact-check's fixes"; then `f9894514`, "5.2a prototype: the second
  fact-check's fixes". N's first build agreed with S0 on std's
  part, every loose corpus program, the 88 extracted cases, the fixed
  seeds and 6,500 inputs of longer searches; step 0's probe of the root's
  spellings found the one difference, `sub/./main.nova`, now fixed.
- **Step 0, before the split** (decisions 14, 16 and 26 to 33):
  - **Windows paths:** E0093, the one path in S0's resolution output,
    gets a case, run from inside its directory so the path has no
    directory part on any system; N's `path_parent` follows
    `Path::parent` on `.` components and repeated `/`s; a test compares
    eleven spellings of a root and a root whose label needs escapes
    (decision 27 **(differs)**);
  - the loader's edges: eleven hand cases, the cases with CRLF line endings,
    a non-UTF-8 sibling name (decision 30);
  - `--dump-resolution`'s argument shapes and `--help`, against the base
    (decision 31);
  - a unit test that `parse_std` and `resolve_program_with_std` resolve as
    `resolve_program_for` does (decision 26 **(differs)**);
  - the fuzzer's eight codes each at least twice (decision 16);
  - `(Import Reported` a loose program's form, and every form a dump
    prints listed (decision 14);
  - `ImportTarget` beside N's loader, so the loader builds without the
    resolver (decision 28 **(differs)**);
  - the records: ADR 0053 accepted with the branch, CI's figures, the
    Windows settlement, the counts made one (decision 33).
- **The first fact-check's fixes** (`f2fe29d6`), after a fact-check of
  the plan and the prototype found:
  - **the Windows leg would fail:** `dump.rs`'s test compared
    `include_str!("dump_test_program.txt")` byte for byte, and a Windows
    checkout gives the fixture `\r\n` (no `.gitattributes` rule covers
    `.txt`). The test now reads `\r\n` as `\n`, and a second test
    compares a CRLF copy; with the fixture made CRLF, the old test failed
    and the new ones pass; without the fix the CRLF test fails. An audit
    of every fixture read 5.2a adds found no other (Review Focus 1);
  - **the hidden flags changed `nova check`'s refusals** in 17 of 24
    shapes against the base (tips naming `--std`, `--std=1`, `--std
    --std`, which fault comes first, `--std --help` printing help): the
    flags were real arguments of `CheckCmd`. Now `main` takes the form
    only when `--dump-resolution` comes first after `check`, with a parser
    of its own, and `CheckCmd` is the base's (decision 13). The
    harness's flags test and `usagecmp.sh` hold each shape to the base's
    bytes; on the old tree the test failed at its first shape, and
    `usagecmp.sh` found 90 of 174 runs different;
  - **decision 25 overrode part of the user's decision 4**,
    which put both the 11 codes in 5.2a and test mode and the WASM target
    in 5.2b, which cannot both hold; asked, the user ruled on 2026-11-03
    to move the three codes to 5.2b (decision 8);
  - **a root whose path is not UTF-8:** N prints `IOERR NotFound` where
    S0 dumps it; ADR 0053 §5 corrected, decision 29, and a
    Linux-only test pins it;
  - **six mutants of N passed** (and a seventh on a rerun): four cases,
    a header-test root and two fuzzer siblings now catch all eight of
    its O1 to O8 (decisions 15, 16, 30 and
    39);
  - **record errors:** `LATER_KINDS` is eight; the order test's nine
    names; the fixed temporary names; the differential's time and N's
    speed re-measured (decisions 22 and 23); the
    ADR's figures and line counts made one by one method; spec §3.5's
    "says so" (the test reports the skipped count); Tasks 4 and 5 keep
    `nova test`'s stderr; ADR 0053 §5's directory root qualified for
    Windows; `parsecmp.sh`'s comment; 5.1's renamed-binary run waits out
    ETXTBSY.
- **The second fact-check's fixes** (`f9894514`), after a second
  fact-check, which found no fault in the code and 19,200 generated
  programs on which S0 and N agree, but:
  - **three mutants of N's loader passed,** each a real difference from
    S0 that today's N gets right and no input showed: a root with no
    directory part listing `""` rather than `.` (a directory named
    `ui.nova` beside `main.nova` is then no E0093), a package's directory
    `""` so listed (`src/main.nova` from inside a package is then
    dumped), and a package's `tests/` unseen. A new test,
    `a_root_with_no_directory_part_or_in_a_package_loads_alike`, compares
    these roots and catches all three (Review Focus 7; decision
    30);
  - **the form's empty file was unpinned:** with its files on clap's
    default `PathBuf` parser, `check --dump-resolution ""` becomes a clap
    refusal and every test passed; the headers' test now holds it to
    `== ` and `IOERR NotFound`, and the mutant is caught (decision
    13);
  - **ruling 8 was asserted only on the fuzzer's dumps:** the corpus's
    and the cases' dumps now hold none of E0084, E0085 and E0090 either;
    a mutant in which both sides keep the `@test` functions reaches that
    check, and it fails first (decisions 8 and 25);
  - **record errors:** the fixed temporary names lacked N's
    `nova-selfhost-loader-test`; the skipped count is reported under
    `--nocapture`, not printed in CI's log; with no root, both sides
    print one line on stderr, not nothing; the phase plan's note dated
    2026-11-02 recorded the ruling of 2026-11-03; N's speed, re-measured
    five times, is stated as 1.5 to 2.0×, a range that holds the noise
    (decision 22); CI's estimate is 27 to 40 s everywhere, on
    one stated basis (decision 23).
- **A replay split the prototype into this plan's tasks,** on the local
  branch `split/5-2a`, by a script, and ran each task's steps as written:
  - from `7878b9bb`, a tests commit and a code commit for each of Tasks 1
    to 6, then Task 7's records, each the patch of its task below. The
    last commit's tree is the prototype head's
    (`git diff --stat proto/5-2a-whole split/5-2a` prints nothing). Each
    patch applies with plain `git apply`, in order, on `7878b9bb`, with no
    warning;
  - each tests commit failed as its step says before its code, and passed
    after it; the whole suite, but the IPv6 test, `cargo fmt`, clippy with
    `-D warnings` and `nova fmt --check` passed after every task:
    `2842`, `2850`, `2852`, `2852`, `2852` and `2857`, each `… 1 10` with only the IPv6 failure;
  - every "Expected:" below is the replay's, from a fresh `NOVA_HOME`, but
    Task 0's suite figures, which are 5.1b's last full run on `1fa7cd3f`.
- **The final checks ran on the split's last commit:**
  - the full suite gave `2857 1 10` (4 min 56 s): the base's `2841 1 10`
    and 16 tests more, which a comparison by binary finds as 16
    added and 0 gone (Task 8, Step 1); the one failure is the IPv6 test;
  - CI's Linux leg, the whole suite under
    `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 … --all-features`:
    `2857 1 10`, the golden suite among them (Task 8, Step 2);
  - the golden suite, in 29 s of wall time: `release golden: 147 passed, 9 excluded`;
  - `nova parse --check` and `nova parse --ast` over every `.nova` file
    `7878b9bb` tracks, by the base's `nova` and the tree's: `parse: 1178 runs over 589 files, 0 differ; --check exits nonzero on 116 at the base, 116 now`;
    `nova check` over the same files: `check: 541 checks, 0 outside std differ, 0 std files differ` (Task 8, Step 4);
  - `--help` of `nova` and of each of its 21 commands byte for byte the
    base's; 29 `parse` and `check` shapes under six first arguments the
    base's, both streams and exit code, eight in a terminal too;
    `--dump-ast`'s errors `--dump-tokens`'s; the form's own refusals each
    one error and exit 2 (Task 8, Step 5);
  - Rust 1.78's check was clean, in 32 s, and `Cargo.lock` did
    not change (Task 8, Step 6);
  - `selfhost.rs` in three runs: 27.2, 26.8 and 28.5 s (82 to 87 s of CPU); N's `nova test`:
    `test result: ok. 25 passed; 0 failed; 0 trapped; 25 total` (Task 8, Step 7).
- **The mutants.** 106 breakages of what 5.2a adds, each a text
  replaced in one file (one also in a second file, so both compilers
  change alike), run one at a time at the split's last commit,
  under the process limit (Task 8, Step 8): S0's order (3); S0's dump, its
  ids, `StdStart`, sorting, exports' order, arity, quoting, labels, notes,
  edits, a reported import, `UNSUPPORTED package`, the header, `IOERR`'s
  exit code, `--std` with a file, the `@test` stripping, the stop, std's
  target and registration order, the fixture's line endings (20); the
  flags, `check` kept the base's (3); the harness, its pass, `KINDS` both
  ways, the codes' floor, the fuzzer's parse check, the std-named case's
  directory, the CRLF copy (7); the second fact-check's empty file and
  `@test` functions kept on both sides (2); and N (71, the first
  fact-check's eight and the second's three among them): the
  loader's order, siblings, E0093, labels and notes, the case-folded name,
  importable lists, `path_parent`, `in_package`, the listing of `""` as
  `.`, the stop and the
  stripping; the resolver's scope seeding, mangling, associated types,
  method and extern indices, exports, attribute checks, E0089, nested
  imports, E0092, self-imports, the glob's order, listed types, `pub`
  fixes and their dropping, E0102, std's glob; each builtin table; the
  dump's sorting, `StdStart`, exports, arity, labels, edits, escapes and
  severities. 96 of 106 were caught, in
  58 min; 10 pass by design: paths natively unreachable until 5.2b's packages, WASM target and test mode (62, 76, 77, 78, 95); equivalent (3, 22, 31); caught only on another system, by a separator or a case-insensitive disk (32, 40)
  (decision 39).
- **What the checks left standing,** each recorded or handed on:
  - macOS and Windows are reasoned from the code, not run here: E0093's
    path, line endings, the case-folded import (decisions 27
    and 32);
  - each system's time for `selfhost.rs`, estimated from `main`'s CI at
    27 to 40 s, is the PR's to give; past a minute, ask the user (decision
    23);
  - `selfhost resolve` writes `/` in E0093's path on Windows for a root
    with a directory part, where S0 writes `\` (decision 27);
  - a root whose path is not UTF-8, which N cannot open (decision
    29);
  - 5.1's `parse --dump-ast`, whose near-misses still name it, in the
    base (decision 13);
  - the mutants that pass by design (decision 39).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` under `RUSTUP_TOOLCHAIN=1.78.0`
  with `RUSTFLAGS=-D warnings`. No `std::sync::LazyLock` (1.80), no
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79). 5.2a's Rust
  uses `Option::is_some_and` (1.70), `str::split_once` (1.52), a `[char;
  N]` pattern (1.51) and `std::thread::scope` (1.63), nothing newer.
- **No new crate, and `Cargo.lock` does not change** (spec §6). The order
  is a `Vec` beside each map, not `indexmap` (the user's decision 3). Any
  change to the lock stops the task: ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`. 5.2a's Rust outside tests has none; `Exports`'
  `values[n]` indexes a map by a name its own `Vec` holds. N's `unwrap()`s
  are on values its own code has checked.
- **No `unsafe`.** 5.2a adds none.
- **Tests never touch the internet,** bind no port, and need no `clang`
  (spec §6). The differential runs `nova` and the N it builds, nothing
  else.
- **Temporary names are fixed** (spec §6): under `std::env::temp_dir()`,
  `nova-selfhost-<name>`; 5.2a adds `resolution-headers`,
  `resolution-spellings`, `resolution-roots`, `resolution-escapes`,
  `resolution-non-utf8`, `crlf-resolve`, `names-301` to `names-303` and
  `search-names` (the longer search's) to 5.1's, every `fresh_dir` and
  `agree_on_inputs` name the diff adds; and N's own loader test, under
  Nova's `temp_dir()`, `nova-selfhost-loader-test`
  (`selfhost/src/loader_tests.nova`). These are every fixed temporary
  name in the diff (`grep -n 'fresh_dir(\|agree_on_inputs(\|temp_dir'`
  over its added lines). Each is removed and made afresh by the test that
  uses it. A test that needs `$NOVA_HOME` sets a fresh one.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and the golden suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the suite here with `NOVA_REQUIRE_NODE=1`. No test of 5.2a needs Node.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and
  passes. 5.2a adds no runtime program, so it stays at 147.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary, which only a build
  of the workspace writes. `nova build` of `selfhost/` links against that
  runtime. Every run step starts with `cargo build --locked --workspace`
  (Conventions).
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
  `/tmp/nova-todo-*`, which is never cleaned: this plan's eight runs take
  about 2.4 GB, so begin with room for them (the replay began with
  11G and ended with 11G). Every `cargo` command here
  selects the whole workspace (Conventions), so cargo keeps one copy of
  each test executable. Task 8's Rust 1.78 check uses its own target
  directory, in `/dev/shm`, removed after.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.2a's
  patches holds one. Apply the patches with `patch_block`, from the
  committed plan.
- **No patch adds a CR, a NUL or trailing whitespace.** The inputs that
  need them are written by the tests at run time (the CRLF cases, the
  non-UTF-8 root and sibling names), not kept as files.
- **Nothing writes into the repository but the patches.** `nova build -o`
  writes N under the system's temporary directory and `nova test` writes
  nothing into the package; `git status --short` is empty after every run.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.2a.
- **This machine is Linux.** macOS and Windows results come from CI; what
  differs there is reasoned from the code (decisions 27 and
  32).

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-52a.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-52a-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-52a-<task>.txt"`. A task with a tests
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
  mutants ran under it on 4 CPUs, and peaked at 139 tasks (274 in an earlier replay, before a container restart). Test threads
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
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-52a"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG
  mkdir -p "$NOVA_HOME"
  SH() { cargo test --locked --workspace --test selfhost -- "$@"; }
  S() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test run_tests -- release_golden_suite --nocapture "$@"; }
  ```

  So `SH` runs `selfhost.rs`, the differential and the flags' tests, `S`
  the whole suite, and `G` the golden suite through LLVM as CI's Linux leg
  runs it. `NOVA_HOME` keeps a run by hand off `~/.nova`; every test that
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
  PLAN=docs/superpowers/plans/2026-11-02-phase-5-2a-the-self-hosted-loader-and-resolver.md   # this file, committed
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
  nothing: no 5.2a patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. Task 3's tests patch
  adds 139 files and Task 5's code patch about 2,000 lines: each is one
  block all the same. The non-ASCII characters the patches add are
  `§`, `–`, `—`, `…` and `×`, in comments and records. A block that does not apply means the tree differs from
  the plan's: stop, and record why in the ledger.
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

1. **Windows paths** (`crates/nova-driver/src/program.rs`'s `find`,
   `selfhost/src/loader.nova`'s `find`, `path_parent`, `path_join`,
   `file_name`; `selfhost.rs`'s `agree_on_resolve_cases`): the one path in
   S0's resolution output is E0093's, the root's directory joined with the
   sibling's file name, `dir\ui.nova` on Windows where N writes
   `dir/ui.nova`; the harness runs the std-named case from inside its
   directory, and compares only the bare root on Windows; `nova check`'s
   message is the base's on every system (decision 27) →
   `a_root_spelled_any_way_names_its_siblings_alike`, the case
   `a_sibling_named_for_a_std_module_is_e0093`, the CRLF cases (Tasks 3
   and 6) (mutants 36, 37,
   42, 32; 32 passes here by
   design). The reviewer reads `std::path`'s `parent` and `join` on
   Windows against N's functions and the harness's rule, and the PR's
   Windows leg is the run. **Line endings in checked-in text:** git's
   `core.autocrlf` gives a Windows checkout `\r\n` in every text file no
   `.gitattributes` rule covers, so a test that compares a fixture read by
   `include_str!` or `read_to_string` byte for byte fails there; the first
   fact-check found `dump_test_program.txt` so, and its test now reads
   `\r\n` as `\n` → `the_expected_program_part_may_have_windows_line_endings`
   (Task 2) (mutant 26). The reviewer greps the diff
   for every fixture read and comparison.
2. **N's resolver against S0's, word for word** (`selfhost/src/resolver.nova`,
   `builtins.nova`, `diagnostic.nova` against
   `crates/nova-resolver/src/lib.rs` and `suggest.rs`): every message,
   label, note and fix of the 11 codes; the builtin table's 118 names and
   the three lists; the std tables; `collect_item`'s ids, its
   `type_full_name` mangling and `$default`; `resolve_import`'s errors and
   the glob's order; the fix-dropping step and `would_receive`;
   `import_std_module`; E0102 (decisions 18 and
   19) → the differential on std's part, the corpus, the cases,
   the fuzzer (Tasks 5 and 6) (mutants 46 to
   106). A path no input reaches is the spec's first
   risk (§9 risk 1): the reviewer reads the paths the mutants accept
   (E0084, E0085, E0090, `missing`, `owned`, a reported import) against
   the Rust, since 5.2b is the first to run them.
3. **The S0 refactor's equivalence** (`parse_std`,
   `resolve_program_with_std`, `resolve_program_for`,
   `ResolutionDumps`): every caller but the dump calls
   `resolve_program_for`, now the two in one; `check`, `build`, `run`,
   `test` and the language server print what the base prints; the dump
   registers std first in each program's file database (decision
   26) → `parsing_std_once_resolves_as_resolve_program_for_does`
   (Task 2), `checkcmp.sh` (Task 8, Step 4) (mutants
   21, 22; 22
   passes by design).
4. **The order of E0002s** (`Exports`, `resolve_import`'s glob): the
   names in the order exported, values, types, traits; nothing else
   reordered; no corpus file's `nova check` changes (decision
   11) → `a_glob_imports_clashes_are_reported_in_declaration_order`
   (Task 1), `checkcmp.sh` (mutants 1 to
   3).
5. **The dump on both sides** against ADR 0053 §1: an id, `StdStart`, a
   scope's sorting, exports' order, a label, an escape, a reported import,
   `UNSUPPORTED package`, the header and `IOERR`; an example in the ADR
   that S0 no longer prints (decisions 12 and 14) →
   `dump::tests` (Task 2), the every-form check both ways (Task 6), Task
   7's `adrcheck.sh`.
6. **The hidden flags** (`main.rs`'s `Job`, `DumpResolutionCmd` and
   `from_args`, `dump_resolution`; `CheckCmd` the base's): every `nova
   check` command line whose first argument after `check` is not
   `--dump-resolution` gives the base's stdout, stderr and exit code, its
   tips, usage lines, error order and `--help`/`-h` included, under any
   first argument and in a terminal; the form's own misuses one error and
   exit 2; `--std` with a file refused; an empty file `IOERR NotFound`,
   not a clap error (decisions 13 and 31) → the
   flags' and headers' tests (Task 2), `helpcmp.sh`, `usagecmp.sh` (Task
   8, Step 5) (mutants 23 to 25,
   90).

7. **Roots whose directory is empty, and package detection**
   (`selfhost/src/loader.nova`'s `has` and `in_package`, against
   `crates/nova-driver/src/program.rs`'s listing and `package_of`): a
   root with no directory part (`main.nova`) has the directory `""`,
   which both sides must list as `.`, so a directory named `ui.nova`
   beside it is still E0093's though no read of it succeeds; a root is a
   package's when its directory is named `src` or `tests` and the
   directory above holds a file `nova.toml`, that directory `""` (listed
   as `.`) for `src/main.nova` named from inside the package; a
   `nova.toml` that is a directory makes no package (decision
   30) → `a_root_with_no_directory_part_or_in_a_package_loads_alike`
   and the headers' test (Task 6) (mutants 43,
   86, 87 to
   89). The second fact-check found these three
   mutants passing: today's N was right, and no input showed it.
## Decisions

The user's, from the spec's header (2026-11-02):

1. **5.2 is the loader and the resolver, and HIR moves to 5.3** (the
   user's decision 1): HIR is the type checker's output in S0, so 5.2
   ports what runs before it, and the phase plan says so (Task 7).
2. **N reads std from disk, through `--std DIR`** (the user's decision
   2): the harness passes the repository's `std/`; 5.5's driver finds std
   beside its executable or in `$NOVA_HOME`.
3. **S0 keeps insertion order where an error order depended on a hash
   map** (the user's decision 3): a `Vec` beside each of `Exports`' maps,
   no crate; the CHANGELOG's Changed records it (decision 11).
4. **Two pull requests, 5.2a then 5.2b** (the user's decision 4): this is
   5.2a, loose programs; packages, the WASM target, test mode and the live
   heap are 5.2b's.
5. **The dump is an S-expression like 5.1's, and std's part is dumped
   once** (the user's decision 5): decision 12.
6. **The inputs come from a name-level fuzzer and the Rust tests'
   sources** (the user's decision 6): decisions 15 and 16.
7. **The live heap is measured with outside tools** (the user's decision
   7): 5.2b's, once N resolves all of std under every target; nothing
   here.

The user's ruling of 2026-11-03, asked after the prototype found decision
4 self-contradictory (E0084 and E0085 need test mode, E0090 the WASM
target, both of which decision 4 puts in 5.2b):

8. **The three codes move to 5.2b** (the user's decision 8, 2026-11-03,
   choosing "ย้าย 3 รหัสไป 5.2b (แนะนำ)", move the 3 codes to 5.2b,
   recommended): 5.2a covers the 8 codes reachable on the native `check`
   path and asserts that the other 3 do not occur, in the fuzzer's dumps
   and in the corpus's and the cases'; E0084, E0085 and
   E0090 come with test mode and the WASM target in 5.2b (decision 25).

The spec's "not asked" recommendations, taken; the user can overturn
them:

9. **Diagnostics are compared completely** (spec §3.6): code, severity,
   every label, note and fix, in order, the loader's, the lexer's and the
   parser's among them.
10. **S0's dump is a hidden flag on `nova check`** (spec §3.4): decision
   13.

As built (spec §3):

11. **The order of a glob's E0002s** (spec §3.1): `Exports` gains
    `value_names`, `type_names` and `trait_names`, pushed by
    `export_value`, `export_type` and `export_trait` when a name is new,
    and `values_in_order` and its two siblings; `resolve_import`'s glob
    binds through them, so its E0002s follow declaration order, values
    first. The test is the spec's case, nine names that were `beta,
    delta, epsilon, alpha, gamma`, then `Eta, Zeta`, then `Sigma, Tau`.
    No other output
    depends on a map's order: `import_std_module` and `missing_names`
    bind with `or_insert`, and the namespaces are looked up, never
    iterated into output; `nova check` over every file the base tracks
    prints what the base's does (Task 8, Step 4).
12. **The dump** (spec §3.4; ADR 0053 §1): `nova_resolver::dump`.
    - **Two parts.** A program's part prints its modules (label, package,
      the loader's imports sorted, its importable list), `StdStart`, its
      definitions, its modules' scopes sorted by name and exports in the
      order exported, its items, its splits and its tests, then every
      diagnostic; std's part prints std's modules, definitions, scopes,
      exports, items and `missing` once.
    - **Ids.** A user definition is `u<n>`, its `DefId`; a std one `s<n>`,
      less the first std definition's; modules and items count from 0 in
      each part.
    - **Labels.** A place is a file's label and a span: a module's stem,
      or `<std/core>`.
    - **Strings** are ADR 0052's, `\xHH` for a control character, `"`
      and `\`. The grammar is written out in ADR 0053 with an example of
      every form, regenerated from S0 by Task 7's `adr_examples.py`.
    - **Sizes.** std's native part is 7,728 lines, 713 definitions and
      548 items; a typical program's part is about 180 lines.
13. **The hidden flags** (spec §3.4; ADR 0053 §1): `nova check
    --dump-resolution FILE…` prints `== FILE` (the path as given) and each
    root's part, `IOERR` and the kind for a root it cannot read, then exit
    code 2 after the rest; `UNSUPPORTED package` for a root directly in a
    package's `src/` or `tests/`, 5.2b's; `--std` prints std's part,
    refusing a FILE; no FILE and no `--std` is refused with exit code 2.
    It runs `check`'s pipeline up to the resolver: load, stop on any
    error, strip the `@test` functions, resolve.
    - **`check` stays the base's.** The form is taken only when
      `--dump-resolution` is the first argument after `check`: `main`
      looks before clap parses (`DumpResolutionCmd::from_args`) and parses
      the rest with the form's own parser, and `CheckCmd` is the base's,
      field for field. So every other `nova check` command line, under any
      first argument and in a terminal, gives the base's stdout, stderr and
      exit code: tips, usage lines, which fault clap reports first,
      `--help` and `-h`. This is the smallest honest way: the first
      version declared the flags in `CheckCmd` and re-parsed without them
      to refuse, which clap's suggestions, `--flag=value`, repeats, error
      order and `--help` all leaked (17 of the first fact-check's 24
      shapes); any version that keeps them in `CheckCmd` leaks the
      suggestions at least.
    - **What the form accepts and refuses.** After the flag: `--std`,
      `--target native|wasm` (or `--target=…`) and files, each option
      once, `--` before a file that starts with `-`, an empty file taken
      as one that is not there (`== ` and `IOERR NotFound`, exit 2; the
      headers' test pins it, since clap's default `PathBuf` parser would
      refuse it). Anything else, `--help`, `-h` and a second
      `--dump-resolution` included, is one clap error under the usage
      `nova check --dump-resolution …`, exit 2. `--dump-resolution=…` is
      not the form, so the base's refusal of it stands.
    - **5.1's `parse --dump-ast` leaks the same way** (`parse --dump` is
      told "a similar argument exists: '--dump-ast'"): in the base, out of
      scope, recorded in ADR 0053 §1 as a known 5.1 leak. This mechanism
      does not reach `parse`.
14. **Every form appears, and every form is listed** (spec §3.5):
    `KINDS` lists the 43 forms 5.2a's programs print, `(Import Reported`
    among them since E0093 reports an import on the loose path;
    `LATER_KINDS` the eight only packages, WASM or test mode print. The
    differential checks that every form of `KINDS`, and none of
    `LATER_KINDS`, is in the dumps of std, the corpus and the cases, and,
    since step 0, that every form those dumps print is listed (`listed`;
    a field whose value is a number or an id stands for itself); the unit
    test `the_program_prints_every_form` checks both on its own program.
15. **The cases** (spec §3.5): 103 programs in
    `crates/nova-cli/tests/selfhost/cases/resolve/`, 139 files. 88 are the
    inline sources of `nova-resolver`'s and the type checker's tests that
    reach the resolver, extracted by a script and named
    `<testfile>__<test>[_n]`, a program of several modules a directory whose
    root is `main.nova`; eleven were written by hand in step 0 and four after
    the first fact-check (decision 30). 5.1's token and AST differentials read
    them too (`the_fixed_cases_agree` reads `cases/` whole), and `nova fmt`'s
    gate skips the directory.
16. **The name fuzzer** (spec §3.5): `fuzz::name_mutate` mutates a loose
    corpus program that parses one to three times: renames that collide
    with another item, a builtin, a std name, a built-in type or a
    std-only builtin; an item duplicated or deleted; `pub` toggled; an
    import of a missing module, a sibling, `dom`, `ui`, a nested path, an
    alias or itself; an unknown attribute or a malformed `@test`; an
    alias, a sum hiding std's variants, a trait twice, an `extern` block,
    a record named `Bytes`, a `@test` impl block, impls for `()` and a
    function type. A mutation that would not parse is skipped. Input `i`
    is `f{i:05}.nova` beside four siblings (`NAME_SIBLINGS`): `shapes`
    and `helpers`, and, from the first fact-check, `vault` (a private
    item of each kind a fix can make public, and `hidden` twice, the
    second's keyword on a line of its own) and `keeper` (imports `vault`
    whole and declares a record named as `vault`'s private function
    `Kept`), reached by three more imports. The fact-check's mutants of
    `make_public`'s item kinds and of `binds`' namespaces passed because
    the two first siblings held one private function and nothing else;
    the fuzzer alone now catches them, and the E0083 and `anon` ones
    (ADR 0053 §8). A file system's shape (a `nova.toml` that is a
    directory) is not the fuzzer's: the header test holds it. The seeds
    301 to 303 give 50 inputs each, and the harness asserts that each of
    the eight codes (decision 25) occurs at least twice: E0001 46, E0002
    63, E0082 14, E0083 8, E0089 10, E0092 13, E0102 50, E0900 18.
    The inputs are the same on every system: sorted listings, `\r\n` read
    as `\n`, checked against a Windows checkout's corpus listed in
    reverse.
17. **N's loader** (spec §3.2; ADR 0053 §3): `load_program`'s loose path,
    breadth first, each module lexed and parsed by 5.1's N, each
    single-segment import a file beside the root found by one listing of
    the directory (an exact name; not when only a name differing in case
    is listed; else a read), E0093 for `dom` and `ui`; a module that
    cannot be read skipped, a root that cannot be read `IOERR`; labels by
    stem; the importable lists; `strip_test_functions` as `check` calls
    it; `package_of` as `in_package`.
18. **N's resolver** (spec §3.3; ADR 0053 §4): `resolve_program_for`
    function by function with S0's names, std read and parsed once per
    process and appended after the user's modules, the scopes seeded
    (`GLOBAL`; `STD_ONLY` and on WASM `WASM_ONLY` in std's), pass 1
    `collect_item` with its attribute checks, E0090's path, pass 2
    `resolve_import`, the fix-dropping step, std's glob, E0102,
    `missing_names`. `builtins.nova` holds the 118 builtins' variant
    names and names and the three lists, and the std tables, word for
    word; `diagnostic.nova` the `Diagnostic` record and `suggest.rs`'s
    `distance`, `closest` and `change_to`. N's sizes, in code lines
    (neither blank nor a `//` comment, and nothing from a Rust file's
    first `#[cfg(test)]` on; `$P/plan52a/loc.py`): 2,596 (`resolver.nova`
    1,224 of its 1,486 lines, under the 1,500 past which it would split),
    against `nova-resolver`'s 1,962 (1,921 before), `dump.rs`'s 552 and
    about 250 of `program.rs`'s 646; N's tests 277.
19. **Each Rust idiom has one translation** (ADR 0053 §4's table): ids
    and keys as `Int`s, -1 for none; `Builtin` as its index into the
    table; sums with prefixed variants; a span's file as the module's
    label kept beside it; maps looked up, never iterated into output, the
    dump sorting their keys; `Exports`' name lists for every loop; a
    builder's `mut self` call on a variable, not a temporary; the builtin
    table read once into `BuiltinTable` (a constant array is built again
    on each read, which made the first build 9× slower); `VecDeque` as a
    `Vec` and a head index; paths as `/`-joined strings.
20. **N's dump and command** (spec §3.4): `resolution_dump.nova` prints
    decision 12's grammar byte for byte; `selfhost resolve --std DIR
    FILE…` and `--std DIR --std-part`, with S0's headers, `IOERR`,
    `UNSUPPORTED package` and exit codes. N's own tests, in `src/` as
    5.1's are: three in `loader_tests.nova` (the breadth-first order and
    the imports, what `check` strips, the paths) and five in
    `resolver_tests.nova` (a glob's order, the mangling, a builtin
    redefined, `make_public`'s edits, the attribute fix); `nova test` in
    `selfhost/` runs 25, 5.1's 17 and these 8.
21. **The comparison** (spec §3.5–§3.7): `selfhost.rs` compares S0's and
    N's dumps byte for byte with their exit codes through 5.1's
    `compare_batch`: std's part once; the 251 loose corpus programs (every
    corpus file but those directly in a package's `src/` or `tests/`,
    whose count, 66, the test floors and reports with `eprintln!`, which
    libtest shows only under `--nocapture`, so spec §3.5's "says so" holds
    in such a run and not in CI's log) in batches of 50 side by side,
    beside the 103 cases, none of whose dumps holds E0084, E0085 or E0090
    (decision 8); the cases again with `\r\n` line endings; the name fuzzer's 150 inputs
    (`Dumps::Resolution`); the headers (a missing root, a package's root,
    a root that does not parse, a root beside a `nova.toml` that is a
    directory, a root that is not UTF-8, no root); roots whose directory
    is listed as `.` (decision 30); a root whose label
    needs escapes (not on Windows, which refuses `"` in a name); eleven
    spellings of a root (decision 27); on Linux, a root whose path is not
    UTF-8 (decision 29).

22. **The speed, recorded, not gated** (ADR 0053 §10): with the debug
    runtime CI uses, five runs here after the second fact-check's fixes,
    N resolves and dumps the corpus's loose programs in one process in
    6.3–6.7 s against S0 debug's 3.6–4.3 s, 1.5 to 2.0× (1.53 to 1.81
    by run here, 1.69 to 1.84 in three runs after the first fact-check's
    fixes; the second fact-check measured 1.48, 2.01 and
    1.86: the range holds the noise), and std's part in 0.46–0.52 s
    against 0.082–0.142 s (about 4 to 6×, 3.6 to 6.3 by run): under the
    user's 10× line.
23. **CI's time** (spec §9 risk 4): on `main`'s CI at `1fa7cd3f`,
    `selfhost.rs`, 5.1's tests, took 19.45 s on Windows, 22.50 s on macOS
    and about 19.3 s on Linux (5.1b's PR). This machine's speed varies
    from day to day, so 5.2a's tests are measured against 5.1's 27, run
    alone in the same build, interleaved, on 4 CPUs with the target and
    the temporary directory on the same disk: the replay's three runs of
    the 36 (Task 8, Step 7) took 26.8 to 28.5 s and 5.1's 17.9 to 18.9 s,
    1.42 to 1.59 times; the prototype's, after the second fact-check's
    fixes, 30.9 to 32.3 s against 20.0 to 20.9 s (1.48 to 1.61), and
    after the first, 24.8 to 26.5 s against 16.7 to 17.6 s (1.40 to
    1.58), when the replay's slowest run, 29.5 s, was 1.77 times 5.1's
    fastest. Every
    measurement lies within 1.4 to 1.8 times, so 5.2a's should take about
    27 s (19.3 × 1.4) to 40 s (22.50 × 1.8) on CI (ADR 0053 §9), under
    the minute; no count is cut. (The first fact-check measured 24.3 s,
    and 24 to 26 s over about 20 runs, with the target on tmpfs.) The
    PR's three legs give the figures; past a minute, ask the user.
24. **A change to resolution changes both compilers** (spec §1; phase
    plan decision 6): from 5.2a, a change to how a loose program loads,
    how names bind, the builtin or std tables, the mangling or a message
    changes `nova-resolver` (or the driver's loader) and `selfhost/` in
    one pull request; ADR 0053 §11 and the dated notes say so.

Departures, and rulings the spec leaves open:

25. **Eight of the 11 codes occur in 5.2a's inputs.** **(differs)** The
    spec (§3.5, §3.7) asks for all 11 among CI's seeds. The user's
    decision 4 put both "the 11 codes" in 5.2a and "the WASM target, test
    mode" in 5.2b, and the prototype found that those cannot both hold:
    E0084 and E0085 are given only for a `@test` function that test mode
    keeps (`check` strips every `@test` function before resolving), and
    E0090 only on the WASM target. Settled by the user's ruling of
    2026-11-03 (decision 8): the three codes move to 5.2b with test mode
    and the WASM target. N ports all three paths; the harness asserts the
    other 8 (`NATIVE_RESOLVER_CODES`) and the absence of the 3
    (`LATER_RESOLVER_CODES`); 5.2b moves them to the first list. Cost:
    the three paths run first in 5.2b.
26. **S0 parses std once per dump.** **(differs)** The spec changes S0's
    resolver only for decision 11. `nova_resolver::parse_std` lexes and parses
    a target's std modules, `resolve_program_with_std` resolves a program with
    them, and `resolve_program_for` is the two in one, as before, for every
    other caller; `nova_driver::ResolutionDumps` parses std once and registers
    std's modules first in each program's file database, so ids and spans
    match. Reason: CI's budget; S0 dumped the corpus in 17.8 s parsing std per
    program, 3.6–4.3 s now, and `selfhost.rs` went from 47 s to 33 s at the
    prototype. Cost: two public functions and a struct, and a second path into
    the resolver that only the dump takes; a unit test checks that the two
    paths give the same file, definitions, diagnostics, tests and import
    occurrences for two programs sharing one `ParsedStd`, and `checkcmp.sh`
    that `check` prints what the base prints.
27. **Windows paths: settled in the harness, not in either compiler.**
    **(differs)** The spec asks for every diagnostic word for word (§3.3,
    §3.6). S0's resolution output holds one path, E0093's sibling, which
    `Path::join` writes `dir\ui.nova` on Windows and N `dir/ui.nova`. A
    case directory holding a `dom.nova` or `ui.nova` runs from inside
    itself with its root named `main.nova`, so the path has no directory
    part on any system, and the spellings test compares only the bare
    root on Windows. S0 printing `/` would change `nova check`'s message
    on Windows against the base; N cannot tell the system it runs on; and
    running every case from its own directory costs about 10 s of CPU.
    So `selfhost resolve` with a root that has a directory part writes
    `/` on Windows; 5.5's driver, which owns paths, settles it. Step 0
    also found and fixed the one difference on Linux: `Path::parent`
    trims a `.` component and repeated `/`s (`sub/./main.nova` is in
    `sub`), which N's `path_parent` and `file_name` now do. Reasoned from
    `std::path`; the PR's Windows leg is the proof.
28. **`ImportTarget` lives in `loader.nova`.** **(differs)** The spec asks
    for `nova-resolver`'s structure; S0 keeps `ImportTarget` in the
    resolver crate. N keeps it beside the loader that decides it, so
    Task 4's loader builds and is tested without Task 5's resolver.
29. **What N does not port, and its command line.** Not ported: the
    language server's import occurrences (`Occurrence`, `imports`), which
    no dump prints, and `resolve`, the one-file wrapper only S0's tests
    call. `selfhost resolve` takes no `--target`: 5.2a is native, and
    5.2b adds the flag. Its command line is a test tool's: an empty first
    FILE or a `--` is a root so named (`IOERR NotFound`), where S0's clap
    refuses the one and skips the other; the harness passes neither. A
    root whose path is not UTF-8 (`r\xff.nova`, `sub\xfe/main.nova`, on
    Linux) S0 dumps, under a header with U+FFFD, while N, whose arguments
    arrive as UTF-8 with U+FFFD, opens another name and prints `IOERR
    NotFound`, exit 2, as ADR 0051 records for 5.1; Nova has no byte paths
    before 5.5's driver. Recorded, not settled (ADR 0053 §5);
    `a_root_whose_path_is_not_utf8_is_dumped_by_s0_and_not_found_by_n`
    pins it on Linux.
30. **The loader's edges, probed in step 0:** eleven hand cases (a sibling
    named for a std module, a sibling that fails to lex or parse, a parse
    error with a note, an import cycle, an import of a directory, an import
    differing in case from its file, a root that imports itself, a `@test`
    function that would collide, an empty file, and a `pub` fix dropped
    because it would bind a name twice, through a glob and through a listed
    import; the note and the two fixes close gaps the first run of the mutants
    found), and four more after the first fact-check, whose mutants of N
    passed (a fix dropped when one item's keyword is not on its name's line, a
    fix dropped that would bind a type twice, a `@test` impl block, impls for
    a function type and `()`), with a root beside a `nova.toml` that is a
    directory in the header test; the cases with `\r\n` endings, since git
    gives `cases/` no `.gitattributes` rule and a Windows checkout compares
    them so; a root given as `./x.nova`, an absolute path and with `..`; a
    non-UTF-8 sibling name, which S0's listing drops and N's keeps with
    U+FFFD, unreachable since no identifier holds U+FFFD, tested on Linux
    (macOS refuses such names). A root is a package's only when directly in a
    `src/` or `tests/` whose parent holds a `nova.toml`, so a stray
    `nova.toml` above the repository or the temporary directory changes
    nothing. After the second fact-check, roots whose directory both sides
    list as `.` (the directory is `""`): a root with no directory part
    beside a directory named `ui.nova`, which the listing holds though it
    cannot be read (E0093), a package's `src/main.nova` and
    `tests/main.nova` named from the package's directory, and
    `pkg/tests/main.nova` from outside, with an empty root, in
    `a_root_with_no_directory_part_or_in_a_package_loads_alike`.

31. **`--dump-resolution`'s arguments, probed against the base:** 29
    `parse` and `check` shapes, the first fact-check's 24 `check` shapes
    among them (a near-miss flag such as `--st`, `--stdd`, `--std-part`,
    `--dump`, `--dump-res`; `--std=1` and `--dump-resolution=1`; `--std
    --std`; a second file before a bad `--target`; `--std --help` and
    `-h`; a second file before an unknown flag; the flag not first), under
    six first arguments (`nova`, `othernova`, and four with no UTF-8
    name), and eight in a terminal, are the base's byte for byte, both
    streams and the exit code; the harness's flags test holds 22 of them
    to the base's text. The form's own misuses (no file, `--std` with a
    file, the flag twice, a bad target, `--std` twice, a near-miss,
    `--help`, `-h`) are one error and exit 2, no panic; `--help` of every
    command is the base's (Task 8, Step 5's `usagecmp.sh`, which compared
    only "one error, exit 2" for the refusals before the first
    fact-check, and now compares each with the base).
32. **macOS and Windows, reasoned from the code.** Paths (decision 27);
    line endings (the CRLF cases); a case-insensitive disk, where an
    import differing in case from its file is refused by the listing's
    answer on both sides, `an_import_differing_in_case_finds_no_module`;
    sorted listings for the cases and the fuzzer. CI's three legs run
    them.
33. **The records made true.** ADR 0053 accepted, with the branch; every
    record gives 251 loose corpus programs and says what it counts; the
    dated notes name `claude/loving-archimedes-frtf5q-52a`; the CHANGELOG
    states the user-visible changes, the hidden flag and the E0002 order,
    and nothing else changes for a user; CI's figures replace part B's
    estimate (decision 23).

How the plan is cut:

34. **Seven tasks, six test first, then the records:** the order (1);
    S0's dump, `parse_std` and the flags (2); the cases and the name
    fuzzer through S0's dump (3); N's diagnostics, builtin tables and
    loader, tested by N's `@test`s alone (4); N's resolver, dump and
    command (5); the differential (6); ADR 0053 and the records (7). No
    tree between holds a half-ported resolver that the harness compares:
    the harness compares N's dumps from Task 6 on.
35. **`selfhost.rs` is cut by task.** Tasks 2 to 5 hold S0-only versions
    of the resolution tests, which Task 6 replaces with the comparison:
    Task 2 the flags and the headers on S0; Task 3 the cases' every-form
    check and the codes' floor on S0's dumps, with `s0_resolution` and
    `resolve_cases`; Task 6's tests patch the comparison's tests, and its
    code patch `resolution()`, `std_resolution()`, `agree_on_resolution`,
    `agree_on_resolve_cases` and `Dumps::Resolution`. `main.nova` imports
    each test module in its task's tests patch.
36. **Tests that cannot stand first.** Task 3's and Task 6's tests do
    not compile without their code; Task 4's and Task 5's break N's build,
    so every test that builds N fails. Task 2's flags' test holds `check`
    to the base's `--help` and refusals, which pass before the code, and
    the form's own misuses to its usage line, which fail before it. Each
    step says so.
37. **Every commit builds correct code and passes the suite.** Each code
    commit was replayed with the whole suite, fmt, clippy and `nova fmt
    --check`.
38. **The process limit** (Conventions): 600 tasks, set by a pids control
    group here, peaking at 139 tasks (274 in an earlier replay, before a container restart).
39. **The mutants** (Task 8, Step 8): 106, 71 of them
    N's. A first run, before step 0 was final, found four gaps, each now
    closed: N dropping a parse error's notes (a case with the match arm's
    note), N's `would_receive` blind to a listed import (a case where a
    listed import makes a `pub` fix clash), N's strings unescaped (a root
    named `q"uo\te.nova`, not on Windows) and the CRLF copy written with
    `\n` (the test checks the copy holds `\r`); one mutant that did not
    build was rewritten. The first fact-check's own mutants of N then
    found six more that passed (O1 to O4, O6, O8: `make_public` keeping a
    fix without one item, losing its `const` arm, `binds` blind to types,
    E0083's "an impl block", `type_full_name`'s `anon` fallback, a
    `nova.toml` directory), and O5 (`()`'s `anon`) passed on a rerun; all
    eight of its O1 to O8 are now in this list, caught by four cases, the
    header test and the fuzzer's new siblings (decisions 16 and 30), with
    mutants for the fact-check's new checks: `check` given a hidden
    argument again, the form taken for a near-miss or given `--help`, and
    the fixture compared with its `\r\n`s. The second fact-check's
    mutants then found four more that passed, each a real difference no
    input reached (G1, G2): N listing `""` rather than `.` for a root
    with no directory part, and for a package's directory, N blind to a
    package's `tests/`, and the form's files on clap's `PathBuf` parser,
    which refuses an empty file; all four are now in this list, caught by
    `a_root_with_no_directory_part_or_in_a_package_loads_alike` and the
    headers' test (decisions 13 and 30), with one more, the only mutant
    with a second replacement (G3): both sides keeping the `@test`
    functions, so that E0084 and E0085 reach the corpus's and the cases'
    dumps alike, which the new codes check there catches first. The run
    at the split's last commit: 96
    caught; 10 pass by design, each with its reason:
    paths natively unreachable until 5.2b's packages, WASM target and test mode (62, 76, 77, 78, 95); equivalent (3, 22, 31); caught only on another system, by a separator or a case-insensitive disk (32, 40). The run ends with `cargo build --locked
    --workspace`, so the tree's binaries are the last commit's.

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-52a`: from `7878b9bb`
(`main` after 5.1b, `1fa7cd3f`, then the approved spec in two commits,
`f0796204` and `7878b9bb`), then this plan's commit, which adds this file
alone, so the patches apply to it as to `7878b9bb`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-52a`.
  - `git log --format='%h %s' -1 HEAD~1` prints
    `7878b9bb docs: the Phase 5.2 spec is approved`.
  - `git diff --stat 7878b9bb HEAD` lists only
    `docs/superpowers/plans/2026-11-02-phase-5-2a-the-self-hosted-loader-and-resolver.md`.
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
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-52a.txt" 2>&1
  grep '^test result' "$P/base-52a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-52a.txt"
  cp target/debug/nova "$P/nova-base-52a"
  cargo test --locked --workspace -- --list > "$P/base-list-52a.txt" 2>&1
  grep -c ': test$' "$P/base-list-52a.txt"
  ```

  Expected, from 5.1b's last full run on `1fa7cd3f`, whose code `7878b9bb`
  keeps, and not the replay's: `2841 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. The replay built `7878b9bb` and listed its tests, and
  found `2852` names, the run's 2841 + 1 + 10. The copy of `nova`
  is the base's, for Task 8's comparisons; the list is the base's names.
  Task 8 expects `2857 1 10`: 5.2a's tests make 16 more, 16 added
  and 0 gone.
- [ ] **Step 4: The ledger.** Start `$P/ledger-52a.md`, headed with the
  base commit, the baseline's figures and the process limit's form.

### Task 1: A glob import's E0002s in declaration order

Spec §3.1; decisions 3 and 11.

**Files:**
- Modify: `crates/nova-resolver/src/lib.rs` (`Exports`, `insert_value`,
  `insert_type`, `collect_item`'s traits, `resolve_import`'s glob, and a
  test)

- [ ] **Step 1: The test.** `patch_block t1-tests`: the resolver's inline
  test `a_glob_imports_clashes_are_reported_in_declaration_order`, a
  module of five functions, two records and two traits that imports a
  module exporting the same nine names; the nine E0002s must come in
  declaration order, values, then types, then traits.
- [ ] **Step 2: Run, and see it fail.** First the build, which ends with
  `Finished`. Then
  `cargo test --locked --workspace --lib -- a_glob_imports_clashes_are_reported_in_declaration_order`.
  Expected: `0 passed; 1 failed`: the nine E0002s come in the map's order, `beta`, `delta`, `epsilon`, `alpha`, `gamma`, then `Eta`, `Zeta`, then `Sigma`, `Tau`.
- [ ] **Step 3: The code.** `patch_block t1-code`: `Exports` gains
  `value_names`, `type_names` and `trait_names`, `export_value`,
  `export_type` and `export_trait`, which push a name the map did not
  hold, and `values_in_order`, `types_in_order` and `traits_in_order`;
  `insert_value`, `insert_type` and `collect_item`'s traits export through
  them; `resolve_import`'s glob binds in their order.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - The test of Step 2. Expected: `1 passed; 0 failed`.
  - `S` (4 min 55 s). Expected: `2842 1 10`: the base's figures and
    Task 1's test, the one failure the IPv6 test.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `resolver: a glob import's E0002s in declaration order`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index a3ad6f2d320e95e7c8b9328286400be83c0d3024..907ea9b98907d15432e4dc95f71e98d027ba4646 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -3631,6 +3631,40 @@ mod tests {
         assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
     }
 
+    /// A glob import's E0002s for the names it clashes with come out in the
+    /// order the imported module declared them, values, then types, then
+    /// traits, whatever a hash map's order (spec 5.2 §3.1). Before, they
+    /// came in `FxHash` order: `beta, delta, epsilon, alpha, gamma`, then
+    /// `Eta, Zeta`.
+    #[test]
+    fn a_glob_imports_clashes_are_reported_in_declaration_order() {
+        let p = resolve_two(
+            "fn alpha() -> Int { 10 }\nfn beta() -> Int { 20 }\nfn gamma() -> Int { 30 }\n\
+             fn delta() -> Int { 40 }\nfn epsilon() -> Int { 50 }\n\
+             record Zeta { y: Int }\nrecord Eta { y: Int }\ntrait Tau {}\ntrait Sigma {}\n\
+             import lib\nfn main() { }\n",
+            "pub fn alpha() -> Int { 1 }\npub fn beta() -> Int { 2 }\n\
+             pub fn gamma() -> Int { 3 }\npub fn delta() -> Int { 4 }\n\
+             pub fn epsilon() -> Int { 5 }\npub record Zeta { x: Int }\n\
+             pub record Eta { x: Int }\npub trait Tau {}\npub trait Sigma {}\n",
+        );
+        let messages: Vec<&str> = p.diagnostics.iter().map(|d| d.message.as_str()).collect();
+        assert_eq!(
+            messages,
+            [
+                "`alpha` is already defined or imported in this module",
+                "`beta` is already defined or imported in this module",
+                "`gamma` is already defined or imported in this module",
+                "`delta` is already defined or imported in this module",
+                "`epsilon` is already defined or imported in this module",
+                "type `Zeta` is already defined or imported in this module",
+                "type `Eta` is already defined or imported in this module",
+                "trait `Tau` is already defined or imported in this module",
+                "trait `Sigma` is already defined or imported in this module",
+            ]
+        );
+    }
+
     #[test]
     fn qualified_import_path_is_rejected() {
         // `import lib::extra` is a qualified/nested path; it must be reported,
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 907ea9b98907d15432e4dc95f71e98d027ba4646..7d262bd88b5a55aedde2b9bd7f01b15940f464f8 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -1833,11 +1833,70 @@ pub fn name_imports(modules: &mut [ModuleSource]) {
 }
 
 /// The public exports of one module (its `pub` items), used to resolve imports.
+///
+/// Each namespace's names are kept in a `Vec` beside its map, in the order
+/// they were exported, which is the order they were declared: a glob import
+/// binds them, and reports the ones it clashes with, in that order rather
+/// than the map's (spec
+/// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+/// §3.1), so the self-hosted resolver, whose maps hash otherwise, gives the
+/// same errors in the same order.
 #[derive(Debug, Default)]
 struct Exports {
     values: FxHashMap<String, Res>,
     types: FxHashMap<String, DefId>,
     traits: FxHashMap<String, DefId>,
+    value_names: Vec<String>,
+    type_names: Vec<String>,
+    trait_names: Vec<String>,
+}
+
+impl Exports {
+    /// Export `name` as a value. A module binds a name once per namespace,
+    /// so no name is exported twice.
+    fn export_value(&mut self, name: String, res: Res) {
+        if self.values.insert(name.clone(), res).is_none() {
+            self.value_names.push(name);
+        }
+    }
+
+    /// Export `name` as a type.
+    fn export_type(&mut self, name: String, id: DefId) {
+        if self.types.insert(name.clone(), id).is_none() {
+            self.type_names.push(name);
+        }
+    }
+
+    /// Export `name` as a trait.
+    fn export_trait(&mut self, name: String, id: DefId) {
+        if self.traits.insert(name.clone(), id).is_none() {
+            self.trait_names.push(name);
+        }
+    }
+
+    /// The exported values, in the order they were exported.
+    fn values_in_order(&self) -> Vec<(String, Res)> {
+        self.value_names
+            .iter()
+            .map(|n| (n.clone(), self.values[n]))
+            .collect()
+    }
+
+    /// The exported types, in the order they were exported.
+    fn types_in_order(&self) -> Vec<(String, DefId)> {
+        self.type_names
+            .iter()
+            .map(|n| (n.clone(), self.types[n]))
+            .collect()
+    }
+
+    /// The exported traits, in the order they were exported.
+    fn traits_in_order(&self) -> Vec<(String, DefId)> {
+        self.trait_names
+            .iter()
+            .map(|n| (n.clone(), self.traits[n]))
+            .collect()
+    }
 }
 
 fn push_def(defs: &mut Vec<Def>, def: Def) -> DefId {
@@ -2686,7 +2745,7 @@ fn collect_item(
             } else {
                 scope.traits.insert(name.clone(), id);
                 if pubv {
-                    exp.traits.insert(name, id);
+                    exp.export_trait(name, id);
                 }
             }
             // Default-method bodies become their own method defs (global);
@@ -3146,29 +3205,16 @@ fn resolve_import(
     }
     match &imp.kind {
         ImportKind::Simple => {
-            // Glob: bring all of the target module's public names into scope.
-            let names: Vec<(String, Res)> = exports[target]
-                .values
-                .iter()
-                .map(|(k, v)| (k.clone(), *v))
-                .collect();
-            for (n, r) in names {
+            // Glob: bring all of the target module's public names into
+            // scope, in the order they were declared, so the conflicts come
+            // out in that order (spec 5.2 §3.1).
+            for (n, r) in exports[target].values_in_order() {
                 bind_value(&mut definitions.modules[mid], diagnostics, n, span, r);
             }
-            let tys: Vec<(String, DefId)> = exports[target]
-                .types
-                .iter()
-                .map(|(k, v)| (k.clone(), *v))
-                .collect();
-            for (n, id) in tys {
+            for (n, id) in exports[target].types_in_order() {
                 bind_type(&mut definitions.modules[mid], diagnostics, n, span, id);
             }
-            let trs: Vec<(String, DefId)> = exports[target]
-                .traits
-                .iter()
-                .map(|(k, v)| (k.clone(), *v))
-                .collect();
-            for (n, id) in trs {
+            for (n, id) in exports[target].traits_in_order() {
                 bind_trait(&mut definitions.modules[mid], diagnostics, n, span, id);
             }
         }
@@ -3338,7 +3384,7 @@ fn insert_value(
     }
     first.insert(name.clone(), span);
     if is_pub {
-        exp.values.insert(name.clone(), res);
+        exp.export_value(name.clone(), res);
     }
     scope.values.insert(name, res);
 }
@@ -3365,7 +3411,7 @@ fn insert_type(
     }
     first.insert(name.clone(), span);
     if is_pub {
-        exp.types.insert(name.clone(), id);
+        exp.export_type(name.clone(), id);
     }
     scope.types.insert(name, id);
 }
````

### Task 2: S0's resolution dump, `parse_std`, and the hidden flags

Spec §3.4; decisions 5, 9, 10, 12 to 14 and 26.

**Files:**
- Create: `crates/nova-resolver/src/dump.rs` (`dump_program`, `dump_std`,
  `std_part_of`, `KINDS`, `LATER_KINDS`, `forms_on`, `listed`, and six
  unit tests), `crates/nova-resolver/src/dump_test_program.txt`
- Modify: `crates/nova-resolver/src/lib.rs` (`pub mod dump`, `ParsedStd`,
  `parse_std`, `resolve_program_with_std`), `crates/nova-driver/src/lib.rs`
  (`resolution_dump`, `ResolutionDumps`, `dump_std_resolution`,
  `register_std`), `crates/nova-cli/src/cmd/run.rs` (`DumpResolutionCmd`,
  its `from_args`, `dump_resolution`; `CheckCmd` unchanged),
  `crates/nova-cli/src/main.rs` (`Job`: the form looked for before clap
  parses), `crates/nova-cli/src/cmd/parse.rs` (`io_kind_name` shared)
- Test: `crates/nova-cli/tests/selfhost.rs`

- [ ] **Step 1: The tests.** `patch_block t2-tests`: `selfhost.rs`'s 5.2a
  section begins, on S0 alone:
  - `the_resolution_dump_flags_are_hidden_and_refused_as_before`:
    `check --help` names neither flag, and `check --help
    --dump-resolution` prints it; 22 shapes, the first fact-check's among
    them (`--st`, `--stdd`, `--std-part`, `--dump`, `--dump-res`,
    `--std=1`, `--dump-resolution=1`, `--std --std`, `a.nova b.nova
    --target bogus`, `--std --help`, `--std -h`, `a.nova b.nova --bogus`,
    the flag after a file or after `--target`), each refused with the
    base's stderr byte for byte, its tip and usage line included, exit 2,
    nothing on stdout; the form's misuses (`--stdd`, `--std --std`, a bad
    target, `--help`, `-h`, the flag twice) one error under the form's
    usage line, exit 2;
  - in 5.1's `the_token_dump_flag_is_hidden`, the renamed binary's run
    waits out ETXTBSY: a copy, closed, can still be open for writing in a
    child another test's thread forked and has not yet exec'd (the first
    fact-check saw it flake once in about 20 runs, across file systems);
  - `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`: a
    root's part after `== a.nova`, its first lines and its end; `IOERR
    NotFound`; `UNSUPPORTED package` for `pkg/src/main.nova`; a root that
    does not parse is `resolved=false` with its P0001; exit 2; `IOERR
    InvalidData` for a root that is not UTF-8; no root, or a root with
    `--std`, exit 2 and nothing on stdout; an empty root, `== ` and
    `IOERR NotFound`, exit 2, nothing on stderr (decision 13); `--std`
    for both targets.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. Then `SH the_resolution_dump`. Expected: `0 passed; 2 failed`. The flags' test passes its 22 shapes and `--help`, the base's own (decision 36), then fails at the form's first misuse, `--stdd`: the base refuses `--dump-resolution` itself under `check`'s usage line. The headers' test fails, since the base's clap refuses `--dump-resolution` and nothing reaches stdout.
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`dump.rs`:** the grammar of decision 12, `KINDS` and `LATER_KINDS`
    (decision 14), `forms_on` and `listed`; its tests: a program of two
    modules, one with every kind of definition, binding, split and
    diagnostic, printed as `dump_test_program.txt` holds, less its
    bindings of std's names and of builtins, each `\r\n` of the fixture
    read as `\n`, since a Windows checkout gives it CRLF (no
    `.gitattributes` rule covers `.txt`), and again against a CRLF copy;
    every form of `KINDS` in it,
    its std's part or a stopped program's, and every form it prints
    listed; std's part the same as a program of no module's; every kind
    of definition, value and severity listed; and
    `parsing_std_once_resolves_as_resolve_program_for_does`.
  - **`lib.rs`:** `resolve_program_for` becomes `parse_std` then
    `resolve_program_with_std` (decision 26).
  - **The driver:** `ResolutionDumps::new` parses std once;
    `ResolutionDumps::dump` loads a root, stops on any error, strips the
    `@test` functions, resolves, and dumps; `dump_std_resolution`;
    `register_std`, which `FrontendContext` shares.
  - **`run.rs` and `main.rs`:** `DumpResolutionCmd`, the form's own
    parser (`--std`, `--target`, the files; no `--help`), which `main`
    takes only when `--dump-resolution` is the first argument after
    `check` (decision 13); `dump_resolution`'s headers, `IOERR` and exit
    codes. `CheckCmd` is untouched.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH the_resolution_dump`. Expected: `2 passed; 0 failed`.
  - `cargo test --locked --workspace --lib -- dump::tests`. Expected:
    `4 passed`, `2 passed` and `6 passed`: `nova-ast`'s and `nova-parser`'s dump tests, as before, and `nova-resolver`'s six.
  - `S` (5 min 4 s). Expected: `2850 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `resolver, driver, cli: the resolution dump, and the hidden nova check --dump-resolution`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 60b48178099e5775936dbea7aa743b75ecc5121e..ad2efd024839a4505202263e4d280edd4968274e 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -26,6 +26,12 @@
 //! big-stack thread, which keeps a panic's message and exit code, and
 //! leaves `nova run`'s program on the main thread.
 //!
+//! 5.2a (spec
+//! `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+//! §3.4; ADR 0053): S0's hidden `nova check --dump-resolution FILE…` and
+//! `--std`, absent from `--help`, with every other `check` command line
+//! parsed as before, a header per root and std's part its own.
+//!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
 //!
@@ -210,10 +216,22 @@ fn the_token_dump_flag_is_hidden() {
     if std::fs::hard_link(nova(), &renamed).is_err() {
         std::fs::copy(nova(), &renamed).expect("copy nova");
     }
-    let extra = Command::new(&renamed)
-        .args(["parse", "a.nova", "b.nova"])
-        .output()
-        .expect("run nova under another name");
+    // A copy, closed as it is, may still be open for writing in a child
+    // that another test's thread forked meanwhile and has not yet exec'd:
+    // running it then fails with ETXTBSY (26) until that child execs.
+    let mut tries = 0;
+    let extra = loop {
+        match Command::new(&renamed)
+            .args(["parse", "a.nova", "b.nova"])
+            .output()
+        {
+            Err(e) if cfg!(unix) && e.raw_os_error() == Some(26) && tries < 50 => {
+                tries += 1;
+                std::thread::sleep(Duration::from_millis(100));
+            }
+            out => break out.expect("run nova under another name"),
+        }
+    };
     let stderr = String::from_utf8_lossy(&extra.stderr);
     let usage = format!("\nUsage: nova-renamed{EXE_SUFFIX} parse [OPTIONS] <FILE>\n");
     assert!(stderr.contains(&usage), "{stderr}");
@@ -1343,3 +1361,236 @@ fn nova_run_runs_the_program_on_the_main_thread() {
     assert_ne!(code, Some(0), "{stderr}");
     assert!(stderr.contains("has overflowed its stack"), "{stderr}");
 }
+
+// --- 5.2a: the resolution dump (spec
+// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+// §3.4, §3.5; ADR 0053) ---
+
+/// S0's arguments for the resolution dump, each file after them a
+/// program's root.
+const RESOLUTION_S0: &[&str] = &["check", "--dump-resolution"];
+
+/// `nova check`'s help names neither hidden flag, and every `nova check`
+/// command line whose first argument after `check` is not
+/// `--dump-resolution` is parsed as before the flags (ADR 0053 §1): each
+/// shape here gives the base's error, byte for byte, tips and usage line
+/// included, which names neither flag; with `--help` before the flag,
+/// `check`'s help. The dump's own parser refuses its misuses with one
+/// error and the exit code 2.
+#[test]
+fn the_resolution_dump_flags_are_hidden_and_refused_as_before() {
+    let help = Command::new(nova())
+        .args(["check", "--help"])
+        .output()
+        .expect("run nova check --help");
+    let help = String::from_utf8_lossy(&help.stdout).into_owned();
+    assert!(help.contains("--target"), "{help}");
+    assert!(!help.contains("dump-resolution"), "{help}");
+    assert!(!help.contains("--std"), "{help}");
+    let out = Command::new(nova())
+        .args(["check", "--help", "--dump-resolution"])
+        .output()
+        .expect("run nova check --help --dump-resolution");
+    assert_eq!(out.status.code(), Some(0));
+    assert_eq!(String::from_utf8_lossy(&out.stdout), help);
+    // The base's errors: (arguments, the argument named, the usage line's
+    // arguments); a tip follows a refused flag, as clap gives it.
+    let (opts, file, target) = ("[OPTIONS] [FILE]", "<FILE>", "--target <TARGET> [FILE]");
+    for (args, named, usage) in [
+        (&["check", "a.nova", "b.nova"][..], "b.nova", opts),
+        (&["check", "a.nova", ""], "", opts),
+        (&["check", "--std"], "--std", opts),
+        (&["check", "--std", "a.nova"], "--std", opts),
+        (&["check", "--", "a.nova", "b.nova"], "b.nova", opts),
+        (
+            &["check", "--target", "wasm", "a.nova", "b.nova"],
+            "b.nova",
+            opts,
+        ),
+        (&["check", "a.nova", "--std"], "--std", file),
+        (&["check", "--st", "a.nova"], "--st", opts),
+        (&["check", "--stdd", "a.nova"], "--stdd", opts),
+        (&["check", "--std-part", "a.nova"], "--std-part", opts),
+        (&["check", "--dump", "a.nova"], "--dump", opts),
+        (&["check", "--dump-res", "a.nova"], "--dump-res", opts),
+        (
+            &["check", "--dump-resolutio", "a.nova"],
+            "--dump-resolutio",
+            opts,
+        ),
+        (&["check", "--std=1", "a.nova"], "--std", opts),
+        (
+            &["check", "--dump-resolution=1", "a.nova"],
+            "--dump-resolution",
+            opts,
+        ),
+        (&["check", "--std", "--std", "a.nova"], "--std", opts),
+        (
+            &["check", "a.nova", "b.nova", "--target", "bogus"],
+            "b.nova",
+            opts,
+        ),
+        (&["check", "--std", "--help"], "--std", opts),
+        (&["check", "--std", "-h"], "--std", opts),
+        (&["check", "a.nova", "b.nova", "--bogus"], "b.nova", opts),
+        (
+            &["check", "a.nova", "--dump-resolution"],
+            "--dump-resolution",
+            file,
+        ),
+        (
+            &["check", "--target", "native", "--dump-resolution", "a.nova"],
+            "--dump-resolution",
+            target,
+        ),
+    ] {
+        let out = Command::new(nova())
+            .args(args)
+            .output()
+            .expect("run nova check");
+        let stderr = String::from_utf8_lossy(&out.stderr);
+        let tip = if named.starts_with('-') {
+            format!("  tip: to pass '{named}' as a value, use '-- {named}'\n\n")
+        } else {
+            String::new()
+        };
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
+    // The dump's misuses, which the base has no form of.
+    for args in [
+        &["--stdd"][..],
+        &["--std", "--std"],
+        &["--target", "bogus", "a.nova"],
+        &["--help"],
+        &["-h"],
+        &["--dump-resolution", "a.nova"],
+    ] {
+        let out = Command::new(nova())
+            .args(RESOLUTION_S0)
+            .args(args)
+            .output()
+            .expect("run nova check --dump-resolution");
+        let stderr = String::from_utf8_lossy(&out.stderr);
+        assert_eq!(out.status.code(), Some(2), "{args:?}: {stderr}");
+        assert!(stderr.starts_with("error: "), "{args:?}: {stderr}");
+        assert_eq!(stderr.matches("error:").count(), 1, "{args:?}: {stderr}");
+        // A usage line, when clap gives one, is the dump's own.
+        assert_eq!(
+            stderr.matches("Usage: ").count(),
+            stderr
+                .matches("Usage: nova check --dump-resolution ")
+                .count(),
+            "{args:?}: {stderr}"
+        );
+        assert!(out.stdout.is_empty(), "{args:?}");
+    }
+}
+
+/// Each root's part follows a header with its path as given; a root that
+/// cannot be read is `IOERR` and its kind, the rest are dumped, and the
+/// exit code is 2; a root in a package is `UNSUPPORTED package`; a program
+/// that fails to load holds the diagnostics so far; `--std` prints std's
+/// part for the target.
+#[test]
+fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
+    let dir = fresh_dir("resolution-headers");
+    std::fs::write(dir.join("a.nova"), "fn main() {}\n").expect("write a.nova");
+    std::fs::write(dir.join("b.nova"), "fn g( {").expect("write b.nova");
+    std::fs::create_dir_all(dir.join("pkg").join("src")).expect("create pkg/src");
+    std::fs::write(dir.join("pkg").join("nova.toml"), "").expect("write nova.toml");
+    std::fs::write(
+        dir.join("pkg").join("src").join("main.nova"),
+        "fn main() {}\n",
+    )
+    .expect("write pkg/src/main.nova");
+    let out = Command::new(nova())
+        .current_dir(&dir)
+        .args(RESOLUTION_S0)
+        .args(["a.nova", "missing.nova", "pkg/src/main.nova", "b.nova"])
+        .output()
+        .expect("run nova check --dump-resolution");
+    let stdout = String::from_utf8_lossy(&out.stdout);
+    assert_eq!(out.status.code(), Some(2), "{stdout}");
+    assert!(
+        stdout.starts_with(
+            "== a.nova\n(Program target=native resolved=true\n  \
+             (Module 0 \"a\" package=None)\n  (StdStart module=1 def=1 item=1)\n  \
+             (Def.Fn u0 \"main\" \"a\":3..7 item=0)\n  (Scope 0\n"
+        ),
+        "{stdout}"
+    );
+    assert!(
+        stdout.contains(
+            "  (Items 0..1 0))\n(Diagnostics)\n\
+             == missing.nova\nIOERR NotFound\n\
+             == pkg/src/main.nova\nUNSUPPORTED package\n\
+             == b.nova\n(Program target=native resolved=false\n  \
+             (Module 0 \"b\" package=None))\n(Diagnostics\n  \
+             (Error \"P0001\" \"expected identifier (in parameter name), found `{`\"\n    \
+             (Primary \"b\":6..7 \"here\"))\n"
+        ),
+        "{stdout}"
+    );
+    // A root that is not UTF-8 is `IOERR InvalidData`.
+    std::fs::write(dir.join("latin1.nova"), b"fn \xe9() {}\n").expect("write latin1.nova");
+    let out = run(&nova(), RESOLUTION_S0, &dir, &["latin1.nova".to_string()])
+        .unwrap_or_else(|e| panic!("{e}"));
+    assert_eq!(out.code, Some(2), "{}", out.stderr);
+    assert_eq!(
+        String::from_utf8_lossy(&out.stdout),
+        "== latin1.nova\nIOERR InvalidData\n"
+    );
+    // No root: nothing printed, and 2; nor a root with `--std`.
+    for args in [&[][..], &["--std", "a.nova"]] {
+        let out = Command::new(nova())
+            .current_dir(&dir)
+            .args(RESOLUTION_S0)
+            .args(args)
+            .output()
+            .expect("run nova check --dump-resolution");
+        assert_eq!(out.status.code(), Some(2), "{args:?}");
+        assert!(out.stdout.is_empty(), "{args:?}");
+    }
+    // An empty root is a file that is not there (ADR 0053 §1), not a
+    // refused argument.
+    let out = Command::new(nova())
+        .current_dir(&dir)
+        .args(RESOLUTION_S0)
+        .arg("")
+        .output()
+        .expect("run nova check --dump-resolution \"\"");
+    assert_eq!(out.status.code(), Some(2));
+    assert_eq!(
+        String::from_utf8_lossy(&out.stdout),
+        "== \nIOERR NotFound\n"
+    );
+    assert!(out.stderr.is_empty());
+    for (target, head) in [
+        ("native", "(Std target=native modules=21 "),
+        ("wasm", "(Std target=wasm modules=16 "),
+    ] {
+        let out = Command::new(nova())
+            .args(RESOLUTION_S0)
+            .args(["--std", "--target", target])
+            .output()
+            .expect("run nova check --dump-resolution --std");
+        let stdout = String::from_utf8_lossy(&out.stdout);
+        assert_eq!(out.status.code(), Some(0), "{target}");
+        assert!(stdout.starts_with(head), "{target}: {}", &stdout[..80]);
+        assert!(stdout.ends_with("))\n(Diagnostics)\n"), "{target}");
+        assert!(
+            !stdout.contains(" u"),
+            "{target}: std's part names a user definition"
+        );
+    }
+}
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-cli/src/cmd/parse.rs b/crates/nova-cli/src/cmd/parse.rs
index 9e3ac5e1ee5f2f933a9baf0d86bc00b00c310296..bede921ec3bd45504ed9a37867e21f8a24b779b1 100644
--- a/crates/nova-cli/src/cmd/parse.rs
+++ b/crates/nova-cli/src/cmd/parse.rs
@@ -194,7 +194,7 @@ fn dump_each(files: &[&Path], dump: fn(&str) -> String) -> Result<i32> {
 /// The kind as std/io's `IoErrorKind` names it, so N's dump, whose reads
 /// fail with those kinds, prints the same word: the seven kinds it has, and
 /// `Other` for every other.
-fn io_kind_name(kind: std::io::ErrorKind) -> &'static str {
+pub(crate) fn io_kind_name(kind: std::io::ErrorKind) -> &'static str {
     use std::io::ErrorKind;
     match kind {
         ErrorKind::NotFound => "NotFound",
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index 4a80950e3b62257ad1a2864100cb723cfc914cba..ba8bb73ba30a785afaff67ba05f4e925a2d84577 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -12,6 +12,7 @@ use std::path::{Component, Path, PathBuf};
 use std::process::{Command, Stdio};
 
 use anyhow::{bail, Context, Result};
+use clap::builder::{OsStringValueParser, TypedValueParser};
 use clap::Args;
 use nova_driver::{Outcome, Program, Roots};
 
@@ -477,6 +478,105 @@ pub fn check(cmd: CheckCmd) -> Result<()> {
     }
 }
 
+/// `nova check --dump-resolution …` (spec
+/// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+/// §3.4, ADR 0053 §1): the resolution dump, for the self-hosted resolver's
+/// differential test. Hidden, with no stability promise. It is taken only
+/// when `--dump-resolution` is the first argument after `check`
+/// ([`DumpResolutionCmd::from_args`]), and then parsed by this parser of
+/// its own: so [`CheckCmd`] stays as it was, and every other `nova check`
+/// command line is parsed, refused, suggested and helped exactly as
+/// before the flag. After the flag it takes `--std`, `--target native|wasm`
+/// and files, `--` before a file that starts with `-`, each option once;
+/// it refuses everything else, `--help` and `-h` included, in clap's words
+/// under the usage `nova check --dump-resolution`.
+#[derive(clap::Parser)]
+#[command(
+    name = "nova check --dump-resolution",
+    bin_name = "nova check --dump-resolution",
+    no_binary_name = true,
+    disable_help_flag = true,
+    disable_version_flag = true
+)]
+pub struct DumpResolutionCmd {
+    /// std's part of the dump, which is the same in every program for the
+    /// target, instead of a program's.
+    #[arg(long)]
+    std: bool,
+
+    /// The target whose std and rules the programs are resolved with.
+    #[arg(long, value_enum, default_value_t = TargetArg::Native)]
+    target: TargetArg,
+
+    /// Each a program's root. Taken as they are, so that an empty one is a
+    /// file that is not there.
+    #[arg(value_parser = OsStringValueParser::new().map(PathBuf::from))]
+    files: Vec<PathBuf>,
+}
+
+impl DumpResolutionCmd {
+    /// The hidden command, parsed from `args` (a process's arguments,
+    /// `argv[0]` first), when they are `check --dump-resolution` and more;
+    /// `None` for every other command line, which the ordinary parser
+    /// takes. A misuse after the flag exits as clap's errors do.
+    pub fn from_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Option<Self> {
+        let mut args = args.into_iter().skip(1);
+        if args.next()? != "check" || args.next()? != "--dump-resolution" {
+            return None;
+        }
+        Some(<Self as clap::Parser>::parse_from(args))
+    }
+}
+
+/// Run [`DumpResolutionCmd`]: with `--std`, std's part for the target;
+/// otherwise, for each file, `== FILE` (the path as given), then its
+/// program's part, or `IOERR` and the error's kind for a root that cannot
+/// be read, after which the rest are dumped and the exit code is 2, as
+/// `parse --dump-ast` does. With neither a file nor `--std`, or with both,
+/// it prints one line to stderr, nothing to stdout, and exits 2, as N
+/// does.
+pub fn dump_resolution(cmd: DumpResolutionCmd) -> Result<()> {
+    let code = dump_resolution_code(&cmd)?;
+    if code != 0 {
+        std::process::exit(code);
+    }
+    Ok(())
+}
+
+fn dump_resolution_code(cmd: &DumpResolutionCmd) -> Result<i32> {
+    let target: nova_driver::Target = cmd.target.into();
+    let stdout = std::io::stdout();
+    let mut out = stdout.lock();
+    if cmd.std {
+        if !cmd.files.is_empty() {
+            eprintln!("error: --dump-resolution --std takes no FILE");
+            return Ok(2);
+        }
+        out.write_all(nova_driver::dump_std_resolution(target).as_bytes())?;
+        out.flush()?;
+        return Ok(0);
+    }
+    if cmd.files.is_empty() {
+        eprintln!("error: --dump-resolution needs a FILE, or --std");
+        return Ok(2);
+    }
+    // std is parsed once for every program, as N parses it.
+    let dumps = nova_driver::ResolutionDumps::new(target);
+    let mut code = 0;
+    for file in &cmd.files {
+        writeln!(out, "== {}", file.display())?;
+        match dumps.dump(file) {
+            Ok(dump) => out.write_all(dump.as_bytes())?,
+            Err(e) => {
+                writeln!(out, "IOERR {}", crate::cmd::parse::io_kind_name(e.kind()))?;
+                code = 2;
+            }
+        }
+    }
+    out.flush()?;
+    Ok(code)
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
diff --git a/crates/nova-cli/src/main.rs b/crates/nova-cli/src/main.rs
index f90706bf9497f44c3c952478299ee40ad60c233d..69d679e774c796b5ecaf242c5374c995bec60d5e 100644
--- a/crates/nova-cli/src/main.rs
+++ b/crates/nova-cli/src/main.rs
@@ -98,15 +98,26 @@ enum Ran {
     Program(Box<cmd::run::Compiled>),
 }
 
+/// What a command line asks for: a command [`Cli`] parses, or the hidden
+/// `nova check --dump-resolution`, which has a parser of its own, so that
+/// `check`'s is as it was ([`cmd::run::DumpResolutionCmd`]).
+enum Job {
+    Cli(Command),
+    DumpResolution(cmd::run::DumpResolutionCmd),
+}
+
 fn main() -> Result<()> {
-    let cli = Cli::parse();
+    let job = match cmd::run::DumpResolutionCmd::from_args(std::env::args_os()) {
+        Some(cmd) => Job::DumpResolution(cmd),
+        None => Job::Cli(Cli::parse().command),
+    };
     let filter = tracing_subscriber::EnvFilter::from_default_env()
         .add_directive(tracing::Level::WARN.into());
     // Under `nova lsp`, stdout carries only protocol messages, so logs (and
     // the `log` records tracing forwards, lsp-server's included) go to
     // stderr (spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md
     // §6.1). Every other command keeps its subscriber as it was.
-    if matches!(cli.command, Command::Lsp) {
+    if matches!(job, Job::Cli(Command::Lsp)) {
         tracing_subscriber::fmt()
             .with_env_filter(filter)
             .with_writer(std::io::stderr)
@@ -125,11 +136,13 @@ fn main() -> Result<()> {
     // as it did, with the exit code 101, and is not printed again. A
     // stack overflow aborts the process from either thread, and
     // `std::process::exit` ends it from either.
-    let command = cli.command;
     let thread = std::thread::Builder::new()
         .name("main".to_string())
         .stack_size(nova_driver::PIPELINE_STACK)
-        .spawn(move || dispatch(command));
+        .spawn(move || match job {
+            Job::Cli(command) => dispatch(command),
+            Job::DumpResolution(cmd) => Ran::Done(cmd::run::dump_resolution(cmd)),
+        });
     let ran = match thread {
         Ok(thread) => thread.join(),
         Err(e) => anyhow::bail!("could not start the compiler's thread: {e}"),
diff --git a/crates/nova-driver/src/lib.rs b/crates/nova-driver/src/lib.rs
index 585fe35f55b96b8287e4f55afbfada87172516f4..a0d536771c8793c83b5d0e33e0e24ab7d268ff1e 100644
--- a/crates/nova-driver/src/lib.rs
+++ b/crates/nova-driver/src/lib.rs
@@ -13,6 +13,9 @@ mod runtime_cache;
 
 pub use analyze::{analyze, analyze_program, Analysis, DiskSources, Options, Probe, Sources};
 pub use nova_codegen_cranelift::CompiledProgram;
+/// The resolution dump's forms, for the self-hosted resolver's differential
+/// test (spec 5.2 §3.4).
+pub use nova_resolver::dump as resolution_dump;
 pub use nova_resolver::CompileTarget as Target;
 pub use program::{package_of, Program, Root, RootKind, Roots};
 pub use runtime_cache::{set_embedded_runtime, EmbeddedRuntime};
@@ -110,6 +113,100 @@ pub fn check_program_counted_for(program: Program, target: Target) -> Result<Che
     })
 }
 
+/// The resolution dumps of programs for one target (`nova_resolver::dump`;
+/// spec
+/// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+/// §3.4, ADR 0053), with std's modules lexed and parsed once for them all,
+/// as the self-hosted resolver parses them: each program's own file
+/// database registers std's modules first, so their ids, and the parsed
+/// trees' spans, are the same in every program.
+pub struct ResolutionDumps {
+    target: Target,
+    std: nova_resolver::ParsedStd,
+}
+
+impl ResolutionDumps {
+    /// std's modules for `target`, parsed.
+    pub fn new(target: Target) -> ResolutionDumps {
+        let mut db = FileDb::new();
+        let (std_files, _) = register_std(&mut db, target);
+        ResolutionDumps {
+            target,
+            std: nova_resolver::parse_std(target, &std_files, None),
+        }
+    }
+
+    /// The resolution dump of the program whose root is `path`: `check`'s
+    /// pipeline up to the resolver, rendering nothing. It loads the
+    /// program, stops there on any error, as `check` does, strips the
+    /// `@test` functions, and resolves; the dump holds what was built and
+    /// every diagnostic so far. A root directly in a package's `src/` or
+    /// `tests/` is `UNSUPPORTED package`: 5.2a dumps loose programs alone.
+    /// `Err` when the root cannot be read.
+    pub fn dump(&self, path: &Path) -> std::io::Result<String> {
+        if package_of(path).is_some() {
+            return Ok("UNSUPPORTED package\n".to_string());
+        }
+        let Program { mut db, roots, .. } = Program::loose(path);
+        let (_, mut labels) = register_std(&mut db, self.target);
+        let load = program::load_program(None, &roots, &DiskSources, &mut db)?;
+        let mut modules = load.modules;
+        let mut diagnostics = load.diagnostics;
+        let failed = diagnostics.iter().any(|d| d.severity == Severity::Error);
+        if !failed {
+            strip_test_functions(&mut modules, false);
+        }
+        labels.extend(modules.iter().map(|m| (m.file, m.name.clone())));
+        let sources: Vec<ModuleSource> = modules
+            .iter()
+            .map(|m| ModuleSource {
+                name: m.name.clone(),
+                file: &m.ast,
+                imports: m.imports.clone(),
+                text: db.get_source(m.file),
+                package: m.package.map(|p| p.0),
+                importable: m.importable.clone(),
+            })
+            .collect();
+        let resolved = (!failed)
+            .then(|| nova_resolver::resolve_program_with_std(self.target, &sources, &self.std));
+        if let Some(resolved) = &resolved {
+            diagnostics.extend(resolved.diagnostics.iter().cloned());
+        }
+        Ok(nova_resolver::dump::dump_program(
+            self.target,
+            &sources,
+            &labels,
+            resolved.as_ref(),
+            &diagnostics,
+        ))
+    }
+}
+
+/// std's part of the resolution dump for `target`: the resolution of a
+/// program of no module of its own, which is std's in every program.
+pub fn dump_std_resolution(target: Target) -> String {
+    let mut db = FileDb::new();
+    let (std_files, labels) = register_std(&mut db, target);
+    let resolved = nova_resolver::resolve_program_for(target, &[], &std_files, None);
+    nova_resolver::dump::dump_std(target, &labels, &resolved)
+}
+
+/// Register each of `target`'s std modules in `db` as `check` does, as
+/// `<std/core>`: their ids, in order, and each id with that name.
+fn register_std(db: &mut FileDb, target: Target) -> (Vec<FileId>, Vec<(FileId, String)>) {
+    let mut ids = Vec::new();
+    let mut labels = Vec::new();
+    for &(name, src) in target.std_modules() {
+        let short = name.strip_prefix("$std.").unwrap_or(name);
+        let label = format!("<std/{short}>");
+        let id = db.add(label.clone(), src);
+        ids.push(id);
+        labels.push((id, label));
+    }
+    (ids, labels)
+}
+
 /// The stack of the threads the pipeline runs on: every command's (`nova`'s
 /// `main`), the language server's checker's and `nova dev`'s rebuilds'
 /// (spec
@@ -889,15 +986,7 @@ impl<'a> FrontendContext<'a> {
         // `FileId::DUMMY`. One id per entry of the target's std modules
         // (`CompileTarget::std_modules`), in order — e.g. `$std.core` names
         // `<std/core>`.
-        let std_files: Vec<FileId> = self
-            .target
-            .std_modules()
-            .iter()
-            .map(|&(name, src)| {
-                let short = name.strip_prefix("$std.").unwrap_or(name);
-                self.db.add(format!("<std/{short}>"), src)
-            })
-            .collect();
+        let (std_files, _) = register_std(&mut self.db, self.target);
         // `std/test`, registered the same way but only under `nova test`
         // (`with_test_module`): one more `FileId` allocated *conditionally*,
         // which is fine precisely because it rides alongside `STD_MODULES`
diff --git a/crates/nova-resolver/src/dump.rs b/crates/nova-resolver/src/dump.rs
new file mode 100644
index 0000000000000000000000000000000000000000..d1c714638f1ad3249711419e98f200481b47b358
--- /dev/null
+++ b/crates/nova-resolver/src/dump.rs
@@ -0,0 +1,952 @@
+//! The resolution dump: what the loader and the resolver built for one
+//! program, as text, one entry per line, which the self-hosted resolver
+//! (`selfhost/`) prints too, so that the two can be compared byte for byte
+//! (spec
+//! `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+//! §3.4; ADR 0053 writes the grammar out, with an example of every form).
+//! It carries no stability promise: it changes whenever resolution does, on
+//! both sides in one pull request. `nova check --dump-resolution FILE…`
+//! prints a program's part after a `== FILE` header, and `nova check
+//! --dump-resolution --std` std's part.
+//!
+//! - It follows the AST dump's conventions (ADR 0052 §1): a line is its
+//!   indentation, two spaces per depth, `(` and its kind, then its values
+//!   and its fields `key=value`, each after one space; its children follow,
+//!   one level deeper, and its `)` closes it at the end of its last line.
+//!   Strings are quoted as the AST dump quotes them ([`quote`]), messages
+//!   included; a span is `start..end`; a place, a span in a file, is the
+//!   file's label quoted, `:` and the span.
+//! - **Two parts.** std's modules come after the user's, and std's
+//!   resolution is the same in every program for a target, so it is
+//!   printed once, by [`dump_std`], and a program's part, by
+//!   [`dump_program`], prints only what is the program's own: its
+//!   modules, its definitions, its modules' namespaces and exports, its
+//!   items and splits, its tests and every diagnostic.
+//! - **Ids.** A program's own definition is `u<n>`, its `DefId`; a std
+//!   definition is `s<n>`, its `DefId` less the first std definition's, so
+//!   std's part prints the same ids whatever program it came from. A
+//!   program's part says where std begins, `(StdStart module= def=
+//!   item=)`. Modules and items are numbered from 0 in each part: std's
+//!   part numbers std's modules and items from its first.
+//! - **Namespaces** are printed sorted by name, values, then types, then
+//!   traits; **exports** in the order they were exported, which a glob
+//!   import follows (spec §3.1); `missing`, a target's, in std's part,
+//!   sorted by name.
+
+use std::fmt::Write;
+
+use nova_ast::dump::quote;
+use nova_diagnostics::{Diagnostic, FileId, Severity, Span};
+
+use crate::{
+    CompileTarget, DefId, DefKind, Definitions, ImportTarget, MethodOwner, ModuleSource,
+    ProgramResolution, Res,
+};
+
+/// Every form the dump can print, by which the differential checks that
+/// the corpus and the cases hold each of them
+/// (`crates/nova-cli/tests/selfhost.rs`): a line's kind as `(Kind`, and a
+/// field's word values as `key=Value` or, for a `bool`, after its line's
+/// kind as `(Kind key=value` ([`forms_on`]). [`LATER_KINDS`] are the
+/// forms only a package, the WASM target or test mode prints, which 5.2b
+/// compares. An import the loader reported, `(Import Reported`, is a
+/// loose program's too: E0093 reports one.
+pub const KINDS: &[&str] = &[
+    "(Program",
+    "(Program resolved=true",
+    "(Program resolved=false",
+    "target=native",
+    "(Module",
+    "package=None",
+    "(Import",
+    "(Import Reported",
+    "(Importable",
+    "(StdStart",
+    "(Def.Fn",
+    "(Def.Sum",
+    "(Variant",
+    "(Def.Record",
+    "(Def.Const",
+    "(Def.Trait",
+    "(Def.Method",
+    "owner=Impl",
+    "owner=TraitDefault",
+    "(Def.ExternFn",
+    "(Def.AssocType",
+    "(Scope",
+    "(Value.Def",
+    "(Value.Variant",
+    "(Value.Builtin",
+    "(Type",
+    "(Trait",
+    "(Exports",
+    "(Items",
+    "(Split",
+    "kind=Call",
+    "kind=Index",
+    "kind=Minus",
+    "kind=Value",
+    "(Std",
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
+/// The forms a loose program does not print natively outside test mode:
+/// 5.2b's packages (a module's package key), its WASM target (`missing`)
+/// and its test mode (`@test` functions are stripped under `check`), and
+/// the two severities the resolver never gives.
+pub const LATER_KINDS: &[&str] = &[
+    "target=wasm",
+    "package=<n>",
+    "(Test",
+    "(Test should_panic=true",
+    "(Test should_panic=false",
+    "(Missing",
+    "(Help",
+    "(NoteDiagnostic",
+];
+
+/// The label each file of a program is printed under: a user module's
+/// label, and std's modules' names in the file database, as `<std/core>`.
+pub type Labels = [(FileId, String)];
+
+/// A program's part (module docs) for `target`: its `modules` as the loader
+/// gave them, the files' `labels`, its resolution once it got that far, and
+/// every diagnostic, the loader's and the resolver's, in order. Without a
+/// resolution the part holds the modules loaded and the diagnostics.
+pub fn dump_program(
+    target: CompileTarget,
+    modules: &[ModuleSource],
+    labels: &Labels,
+    resolution: Option<&ProgramResolution>,
+    diagnostics: &[Diagnostic],
+) -> String {
+    let mut d = Dump::new(labels, u32::MAX);
+    d.open(0, "Program");
+    d.field("target", target.name());
+    d.bool_field("resolved", resolution.is_some());
+    for (index, m) in modules.iter().enumerate() {
+        d.open(1, "Module");
+        let _ = write!(d.out, " {index}");
+        d.string(&m.name);
+        d.field("package", &package(m.package));
+        let mut imports: Vec<(&String, &ImportTarget)> = m.imports.iter().collect();
+        imports.sort_by(|a, b| a.0.cmp(b.0));
+        for (name, target) in imports {
+            d.open(2, "Import");
+            d.string(name);
+            match target {
+                ImportTarget::Module(t) => {
+                    let _ = write!(d.out, " {t}");
+                }
+                ImportTarget::Reported => d.out.push_str(" Reported"),
+            }
+            d.close();
+        }
+        for (name, t) in &m.importable {
+            d.open(2, "Importable");
+            d.string(name);
+            let _ = write!(d.out, " {t}");
+            d.close();
+        }
+        d.close();
+    }
+    if let Some(r) = resolution {
+        let defs = &r.definitions;
+        let std_module = defs.std_start;
+        let std_item = defs
+            .item_module
+            .iter()
+            .position(|&m| m as usize >= std_module)
+            .unwrap_or(defs.item_module.len());
+        let std_def = first_std_def(defs, std_item);
+        d.std_def = std_def;
+        d.open(1, "StdStart");
+        d.field("module", &std_module.to_string());
+        d.field("def", &std_def.to_string());
+        d.field("item", &std_item.to_string());
+        d.close();
+        d.defs(defs, 0..std_def, 0);
+        d.scopes(defs, 0..std_module, 0);
+        d.exports(defs, 0..std_module, 0);
+        d.items(defs, 0..std_item, 0, 0);
+        for split in &r.file.newline_splits {
+            d.open(1, "Split");
+            d.place(split.at);
+            d.field("kind", &format!("{:?}", split.kind));
+            d.close();
+        }
+        for test in &r.tests {
+            d.open(1, "Test");
+            let id = d.id(test.def_id);
+            let _ = write!(d.out, " {id}");
+            d.string(&test.name);
+            d.bool_field("should_panic", test.should_panic);
+            d.close();
+        }
+    }
+    d.close();
+    d.out.push('\n');
+    d.diagnostics(diagnostics);
+    d.out
+}
+
+/// std's part (module docs) for `target`, from the resolution of a program
+/// of no module of its own, whose std files have `labels`.
+pub fn dump_std(target: CompileTarget, labels: &Labels, resolution: &ProgramResolution) -> String {
+    let mut d = Dump::new(labels, 0);
+    d.std_part(target, &resolution.definitions, 0, 0, 0);
+    d.out.push('\n');
+    d.diagnostics(&resolution.diagnostics);
+    d.out
+}
+
+/// std's part of any program's `resolution`, without its diagnostics: the
+/// same text as [`dump_std`]'s for a program of no module of its own, in
+/// every program for one target, which a test checks.
+pub fn std_part_of(
+    target: CompileTarget,
+    labels: &Labels,
+    resolution: &ProgramResolution,
+) -> String {
+    let defs = &resolution.definitions;
+    let std_module = defs.std_start;
+    let std_item = defs
+        .item_module
+        .iter()
+        .position(|&m| m as usize >= std_module)
+        .unwrap_or(defs.item_module.len());
+    let std_def = first_std_def(defs, std_item);
+    let mut d = Dump::new(labels, std_def);
+    d.std_part(target, defs, std_module, std_def, std_item);
+    d.out.push('\n');
+    d.out
+}
+
+/// The first std definition's index: the first definition of an item of a
+/// std module. The user's definitions all come before std's, since pass 1
+/// collects the modules in order, and every kind of definition but an
+/// associated type names its item; an associated type follows its trait.
+fn first_std_def(defs: &Definitions, std_item: usize) -> u32 {
+    let mut first = defs.defs.len();
+    for (i, def) in defs.defs.iter().enumerate() {
+        let item = match def.kind {
+            DefKind::Fn { item_index }
+            | DefKind::Sum { item_index, .. }
+            | DefKind::Record { item_index }
+            | DefKind::Const { item_index }
+            | DefKind::Trait { item_index }
+            | DefKind::Method { item_index, .. }
+            | DefKind::ExternFn { item_index, .. } => item_index,
+            DefKind::AssocType { .. } => continue,
+        };
+        if item >= std_item {
+            first = i;
+            break;
+        }
+    }
+    first as u32
+}
+
+/// The forms of [`KINDS`] and [`LATER_KINDS`] that `line` of a dump
+/// prints: its kind, `(Kind`; each field outside quotes, `key=value`, but a
+/// number's, as `package=<n>`, or a `bool`'s, after the kind, as `(Kind
+/// key=value`; and an import the loader reported, `(Import Reported`.
+/// Fields whose values are numbers, as `item=3`, come back too, as
+/// `item=<n>`, and those whose values are ids as `trait=<id>`.
+pub fn forms_on(line: &str) -> Vec<String> {
+    let mut found = Vec::new();
+    let node = line.trim_start();
+    let Some(rest) = node.strip_prefix('(') else {
+        return found;
+    };
+    let kind: String = rest
+        .chars()
+        .take_while(|c| *c != ' ' && *c != ')')
+        .collect();
+    found.push(format!("({kind}"));
+    // Words, split at the spaces outside quotes: a quote inside a string is
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
+                match w.split_once('=') {
+                    Some((key, value @ ("true" | "false"))) => {
+                        found.push(format!("({kind} {key}={value}"));
+                    }
+                    Some((key, value))
+                        if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) =>
+                    {
+                        found.push(format!("{key}=<n>"));
+                    }
+                    Some((key, value))
+                        if value.len() > 1
+                            && value.starts_with(['u', 's'])
+                            && value[1..].bytes().all(|b| b.is_ascii_digit()) =>
+                    {
+                        found.push(format!("{key}=<id>"));
+                    }
+                    Some(_) if !w.contains('"') => found.push(w.to_string()),
+                    _ if kind == "Import" && w == "Reported" => {
+                        found.push("(Import Reported".to_string());
+                    }
+                    _ => {}
+                }
+                word.clear();
+            }
+            _ => word.push(c),
+        }
+    }
+    found
+}
+
+/// Whether `form`, from [`forms_on`], is one of [`KINDS`] or
+/// [`LATER_KINDS`], or a field whose value is a number or an id, which no
+/// list holds but `package=<n>`: the differential checks that every form a
+/// dump prints is listed, so a form added to the dump is added to a list.
+pub fn listed(form: &str) -> bool {
+    form.ends_with("=<n>")
+        || form.ends_with("=<id>")
+        || KINDS.contains(&form)
+        || LATER_KINDS.contains(&form)
+}
+
+/// A package key as the dump prints it.
+fn package(key: Option<u32>) -> String {
+    match key {
+        None => "None".to_string(),
+        Some(k) => k.to_string(),
+    }
+}
+
+/// The line kind of a definition, one per [`DefKind`] variant.
+pub fn def_kind(kind: &DefKind) -> &'static str {
+    match kind {
+        DefKind::Fn { .. } => "Def.Fn",
+        DefKind::Sum { .. } => "Def.Sum",
+        DefKind::Record { .. } => "Def.Record",
+        DefKind::Const { .. } => "Def.Const",
+        DefKind::Trait { .. } => "Def.Trait",
+        DefKind::Method { .. } => "Def.Method",
+        DefKind::ExternFn { .. } => "Def.ExternFn",
+        DefKind::AssocType { .. } => "Def.AssocType",
+    }
+}
+
+/// The line kind of a value's resolution, one per [`Res`] variant.
+pub fn res_kind(res: &Res) -> &'static str {
+    match res {
+        Res::Def(_) => "Value.Def",
+        Res::Variant(..) => "Value.Variant",
+        Res::Builtin(_) => "Value.Builtin",
+    }
+}
+
+/// The line kind of a diagnostic, one per [`Severity`] variant. A
+/// diagnostic of severity `Note` is `NoteDiagnostic`, apart from a
+/// diagnostic's own note, `Note`.
+pub fn severity_kind(severity: Severity) -> &'static str {
+    match severity {
+        Severity::Error => "Error",
+        Severity::Warning => "Warning",
+        Severity::Note => "NoteDiagnostic",
+        Severity::Help => "Help",
+    }
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
+    /// ` ` and a place: the span's file's label quoted, `:`, the span. A
+    /// file with no label is `?`.
+    fn place(&mut self, span: Span) {
+        self.out.push(' ');
+        match self.labels.iter().find(|(f, _)| *f == span.file) {
+            Some((_, label)) => quote(&mut self.out, label),
+            None => self.out.push('?'),
+        }
+        let _ = write!(self.out, ":{}..{}", span.start, span.end);
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
+    // --- The parts ---
+
+    /// std's part: its modules, definitions, namespaces, exports, items
+    /// and `missing`, each numbered from std's first.
+    fn std_part(
+        &mut self,
+        target: CompileTarget,
+        defs: &Definitions,
+        std_module: usize,
+        std_def: u32,
+        std_item: usize,
+    ) {
+        self.open(0, "Std");
+        self.field("target", target.name());
+        let modules = defs.modules.len() - std_module;
+        self.field("modules", &modules.to_string());
+        self.field("defs", &(defs.defs.len() as u32 - std_def).to_string());
+        self.field("items", &(defs.item_module.len() - std_item).to_string());
+        for (index, m) in defs.modules[std_module..].iter().enumerate() {
+            self.open(1, "Module");
+            let _ = write!(self.out, " {index}");
+            self.string(&m.name);
+            self.field("package", &package(defs.packages[std_module + index]));
+            for (name, t) in &defs.importable[std_module + index] {
+                self.open(2, "Importable");
+                self.string(name);
+                let _ = write!(self.out, " {}", t.0 as usize - std_module);
+                self.close();
+            }
+            self.close();
+        }
+        self.defs(defs, std_def..defs.defs.len() as u32, std_item);
+        self.scopes(defs, std_module..defs.modules.len(), std_module);
+        self.exports(defs, std_module..defs.modules.len(), std_module);
+        self.items(defs, std_item..defs.item_module.len(), std_item, std_module);
+        let mut missing: Vec<(&String, &&str)> = defs.missing.iter().collect();
+        missing.sort();
+        for (name, module) in missing {
+            self.open(1, "Missing");
+            self.string(name);
+            self.string(module);
+            self.close();
+        }
+        self.close();
+    }
+
+    /// The definitions `range`, their items numbered from `first_item`.
+    fn defs(&mut self, defs: &Definitions, range: std::ops::Range<u32>, first_item: usize) {
+        for i in range {
+            let def = &defs.defs[i as usize];
+            self.open(1, def_kind(&def.kind));
+            let id = self.id(DefId(i));
+            let _ = write!(self.out, " {id}");
+            self.string(&def.name);
+            self.place(def.span);
+            let item = |index: usize| (index - first_item).to_string();
+            match &def.kind {
+                DefKind::Fn { item_index }
+                | DefKind::Record { item_index }
+                | DefKind::Const { item_index }
+                | DefKind::Trait { item_index } => self.field("item", &item(*item_index)),
+                DefKind::Sum {
+                    item_index,
+                    variants,
+                } => {
+                    self.field("item", &item(*item_index));
+                    for v in variants {
+                        self.open(2, "Variant");
+                        self.string(&v.name);
+                        self.place(v.span);
+                        self.field("arity", &v.arity.to_string());
+                        self.close();
+                    }
+                }
+                DefKind::Method {
+                    item_index,
+                    method_index,
+                    owner,
+                } => {
+                    self.field("item", &item(*item_index));
+                    self.field("method", &method_index.to_string());
+                    let owner = match owner {
+                        MethodOwner::Impl => "Impl",
+                        MethodOwner::TraitDefault => "TraitDefault",
+                    };
+                    self.field("owner", owner);
+                }
+                DefKind::ExternFn {
+                    item_index,
+                    fn_index,
+                } => {
+                    self.field("item", &item(*item_index));
+                    self.field("fn", &fn_index.to_string());
+                }
+                DefKind::AssocType { trait_def } => {
+                    let t = self.id(*trait_def);
+                    self.field("trait", &t);
+                }
+            }
+            self.close();
+        }
+    }
+
+    /// Each module of `range`'s three namespaces, sorted by name, the
+    /// modules numbered from `first`.
+    fn scopes(&mut self, defs: &Definitions, range: std::ops::Range<usize>, first: usize) {
+        for mid in range {
+            let scope = &defs.modules[mid];
+            self.open(1, "Scope");
+            let _ = write!(self.out, " {}", mid - first);
+            let mut values: Vec<(&String, &Res)> = scope.values.iter().collect();
+            values.sort_by(|a, b| a.0.cmp(b.0));
+            for (name, res) in values {
+                self.value(name, *res);
+            }
+            let mut types: Vec<(&String, &DefId)> = scope.types.iter().collect();
+            types.sort_by(|a, b| a.0.cmp(b.0));
+            for (name, id) in types {
+                self.binding("Type", name, *id);
+            }
+            let mut traits: Vec<(&String, &DefId)> = scope.traits.iter().collect();
+            traits.sort_by(|a, b| a.0.cmp(b.0));
+            for (name, id) in traits {
+                self.binding("Trait", name, *id);
+            }
+            self.close();
+        }
+    }
+
+    /// Each module of `range`'s exports, in the order they were exported:
+    /// values, then types, then traits.
+    fn exports(&mut self, defs: &Definitions, range: std::ops::Range<usize>, first: usize) {
+        for mid in range {
+            let e = &defs.exports[mid];
+            self.open(1, "Exports");
+            let _ = write!(self.out, " {}", mid - first);
+            for (name, res) in e.values_in_order() {
+                self.value(&name, res);
+            }
+            for (name, id) in e.types_in_order() {
+                self.binding("Type", &name, id);
+            }
+            for (name, id) in e.traits_in_order() {
+                self.binding("Trait", &name, id);
+            }
+            self.close();
+        }
+    }
+
+    /// A value's line: its name and what it resolves to.
+    fn value(&mut self, name: &str, res: Res) {
+        self.open(2, res_kind(&res));
+        self.string(name);
+        match res {
+            Res::Def(id) => {
+                let id = self.id(id);
+                let _ = write!(self.out, " {id}");
+            }
+            Res::Variant(sum, index) => {
+                let sum = self.id(sum);
+                let _ = write!(self.out, " {sum} {index}");
+            }
+            Res::Builtin(b) => {
+                let _ = write!(self.out, " {b:?}");
+            }
+        }
+        self.close();
+    }
+
+    /// A type's or a trait's line: its name and its definition.
+    fn binding(&mut self, kind: &str, name: &str, id: DefId) {
+        self.open(2, kind);
+        self.string(name);
+        let id = self.id(id);
+        let _ = write!(self.out, " {id}");
+        self.close();
+    }
+
+    /// The merged items `range` as runs of one module, `(Items start..end
+    /// module)`, the items numbered from `first_item` and the modules from
+    /// `first_module`.
+    fn items(
+        &mut self,
+        defs: &Definitions,
+        range: std::ops::Range<usize>,
+        first_item: usize,
+        first_module: usize,
+    ) {
+        let mut start = range.start;
+        while start < range.end {
+            let module = defs.item_module[start];
+            let mut end = start + 1;
+            while end < range.end && defs.item_module[end] == module {
+                end += 1;
+            }
+            self.open(1, "Items");
+            let _ = write!(
+                self.out,
+                " {}..{} {}",
+                start - first_item,
+                end - first_item,
+                module as usize - first_module
+            );
+            self.close();
+            start = end;
+        }
+    }
+
+    /// `(Diagnostics`, each diagnostic in order, `)`: its severity's kind,
+    /// code and message, then its labels in order, notes and fixes.
+    fn diagnostics(&mut self, diagnostics: &[Diagnostic]) {
+        self.open(0, "Diagnostics");
+        for diag in diagnostics {
+            self.open(1, severity_kind(diag.severity));
+            self.string(&diag.code);
+            self.string(&diag.message);
+            for label in &diag.labels {
+                self.open(
+                    2,
+                    if label.primary {
+                        "Primary"
+                    } else {
+                        "Secondary"
+                    },
+                );
+                self.place(label.span);
+                self.string(&label.message);
+                self.close();
+            }
+            for note in &diag.notes {
+                self.open(2, "Note");
+                self.string(note);
+                self.close();
+            }
+            for fix in &diag.fixes {
+                self.open(2, "Fix");
+                self.string(&fix.title);
+                for edit in &fix.edits {
+                    self.open(3, "Edit");
+                    self.place(edit.span);
+                    self.string(&edit.text);
+                    self.close();
+                }
+                self.close();
+            }
+            self.close();
+        }
+        self.close();
+        self.out.push('\n');
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::{name_imports, resolve_program_for, STD_MODULES};
+    use nova_diagnostics::FileDb;
+
+    /// A program whose two modules print every form of [`KINDS`]: the
+    /// loader's imports and what each module could import, every kind of
+    /// definition, every kind of binding, exports, items and the four
+    /// kinds of split, and diagnostics with secondary labels, notes and a
+    /// fix.
+    const MAIN: &str = "import lib\nimport lib::{secret}\nimport nosuch\nimport dom\n\
+        import lib as l\nimport a::b\n\
+        type Name = String\nrecord Int { x: Int }\n\
+        @tset\nfn main() {\n    g\n    (1)\n    g\n    [1]\n    g\n    -1\n}\n\
+        @test\nrecord R {}\n@bogus\npub const K: Int = 1\n\
+        pub type Shade = | Red | Ok(Int)\n\
+        trait Show {\n    type Item\n    fn twice(self) {}\n}\n\
+        impl Show for R {\n    fn twice(self) {}\n}\n\
+        extern \"C\" {\n    fn abs(x: Int) -> Int\n}\n\
+        fn println() {}\nfn dup() {}\nfn dup() {}\nfn alpha() -> Int { 0 }\n\
+        fn back() -> Int {\n    return\n    1\n}\n";
+    const LIB: &str = "import lib\npub fn alpha() -> Int { 1 }\nfn secret() -> Int { 2 }\n\
+        pub record P {}\npub trait T {}\n";
+
+    /// Parse `src` as `file`.
+    fn parse(src: &str, file: FileId) -> nova_ast::File {
+        let (tokens, _) = nova_lexer::lex(src, file);
+        let parsed = nova_parser::parse_recovering(&tokens, file);
+        assert!(parsed.errors.is_empty(), "{:?}", parsed.errors);
+        parsed.file
+    }
+
+    /// The std modules registered in `db` as the driver registers them.
+    fn std_files(db: &mut FileDb) -> (Vec<FileId>, Vec<(FileId, String)>) {
+        let mut ids = Vec::new();
+        let mut labels = Vec::new();
+        for &(name, src) in &STD_MODULES {
+            let label = format!("<std/{}>", name.strip_prefix("$std.").unwrap_or(name));
+            let id = db.add(label.clone(), src);
+            ids.push(id);
+            labels.push((id, label));
+        }
+        (ids, labels)
+    }
+
+    /// The program part of [`MAIN`] and [`LIB`], and std's part of the same
+    /// resolution.
+    fn the_program() -> (String, String, Vec<(FileId, String)>, Vec<FileId>) {
+        let mut db = FileDb::new();
+        let main_id = db.add("main.nova", MAIN);
+        let lib_id = db.add("lib.nova", LIB);
+        let (std_ids, mut labels) = std_files(&mut db);
+        let main = parse(MAIN, main_id);
+        let lib = parse(LIB, lib_id);
+        let mut sources = [
+            ModuleSource::new("main", &main),
+            ModuleSource::new("lib", &lib),
+        ];
+        name_imports(&mut sources);
+        sources[0].text = Some(MAIN);
+        sources[1].text = Some(LIB);
+        let r = resolve_program_for(CompileTarget::Native, &sources, &std_ids, None);
+        let std_labels = labels.clone();
+        labels.push((main_id, "main".to_string()));
+        labels.push((lib_id, "lib".to_string()));
+        let program = dump_program(
+            CompileTarget::Native,
+            &sources,
+            &labels,
+            Some(&r),
+            &r.diagnostics,
+        );
+        let std = std_part_of(CompileTarget::Native, &labels, &r);
+        (program, std, std_labels, std_ids)
+    }
+
+    /// The expected program part, `dump_test_program.txt`, as the tests
+    /// read it: no `.gitattributes` rule covers it, so a Windows checkout
+    /// with git's `core.autocrlf` gives it `\r\n` line endings, which are
+    /// read as `\n`.
+    const WANT_PROGRAM: &str = include_str!("dump_test_program.txt");
+
+    /// Assert that `program`'s part, all but its modules' bindings of std's
+    /// names and of the builtins, which std's part and the builtin table
+    /// pin, is `want`, with each `\r\n` in `want` read as `\n`.
+    fn assert_own_part(program: &str, want: &str) {
+        let own: Vec<&str> = program
+            .lines()
+            .filter(|l| {
+                let binding = ["    (Value.", "    (Type ", "    (Trait "]
+                    .iter()
+                    .any(|p| l.starts_with(p));
+                !(binding && (l.contains("(Value.Builtin") || l.contains(" s")))
+            })
+            .collect();
+        assert_eq!(own.join("\n") + "\n", want.replace("\r\n", "\n"));
+    }
+
+    /// The program's part, all but its modules' bindings of std's names and
+    /// of the builtins.
+    #[test]
+    fn a_programs_part_prints_its_modules_definitions_items_and_diagnostics() {
+        let (program, _, _, _) = the_program();
+        assert_own_part(&program, WANT_PROGRAM);
+    }
+
+    /// The same, with the expected part as a Windows checkout has it.
+    #[test]
+    fn the_expected_program_part_may_have_windows_line_endings() {
+        let (program, _, _, _) = the_program();
+        let crlf = WANT_PROGRAM.replace("\r\n", "\n").replace('\n', "\r\n");
+        assert!(crlf.contains("\r\n"));
+        assert_own_part(&program, &crlf);
+    }
+
+    /// Every form of [`KINDS`] is in the program's part, its std's part, or
+    /// the part of a program that stopped before the resolver, whose import
+    /// the loader reported; and every form they print is listed.
+    #[test]
+    fn the_program_prints_every_form() {
+        let (program, std, _, _) = the_program();
+        let mut db = FileDb::new();
+        let id = db.add("main.nova", "import ui\n");
+        let tree = parse("import ui\n", id);
+        let mut module = ModuleSource::new("main", &tree);
+        module
+            .imports
+            .insert("ui".to_string(), ImportTarget::Reported);
+        let labels = [(id, "main".to_string())];
+        let stopped = dump_program(CompileTarget::Native, &[module], &labels, None, &[]);
+        assert_eq!(
+            stopped,
+            "(Program target=native resolved=false\n  (Module 0 \"main\" package=None\n    \
+             (Import \"ui\" Reported)))\n(Diagnostics)\n"
+        );
+        let mut found = std::collections::BTreeSet::new();
+        for line in program.lines().chain(std.lines()).chain(stopped.lines()) {
+            found.extend(forms_on(line));
+        }
+        let missing: Vec<&&str> = KINDS.iter().filter(|k| !found.contains(**k)).collect();
+        assert!(missing.is_empty(), "{missing:?}");
+        let unlisted: Vec<&String> = found.iter().filter(|f| !listed(f)).collect();
+        assert!(unlisted.is_empty(), "{unlisted:?}");
+    }
+
+    /// std's part of a program's resolution is std's part of a program of
+    /// no module of its own: the user's modules change no std scope, and
+    /// the ids, modules and items are std-relative.
+    #[test]
+    fn std_part_is_the_same_in_every_program() {
+        let (_, std, std_labels, std_ids) = the_program();
+        let alone = resolve_program_for(CompileTarget::Native, &[], &std_ids, None);
+        assert_eq!(std, std_part_of(CompileTarget::Native, &std_labels, &alone));
+        let full = dump_std(CompileTarget::Native, &std_labels, &alone);
+        assert_eq!(full, std.clone() + "(Diagnostics)\n");
+        assert!(
+            std.starts_with("(Std target=native modules=21 "),
+            "{}",
+            &std[..80]
+        );
+        assert!(!std.contains(" u"), "std's part names a user definition");
+    }
+
+    /// [`resolve_program_for`] is [`crate::parse_std`] then
+    /// [`crate::resolve_program_with_std`], which the resolution dumps call
+    /// with std parsed once for every root (ADR 0053 §9): one `ParsedStd`
+    /// resolves two programs in turn, each with diagnostics, to the same
+    /// merged file, definitions, diagnostics, tests and import occurrences
+    /// as `resolve_program_for` gives each.
+    #[test]
+    fn parsing_std_once_resolves_as_resolve_program_for_does() {
+        let mut db = FileDb::new();
+        let main_id = db.add("main.nova", MAIN);
+        let lib_id = db.add("lib.nova", LIB);
+        let (std_ids, _) = std_files(&mut db);
+        let (main, lib) = (parse(MAIN, main_id), parse(LIB, lib_id));
+        let mut two = [
+            ModuleSource::new("main", &main),
+            ModuleSource::new("lib", &lib),
+        ];
+        name_imports(&mut two);
+        let mut one = [ModuleSource::new("lib", &lib)];
+        name_imports(&mut one);
+        let std = crate::parse_std(CompileTarget::Native, &std_ids, None);
+        for program in [&two[..], &one[..]] {
+            let a = resolve_program_for(CompileTarget::Native, program, &std_ids, None);
+            let b = crate::resolve_program_with_std(CompileTarget::Native, program, &std);
+            assert!(!a.diagnostics.is_empty());
+            assert_eq!(
+                nova_ast::dump::dump_file(&a.file),
+                nova_ast::dump::dump_file(&b.file)
+            );
+            for (x, y) in [
+                (
+                    format!("{:?}", a.definitions),
+                    format!("{:?}", b.definitions),
+                ),
+                (
+                    format!("{:?}", a.diagnostics),
+                    format!("{:?}", b.diagnostics),
+                ),
+                (format!("{:?}", a.tests), format!("{:?}", b.tests)),
+                (format!("{:?}", a.imports), format!("{:?}", b.imports)),
+            ] {
+                assert_eq!(x, y);
+            }
+        }
+    }
+
+    /// Each kind of definition, of value and of diagnostic has its form in
+    /// [`KINDS`] or [`LATER_KINDS`].
+    #[test]
+    fn every_kind_of_definition_value_and_diagnostic_is_listed() {
+        let listed = |k: &str| {
+            let form = format!("({k}");
+            KINDS.contains(&form.as_str()) || LATER_KINDS.contains(&form.as_str())
+        };
+        let item_index = 0;
+        let kinds = [
+            DefKind::Fn { item_index },
+            DefKind::Sum {
+                item_index,
+                variants: Vec::new(),
+            },
+            DefKind::Record { item_index },
+            DefKind::Const { item_index },
+            DefKind::Trait { item_index },
+            DefKind::Method {
+                item_index,
+                method_index: 0,
+                owner: MethodOwner::Impl,
+            },
+            DefKind::ExternFn {
+                item_index,
+                fn_index: 0,
+            },
+            DefKind::AssocType {
+                trait_def: DefId(0),
+            },
+        ];
+        for kind in &kinds {
+            assert!(listed(def_kind(kind)), "{kind:?}");
+        }
+        for res in [
+            Res::Def(DefId(0)),
+            Res::Variant(DefId(0), 0),
+            Res::Builtin(crate::Builtin::Println),
+        ] {
+            assert!(listed(res_kind(&res)), "{res:?}");
+        }
+        for severity in [
+            Severity::Error,
+            Severity::Warning,
+            Severity::Note,
+            Severity::Help,
+        ] {
+            assert!(listed(severity_kind(severity)), "{severity:?}");
+        }
+    }
+}
diff --git a/crates/nova-resolver/src/dump_test_program.txt b/crates/nova-resolver/src/dump_test_program.txt
new file mode 100644
index 0000000000000000000000000000000000000000..841af203c4bf4ef89c0173e8df6825331decfd9e
--- /dev/null
+++ b/crates/nova-resolver/src/dump_test_program.txt
@@ -0,0 +1,100 @@
+(Program target=native resolved=true
+  (Module 0 "main" package=None
+    (Import "lib" 1)
+    (Importable "lib" 1))
+  (Module 1 "lib" package=None
+    (Import "lib" 1)
+    (Importable "main" 0))
+  (StdStart module=2 def=18 item=25)
+  (Def.Fn u0 "main" "main":135..139 item=8)
+  (Def.Record u1 "R" "main":200..201 item=9)
+  (Def.Const u2 "K" "main":222..223 item=10)
+  (Def.Sum u3 "Shade" "main":242..247 item=11
+    (Variant "Red" "main":252..255 arity=0)
+    (Variant "Ok" "main":258..260 arity=1))
+  (Def.Trait u4 "Show" "main":272..276 item=12)
+  (Def.AssocType u5 "Item" "main":288..292 trait=u4)
+  (Def.Method u6 "Show::twice$default" "main":300..305 item=12 method=1 owner=TraitDefault)
+  (Def.Method u7 "R.Show.twice" "main":342..347 item=13 method=0 owner=Impl)
+  (Def.ExternFn u8 "abs" "main":379..382 item=14 fn=0)
+  (Def.Fn u9 "println" "main":403..410 item=15)
+  (Def.Fn u10 "dup" "main":419..422 item=16)
+  (Def.Fn u11 "dup" "main":431..434 item=17)
+  (Def.Fn u12 "alpha" "main":443..448 item=18)
+  (Def.Fn u13 "back" "main":467..471 item=19)
+  (Def.Fn u14 "alpha" "lib":18..23 item=21)
+  (Def.Fn u15 "secret" "lib":42..48 item=22)
+  (Def.Record u16 "P" "lib":75..76 item=23)
+  (Def.Trait u17 "T" "lib":90..91 item=24)
+  (Scope 0
+    (Value.Def "K" u2)
+    (Value.Variant "Ok" u3 1)
+    (Value.Variant "Red" u3 0)
+    (Value.Def "abs" u8)
+    (Value.Def "alpha" u12)
+    (Value.Def "back" u13)
+    (Value.Def "dup" u10)
+    (Value.Def "main" u0)
+    (Type "P" u16)
+    (Type "R" u1)
+    (Type "Shade" u3)
+    (Trait "Show" u4)
+    (Trait "T" u17)
+  (Scope 1
+    (Value.Def "alpha" u14)
+    (Value.Def "secret" u15)
+    (Type "P" u16)
+    (Trait "T" u17)
+  (Exports 0
+    (Value.Def "K" u2)
+    (Value.Variant "Red" u3 0)
+    (Value.Variant "Ok" u3 1)
+    (Type "Shade" u3))
+  (Exports 1
+    (Value.Def "alpha" u14)
+    (Type "P" u16)
+    (Trait "T" u17))
+  (Items 0..20 0)
+  (Items 20..25 1)
+  (Split "main":154..155 kind=Call)
+  (Split "main":168..169 kind=Index)
+  (Split "main":182..183 kind=Minus)
+  (Split "main":498..499 kind=Value))
+(Diagnostics
+  (Error "E0900" "type aliases are not supported yet in the Phase 1 compiler"
+    (Primary "main":90..94 "not supported yet"))
+  (Error "E0089" "`Int` is already a built-in type; a type declared under this name could never be referred to in a type annotation, since `Int` in a type position never resolves to a declaration under this name"
+    (Primary "main":111..114 "redeclares a built-in type name"))
+  (Error "E0082" "unknown attribute `@tset`; known attributes are: test"
+    (Primary "main":127..131 "unknown attribute")
+    (Fix "change `tset` to `test`"
+      (Edit "main":127..131 "test")))
+  (Error "E0083" "`@test` can only be applied to a function, not a record"
+    (Primary "main":188..192 "not a function"))
+  (Error "E0082" "unknown attribute `@bogus`; known attributes are: test"
+    (Primary "main":206..211 "unknown attribute"))
+  (Error "E0002" "duplicate definition of `println`"
+    (Primary "main":403..410 "redefined here")
+    (Note "`println` is a compiler builtin"))
+  (Error "E0002" "duplicate definition of `dup`"
+    (Primary "main":431..434 "redefined here")
+    (Secondary "main":419..422 "first defined here"))
+  (Error "E0002" "`alpha` is already defined or imported in this module"
+    (Primary "main":7..10 "conflicting import"))
+  (Error "E0001" "`secret` is not a public item of module `lib`"
+    (Primary "main":24..30 "not found or not `pub`")
+    (Fix "make `secret` public in `lib`"
+      (Edit "lib":39..39 "pub ")))
+  (Error "E0001" "cannot find module `nosuch`"
+    (Primary "main":39..45 "no such module"))
+  (Error "E0092" "`dom` is a std module, which target native does not have"
+    (Primary "main":53..56 "not on target native"))
+  (Error "E0900" "`import ... as` aliases are not supported yet"
+    (Primary "main":64..67 "not supported yet"))
+  (Error "E0900" "qualified or nested import paths (`a::b`) are not supported yet; import a top-level module by its name"
+    (Primary "main":80..84 "not supported yet"))
+  (Error "E0001" "module `lib` imports itself"
+    (Primary "lib":7..10 "self-import"))
+  (Warning "E0102" "variant `Ok` hides std's `Result::Ok`"
+    (Primary "main":258..260 "this variant")
+    (Note "in this module the bare name `Ok` now means this variant; std's is `Result::Ok`")))
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 7d262bd88b5a55aedde2b9bd7f01b15940f464f8..7f9ab24798059a1a4063065d7fbba6b34255477b 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -17,6 +17,7 @@ use nova_ast::{File, Function, Item, Type};
 use nova_diagnostics::{suggest, Diagnostic, Edit, FileId, Fix, Span};
 use rustc_hash::FxHashMap;
 
+pub mod dump;
 pub mod index;
 pub use index::{Index, LocalFlags, Occurrence, Role, Target};
 
@@ -2015,12 +2016,29 @@ pub fn resolve_program_for(
     std_files: &[FileId],
     extra_std: Option<((&str, &str), FileId)>,
 ) -> ProgramResolution {
-    let mut definitions = Definitions::default();
-    let mut diagnostics = Vec::new();
-    let mut merged = Vec::new();
-    let mut tests = Vec::new();
-    let mut imports: Vec<Occurrence> = Vec::new();
+    resolve_program_with_std(target, modules, &parse_std(target, std_files, extra_std))
+}
 
+/// std's modules for one target, lexed and parsed by [`parse_std`]: each
+/// module's name and tree, `None` for one that failed to parse, in order,
+/// and the diagnostics of parsing them. A caller that resolves many
+/// programs parses std once and hands each [`resolve_program_with_std`]
+/// the same `ParsedStd`, as the resolution dump does (spec
+/// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+/// §3.5, ADR 0053); its spans name the `std_files` it was parsed with.
+pub struct ParsedStd {
+    modules: Vec<(String, Option<File>)>,
+    diagnostics: Vec<Diagnostic>,
+}
+
+/// Lex and parse `target`'s std modules, and `extra_std` after them, each
+/// under its id in `std_files` ([`resolve_program`] says what the ids and
+/// `extra_std` are).
+pub fn parse_std(
+    target: CompileTarget,
+    std_files: &[FileId],
+    extra_std: Option<((&str, &str), FileId)>,
+) -> ParsedStd {
     // Every `STD_MODULES` entry plus, only when the caller passed one, the
     // extra std module — combined here so the rest of this function has one
     // uniform list to iterate instead of a special case for the extra entry.
@@ -2043,15 +2061,34 @@ pub fn resolve_program_for(
     // reported against a real file so it is debuggable, and a module that
     // fails to parse outright is skipped entirely (`None`) rather than
     // silently merged in as empty.
-    let std_modules: Vec<Option<File>> = std_entries
+    let mut diagnostics = Vec::new();
+    let modules = std_entries
         .iter()
         .zip(std_file_ids.iter())
-        .map(|(&(_, src), &file_id)| {
+        .map(|(&(name, src), &file_id)| {
             let (ast, diags) = std_module(src, file_id);
             diagnostics.extend(diags);
-            ast
+            (name.to_string(), ast)
         })
         .collect();
+    ParsedStd {
+        modules,
+        diagnostics,
+    }
+}
+
+/// [`resolve_program_for`] with std's modules already parsed, by
+/// [`parse_std`] for the same `target`.
+pub fn resolve_program_with_std(
+    target: CompileTarget,
+    modules: &[ModuleSource],
+    std: &ParsedStd,
+) -> ProgramResolution {
+    let mut definitions = Definitions::default();
+    let mut diagnostics = std.diagnostics.clone();
+    let mut merged = Vec::new();
+    let mut tests = Vec::new();
+    let mut imports: Vec<Occurrence> = Vec::new();
 
     // They go *after* all user modules, so user module indices are
     // unaffected — module 0 stays the first user module.
@@ -2066,15 +2103,10 @@ pub fn resolve_program_for(
             package: m.package,
             importable: m.importable.clone(),
         })
-        .chain(
-            std_entries
-                .iter()
-                .zip(std_modules.iter())
-                .filter_map(|(&(name, _), file)| {
-                    file.as_ref()
-                        .map(|file| ModuleSource::new(name.to_string(), file))
-                }),
-        )
+        .chain(std.modules.iter().filter_map(|(name, file)| {
+            file.as_ref()
+                .map(|file| ModuleSource::new(name.clone(), file))
+        }))
         .collect();
 
     // The std modules that parsed successfully are exactly the contiguous
````

### Task 3: The resolver's error cases and the name fuzzer, through S0's dump

Spec §3.5; decisions 6, 14 to 16, 25 and 30.

**Files:**
- Create: `crates/nova-cli/tests/selfhost/cases/resolve/` (139 files,
  103 programs)
- Modify: `crates/nova-cli/tests/selfhost/fuzz.rs` (`name_mutate`,
  `parses`, `NAME_SIBLINGS` and the tables they draw from)
- Test: `crates/nova-cli/tests/selfhost.rs`

- [ ] **Step 1: The tests.** `patch_block t3-tests`:
  - the 103 cases (decision 15), which 5.1's `the_fixed_cases_agree` reads
    at once through the token and AST dumps;
  - `selfhost.rs`: `Dumps`, a token or a token-and-AST pass, in place of
    `agree_on_inputs`' `bool`; `crlf`, split out of `windows_corpus`;
    `resolve_cases`, `loose_corpus`, `s0_resolution`;
    `every_resolution_form_appears_in_the_corpus_and_the_cases` (on S0's
    dumps of std, the 251 loose programs and the cases: every form of
    `KINDS`, every form listed, none of `LATER_KINDS`);
    `the_name_fuzzer_gives_every_native_resolver_code` (seeds 301 to 303,
    50 inputs each: every input resolved, each of the 8 codes twice or
    more, none of the 3); and the name fuzzer's inputs in
    `the_mutations_are_the_same_with_windows_line_endings`.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. Then `SH`. Expected: it does not compile, `cannot find function `name_mutate` in module `fuzz``, and the same for `parses` and `NAME_SIBLINGS`
  (decision 36).
- [ ] **Step 3: The code.** `patch_block t3-code`: `fuzz.rs`'s
  `name_mutate` and its tables (decision 16).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH`. Expected: `31 passed; 0 failed`.
  - `S` (4 min 47 s). Expected: `2852 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit**
  `cli tests: the resolver's error cases and the name fuzzer, through S0's dump`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index ad2efd024839a4505202263e4d280edd4968274e..93c7702e750fb4a203d81a8c0c73990fe0b6c09e 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -28,9 +28,13 @@
 //!
 //! 5.2a (spec
 //! `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
-//! §3.4; ADR 0053): S0's hidden `nova check --dump-resolution FILE…` and
-//! `--std`, absent from `--help`, with every other `check` command line
-//! parsed as before, a header per root and std's part its own.
+//! §3.4–§3.5; ADR 0053): S0's hidden `nova check --dump-resolution FILE…`
+//! and `--std`, absent from `--help`, with every other `check` command
+//! line parsed as before; every form of its dump in its dumps of std's part, every loose corpus
+//! program and the resolver's error cases in
+//! `tests/selfhost/cases/resolve/`; and the name-level fuzzer's inputs
+//! from fixed seeds, among which every resolver code `check` can give
+//! natively occurs.
 //!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
@@ -514,8 +518,13 @@ fn compare_batch(
     nova: &Path,
     selfhost: &Path,
 ) -> Result<String, String> {
-    let s0 = run(nova, dump.s0, dir, files)?;
-    let n = run(selfhost, dump.n, dir, files)?;
+    // The two sides run at once.
+    let (s0, n) = std::thread::scope(|scope| {
+        let s0 = scope.spawn(|| run(nova, dump.s0, dir, files));
+        let n = run(selfhost, dump.n, dir, files);
+        (s0.join().expect("S0's thread"), n)
+    });
+    let (s0, n) = (s0?, n?);
     let mismatch = first_difference(&s0.stdout, &n.stdout);
     if mismatch.is_none() && s0.code == Some(code) && n.code == Some(code) {
         return Ok(String::from_utf8_lossy(&s0.stdout).into_owned());
@@ -563,12 +572,21 @@ fn agree(dir: &Path, files: &[String], batch: usize, code: i32) -> String {
     tokens + &ast
 }
 
+/// Which dumps an input set goes through.
+#[derive(Clone, Copy, PartialEq, Eq)]
+enum Dumps {
+    /// 5.1a's token dump.
+    Tokens,
+    /// The token dump, then 5.1b's AST dump.
+    TokensAndAst,
+}
+
 /// Write `inputs` to a fixed directory named after `name`, as
-/// `f00000.nova` and on, and check that both sides agree on them, in the
-/// token dump, and in the AST dump too if `ast`. Returns S0's output, its
-/// token dump and, if `ast`, its AST dump after it. The fuzzer's generators all come through here, 5.1b's deep nesting
-/// included.
-fn agree_on_inputs(name: &str, inputs: &[String], ast: bool) -> String {
+/// `f00000.nova` and on, and check that both sides agree on them through
+/// `dumps`. Returns S0's output: its token dump and, with the AST dump, its
+/// AST dump after it. The fuzzer's generators all come through here,
+/// 5.1b's deep nesting included.
+fn agree_on_inputs(name: &str, inputs: &[String], dumps: Dumps) -> String {
     let dir = fresh_dir(name);
     let mut files = Vec::with_capacity(inputs.len());
     for (i, input) in inputs.iter().enumerate() {
@@ -576,10 +594,9 @@ fn agree_on_inputs(name: &str, inputs: &[String], ast: bool) -> String {
         std::fs::write(dir.join(&file), input).expect("write a fuzz input");
         files.push(file);
     }
-    if ast {
-        agree(&dir, &files, 100, 0)
-    } else {
-        agree_on(&TOKENS, &dir, &files, 100, 0)
+    match dumps {
+        Dumps::TokensAndAst => agree(&dir, &files, 100, 0),
+        _ => agree_on(&TOKENS, &dir, &files, 100, 0),
     }
 }
 
@@ -619,25 +636,29 @@ fn windows_corpus(name: &str) -> (PathBuf, Vec<String>) {
     let files = corpus();
     for file in &files {
         let text = std::fs::read(root.join(file)).expect("read a corpus file");
-        let crlf = if text.contains(&b'\r') {
-            text
-        } else {
-            let mut out = Vec::with_capacity(text.len() + text.len() / 16);
-            for &b in &text {
-                if b == b'\n' {
-                    out.push(b'\r');
-                }
-                out.push(b);
-            }
-            out
-        };
         let path = dir.join(file);
         std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
-        std::fs::write(&path, crlf).expect("write a corpus file");
+        std::fs::write(&path, crlf(text)).expect("write a corpus file");
     }
     (dir, files)
 }
 
+/// `text` as git's `core.autocrlf` checks it out ([`windows_corpus`]):
+/// each `\n` as `\r\n`, unless it already holds a `\r`.
+fn crlf(text: Vec<u8>) -> Vec<u8> {
+    if text.contains(&b'\r') {
+        return text;
+    }
+    let mut out = Vec::with_capacity(text.len() + text.len() / 16);
+    for &b in &text {
+        if b == b'\n' {
+            out.push(b'\r');
+        }
+        out.push(b);
+    }
+    out
+}
+
 #[test]
 fn the_corpus_dumps_agree() {
     let files = corpus();
@@ -673,6 +694,16 @@ fn the_mutations_are_the_same_with_windows_line_endings() {
             "seed {seed} mutates the Windows checkout otherwise"
         );
     }
+    // The name fuzzer's too, from the loose programs (5.2a).
+    let loose = loose_corpus();
+    let mut reversed = loose.clone();
+    reversed.reverse();
+    for seed in NAME_SEEDS {
+        assert!(
+            names_of(&dir, &reversed, seed, 50) == names_of(&root(), &loose, seed, 50),
+            "seed {seed} name-mutates the Windows checkout otherwise"
+        );
+    }
 }
 
 #[test]
@@ -686,7 +717,7 @@ fn the_fixed_cases_agree() {
 
 #[test]
 fn the_edge_inputs_agree() {
-    agree_on_inputs("edges", &fuzz::edges(), true);
+    agree_on_inputs("edges", &fuzz::edges(), Dumps::TokensAndAst);
 }
 
 #[test]
@@ -696,7 +727,11 @@ fn the_soup_agrees_and_holds_every_lex_error() {
         dumps.push_str(&agree_on_inputs(
             &format!("soup-{seed}"),
             &fuzz::soups(seed, SOUP_COUNT),
-            seed == AST_SOUP_SEED,
+            if seed == AST_SOUP_SEED {
+                Dumps::TokensAndAst
+            } else {
+                Dumps::Tokens
+            },
         ));
     }
     // One seed's trees were compared too.
@@ -719,7 +754,7 @@ fn the_mutated_corpus_agrees() {
         let out = agree_on_inputs(
             &format!("mutated-{seed}"),
             &mutations(seed, MUTATION_COUNT),
-            true,
+            Dumps::TokensAndAst,
         );
         // Their trees were compared too.
         assert!(out.contains("\n(File"), "seed {seed}");
@@ -737,7 +772,11 @@ fn a_longer_search_from_the_environment() {
         .split_once(':')
         .and_then(|(s, c)| Some((s.parse::<u64>().ok()?, c.parse::<usize>().ok()?)))
         .expect("NOVA_SELFHOST_FUZZ is SEED:COUNT");
-    agree_on_inputs("search-soup", &fuzz::soups(seed, count), true);
+    agree_on_inputs(
+        "search-soup",
+        &fuzz::soups(seed, count),
+        Dumps::TokensAndAst,
+    );
     // Mutations go in batches of 500, each from a seed of its own, so a long
     // search keeps few inputs on disk at a time.
     let mut done = 0;
@@ -746,7 +785,7 @@ fn a_longer_search_from_the_environment() {
         agree_on_inputs(
             "search-mutated",
             &mutations(seed.wrapping_add(done as u64), n),
-            true,
+            Dumps::TokensAndAst,
         );
         done += n;
     }
@@ -1107,7 +1146,7 @@ fn the_deep_inputs_agree() {
             .into_iter()
             .map(|(_, _, src)| src)
             .collect();
-        let out = agree_on_inputs(&format!("deep-{seed}"), &inputs, true);
+        let out = agree_on_inputs(&format!("deep-{seed}"), &inputs, Dumps::TokensAndAst);
         // The AST dumps were compared too: past the limit, P0002.
         assert!(out.contains(" this is nested too deeply"), "seed {seed}");
     }
@@ -1370,6 +1409,88 @@ fn nova_run_runs_the_program_on_the_main_thread() {
 /// program's root.
 const RESOLUTION_S0: &[&str] = &["check", "--dump-resolution"];
 
+/// The error cases of the resolver (spec §3.5): the inline sources of
+/// `nova-resolver`'s tests and of the type checker's tests that reach the
+/// resolver, each a file, or a directory whose `main.nova` is the root of
+/// a program of several modules.
+fn resolve_cases_dir() -> PathBuf {
+    cases_dir().join("resolve")
+}
+
+/// The roots of the resolver's error cases, relative to the repository:
+/// each file directly in [`resolve_cases_dir`], and each directory's
+/// `main.nova`, sorted.
+fn resolve_cases() -> Vec<String> {
+    let root = root();
+    let mut entries: Vec<_> = std::fs::read_dir(resolve_cases_dir())
+        .expect("read the resolver's cases")
+        .map(|e| e.expect("a directory entry").path())
+        .collect();
+    entries.sort();
+    let mut roots = Vec::new();
+    for path in entries {
+        let file = if path.is_dir() {
+            path.join("main.nova")
+        } else if path.extension().is_some_and(|e| e == "nova") {
+            path.clone()
+        } else {
+            continue;
+        };
+        assert!(file.is_file(), "{} has no main.nova", path.display());
+        let rel = file.strip_prefix(&root).expect("under the root");
+        let parts: Vec<String> = rel
+            .components()
+            .map(|c| c.as_os_str().to_string_lossy().into_owned())
+            .collect();
+        roots.push(parts.join("/"));
+    }
+    roots
+}
+
+/// The programs of the corpus that the resolution dump compares in 5.2a:
+/// every corpus file but those directly in a package's `src/` or
+/// `tests/`, which are 5.2b's.
+fn loose_corpus() -> Vec<String> {
+    let root = root();
+    corpus()
+        .into_iter()
+        .filter(|f| {
+            let path = root.join(f);
+            let parent = path.parent().expect("a parent");
+            let in_package = matches!(
+                parent.file_name().and_then(|n| n.to_str()),
+                Some("src" | "tests")
+            ) && parent
+                .parent()
+                .is_some_and(|p| p.join("nova.toml").is_file());
+            !in_package
+        })
+        .collect()
+}
+
+/// S0's resolution dump of `files`, relative to `dir`, [`BATCH`] files to
+/// a process, the processes at once, each exiting with `code`. N's is
+/// compared with it from Task 6 on.
+fn s0_resolution(dir: &Path, files: &[String], code: i32) -> String {
+    let nova = nova();
+    let outs: Vec<Result<Run, String>> = std::thread::scope(|scope| {
+        let runs: Vec<_> = files
+            .chunks(BATCH)
+            .map(|chunk| scope.spawn(|| run(&nova, RESOLUTION_S0, dir, chunk)))
+            .collect();
+        runs.into_iter()
+            .map(|r| r.join().expect("a batch's thread"))
+            .collect()
+    });
+    let mut all = String::new();
+    for out in outs {
+        let out = out.unwrap_or_else(|e| panic!("{e}"));
+        assert_eq!(out.code, Some(code), "{}", out.stderr);
+        all.push_str(&String::from_utf8_lossy(&out.stdout));
+    }
+    all
+}
+
 /// `nova check`'s help names neither hidden flag, and every `nova check`
 /// command line whose first argument after `check` is not
 /// `--dump-resolution` is parsed as before the flags (ADR 0053 §1): each
@@ -1594,3 +1715,169 @@ fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
         );
     }
 }
+
+/// Every form of the resolution dump's `KINDS` appears in S0's dump of
+/// std's part, or of some loose corpus program or resolver case (spec
+/// §3.5), every form they print is listed, and no form of `LATER_KINDS`
+/// appears, nor any of the three codes 5.2b's test mode and WASM target
+/// give; N's dumps are compared with these from Task 6 on.
+#[test]
+fn every_resolution_form_appears_in_the_corpus_and_the_cases() {
+    let root = root();
+    let loose = loose_corpus();
+    // 251 loose programs at 5.2a.
+    assert!(loose.len() >= 240, "only {} loose programs", loose.len());
+    let cases = resolve_cases();
+    // 103 cases at 5.2a.
+    assert!(cases.len() >= 90, "only {} resolver cases", cases.len());
+    let mut files = loose;
+    files.extend(cases);
+    let mut dumps = s0_resolution(&root, &files, 0);
+    let parts = dumps.lines().filter(|l| l.starts_with("(Program ")).count();
+    assert_eq!(parts, files.len());
+    // None of the codes that only 5.2b's test mode and WASM target give
+    // (decision 8), though cases hold the programs that would give them.
+    let codes = codes_in(&dumps);
+    for code in LATER_RESOLVER_CODES {
+        assert!(!codes.contains_key(code), "{code} in {codes:?}");
+    }
+    let std = run(
+        &nova(),
+        &["check", "--dump-resolution", "--std"],
+        &root,
+        &[],
+    )
+    .unwrap_or_else(|e| panic!("{e}"));
+    assert_eq!(std.code, Some(0), "{}", std.stderr);
+    dumps.push_str(&String::from_utf8_lossy(&std.stdout));
+    let mut found = std::collections::HashSet::new();
+    for line in dumps.lines() {
+        found.extend(nova_driver::resolution_dump::forms_on(line));
+    }
+    let missing: Vec<&str> = nova_driver::resolution_dump::KINDS
+        .iter()
+        .copied()
+        .filter(|k| !found.contains(*k))
+        .collect();
+    assert!(missing.is_empty(), "no S0 dump holds {missing:?}");
+    let unlisted: Vec<&String> = found
+        .iter()
+        .filter(|f| !nova_driver::resolution_dump::listed(f))
+        .collect();
+    assert!(unlisted.is_empty(), "{unlisted:?} are in no list");
+    let later: Vec<&str> = nova_driver::resolution_dump::LATER_KINDS
+        .iter()
+        .copied()
+        .filter(|k| found.contains(*k))
+        .collect();
+    assert!(
+        later.is_empty(),
+        "{later:?} appear, which LATER_KINDS says 5.2a's programs never print"
+    );
+}
+
+/// The name fuzzer's fixed seeds and counts every run checks (spec §3.5).
+const NAME_SEEDS: [u64; 3] = [301, 302, 303];
+const NAME_COUNT: usize = 50;
+
+/// The resolver's codes that `check`'s pipeline gives a loose program on
+/// the native target. Of its 11, E0084 and E0085 are a `@test` function's,
+/// which `check` strips before resolving, and E0090 is WASM's: they come
+/// with 5.2b's test mode and target.
+const NATIVE_RESOLVER_CODES: [&str; 8] = [
+    "E0001", "E0002", "E0082", "E0083", "E0089", "E0092", "E0102", "E0900",
+];
+const LATER_RESOLVER_CODES: [&str; 3] = ["E0084", "E0085", "E0090"];
+
+/// `count` name-mutated programs from `seed` ([`fuzz::name_mutate`]), each
+/// one of `files`, under `root`, that the seed picks and that parses. As
+/// for [`mutations_of`], the files are picked from their sorted list and
+/// read with each `\r\n` as `\n`, so a seed gives the same inputs on every
+/// system. Input `i` is written as `f{i:05}.nova`, which a self-import
+/// names.
+fn names_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<String> {
+    let mut files = files.to_vec();
+    files.sort();
+    let mut rng = SplitMix64::new(seed);
+    (0..count)
+        .map(|i| loop {
+            let file = rng.pick(&files);
+            let source = std::fs::read_to_string(root.join(file))
+                .expect("read a corpus file")
+                .replace("\r\n", "\n");
+            if fuzz::parses(&source) {
+                break fuzz::name_mutate(&mut rng, &source, &format!("f{i:05}"));
+            }
+        })
+        .collect()
+}
+
+/// The name-mutated loose corpus: [`names_of`] it.
+fn names(seed: u64, count: usize) -> Vec<String> {
+    names_of(&root(), &loose_corpus(), seed, count)
+}
+
+/// Write `inputs` to a fixed directory named after `name`, as
+/// `f00000.nova` and on, beside [`fuzz::NAME_SIBLINGS`]: the directory, and
+/// the inputs' files, each a program's root.
+fn write_name_inputs(name: &str, inputs: &[String]) -> (PathBuf, Vec<String>) {
+    let dir = fresh_dir(name);
+    for (stem, source) in fuzz::NAME_SIBLINGS {
+        std::fs::write(dir.join(format!("{stem}.nova")), source).expect("write a sibling");
+    }
+    let mut files = Vec::with_capacity(inputs.len());
+    for (i, input) in inputs.iter().enumerate() {
+        let file = format!("f{i:05}.nova");
+        std::fs::write(dir.join(&file), input).expect("write a name-fuzz input");
+        files.push(file);
+    }
+    (dir, files)
+}
+
+/// How many diagnostics of each code `dump`, a resolution dump, holds.
+fn codes_in(dump: &str) -> std::collections::BTreeMap<String, usize> {
+    let mut codes = std::collections::BTreeMap::new();
+    for line in dump.lines() {
+        for kind in ["  (Error \"", "  (Warning \""] {
+            if let Some(rest) = line.strip_prefix(kind) {
+                let code = rest.split('"').next().unwrap_or("").to_string();
+                *codes.entry(code).or_insert(0) += 1;
+            }
+        }
+    }
+    codes
+}
+
+/// S0's resolution dump of each of the name fuzzer's inputs from the fixed
+/// seeds: every input reaches the resolver, and among them every resolver
+/// code `check` gives a loose native program occurs twice or more, and
+/// none of the three only test mode and WASM give. N's dumps are compared
+/// with these from Task 6 on.
+#[test]
+fn the_name_fuzzer_gives_every_native_resolver_code() {
+    let mut all = std::collections::BTreeMap::new();
+    for seed in NAME_SEEDS {
+        let inputs = names(seed, NAME_COUNT);
+        let (dir, files) = write_name_inputs(&format!("names-{seed}"), &inputs);
+        let dump = s0_resolution(&dir, &files, 0);
+        let resolved = dump.matches("resolved=true").count();
+        assert_eq!(
+            resolved,
+            files.len(),
+            "seed {seed}: an input did not reach the resolver"
+        );
+        for (code, n) in codes_in(&dump) {
+            *all.entry(code).or_insert(0) += n;
+        }
+    }
+    eprintln!("codes over seeds {NAME_SEEDS:?}: {all:?}");
+    // At 5.2a: E0001 46, E0002 63, E0082 14, E0083 8, E0089 10, E0092 13,
+    // E0102 50, E0900 18.
+    for code in NATIVE_RESOLVER_CODES {
+        let n = all.get(code).copied().unwrap_or(0);
+        assert!(n >= 2, "{code} occurs {n} times, under 2, in {all:?}");
+    }
+    for code in LATER_RESOLVER_CODES {
+        assert!(!all.contains_key(code), "{code} in {all:?}");
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_is_dropped_when_one_items_keyword_is_not_on_its_line/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_is_dropped_when_one_items_keyword_is_not_on_its_line/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6759b56cb5f17569a0644b774f101b232f480450
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_is_dropped_when_one_items_keyword_is_not_on_its_line/lib.nova
@@ -0,0 +1,3 @@
+fn secret() {}
+const
+  secret: Int = 1
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_is_dropped_when_one_items_keyword_is_not_on_its_line/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_is_dropped_when_one_items_keyword_is_not_on_its_line/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..22c59953553d66ab7c6718dc2296c1fe01a7c541
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_is_dropped_when_one_items_keyword_is_not_on_its_line/main.nova
@@ -0,0 +1,3 @@
+import lib::{secret}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_a_listed_import_would_bind_twice_is_dropped/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_a_listed_import_would_bind_twice_is_dropped/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7437352fee81d17b1e809f65695b737c5b4292e4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_a_listed_import_would_bind_twice_is_dropped/lib.nova
@@ -0,0 +1 @@
+fn secret() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_a_listed_import_would_bind_twice_is_dropped/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_a_listed_import_would_bind_twice_is_dropped/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5d544b76226ee68b296c52a0a97f55b5ca2cc091
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_a_listed_import_would_bind_twice_is_dropped/main.nova
@@ -0,0 +1,4 @@
+import lib::{secret}
+import other
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_a_listed_import_would_bind_twice_is_dropped/other.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_a_listed_import_would_bind_twice_is_dropped/other.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7c7140094ff37b19c4ae4656354ea3cee45c4423
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_a_listed_import_would_bind_twice_is_dropped/other.nova
@@ -0,0 +1,3 @@
+import lib::{secret}
+
+fn secret() -> Int { 2 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_name_twice_is_dropped/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_name_twice_is_dropped/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7437352fee81d17b1e809f65695b737c5b4292e4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_name_twice_is_dropped/lib.nova
@@ -0,0 +1 @@
+fn secret() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_name_twice_is_dropped/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_name_twice_is_dropped/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5d544b76226ee68b296c52a0a97f55b5ca2cc091
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_name_twice_is_dropped/main.nova
@@ -0,0 +1,4 @@
+import lib::{secret}
+import other
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_name_twice_is_dropped/other.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_name_twice_is_dropped/other.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a6707ab106bd6b3db37fa80d1376af1a16b29ba8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_name_twice_is_dropped/other.nova
@@ -0,0 +1,3 @@
+import lib
+
+fn secret() -> Int { 2 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_type_twice_is_dropped/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_type_twice_is_dropped/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7437352fee81d17b1e809f65695b737c5b4292e4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_type_twice_is_dropped/lib.nova
@@ -0,0 +1 @@
+fn secret() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_type_twice_is_dropped/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_type_twice_is_dropped/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5d544b76226ee68b296c52a0a97f55b5ca2cc091
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_type_twice_is_dropped/main.nova
@@ -0,0 +1,4 @@
+import lib::{secret}
+import other
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_type_twice_is_dropped/other.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_type_twice_is_dropped/other.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2a478566b131e321b7f8a1affaa98081d7ff0b4c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_fix_that_would_bind_a_type_twice_is_dropped/other.nova
@@ -0,0 +1,3 @@
+import lib
+
+record secret { x: Int }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_parse_error_with_a_note_stops_the_program.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_parse_error_with_a_note_stops_the_program.nova
new file mode 100644
index 0000000000000000000000000000000000000000..24cbb9e6ee3143b1252bffe65f6e05262eceee4c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_parse_error_with_a_note_stops_the_program.nova
@@ -0,0 +1,9 @@
+fn f() {
+match x {
+    0 => a
+    - 1,
+    _ => 2
+}
+}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_root_that_imports_itself_is_e0001.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_root_that_imports_itself_is_e0001.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7e8c612efed18fae9a51c5b5c747ee1dfc47aeed
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_root_that_imports_itself_is_e0001.nova
@@ -0,0 +1,3 @@
+import a_root_that_imports_itself_is_e0001
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_named_for_a_std_module_is_e0093/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_named_for_a_std_module_is_e0093/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5543aaedc0d029b29243ed345c678ef1825e79a3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_named_for_a_std_module_is_e0093/main.nova
@@ -0,0 +1,3 @@
+import ui
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_named_for_a_std_module_is_e0093/ui.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_named_for_a_std_module_is_e0093/ui.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cdc6c27d800bdf61531058e432b2217e45a34dc7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_named_for_a_std_module_is_e0093/ui.nova
@@ -0,0 +1 @@
+pub fn f() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_that_fails_to_lex_or_parse_stops_the_program/broken.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_that_fails_to_lex_or_parse_stops_the_program/broken.nova
new file mode 100644
index 0000000000000000000000000000000000000000..25f46c074e6fd7a9002436a2b5ca9b81621499dc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_that_fails_to_lex_or_parse_stops_the_program/broken.nova
@@ -0,0 +1 @@
+pub fn f( {
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_that_fails_to_lex_or_parse_stops_the_program/lexbad.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_that_fails_to_lex_or_parse_stops_the_program/lexbad.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5bb71cfeef5ada51c2653f660d3b744ab0c3a2b3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_that_fails_to_lex_or_parse_stops_the_program/lexbad.nova
@@ -0,0 +1 @@
+pub fn g() { let s = "abc }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_that_fails_to_lex_or_parse_stops_the_program/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_that_fails_to_lex_or_parse_stops_the_program/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..609b621e9e3225299091c84dd3410e9d7d4bcfab
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_sibling_that_fails_to_lex_or_parse_stops_the_program/main.nova
@@ -0,0 +1,4 @@
+import broken
+import lexbad
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_test_attribute_on_an_impl_block_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_test_attribute_on_an_impl_block_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c4f1f359b443965cd3dbd33bfe08d2776bb68305
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_test_attribute_on_an_impl_block_is_e0083.nova
@@ -0,0 +1,7 @@
+trait Tr {}
+record R { x: Int }
+
+@test
+impl Tr for R {}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_test_function_that_would_collide_is_stripped.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_test_function_that_would_collide_is_stripped.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fc0d40d1b01f914ebeba1ff2c710436c30910225
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_test_function_that_would_collide_is_stripped.nova
@@ -0,0 +1,6 @@
+fn helper() {}
+
+@test
+fn helper() {}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/an_empty_file.nova b/crates/nova-cli/tests/selfhost/cases/resolve/an_empty_file.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e69de29bb2d1d6434b8b29ae775ad8c2e48c5391
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/an_import_differing_in_case_finds_no_module/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/an_import_differing_in_case_finds_no_module/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..84dfd96c56ddfe3c074ac5e4f4e3e78bbe53fa3e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/an_import_differing_in_case_finds_no_module/main.nova
@@ -0,0 +1,3 @@
+import Shapes
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/an_import_differing_in_case_finds_no_module/shapes.nova b/crates/nova-cli/tests/selfhost/cases/resolve/an_import_differing_in_case_finds_no_module/shapes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a92bed8bc0325a2d33410e9279c331fade798043
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/an_import_differing_in_case_finds_no_module/shapes.nova
@@ -0,0 +1 @@
+pub fn area() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/an_import_of_a_directory_finds_no_module/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/an_import_of_a_directory_finds_no_module/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4c73305a8f70c16a76e404af1cc5863aea18aff8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/an_import_of_a_directory_finds_no_module/main.nova
@@ -0,0 +1,3 @@
+import sub
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/an_import_of_a_directory_finds_no_module/sub.nova/inner.nova b/crates/nova-cli/tests/selfhost/cases/resolve/an_import_of_a_directory_finds_no_module/sub.nova/inner.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cdc6c27d800bdf61531058e432b2217e45a34dc7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/an_import_of_a_directory_finds_no_module/sub.nova/inner.nova
@@ -0,0 +1 @@
+pub fn f() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__a_next_returning_a_three_variant_sum_is_not_an_iterator.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__a_next_returning_a_three_variant_sum_is_not_an_iterator.nova
new file mode 100644
index 0000000000000000000000000000000000000000..416c9b8f49605699438537e5c72089696616a5db
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__a_next_returning_a_three_variant_sum_is_not_an_iterator.nova
@@ -0,0 +1,6 @@
+type Tri = | Some(Int) | None | Third
+record R { n: Int }
+trait It3 { fn next(mut self) -> Tri }
+impl It3 for R { fn next(mut self) -> Tri { Third } }
+fn main() { let r = R { n: 0 }
+ for x in r { println("${x}") } }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__a_rejected_declarations_own_body_is_never_checked_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__a_rejected_declarations_own_body_is_never_checked_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8bdf8c93cb42c2b0100dd9e41b0e1797a44d6ef9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__a_rejected_declarations_own_body_is_never_checked_1.nova
@@ -0,0 +1,2 @@
+record Bool { v: Nope }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__a_rejected_declarations_own_body_is_never_checked_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__a_rejected_declarations_own_body_is_never_checked_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..887dd92aee292903e85b26773976b02d40cf92e3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__a_rejected_declarations_own_body_is_never_checked_2.nova
@@ -0,0 +1,2 @@
+type Bool = | A(Nope) | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_before_a_module_is_e0082_and_names_it.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_before_a_module_is_e0082_and_names_it.nova
new file mode 100644
index 0000000000000000000000000000000000000000..78330bc04a9c971be15b1b01a0730c63d8527673
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_before_a_module_is_e0082_and_names_it.nova
@@ -0,0 +1,3 @@
+@tset
+module foo;
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_before_an_extern_block_is_e0082_and_names_it.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_before_an_extern_block_is_e0082_and_names_it.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2ec56a1c73c392e3de7e64e95bbe80edcde037ff
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_before_an_extern_block_is_e0082_and_names_it.nova
@@ -0,0 +1,3 @@
+@tset
+extern { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_before_an_import_is_e0082_and_names_it.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_before_an_import_is_e0082_and_names_it.nova
new file mode 100644
index 0000000000000000000000000000000000000000..961e94006231b0dd14edad6afa654a8afa1755db
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_before_an_import_is_e0082_and_names_it.nova
@@ -0,0 +1,3 @@
+@tset
+import core
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_is_e0082_and_names_it.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_is_e0082_and_names_it.nova
new file mode 100644
index 0000000000000000000000000000000000000000..473626f40e04858e14d5348d223a7eebbedcee0f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_attribute_is_e0082_and_names_it.nova
@@ -0,0 +1,3 @@
+@tset
+fn t() { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_test_argument_is_e0085.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_test_argument_is_e0085.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f936da3e4bfd81fc1c51ecf702962609a385bc7e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__an_unknown_test_argument_is_e0085.nova
@@ -0,0 +1,3 @@
+@test(shuold_panic)
+fn t() { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5510dabf5a18ebcf8f0c58abb794206ee4a05a2c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_1.nova
@@ -0,0 +1,2 @@
+record Int { v: Bool }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_10.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_10.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b502409d4c603c8b8f875d02546c7d3bab07152c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_10.nova
@@ -0,0 +1,2 @@
+type String = | A | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_11.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_11.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1499a575bb55b5c0517aa1a7aa00d43122990d90
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_11.nova
@@ -0,0 +1,2 @@
+record Future { v: Bool }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_12.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_12.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2e548539702f72dca54561ee180754f27f580f8d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_12.nova
@@ -0,0 +1,2 @@
+type Future = | A | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_13.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_13.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1b70e557b0a02a322adb43d2085659f27900bfa8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_13.nova
@@ -0,0 +1,2 @@
+record Bytes { v: Bool }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_14.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_14.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a01bfc9527c13b580ea62317dc2d8bc29512a0bc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_14.nova
@@ -0,0 +1,2 @@
+type Bytes = | A | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d7719779c884394d149401b79a1f8214a47ee4bc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_2.nova
@@ -0,0 +1,2 @@
+type Int = | A | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_3.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c8401c571decd24be3d0d2e1f6156b6b205ce5c2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_3.nova
@@ -0,0 +1,2 @@
+record Float { v: Bool }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_4.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c696b37f4756b4650047da48e1a5c9fc38358a4f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_4.nova
@@ -0,0 +1,2 @@
+type Float = | A | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_5.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4544520da79f8769e5fbed23d2f09dd40101b340
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_5.nova
@@ -0,0 +1,2 @@
+record Bool { v: Bool }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_6.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_6.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1de5d7c7e6e1fbbfa2a079dea3ba237971f29ff3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_6.nova
@@ -0,0 +1,2 @@
+type Bool = | A | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_7.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_7.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7f0025f5243cbcd7f66d25bbea57c9d1a6780e75
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_7.nova
@@ -0,0 +1,2 @@
+record Char { v: Bool }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_8.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_8.nova
new file mode 100644
index 0000000000000000000000000000000000000000..16ac1f726144053880ea8dbef33ae865ff7f6e98
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_8.nova
@@ -0,0 +1,2 @@
+type Char = | A | B
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_9.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_9.nova
new file mode 100644
index 0000000000000000000000000000000000000000..69989f532bb868b1cbcd292ba20a7ca826ac29c8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__declaring_a_type_named_for_a_builtin_is_rejected_9.nova
@@ -0,0 +1,2 @@
+record String { v: Bool }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__duplicate_variant_arm_is_unreachable_e0021.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__duplicate_variant_arm_is_unreachable_e0021.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c13a30a476cbe7add313bade3b4a74b308aee304
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__duplicate_variant_arm_is_unreachable_e0021.nova
@@ -0,0 +1,3 @@
+type Opt = | Some(Int) | None
+fn f(o: Opt) -> Int { match o { None => 0, Some(x) => x, None => 9 } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__non_exhaustive_witness_names_missing_variant.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__non_exhaustive_witness_names_missing_variant.nova
new file mode 100644
index 0000000000000000000000000000000000000000..256ba50f1f38f6e68de55f4651d294245a338782
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__non_exhaustive_witness_names_missing_variant.nova
@@ -0,0 +1,3 @@
+type Opt = | Some(Int) | None
+fn f(o: Opt) -> Int { match o { Some(x) => x } }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__test_before_a_module_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_before_a_module_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..47e5719946db0b30117de1c2152fe81681235ff7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_before_a_module_is_e0083.nova
@@ -0,0 +1,3 @@
+@test
+module foo;
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__test_before_an_extern_block_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_before_an_extern_block_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8067c3b69cbb5fb23c496c1d0c0753ec9aa968dc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_before_an_extern_block_is_e0083.nova
@@ -0,0 +1,3 @@
+@test
+extern { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__test_before_an_import_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_before_an_import_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0e0ef1cd536b587eb1fe1fbdb81e6c47e4b52663
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_before_an_import_is_e0083.nova
@@ -0,0 +1,3 @@
+@test
+import core
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_function_returning_a_value_is_e0084.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_function_returning_a_value_is_e0084.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0d4d655977da09d11c1b4d2d135e0e4337c57f3e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_function_returning_a_value_is_e0084.nova
@@ -0,0 +1,3 @@
+@test
+fn t() -> Int { 1 }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_function_with_parameters_is_e0084.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_function_with_parameters_is_e0084.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2386927bc67d2badcb6edf02a5b31b9100a67d4a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_function_with_parameters_is_e0084.nova
@@ -0,0 +1,3 @@
+@test
+fn t(x: Int) { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_generic_function_is_e0084.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_generic_function_is_e0084.nova
new file mode 100644
index 0000000000000000000000000000000000000000..627b38dbf20ad9fb1f2be9a2ed5f653ae76157f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_generic_function_is_e0084.nova
@@ -0,0 +1,3 @@
+@test
+fn t<T>() { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_non_function_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_non_function_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..537074e237de5c908cadab98f0fc12feba2f68c0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_a_non_function_is_e0083.nova
@@ -0,0 +1,3 @@
+@test
+record R { n: Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_an_async_function_is_e0084.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_an_async_function_is_e0084.nova
new file mode 100644
index 0000000000000000000000000000000000000000..93afb6873bc5fcd9e977b5b89d077ab3daca89fa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__test_on_an_async_function_is_e0084.nova
@@ -0,0 +1,3 @@
+@test
+async fn t() { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/check__two_declarations_of_the_same_reserved_name_each_raise_e0089_not_e0002.nova b/crates/nova-cli/tests/selfhost/cases/resolve/check__two_declarations_of_the_same_reserved_name_each_raise_e0089_not_e0002.nova
new file mode 100644
index 0000000000000000000000000000000000000000..79ec7de6d4d74751305ac66291e3b91cbf94605e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/check__two_declarations_of_the_same_reserved_name_each_raise_e0089_not_e0002.nova
@@ -0,0 +1,3 @@
+record Bool { v: Int }
+record Bool { w: Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/impls_for_a_function_type_and_the_unit_type_name_their_methods.nova b/crates/nova-cli/tests/selfhost/cases/resolve/impls_for_a_function_type_and_the_unit_type_name_their_methods.nova
new file mode 100644
index 0000000000000000000000000000000000000000..122975dd7ef106931b4a0a53db482092830b3b8b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/impls_for_a_function_type_and_the_unit_type_name_their_methods.nova
@@ -0,0 +1,7 @@
+trait Tr {}
+
+impl Tr for fn(Int) -> Int {}
+impl Tr for fn(Int) -> Int { fn a(self) {} }
+impl Tr for () { fn b(self) {} }
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/import_cycles_load_each_module_once/left.nova b/crates/nova-cli/tests/selfhost/cases/resolve/import_cycles_load_each_module_once/left.nova
new file mode 100644
index 0000000000000000000000000000000000000000..01f1c5d92891a66ef2953ed391b0363713df3e92
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/import_cycles_load_each_module_once/left.nova
@@ -0,0 +1,3 @@
+import right
+
+pub fn l() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/import_cycles_load_each_module_once/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/import_cycles_load_each_module_once/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8a7249f828bd060df97d9ef5a27a682fba6c33e1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/import_cycles_load_each_module_once/main.nova
@@ -0,0 +1,4 @@
+import left
+import right
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/import_cycles_load_each_module_once/right.nova b/crates/nova-cli/tests/selfhost/cases/resolve/import_cycles_load_each_module_once/right.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0c105c91a28ecd732aba2d71c835c689c8f897f2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/import_cycles_load_each_module_once/right.nova
@@ -0,0 +1,4 @@
+import left
+import main
+
+pub fn r() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/lang_tests__try_in_a_function_returning_its_own_option_says_so.nova b/crates/nova-cli/tests/selfhost/cases/resolve/lang_tests__try_in_a_function_returning_its_own_option_says_so.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2c56f93337f7cba255b1ca102146e8bcaa1b8ed9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/lang_tests__try_in_a_function_returning_its_own_option_says_so.nova
@@ -0,0 +1,8 @@
+type Option<T> =
+    | Some(T)
+    | None
+fn f(s: String) -> Option<Int> {
+    let i = s.index_of("b")?
+    Some(i)
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/lang_tests__try_on_the_programs_own_option_is_e0098.nova b/crates/nova-cli/tests/selfhost/cases/resolve/lang_tests__try_on_the_programs_own_option_is_e0098.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9f69f7e1adcf791f61f5d4c803827e7142b18a9c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/lang_tests__try_on_the_programs_own_option_is_e0098.nova
@@ -0,0 +1,8 @@
+type Option<T> =
+    | Some(T)
+    | None
+fn f(o: Option<Int>) -> Option<Int> {
+    let v = o?
+    Some(v)
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/pattern_tests__e0104_on_a_variant_not_imported_says_to_qualify_or_import_it/lex.nova b/crates/nova-cli/tests/selfhost/cases/resolve/pattern_tests__e0104_on_a_variant_not_imported_says_to_qualify_or_import_it/lex.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bb390e73e65d35134a41a1f74ae0eca6148fd883
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/pattern_tests__e0104_on_a_variant_not_imported_says_to_qualify_or_import_it/lex.nova
@@ -0,0 +1,3 @@
+pub type Tok =
+  | Eof
+  | Num(Int)
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/pattern_tests__e0104_on_a_variant_not_imported_says_to_qualify_or_import_it/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/pattern_tests__e0104_on_a_variant_not_imported_says_to_qualify_or_import_it/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..eaee7bfca6afd369a6adcf274070ef18b5ded056
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/pattern_tests__e0104_on_a_variant_not_imported_says_to_qualify_or_import_it/main.nova
@@ -0,0 +1,11 @@
+import lex::{Tok}
+fn f(o: Opt<Tok>) -> Int {
+    match o {
+        Yes(Eof) => 1
+        _ => 2
+    }
+}
+type Opt<T> =
+  | Yes(T)
+  | No
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_glob_imports_clashes_are_reported_in_declaration_order/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_glob_imports_clashes_are_reported_in_declaration_order/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a913d3b0b93b48f26170df5abdd3d9b17a3b7c25
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_glob_imports_clashes_are_reported_in_declaration_order/lib.nova
@@ -0,0 +1,9 @@
+pub fn alpha() -> Int { 1 }
+pub fn beta() -> Int { 2 }
+pub fn gamma() -> Int { 3 }
+pub fn delta() -> Int { 4 }
+pub fn epsilon() -> Int { 5 }
+pub record Zeta { x: Int }
+pub record Eta { x: Int }
+pub trait Tau {}
+pub trait Sigma {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_glob_imports_clashes_are_reported_in_declaration_order/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_glob_imports_clashes_are_reported_in_declaration_order/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2a82889d586e5fa84632e8e4b80ca7682a7b57b5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_glob_imports_clashes_are_reported_in_declaration_order/main.nova
@@ -0,0 +1,11 @@
+fn alpha() -> Int { 10 }
+fn beta() -> Int { 20 }
+fn gamma() -> Int { 30 }
+fn delta() -> Int { 40 }
+fn epsilon() -> Int { 50 }
+record Zeta { y: Int }
+record Eta { y: Int }
+trait Tau {}
+trait Sigma {}
+import lib
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_list_imports_module_is_its_first_segment/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_list_imports_module_is_its_first_segment/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a92bed8bc0325a2d33410e9279c331fade798043
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_list_imports_module_is_its_first_segment/lib.nova
@@ -0,0 +1 @@
+pub fn area() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_list_imports_module_is_its_first_segment/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_list_imports_module_is_its_first_segment/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..faedc487b0ce179023662838dbda72aeacba0982
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_list_imports_module_is_its_first_segment/main.nova
@@ -0,0 +1,2 @@
+import lib::{area}
+fn main() { let a = area() }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_listed_name_is_a_use_of_each_namespace_it_binds/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_listed_name_is_a_use_of_each_namespace_it_binds/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..57dbdfc8708b9934586aff725536162f18559fbe
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_listed_name_is_a_use_of_each_namespace_it_binds/lib.nova
@@ -0,0 +1,3 @@
+pub fn area() -> Int { 1 }
+pub type Figure = | Round(Int) | Square(Int)
+pub trait Paint { fn paint(self) -> String }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_listed_name_is_a_use_of_each_namespace_it_binds/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_listed_name_is_a_use_of_each_namespace_it_binds/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c063c96d3c80db35d3141898e3a8fd07a9469ae2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_listed_name_is_a_use_of_each_namespace_it_binds/main.nova
@@ -0,0 +1,2 @@
+import lib::{area, Figure, Paint, Round}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_name_bound_through_std_is_not_bound_outside_it/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_name_bound_through_std_is_not_bound_outside_it/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a92bed8bc0325a2d33410e9279c331fade798043
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_name_bound_through_std_is_not_bound_outside_it/lib.nova
@@ -0,0 +1 @@
+pub fn area() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_name_bound_through_std_is_not_bound_outside_it/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_name_bound_through_std_is_not_bound_outside_it/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4db0d360e4e0eec1f3198363e08704a818294605
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_name_bound_through_std_is_not_bound_outside_it/main.nova
@@ -0,0 +1,2 @@
+record Option { x: Int }
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_name_that_is_not_one_of_stds_variants_is_not_reported.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_name_that_is_not_one_of_stds_variants_is_not_reported.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4310f341533ade47b5898a7ad35173f9254615f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_name_that_is_not_one_of_stds_variants_is_not_reported.nova
@@ -0,0 +1,5 @@
+type Token =
+    | Word(String)
+record JsonValue { x: Int }
+fn args() -> Int { 1 }
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fbd3b610e911050c295074159c6e3a756fb33f5e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_1.nova
@@ -0,0 +1,2 @@
+import ui::router::{link}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9697bb6d9a5498868693e4d7508d577484a9ea98
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_2.nova
@@ -0,0 +1,2 @@
+import ui
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_3.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b1e3a58db64316012a16f7b8fa89f94517ad7038
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_3.nova
@@ -0,0 +1,2 @@
+import ui::html::{div}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_4.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..06a2ca2d0b95ccb100973b6dfc31b8ba3c54f2ff
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_4.nova
@@ -0,0 +1,2 @@
+import ui::html
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_5.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3f90aa91ddabd395732913a54a0bf2c206f4b88c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_5.nova
@@ -0,0 +1,2 @@
+import a::b
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_6.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_6.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a2ac1f9607a89f273855cd5d74ecca9fdb5afe36
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_nested_import_path_names_only_stds_modules_6.nova
@@ -0,0 +1,2 @@
+import ui::nope
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_qualified_use_of_stds_variant_is_fine.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_qualified_use_of_stds_variant_is_fine.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3f814e34d6d2a4c63432bc5cc44d15af8e99156d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_qualified_use_of_stds_variant_is_fine.nova
@@ -0,0 +1,6 @@
+type Token =
+    | Number(Int)
+fn main() {
+    let j = JsonValue::Number(1.0)
+    let t = Number(1)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_reported_import_binds_nothing_and_says_nothing.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_reported_import_binds_nothing_and_says_nothing.nova
new file mode 100644
index 0000000000000000000000000000000000000000..89bdc7d12c5409eeb2122fd9c824de356e02f4da
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_reported_import_binds_nothing_and_says_nothing.nova
@@ -0,0 +1,2 @@
+import geom
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_std_name_the_module_shadows_is_listed_once_as_its_own/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_std_name_the_module_shadows_is_listed_once_as_its_own/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cdc6c27d800bdf61531058e432b2217e45a34dc7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_std_name_the_module_shadows_is_listed_once_as_its_own/lib.nova
@@ -0,0 +1 @@
+pub fn f() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_std_name_the_module_shadows_is_listed_once_as_its_own/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_std_name_the_module_shadows_is_listed_once_as_its_own/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6b5a66c1d6541a07eff511faf4cd7ba4477a1945
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_std_name_the_module_shadows_is_listed_once_as_its_own/main.nova
@@ -0,0 +1,2 @@
+record Vec { n: Int }
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_variant_named_as_one_of_stds_is_e0102.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_variant_named_as_one_of_stds_is_e0102.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9cadb0326b96ca335a4de0baf4953a9fac786855
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_variant_named_as_one_of_stds_is_e0102.nova
@@ -0,0 +1,4 @@
+type Token =
+    | Number(Int)
+    | Word(String)
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_wasm_build_compiles_wasms_std_modules.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_wasm_build_compiles_wasms_std_modules.nova
new file mode 100644
index 0000000000000000000000000000000000000000..45590d86ba6c51f0babffb0b43e3e2f44d2f9e07
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__a_wasm_build_compiles_wasms_std_modules.nova
@@ -0,0 +1 @@
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_extern_block_is_e0090_on_wasm.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_extern_block_is_e0090_on_wasm.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5a967529c7b14d22a9a92b5719f4e2bdd5dc446a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_extern_block_is_e0090_on_wasm.nova
@@ -0,0 +1,4 @@
+extern "C" {
+    fn abs(x: Int) -> Int
+}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_missing_from_the_table_cannot_find_its_module/geom.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_missing_from_the_table_cannot_find_its_module/geom.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a92bed8bc0325a2d33410e9279c331fade798043
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_missing_from_the_table_cannot_find_its_module/geom.nova
@@ -0,0 +1 @@
+pub fn area() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_missing_from_the_table_cannot_find_its_module/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_missing_from_the_table_cannot_find_its_module/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..89bdc7d12c5409eeb2122fd9c824de356e02f4da
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_missing_from_the_table_cannot_find_its_module/main.nova
@@ -0,0 +1,2 @@
+import geom
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_table_can_name_a_module_by_any_label/a_label_no_import_names.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_table_can_name_a_module_by_any_label/a_label_no_import_names.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a92bed8bc0325a2d33410e9279c331fade798043
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_table_can_name_a_module_by_any_label/a_label_no_import_names.nova
@@ -0,0 +1 @@
+pub fn area() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_table_can_name_a_module_by_any_label/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_table_can_name_a_module_by_any_label/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7dd8c6e9afd05b4f9c7295ba6a6ff01a0ec8f784
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_import_table_can_name_a_module_by_any_label/main.nova
@@ -0,0 +1,2 @@
+import geom::{area}
+fn main() { let a = area() }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_imports_module_is_a_use_of_it/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_imports_module_is_a_use_of_it/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a92bed8bc0325a2d33410e9279c331fade798043
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_imports_module_is_a_use_of_it/lib.nova
@@ -0,0 +1 @@
+pub fn area() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_imports_module_is_a_use_of_it/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_imports_module_is_a_use_of_it/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..837579566324dc5c06f5d191f38995f90285f80c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_imports_module_is_a_use_of_it/main.nova
@@ -0,0 +1,2 @@
+import lib
+fn main() { let a = area() }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_unresolved_import_records_nothing/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_unresolved_import_records_nothing/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a92bed8bc0325a2d33410e9279c331fade798043
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_unresolved_import_records_nothing/lib.nova
@@ -0,0 +1 @@
+pub fn area() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_unresolved_import_records_nothing/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_unresolved_import_records_nothing/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fb3eb5a5cad3e5ca7922758d5cbfbe66c7060d51
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__an_unresolved_import_records_nothing/main.nova
@@ -0,0 +1,3 @@
+import nowhere
+import lib::{missing}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__another_modules_variant_is_reported_too/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__another_modules_variant_is_reported_too/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0346f2231c34128285473b0240980634a819d070
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__another_modules_variant_is_reported_too/lib.nova
@@ -0,0 +1,3 @@
+pub type Answer =
+    | Ok
+    | Maybe
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__another_modules_variant_is_reported_too/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__another_modules_variant_is_reported_too/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f328e4d9d04c31d0d70d16d21a07d1613be9d577
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__another_modules_variant_is_reported_too/main.nova
@@ -0,0 +1 @@
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__associated_type_is_not_in_the_type_namespace.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__associated_type_is_not_in_the_type_namespace.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d5d5e8aa684831fe2294f660efa6119fa637a951
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__associated_type_is_not_in_the_type_namespace.nova
@@ -0,0 +1,2 @@
+trait It { type Item }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__collects_functions_and_sum_types.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__collects_functions_and_sum_types.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d4c5ae867c10ca739d0485522e62698f0e2643b9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__collects_functions_and_sum_types.nova
@@ -0,0 +1,3 @@
+type Shape = | Circle(Int) | Empty
+fn area(s: Shape) -> Int { 0 }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__collects_records.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__collects_records.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0ffb6d896a49565f3ca7739a8f04f361d9a7d05e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__collects_records.nova
@@ -0,0 +1,2 @@
+record Point { x: Float, y: Float }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__collects_traits_and_impl_methods.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__collects_traits_and_impl_methods.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0fc78222ae25f7b1893b8b6c28e3b1bfaba85598
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__collects_traits_and_impl_methods.nova
@@ -0,0 +1,6 @@
+record P { v: Int }
+trait Show { fn name(self) -> String
+ fn shout(self) -> String { self.name() } }
+impl Show for P { fn name(self) -> String { "p" } }
+impl P { fn get(self) -> Int { self.v } }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dangling_import_reports_missing_module/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dangling_import_reports_missing_module/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2e294688062c89d3796292c2a472dbc9231bf683
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dangling_import_reports_missing_module/lib.nova
@@ -0,0 +1 @@
+pub fn add() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dangling_import_reports_missing_module/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dangling_import_reports_missing_module/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0408aa6fb0d5c9d051646e77c379476aee1d14f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dangling_import_reports_missing_module/main.nova
@@ -0,0 +1,2 @@
+import nope::{x}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dom_is_seen_only_through_an_import_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dom_is_seen_only_through_an_import_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..424868c66db74cf7850e7b527432c805e46444f8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dom_is_seen_only_through_an_import_1.nova
@@ -0,0 +1,2 @@
+import dom::{Element}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dom_is_seen_only_through_an_import_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dom_is_seen_only_through_an_import_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..36f82968d85b87f4d0d1072862fdcb546d34a3b7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__dom_is_seen_only_through_an_import_2.nova
@@ -0,0 +1,2 @@
+import dom
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__duplicate_function_reports_e0002.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__duplicate_function_reports_e0002.nova
new file mode 100644
index 0000000000000000000000000000000000000000..07a372872aca0de5921741f49b2920f83d907d41
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__duplicate_function_reports_e0002.nova
@@ -0,0 +1,2 @@
+fn f() { }
+fn f() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__each_hidden_variant_is_reported.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__each_hidden_variant_is_reported.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4157a91cfa297c181813ec49ccccb22990ab2b58
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__each_hidden_variant_is_reported.nova
@@ -0,0 +1,5 @@
+type Mine =
+    | Some(Int)
+    | None
+    | NotFound
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__exported_names_each_namespace_a_public_name_has/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__exported_names_each_namespace_a_public_name_has/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..73c865c4b1672ec169816aa0f0f06da8ea6ec98f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__exported_names_each_namespace_a_public_name_has/lib.nova
@@ -0,0 +1,3 @@
+pub record Area { x: Int }
+pub fn area() -> Int { 1 }
+fn hidden() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__exported_names_each_namespace_a_public_name_has/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__exported_names_each_namespace_a_public_name_has/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f328e4d9d04c31d0d70d16d21a07d1613be9d577
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__exported_names_each_namespace_a_public_name_has/main.nova
@@ -0,0 +1 @@
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__extern_fn_resolves_as_callable_extern_def.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__extern_fn_resolves_as_callable_extern_def.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4cf09bfb3c407c317946023872c87b867a8400c7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__extern_fn_resolves_as_callable_extern_def.nova
@@ -0,0 +1,2 @@
+extern "C" { fn sqrt(x: Float) -> Float }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__import_binds_public_names_across_modules/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__import_binds_public_names_across_modules/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a4b895829eda4287a100e3a8cd516671ef8c339f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__import_binds_public_names_across_modules/lib.nova
@@ -0,0 +1,2 @@
+pub fn add(a: Int, b: Int) -> Int { a + b }
+fn secret() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__import_binds_public_names_across_modules/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__import_binds_public_names_across_modules/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..eed452983423bd5735ba412c66494d2b4a2b1dd3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__import_binds_public_names_across_modules/main.nova
@@ -0,0 +1,2 @@
+import lib::{add}
+fn main() { let x = add(1, 2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_private_item_is_rejected/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_private_item_is_rejected/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8c4a9a44c7446179c7600af9639cb6cd466edf25
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_private_item_is_rejected/lib.nova
@@ -0,0 +1,2 @@
+pub fn add() -> Int { 0 }
+fn secret() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_private_item_is_rejected/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_private_item_is_rejected/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..23353b68b7d3865397722bf379da4954cebdcb2e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_private_item_is_rejected/main.nova
@@ -0,0 +1,2 @@
+import lib::{secret}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_std_core_name_from_a_module_is_allowed/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_std_core_name_from_a_module_is_allowed/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c298ac14021a8c27106157e147d149d88b2ca579
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_std_core_name_from_a_module_is_allowed/lib.nova
@@ -0,0 +1 @@
+pub type Status = | Ok | Fail
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_std_core_name_from_a_module_is_allowed/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_std_core_name_from_a_module_is_allowed/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..34ecb4c1d90b32812bb8ec8dcd18b7cf5733727c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__importing_a_std_core_name_from_a_module_is_allowed/main.nova
@@ -0,0 +1,2 @@
+import lib
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__name_imports_makes_every_other_module_importable/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__name_imports_makes_every_other_module_importable/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a92bed8bc0325a2d33410e9279c331fade798043
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__name_imports_makes_every_other_module_importable/lib.nova
@@ -0,0 +1 @@
+pub fn area() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__name_imports_makes_every_other_module_importable/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__name_imports_makes_every_other_module_importable/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f328e4d9d04c31d0d70d16d21a07d1613be9d577
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__name_imports_makes_every_other_module_importable/main.nova
@@ -0,0 +1 @@
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__names_in_scope_lists_items_imports_builtins_variants_and_std/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__names_in_scope_lists_items_imports_builtins_variants_and_std/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a92bed8bc0325a2d33410e9279c331fade798043
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__names_in_scope_lists_items_imports_builtins_variants_and_std/lib.nova
@@ -0,0 +1 @@
+pub fn area() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__names_in_scope_lists_items_imports_builtins_variants_and_std/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__names_in_scope_lists_items_imports_builtins_variants_and_std/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1dd88fbeeaa6acc24f3d71c35f3768d46a879a44
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__names_in_scope_lists_items_imports_builtins_variants_and_std/main.nova
@@ -0,0 +1,3 @@
+import lib::{area}
+record Point { x: Int }
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__no_std_only_builtin_is_a_reserved_word.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__no_std_only_builtin_is_a_reserved_word.nova
new file mode 100644
index 0000000000000000000000000000000000000000..655cf33030edf38334d7aa241abc117c36407db1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__no_std_only_builtin_is_a_reserved_word.nova
@@ -0,0 +1,101 @@
+fn str_cmp(a: Int, b: Int) -> Int { a + b }
+fn main() { }
+fn str_hash(a: Int, b: Int) -> Int { a + b }
+fn char_to_int(a: Int, b: Int) -> Int { a + b }
+fn str_len_chars(a: Int, b: Int) -> Int { a + b }
+fn str_chars(a: Int, b: Int) -> Int { a + b }
+fn str_from_chars(a: Int, b: Int) -> Int { a + b }
+fn str_join(a: Int, b: Int) -> Int { a + b }
+fn str_index_of(a: Int, b: Int) -> Int { a + b }
+fn json_quote(a: Int, b: Int) -> Int { a + b }
+fn str_to_upper(a: Int, b: Int) -> Int { a + b }
+fn str_to_lower(a: Int, b: Int) -> Int { a + b }
+fn float_fixed(a: Int, b: Int) -> Int { a + b }
+fn str_to_float(a: Int, b: Int) -> Int { a + b }
+fn str_is_float(a: Int, b: Int) -> Int { a + b }
+fn float_to_bits(a: Int, b: Int) -> Int { a + b }
+fn int_to_char(a: Int, b: Int) -> Int { a + b }
+fn str_as_bytes(a: Int, b: Int) -> Int { a + b }
+fn test_selector(a: Int, b: Int) -> Int { a + b }
+fn task_spawn(a: Int, b: Int) -> Int { a + b }
+fn task_is_done(a: Int, b: Int) -> Int { a + b }
+fn task_release(a: Int, b: Int) -> Int { a + b }
+fn task_drive(a: Int, b: Int) -> Int { a + b }
+fn task_output(a: Int, b: Int) -> Int { a + b }
+fn task_yield_future(a: Int, b: Int) -> Int { a + b }
+fn task_sleep_future_nanos(a: Int, b: Int) -> Int { a + b }
+fn task_join_future(a: Int, b: Int) -> Int { a + b }
+fn task_timeout_future(a: Int, b: Int) -> Int { a + b }
+fn fs_read_to_string(a: Int, b: Int) -> Int { a + b }
+fn fs_write_string(a: Int, b: Int) -> Int { a + b }
+fn fs_take_string(a: Int, b: Int) -> Int { a + b }
+fn fs_last_error_message(a: Int, b: Int) -> Int { a + b }
+fn fs_temp_dir(a: Int, b: Int) -> Int { a + b }
+fn fs_exists(a: Int, b: Int) -> Int { a + b }
+fn fs_create_dir(a: Int, b: Int) -> Int { a + b }
+fn fs_create_dir_all(a: Int, b: Int) -> Int { a + b }
+fn fs_remove_file(a: Int, b: Int) -> Int { a + b }
+fn fs_remove_dir_all(a: Int, b: Int) -> Int { a + b }
+fn fs_rename(a: Int, b: Int) -> Int { a + b }
+fn fs_read_dir(a: Int, b: Int) -> Int { a + b }
+fn fs_take_string_array(a: Int, b: Int) -> Int { a + b }
+fn fs_kind(a: Int, b: Int) -> Int { a + b }
+fn fs_read(a: Int, b: Int) -> Int { a + b }
+fn fs_take_bytes(a: Int, b: Int) -> Int { a + b }
+fn fs_write(a: Int, b: Int) -> Int { a + b }
+fn file_open(a: Int, b: Int) -> Int { a + b }
+fn file_close(a: Int, b: Int) -> Int { a + b }
+fn file_read(a: Int, b: Int) -> Int { a + b }
+fn file_write(a: Int, b: Int) -> Int { a + b }
+fn file_flush(a: Int, b: Int) -> Int { a + b }
+fn io_stdin_read(a: Int, b: Int) -> Int { a + b }
+fn io_stdout_write(a: Int, b: Int) -> Int { a + b }
+fn io_stderr_write(a: Int, b: Int) -> Int { a + b }
+fn io_stdout_flush(a: Int, b: Int) -> Int { a + b }
+fn io_stderr_flush(a: Int, b: Int) -> Int { a + b }
+fn net_connect(a: Int, b: Int) -> Int { a + b }
+fn net_close(a: Int, b: Int) -> Int { a + b }
+fn net_read(a: Int, b: Int) -> Int { a + b }
+fn net_write(a: Int, b: Int) -> Int { a + b }
+fn net_read_timeout(a: Int, b: Int) -> Int { a + b }
+fn net_listen(a: Int, b: Int) -> Int { a + b }
+fn net_local_port(a: Int, b: Int) -> Int { a + b }
+fn net_accept(a: Int, b: Int) -> Int { a + b }
+fn bytes_len(a: Int, b: Int) -> Int { a + b }
+fn bytes_from_string_intrinsic(a: Int, b: Int) -> Int { a + b }
+fn bytes_is_utf8(a: Int, b: Int) -> Int { a + b }
+fn bytes_to_string_unchecked(a: Int, b: Int) -> Int { a + b }
+fn bytes_at(a: Int, b: Int) -> Int { a + b }
+fn bytes_slice(a: Int, b: Int) -> Int { a + b }
+fn bytes_concat(a: Int, b: Int) -> Int { a + b }
+fn bytes_to_ints(a: Int, b: Int) -> Int { a + b }
+fn bytes_from_ints_intrinsic(a: Int, b: Int) -> Int { a + b }
+fn bytes_eq(a: Int, b: Int) -> Int { a + b }
+fn int_hash_seed(a: Int, b: Int) -> Int { a + b }
+fn http_parse_request(a: Int, b: Int) -> Int { a + b }
+fn crypto_hash(a: Int, b: Int) -> Int { a + b }
+fn crypto_random_bytes(a: Int, b: Int) -> Int { a + b }
+fn crypto_random_int(a: Int, b: Int) -> Int { a + b }
+fn crypto_pbkdf2_sha256(a: Int, b: Int) -> Int { a + b }
+fn crypto_pbkdf2_sha256_verify(a: Int, b: Int) -> Int { a + b }
+fn time_now_nanos(a: Int, b: Int) -> Int { a + b }
+fn time_now_epoch_nanos(a: Int, b: Int) -> Int { a + b }
+fn log_config_level(a: Int, b: Int) -> Int { a + b }
+fn log_config_to_stderr(a: Int, b: Int) -> Int { a + b }
+fn log_set_config(a: Int, b: Int) -> Int { a + b }
+fn shutdown_requested(a: Int, b: Int) -> Int { a + b }
+fn process_arg_count(a: Int, b: Int) -> Int { a + b }
+fn process_arg(a: Int, b: Int) -> Int { a + b }
+fn process_exit(a: Int, b: Int) -> Int { a + b }
+fn process_run(a: Int, b: Int) -> Int { a + b }
+fn process_take_status(a: Int, b: Int) -> Int { a + b }
+fn process_take_stdout(a: Int, b: Int) -> Int { a + b }
+fn process_take_stderr(a: Int, b: Int) -> Int { a + b }
+fn process_env(a: Int, b: Int) -> Int { a + b }
+fn process_current_exe(a: Int, b: Int) -> Int { a + b }
+fn ui_state_present(a: Int, b: Int) -> Int { a + b }
+fn ui_state_load(a: Int, b: Int) -> Int { a + b }
+fn ui_state_store(a: Int, b: Int) -> Int { a + b }
+fn router_state_present(a: Int, b: Int) -> Int { a + b }
+fn router_state_load(a: Int, b: Int) -> Int { a + b }
+fn router_state_store(a: Int, b: Int) -> Int { a + b }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__private_item_is_not_visible_to_other_modules/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__private_item_is_not_visible_to_other_modules/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8c4a9a44c7446179c7600af9639cb6cd466edf25
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__private_item_is_not_visible_to_other_modules/lib.nova
@@ -0,0 +1,2 @@
+pub fn add() -> Int { 0 }
+fn secret() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__private_item_is_not_visible_to_other_modules/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__private_item_is_not_visible_to_other_modules/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..34ecb4c1d90b32812bb8ec8dcd18b7cf5733727c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__private_item_is_not_visible_to_other_modules/main.nova
@@ -0,0 +1,2 @@
+import lib
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__qualified_import_path_is_rejected/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__qualified_import_path_is_rejected/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2e294688062c89d3796292c2a472dbc9231bf683
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__qualified_import_path_is_rejected/lib.nova
@@ -0,0 +1 @@
+pub fn add() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__qualified_import_path_is_rejected/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__qualified_import_path_is_rejected/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dce7a6db02a6ce0a953105122f81dbd7d5db6b1c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__qualified_import_path_is_rejected/main.nova
@@ -0,0 +1,2 @@
+import lib::extra
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__same_name_in_two_modules_does_not_collide/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__same_name_in_two_modules_does_not_collide/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..23d881782af8188a35d5454acbba417b581878c1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__same_name_in_two_modules_does_not_collide/lib.nova
@@ -0,0 +1 @@
+pub fn helper() -> Int { 2 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__same_name_in_two_modules_does_not_collide/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__same_name_in_two_modules_does_not_collide/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..734b793e42573da969cd41707a7cdeb9b8d6c9f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__same_name_in_two_modules_does_not_collide/main.nova
@@ -0,0 +1,2 @@
+fn helper() -> Int { 1 }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__shadowing_builtin_reports_e0002.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__shadowing_builtin_reports_e0002.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dd62c6ec23282f4a5da875b350c02e1027e1df15
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__shadowing_builtin_reports_e0002.nova
@@ -0,0 +1 @@
+fn println() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__ssr_is_imported_by_name_and_only_natively_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__ssr_is_imported_by_name_and_only_natively_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bfbc8b9aa4dad75455b526014cc81b55dd621f4e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__ssr_is_imported_by_name_and_only_natively_1.nova
@@ -0,0 +1,2 @@
+import ui::ssr::{page}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__ssr_is_imported_by_name_and_only_natively_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__ssr_is_imported_by_name_and_only_natively_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..beabd0fdc502bd14ed62da93c516a23b262fa6c5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__ssr_is_imported_by_name_and_only_natively_2.nova
@@ -0,0 +1,2 @@
+import ui::ssr
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__std_collections_module_is_compiled_and_visible.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__std_collections_module_is_compiled_and_visible.nova
new file mode 100644
index 0000000000000000000000000000000000000000..af96210eac5bf41bc001ccbbebf8e4327753fffb
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__std_collections_module_is_compiled_and_visible.nova
@@ -0,0 +1 @@
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__std_itself_is_never_reported.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__std_itself_is_never_reported.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f328e4d9d04c31d0d70d16d21a07d1613be9d577
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__std_itself_is_never_reported.nova
@@ -0,0 +1 @@
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__ui_is_seen_only_through_an_import.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__ui_is_seen_only_through_an_import.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c8df1ae026087e2eba13c4dbb9fb7068dd6fb852
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__ui_is_seen_only_through_an_import.nova
@@ -0,0 +1,2 @@
+import ui::{signal}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__user_fn_named_panic_is_a_reserved_word.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__user_fn_named_panic_is_a_reserved_word.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bfe5cef9af96c98c537b3a612d20daef273d9592
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__user_fn_named_panic_is_a_reserved_word.nova
@@ -0,0 +1,2 @@
+fn panic(msg: String) { }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__user_trait_shadows_std_core.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__user_trait_shadows_std_core.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b3b6c9d2583c7e7975f47be3332b4aee9e6cb4af
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__user_trait_shadows_std_core.nova
@@ -0,0 +1,2 @@
+trait Display { fn only_here(self) -> Int }
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver__user_type_shadows_std_core.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__user_type_shadows_std_core.nova
new file mode 100644
index 0000000000000000000000000000000000000000..90a71d3458f81047365e00cece4b9b3fb83b7be9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver__user_type_shadows_std_core.nova
@@ -0,0 +1,2 @@
+type Option<T> = | Present(T) | Absent
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5f5375c1147a617cad80a5c638a46e700a4e8cb5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/lib.nova
@@ -0,0 +1,5 @@
+import lib
+pub fn alpha() -> Int { 1 }
+fn secret() -> Int { 2 }
+pub record P {}
+pub trait T {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a6a094393b9189099f7cea2e1c3344d360f9884c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/main.nova
@@ -0,0 +1,40 @@
+import lib
+import lib::{secret}
+import nosuch
+import dom
+import lib as l
+import a::b
+type Name = String
+record Int { x: Int }
+@tset
+fn main() {
+    g
+    (1)
+    g
+    [1]
+    g
+    -1
+}
+@test
+record R {}
+@bogus
+pub const K: Int = 1
+pub type Shade = | Red | Ok(Int)
+trait Show {
+    type Item
+    fn twice(self) {}
+}
+impl Show for R {
+    fn twice(self) {}
+}
+extern "C" {
+    fn abs(x: Int) -> Int
+}
+fn println() {}
+fn dup() {}
+fn dup() {}
+fn alpha() -> Int { 0 }
+fn back() -> Int {
+    return
+    1
+}
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-cli/tests/selfhost/fuzz.rs b/crates/nova-cli/tests/selfhost/fuzz.rs
index e4883cf8ad7bcec7531ad727643a7d30a36f4b20..79c68ea7a1e6b047f7dbe1b37c059fbb07b83017 100644
--- a/crates/nova-cli/tests/selfhost/fuzz.rs
+++ b/crates/nova-cli/tests/selfhost/fuzz.rs
@@ -20,6 +20,12 @@
 //!    3, §4.4; ADR 0052): well typed up to the limit, so that `nova check`
 //!    goes through every stage on it, and P0002 past it.
 //!
+//! 4. [`name_mutate`] (5.2a, spec
+//!    `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+//!    §3.5): a loose corpus program that parses, mutated at the level of
+//!    names, beside four sibling modules ([`NAME_SIBLINGS`]), so that the
+//!    resolver's codes occur among the fixed seeds' inputs.
+//!
 //! Beside them, [`edges`]: fixed inputs that no file under `cases/` can
 //! hold as it is, such as a lone `\r` or a NUL.
 
@@ -574,3 +580,282 @@ pub fn deeps(seed: u64, mixed: usize) -> Vec<(String, usize, String)> {
     }
     out
 }
+
+// --- 5.2a: the name-level fuzzer (spec
+// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+// §3.5; ADR 0053) ---
+
+/// The modules every name-mutated input sits beside, `(stem, source)`, so
+/// that an import of a sibling finds one: `shapes` exports one item of
+/// each kind and keeps one private, and `helpers` exports a `main`, which
+/// clashes with a program's own when imported whole, and a variant that
+/// hides std's `Some` (E0102). `vault` keeps one private item of each kind
+/// that a fix can make public, and `hidden` twice, the second with its
+/// keyword on a line of its own, so no fix is offered for it; `keeper`
+/// imports `vault` whole and declares a record named as `vault`'s private
+/// function `Kept`, so the fix that would make `Kept` public is dropped.
+pub const NAME_SIBLINGS: [(&str, &str); 4] = [
+    (
+        "shapes",
+        "pub fn area() -> Int { 1 }\nfn secret() -> Int { 2 }\npub record Box { w: Int }\n\
+         pub type Tint = | Light | Dark\npub trait Shade {}\npub const LIMIT: Int = 3\n",
+    ),
+    (
+        "helpers",
+        "pub fn main() {}\npub fn helper() -> Int { 0 }\npub type Signal = | Some(Int) | Quiet\n",
+    ),
+    (
+        "vault",
+        "fn hidden() -> Int { 1 }\nconst\n    hidden: Int = 2\nconst SEALED: Int = 3\n\
+         record Cell { n: Int }\ntype Hue = | Pale\ntrait Veiled {}\nfn Kept() -> Int { 4 }\n",
+    ),
+    ("keeper", "import vault\nrecord Kept { n: Int }\n"),
+];
+
+/// A name a renamed definition can take, to collide with a builtin.
+const BUILTIN_NAMES: [&str; 5] = ["println", "print", "eprint", "eprintln", "panic"];
+
+/// Names std brings into every module: types, variants, traits and
+/// functions, which a program's own definition shadows, or, for a
+/// variant, hides with E0102.
+const STD_NAMES: [&str; 11] = [
+    "Option",
+    "Vec",
+    "Map",
+    "Some",
+    "None",
+    "Ok",
+    "Err",
+    "Display",
+    "Iterator",
+    "read_to_string",
+    "sleep",
+];
+
+/// The built-in type names (`nova_resolver::RESERVED_TYPE_NAMES`): a
+/// record or a sum under one is E0089.
+const RESERVED_NAMES: [&str; 7] = ["Int", "Float", "Bool", "Char", "String", "Future", "Bytes"];
+
+/// Names only std's modules see (`Builtin::STD_ONLY`), which a program may
+/// define freely.
+const STD_ONLY_NAMES: [&str; 5] = [
+    "str_cmp",
+    "str_hash",
+    "char_to_int",
+    "fs_read",
+    "task_spawn",
+];
+
+/// The imports a mutation adds at the top of a file: a module that is not
+/// there, a sibling whole or by a list (a private name, a missing one),
+/// std's imported modules (`dom` is not native's, E0092), a nested path
+/// under one and elsewhere, an alias (E0900), `{self}`, the input's own
+/// stem, a self-import, and `vault`'s private items, with `keeper`, which
+/// a fix for `Kept` would bind twice.
+const NAME_IMPORTS: [&str; 18] = [
+    "import nosuch",
+    "import shapes",
+    "import shapes::{secret}",
+    "import shapes::{area, nothing}",
+    "import shapes::{Box, Shade, LIMIT, Tint, Light}",
+    "import helpers",
+    "import helpers::{main}",
+    "import dom",
+    "import ui",
+    "import ui::nope",
+    "import ui::html::{div}",
+    "import a::b",
+    "import shapes as s",
+    "import {self}",
+    "import helpers::{Signal, Some}",
+    "import vault::{hidden}",
+    "import vault::{SEALED, Cell, Hue, Veiled}",
+    "import vault::{Kept}\nimport keeper",
+];
+
+/// The attributes a mutation puts on an item: unknown ones (E0082, with a
+/// fix on a function when the name is close to `test`), `@test` (E0083 on
+/// anything but a function; a function with one is stripped under
+/// `check`), and malformed `@test`s.
+const NAME_ATTRIBUTES: [&str; 6] = [
+    "@tset",
+    "@inline",
+    "@test",
+    "@test(bogus)",
+    "@test(should_panic, nope)",
+    "@deprecated(soon)",
+];
+
+/// The items a mutation adds at the end of a file: a type alias (E0900), a
+/// sum whose variants hide std's (E0102), a trait twice (E0002), an
+/// `extern` block, a function under a std-only name, a record under a
+/// built-in type's name (E0089), a second `main`, a `@test` record and
+/// impl block (E0083), a trait with a misspelt one (E0082), and impls for
+/// types that name their methods `anon`.
+const NAME_ITEMS: [&str; 11] = [
+    "type Alias = Int\n",
+    "type Mine = | Some(Int) | Nope | Err\n",
+    "trait Twice {}\ntrait Twice {}\n",
+    "extern \"C\" {\n    fn abs(x: Int) -> Int\n}\n",
+    "fn str_cmp(a: Int) -> Int { a }\n",
+    "record Bytes { n: Int }\n",
+    "pub fn main() {}\n",
+    "@test\nrecord Probe { n: Int }\n",
+    "@tset\ntrait Probing {}\n",
+    "record Held { n: Int }\n@test\nimpl Held {}\n",
+    "trait Marked {}\nimpl Marked for () { fn mark(self) {} }\nimpl Marked for fn(Int) -> Int { fn mark(self) {} }\n",
+];
+
+/// One top-level item, as the name fuzzer edits it: its span, and its
+/// name's span, its name and whether it is `pub` when it has one, with
+/// whether it introduces a type name (a record or a sum) and whether it is
+/// a function.
+struct NamedItem {
+    span: (usize, usize),
+    name: Option<(usize, usize, String)>,
+    is_pub: bool,
+    is_type: bool,
+    is_fn: bool,
+}
+
+/// `source`'s items, if it lexes and parses without an error.
+fn named_items(source: &str) -> Option<Vec<NamedItem>> {
+    use nova_ast::item::Visibility;
+    use nova_ast::Item;
+    let (tokens, lex_errors) = nova_lexer::lex(source, FileId::DUMMY);
+    if !lex_errors.is_empty() {
+        return None;
+    }
+    let parsed = nova_parser::parse_recovering(&tokens, FileId::DUMMY);
+    if !parsed.errors.is_empty() {
+        return None;
+    }
+    let items = parsed
+        .file
+        .items
+        .iter()
+        .map(|item| {
+            let (name, vis, is_type) = match &item.value {
+                Item::Function(f) => (Some(&f.name), Some(f.vis), false),
+                Item::Record(r) => (Some(&r.name), Some(r.vis), true),
+                Item::Type(t) => (Some(&t.name), Some(t.vis), true),
+                Item::Trait(t) => (Some(&t.name), Some(t.vis), false),
+                Item::Const(c) => (Some(&c.name), Some(c.vis), false),
+                _ => (None, None, false),
+            };
+            NamedItem {
+                span: (item.span.start as usize, item.span.end as usize),
+                name: name.map(|n| (n.span.start as usize, n.span.end as usize, n.value.clone())),
+                is_pub: vis == Some(Visibility::Pub),
+                is_type,
+                is_fn: matches!(item.value, Item::Function(_)),
+            }
+        })
+        .collect();
+    Some(items)
+}
+
+/// Where the line holding byte `at` starts.
+fn line_start(source: &str, at: usize) -> usize {
+    source[..at].rfind('\n').map_or(0, |i| i + 1)
+}
+
+/// `source`, a program that parses, mutated one to three times at the
+/// level of names: a definition renamed to collide with another, with a
+/// builtin, with a std name, a built-in type's or a std-only builtin's; an
+/// item duplicated or deleted; `pub` toggled; an import, an attribute or
+/// an item from the tables above added. `stem` is the input's own file
+/// stem, which a self-import names. A mutation that would leave the
+/// program unparsable is skipped, so every input reaches the resolver.
+pub fn name_mutate(rng: &mut SplitMix64, source: &str, stem: &str) -> String {
+    let mut text = source.to_string();
+    for _ in 0..1 + rng.below(3) {
+        let Some(items) = named_items(&text) else {
+            break;
+        };
+        let named: Vec<&NamedItem> = items.iter().filter(|i| i.name.is_some()).collect();
+        let edited = match rng.below(28) {
+            // Renames: 0-2 to another item's name, 3-4 a builtin's, 5-6 a
+            // std name, 7-8 a built-in type's (on a type), 9 a std-only
+            // builtin's.
+            k @ 0..=9 if !named.is_empty() => {
+                let types: Vec<&NamedItem> = named.iter().copied().filter(|i| i.is_type).collect();
+                let item = if matches!(k, 7 | 8) && !types.is_empty() {
+                    *rng.pick(&types)
+                } else {
+                    *rng.pick(&named)
+                };
+                let (start, end, _) = item.name.clone().expect("a named item");
+                let to = match k {
+                    0..=2 => rng.pick(&named).name.clone().expect("a named item").2,
+                    3 | 4 => rng.pick(&BUILTIN_NAMES).to_string(),
+                    5 | 6 => rng.pick(&STD_NAMES).to_string(),
+                    7 | 8 => rng.pick(&RESERVED_NAMES).to_string(),
+                    _ => rng.pick(&STD_ONLY_NAMES).to_string(),
+                };
+                Some(format!("{}{to}{}", &text[..start], &text[end..]))
+            }
+            // An item duplicated at the end of the file, or deleted.
+            10..=12 if !items.is_empty() => {
+                let item = rng.pick(&items);
+                let copy = &text[item.span.0..item.span.1];
+                Some(format!("{}\n{copy}\n", text.trim_end_matches('\n')))
+            }
+            13 | 14 if !items.is_empty() => {
+                let item = rng.pick(&items);
+                Some(format!("{}{}", &text[..item.span.0], &text[item.span.1..]))
+            }
+            // `pub` toggled.
+            15 | 16 if !named.is_empty() => {
+                let item = rng.pick(&named);
+                let (start, _, _) = item.name.clone().expect("a named item");
+                let keyword = nova_diagnostics::lines::keyword_start(&text, start);
+                match (item.is_pub, keyword) {
+                    (false, Some(at)) => Some(format!("{}pub {}", &text[..at], &text[at..])),
+                    (true, Some(at)) => text[..at]
+                        .rfind("pub ")
+                        .filter(|&p| p >= item.span.0)
+                        .map(|p| format!("{}{}", &text[..p], &text[p + 4..])),
+                    _ => None,
+                }
+            }
+            // An import at the top.
+            17..=21 => {
+                let import = rng.pick(&NAME_IMPORTS).replace("{self}", stem);
+                Some(format!("{import}\n{text}"))
+            }
+            // An attribute on an item's line: a `@test` on an item that is
+            // not a function half the time, since one on a function is
+            // stripped.
+            22..=24 if !named.is_empty() => {
+                let attr = rng.pick(&NAME_ATTRIBUTES);
+                let others: Vec<&NamedItem> = named.iter().copied().filter(|i| !i.is_fn).collect();
+                let item = if attr.starts_with("@test") && !others.is_empty() && rng.one_in(2) {
+                    *rng.pick(&others)
+                } else {
+                    *rng.pick(&named)
+                };
+                let (start, _, _) = item.name.clone().expect("a named item");
+                let at = line_start(&text, start);
+                Some(format!("{}{attr}\n{}", &text[..at], &text[at..]))
+            }
+            // An item at the end.
+            _ => {
+                let item = rng.pick(&NAME_ITEMS);
+                Some(format!("{}\n{item}", text.trim_end_matches('\n')))
+            }
+        };
+        if let Some(next) = edited {
+            if named_items(&next).is_some() {
+                text = next;
+            }
+        }
+    }
+    text
+}
+
+/// Whether `source` lexes and parses without an error: what the name
+/// fuzzer starts from.
+pub fn parses(source: &str) -> bool {
+    named_items(source).is_some()
+}
````

### Task 4: N's diagnostics, builtin tables and loader

Spec §3.2, §3.3; decisions 2, 17, 19, 27, 28 and 30.

**Files:**
- Create: `selfhost/src/diagnostic.nova`, `selfhost/src/builtins.nova`,
  `selfhost/src/loader.nova`, `selfhost/src/loader_tests.nova`
- Modify: `selfhost/src/main.nova` (imports `loader_tests`)

- [ ] **Step 1: The tests.** `patch_block t4-tests`: `loader_tests.nova`'s
  three `@test`s, `a_loose_program_loads_breadth_first_from_its_root`
  (the modules' order, an import's target, E0093 for a `ui.nova`, the
  importable lists), `check_strips_test_functions_and_nothing_else` and
  `paths_split_and_join_as_rusts_do` (`module_name`, `path_parent` with
  `.` components and repeated `/`s, `path_join`, `sorted_dedup`); and
  `main.nova`'s import of the module.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`.
  - `SH the_selfhost_tests_pass`. Expected: `0 passed; 1 failed`: `nova test` cannot compile the package, `cannot find module `diagnostic`` and `` `loader` ``.
  - `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 3`, stderr
    too, where a compile error goes. Expected: the same two errors, then `Error: could not compile due to 2 previous errors`.
- [ ] **Step 3: The code.** `patch_block t4-code`:
  - **`diagnostic.nova`:** `Diagnostic`, `Label`, `Fix`, `Edit`,
    `error_at`, and `suggest.rs`'s `distance`, `closest`, `change_to` and
    `lines.rs`'s `keyword_start`;
  - **`builtins.nova`:** the 118 builtins' variant names and names,
    `GLOBAL`, `STD_ONLY`, `WASM_ONLY`, `BuiltinTable`, and the std tables;
  - **`loader.nova`:** `ImportTarget` (decision 28), `load_program`,
    `Loader`'s `find`, `find_place` and `has`, `strip_test_functions`,
    `in_package`, and the paths (decision 27).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH the_selfhost_tests_pass`. Expected: `1 passed; 0 failed`.
  - `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 3`.
    Expected: `test result: ok. 20 passed; 0 failed; 0 trapped; 20 total`.
  - `SH`. Expected: `31 passed; 0 failed`.
  - `git status --short`. Expected: nothing.
  - `S` (4 min 40 s). Expected: `2852 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `selfhost: N's diagnostics, builtin tables and loader`.

**Patch `t4-tests`:**

<!-- patch t4-tests -->

````diff
diff --git a/selfhost/src/loader_tests.nova b/selfhost/src/loader_tests.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f83ae74e8e1ad379587ac73dec2d662bb438940e
--- /dev/null
+++ b/selfhost/src/loader_tests.nova
@@ -0,0 +1,143 @@
+// N's own tests of the loader's edges: a loose program loaded breadth
+// first from its root, a missing sibling left to the resolver, a sibling
+// under a std module's name refused with E0093, what each module could
+// import, the `@test` functions `check` strips, and the paths and the sort
+// the loader leans on. Each expected value is what S0's driver gives for
+// the same files; the differential compares the corpus and the cases.
+//
+// They live in `src/`, imported by `main.nova`, for the reason
+// lexer_tests.nova gives.
+
+import ast
+import diagnostic
+import lexer
+import loader
+import parser
+import token
+
+// The texts, one to a line, to compare.
+fn lines(texts: [String]) -> String { "\n".join(texts) }
+
+@test
+fn a_loose_program_loads_breadth_first_from_its_root() {
+    // main imports b, then a, then nosuch (no file: the resolver's), then
+    // ui (a file under a std module's name: E0093, and the import is
+    // reported); a imports c. Breadth first: main, b, a, c.
+    let dir = "${temp_dir()}/nova-selfhost-loader-test"
+    let _ = block_on(remove_dir_all(dir))
+    let _ = block_on(create_dir_all(dir))
+    let files = [
+        ("main.nova", "import b\nimport a\nimport nosuch\nimport ui\nfn main() {}\n"),
+        ("a.nova", "import c\npub fn fa() {}\n"),
+        ("b.nova", "pub fn fb() {}\n"),
+        ("c.nova", "pub fn fc() {}\n"),
+        ("ui.nova", "pub fn fu() {}\n"),
+    ]
+    for (name, text) in files.iter() { let _ = block_on(write_string("${dir}/${name}", text)) }
+    let load = match block_on(load_program("${dir}/main.nova")) {
+        Ok(l) => l
+        Err(_) => {
+            assert(false, "unreachable")
+            return
+        }
+    }
+    let mut names: Vec<String> = Vec::new()
+    for m in load.modules.iter() { names.push(m.name) }
+    assert_eq(lines(names.to_array()), lines(["main", "b", "a", "c"]))
+    let main = load.modules.at(0)
+    match main.imports.get("a") {
+        Some(ImportTargetModule(t)) => assert_eq(t, 2)
+        _ => assert(false, "unreachable")
+    }
+    match main.imports.get("ui") {
+        Some(ImportTargetReported) => ()
+        _ => assert(false, "unreachable")
+    }
+    assert(!main.imports.contains_key("nosuch"), "nosuch names nothing")
+    // What each could import: every other module, by stem, in byte order.
+    let mut importable: Vec<String> = Vec::new()
+    for (name, index) in main.importable.iter() { importable.push("${name} ${index}") }
+    assert_eq(lines(importable.to_array()), lines(["a 2", "b 1", "c 3"]))
+    assert_eq(load.diagnostics.len(), 1)
+    let d = load.diagnostics.at(0)
+    assert_eq(d.code, "E0093")
+    assert_eq(
+        d.message,
+        "`ui` is the name of a std module, so no module may take it: rename ${dir}/ui.nova",
+    )
+    assert_eq(d.labels.at(0).file, "main")
+    assert_eq(d.labels.at(0).span.start, 39)
+    assert_eq(d.labels.at(0).span.end, 41)
+    // A root that is not there is an error; a sibling that is not is not.
+    match block_on(load_program("${dir}/missing.nova")) {
+        Ok(_) => assert(false, "unreachable")
+        Err(e) => {
+            match e.kind {
+                NotFound => ()
+                _ => assert(false, "unreachable")
+            }
+        }
+    }
+    let _ = block_on(remove_dir_all(dir))
+}
+
+@test
+fn check_strips_test_functions_and_nothing_else() {
+    // `strip_test_functions`: a function with `@test` goes, whatever its
+    // arguments; `@test` on anything else stays, for the resolver's E0083.
+    let source = "@test\nfn t() {}\n@test(should_panic)\nfn u() {}\nfn main() {}\n@test\nrecord R {}\n"
+    let (tokens, _) = lex(source)
+    let (file, _) = parse(tokens)
+    let mut modules: Vec<Loaded> = Vec::new()
+    modules.push(
+        Loaded {
+            name: "main",
+            path: "main.nova",
+            ast: file,
+            text: source,
+            imports: Map::new(),
+            importable: Vec::new(),
+        },
+    )
+    strip_test_functions(modules)
+    let items = modules.at(0).ast.items
+    assert_eq(items.len(), 2)
+    match items.at(0).value {
+        ItemFunction(f) => assert_eq(f.name.value, "main")
+        _ => assert(false, "unreachable")
+    }
+}
+
+@test
+fn paths_split_and_join_as_rusts_do() {
+    // A module's name is its file's stem; `Path::parent` and `Path::join`
+    // on Linux and macOS.
+    assert_eq(module_name("tests/runtime/modules/main.nova"), "main")
+    assert_eq(module_name("a.b.nova"), "a.b")
+    assert_eq(module_name(".nova"), ".nova")
+    assert_eq(module_name("noext"), "noext")
+    assert_eq(path_parent("main.nova").unwrap_or("-"), "")
+    assert_eq(path_parent("a/b/c.nova").unwrap_or("-"), "a/b")
+    assert_eq(path_parent("/c.nova").unwrap_or("-"), "/")
+    assert_eq(path_parent("").unwrap_or("-"), "-")
+    // A `.` component or `/`s before the last part go, but not a leading
+    // `./` or root.
+    assert_eq(path_parent("sub/./main.nova").unwrap_or("-"), "sub")
+    assert_eq(path_parent("./sub/.//main.nova").unwrap_or("-"), "./sub")
+    assert_eq(path_parent("./main.nova").unwrap_or("-"), ".")
+    assert_eq(path_parent("a//b//c.nova").unwrap_or("-"), "a//b")
+    assert_eq(path_parent("a/b/.//").unwrap_or("-"), "a")
+    assert_eq(path_parent("/./a").unwrap_or("-"), "/")
+    assert_eq(path_parent("a/.").unwrap_or("-"), "")
+    assert_eq(path_parent("./.").unwrap_or("-"), "")
+    assert_eq(path_parent("/.").unwrap_or("-"), "-")
+    assert_eq(path_parent("//").unwrap_or("-"), "-")
+    assert_eq(module_name("a/."), "a")
+    assert_eq(module_name("a/.."), "main")
+    assert_eq(path_join("", "x.nova"), "x.nova")
+    assert_eq(path_join("a/", "x.nova"), "a/x.nova")
+    assert_eq(path_join("a", "x.nova"), "a/x.nova")
+    // Names sort in byte order, each once.
+    let sorted = sorted_dedup(Vec::from_array(["b", "B", "a", "ab", "b", "_"]))
+    assert_eq(lines(sorted.to_array()), lines(["B", "_", "a", "ab", "b"]))
+}
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index d8c53adccfb9dec0f9ef0a39e745329a64a76901..bd10996c4cee33c74b687fdbb444b784c7f5f7fd 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -17,6 +17,7 @@ import ast_dump
 import dump
 import lexer
 import lexer_tests
+import loader_tests
 import parser_tests
 import token
 
````

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/selfhost/src/builtins.nova b/selfhost/src/builtins.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c16231570d649d9a905e355a24beacd5f86725b0
--- /dev/null
+++ b/selfhost/src/builtins.nova
@@ -0,0 +1,537 @@
+// The resolver's tables, word for word: a port of
+// crates/nova-resolver/src/lib.rs's builtin table (`Builtin`, its `name`,
+// `GLOBAL`, `STD_ONLY` and `WASM_ONLY`), `RESERVED_TYPE_NAMES`, and its std
+// tables (`STD_MODULES`, `WASM_STD_MODULES`, `IMPORTED_STD`,
+// `RESERVED_MODULE_NAMES`, `STD_INTERNAL`, `STD_TEST_MODULE`) (spec
+// docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
+// §3.3, ADR 0053).
+//
+// - A builtin is its index in `BUILTIN_VARIANTS`, the `builtins!` macro's
+//   declaration order, which is `Builtin::ALL`'s: S0's 118-variant enum is
+//   a table of Rust variant names, which the resolution dump prints, and
+//   one of the names a program writes (`Builtin::name`). `GLOBAL`,
+//   `STD_ONLY` and `WASM_ONLY` list variant names, as the Rust lists
+//   variants; `builtin_table` reads them once, as indices.
+// - A std module is its name and where its source is under the directory
+//   `--std` names, where S0 embeds the source itself (`nova_std::CORE` is
+//   `std/core/lib.nova`, std/lib.rs).
+
+/// Every builtin's Rust variant name, in declaration order (`Builtin::ALL`).
+pub const BUILTIN_VARIANTS: [String] = [
+    "Println",
+    "Print",
+    "EPrint",
+    "EPrintln",
+    "Panic",
+    "StrCmp",
+    "StrHash",
+    "CharToInt",
+    "StrLenChars",
+    "StrChars",
+    "StrFromChars",
+    "StrJoin",
+    "StrIndexOf",
+    "JsonQuote",
+    "StrToUpper",
+    "StrToLower",
+    "FloatFixed",
+    "StrToFloat",
+    "StrIsFloat",
+    "FloatToBits",
+    "IntToChar",
+    "StrAsBytes",
+    "TestSelector",
+    "TaskSpawn",
+    "TaskIsDone",
+    "TaskRelease",
+    "TaskDrive",
+    "TaskOutput",
+    "TaskYieldFuture",
+    "TaskSleepFutureNanos",
+    "TaskJoinFuture",
+    "TaskTimeoutFuture",
+    "FsReadToString",
+    "FsWriteString",
+    "FsTakeString",
+    "FsLastErrorMessage",
+    "FsTempDir",
+    "FsExists",
+    "FsCreateDir",
+    "FsCreateDirAll",
+    "FsRemoveFile",
+    "FsRemoveDirAll",
+    "FsRename",
+    "FsReadDir",
+    "FsTakeStringArray",
+    "FsKind",
+    "FsRead",
+    "FsTakeBytes",
+    "FsWrite",
+    "FileOpen",
+    "FileClose",
+    "FileRead",
+    "FileWrite",
+    "FileFlush",
+    "IoStdinRead",
+    "IoStdoutWrite",
+    "IoStderrWrite",
+    "IoStdoutFlush",
+    "IoStderrFlush",
+    "NetConnect",
+    "NetClose",
+    "NetRead",
+    "NetWrite",
+    "NetReadTimeout",
+    "NetListen",
+    "NetLocalPort",
+    "NetAccept",
+    "BytesLen",
+    "BytesFromString",
+    "BytesIsUtf8",
+    "BytesToStringUnchecked",
+    "BytesAt",
+    "BytesSlice",
+    "BytesConcat",
+    "BytesToInts",
+    "BytesFromInts",
+    "BytesEq",
+    "IntHashSeed",
+    "HttpParseRequest",
+    "CryptoHash",
+    "CryptoRandomBytes",
+    "CryptoRandomInt",
+    "CryptoPbkdf2Sha256",
+    "CryptoPbkdf2Sha256Verify",
+    "TimeNowNanos",
+    "TimeNowEpochNanos",
+    "LogConfigLevel",
+    "LogConfigToStderr",
+    "LogSetConfig",
+    "ShutdownRequested",
+    "ProcessArgCount",
+    "ProcessArg",
+    "ProcessExit",
+    "ProcessRun",
+    "ProcessTakeStatus",
+    "ProcessTakeStdout",
+    "ProcessTakeStderr",
+    "ProcessEnv",
+    "ProcessCurrentExe",
+    "UiStatePresent",
+    "UiStateLoad",
+    "UiStateStore",
+    "RouterStatePresent",
+    "RouterStateLoad",
+    "RouterStateStore",
+    "MemLoad8",
+    "MemStore8",
+    "MemLoad64",
+    "MemStore64",
+    "MemCopy",
+    "MemFill",
+    "MemPages",
+    "MemGrow",
+    "HeapBase",
+    "FuturePoll",
+    "HostAbort",
+    "ClosureWord",
+    "ClosureCall",
+]
+
+/// Each builtin's name in a program, `BUILTIN_VARIANTS`' order (`Builtin::name`).
+pub const BUILTIN_NAMES: [String] = [
+    "println",
+    "print",
+    "eprint",
+    "eprintln",
+    "panic",
+    "str_cmp",
+    "str_hash",
+    "char_to_int",
+    "str_len_chars",
+    "str_chars",
+    "str_from_chars",
+    "str_join",
+    "str_index_of",
+    "json_quote",
+    "str_to_upper",
+    "str_to_lower",
+    "float_fixed",
+    "str_to_float",
+    "str_is_float",
+    "float_to_bits",
+    "int_to_char",
+    "str_as_bytes",
+    "test_selector",
+    "task_spawn",
+    "task_is_done",
+    "task_release",
+    "task_drive",
+    "task_output",
+    "task_yield_future",
+    "task_sleep_future_nanos",
+    "task_join_future",
+    "task_timeout_future",
+    "fs_read_to_string",
+    "fs_write_string",
+    "fs_take_string",
+    "fs_last_error_message",
+    "fs_temp_dir",
+    "fs_exists",
+    "fs_create_dir",
+    "fs_create_dir_all",
+    "fs_remove_file",
+    "fs_remove_dir_all",
+    "fs_rename",
+    "fs_read_dir",
+    "fs_take_string_array",
+    "fs_kind",
+    "fs_read",
+    "fs_take_bytes",
+    "fs_write",
+    "file_open",
+    "file_close",
+    "file_read",
+    "file_write",
+    "file_flush",
+    "io_stdin_read",
+    "io_stdout_write",
+    "io_stderr_write",
+    "io_stdout_flush",
+    "io_stderr_flush",
+    "net_connect",
+    "net_close",
+    "net_read",
+    "net_write",
+    "net_read_timeout",
+    "net_listen",
+    "net_local_port",
+    "net_accept",
+    "bytes_len",
+    "bytes_from_string_intrinsic",
+    "bytes_is_utf8",
+    "bytes_to_string_unchecked",
+    "bytes_at",
+    "bytes_slice",
+    "bytes_concat",
+    "bytes_to_ints",
+    "bytes_from_ints_intrinsic",
+    "bytes_eq",
+    "int_hash_seed",
+    "http_parse_request",
+    "crypto_hash",
+    "crypto_random_bytes",
+    "crypto_random_int",
+    "crypto_pbkdf2_sha256",
+    "crypto_pbkdf2_sha256_verify",
+    "time_now_nanos",
+    "time_now_epoch_nanos",
+    "log_config_level",
+    "log_config_to_stderr",
+    "log_set_config",
+    "shutdown_requested",
+    "process_arg_count",
+    "process_arg",
+    "process_exit",
+    "process_run",
+    "process_take_status",
+    "process_take_stdout",
+    "process_take_stderr",
+    "process_env",
+    "process_current_exe",
+    "ui_state_present",
+    "ui_state_load",
+    "ui_state_store",
+    "router_state_present",
+    "router_state_load",
+    "router_state_store",
+    "mem_load8",
+    "mem_store8",
+    "mem_load64",
+    "mem_store64",
+    "mem_copy",
+    "mem_fill",
+    "mem_pages",
+    "mem_grow",
+    "heap_base",
+    "future_poll",
+    "host_abort",
+    "closure_word",
+    "closure_call",
+]
+
+/// The builtins every module sees (`Builtin::GLOBAL`).
+pub const GLOBAL: [String] = ["Println", "Print", "EPrint", "EPrintln", "Panic"]
+
+/// The builtins only std's modules see (`Builtin::STD_ONLY`).
+pub const STD_ONLY: [String] = [
+    "StrCmp",
+    "StrHash",
+    "CharToInt",
+    "StrLenChars",
+    "StrChars",
+    "StrFromChars",
+    "StrJoin",
+    "StrIndexOf",
+    "JsonQuote",
+    "StrToUpper",
+    "StrToLower",
+    "FloatFixed",
+    "StrToFloat",
+    "StrIsFloat",
+    "FloatToBits",
+    "IntToChar",
+    "StrAsBytes",
+    "TestSelector",
+    "TaskSpawn",
+    "TaskIsDone",
+    "TaskRelease",
+    "TaskDrive",
+    "TaskOutput",
+    "TaskYieldFuture",
+    "TaskSleepFutureNanos",
+    "TaskJoinFuture",
+    "TaskTimeoutFuture",
+    "FsReadToString",
+    "FsWriteString",
+    "FsTakeString",
+    "FsLastErrorMessage",
+    "FsTempDir",
+    "FsExists",
+    "FsCreateDir",
+    "FsCreateDirAll",
+    "FsRemoveFile",
+    "FsRemoveDirAll",
+    "FsRename",
+    "FsReadDir",
+    "FsTakeStringArray",
+    "FsKind",
+    "FsRead",
+    "FsTakeBytes",
+    "FsWrite",
+    "FileOpen",
+    "FileClose",
+    "FileRead",
+    "FileWrite",
+    "FileFlush",
+    "IoStdinRead",
+    "IoStdoutWrite",
+    "IoStderrWrite",
+    "IoStdoutFlush",
+    "IoStderrFlush",
+    "NetConnect",
+    "NetClose",
+    "NetRead",
+    "NetWrite",
+    "NetReadTimeout",
+    "NetListen",
+    "NetLocalPort",
+    "NetAccept",
+    "BytesLen",
+    "BytesFromString",
+    "BytesIsUtf8",
+    "BytesToStringUnchecked",
+    "BytesAt",
+    "BytesSlice",
+    "BytesConcat",
+    "BytesToInts",
+    "BytesFromInts",
+    "BytesEq",
+    "IntHashSeed",
+    "HttpParseRequest",
+    "CryptoHash",
+    "CryptoRandomBytes",
+    "CryptoRandomInt",
+    "CryptoPbkdf2Sha256",
+    "CryptoPbkdf2Sha256Verify",
+    "TimeNowNanos",
+    "TimeNowEpochNanos",
+    "LogConfigLevel",
+    "LogConfigToStderr",
+    "LogSetConfig",
+    "ShutdownRequested",
+    "ProcessArgCount",
+    "ProcessArg",
+    "ProcessExit",
+    "ProcessRun",
+    "ProcessTakeStatus",
+    "ProcessTakeStdout",
+    "ProcessTakeStderr",
+    "ProcessEnv",
+    "ProcessCurrentExe",
+    "UiStatePresent",
+    "UiStateLoad",
+    "UiStateStore",
+    "RouterStatePresent",
+    "RouterStateLoad",
+    "RouterStateStore",
+]
+
+/// The builtins only std's modules see on the WASM target (`Builtin::WASM_ONLY`).
+pub const WASM_ONLY: [String] = [
+    "MemLoad8",
+    "MemStore8",
+    "MemLoad64",
+    "MemStore64",
+    "MemCopy",
+    "MemFill",
+    "MemPages",
+    "MemGrow",
+    "HeapBase",
+    "FuturePoll",
+    "HostAbort",
+    "ClosureWord",
+    "ClosureCall",
+]
+
+/// The builtin table, read once: a constant array is built again each time
+/// it is read, so the resolver and the dump read this record's copies.
+pub record BuiltinTable {
+    pub variants: [String],
+    pub names: [String],
+    pub global: Vec<Int>,
+    pub std_only: Vec<Int>,
+    pub wasm_only: Vec<Int>,
+}
+
+/// The builtin table, with `GLOBAL`, `STD_ONLY` and `WASM_ONLY` as indices.
+pub fn builtin_table() -> BuiltinTable {
+    let variants = BUILTIN_VARIANTS
+    BuiltinTable {
+        variants: variants,
+        names: BUILTIN_NAMES,
+        global: builtin_indices(variants, GLOBAL),
+        std_only: builtin_indices(variants, STD_ONLY),
+        wasm_only: builtin_indices(variants, WASM_ONLY),
+    }
+}
+
+// Each builtin of `list`, Rust variant names, by its index in `variants`.
+fn builtin_indices(variants: [String], list: [String]) -> Vec<Int> {
+    let mut out: Vec<Int> = Vec::new()
+    for v in list.iter() {
+        let mut i = 0
+        while i < variants.len() && variants[i] != v { i += 1 }
+        out.push(i)
+    }
+    out
+}
+
+/// The type names a program may not declare (`RESERVED_TYPE_NAMES`).
+pub const RESERVED_TYPE_NAMES: [String] = [
+    "Int",
+    "Float",
+    "Bool",
+    "Char",
+    "String",
+    "Future",
+    "Bytes",
+]
+
+/// The target a program is resolved for (`CompileTarget`).
+pub type CompileTarget = | CompileTargetNative | CompileTargetWasm
+
+/// The target's name (`CompileTarget::name`).
+pub fn compile_target_name(target: CompileTarget) -> String {
+    match target {
+        CompileTargetNative => "native"
+        CompileTargetWasm => "wasm"
+    }
+}
+
+/// The std modules every native program compiles, in order: each name,
+/// and its source under std's directory (`STD_MODULES`).
+pub const STD_MODULES: [(String, String)] = [
+    ("$std.core", "core/lib.nova"),
+    ("$std.bytes", "bytes/lib.nova"),
+    ("$std.io", "io/lib.nova"),
+    ("$std.fs", "fs/lib.nova"),
+    ("$std.collections", "collections/lib.nova"),
+    ("$std.strings", "strings/lib.nova"),
+    ("$std.fmt", "fmt/lib.nova"),
+    ("$std.task", "task/lib.nova"),
+    ("$std.sync", "sync/lib.nova"),
+    ("$std.net", "net/lib.nova"),
+    ("$std.time", "time/lib.nova"),
+    ("$std.json", "json/lib.nova"),
+    ("$std.log", "log/lib.nova"),
+    ("$std.http", "http/lib.nova"),
+    ("$std.crypto", "crypto/lib.nova"),
+    ("$std.process", "process/lib.nova"),
+    ("$std.ui", "ui/lib.nova"),
+    ("$std.ui.html", "ui/html/lib.nova"),
+    ("$std.ui.backend", "ui/backend/native.nova"),
+    ("$std.ui.router", "ui/router/lib.nova"),
+    ("$std.ui.ssr", "ui/ssr/lib.nova"),
+]
+
+/// The std modules a WASM program compiles, in order (`WASM_STD_MODULES`).
+pub const WASM_STD_MODULES: [(String, String)] = [
+    ("$std.core", "core/lib.nova"),
+    ("$std.bytes", "bytes/lib.nova"),
+    ("$std.collections", "collections/lib.nova"),
+    ("$std.strings", "strings/lib.nova"),
+    ("$std.fmt", "fmt/lib.nova"),
+    ("$std.task", "task/lib.nova"),
+    ("$std.sync", "sync/lib.nova"),
+    ("$std.time", "time/lib.nova"),
+    ("$std.json", "json/lib.nova"),
+    ("$std.log", "log/lib.nova"),
+    ("$std.dom", "dom/lib.nova"),
+    ("$std.ui", "ui/lib.nova"),
+    ("$std.ui.html", "ui/html/lib.nova"),
+    ("$std.ui.backend", "ui/backend/wasm.nova"),
+    ("$std.ui.router", "ui/router/lib.nova"),
+    ("$std.rt", "rt/lib.nova"),
+]
+
+/// The target's std modules (`CompileTarget::std_modules`).
+pub fn std_modules(target: CompileTarget) -> [(String, String)] {
+    match target {
+        CompileTargetNative => STD_MODULES
+        CompileTargetWasm => WASM_STD_MODULES
+    }
+}
+
+/// The std modules a program reaches only through an `import`, by the path
+/// the import writes, and each one's module (`IMPORTED_STD`).
+pub const IMPORTED_STD: [(String, String)] = [
+    ("dom", "$std.dom"),
+    ("ui", "$std.ui"),
+    ("ui::html", "$std.ui.html"),
+    ("ui::router", "$std.ui.router"),
+    ("ui::ssr", "$std.ui.ssr"),
+]
+
+/// The names no module of a program may take (`RESERVED_MODULE_NAMES`).
+pub const RESERVED_MODULE_NAMES: [String] = ["dom", "ui"]
+
+/// The std modules whose names only std's modules see (`STD_INTERNAL`).
+pub const STD_INTERNAL: [String] = ["$std.rt", "$std.ui.backend"]
+
+/// `std/test`, which only `nova test` compiles (`STD_TEST_MODULE`).
+pub const STD_TEST_MODULE: (String, String) = ("$std.test", "test/lib.nova")
+
+/// Whether `name` is the import path of one of `IMPORTED_STD`.
+pub fn is_imported_std_path(name: String) -> Bool {
+    for (path, _) in IMPORTED_STD.iter() { if path == name { return true } }
+    false
+}
+
+/// Whether `module` is the module of one of `IMPORTED_STD`.
+pub fn is_imported_std_module(name: String) -> Bool {
+    for (_, m) in IMPORTED_STD.iter() { if m == name { return true } }
+    false
+}
+
+/// The module `IMPORTED_STD` gives the import path `name`, or "".
+pub fn imported_std_module(name: String) -> String {
+    for (path, m) in IMPORTED_STD.iter() { if path == name { return m } }
+    ""
+}
+
+/// Whether `list` holds `name`.
+pub fn holds(list: [String], name: String) -> Bool {
+    for n in list.iter() { if n == name { return true } }
+    false
+}
diff --git a/selfhost/src/diagnostic.nova b/selfhost/src/diagnostic.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1cc6dcc35af89ebcf243e16b3f7147a6db87031d
--- /dev/null
+++ b/selfhost/src/diagnostic.nova
@@ -0,0 +1,280 @@
+// A diagnostic, as S0's nova-diagnostics holds one: a port of
+// crates/nova-diagnostics/src/lib.rs's `Diagnostic`, `Label`, `Fix` and
+// `Edit`, of suggest.rs (the nearest name, for a fix) and of lines.rs's
+// `keyword_start` (where `pub` goes, for a fix), function by function
+// (spec
+// docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
+// §3.3, ADR 0053).
+//
+// - A span in S0 carries its file's `FileId`; N's spans do not (ADR 0052),
+//   so a label and an edit carry their file's label beside the span, the
+//   label the resolution dump prints: a loose module's stem, or a std
+//   module's `<std/core>`.
+// - S0's builders (`Diagnostic::error(…).with_primary_label(…)`) mutate a
+//   temporary, which a `mut self` method may not (E0060), so a diagnostic
+//   is made by `Diagnostic::error` or `error_at` and added to through a
+//   variable: `let mut d = …; d.add_note(…)`.
+
+import token
+
+/// How severe a diagnostic is (`Severity`).
+pub type Severity = | SeverityError | SeverityWarning | SeverityNote | SeverityHelp
+
+/// A span of a file, with a message (`Label`).
+pub record Label { pub file: String, pub span: Span, pub message: String, pub primary: Bool }
+
+/// One edit of a fix: the text that replaces a span of a file (`Edit`).
+pub record Edit { pub file: String, pub span: Span, pub text: String }
+
+impl Edit {
+    /// `text` inserted at `at` (`Edit::insert`).
+    pub fn insert(at: Int, file: String, text: String) -> Edit {
+        Edit { file: file, span: Span::point(at), text: text }
+    }
+
+    /// `text` in place of `span` (`Edit::replace`).
+    pub fn replace(file: String, span: Span, text: String) -> Edit {
+        Edit { file: file, span: span, text: text }
+    }
+}
+
+/// A fix: a title and its edits (`Fix`).
+pub record Fix { pub title: String, pub edits: Vec<Edit> }
+
+/// A diagnostic (`Diagnostic`): severity, code, message, labels in order,
+/// notes and fixes.
+pub record Diagnostic {
+    pub severity: Severity,
+    pub code: String,
+    pub message: String,
+    pub labels: Vec<Label>,
+    pub notes: Vec<String>,
+    pub fixes: Vec<Fix>,
+}
+
+impl Diagnostic {
+    /// An error with no label yet (`Diagnostic::error`).
+    pub fn error(code: String, message: String) -> Diagnostic {
+        Diagnostic {
+            severity: SeverityError,
+            code: code,
+            message: message,
+            labels: Vec::new(),
+            notes: Vec::new(),
+            fixes: Vec::new(),
+        }
+    }
+
+    /// A warning with no label yet (`Diagnostic::warning`).
+    pub fn warning(code: String, message: String) -> Diagnostic {
+        let mut d = Diagnostic::error(code, message)
+        d.severity = SeverityWarning
+        d
+    }
+
+    /// `with_primary_label`.
+    pub fn add_primary(mut self, file: String, span: Span, message: String) {
+        self.labels.push(Label { file: file, span: span, message: message, primary: true })
+    }
+
+    /// `with_secondary_label`.
+    pub fn add_secondary(mut self, file: String, span: Span, message: String) {
+        self.labels.push(Label { file: file, span: span, message: message, primary: false })
+    }
+
+    /// `with_note`.
+    pub fn add_note(mut self, note: String) { self.notes.push(note) }
+
+    /// `with_fix`.
+    pub fn add_fix(mut self, fix: Fix) { self.fixes.push(fix) }
+
+    /// Whether it is an error.
+    pub fn is_error(self) -> Bool {
+        match self.severity {
+            SeverityError => true
+            _ => false
+        }
+    }
+}
+
+/// An error with its primary label: `Diagnostic::error(code,
+/// message).with_primary_label(span, label)`.
+pub fn error_at(
+    code: String,
+    message: String,
+    file: String,
+    span: Span,
+    label: String,
+) -> Diagnostic {
+    let mut d = Diagnostic::error(code, message)
+    d.add_primary(file, span, label)
+    d
+}
+
+// ---------------------------------------------------------------------------
+// suggest.rs
+// ---------------------------------------------------------------------------
+
+/// The Damerau-Levenshtein distance between `a` and `b`, in characters,
+/// a swap of two neighbours one edit (`suggest::distance`).
+pub fn distance(a: String, b: String) -> Int {
+    let a = a.chars()
+    let b = b.chars()
+    let width = b.len() + 1
+    let mut d = [0; (a.len() + 1) * width]
+    let mut i = 0
+    while i <= a.len() {
+        d[i * width] = i
+        i += 1
+    }
+    let mut j = 0
+    while j <= b.len() {
+        d[j] = j
+        j += 1
+    }
+    i = 1
+    while i <= a.len() {
+        j = 1
+        while j <= b.len() {
+            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 }
+            let mut best = min(
+                min(d[(i - 1) * width + j] + 1, d[i * width + j - 1] + 1),
+                d[(i - 1) * width + j - 1] + cost,
+            )
+            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
+                best = min(best, d[(i - 2) * width + j - 2] + 1)
+            }
+            d[i * width + j] = best
+            j += 1
+        }
+        i += 1
+    }
+    d[a.len() * width + b.len()]
+}
+
+/// The candidate nearest `name`, other than itself, within a third of its
+/// length (at least one edit), as `suggest::closest` picks it: one equal
+/// to it ignoring ASCII case first, at any distance; then the least
+/// distance; then the least in byte order. `None` when none is near.
+pub fn closest(name: String, candidates: [String]) -> Option<String> {
+    let len = name.len()
+    let bound = max(len, 3) / 3
+    // The best so far: (far, distance, candidate), compared in that order;
+    // `far` is false for a name equal ignoring case.
+    let mut found = false
+    let mut best_far = false
+    let mut best_d = 0
+    let mut best = ""
+    for c in candidates.iter() {
+        if c == name { continue }
+        let mut far = true
+        let mut d = 0
+        if ascii_case_only(c, name) {
+            far = false
+        } else {
+            if abs(c.len() - len) > bound { continue }
+            d = distance(name, c)
+            if d > bound { continue }
+        }
+        let better = if !found {
+            true
+        } else if far != best_far {
+            !far
+        } else if d != best_d {
+            d < best_d
+        } else {
+            less(c, best)
+        }
+        if better {
+            found = true
+            best_far = far
+            best_d = d
+            best = c
+        }
+    }
+    if found { Some(best) } else { None }
+}
+
+// Whether `a` and `b` are equal but for the case of ASCII letters
+// (`eq_ignore_ascii_case`), byte by byte.
+fn ascii_case_only(a: String, b: String) -> Bool {
+    let n = a.byte_len()
+    if n != b.byte_len() { return false }
+    let mut i = 0
+    while i < n {
+        if fold(a.byte_at(i)) != fold(b.byte_at(i)) { return false }
+        i += 1
+    }
+    true
+}
+
+fn fold(b: Int) -> Int { if b >= 65 && b <= 90 { b + 32 } else { b } }
+
+/// A fix that changes `name` to the candidate nearest it, if one is near
+/// (`suggest::change_to`).
+pub fn change_to(file: String, name: Spanned<String>, candidates: [String]) -> Option<Fix> {
+    let to = closest(name.value, candidates)?
+    let mut edits: Vec<Edit> = Vec::new()
+    edits.push(Edit::replace(file, name.span, to))
+    Some(Fix { title: "change `${name.value}` to `${to}`", edits: edits })
+}
+
+// ---------------------------------------------------------------------------
+// lines.rs
+// ---------------------------------------------------------------------------
+
+/// Where the keyword before the name at byte `name_start` of `text`
+/// begins, when it is `fn` (or `async fn`, then `async`), `record`,
+/// `type`, `trait`, `const` or `component`: where `pub` goes
+/// (`lines::keyword_start`).
+pub fn keyword_start(text: String, name_start: Int) -> Option<Int> {
+    let end = back_over_space(text, name_start)
+    let start = back_over_word(text, end)
+    let keyword = text.byte_slice(start, end)
+    if keyword != "fn"
+        && keyword != "record"
+        && keyword != "type"
+        && keyword != "trait"
+        && keyword != "const"
+        && keyword != "component" {
+        return None
+    }
+    if keyword == "fn" {
+        let end = back_over_space(text, start)
+        let word = back_over_word(text, end)
+        if text.byte_slice(word, end) == "async" { return Some(word) }
+    }
+    Some(start)
+}
+
+fn back_over_space(text: String, from: Int) -> Int {
+    let mut i = from
+    while i > 0 && (text.byte_at(i - 1) == 32 || text.byte_at(i - 1) == 9) { i -= 1 }
+    i
+}
+
+fn back_over_word(text: String, from: Int) -> Int {
+    let mut i = from
+    while i > 0 && is_ascii_alphabetic(text.byte_at(i - 1)) { i -= 1 }
+    i
+}
+
+fn is_ascii_alphabetic(b: Int) -> Bool { (b >= 65 && b <= 90) || (b >= 97 && b <= 122) }
+
+// ---------------------------------------------------------------------------
+// Helpers
+// ---------------------------------------------------------------------------
+
+/// Whether `a` comes before `b` in byte order (`String`'s `Ord`).
+pub fn less(a: String, b: String) -> Bool {
+    match a.cmp(b) {
+        Less => true
+        _ => false
+    }
+}
+
+fn min(a: Int, b: Int) -> Int { if a < b { a } else { b } }
+
+fn max(a: Int, b: Int) -> Int { if a > b { a } else { b } }
+
+fn abs(a: Int) -> Int { if a < 0 { -a } else { a } }
diff --git a/selfhost/src/loader.nova b/selfhost/src/loader.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4785923e65e8cf40addd003585379955409c5d02
--- /dev/null
+++ b/selfhost/src/loader.nova
@@ -0,0 +1,450 @@
+// The loader of a loose program: a port of
+// crates/nova-driver/src/program.rs's `load_program` and `Loader` on the
+// loose path, and of the driver's `strip_test_functions` and `package_of`
+// (spec
+// docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
+// §3.2, ADR 0053). From the root it reads, lexes and parses every module
+// it reaches, breadth first, finding each single-segment import as a file
+// beside the root; the order of the modules is the order of that search,
+// and each module's label is its file's stem, as in S0. Packages
+// (`nova.toml`, `src/`, `tests/`, a dependency) are 5.2b's.
+//
+// The Rust's idioms, translated:
+//
+// - A loose `Place` is a directory and a stem, and every module of a loose
+//   program is in the root's directory, so a place is its stem.
+// - `VecDeque` is a `Vec` and the index of its head.
+// - A path is a `String` with `/` between its parts, as `Path::join` and
+//   `Path::parent` treat one on Linux and macOS: `Path::join` writes `\`
+//   on Windows, which the harness keeps out of what it compares (ADR 0053
+//   §5).
+
+import ast
+import builtins
+import diagnostic
+import lexer
+import parser
+import token
+
+/// One module the loader read (`Loaded`): its label, path, tree and
+/// source, what each of its imports names by first segment, and what it
+/// could import.
+pub record Loaded {
+    pub name: String,
+    pub path: String,
+    pub ast: File,
+    pub text: String,
+    pub imports: Map<String, ImportTarget>,
+    pub importable: Vec<(String, Int)>,
+}
+
+/// What `load_program` found (`Load`): the modules in load order, and the
+/// lex, parse and import diagnostics in load order.
+pub record Load { pub modules: Vec<Loaded>, pub diagnostics: Vec<Diagnostic> }
+
+/// What a module's import names, as the loader decided (`ImportTarget`,
+/// which S0 keeps in `nova-resolver`): kept here, with the loader that
+/// decides it, so the loader builds without the resolver.
+pub type ImportTarget = | ImportTargetModule(Int) | ImportTargetReported
+
+// What an import names before every module is loaded (`Pending`).
+type Pending = | PendingPlace(String) | PendingReported
+
+// What `find` found for one import (`Found`): a module, by its place and
+// path; an error, reported here; or nothing, which the resolver reports.
+type Found = | FoundModule(String, String) | FoundReported(Diagnostic) | FoundNothing
+
+// The loader's state (`Loader`): the root's directory, and each stem's
+// answer to `has`, from one listing of the directory read once.
+record Loader { dir: String, listing: Vec<String>, listed: Bool, known: Map<String, Bool> }
+
+/// Read, lex and parse every module the loose program whose root is
+/// `root` reaches, breadth first (`load_program`). A module that cannot be
+/// read is skipped, so an import of it is "cannot find module"; `Err` only
+/// when the root cannot be read.
+pub async fn load_program(root: String) -> Result<Load, IoError> {
+    let mut loader = Loader {
+        dir: path_parent_or_empty(root),
+        listing: Vec::new(),
+        listed: false,
+        known: Map::new(),
+    }
+    let mut modules: Vec<Loaded> = Vec::new()
+    let mut diagnostics: Vec<Diagnostic> = Vec::new()
+    let mut pending: Vec<Vec<(String, Pending)>> = Vec::new()
+    let mut places: Map<String, Int> = Map::new()
+    let mut place_order: Vec<String> = Vec::new()
+    let mut tried: Set<String> = Set::new()
+    let mut queue: Vec<(String, String)> = Vec::new()
+    queue.push((module_name(root), root))
+    let mut head = 0
+    let mut is_entry = true
+
+    while head < queue.len() {
+        let (place, path) = queue.at(head)
+        head += 1
+        let entry = is_entry
+        is_entry = false
+        if !tried.insert(place) { continue }
+        let source = match read_to_string(path).await {
+            Ok(s) => s
+            Err(e) => {
+                if entry { return Err(e) }
+                continue
+            }
+        }
+        let label = place
+        let (tokens, lex_errors) = lex(source)
+        for e in lex_errors.iter() {
+            diagnostics.push(error_at("L0001", "${e}", label, e.span(), "here"))
+        }
+        let (file, parse_errors) = parse(tokens)
+        for e in parse_errors.iter() {
+            let mut d = error_at(e.code(), "${e}", label, e.span(), "here")
+            for n in e.notes().iter() { d.add_note(n) }
+            diagnostics.push(d)
+        }
+
+        // Only a single-segment import names a module; the resolver
+        // rejects the others.
+        let mut imports: Vec<(String, Pending)> = Vec::new()
+        for item in file.items.iter() {
+            match item.value {
+                ItemImport(imp) => {
+                    if imp.path.value.segments.len() != 1 { continue }
+                    let name = imp.path.value.segments.at(0).value
+                    match loader.find(place, name, imp.path.span).await {
+                        FoundModule(target, target_path) => {
+                            queue.push((target, target_path))
+                            imports.push((name, PendingPlace(target)))
+                        }
+                        FoundReported(diagnostic) => {
+                            diagnostics.push(diagnostic)
+                            imports.push((name, PendingReported))
+                        }
+                        FoundNothing => ()
+                    }
+                }
+                _ => ()
+            }
+        }
+        let _ = places.insert(place, modules.len())
+        place_order.push(place)
+        modules.push(
+            Loaded {
+                name: label,
+                path: path,
+                ast: file,
+                text: source,
+                imports: Map::new(),
+                importable: Vec::new(),
+            },
+        )
+        pending.push(imports)
+    }
+
+    let mut i = 0
+    while i < modules.len() {
+        let mut m = modules.at(i)
+        for (name, target) in pending.at(i).iter() {
+            match target {
+                PendingPlace(place) => {
+                    match places.get(place) {
+                        Some(index) => { let _ = m.imports.insert(name, ImportTargetModule(index)) }
+                        // It could not be read.
+                        None => ()
+                    }
+                }
+                PendingReported => { let _ = m.imports.insert(name, ImportTargetReported) }
+            }
+        }
+        i += 1
+    }
+
+    // What each loaded module could import, by the name an import of it
+    // writes, through `find` itself: every module beside it, by stem.
+    let mut names: Vec<String> = Vec::new()
+    for p in place_order.iter() { names.push(p) }
+    names = sorted_dedup(names)
+    i = 0
+    while i < modules.len() {
+        let from = place_order.at(i)
+        let mut importable: Vec<(String, Int)> = Vec::new()
+        for name in names.iter() {
+            match loader.find(from, name, Span::point(0)).await {
+                FoundModule(target, _) => {
+                    match places.get(target) {
+                        Some(j) => { if j != i { importable.push((name, j)) } }
+                        None => ()
+                    }
+                }
+                _ => ()
+            }
+        }
+        let mut m = modules.at(i)
+        m.importable = importable
+        i += 1
+    }
+    Ok(Load { modules: modules, diagnostics: diagnostics })
+}
+
+impl Loader {
+    // What `import name`, at `at` in the module `from`, names (`find`): a
+    // module under a std module's name is E0093; a std module's name that
+    // names no file is left to the resolver.
+    async fn find(mut self, from: String, name: String, at: Span) -> Found {
+        let found = self.find_place(from, name).await
+        if !holds(RESERVED_MODULE_NAMES, name) { return found }
+        let what = match found {
+            FoundModule(_, path) => path
+            FoundReported(_) => "this module or dependency"
+            FoundNothing => return found
+        }
+        FoundReported(
+            error_at(
+                "E0093",
+                "`${name}` is the name of a std module, so no module may take it: rename ${what}",
+                from,
+                at,
+                "std's module, not this one",
+            ),
+        )
+    }
+
+    // `find`, before std's names are checked (`find_place`): on the loose
+    // path, the file `name.nova` beside the root, or nothing.
+    async fn find_place(mut self, from: String, name: String) -> Found {
+        if !self.has(name).await { return FoundNothing }
+        FoundModule(name, path_join(self.dir, "${name}.nova"))
+    }
+
+    // Whether the root's directory holds `<stem>.nova` (`has`, `look`),
+    // answered once per load: its name exactly in the directory's listing,
+    // read once; not if only a name differing in case is; otherwise
+    // whether a file of that path reads.
+    async fn has(mut self, stem: String) -> Bool {
+        match self.known.get(stem) {
+            Some(known) => return known
+            None => ()
+        }
+        if !self.listed {
+            let listed = if self.dir == "" { "." } else { self.dir }
+            match read_dir(listed).await {
+                Ok(entries) => { for e in entries.iter() { self.listing.push(e.name) } }
+                Err(_) => ()
+            }
+            self.listed = true
+        }
+        let name = "${stem}.nova"
+        let mut found = false
+        let mut folded = false
+        for n in self.listing.iter() {
+            if n == name { found = true }
+            if eq_ignore_ascii_case(n, name) { folded = true }
+        }
+        if !found && !folded {
+            found = match read_to_string(path_join(self.dir, name)).await {
+                Ok(_) => true
+                Err(_) => false
+            }
+        }
+        let _ = self.known.insert(stem, found)
+        found
+    }
+}
+
+/// Remove every `@test` function, as the driver does before resolving
+/// under `check` (`strip_test_functions` with `keep_root_tests` false).
+pub fn strip_test_functions(modules: Vec<Loaded>) {
+    for loaded in modules.iter() {
+        let mut kept: Vec<Spanned<Item>> = Vec::new()
+        for item in loaded.ast.items.iter() {
+            let test = match item.value {
+                ItemFunction(f) => {
+                    let mut any = false
+                    for a in f.attrs.iter() { if a.name.value == "test" { any = true } }
+                    any
+                }
+                _ => false
+            }
+            if !test { kept.push(item) }
+        }
+        let mut file = loaded.ast
+        file.items = kept
+    }
+}
+
+/// The package `path` is a module of, when it is directly in the `src/` or
+/// `tests/` of a directory holding a `nova.toml` (`package_of`): whether it
+/// is.
+pub async fn in_package(path: String) -> Bool {
+    let parent = match path_parent(path) {
+        Some(p) => p
+        None => return false
+    }
+    let dir_name = file_name(parent)
+    if dir_name != "src" && dir_name != "tests" { return false }
+    let root = match path_parent(parent) {
+        Some(r) => r
+        None => return false
+    }
+    let listed = if root == "" { "." } else { root }
+    match read_dir(listed).await {
+        Ok(entries) => {
+            for e in entries.iter() { if e.name == "nova.toml" && e.is_file { return true } }
+            false
+        }
+        Err(_) => false
+    }
+}
+
+// ---------------------------------------------------------------------------
+// Paths
+// ---------------------------------------------------------------------------
+
+/// A module's name from its path, its file's stem, or `main` when it has
+/// none (`FrontendContext::module_name`).
+pub fn module_name(path: String) -> String {
+    let name = file_name(path)
+    if name == "" || name == ".." { return "main" }
+    let n = name.byte_len()
+    let mut dot = -1
+    let mut i = n - 1
+    while i > 0 {
+        if name.byte_at(i) == 46 {
+            dot = i
+            break
+        }
+        i -= 1
+    }
+    if dot <= 0 { name } else { name.byte_slice(0, dot) }
+}
+
+// The last part of `path` (`Path::file_name`), "" for none: a path's
+// last component, past any `/` or `.` component that ends it, unless it is
+// `..`.
+fn file_name(path: String) -> String {
+    let lo = before_body(path)
+    let end = trim_back(path, lo, path.byte_len())
+    if end == lo { return "" }
+    let mut i = end
+    while i > lo && path.byte_at(i - 1) != 47 { i -= 1 }
+    let name = path.byte_slice(i, end)
+    if name == ".." { "" } else { name }
+}
+
+/// The directory `path` is in (`Path::parent`): `None` for "", "/" and
+/// "/."; "" for a path of one part, "." and "./"; otherwise `path` up to
+/// its last component's start, less the `/`s and `.` components that end
+/// it, as `Components::as_path` trims them, but never a leading `/` or
+/// `./`: `sub/./main.nova` is in `sub`, `./main.nova` in `.`.
+pub fn path_parent(path: String) -> Option<String> {
+    let lo = before_body(path)
+    let end = trim_back(path, lo, path.byte_len())
+    if end == lo {
+        // No component past the root or the leading `.`.
+        if lo == 1 && path.byte_at(0) == 46 { return Some("") }
+        return None
+    }
+    let mut i = end
+    while i > lo && path.byte_at(i - 1) != 47 { i -= 1 }
+    Some(path.byte_slice(0, trim_back(path, lo, i)))
+}
+
+// How much of `path` comes before its first component, which `Path` never
+// trims: a root `/`, or the `.` of a leading `./` (or of `.` alone).
+fn before_body(path: String) -> Int {
+    let n = path.byte_len()
+    if n == 0 { return 0 }
+    if path.byte_at(0) == 47 { return 1 }
+    if path.byte_at(0) == 46 && (n == 1 || path.byte_at(1) == 47) { return 1 }
+    0
+}
+
+// `end`, moved back past the `/`s and `.` components that end
+// `path.byte_slice(0, end)`, but not below `lo`.
+fn trim_back(path: String, lo: Int, end: Int) -> Int {
+    let mut e = end
+    let mut more = true
+    while more {
+        while e > lo && path.byte_at(e - 1) == 47 { e -= 1 }
+        more = e > lo && path.byte_at(e - 1) == 46 && (e - 1 == lo || path.byte_at(e - 2) == 47)
+        if more { e -= 1 }
+    }
+    e
+}
+
+// `path_parent`, or "" (`root.path.parent().unwrap_or_default()`).
+fn path_parent_or_empty(path: String) -> String {
+    match path_parent(path) {
+        Some(p) => p
+        None => ""
+    }
+}
+
+/// `dir` joined with `name` (`Path::join`): `name` alone for an empty
+/// `dir`, and no second `/` after one `dir` ends with.
+pub fn path_join(dir: String, name: String) -> String {
+    if dir == "" { return name }
+    if dir.ends_with("/") { "${dir}${name}" } else { "${dir}/${name}" }
+}
+
+// Whether `a` and `b` are equal but for the case of ASCII letters.
+fn eq_ignore_ascii_case(a: String, b: String) -> Bool {
+    let n = a.byte_len()
+    if n != b.byte_len() { return false }
+    let mut i = 0
+    while i < n {
+        let x = a.byte_at(i)
+        let y = b.byte_at(i)
+        let fx = if x >= 65 && x <= 90 { x + 32 } else { x }
+        let fy = if y >= 65 && y <= 90 { y + 32 } else { y }
+        if fx != fy { return false }
+        i += 1
+    }
+    true
+}
+
+/// `names` sorted in byte order, each once (`sort`, then `dedup`): a merge
+/// sort.
+pub fn sorted_dedup(names: Vec<String>) -> Vec<String> {
+    let sorted = sort_strings(names)
+    let mut out: Vec<String> = Vec::new()
+    for n in sorted.iter() { if out.is_empty() || out.last() != n { out.push(n) } }
+    out
+}
+
+/// `names` sorted in byte order, stably: a merge sort.
+pub fn sort_strings(names: Vec<String>) -> Vec<String> {
+    let n = names.len()
+    let mut a = names.to_array()
+    if n < 2 { return Vec::from_array(a) }
+    let mut b = [""; n]
+    let mut width = 1
+    while width < n {
+        let mut lo = 0
+        while lo < n {
+            let mid = if lo + width < n { lo + width } else { n }
+            let hi = if lo + 2 * width < n { lo + 2 * width } else { n }
+            let mut i = lo
+            let mut j = mid
+            let mut k = lo
+            while k < hi {
+                if i < mid && (j >= hi || !less(a[j], a[i])) {
+                    b[k] = a[i]
+                    i += 1
+                } else {
+                    b[k] = a[j]
+                    j += 1
+                }
+                k += 1
+            }
+            lo = hi
+        }
+        let t = a
+        a = b
+        b = t
+        width = width * 2
+    }
+    Vec::from_array(a)
+}
````

### Task 5: N's resolver, its dump and `selfhost resolve`

Spec §3.3, §3.4; decisions 18 to 20 and 29.

**Files:**
- Create: `selfhost/src/resolver.nova`, `selfhost/src/resolution_dump.nova`,
  `selfhost/src/resolver_tests.nova`
- Modify: `selfhost/src/main.nova` (`selfhost resolve`, the imports)

- [ ] **Step 1: The tests.** `patch_block t5-tests`:
  `resolver_tests.nova`'s five `@test`s,
  `a_glob_imports_clashes_are_reported_in_declaration_order`,
  `an_impls_type_names_its_methods`,
  `a_builtins_name_redefined_is_e0002_with_a_note`,
  `importing_a_private_item_offers_pub_where_its_keyword_starts` and
  `an_unknown_attribute_on_a_function_offers_the_nearest_known_one`; and
  `main.nova`'s import of the module.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`.
  - `SH the_selfhost_tests_pass`. Expected: `0 passed; 1 failed`: `cannot find module `resolver``.
  - `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 3`, stderr
    too, where a compile error goes. Expected: the same error, then `Error: could not compile due to 1 previous error`.
- [ ] **Step 3: The code.** `patch_block t5-code`:
  - **`resolver.nova`:** the definitions, scopes, exports and
    `ProgramResolution`; `std_module`; `resolve_program_for`, its
    seeding, pass 1, E0090, pass 2, the fix-dropping step, std's glob,
    E0102 and `missing_names`; `collect_item` and its attribute checks;
    `resolve_import`, `make_public`, `would_receive`, the bindings,
    `type_full_name` (decision 18);
  - **`resolution_dump.nova`:** `dump_program`, `dump_std` and the
    writer (decision 20);
  - **`main.nova`:** `selfhost resolve --std DIR (--std-part | FILE…)`,
    std read and parsed once (`load_std`), and `dump_resolution`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH the_selfhost_tests_pass`. Expected: `1 passed; 0 failed`.
  - `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 3`.
    Expected: `test result: ok. 25 passed; 0 failed; 0 trapped; 25 total`.
  - `SH`. Expected: `31 passed; 0 failed`.
  - `git status --short`. Expected: nothing.
  - `S` (4 min 37 s). Expected: `2852 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `selfhost: N's resolver, its dump and selfhost resolve`.

**Patch `t5-tests`:**

<!-- patch t5-tests -->

````diff
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index bd10996c4cee33c74b687fdbb444b784c7f5f7fd..9f0571ce206b22ad914a3acf7961bd7de900769e 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -19,6 +19,7 @@ import lexer
 import lexer_tests
 import loader_tests
 import parser_tests
+import resolver_tests
 import token
 
 async fn main() {
diff --git a/selfhost/src/resolver_tests.nova b/selfhost/src/resolver_tests.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1f04facf217d9953d158b41a3c95b81f4c01a5d4
--- /dev/null
+++ b/selfhost/src/resolver_tests.nova
@@ -0,0 +1,206 @@
+// N's own tests of the resolver's edges: a glob import's clashes in the
+// order declared, the method names an impl's type gives (which reach
+// symbols in 5.5), a builtin's name redefined, the fix that makes a private
+// item public, and the fix for a misspelt attribute. Each expected value is
+// what S0's resolver gives for the same source (its tests in
+// crates/nova-resolver/src/lib.rs, or `nova check --dump-resolution`); the
+// differential compares every other input, std's modules included, which
+// these leave out.
+//
+// They live in `src/`, imported by `main.nova`, for the reason
+// lexer_tests.nova gives.
+
+import ast
+import builtins
+import diagnostic
+import lexer
+import loader
+import parser
+import resolver
+import token
+
+// The program of `sources`, each `(name, source)` a module in that order,
+// resolved natively with no std module: each module's imports named as
+// S0's `name_imports` names them, by first segment among the modules'
+// names, and every other module importable.
+fn resolve_sources(sources: [(String, String)]) -> ProgramResolution {
+    let mut names: Vec<String> = Vec::new()
+    for (name, _) in sources.iter() { names.push(name) }
+    let mut modules: Vec<ModuleSource> = Vec::new()
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
+        i += 1
+    }
+    let std_set = StdSet { modules: Vec::new(), natives: Vec::new(), builtins: builtin_table() }
+    resolve_program_for(CompileTargetNative, modules, std_set)
+}
+
+// The texts, one to a line, to compare.
+fn lines(texts: [String]) -> String { "\n".join(texts) }
+
+// Each diagnostic's message, in order.
+fn messages(r: ProgramResolution) -> [String] {
+    let mut out: Vec<String> = Vec::new()
+    for d in r.diagnostics.iter() { out.push(d.message) }
+    out.to_array()
+}
+
+@test
+fn a_glob_imports_clashes_are_reported_in_declaration_order() {
+    // S0's `a_glob_imports_clashes_are_reported_in_declaration_order`:
+    // values, then types, then traits, each in the order the imported
+    // module declared them, whatever a map's order.
+    let main = "fn alpha() -> Int { 10 }\nfn beta() -> Int { 20 }\nfn gamma() -> Int { 30 }\nfn delta() -> Int { 40 }\nfn epsilon() -> Int { 50 }\nrecord Zeta { y: Int }\nrecord Eta { y: Int }\ntrait Tau {}\ntrait Sigma {}\nimport lib\nfn main() { }\n"
+    let lib = "pub fn alpha() -> Int { 1 }\npub fn beta() -> Int { 2 }\npub fn gamma() -> Int { 3 }\npub fn delta() -> Int { 4 }\npub fn epsilon() -> Int { 5 }\npub record Zeta { x: Int }\npub record Eta { x: Int }\npub trait Tau {}\npub trait Sigma {}\n"
+    let r = resolve_sources([("main", main), ("lib", lib)])
+    assert_eq(
+        lines(messages(r)),
+        lines(
+            [
+                "`alpha` is already defined or imported in this module",
+                "`beta` is already defined or imported in this module",
+                "`gamma` is already defined or imported in this module",
+                "`delta` is already defined or imported in this module",
+                "`epsilon` is already defined or imported in this module",
+                "type `Zeta` is already defined or imported in this module",
+                "type `Eta` is already defined or imported in this module",
+                "trait `Tau` is already defined or imported in this module",
+                "trait `Sigma` is already defined or imported in this module",
+            ],
+        ),
+    )
+    // Each at the import's path.
+    let label = r.diagnostics.at(0).labels.at(0)
+    assert_eq(label.file, "main")
+    assert_eq(label.span.start, 206)
+    assert_eq(label.message, "conflicting import")
+}
+
+@test
+fn an_impls_type_names_its_methods() {
+    // `type_full_name`: a path's last segment and its arguments after `_`,
+    // `Arr_` for an array, `TupN_` for a tuple; a trait's method is
+    // `Self.Trait.method`, a default body `Trait::method$default`; an
+    // associated type is named after itself.
+    let source = "record P { x: Int }\ntrait Show {\n    type Item\n    fn twice(self) {}\n    fn shown(self) -> Int\n}\nimpl Show for P {\n    fn twice(self) {}\n}\nimpl<T> Option<T> {\n    fn is_it(self) {}\n}\nimpl Box<(Int, Float)> {\n    fn get(self) {}\n}\nimpl Wrap<[Int]> {\n    fn first(self) {}\n}\nfn main() {}\n"
+    let r = resolve_sources([("defs", source)])
+    let mut names: Vec<String> = Vec::new()
+    for d in r.definitions.defs.iter() { names.push(d.name) }
+    assert_eq(
+        lines(names.to_array()),
+        lines(
+            [
+                "P",
+                "Show",
+                "Item",
+                "Show::twice$" + "default",
+                "P.Show.twice",
+                "Option_T.is_it",
+                "Box_Tup2_Int_Float.get",
+                "Wrap_Arr_Int.first",
+                "main",
+            ],
+        ),
+    )
+    // The default body is the trait item's second, the impl's method its
+    // first.
+    match r.definitions.defs.at(3).kind {
+        DefKindMethod(item, method, MethodOwnerTraitDefault) => {
+            assert_eq(item, 1)
+            assert_eq(method, 1)
+        }
+        _ => assert(false, "unreachable")
+    }
+    match r.definitions.defs.at(2).kind {
+        DefKindAssocType(t) => assert_eq(t, 1)
+        _ => assert(false, "unreachable")
+    }
+}
+
+@test
+fn a_builtins_name_redefined_is_e0002_with_a_note() {
+    // A module's scope holds the global builtins before its items, so the
+    // first definition of `println` is the builtin: a note, not a
+    // secondary label. A std-only builtin's name is free outside std.
+    let r = resolve_sources([("main", "fn println() {}\nfn str_cmp() {}\nfn main() {}\n")])
+    assert_eq(lines(messages(r)), "duplicate definition of `println`")
+    let d = r.diagnostics.at(0)
+    assert_eq(d.labels.len(), 1)
+    assert_eq(lines(d.notes.to_array()), "`println` is a compiler builtin")
+}
+
+@test
+fn importing_a_private_item_offers_pub_where_its_keyword_starts() {
+    // `make_public`: `pub ` before each private item of the name, at its
+    // keyword (`async` for an `async fn`), in the imported module's file.
+    let lib = "async fn secret() -> Int { 2 }\nrecord secret2 {}\n"
+    let r = resolve_sources(
+        [("main", "import lib::{secret, secret2, nothing}\nfn main() {}\n"), ("lib", lib)],
+    )
+    assert_eq(
+        lines(messages(r)),
+        lines(
+            [
+                "`secret` is not a public item of module `lib`",
+                "`secret2` is not a public item of module `lib`",
+                "`nothing` is not a public item of module `lib`",
+            ],
+        ),
+    )
+    let fix = r.diagnostics.at(0).fixes.at(0)
+    assert_eq(fix.title, "make `secret` public in `lib`")
+    assert_eq(fix.edits.at(0).file, "lib")
+    assert_eq(fix.edits.at(0).span.start, 0)
+    assert_eq(fix.edits.at(0).span.end, 0)
+    assert_eq(fix.edits.at(0).text, "pub ")
+    assert_eq(r.diagnostics.at(1).fixes.at(0).edits.at(0).span.start, 31)
+    assert(r.diagnostics.at(2).fixes.is_empty(), "no fix")
+}
+
+@test
+fn an_unknown_attribute_on_a_function_offers_the_nearest_known_one() {
+    // On a function an unknown attribute's E0082 carries a fix when a known
+    // attribute is near (`suggest::change_to`); elsewhere it never does.
+    let r = resolve_sources(
+        [("main", "@tset\nfn main() {}\n@inline\nfn f() {}\n@tset\nrecord R {}\n")],
+    )
+    assert_eq(r.diagnostics.len(), 3)
+    let fix = r.diagnostics.at(0).fixes.at(0)
+    assert_eq(fix.title, "change `tset` to `test`")
+    assert_eq(fix.edits.at(0).span.start, 1)
+    assert_eq(fix.edits.at(0).span.end, 5)
+    assert(r.diagnostics.at(1).fixes.is_empty(), "no fix")
+    assert(r.diagnostics.at(2).fixes.is_empty(), "no fix")
+    // `suggest::closest`'s edges: a name equal ignoring case at any
+    // distance, the least distance, then byte order; a third of the length.
+    assert_eq(closest("HTTPCLIENT", ["HTTPCLIENTS", "HttpClient"]).unwrap_or("-"), "HttpClient")
+    assert_eq(closest("ab", ["ac", "aa"]).unwrap_or("-"), "aa")
+    assert_eq(closest("abcde", ["axxde"]).unwrap_or("-"), "-")
+    assert_eq(distance("cuont", "count"), 1)
+}
````

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index 9f0571ce206b22ad914a3acf7961bd7de900769e..fe9c3bb965eed0038e7cfe5e9ebb1476ee575d57 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -1,6 +1,9 @@
-// N, the self-hosted Nova compiler, so far its lexer and its parser (spec
+// N, the self-hosted Nova compiler, so far its lexer, its parser, its
+// loader of loose programs and its resolver (spec
 // docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md
-// §3.2 and §4.3, ADR 0051 and ADR 0052).
+// §3.2 and §4.3, ADR 0051 and ADR 0052; spec
+// docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
+// §3.4, ADR 0053).
 //
 //   selfhost tokens FILE...   prints each file's token dump, after a header
 //                             line `== FILE`, as `nova parse --dump-tokens`
@@ -8,26 +11,44 @@
 //   selfhost ast FILE...      prints each file's AST dump, its tree and its
 //                             errors, after the same header, as
 //                             `nova parse --dump-ast` does
+//   selfhost resolve --std DIR FILE...
+//                             prints the resolution dump of each program
+//                             whose root is FILE, after the same header,
+//                             as `nova check --dump-resolution` does, with
+//                             std's modules read from DIR once
+//   selfhost resolve --std DIR --std-part
+//                             prints std's part, as `nova check
+//                             --dump-resolution --std` does
 //
 // A file with lex or parse errors is dumped with them, and the exit code
 // stays 0. A file that cannot be read prints `IOERR` and its kind after its
-// header; the rest are dumped, then the exit code is 2.
+// header; the rest are dumped, then the exit code is 2. No file is refused
+// with the exit code 2 and nothing printed.
 
 import ast_dump
+import builtins
+import diagnostic
 import dump
 import lexer
 import lexer_tests
+import loader
 import loader_tests
 import parser_tests
+import resolution_dump
+import resolver
 import resolver_tests
 import token
 
 async fn main() {
     let argv = args()
+    let command = if argv.len() < 2 { "" } else { argv.at(1) }
+    if command == "resolve" {
+        resolve_command(argv).await
+        return
+    }
     // No file is refused, as S0's clap refuses it.
-    let command = if argv.len() < 3 { "" } else { argv.at(1) }
     let tokens = command == "tokens"
-    if !tokens && command != "ast" {
+    if argv.len() < 3 || (!tokens && command != "ast") {
         eprintln("usage: selfhost tokens|ast FILE...")
         exit(2)
         return
@@ -51,6 +72,112 @@ async fn main() {
     if failed { exit(2) }
 }
 
+// `selfhost resolve --std DIR (--std-part | FILE...)`: std's modules are
+// read from DIR and parsed once, then std's part, or each program's part
+// after its header. Without `--std DIR`, or with neither `--std-part` nor
+// a file, or with both, nothing is printed and the exit code is 2, as S0's
+// `check --dump-resolution` gives.
+async fn resolve_command(argv: Vec<String>) {
+    let mut std_dir = ""
+    let mut std_part = false
+    let mut files: Vec<String> = Vec::new()
+    let mut i = 2
+    while i < argv.len() {
+        let a = argv.at(i)
+        if a == "--std" && i + 1 < argv.len() {
+            std_dir = argv.at(i + 1)
+            i += 2
+            continue
+        }
+        if a == "--std-part" { std_part = true } else { files.push(a) }
+        i += 1
+    }
+    if std_dir == "" || std_part == !files.is_empty() {
+        eprintln("usage: selfhost resolve --std DIR (--std-part | FILE...)")
+        exit(2)
+        return
+    }
+    let target = CompileTargetNative
+    let modules = match load_std(std_dir, target).await {
+        Ok(s) => s
+        Err(message) => {
+            eprintln(message)
+            exit(2)
+            return
+        }
+    }
+    // Natively the target's std modules are `STD_MODULES`.
+    let std_set = StdSet { modules: modules, natives: modules, builtins: builtin_table() }
+    if std_part {
+        let resolved = resolve_program_for(target, Vec::new(), std_set)
+        print(dump_std(target, resolved, std_set.builtins))
+        return
+    }
+    let mut failed = false
+    for path in files.iter() {
+        match dump_resolution(path, target, std_set).await {
+            Ok(out) => print("== ${path}\n${out}")
+            Err(e) => {
+                print("== ${path}\nIOERR ${io_kind_name(e.kind)}\n")
+                failed = true
+            }
+        }
+    }
+    if failed { exit(2) }
+}
+
+// The target's std modules, each read from under `dir` and parsed once,
+// in order, labelled as the driver registers them (`<std/core>`).
+async fn load_std(dir: String, target: CompileTarget) -> Result<Vec<StdModule>, String> {
+    let mut out: Vec<StdModule> = Vec::new()
+    for (name, rel) in std_modules(target).iter() {
+        let path = "${dir}/${rel}"
+        match read_to_string(path).await {
+            Ok(source) => out.push(std_module(name, std_label(name), source))
+            Err(e) => return Err("error: cannot read std's ${path}: ${io_kind_name(e.kind)}")
+        }
+    }
+    Ok(out)
+}
+
+// The resolution dump of the program whose root is `path`
+// (nova-driver's `dump_resolution`): `check`'s pipeline up to the
+// resolver. It loads the program, stops there on any error, strips the
+// `@test` functions and resolves; the dump holds what was built and every
+// diagnostic so far. A root directly in a package's `src/` or `tests/` is
+// `UNSUPPORTED package`, 5.2b's. `Err` when the root cannot be read.
+async fn dump_resolution(
+    path: String,
+    target: CompileTarget,
+    std_set: StdSet,
+) -> Result<String, IoError> {
+    if in_package(path).await { return Ok("UNSUPPORTED package\n") }
+    let load = load_program(path).await?
+    let modules = load.modules
+    let mut diagnostics = load.diagnostics
+    let mut failed = false
+    for d in diagnostics.iter() { if d.is_error() { failed = true } }
+    if !failed { strip_test_functions(modules) }
+    let mut sources: Vec<ModuleSource> = Vec::new()
+    for m in modules.iter() {
+        sources.push(
+            ModuleSource {
+                name: m.name,
+                label: m.name,
+                file: m.ast,
+                imports: m.imports,
+                text: Some(m.text),
+                package: -1,
+                importable: m.importable,
+            },
+        )
+    }
+    if failed { return Ok(dump_program(target, sources, None, diagnostics, std_set.builtins)) }
+    let resolved = resolve_program_for(target, sources, std_set)
+    for d in resolved.diagnostics.iter() { diagnostics.push(d) }
+    Ok(dump_program(target, sources, Some(resolved), diagnostics, std_set.builtins))
+}
+
 // The kind's name, as S0's dump prints it.
 fn io_kind_name(kind: IoErrorKind) -> String {
     match kind {
diff --git a/selfhost/src/resolution_dump.nova b/selfhost/src/resolution_dump.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9b7d44d1228eb87874183b89fc5cc6a2f4012445
--- /dev/null
+++ b/selfhost/src/resolution_dump.nova
@@ -0,0 +1,455 @@
+// The resolution dump: what the loader and the resolver built for one
+// program, as text, one entry per line, exactly as S0's `nova check
+// --dump-resolution` prints it: a port of crates/nova-resolver/src/dump.rs,
+// function by function (spec
+// docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
+// §3.4). ADR 0053 writes the grammar out, with an example of every form.
+//
+// - Two parts: std's, printed once (`dump_std`), and a program's
+//   (`dump_program`), which prints only what is the program's own.
+// - A program's own definition is `u<n>`, a std one `s<n>`, its id less the
+//   first std definition's; modules and items are numbered from 0 in each
+//   part.
+// - Namespaces print sorted by name (a `Map`'s keys, sorted), exports in
+//   the order they were exported.
+// - Strings are quoted as the AST dump quotes them (dump.nova's
+//   `push_escaped`).
+
+import ast
+import builtins
+import diagnostic
+import dump
+import loader
+import resolver
+import token
+
+/// A program's part for `target`: its `modules` as the loader gave them,
+/// its resolution once it got that far, and every diagnostic, the
+/// loader's and the resolver's, in order (`dump_program`); `table` names
+/// the builtins.
+pub fn dump_program(
+    target: CompileTarget,
+    modules: Vec<ModuleSource>,
+    resolution: Option<ProgramResolution>,
+    diagnostics: Vec<Diagnostic>,
+    table: BuiltinTable,
+) -> String {
+    let mut d = ResDump {
+        out: StringBuilder::new(),
+        std_def: 9223372036854775807,
+        fresh: true,
+        variants: table.variants,
+    }
+    d.open(0, "Program")
+    d.field("target", compile_target_name(target))
+    d.bool_field("resolved", resolution.is_some())
+    let mut index = 0
+    for m in modules.iter() {
+        d.open(1, "Module")
+        d.out.push(" ${index}")
+        d.string(m.name)
+        d.field("package", package(m.package))
+        for name in sort_strings(Vec::from_array(m.imports.keys())).iter() {
+            d.open(2, "Import")
+            d.string(name)
+            match m.imports.get(name).unwrap() {
+                ImportTargetModule(t) => d.out.push(" ${t}")
+                ImportTargetReported => d.out.push(" Reported")
+            }
+            d.close()
+        }
+        for (name, t) in m.importable.iter() {
+            d.open(2, "Importable")
+            d.string(name)
+            d.out.push(" ${t}")
+            d.close()
+        }
+        d.close()
+        index += 1
+    }
+    match resolution {
+        Some(r) => {
+            let defs = r.definitions
+            let std_module = defs.std_start
+            let std_item = first_item_of(defs, std_module)
+            let std_def = first_std_def(defs, std_item)
+            d.std_def = std_def
+            d.open(1, "StdStart")
+            d.field("module", "${std_module}")
+            d.field("def", "${std_def}")
+            d.field("item", "${std_item}")
+            d.close()
+            d.defs(defs, 0, std_def, 0)
+            d.scopes(defs, 0, std_module, 0)
+            d.exports(defs, 0, std_module, 0)
+            d.items(defs, 0, std_item, 0, 0)
+            for (file, split) in r.newline_splits.iter() {
+                d.open(1, "Split")
+                d.place(file, split.at)
+                d.field("kind", split_kind_word(split.kind))
+                d.close()
+            }
+            for test in r.tests.iter() {
+                d.open(1, "Test")
+                d.out.push(" ${d.id(test.def_id)}")
+                d.string(test.name)
+                d.bool_field("should_panic", test.should_panic)
+                d.close()
+            }
+        }
+        None => ()
+    }
+    d.close()
+    d.newline()
+    d.diagnostics(diagnostics)
+    d.out.to_string()
+}
+
+/// std's part for `target`, from the resolution of a program of no module
+/// of its own (`dump_std`).
+pub fn dump_std(
+    target: CompileTarget,
+    resolution: ProgramResolution,
+    table: BuiltinTable,
+) -> String {
+    let mut d = ResDump {
+        out: StringBuilder::new(),
+        std_def: 0,
+        fresh: true,
+        variants: table.variants,
+    }
+    d.std_part(target, resolution.definitions, 0, 0, 0)
+    d.newline()
+    d.diagnostics(resolution.diagnostics)
+    d.out.to_string()
+}
+
+// The index of the first merged item of a module from `std_module` on.
+fn first_item_of(defs: Definitions, std_module: Int) -> Int {
+    let mut i = 0
+    while i < defs.item_module.len() {
+        if defs.item_module.at(i) >= std_module { return i }
+        i += 1
+    }
+    defs.item_module.len()
+}
+
+/// The first std definition's index: the first definition of an item of a
+/// std module; an associated type, which names no item, is passed over
+/// (`first_std_def`).
+fn first_std_def(defs: Definitions, std_item: Int) -> Int {
+    let mut i = 0
+    while i < defs.defs.len() {
+        let item = item_index_of(defs.defs.at(i).kind)
+        if item >= 0 && item >= std_item { return i }
+        i += 1
+    }
+    defs.defs.len()
+}
+
+// A package key as the dump prints it (`package`).
+fn package(key: Int) -> String { if key < 0 { "None" } else { "${key}" } }
+
+// A newline split's kind, as Rust's `Debug` prints it.
+fn split_kind_word(kind: SplitKind) -> String {
+    match kind {
+        SplitKindCall => "Call"
+        SplitKindIndex => "Index"
+        SplitKindMinus => "Minus"
+        SplitKindValue => "Value"
+    }
+}
+
+/// The line kind of a definition, one per `DefKind` variant (`def_kind`).
+pub fn def_kind(kind: DefKind) -> String {
+    match kind {
+        DefKindFn(_) => "Def.Fn"
+        DefKindSum(_, _) => "Def.Sum"
+        DefKindRecord(_) => "Def.Record"
+        DefKindConst(_) => "Def.Const"
+        DefKindTrait(_) => "Def.Trait"
+        DefKindMethod(_, _, _) => "Def.Method"
+        DefKindExternFn(_, _) => "Def.ExternFn"
+        DefKindAssocType(_) => "Def.AssocType"
+    }
+}
+
+/// The line kind of a value's resolution, one per `Res` variant
+/// (`res_kind`).
+pub fn res_kind(res: Res) -> String {
+    match res {
+        ResDef(_) => "Value.Def"
+        ResVariant(_, _) => "Value.Variant"
+        ResBuiltin(_) => "Value.Builtin"
+    }
+}
+
+/// The line kind of a diagnostic, one per `Severity` variant
+/// (`severity_kind`): a diagnostic of severity `Note` is
+/// `NoteDiagnostic`, apart from a diagnostic's own note.
+pub fn severity_kind(severity: Severity) -> String {
+    match severity {
+        SeverityError => "Error"
+        SeverityWarning => "Warning"
+        SeverityNote => "NoteDiagnostic"
+        SeverityHelp => "Help"
+    }
+}
+
+// The text being written, the first std definition's id, from which ids
+// print as `s<n>`, whether the text is empty or ends a line (`Dump`), and
+// the builtins' Rust variant names.
+record ResDump { out: StringBuilder, std_def: Int, fresh: Bool, variants: [String] }
+
+impl ResDump {
+    // A line, up to its kind (`open`).
+    fn open(mut self, depth: Int, kind: String) {
+        if !self.fresh { self.out.push("\n") }
+        self.fresh = false
+        let mut i = 0
+        while i < depth {
+            self.out.push("  ")
+            i += 1
+        }
+        self.out.push("(")
+        self.out.push(kind)
+    }
+
+    fn close(mut self) { self.out.push(")") }
+
+    // A line break, after which the next line needs none (`push('\n')`).
+    fn newline(mut self) {
+        self.out.push("\n")
+        self.fresh = true
+    }
+
+    // ` key=value`.
+    fn field(mut self, key: String, value: String) { self.out.push(" ${key}=${value}") }
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
+    // ` ` and a place: the file's label quoted, `:`, the span.
+    fn place(mut self, file: String, span: Span) {
+        self.string(file)
+        self.out.push(":${span.start}..${span.end}")
+    }
+
+    // A definition's id: `u<n>` for the program's own, `s<n>` for std's.
+    fn id(self, id: Int) -> String {
+        if id >= self.std_def { "s${id - self.std_def}" } else { "u${id}" }
+    }
+
+    // std's part: its modules, definitions, namespaces, exports, items
+    // and `missing`, each numbered from std's first (`std_part`).
+    fn std_part(
+        mut self,
+        target: CompileTarget,
+        defs: Definitions,
+        std_module: Int,
+        std_def: Int,
+        std_item: Int,
+    ) {
+        self.open(0, "Std")
+        self.field("target", compile_target_name(target))
+        self.field("modules", "${defs.modules.len() - std_module}")
+        self.field("defs", "${defs.defs.len() - std_def}")
+        self.field("items", "${defs.item_module.len() - std_item}")
+        let mut index = 0
+        while std_module + index < defs.modules.len() {
+            let m = defs.modules.at(std_module + index)
+            self.open(1, "Module")
+            self.out.push(" ${index}")
+            self.string(m.name)
+            self.field("package", package(defs.packages.at(std_module + index)))
+            for (name, t) in defs.importable.at(std_module + index).iter() {
+                self.open(2, "Importable")
+                self.string(name)
+                self.out.push(" ${t - std_module}")
+                self.close()
+            }
+            self.close()
+            index += 1
+        }
+        self.defs(defs, std_def, defs.defs.len(), std_item)
+        self.scopes(defs, std_module, defs.modules.len(), std_module)
+        self.exports(defs, std_module, defs.modules.len(), std_module)
+        self.items(defs, std_item, defs.item_module.len(), std_item, std_module)
+        for name in sort_strings(Vec::from_array(defs.missing.keys())).iter() {
+            self.open(1, "Missing")
+            self.string(name)
+            self.string(defs.missing.get(name).unwrap())
+            self.close()
+        }
+        self.close()
+    }
+
+    // The definitions `start..end`, their items numbered from `first_item`
+    // (`defs`).
+    fn defs(mut self, defs: Definitions, start: Int, end: Int, first_item: Int) {
+        let mut i = start
+        while i < end {
+            let def = defs.defs.at(i)
+            self.open(1, def_kind(def.kind))
+            self.out.push(" ${self.id(i)}")
+            self.string(def.name)
+            self.place(def.file, def.span)
+            match def.kind {
+                DefKindFn(item) => self.field("item", "${item - first_item}")
+                DefKindRecord(item) => self.field("item", "${item - first_item}")
+                DefKindConst(item) => self.field("item", "${item - first_item}")
+                DefKindTrait(item) => self.field("item", "${item - first_item}")
+                DefKindSum(item, variants) => {
+                    self.field("item", "${item - first_item}")
+                    for v in variants.iter() {
+                        self.open(2, "Variant")
+                        self.string(v.name)
+                        self.place(v.file, v.span)
+                        self.field("arity", "${v.arity}")
+                        self.close()
+                    }
+                }
+                DefKindMethod(item, method_index, owner) => {
+                    self.field("item", "${item - first_item}")
+                    self.field("method", "${method_index}")
+                    let o = match owner {
+                        MethodOwnerImpl => "Impl"
+                        MethodOwnerTraitDefault => "TraitDefault"
+                    }
+                    self.field("owner", o)
+                }
+                DefKindExternFn(item, fn_index) => {
+                    self.field("item", "${item - first_item}")
+                    self.field("fn", "${fn_index}")
+                }
+                DefKindAssocType(trait_def) => self.field("trait", self.id(trait_def))
+            }
+            self.close()
+            i += 1
+        }
+    }
+
+    // Each module of `start..end`'s three namespaces, sorted by name, the
+    // modules numbered from `first` (`scopes`).
+    fn scopes(mut self, defs: Definitions, start: Int, end: Int, first: Int) {
+        let mut mid = start
+        while mid < end {
+            let scope = defs.modules.at(mid)
+            self.open(1, "Scope")
+            self.out.push(" ${mid - first}")
+            for name in sort_strings(Vec::from_array(scope.values.keys())).iter() {
+                self.value(name, scope.values.get(name).unwrap())
+            }
+            for name in sort_strings(Vec::from_array(scope.types.keys())).iter() {
+                self.binding("Type", name, scope.types.get(name).unwrap())
+            }
+            for name in sort_strings(Vec::from_array(scope.traits.keys())).iter() {
+                self.binding("Trait", name, scope.traits.get(name).unwrap())
+            }
+            self.close()
+            mid += 1
+        }
+    }
+
+    // Each module of `start..end`'s exports, in the order they were
+    // exported: values, then types, then traits (`exports`).
+    fn exports(mut self, defs: Definitions, start: Int, end: Int, first: Int) {
+        let mut mid = start
+        while mid < end {
+            let e = defs.exports.at(mid)
+            self.open(1, "Exports")
+            self.out.push(" ${mid - first}")
+            for name in e.value_names.iter() { self.value(name, e.value(name)) }
+            for name in e.type_names.iter() { self.binding("Type", name, e.type_of(name)) }
+            for name in e.trait_names.iter() { self.binding("Trait", name, e.trait_of(name)) }
+            self.close()
+            mid += 1
+        }
+    }
+
+    // A value's line: its name and what it resolves to (`value`).
+    fn value(mut self, name: String, res: Res) {
+        self.open(2, res_kind(res))
+        self.string(name)
+        match res {
+            ResDef(id) => self.out.push(" ${self.id(id)}")
+            ResVariant(sum, index) => self.out.push(" ${self.id(sum)} ${index}")
+            ResBuiltin(b) => self.out.push(" ${self.variants[b]}")
+        }
+        self.close()
+    }
+
+    // A type's or a trait's line: its name and its definition (`binding`).
+    fn binding(mut self, kind: String, name: String, id: Int) {
+        self.open(2, kind)
+        self.string(name)
+        self.out.push(" ${self.id(id)}")
+        self.close()
+    }
+
+    // The merged items `start..end` as runs of one module, `(Items
+    // start..end module)`, the items numbered from `first_item` and the
+    // modules from `first_module` (`items`).
+    fn items(
+        mut self,
+        defs: Definitions,
+        start: Int,
+        end: Int,
+        first_item: Int,
+        first_module: Int,
+    ) {
+        let mut s = start
+        while s < end {
+            let owner = defs.item_module.at(s)
+            let mut e = s + 1
+            while e < end && defs.item_module.at(e) == owner { e += 1 }
+            self.open(1, "Items")
+            self.out.push(" ${s - first_item}..${e - first_item} ${owner - first_module}")
+            self.close()
+            s = e
+        }
+    }
+
+    // `(Diagnostics`, each diagnostic in order, `)`: its severity's kind,
+    // code and message, then its labels in order, notes and fixes
+    // (`diagnostics`).
+    fn diagnostics(mut self, diagnostics: Vec<Diagnostic>) {
+        self.open(0, "Diagnostics")
+        for diag in diagnostics.iter() {
+            self.open(1, severity_kind(diag.severity))
+            self.string(diag.code)
+            self.string(diag.message)
+            for label in diag.labels.iter() {
+                self.open(2, if label.primary { "Primary" } else { "Secondary" })
+                self.place(label.file, label.span)
+                self.string(label.message)
+                self.close()
+            }
+            for note in diag.notes.iter() {
+                self.open(2, "Note")
+                self.string(note)
+                self.close()
+            }
+            for fix in diag.fixes.iter() {
+                self.open(2, "Fix")
+                self.string(fix.title)
+                for edit in fix.edits.iter() {
+                    self.open(3, "Edit")
+                    self.place(edit.file, edit.span)
+                    self.string(edit.text)
+                    self.close()
+                }
+                self.close()
+            }
+            self.close()
+        }
+        self.close()
+        self.newline()
+    }
+}
diff --git a/selfhost/src/resolver.nova b/selfhost/src/resolver.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4e9ebcb716ac2c4b924dcc40d7d6add225d2aee8
--- /dev/null
+++ b/selfhost/src/resolver.nova
@@ -0,0 +1,1486 @@
+// The item-level resolver: a port of crates/nova-resolver/src/lib.rs's
+// `resolve_program_for` and everything it calls, function by function and
+// with the same names, so that the two read side by side (spec
+// docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
+// §3.3, ADR 0053). It builds what S0's builds, the definitions and their
+// ids and names, each module's three namespaces, the exports, the tests
+// and the merged item list, and gives every diagnostic in S0's order; the
+// resolution dump (resolution_dump.nova) prints it, and the differential
+// in crates/nova-cli/tests/selfhost.rs holds it equal to S0's. A change to
+// how names resolve changes both resolvers in one pull request.
+//
+// The builtin table and the std tables are builtins.nova's. The Rust's
+// idioms, translated one way everywhere (ADR 0053 lists them):
+//
+// - `DefId`, `ModuleId` and `Option<u32>` package keys are `Int`s, -1 for
+//   none; `Builtin` is its index in `BUILTIN_VARIANTS`.
+// - A span carries no file in N, so each function that makes a definition
+//   or a diagnostic takes the label of its module's file, `file`, beside
+//   the spans of its tree.
+// - `FxHashMap`s looked up by name are `Map`s, never iterated into output;
+//   `Exports` keeps each namespace's names in the order they were exported
+//   (S0 since 5.2a), and every loop over exports follows those.
+// - `IndexMap<String, Span>` (the first definitions) is a `Map`: it is only
+//   looked up.
+// - A record is a reference, so `&mut definitions.modules[mid]` is
+//   `definitions.modules.at(mid)`, mutated through a `let mut`.
+// - std's modules are parsed once per process by the caller
+//   (`std_module`), not once per program: `resolve_program_for` takes them
+//   parsed, with their diagnostics, which it gives first, as S0's
+//   `resolve_program_with_std` takes `parse_std`'s.
+// - The language server's import occurrences (`imports`) are not built:
+//   the dump does not print them, and the server stays Rust.
+
+import ast
+import builtins
+import diagnostic
+import lexer
+import loader
+import parser
+import token
+
+// ---------------------------------------------------------------------------
+// The vocabulary
+// ---------------------------------------------------------------------------
+
+/// What a name in the value namespace resolves to (`Res`): a definition, a
+/// sum's variant (the sum and the variant's index) or a builtin.
+pub type Res = | ResDef(Int) | ResVariant(Int, Int) | ResBuiltin(Int)
+
+/// Who owns a method's body (`MethodOwner`).
+pub type MethodOwner = | MethodOwnerImpl | MethodOwnerTraitDefault
+
+/// A sum's variant (`VariantDef`).
+pub record VariantDef { pub name: String, pub file: String, pub span: Span, pub arity: Int }
+
+/// What a definition is (`DefKind`).
+pub type DefKind =
+    // item_index
+    | DefKindFn(Int)
+    // item_index, variants
+    | DefKindSum(Int, Vec<VariantDef>)
+    // item_index
+    | DefKindRecord(Int)
+    // item_index
+    | DefKindConst(Int)
+    // item_index
+    | DefKindTrait(Int)
+    // item_index, method_index, owner
+    | DefKindMethod(Int, Int, MethodOwner)
+    // item_index, fn_index
+    | DefKindExternFn(Int, Int)
+    // trait_def
+    | DefKindAssocType(Int)
+
+/// A definition (`Def`): its name, mangled for a method, where it is, and
+/// what it is.
+pub record Def { pub name: String, pub file: String, pub span: Span, pub kind: DefKind }
+
+/// One module's three namespaces (`ModuleScope`).
+pub record ModuleScope {
+    pub name: String,
+    pub values: Map<String, Res>,
+    pub types: Map<String, Int>,
+    pub traits: Map<String, Int>,
+}
+
+/// A module's public names (`Exports`), each namespace's names also in the
+/// order they were exported, which a glob import follows.
+pub record Exports {
+    pub values: Map<String, Res>,
+    pub types: Map<String, Int>,
+    pub traits: Map<String, Int>,
+    pub value_names: Vec<String>,
+    pub type_names: Vec<String>,
+    pub trait_names: Vec<String>,
+}
+
+/// Everything resolution built (`Definitions`).
+pub record Definitions {
+    pub defs: Vec<Def>,
+    pub modules: Vec<ModuleScope>,
+    pub item_module: Vec<Int>,
+    pub exports: Vec<Exports>,
+    pub importable: Vec<Vec<(String, Int)>>,
+    pub packages: Vec<Int>,
+    pub std_start: Int,
+    pub target: CompileTarget,
+    pub missing: Map<String, String>,
+}
+
+/// One module handed to the resolver (`ModuleSource`): its name, its file's
+/// label, its tree, what each import names by first segment, its source
+/// (for a fix), its package key (-1 for none) and what it could import.
+pub record ModuleSource {
+    pub name: String,
+    pub label: String,
+    pub file: File,
+    pub imports: Map<String, ImportTarget>,
+    pub text: Option<String>,
+    pub package: Int,
+    pub importable: Vec<(String, Int)>,
+}
+
+/// A module of no import table, source or package (`ModuleSource::new`).
+pub fn module_source(name: String, label: String, file: File) -> ModuleSource {
+    ModuleSource {
+        name: name,
+        label: label,
+        file: file,
+        imports: Map::new(),
+        text: None,
+        package: -1,
+        importable: Vec::new(),
+    }
+}
+
+/// A `@test` function (`TestFn`).
+pub record TestFn { pub def_id: Int, pub name: String, pub should_panic: Bool }
+
+/// A std module, parsed once per process (`std_module`'s result): its
+/// name, its file's label, its tree (`None` when it failed to parse) and
+/// its lex and parse diagnostics.
+pub record StdModule {
+    pub name: String,
+    pub label: String,
+    pub file: Option<File>,
+    pub diagnostics: Vec<Diagnostic>,
+}
+
+/// What every program's resolution reads, made once per process: the
+/// target's std modules, parsed, `STD_MODULES` parsed (which only the WASM
+/// target's `missing` reads), and the builtin table.
+pub record StdSet {
+    pub modules: Vec<StdModule>,
+    pub natives: Vec<StdModule>,
+    pub builtins: BuiltinTable,
+}
+
+/// What `resolve_program_for` returns (`ProgramResolution`): the merged
+/// items, the user modules' newline splits with their files' labels, the
+/// definitions, the diagnostics and the tests.
+pub record ProgramResolution {
+    pub items: Vec<Spanned<Item>>,
+    pub newline_splits: Vec<(String, NewlineSplit)>,
+    pub definitions: Definitions,
+    pub diagnostics: Vec<Diagnostic>,
+    pub tests: Vec<TestFn>,
+}
+
+impl Exports {
+    /// No names yet (`Exports::default`).
+    pub fn new() -> Exports {
+        Exports {
+            values: Map::new(),
+            types: Map::new(),
+            traits: Map::new(),
+            value_names: Vec::new(),
+            type_names: Vec::new(),
+            trait_names: Vec::new(),
+        }
+    }
+
+    /// Export `name` as a value (`export_value`).
+    pub fn export_value(mut self, name: String, res: Res) {
+        if self.values.insert(name, res).is_none() { self.value_names.push(name) }
+    }
+
+    /// Export `name` as a type (`export_type`).
+    pub fn export_type(mut self, name: String, id: Int) {
+        if self.types.insert(name, id).is_none() { self.type_names.push(name) }
+    }
+
+    /// Export `name` as a trait (`export_trait`).
+    pub fn export_trait(mut self, name: String, id: Int) {
+        if self.traits.insert(name, id).is_none() { self.trait_names.push(name) }
+    }
+
+    /// The exported values, in the order they were exported
+    /// (`values_in_order`).
+    pub fn values_in_order(self) -> Vec<(String, Res)> {
+        let mut out: Vec<(String, Res)> = Vec::new()
+        for n in self.value_names.iter() { out.push((n, self.value(n))) }
+        out
+    }
+
+    /// The exported types, in the order they were exported
+    /// (`types_in_order`).
+    pub fn types_in_order(self) -> Vec<(String, Int)> {
+        let mut out: Vec<(String, Int)> = Vec::new()
+        for n in self.type_names.iter() { out.push((n, self.type_of(n))) }
+        out
+    }
+
+    /// The exported traits, in the order they were exported
+    /// (`traits_in_order`).
+    pub fn traits_in_order(self) -> Vec<(String, Int)> {
+        let mut out: Vec<(String, Int)> = Vec::new()
+        for n in self.trait_names.iter() { out.push((n, self.trait_of(n))) }
+        out
+    }
+
+    /// What the exported value `name` resolves to; it is exported.
+    pub fn value(self, name: String) -> Res { self.values.get(name).unwrap() }
+
+    /// The exported type `name`; it is exported.
+    pub fn type_of(self, name: String) -> Int { self.types.get(name).unwrap() }
+
+    /// The exported trait `name`; it is exported.
+    pub fn trait_of(self, name: String) -> Int { self.traits.get(name).unwrap() }
+}
+
+/// A scope of no name yet, `ModuleScope { name, ..Default::default() }`.
+fn new_scope(name: String) -> ModuleScope {
+    ModuleScope { name: name, values: Map::new(), types: Map::new(), traits: Map::new() }
+}
+
+/// `push_def`: the new definition's id is its index.
+fn push_def(mut defs: Vec<Def>, def: Def) -> Int {
+    let id = defs.len()
+    defs.push(def)
+    id
+}
+
+/// A definition's item index; an associated type, which has none, gives
+/// -1.
+pub fn item_index_of(kind: DefKind) -> Int {
+    match kind {
+        DefKindFn(i) => i
+        DefKindSum(i, _) => i
+        DefKindRecord(i) => i
+        DefKindConst(i) => i
+        DefKindTrait(i) => i
+        DefKindMethod(i, _, _) => i
+        DefKindExternFn(i, _) => i
+        DefKindAssocType(_) => -1
+    }
+}
+
+// ---------------------------------------------------------------------------
+// std's modules
+// ---------------------------------------------------------------------------
+
+/// Lex and parse a std module's `source`, whose file is labelled `label`
+/// (`std_module`): its tree and its diagnostics, L0001 for each lex error
+/// and the parser's code for each parse error, each with its notes.
+pub fn std_module(name: String, label: String, source: String) -> StdModule {
+    let (tokens, lex_errors) = lex(source)
+    let mut diags: Vec<Diagnostic> = Vec::new()
+    for e in lex_errors.iter() { diags.push(error_at("L0001", "${e}", label, e.span(), "here")) }
+    let (ast, parse_errors) = parse(tokens)
+    for e in parse_errors.iter() {
+        let mut d = error_at(e.code(), "${e}", label, e.span(), "here")
+        for n in e.notes().iter() { d.add_note(n) }
+        diags.push(d)
+    }
+    StdModule { name: name, label: label, file: Some(ast), diagnostics: diags }
+}
+
+/// A std module's file label, as the driver registers it: `$std.core` is
+/// `<std/core>`.
+pub fn std_label(name: String) -> String {
+    let short = if name.starts_with("$std.") { name.slice(5, name.len()) } else { name }
+    "<std/${short}>"
+}
+
+// ---------------------------------------------------------------------------
+// resolve_program_for
+// ---------------------------------------------------------------------------
+
+/// Resolve the program of `modules`, in load order, for `target`, with
+/// `std_set`'s std modules and builtins (`resolve_program_for`).
+pub fn resolve_program_for(
+    target: CompileTarget,
+    modules: Vec<ModuleSource>,
+    std_set: StdSet,
+) -> ProgramResolution {
+    let std = std_set.modules
+    let table = std_set.builtins
+    let mut definitions = Definitions {
+        defs: Vec::new(),
+        modules: Vec::new(),
+        item_module: Vec::new(),
+        exports: Vec::new(),
+        importable: Vec::new(),
+        packages: Vec::new(),
+        std_start: 0,
+        target: target,
+        missing: Map::new(),
+    }
+    let mut diagnostics: Vec<Diagnostic> = Vec::new()
+    let mut merged: Vec<Spanned<Item>> = Vec::new()
+    let mut tests: Vec<TestFn> = Vec::new()
+
+    // std's parse diagnostics first, as S0 gives them parsing each std
+    // module; a module that failed to parse is skipped.
+    for s in std.iter() { for d in s.diagnostics.iter() { diagnostics.push(d) } }
+
+    // std's modules go after the user's.
+    let user_count = modules.len()
+    let mut all: Vec<ModuleSource> = Vec::new()
+    for m in modules.iter() { all.push(m) }
+    for s in std.iter() {
+        match s.file {
+            Some(file) => all.push(module_source(s.name, s.label, file))
+            None => ()
+        }
+    }
+    let std_start = user_count
+
+    let mut exports: Vec<Exports> = Vec::new()
+
+    // A scope per module, seeded with the builtins: `GLOBAL` everywhere,
+    // `STD_ONLY` (and on WASM `WASM_ONLY`) in std's alone.
+    let mut mid = 0
+    while mid < all.len() {
+        let mut scope = new_scope(all.at(mid).name)
+        seed(scope, table, table.global)
+        if mid >= std_start {
+            seed(scope, table, table.std_only)
+            match target {
+                CompileTargetWasm => seed(scope, table, table.wasm_only)
+                CompileTargetNative => ()
+            }
+        }
+        definitions.modules.push(scope)
+        exports.push(Exports::new())
+        mid += 1
+    }
+
+    // Pass 1: each module's own definitions into its scope and exports.
+    mid = 0
+    while mid < all.len() {
+        let m = all.at(mid)
+        let first_value: Map<String, Span> = Map::new()
+        let first_type: Map<String, Span> = Map::new()
+        let before = diagnostics.len()
+        for item in m.file.items.iter() {
+            let item_index = merged.len()
+            definitions.item_module.push(mid)
+            collect_item(
+                definitions.defs,
+                definitions.modules.at(mid),
+                exports.at(mid),
+                first_value,
+                first_type,
+                diagnostics,
+                tests,
+                item_index,
+                item.value,
+                m.label,
+            )
+            merged.push(item)
+        }
+        if !owned(all, std_start, mid) { clear_fixes(diagnostics, before) }
+        mid += 1
+    }
+
+    // A WASM build links no C library, so an `extern` block can name
+    // nothing (E0090).
+    match target {
+        CompileTargetWasm => {
+            mid = 0
+            while mid < std_start {
+                let m = all.at(mid)
+                for item in m.file.items.iter() {
+                    match item.value {
+                        ItemExtern(_) => {
+                            diagnostics.push(
+                                error_at(
+                                    "E0090",
+                                    "extern functions are not available on target wasm",
+                                    m.label,
+                                    item.span,
+                                    "this `extern` block",
+                                ),
+                            )
+                        }
+                        _ => ()
+                    }
+                }
+                mid += 1
+            }
+        }
+        CompileTargetNative => ()
+    }
+
+    // Pass 2: the imports, binding other modules' public names.
+    let mut public_fixes: Vec<(Int, Int, String)> = Vec::new()
+    mid = 0
+    while mid < all.len() {
+        let m = all.at(mid)
+        let before = diagnostics.len()
+        for item in m.file.items.iter() {
+            match item.value {
+                ItemImport(imp) => resolve_import(
+                    definitions,
+                    exports,
+                    all,
+                    target,
+                    mid,
+                    imp,
+                    diagnostics,
+                    public_fixes,
+                )
+                _ => ()
+            }
+        }
+        if !owned(all, std_start, mid) { clear_fixes(diagnostics, before) }
+        mid += 1
+    }
+
+    // Making an item public must not bind its name into a module that
+    // already has it.
+    for (at, fix_target, name) in public_fixes.iter() {
+        let mut clash = false
+        let mut g = 0
+        while g < all.len() && !clash {
+            if g != fix_target
+                && would_receive(all.at(g), fix_target, name)
+                && binds(definitions.modules.at(g), name) {
+                clash = true
+            }
+            g += 1
+        }
+        if clash {
+            let mut d = diagnostics.at(at)
+            d.fixes = Vec::new()
+        }
+    }
+
+    // std's glob, last and lowest-priority: each std module's public names
+    // into every other module, but an imported std module's into none and
+    // `STD_INTERNAL`'s into std's alone.
+    mid = std_start
+    while mid < exports.len() {
+        let name = all.at(mid).name
+        if !is_imported_std_module(name) {
+            let from = if holds(STD_INTERNAL, name) { std_start } else { 0 }
+            import_std_module(definitions, exports.at(mid), mid, from)
+        }
+        mid += 1
+    }
+
+    // E0102: a variant of a program's own sum that hides one of std's.
+    for d in variant_shadowing(definitions, exports, all, std_start).iter() { diagnostics.push(d) }
+
+    // What a fix needs to know.
+    for m in all.iter() {
+        definitions.importable.push(m.importable)
+        definitions.packages.push(m.package)
+    }
+    definitions.std_start = std_start
+    definitions.exports = exports
+    definitions.missing = missing_names(target, std_set.natives)
+
+    // The newline rule's splits of the user's modules.
+    let mut splits: Vec<(String, NewlineSplit)> = Vec::new()
+    mid = 0
+    while mid < std_start && mid < all.len() {
+        let m = all.at(mid)
+        for s in m.file.newline_splits.iter() { splits.push((m.label, s)) }
+        mid += 1
+    }
+    ProgramResolution {
+        items: merged,
+        newline_splits: splits,
+        definitions: definitions,
+        diagnostics: diagnostics,
+        tests: tests,
+    }
+}
+
+// Bind each builtin of `builtins` by its name in `scope`'s values.
+fn seed(mut scope: ModuleScope, table: BuiltinTable, builtins: Vec<Int>) {
+    for b in builtins.iter() { let _ = scope.values.insert(table.names[b], ResBuiltin(b)) }
+}
+
+// Whether a fix may edit module `mid`: the root package's or a loose
+// program's (`owned` in `resolve_program_for`).
+fn owned(all: Vec<ModuleSource>, std_start: Int, mid: Int) -> Bool {
+    let package = all.at(mid).package
+    mid < std_start && (package == -1 || package == 0)
+}
+
+// Drop the fixes of the diagnostics from `before` on.
+fn clear_fixes(diagnostics: Vec<Diagnostic>, before: Int) {
+    let mut i = before
+    while i < diagnostics.len() {
+        let mut d = diagnostics.at(i)
+        d.fixes = Vec::new()
+        i += 1
+    }
+}
+
+/// Each variant std brings into every module, by name, with its sum's name:
+/// the first std module's to export it wins (`std_globbed_variants`).
+fn std_globbed_variants(
+    definitions: Definitions,
+    exports: Vec<Exports>,
+    all: Vec<ModuleSource>,
+    std_start: Int,
+) -> Map<String, String> {
+    let mut out: Map<String, String> = Map::new()
+    let mut mid = std_start
+    while mid < exports.len() {
+        let name = all.at(mid).name
+        if !is_imported_std_module(name) && !holds(STD_INTERNAL, name) {
+            let e = exports.at(mid)
+            for variant in e.value_names.iter() {
+                match e.value(variant) {
+                    ResVariant(sum, _) => {
+                        if !out.contains_key(variant) {
+                            let _ = out.insert(variant, definitions.defs.at(sum).name)
+                        }
+                    }
+                    _ => ()
+                }
+            }
+        }
+        mid += 1
+    }
+    out
+}
+
+/// E0102: each variant of a user module's own sum that hides a variant std
+/// brings into every module (`variant_shadowing`).
+fn variant_shadowing(
+    definitions: Definitions,
+    exports: Vec<Exports>,
+    all: Vec<ModuleSource>,
+    std_start: Int,
+) -> Vec<Diagnostic> {
+    let std_variants = std_globbed_variants(definitions, exports, all, std_start)
+    let mut out: Vec<Diagnostic> = Vec::new()
+    let mut mid = 0
+    while mid < std_start && mid < all.len() {
+        let m = all.at(mid)
+        for item in m.file.items.iter() {
+            match item.value {
+                ItemType(t) => {
+                    match t.def {
+                        TypeDefSum(variants) => {
+                            for v in variants.iter() {
+                                match std_variants.get(v.name.value) {
+                                    Some(sum) => {
+                                        let name = v.name.value
+                                        let mut d = Diagnostic::warning(
+                                            "E0102",
+                                            "variant `${name}` hides std's `${sum}::${name}`",
+                                        )
+                                        d.add_primary(m.label, v.name.span, "this variant")
+                                        d.add_note(
+                                            "in this module the bare name `${name}` now means this variant; std's is `${sum}::${name}`",
+                                        )
+                                        out.push(d)
+                                    }
+                                    None => ()
+                                }
+                            }
+                        }
+                        _ => ()
+                    }
+                }
+                _ => ()
+            }
+        }
+        mid += 1
+    }
+    out
+}
+
+/// The names of the std modules `target` does not compile, each with its
+/// module, for a message that says which (`missing_names`). Natively every
+/// std module compiles, so there are none; `natives` are `STD_MODULES`
+/// parsed.
+fn missing_names(target: CompileTarget, natives: Vec<StdModule>) -> Map<String, String> {
+    let compiled = std_modules(target)
+    let mut missing: Map<String, String> = Map::new()
+    for s in natives.iter() {
+        let mut skip = is_imported_std_module(s.name)
+        for (n, _) in compiled.iter() { if n == s.name { skip = true } }
+        if skip { continue }
+        let file = match s.file {
+            Some(f) => f
+            None => continue
+        }
+        let exp = Exports::new()
+        let mut i = 0
+        for item in file.items.iter() {
+            collect_item(
+                Vec::new(),
+                new_scope(""),
+                exp,
+                Map::new(),
+                Map::new(),
+                Vec::new(),
+                Vec::new(),
+                i,
+                item.value,
+                s.label,
+            )
+            i += 1
+        }
+        for n in exp.value_names.iter() {
+            if !missing.contains_key(n) { let _ = missing.insert(n, s.name) }
+        }
+        for n in exp.type_names.iter() {
+            if !missing.contains_key(n) { let _ = missing.insert(n, s.name) }
+        }
+        for n in exp.trait_names.iter() {
+            if !missing.contains_key(n) { let _ = missing.insert(n, s.name) }
+        }
+    }
+    missing
+}
+
+/// Bind std module `std_mid`'s exports into every other module from
+/// `from` on, where the name is not bound already (`import_std_module`).
+fn import_std_module(definitions: Definitions, std_exports: Exports, std_mid: Int, from: Int) {
+    let values = std_exports.values_in_order()
+    let types = std_exports.types_in_order()
+    let traits = std_exports.traits_in_order()
+    let mut mid = from
+    while mid < definitions.modules.len() {
+        if mid != std_mid {
+            let mut scope = definitions.modules.at(mid)
+            for (n, r) in values.iter() {
+                if !scope.values.contains_key(n) { let _ = scope.values.insert(n, r) }
+            }
+            for (n, id) in types.iter() {
+                if !scope.types.contains_key(n) { let _ = scope.types.insert(n, id) }
+            }
+            for (n, id) in traits.iter() {
+                if !scope.traits.contains_key(n) { let _ = scope.traits.insert(n, id) }
+            }
+        }
+        mid += 1
+    }
+}
+
+// ---------------------------------------------------------------------------
+// Pass 1: collect_item
+// ---------------------------------------------------------------------------
+
+/// Collect one item, the merged item `item_index` of the module whose
+/// scope, exports and first definitions are given and whose file is
+/// labelled `file` (`collect_item`).
+fn collect_item(
+    mut defs: Vec<Def>,
+    mut scope: ModuleScope,
+    mut exp: Exports,
+    mut first_value: Map<String, Span>,
+    mut first_type: Map<String, Span>,
+    mut diagnostics: Vec<Diagnostic>,
+    mut tests: Vec<TestFn>,
+    item_index: Int,
+    item: Item,
+    file: String,
+) {
+    match item {
+        ItemFunction(f) => {
+            let name = f.name.value
+            let span = f.name.span
+            let id = push_def(
+                defs,
+                Def { name: name, file: file, span: span, kind: DefKindFn(item_index) },
+            )
+            insert_value(
+                scope,
+                exp,
+                first_value,
+                diagnostics,
+                name,
+                span,
+                ResDef(id),
+                is_pub(f.vis),
+                file,
+            )
+            validate_test_function(f, id, diagnostics, tests, file)
+        }
+        ItemType(t) => {
+            validate_attrs_reject_test(t.attrs, "a type declaration", diagnostics, file)
+            match t.def {
+                TypeDefSum(variants) => {
+                    let name = t.name.value
+                    let span = t.name.span
+                    if reject_reserved_type_name(diagnostics, name, span, file) { return }
+                    let pubv = is_pub(t.vis)
+                    let mut variant_defs: Vec<VariantDef> = Vec::new()
+                    for v in variants.iter() {
+                        variant_defs.push(
+                            VariantDef {
+                                name: v.name.value,
+                                file: file,
+                                span: v.name.span,
+                                arity: v.fields.len(),
+                            },
+                        )
+                    }
+                    let id = push_def(
+                        defs,
+                        Def {
+                            name: name,
+                            file: file,
+                            span: span,
+                            kind: DefKindSum(item_index, variant_defs),
+                        },
+                    )
+                    insert_type(scope, exp, first_type, diagnostics, name, span, id, pubv, file)
+                    // Variants live in the value namespace and take the
+                    // type's visibility.
+                    let mut vi = 0
+                    for v in variants.iter() {
+                        insert_value(
+                            scope,
+                            exp,
+                            first_value,
+                            diagnostics,
+                            v.name.value,
+                            v.name.span,
+                            ResVariant(id, vi),
+                            pubv,
+                            file,
+                        )
+                        vi += 1
+                    }
+                }
+                TypeDefAlias(_) => {
+                    diagnostics.push(
+                        unsupported(
+                            file,
+                            t.name.span,
+                            "type aliases are not supported yet in the Phase 1 compiler",
+                        ),
+                    )
+                }
+            }
+        }
+        ItemConst(c) => {
+            validate_attrs_reject_test(c.attrs, "a const", diagnostics, file)
+            let name = c.name.value
+            let span = c.name.span
+            let id = push_def(
+                defs,
+                Def { name: name, file: file, span: span, kind: DefKindConst(item_index) },
+            )
+            insert_value(
+                scope,
+                exp,
+                first_value,
+                diagnostics,
+                name,
+                span,
+                ResDef(id),
+                is_pub(c.vis),
+                file,
+            )
+        }
+        ItemRecord(r) => {
+            validate_attrs_reject_test(r.attrs, "a record", diagnostics, file)
+            let name = r.name.value
+            let span = r.name.span
+            if reject_reserved_type_name(diagnostics, name, span, file) { return }
+            let id = push_def(
+                defs,
+                Def { name: name, file: file, span: span, kind: DefKindRecord(item_index) },
+            )
+            insert_type(scope, exp, first_type, diagnostics, name, span, id, is_pub(r.vis), file)
+        }
+        ItemTrait(t) => {
+            validate_attrs_reject_test(t.attrs, "a trait", diagnostics, file)
+            let name = t.name.value
+            let span = t.name.span
+            let pubv = is_pub(t.vis)
+            let id = push_def(
+                defs,
+                Def { name: name, file: file, span: span, kind: DefKindTrait(item_index) },
+            )
+            if scope.traits.contains_key(name) {
+                diagnostics.push(
+                    error_at("E0002", "duplicate trait `${name}`", file, span, "redefined here"),
+                )
+            } else {
+                let _ = scope.traits.insert(name, id)
+                if pubv { exp.export_trait(name, id) }
+            }
+            // Default-method bodies become method definitions, and each
+            // associated type a definition of its own.
+            let mut method_index = 0
+            for ti in t.items.iter() {
+                match ti {
+                    TraitItemProvided(f) => {
+                        let _ = push_def(
+                            defs,
+                            Def {
+                                name: "${t.name.value}::${f.name.value}$" + "default",
+                                file: file,
+                                span: f.name.span,
+                                kind: DefKindMethod(
+                                    item_index,
+                                    method_index,
+                                    MethodOwnerTraitDefault,
+                                ),
+                            },
+                        )
+                    }
+                    TraitItemAssocType(_, at_name, _) => {
+                        let _ = push_def(
+                            defs,
+                            Def {
+                                name: at_name.value,
+                                file: file,
+                                span: at_name.span,
+                                kind: DefKindAssocType(id),
+                            },
+                        )
+                    }
+                    TraitItemRequired(_) => ()
+                }
+                method_index += 1
+            }
+        }
+        ItemImpl(i) => {
+            validate_attrs_reject_test(i.attrs, "an impl block", diagnostics, file)
+            // Impls are bound into no scope.
+            let self_name = type_full_name(i.ty.value)
+            let mut method_index = 0
+            for f in i.functions.iter() {
+                let mangled = match i.trait_ {
+                    Some(tr) => "${self_name}.${path_tail(tr.value)}.${f.name.value}"
+                    None => "${self_name}.${f.name.value}"
+                }
+                let _ = push_def(
+                    defs,
+                    Def {
+                        name: mangled,
+                        file: file,
+                        span: f.name.span,
+                        kind: DefKindMethod(item_index, method_index, MethodOwnerImpl),
+                    },
+                )
+                method_index += 1
+            }
+        }
+        ItemExtern(block) => {
+            validate_attrs_reject_test(block.attrs, "an extern block", diagnostics, file)
+            // Each declaration binds as a private value under its C name.
+            let mut fn_index = 0
+            for ext_item in block.items.iter() {
+                match ext_item {
+                    ExternItemFn(sig) => {
+                        let name = sig.name.value
+                        let span = sig.name.span
+                        let id = push_def(
+                            defs,
+                            Def {
+                                name: name,
+                                file: file,
+                                span: span,
+                                kind: DefKindExternFn(item_index, fn_index),
+                            },
+                        )
+                        insert_value(
+                            scope,
+                            exp,
+                            first_value,
+                            diagnostics,
+                            name,
+                            span,
+                            ResDef(id),
+                            false,
+                            file,
+                        )
+                    }
+                }
+                fn_index += 1
+            }
+        }
+        ItemImport(imp) => {
+            // Otherwise pass 2's.
+            validate_attrs_reject_test(imp.attrs, "an import", diagnostics, file)
+        }
+        ItemModule(m) => validate_attrs_reject_test(
+            m.attrs,
+            "a module declaration",
+            diagnostics,
+            file,
+        )
+    }
+}
+
+/// E0089 when `name` is a built-in type's (`reject_reserved_type_name`):
+/// whether it was.
+fn reject_reserved_type_name(
+    mut diagnostics: Vec<Diagnostic>,
+    name: String,
+    span: Span,
+    file: String,
+) -> Bool {
+    if !holds(RESERVED_TYPE_NAMES, name) { return false }
+    diagnostics.push(
+        error_at(
+            "E0089",
+            "`${name}` is already a built-in type; a type declared under this name could never be referred to in a type annotation, since `${name}` in a type position never resolves to a declaration under this name",
+            file,
+            span,
+            "redeclares a built-in type name",
+        ),
+    )
+    true
+}
+
+/// The attributes the compiler knows (`KNOWN_ATTRIBUTES`).
+const KNOWN_ATTRIBUTES: [String] = ["test"]
+
+/// The arguments `@test` takes (`KNOWN_TEST_ARGS`).
+const KNOWN_TEST_ARGS: [String] = ["should_panic"]
+
+/// E0084's note on an `async` test (`TEST_ASYNC_NOTE`).
+const TEST_ASYNC_NOTE: String = "an `async fn` returns a future, and a `@test` function's result is discarded rather than awaited, so an `async` test's body would never run; write the test as a synchronous one that drives the future itself, e.g. `@test fn t() { assert_eq(block_on(f()), 42) }` — `block_on` runs `f()`'s future to completion and produces its value"
+
+/// E0082 for `attr` (`unknown_attribute`).
+fn unknown_attribute(attr: Attribute, file: String) -> Diagnostic {
+    error_at(
+        "E0082",
+        "unknown attribute `@${attr.name.value}`; known attributes are: ${", ".join(KNOWN_ATTRIBUTES)}",
+        file,
+        attr.name.span,
+        "unknown attribute",
+    )
+}
+
+/// E0083 for a `@test` and E0082 for an unknown attribute on an item that
+/// is not a function, `kind` (`validate_attrs_reject_test`).
+fn validate_attrs_reject_test(
+    attrs: Vec<Attribute>,
+    kind: String,
+    mut diagnostics: Vec<Diagnostic>,
+    file: String,
+) {
+    for attr in attrs.iter() {
+        if attr.name.value == "test" {
+            diagnostics.push(
+                error_at(
+                    "E0083",
+                    "`@test` can only be applied to a function, not ${kind}",
+                    file,
+                    attr.name.span,
+                    "not a function",
+                ),
+            )
+        } else if !holds(KNOWN_ATTRIBUTES, attr.name.value) {
+            diagnostics.push(unknown_attribute(attr, file))
+        }
+    }
+}
+
+/// A function's attributes: E0082 with a fix for an unknown one, and for
+/// `@test`, E0085 for an unknown argument and E0084 for a shape a test may
+/// not have; a well-formed test is collected once
+/// (`validate_test_function`).
+fn validate_test_function(
+    f: Function,
+    def_id: Int,
+    mut diagnostics: Vec<Diagnostic>,
+    mut tests: Vec<TestFn>,
+    file: String,
+) {
+    let mut collected = false
+    for attr in f.attrs.iter() {
+        if attr.name.value != "test" {
+            let mut d = unknown_attribute(attr, file)
+            match change_to(file, attr.name, KNOWN_ATTRIBUTES) {
+                Some(fix) => d.add_fix(fix)
+                None => ()
+            }
+            diagnostics.push(d)
+            continue
+        }
+        let mut well_formed = true
+        let mut should_panic = false
+        for arg in attr.args.iter() {
+            if arg.value == "should_panic" {
+                should_panic = true
+            } else {
+                let mut d = error_at(
+                    "E0085",
+                    "unknown `@test` argument `${arg.value}`; the accepted arguments are: ${", ".join(KNOWN_TEST_ARGS)}",
+                    file,
+                    arg.span,
+                    "unknown argument",
+                )
+                match change_to(file, arg, KNOWN_TEST_ARGS) {
+                    Some(fix) => d.add_fix(fix)
+                    None => ()
+                }
+                diagnostics.push(d)
+                well_formed = false
+            }
+        }
+        let violations = test_shape_violations(f)
+        if !violations.is_empty() {
+            let mut names: Vec<String> = Vec::new()
+            for (name, _, _) in violations.iter() { names.push(name) }
+            let mut diag = error_at(
+                "E0084",
+                "a `@test` function may not have ${", ".join(names.to_array())}",
+                file,
+                f.name.span,
+                "invalid test signature",
+            )
+            for (name, has_span, span) in violations.iter() {
+                if has_span { diag.add_secondary(file, span, "${name} here") }
+            }
+            if f.is_async { diag.add_note(TEST_ASYNC_NOTE) }
+            diagnostics.push(diag)
+            well_formed = false
+        }
+        if well_formed && !collected {
+            tests.push(TestFn { def_id: def_id, name: f.name.value, should_panic: should_panic })
+            collected = true
+        }
+    }
+}
+
+/// What a `@test` function may not have, each with whether it has a span
+/// and the span (`test_shape_violations`).
+fn test_shape_violations(f: Function) -> Vec<(String, Bool, Span)> {
+    let mut violations: Vec<(String, Bool, Span)> = Vec::new()
+    if f.is_async { violations.push(("an `async` body", false, Span::point(0))) }
+    if !f.params.is_empty() {
+        violations.push(
+            ("parameters", true, f.params.at(0).name.span.merge(f.params.last().ty.span)),
+        )
+    }
+    if !f.generics.is_empty() {
+        violations.push(
+            (
+                "generic parameters",
+                true,
+                f.generics.at(0).name.span.merge(f.generics.last().name.span),
+            ),
+        )
+    }
+    match f.return_ty {
+        Some(rt) => {
+            let unit = match rt.value {
+                TypeTuple(items) => items.is_empty()
+                _ => false
+            }
+            if !unit { violations.push(("a non-Unit return type", true, rt.span)) }
+        }
+        None => ()
+    }
+    violations
+}
+
+// ---------------------------------------------------------------------------
+// Pass 2: resolve_import
+// ---------------------------------------------------------------------------
+
+/// Resolve module `mid`'s `imp` (`resolve_import`): find the module it
+/// names, then bind its public names, every one for a glob, or each listed
+/// one, reporting what it cannot.
+fn resolve_import(
+    definitions: Definitions,
+    exports: Vec<Exports>,
+    modules: Vec<ModuleSource>,
+    compile_target: CompileTarget,
+    mid: Int,
+    imp: Import,
+    mut diagnostics: Vec<Diagnostic>,
+    mut public_fixes: Vec<(Int, Int, String)>,
+) {
+    let file = modules.at(mid).label
+    let span = imp.path.span
+    let segments = imp.path.value.segments
+    let mut parts: Vec<String> = Vec::new()
+    for s in segments.iter() { parts.push(s.value) }
+    let joined = "::".join(parts.to_array())
+    // Only single-segment imports, and std's nested ones (`ui::html`).
+    if segments.len() > 1 && !is_imported_std_path(joined) {
+        let first = segments.at(0).value
+        if is_imported_std_path(first) {
+            diagnostics.push(
+                error_at("E0001", "cannot find module `${joined}`", file, span, "no such module"),
+            )
+        } else {
+            diagnostics.push(
+                unsupported(
+                    file,
+                    span,
+                    "qualified or nested import paths (`a::b`) are not supported yet; import a top-level module by its name",
+                ),
+            )
+        }
+        return
+    }
+    let target_name = joined
+    let target = match modules.at(mid).imports.get(target_name) {
+        Some(ImportTargetModule(t)) => t
+        Some(ImportTargetReported) => return
+        None => {
+            // An imported std module: the target's, or E0092.
+            if is_imported_std_path(target_name) {
+                let std_name = imported_std_module(target_name)
+                let found = module_named(modules, std_name)
+                if found < 0 {
+                    let t = compile_target_name(compile_target)
+                    diagnostics.push(
+                        error_at(
+                            "E0092",
+                            "`${target_name}` is a std module, which target ${t} does not have",
+                            file,
+                            span,
+                            "not on target ${t}",
+                        ),
+                    )
+                    return
+                }
+                found
+            } else {
+                diagnostics.push(
+                    error_at(
+                        "E0001",
+                        "cannot find module `${target_name}`",
+                        file,
+                        span,
+                        "no such module",
+                    ),
+                )
+                return
+            }
+        }
+    }
+    if target == mid {
+        diagnostics.push(
+            error_at("E0001", "module `${target_name}` imports itself", file, span, "self-import"),
+        )
+        return
+    }
+    let mut scope = definitions.modules.at(mid)
+    let e = exports.at(target)
+    match imp.kind {
+        ImportKindSimple => {
+            // A glob: every public name, in the order declared.
+            for (n, r) in e.values_in_order().iter() {
+                bind_value(scope, diagnostics, n, span, r, file)
+            }
+            for (n, id) in e.types_in_order().iter() {
+                bind_type(scope, diagnostics, n, span, id, file)
+            }
+            for (n, id) in e.traits_in_order().iter() {
+                bind_trait(scope, diagnostics, n, span, id, file)
+            }
+        }
+        ImportKindList(names) => {
+            for n in names.iter() {
+                let name = n.value
+                let mut found = false
+                match e.values.get(name) {
+                    Some(r) => {
+                        bind_value(scope, diagnostics, name, n.span, r, file)
+                        found = true
+                    }
+                    None => ()
+                }
+                match e.types.get(name) {
+                    Some(id) => {
+                        bind_type(scope, diagnostics, name, n.span, id, file)
+                        found = true
+                    }
+                    None => ()
+                }
+                match e.traits.get(name) {
+                    Some(id) => {
+                        bind_trait(scope, diagnostics, name, n.span, id, file)
+                        found = true
+                    }
+                    None => ()
+                }
+                if !found {
+                    let mut diag = error_at(
+                        "E0001",
+                        "`${name}` is not a public item of module `${target_name}`",
+                        file,
+                        n.span,
+                        "not found or not `pub`",
+                    )
+                    let same_package = modules.at(mid).package == modules.at(target).package
+                    match make_public(modules.at(target), name, target_name, same_package) {
+                        Some(fix) => {
+                            diag.add_fix(fix)
+                            public_fixes.push((diagnostics.len(), target, name))
+                        }
+                        None => ()
+                    }
+                    diagnostics.push(diag)
+                }
+            }
+        }
+        ImportKindAlias(_) => diagnostics.push(
+            unsupported(file, span, "`import ... as` aliases are not supported yet"),
+        )
+    }
+}
+
+// The index of the module named `name`, or -1.
+fn module_named(modules: Vec<ModuleSource>, name: String) -> Int {
+    let mut i = 0
+    while i < modules.len() {
+        if modules.at(i).name == name { return i }
+        i += 1
+    }
+    -1
+}
+
+/// Whether a visibility is `pub` (`is_pub`).
+fn is_pub(vis: Visibility) -> Bool {
+    match vis {
+        VisibilityPub => true
+        VisibilityPrivate => false
+    }
+}
+
+/// A fix that makes `name` public in `module`, which `import` names: `pub `
+/// before each private item of that name, when the module is of the same
+/// package and its source is known (`make_public`).
+fn make_public(
+    m: ModuleSource,
+    name: String,
+    import_path: String,
+    same_package: Bool,
+) -> Option<Fix> {
+    if !same_package { return None }
+    let text = m.text?
+    let mut edits: Vec<Edit> = Vec::new()
+    for item in m.file.items.iter() {
+        let (vis, item_name) = match item.value {
+            ItemFunction(f) => (f.vis, f.name)
+            ItemRecord(r) => (r.vis, r.name)
+            ItemType(t) => (t.vis, t.name)
+            ItemTrait(t) => (t.vis, t.name)
+            ItemConst(c) => (c.vis, c.name)
+            _ => continue
+        }
+        if item_name.value != name || is_pub(vis) { continue }
+        let at = keyword_start(text, item_name.span.start)?
+        edits.push(Edit::insert(at, m.label, "pub "))
+    }
+    if edits.is_empty() {
+        None
+    } else {
+        Some(Fix { title: "make `${name}` public in `${import_path}`", edits: edits })
+    }
+}
+
+/// Whether `module` imports module `target` whole, or `name` from it
+/// (`would_receive`).
+fn would_receive(m: ModuleSource, target: Int, name: String) -> Bool {
+    for item in m.file.items.iter() {
+        match item.value {
+            ItemImport(imp) => {
+                if imp.path.value.segments.len() != 1 { continue }
+                let first = imp.path.value.segments.at(0).value
+                let names_target = match m.imports.get(first) {
+                    Some(ImportTargetModule(t)) => t == target
+                    _ => false
+                }
+                if !names_target { continue }
+                match imp.kind {
+                    ImportKindSimple => return true
+                    ImportKindList(names) => {
+                        for n in names.iter() { if n.value == name { return true } }
+                    }
+                    _ => ()
+                }
+            }
+            _ => ()
+        }
+    }
+    false
+}
+
+/// Whether `scope` binds `name` in any namespace (`binds`).
+fn binds(scope: ModuleScope, name: String) -> Bool {
+    scope.values.contains_key(name)
+        || scope.types.contains_key(name)
+        || scope.traits.contains_key(name)
+}
+
+/// Bind a module's own value, E0002 if it is bound (`insert_value`): with
+/// the first definition as a secondary label, or a note when the first is
+/// a builtin.
+fn insert_value(
+    mut scope: ModuleScope,
+    mut exp: Exports,
+    mut first: Map<String, Span>,
+    mut diagnostics: Vec<Diagnostic>,
+    name: String,
+    span: Span,
+    res: Res,
+    is_pub: Bool,
+    file: String,
+) {
+    if scope.values.contains_key(name) {
+        let mut diag = error_at(
+            "E0002",
+            "duplicate definition of `${name}`",
+            file,
+            span,
+            "redefined here",
+        )
+        match first.get(name) {
+            Some(prev) => diag.add_secondary(file, prev, "first defined here")
+            None => diag.add_note("`${name}` is a compiler builtin")
+        }
+        diagnostics.push(diag)
+        return
+    }
+    let _ = first.insert(name, span)
+    if is_pub { exp.export_value(name, res) }
+    let _ = scope.values.insert(name, res)
+}
+
+/// Bind a module's own type, E0002 if it is bound (`insert_type`).
+fn insert_type(
+    mut scope: ModuleScope,
+    mut exp: Exports,
+    mut first: Map<String, Span>,
+    mut diagnostics: Vec<Diagnostic>,
+    name: String,
+    span: Span,
+    id: Int,
+    is_pub: Bool,
+    file: String,
+) {
+    if scope.types.contains_key(name) {
+        let mut diag = error_at(
+            "E0002",
+            "duplicate definition of type `${name}`",
+            file,
+            span,
+            "redefined here",
+        )
+        match first.get(name) {
+            Some(prev) => diag.add_secondary(file, prev, "first defined here")
+            None => ()
+        }
+        diagnostics.push(diag)
+        return
+    }
+    let _ = first.insert(name, span)
+    if is_pub { exp.export_type(name, id) }
+    let _ = scope.types.insert(name, id)
+}
+
+/// Bind an imported value, E0002 if it is bound (`bind_value`).
+fn bind_value(
+    mut scope: ModuleScope,
+    mut diagnostics: Vec<Diagnostic>,
+    name: String,
+    span: Span,
+    res: Res,
+    file: String,
+) {
+    if scope.values.contains_key(name) {
+        diagnostics.push(
+            error_at(
+                "E0002",
+                "`${name}` is already defined or imported in this module",
+                file,
+                span,
+                "conflicting import",
+            ),
+        )
+        return
+    }
+    let _ = scope.values.insert(name, res)
+}
+
+/// Bind an imported type, E0002 if it is bound (`bind_type`).
+fn bind_type(
+    mut scope: ModuleScope,
+    mut diagnostics: Vec<Diagnostic>,
+    name: String,
+    span: Span,
+    id: Int,
+    file: String,
+) {
+    if scope.types.contains_key(name) {
+        diagnostics.push(
+            error_at(
+                "E0002",
+                "type `${name}` is already defined or imported in this module",
+                file,
+                span,
+                "conflicting import",
+            ),
+        )
+        return
+    }
+    let _ = scope.types.insert(name, id)
+}
+
+/// Bind an imported trait, E0002 if it is bound (`bind_trait`).
+fn bind_trait(
+    mut scope: ModuleScope,
+    mut diagnostics: Vec<Diagnostic>,
+    name: String,
+    span: Span,
+    id: Int,
+    file: String,
+) {
+    if scope.traits.contains_key(name) {
+        diagnostics.push(
+            error_at(
+                "E0002",
+                "trait `${name}` is already defined or imported in this module",
+                file,
+                span,
+                "conflicting import",
+            ),
+        )
+        return
+    }
+    let _ = scope.traits.insert(name, id)
+}
+
+/// E0900 at `span` (`unsupported`).
+fn unsupported(file: String, span: Span, msg: String) -> Diagnostic {
+    error_at("E0900", msg, file, span, "not supported yet")
+}
+
+/// The name an impl's type gives its methods (`type_full_name`):
+/// `Option_T`, `Arr_Int`, `Tup2_Int_Int`, or `anon`.
+fn type_full_name(ty: Type) -> String {
+    match ty {
+        TypePath(path, args) => {
+            let head = path_tail(path)
+            if args.is_empty() {
+                head
+            } else {
+                let mut parts: Vec<String> = Vec::new()
+                for a in args.iter() { parts.push(type_full_name(a.value)) }
+                "${head}_${"_".join(parts.to_array())}"
+            }
+        }
+        TypeArray(elem) => "Arr_${type_full_name(elem.value)}"
+        TypeTuple(items) => {
+            if items.is_empty() {
+                "anon"
+            } else {
+                let mut parts: Vec<String> = Vec::new()
+                for a in items.iter() { parts.push(type_full_name(a.value)) }
+                "Tup${items.len()}_${"_".join(parts.to_array())}"
+            }
+        }
+        _ => "anon"
+    }
+}
+
+/// A path's last segment, or "" (`path_tail`).
+fn path_tail(path: Path) -> String {
+    if path.segments.is_empty() { "" } else { path.segments.last().value }
+}
````

### Task 6: The resolution differential

Spec §3.5–§3.7; decisions 14, 21, 23, 25, 27, 30 and 35.

**Files:**
- Test and code: `crates/nova-cli/tests/selfhost.rs`

- [ ] **Step 1: The tests.** `patch_block t6-tests`: the S0-only tests of
  Tasks 2 and 3 become comparisons, and the comparison's tests join them:
  - the module's doc names the resolution differential;
  - the headers' test compares N on the same roots;
  - `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`: std's
    part once, the 251 loose programs and the 103 cases side by side, each
    program's part present, every form of `KINDS` there, every form
    listed, none of `LATER_KINDS`, and none of the three codes 5.2b gives
    (`LATER_RESOLVER_CODES`, decision 8);
  - `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`;
  - `a_root_spelled_any_way_names_its_siblings_alike` (decision 27);
  - `a_root_with_no_directory_part_or_in_a_package_loads_alike`: a root
    with no directory part beside a directory named `ui.nova` (E0093),
    a package's `src/main.nova` and `tests/main.nova` named from the
    package's directory, `pkg/tests/main.nova` from outside, and an empty
    root (decision 30);
  - on Linux, `a_root_whose_path_is_not_utf8_is_dumped_by_s0_and_not_found_by_n`
    (decision 29);
  - the headers' test compares a root beside a `nova.toml` that is a
    directory, and the corpus test floors how many package files it
    skips and reports the count with `eprintln!` (seen under
    `--nocapture`);
  - `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`;
  - `both_sides_refuse_no_file` and the longer search take the resolution
    dump too.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. Then `SH`. Expected: it does not compile, fifteen errors, `cannot find function` for `agree_on_resolution` (eight), `agree_on_resolve_cases` (two), `resolution` (two) and `std_resolution`, and no variant `Resolution` of `Dumps` (two)
  (decision 36).
- [ ] **Step 3: The code.** `patch_block t6-code`: `Dumps::Resolution`
  and its arm in `agree_on_inputs`; `resolution()`, `std_resolution()`,
  `std_dir()`, `leak_args()`; `agree_on_resolution`, the batches side by
  side; `agree_on_resolve_cases`, the std-named case from inside its
  directory (decision 27).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH`. Expected: `36 passed; 0 failed`.
  - `S` (4 min 55 s). Expected: `2857 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `cli tests: the resolution dumps compared`.

**Patch `t6-tests`:**

<!-- patch t6-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 93c7702e750fb4a203d81a8c0c73990fe0b6c09e..c67aaaecc9598e5f1aeed4523f7c9d7d6d665d2c 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -28,13 +28,16 @@
 //!
 //! 5.2a (spec
 //! `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
-//! §3.4–§3.5; ADR 0053): S0's hidden `nova check --dump-resolution FILE…`
+//! §3.4–§3.7; ADR 0053): S0's hidden `nova check --dump-resolution FILE…`
 //! and `--std`, absent from `--help`, with every other `check` command
-//! line parsed as before; every form of its dump in its dumps of std's part, every loose corpus
-//! program and the resolver's error cases in
+//! line parsed as before; and N's `selfhost resolve --std DIR FILE…` and
+//! `--std-part`, which must print the same resolution dump byte for byte,
+//! with the same exit codes: std's part once; every loose corpus program,
+//! each file run alone or a root with the sibling files it imports
+//! (packages are 5.2b's); the resolver's error cases in
 //! `tests/selfhost/cases/resolve/`; and the name-level fuzzer's inputs
 //! from fixed seeds, among which every resolver code `check` can give
-//! natively occurs.
+//! natively occurs. Every form of the dump appears in those dumps.
 //!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
@@ -584,8 +587,8 @@ enum Dumps {
 /// Write `inputs` to a fixed directory named after `name`, as
 /// `f00000.nova` and on, and check that both sides agree on them through
 /// `dumps`. Returns S0's output: its token dump and, with the AST dump, its
-/// AST dump after it. The fuzzer's generators all come through here,
-/// 5.1b's deep nesting included.
+/// AST dump after it; or its resolution dump. The fuzzer's generators all
+/// come through here, 5.1b's deep nesting and 5.2a's name fuzzer included.
 fn agree_on_inputs(name: &str, inputs: &[String], dumps: Dumps) -> String {
     let dir = fresh_dir(name);
     let mut files = Vec::with_capacity(inputs.len());
@@ -761,8 +764,9 @@ fn the_mutated_corpus_agrees() {
     }
 }
 
-/// `NOVA_SELFHOST_FUZZ=SEED:COUNT`: COUNT inputs of soup and COUNT mutated
-/// corpus files from SEED. Unset, as in CI, it does nothing.
+/// `NOVA_SELFHOST_FUZZ=SEED:COUNT`: COUNT inputs of soup, COUNT mutated
+/// corpus files and COUNT name-mutated loose programs from SEED. Unset, as
+/// in CI, it does nothing.
 #[test]
 fn a_longer_search_from_the_environment() {
     let Ok(spec) = std::env::var("NOVA_SELFHOST_FUZZ") else {
@@ -778,7 +782,8 @@ fn a_longer_search_from_the_environment() {
         Dumps::TokensAndAst,
     );
     // Mutations go in batches of 500, each from a seed of its own, so a long
-    // search keeps few inputs on disk at a time.
+    // search keeps few inputs on disk at a time; the name fuzzer's beside
+    // them.
     let mut done = 0;
     while done < count {
         let n = (count - done).min(500);
@@ -787,6 +792,11 @@ fn a_longer_search_from_the_environment() {
             &mutations(seed.wrapping_add(done as u64), n),
             Dumps::TokensAndAst,
         );
+        agree_on_inputs(
+            "search-names",
+            &names(seed.wrapping_add(done as u64), n),
+            Dumps::Resolution,
+        );
         done += n;
     }
 }
@@ -813,11 +823,11 @@ fn unreadable_files_are_reported_alike() {
     assert!(out.ends_with("== \nIOERR NotFound\n"), "{out}");
 }
 
-/// No file at all: both sides refuse it, print nothing and exit 2, for
-/// either dump.
+/// No file at all: both sides refuse it, print nothing on stdout and exit
+/// 2, for each dump: a resolution dump needs a root (or std's part).
 #[test]
 fn both_sides_refuse_no_file() {
-    for dump in [&TOKENS, &AST] {
+    for dump in [&TOKENS, &AST, resolution()] {
         let out = compare_batch(dump, &root(), &[], 2, &nova(), selfhost());
         assert_eq!(out, Ok(String::new()));
     }
@@ -1417,36 +1427,6 @@ fn resolve_cases_dir() -> PathBuf {
     cases_dir().join("resolve")
 }
 
-/// The roots of the resolver's error cases, relative to the repository:
-/// each file directly in [`resolve_cases_dir`], and each directory's
-/// `main.nova`, sorted.
-fn resolve_cases() -> Vec<String> {
-    let root = root();
-    let mut entries: Vec<_> = std::fs::read_dir(resolve_cases_dir())
-        .expect("read the resolver's cases")
-        .map(|e| e.expect("a directory entry").path())
-        .collect();
-    entries.sort();
-    let mut roots = Vec::new();
-    for path in entries {
-        let file = if path.is_dir() {
-            path.join("main.nova")
-        } else if path.extension().is_some_and(|e| e == "nova") {
-            path.clone()
-        } else {
-            continue;
-        };
-        assert!(file.is_file(), "{} has no main.nova", path.display());
-        let rel = file.strip_prefix(&root).expect("under the root");
-        let parts: Vec<String> = rel
-            .components()
-            .map(|c| c.as_os_str().to_string_lossy().into_owned())
-            .collect();
-        roots.push(parts.join("/"));
-    }
-    roots
-}
-
 /// The programs of the corpus that the resolution dump compares in 5.2a:
 /// every corpus file but those directly in a package's `src/` or
 /// `tests/`, which are 5.2b's.
@@ -1468,29 +1448,6 @@ fn loose_corpus() -> Vec<String> {
         .collect()
 }
 
-/// S0's resolution dump of `files`, relative to `dir`, [`BATCH`] files to
-/// a process, the processes at once, each exiting with `code`. N's is
-/// compared with it from Task 6 on.
-fn s0_resolution(dir: &Path, files: &[String], code: i32) -> String {
-    let nova = nova();
-    let outs: Vec<Result<Run, String>> = std::thread::scope(|scope| {
-        let runs: Vec<_> = files
-            .chunks(BATCH)
-            .map(|chunk| scope.spawn(|| run(&nova, RESOLUTION_S0, dir, chunk)))
-            .collect();
-        runs.into_iter()
-            .map(|r| r.join().expect("a batch's thread"))
-            .collect()
-    });
-    let mut all = String::new();
-    for out in outs {
-        let out = out.unwrap_or_else(|e| panic!("{e}"));
-        assert_eq!(out.code, Some(code), "{}", out.stderr);
-        all.push_str(&String::from_utf8_lossy(&out.stdout));
-    }
-    all
-}
-
 /// `nova check`'s help names neither hidden flag, and every `nova check`
 /// command line whose first argument after `check` is not
 /// `--dump-resolution` is parsed as before the flags (ADR 0053 §1): each
@@ -1621,7 +1578,7 @@ fn the_resolution_dump_flags_are_hidden_and_refused_as_before() {
 /// cannot be read is `IOERR` and its kind, the rest are dumped, and the
 /// exit code is 2; a root in a package is `UNSUPPORTED package`; a program
 /// that fails to load holds the diagnostics so far; `--std` prints std's
-/// part for the target.
+/// part for the target. N prints the same for the same roots.
 #[test]
 fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
     let dir = fresh_dir("resolution-headers");
@@ -1662,14 +1619,33 @@ fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
         ),
         "{stdout}"
     );
-    // A root that is not UTF-8 is `IOERR InvalidData`.
+    // N prints the same, a file that is not UTF-8 included, and a root
+    // beside a `nova.toml` that is a directory, which makes no package.
     std::fs::write(dir.join("latin1.nova"), b"fn \xe9() {}\n").expect("write latin1.nova");
-    let out = run(&nova(), RESOLUTION_S0, &dir, &["latin1.nova".to_string()])
-        .unwrap_or_else(|e| panic!("{e}"));
-    assert_eq!(out.code, Some(2), "{}", out.stderr);
-    assert_eq!(
-        String::from_utf8_lossy(&out.stdout),
-        "== latin1.nova\nIOERR InvalidData\n"
+    std::fs::create_dir_all(dir.join("dirpkg").join("nova.toml")).expect("create nova.toml/");
+    std::fs::create_dir_all(dir.join("dirpkg").join("src")).expect("create dirpkg/src");
+    std::fs::write(
+        dir.join("dirpkg").join("src").join("main.nova"),
+        "fn main() {}\n",
+    )
+    .expect("write dirpkg/src/main.nova");
+    let files = [
+        "a.nova",
+        "missing.nova",
+        "pkg/src/main.nova",
+        "b.nova",
+        "dirpkg/src/main.nova",
+        "latin1.nova",
+    ]
+    .map(String::from);
+    let out = agree_on_resolution(&dir, &files, 2);
+    assert!(
+        out.contains("== dirpkg/src/main.nova\n(Program target=native resolved=true\n"),
+        "{out}"
+    );
+    assert!(
+        out.ends_with("== latin1.nova\nIOERR InvalidData\n"),
+        "{out}"
     );
     // No root: nothing printed, and 2; nor a root with `--std`.
     for args in [&[][..], &["--std", "a.nova"]] {
@@ -1716,42 +1692,53 @@ fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
     }
 }
 
-/// Every form of the resolution dump's `KINDS` appears in S0's dump of
-/// std's part, or of some loose corpus program or resolver case (spec
-/// §3.5), every form they print is listed, and no form of `LATER_KINDS`
-/// appears, nor any of the three codes 5.2b's test mode and WASM target
-/// give; N's dumps are compared with these from Task 6 on.
+/// std's part, every loose corpus program and every resolver case give the
+/// same resolution dump on both sides, byte for byte (spec §3.5, §3.7):
+/// std's part once, then the programs, [`BATCH`] to a process. Every form
+/// of the dump's `KINDS` appears in these dumps, every form they print is
+/// listed, and no form of `LATER_KINDS` appears, nor any of the three
+/// codes 5.2b's test mode and WASM target give: the dumps are S0's and
+/// N's alike, so each holds them.
 #[test]
-fn every_resolution_form_appears_in_the_corpus_and_the_cases() {
+fn the_resolution_dumps_agree_on_std_the_corpus_and_the_cases() {
     let root = root();
+    let std = compare_batch(std_resolution(), &root, &[], 0, &nova(), selfhost())
+        .unwrap_or_else(|report| panic!("{report}"));
+    assert!(
+        std.starts_with("(Std target=native modules=21 "),
+        "{}",
+        &std[..80]
+    );
     let loose = loose_corpus();
     // 251 loose programs at 5.2a.
     assert!(loose.len() >= 240, "only {} loose programs", loose.len());
-    let cases = resolve_cases();
+    // The files in a package, which are 5.2b's, are skipped, and said to
+    // be: 66 at 5.2a.
+    let skipped = corpus().len() - loose.len();
+    eprintln!(
+        "the resolution pass skips {skipped} corpus files in a package's src/ or tests/ (5.2b's)"
+    );
+    assert!(skipped >= 60, "only {skipped} corpus files in packages");
+    // The corpus and the cases side by side.
+    let (mut dumps, (cases, count)) = std::thread::scope(|scope| {
+        let cases = scope.spawn(|| agree_on_resolve_cases(&resolve_cases_dir()));
+        let corpus = agree_on_resolution(&root, &loose, 0);
+        (corpus, cases.join().expect("the cases' thread"))
+    });
     // 103 cases at 5.2a.
-    assert!(cases.len() >= 90, "only {} resolver cases", cases.len());
-    let mut files = loose;
-    files.extend(cases);
-    let mut dumps = s0_resolution(&root, &files, 0);
+    assert!(count >= 90, "only {count} resolver cases");
+    dumps.push_str(&cases);
+    // Each root's part, so every program was compared.
     let parts = dumps.lines().filter(|l| l.starts_with("(Program ")).count();
-    assert_eq!(parts, files.len());
+    assert_eq!(parts, loose.len() + count);
     // None of the codes that only 5.2b's test mode and WASM target give
     // (decision 8), though cases hold the programs that would give them.
     let codes = codes_in(&dumps);
     for code in LATER_RESOLVER_CODES {
         assert!(!codes.contains_key(code), "{code} in {codes:?}");
     }
-    let std = run(
-        &nova(),
-        &["check", "--dump-resolution", "--std"],
-        &root,
-        &[],
-    )
-    .unwrap_or_else(|e| panic!("{e}"));
-    assert_eq!(std.code, Some(0), "{}", std.stderr);
-    dumps.push_str(&String::from_utf8_lossy(&std.stdout));
     let mut found = std::collections::HashSet::new();
-    for line in dumps.lines() {
+    for line in dumps.lines().chain(std.lines()) {
         found.extend(nova_driver::resolution_dump::forms_on(line));
     }
     let missing: Vec<&str> = nova_driver::resolution_dump::KINDS
@@ -1759,7 +1746,7 @@ fn every_resolution_form_appears_in_the_corpus_and_the_cases() {
         .copied()
         .filter(|k| !found.contains(*k))
         .collect();
-    assert!(missing.is_empty(), "no S0 dump holds {missing:?}");
+    assert!(missing.is_empty(), "no dump holds {missing:?}");
     let unlisted: Vec<&String> = found
         .iter()
         .filter(|f| !nova_driver::resolution_dump::listed(f))
@@ -1776,6 +1763,189 @@ fn every_resolution_form_appears_in_the_corpus_and_the_cases() {
     );
 }
 
+/// The resolver's cases as a Windows checkout has them ([`crlf`]): git
+/// gives no `.gitattributes` rule for `cases/`, so CI's Windows runner
+/// compares them with `\r\n`, and every system does here.
+#[test]
+fn the_resolution_dumps_agree_on_the_cases_with_windows_line_endings() {
+    let dir = fresh_dir("crlf-resolve");
+    let mut files = Vec::new();
+    nova_files(&resolve_cases_dir(), &resolve_cases_dir(), &[], &mut files);
+    for file in &files {
+        let text = std::fs::read(resolve_cases_dir().join(file)).expect("read a case");
+        let path = dir.join(file);
+        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
+        std::fs::write(&path, crlf(text)).expect("write a case");
+    }
+    let crlf_files = files
+        .iter()
+        .filter(|f| std::fs::read(dir.join(f)).is_ok_and(|t| t.contains(&b'\r')))
+        .count();
+    assert!(
+        crlf_files > 90,
+        "only {crlf_files} cases have CRLF line endings"
+    );
+    let (dumps, count) = agree_on_resolve_cases(&dir);
+    let parts = dumps.lines().filter(|l| l.starts_with("(Program ")).count();
+    assert_eq!(parts, count);
+}
+
+/// However its root is spelled, both sides form the same paths from it:
+/// E0093 names the sibling it refuses by the root's directory, as
+/// `Path::parent` gives it, joined with the sibling's file name. On Windows
+/// `Path::join` writes `\` where N writes `/`, so there only a root with no
+/// directory is compared (ADR 0053 §5). On Linux a file name that is not
+/// UTF-8 sits beside the root too, which S0's listing drops and N's keeps
+/// with U+FFFD: no import can name it, so the dumps still agree.
+#[test]
+fn a_root_spelled_any_way_names_its_siblings_alike() {
+    let dir = fresh_dir("resolution-spellings");
+    let sub = dir.join("sub");
+    std::fs::create_dir_all(&sub).expect("create sub");
+    std::fs::write(sub.join("main.nova"), "import ui\n\nfn main() {}\n").expect("write main.nova");
+    std::fs::write(sub.join("ui.nova"), "pub fn f() {}\n").expect("write ui.nova");
+    std::fs::write(sub.join("other.nova"), "import shapes\n\nfn main() {}\n")
+        .expect("write other.nova");
+    #[cfg(target_os = "linux")]
+    {
+        use std::os::unix::ffi::OsStrExt;
+        let name = std::ffi::OsStr::from_bytes(b"shapes\xff.nova");
+        std::fs::write(sub.join(name), "pub fn g() {}\n").expect("write a non-UTF-8 name");
+    }
+    let mut spellings = vec![("sub", "main.nova".to_string())];
+    if !cfg!(windows) {
+        for root in [
+            "sub/main.nova",
+            "./sub/main.nova",
+            "sub/./main.nova",
+            "sub//main.nova",
+            "./sub/.//main.nova",
+            "sub/../sub/main.nova",
+        ] {
+            spellings.push((".", root.to_string()));
+        }
+        for root in ["./main.nova", ".//main.nova", "../sub/main.nova"] {
+            spellings.push(("sub", root.to_string()));
+        }
+        spellings.push((".", sub.join("main.nova").display().to_string()));
+    }
+    // The roots of each directory in one process.
+    for cwd in [".", "sub"] {
+        let roots: Vec<String> = spellings
+            .iter()
+            .filter(|(c, _)| *c == cwd)
+            .map(|(_, root)| root.clone())
+            .collect();
+        if roots.is_empty() {
+            continue;
+        }
+        let out = agree_on_resolution(&dir.join(cwd), &roots, 0);
+        assert_eq!(
+            out.matches("(Error \"E0093\"").count(),
+            roots.len(),
+            "{out}"
+        );
+    }
+    let out = agree_on_resolution(&sub, &["other.nova".to_string()], 0);
+    assert!(
+        out.contains("(Error \"E0001\" \"cannot find module `shapes`\""),
+        "{out}"
+    );
+}
+
+/// A root with no directory part, and a package's (ADR 0053 §5): the
+/// directory both sides list for the root's siblings is then `.`, so a
+/// directory named `ui.nova` beside `main.nova` is E0093's, though it
+/// cannot be read, and a root directly in a package's `src/` or `tests/`,
+/// named from the package's directory, is `UNSUPPORTED package`, as one
+/// named from outside it is. An empty root is a file that is not there.
+#[test]
+fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
+    let dir = fresh_dir("resolution-roots");
+    std::fs::write(dir.join("main.nova"), "import ui\n\nfn main() {}\n").expect("write main.nova");
+    std::fs::create_dir_all(dir.join("ui.nova")).expect("create ui.nova/");
+    let out = agree_on_resolution(&dir, &["main.nova".to_string()], 0);
+    assert!(
+        out.contains("(Program target=native resolved=false\n"),
+        "{out}"
+    );
+    assert_eq!(out.matches("(Error \"E0093\"").count(), 1, "{out}");
+    let pkg = dir.join("pkg");
+    for sub in ["src", "tests"] {
+        std::fs::create_dir_all(pkg.join(sub)).expect("create a package's directory");
+        std::fs::write(pkg.join(sub).join("main.nova"), "fn main() {}\n").expect("write main.nova");
+    }
+    std::fs::write(pkg.join("nova.toml"), "").expect("write nova.toml");
+    let roots = ["src/main.nova", "tests/main.nova"].map(String::from);
+    let out = agree_on_resolution(&pkg, &roots, 0);
+    assert_eq!(
+        out,
+        "== src/main.nova\nUNSUPPORTED package\n== tests/main.nova\nUNSUPPORTED package\n"
+    );
+    let roots = ["pkg/tests/main.nova", ""].map(String::from);
+    let out = agree_on_resolution(&dir, &roots, 2);
+    assert_eq!(
+        out,
+        "== pkg/tests/main.nova\nUNSUPPORTED package\n== \nIOERR NotFound\n"
+    );
+}
+
+/// A module's label is its file's stem, which may hold what the dump's
+/// strings escape: a root named `q"uo\te.nova` is the module
+/// `"q\x22uo\x5cte"` on both sides. Windows refuses `"` in a file name.
+#[cfg(not(windows))]
+#[test]
+fn a_root_whose_name_needs_escapes_is_printed_alike() {
+    let dir = fresh_dir("resolution-escapes");
+    let root = "q\"uo\\te.nova".to_string();
+    std::fs::write(dir.join(&root), "fn main() {}\n").expect("write the root");
+    let out = agree_on_resolution(&dir, std::slice::from_ref(&root), 0);
+    assert!(
+        out.contains("(Module 0 \"q\\x22uo\\x5cte\" package=None)"),
+        "{out}"
+    );
+}
+
+/// A root whose path is not UTF-8 (ADR 0053 §5): S0 dumps its program,
+/// while N, whose arguments arrive as UTF-8 with U+FFFD in place of what
+/// is not, cannot open it and prints `IOERR NotFound`, exiting 2, as for
+/// 5.1's dumps (ADR 0051). No input of the differential has such a path;
+/// this pins the difference. Linux only: macOS and Windows refuse such a
+/// name.
+#[cfg(target_os = "linux")]
+#[test]
+fn a_root_whose_path_is_not_utf8_is_dumped_by_s0_and_not_found_by_n() {
+    use std::ffi::OsStr;
+    use std::os::unix::ffi::OsStrExt;
+    let dir = fresh_dir("resolution-non-utf8");
+    std::fs::create_dir(dir.join(OsStr::from_bytes(b"sub\xfe"))).expect("create sub");
+    for root in [&b"r\xff.nova"[..], b"sub\xfe/main.nova"] {
+        let root = OsStr::from_bytes(root);
+        std::fs::write(dir.join(root), "fn main() {}\n").expect("write the root");
+        let s0 = Command::new(nova())
+            .current_dir(&dir)
+            .args(RESOLUTION_S0)
+            .arg(root)
+            .output()
+            .expect("run nova check --dump-resolution");
+        let stdout = String::from_utf8_lossy(&s0.stdout);
+        assert_eq!(s0.status.code(), Some(0), "{root:?}: {stdout}");
+        assert!(
+            stdout.contains("\n(Program target=native resolved=true\n"),
+            "{root:?}: {stdout}"
+        );
+        let n = Command::new(selfhost())
+            .current_dir(&dir)
+            .args(resolution().n)
+            .arg(root)
+            .output()
+            .expect("run selfhost resolve");
+        let stdout = String::from_utf8_lossy(&n.stdout);
+        assert_eq!(n.status.code(), Some(2), "{root:?}: {stdout}");
+        assert!(stdout.ends_with("\nIOERR NotFound\n"), "{root:?}: {stdout}");
+    }
+}
+
 /// The name fuzzer's fixed seeds and counts every run checks (spec §3.5).
 const NAME_SEEDS: [u64; 3] = [301, 302, 303];
 const NAME_COUNT: usize = 50;
@@ -1848,22 +2018,20 @@ fn codes_in(dump: &str) -> std::collections::BTreeMap<String, usize> {
     codes
 }
 
-/// S0's resolution dump of each of the name fuzzer's inputs from the fixed
-/// seeds: every input reaches the resolver, and among them every resolver
-/// code `check` gives a loose native program occurs twice or more, and
-/// none of the three only test mode and WASM give. N's dumps are compared
-/// with these from Task 6 on.
+/// Both sides give the same resolution dump of each of the name fuzzer's
+/// inputs from the fixed seeds, every input reaches the resolver, and among
+/// them every resolver code `check` gives a loose native program occurs
+/// twice or more, and none of the three only test mode and WASM give.
 #[test]
-fn the_name_fuzzer_gives_every_native_resolver_code() {
+fn the_name_fuzzer_agrees_and_gives_every_native_resolver_code() {
     let mut all = std::collections::BTreeMap::new();
     for seed in NAME_SEEDS {
         let inputs = names(seed, NAME_COUNT);
-        let (dir, files) = write_name_inputs(&format!("names-{seed}"), &inputs);
-        let dump = s0_resolution(&dir, &files, 0);
+        let dump = agree_on_inputs(&format!("names-{seed}"), &inputs, Dumps::Resolution);
         let resolved = dump.matches("resolved=true").count();
         assert_eq!(
             resolved,
-            files.len(),
+            inputs.len(),
             "seed {seed}: an input did not reach the resolver"
         );
         for (code, n) in codes_in(&dump) {
````

**Patch `t6-code`:**

<!-- patch t6-code -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index c67aaaecc9598e5f1aeed4523f7c9d7d6d665d2c..29dc8752d3c94d1d2d94d09ef947371e12386ef8 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -582,6 +582,9 @@ enum Dumps {
     Tokens,
     /// The token dump, then 5.1b's AST dump.
     TokensAndAst,
+    /// 5.2a's resolution dump, each input a program's root beside
+    /// [`fuzz::NAME_SIBLINGS`].
+    Resolution,
 }
 
 /// Write `inputs` to a fixed directory named after `name`, as
@@ -590,6 +593,10 @@ enum Dumps {
 /// AST dump after it; or its resolution dump. The fuzzer's generators all
 /// come through here, 5.1b's deep nesting and 5.2a's name fuzzer included.
 fn agree_on_inputs(name: &str, inputs: &[String], dumps: Dumps) -> String {
+    if dumps == Dumps::Resolution {
+        let (dir, files) = write_name_inputs(name, inputs);
+        return agree_on_resolution(&dir, &files, 0);
+    }
     let dir = fresh_dir(name);
     let mut files = Vec::with_capacity(inputs.len());
     for (i, input) in inputs.iter().enumerate() {
@@ -1427,6 +1434,51 @@ fn resolve_cases_dir() -> PathBuf {
     cases_dir().join("resolve")
 }
 
+/// [`agree_on_resolution`] the resolver's cases in `cases`
+/// ([`resolve_cases_dir`], or a copy of it), from `cases`: each file in it,
+/// and each directory's `main.nova`, [`BATCH`] roots to a process; but a
+/// directory holding a module named for a std module (`dom.nova`,
+/// `ui.nova`, `RESERVED_MODULE_NAMES`) from inside it, its root named
+/// `main.nova`. E0093 names that module by its path, the root's directory
+/// joined with its file name: S0 joins with `Path::join`, which writes `\`
+/// on Windows, and N with `/`, so the path must have no directory part
+/// (ADR 0053 §5). Returns S0's output and how many programs it holds.
+fn agree_on_resolve_cases(cases: &Path) -> (String, usize) {
+    let mut entries: Vec<_> = std::fs::read_dir(cases)
+        .expect("read the resolver's cases")
+        .map(|e| e.expect("a directory entry"))
+        .collect();
+    entries.sort_by_key(|e| e.file_name());
+    let mut roots = Vec::new();
+    let mut apart = Vec::new();
+    for e in entries {
+        let path = e.path();
+        let name = e.file_name().to_string_lossy().into_owned();
+        if path.is_dir() {
+            assert!(
+                path.join("main.nova").is_file(),
+                "{} has no main.nova",
+                path.display()
+            );
+            if ["dom", "ui"]
+                .iter()
+                .any(|std| path.join(format!("{std}.nova")).is_file())
+            {
+                apart.push(path);
+            } else {
+                roots.push(format!("{name}/main.nova"));
+            }
+        } else if name.ends_with(".nova") {
+            roots.push(name);
+        }
+    }
+    let mut all = agree_on_resolution(cases, &roots, 0);
+    for dir in &apart {
+        all.push_str(&agree_on_resolution(dir, &["main.nova".to_string()], 0));
+    }
+    (all, roots.len() + apart.len())
+}
+
 /// The programs of the corpus that the resolution dump compares in 5.2a:
 /// every corpus file but those directly in a package's `src/` or
 /// `tests/`, which are 5.2b's.
@@ -1448,6 +1500,72 @@ fn loose_corpus() -> Vec<String> {
         .collect()
 }
 
+/// The resolution dump through both sides (ADR 0053): S0's `check
+/// --dump-resolution`, and N's `resolve --std DIR`, DIR the repository's
+/// `std/`, each followed by the roots.
+fn resolution() -> &'static Dump {
+    static DUMP: OnceLock<Dump> = OnceLock::new();
+    DUMP.get_or_init(|| Dump {
+        s0: RESOLUTION_S0,
+        n: leak_args(&["resolve", "--std", &std_dir()]),
+    })
+}
+
+/// std's part through both sides: S0's `check --dump-resolution --std`,
+/// and N's `resolve --std DIR --std-part`.
+fn std_resolution() -> &'static Dump {
+    static DUMP: OnceLock<Dump> = OnceLock::new();
+    DUMP.get_or_init(|| Dump {
+        s0: &["check", "--dump-resolution", "--std"],
+        n: leak_args(&["resolve", "--std", &std_dir(), "--std-part"]),
+    })
+}
+
+/// The repository's `std/`, which N reads its std modules from.
+fn std_dir() -> String {
+    root().join("std").display().to_string()
+}
+
+/// `args` for a [`Dump`], which holds them for the whole run.
+fn leak_args(args: &[&str]) -> &'static [&'static str] {
+    let owned: Vec<&'static str> = args
+        .iter()
+        .map(|a| &*Box::leak(a.to_string().into_boxed_str()))
+        .collect();
+    Box::leak(owned.into_boxed_slice())
+}
+
+/// [`agree_on`] the resolution dump: `files`, relative to `dir`, each a
+/// program's root, through both sides, [`BATCH`] roots to a process and
+/// the batches side by side, each side exiting with `code`; fail with the
+/// report of the first difference. Returns S0's output, which is N's.
+/// S0 parses std again for each program (about 0.065 s, debug) and N once
+/// per process (about 0.4 s with the debug runtime), so the batches run at
+/// once.
+fn agree_on_resolution(dir: &Path, files: &[String], code: i32) -> String {
+    let (nova, selfhost) = (nova(), selfhost());
+    let outs: Vec<Result<String, String>> = std::thread::scope(|scope| {
+        let runs: Vec<_> = files
+            .chunks(BATCH)
+            .map(|chunk| {
+                let nova = &nova;
+                scope.spawn(move || compare_batch(resolution(), dir, chunk, code, nova, selfhost))
+            })
+            .collect();
+        runs.into_iter()
+            .map(|r| r.join().expect("a batch's thread"))
+            .collect()
+    });
+    let mut all = String::new();
+    for out in outs {
+        match out {
+            Ok(out) => all.push_str(&out),
+            Err(report) => panic!("{report}"),
+        }
+    }
+    all
+}
+
 /// `nova check`'s help names neither hidden flag, and every `nova check`
 /// command line whose first argument after `check` is not
 /// `--dump-resolution` is parsed as before the flags (ADR 0053 §1): each
````

### Task 7: ADR 0053 and the records

Spec §8; decisions 1, 3, 22 to 33.

**Files:**
- Create: `docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md`
- Modify: `CHANGELOG.md`, `docs/phase-5-plan.md`,
  `nova-spec/00-MASTER-SPEC.md`, `nova-spec/11-PARSER.md`,
  `nova-spec/12-TYPESYSTEM.md`

- [ ] **Step 1: The records.** `patch_block t7-code`:
  - **ADR 0053,** the self-hosted loader and resolver, and the resolution
    dump: accepted, with the branch; §1 the dump, its grammar, the form taken
    before clap so `check` stays the base's, 5.1's known `parse --dump-ast`
    leak, an example of every form, `KINDS`, the sizes; §2 the eight codes
    (decision 25) and the user's ruling of 2026-11-03 (decision 8); §3 N's loader; §4 N's resolver, its sizes and each idiom's
    translation; §5 N's dump and command, and the differences found or
    settled: the root's `.` component, Windows paths and line endings in
    fixtures, the non-UTF-8 sibling name and root (decisions 27 and 29); §6
    std on disk; §7 S0's order; §8 the name fuzzer and its counts; §9 the
    comparison, `parse_std` (decision 26) and CI's figures (decision 23); §10
    the speed; §11 the rule that a change to resolution changes both
    compilers; its consequences.
  - **`CHANGELOG.md`,** Unreleased: the heading names 5.2a; Added (the
    hidden `nova check --dump-resolution`; the self-hosted loader and
    resolver, the comparison, the speed); Changed (a glob import's E0002s
    in declaration order).
  - **Dated notes,** each dated 2026-11-02, naming the branch
    `claude/loving-archimedes-frtf5q-52a` and citing the spec:
    `docs/phase-5-plan.md`'s 5.2 (two parts, HIR to 5.3, the three codes
    to 5.2b, as the user chose on 2026-11-03), 5.3 (HIR, the type checker's E0001s and method order) and
    5.5 (the driver's loader narrowed); `00-MASTER-SPEC.md`'s Phase 5
    (step 3 split); `11-PARSER.md` §4 (the resolution dump);
    `12-TYPESYSTEM.md`'s resolver (item-level only).
- [ ] **Step 2: Read them against the code.** Each record's claim against
  the tree: the ADR's decisions against Tasks 1 to 6 and this plan's
  decisions; its examples against `nova check --dump-resolution` of their
  sources (decision 12), by the prototype's script, which writes the
  ADR's block afresh, and a comparison:

```sh
cat > "$P/adr_examples.py" <<'EOF'
#!/usr/bin/env python3
"""ADR 0053's examples: for each, small sources written to a fresh
directory, S0's resolution dump of the first (`nova check
--dump-resolution ROOT`, or `--std` for std's part), and the lines a list
of selectors keeps, with every parent of a kept line, each line then
closed as a tree of its own: its `)`s close it and each kept parent it is
the last kept line of. A selector is a regex a line must match, or a
(parent, child) pair of regexes. Prints Markdown. Usage:
adr_examples.py NOVA"""
import os, re, subprocess, sys, tempfile

NOVA = sys.argv[1]
ALL = [r"."]

EXAMPLES = [
    ("Program, Module, StdStart, Def.Fn, Scope, Value.Def, Exports, Items, Diagnostics",
     [("hello.nova", "fn main() {}\n")],
     [r"^\(Program", r"^  \(Module ", r"\(StdStart", r"\(Def\.Fn", r'\(Value\.Def "main"',
      r"\(Exports", r"\(Items", r"^\(Diagnostics"]),
    ("Import, Importable, Def.Sum, Variant, Def.Trait, Value.Variant, Type, Trait",
     [("main.nova", "import geometry\nfn main() {}\n"),
      ("geometry.nova", "pub fn area() -> Int { 1 }\npub type Shape = | Circle(Float) | Dot\n"
                        "pub trait Show {}\n")],
     [r"\(Import ", r"\(Importable ", r"\(Def\.Sum", r"\(Variant ", r"\(Def\.Trait",
      (r"\(Exports 1", r"."), r"\(Items "]),
    ("Def.Record, Def.Const, Def.AssocType, Def.Method, Def.ExternFn",
     [("defs.nova", "type Shape = | Circle(Float) | Dot\nrecord P { x: Int }\nconst K: Int = 1\n"
                    "trait Show {\n    type Item\n    fn twice(self) {}\n}\nimpl Show for P {\n"
                    "    fn twice(self) {}\n}\nextern \"C\" {\n    fn abs(x: Int) -> Int\n}\n"
                    "fn main() {}\n")],
     [r"\(Def\.Record", r"\(Def\.Const", r"\(Def\.AssocType", r"\(Def\.Method",
      r"\(Def\.ExternFn"]),
    ("Value.Builtin, and std's definitions by their ids in a program's scope",
     [("hello.nova", "fn main() {}\n")],
     [(r"\(Scope 0", r'"println" Println'), (r"\(Scope 0", r'\(Value\.Variant "Some" s0 '),
      (r"\(Scope 0", r'\(Type "Option" s0')]),
    ("Split",
     [("split.nova", "fn f() {\n    g\n    (1)\n}\nfn main() {}\n")],
     [r"\(Split"]),
    ("Error, Primary, Secondary",
     [("dup.nova", "fn dup() {}\nfn dup() {}\nfn main() {}\n")],
     [r"\(Error", r"\(Primary", r"\(Secondary"]),
    ("Note",
     [("main.nova", "fn println() {}\nfn main() {}\n")],
     [r"\(Error", r"\(Primary", r"\(Note"]),
    ("Fix, Edit",
     [("main.nova", "@tset\nfn main() {}\n")],
     [r"\(Error", r"\(Primary", r"\(Fix", r"\(Edit"]),
    ("Warning",
     [("main.nova", "type Shade = | Red | Ok(Int)\nfn main() {}\n")],
     [(r"\(Warning", r".")]),
    ("Program resolved=false: a program the loader stops",
     [("broken.nova", "fn main( {\n")],
     ALL),
    ("Import Reported: an import the loader refused (E0093)",
     [("main.nova", "import ui\nfn main() {}\n"), ("ui.nova", "pub fn f() {}\n")],
     [r"^\(Program", r"\(Import ", r"^\(Diagnostics", (r"\(Error", r".")]),
    ("Std, from nova check --dump-resolution --std",
     None,
     [r"^\(Std", r'^  \(Module 0 ', r"\(Def\.Sum s0 ", (r"\(Def\.Sum s0 ", r"."),
      r"\(Def\.Method s2 ", r"\(Items 0\.\.", r"^\(Diagnostics"]),
    ("IOERR and UNSUPPORTED: a root that is not there, and a package's src/main.nova",
     [("missing.nova", None), ("pkg/src/main.nova", "fn main() {}\n"), ("pkg/nova.toml", "")],
     ALL),
]


def dump(files):
    with tempfile.TemporaryDirectory() as d:
        if files is None:
            args = ["--std"]
        else:
            roots = [files[0][0]] if files[0][1] is not None else []
            for name, src in files:
                if src is None:
                    continue
                path = os.path.join(d, name)
                os.makedirs(os.path.dirname(path), exist_ok=True)
                open(path, "w").write(src)
            if files[0][1] is None:
                roots = [n for n, _ in files if not n.endswith(".toml")]
            args = roots
        out = subprocess.run([NOVA, "check", "--dump-resolution", *args], cwd=d,
                             capture_output=True, text=True).stdout
    lines = out.splitlines()
    if files is not None and len(roots) == 1:
        assert lines[0] == "== " + roots[0], lines[0]
        lines = lines[1:]
    return lines


def depth(line):
    return (len(line) - len(line.lstrip(" "))) // 2


def select(lines, selectors):
    if selectors is ALL:
        return lines
    keep = set()
    parent = []
    for i, line in enumerate(lines):
        d = depth(line)
        parent.append(next((j for j in range(i - 1, -1, -1) if depth(lines[j]) < d), None)
                      if d > 0 else None)
    for i, line in enumerate(lines):
        for s in selectors:
            if isinstance(s, tuple):
                p = parent[i]
                ok = p is not None and re.search(s[0], lines[p]) and re.search(s[1], line)
            else:
                ok = re.search(s, line)
            if ok:
                j = i
                while j is not None:
                    keep.add(j)
                    j = parent[j]
    kept = [lines[i].rstrip(")") for i in sorted(keep)]
    out = []
    for j, line in enumerate(kept):
        nxt = depth(kept[j + 1]) if j + 1 < len(kept) else 0
        out.append(line + ")" * (depth(line) - nxt + 1))
    return out


def header(forms, files):
    out = [";; " + forms + ":"]
    for name, src in files or []:
        if src is None:
            continue
        shown = src.replace("\n", "\\n") or "(empty)"
        line = ";; " + name + ": "
        parts = re.split(r"(?<=\\n)", shown)
        for part in parts:
            if not part:
                continue
            if len(line) + len(part) > 78 and not line.endswith(": "):
                out.append(line)
                line = ";;     "
            line += part
        out.append(line)
    return out


print("```")
for i, (forms, files, selectors) in enumerate(EXAMPLES):
    lines = select(dump(files), selectors)
    if not lines:
        sys.exit("nothing kept for " + forms)
    if i:
        print()
    print("\n".join(header(forms, files)))
    print("\n".join(lines))
print("```")
EOF
```

```sh
cat > "$P/adrcheck.sh" <<'EOF'
#!/bin/bash
# adrcheck.sh: run from the repository's root, after the build, with $P set
# and $P/adr_examples.py written. Regenerates ADR 0053's examples, one of
# every form, from S0's `nova check --dump-resolution` of their sources,
# and compares them with the ADR's block, byte for byte.
set -u
: "${P:?set P to the scratch directory}"
adr=docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md
python3 -I "$P/adr_examples.py" "$PWD/target/debug/nova" > "$P/adr-examples.txt" || exit 1
awk '/^\*\*One example of every form\*\*/ { on = 1 } on && /^```$/ { n++; print; if (n == 2) exit; next } on && n == 1 { print }' "$adr" > "$P/adr-block.txt"
n=$(grep -c '^;; ' "$P/adr-block.txt")
if cmp -s "$P/adr-examples.txt" "$P/adr-block.txt"; then echo "adr: $n header lines, identical"; else
  echo "adr: the examples differ"; diff "$P/adr-block.txt" "$P/adr-examples.txt" | head -20; exit 1; fi
EOF
```

  ```sh
  cargo build --locked --workspace
  git diff --check HEAD
  P="$P" bash "$P/adrcheck.sh"
  ```

  Expected: `git diff --check` prints nothing, and `adr: 32 header lines, identical`.
- [ ] **Step 3: The checks,** as the conventions list them.
- [ ] **Step 4: Commit**
  `docs: ADR 0053, the self-hosted loader and resolver, and the notes for 5.2a`.

**Patch `t7-code`:**

<!-- patch t7-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 0be8af8c819d2481121c0fc594f9ac5f9545d9ca..c3e2410a0a3106ee6ec82b78e8b11edc0e8f453c 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -16,7 +16,10 @@ patterns (§4; ADR 0049), and 5.0c, std (§5; ADR 0050). Phase 5.1a, the
 self-hosted lexer and the differential (spec
 `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
 §3; ADR 0051), and 5.1b, the self-hosted parser, the AST dump and the
-depth limit (§4; ADR 0052).
+depth limit (§4; ADR 0052). Phase 5.2a, the self-hosted loader and
+resolver for loose programs and the resolution dump (spec
+`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+§3; ADR 0053).
 
 ### Added
 
@@ -185,6 +188,36 @@ depth limit (§4; ADR 0052).
   on a 1 MB main thread. A change to the grammar now changes both parsers
   in one pull request. N lexes, parses and dumps all of std about 6× as
   slowly as release S0 with a release runtime (ADR 0052).
+- **`nova check --dump-resolution FILE…` and `--std`, hidden:** what the
+  loader and the resolver built for each program whose root is FILE, as
+  an indented S-expression after a header `== FILE`: its modules, their
+  imports, its definitions with their ids and mangled names, each
+  module's namespaces sorted by name, its exports, its items and newline
+  splits, then every diagnostic with its labels, notes and fixes. std's
+  part, the same in every native program, is printed once by `--std`, and
+  a program's part names std's definitions by std-relative ids. A root
+  that cannot be read is `IOERR` and its kind, and the exit code 2; a root
+  in a package's `src/` or `tests/` is `UNSUPPORTED package` until 5.2b.
+  std is parsed once for all the roots. It is absent from `--help` and
+  carries no stability promise. It is taken only when `--dump-resolution`
+  comes first after `check`, and parsed apart, so every other `nova check`
+  command line is parsed, refused and helped exactly as before (ADR 0053).
+- **The self-hosted loader and resolver, in `selfhost/`:** N loads a loose
+  program, its root and the files beside it that it imports, breadth
+  first as the driver does, and resolves it with all of std, read from
+  `--std DIR`, as `nova-resolver` does, function for function: the
+  definitions and their ids, mangled method names, the three namespaces
+  with the builtins, std's glob and std-only names, the exports, and
+  every diagnostic in S0's order, word for word. `selfhost resolve --std
+  DIR FILE…` prints the same dump as `nova check --dump-resolution`, and
+  `--std-part` std's. The differential compares the two on std's part,
+  every loose program of the corpus, the resolver's error cases (the
+  inline sources of the resolver's and the type checker's tests) and a
+  name-level fuzzer's mutated programs, among which every resolver code
+  `check` gives natively occurs. A change to how names resolve now
+  changes both compilers in one pull request. With the debug runtime N
+  resolves and dumps the corpus's loose programs 1.5 to 2.0× as slowly as
+  S0's debug build (ADR 0053).
 
 ### Changed
 
@@ -345,6 +378,13 @@ depth limit (§4; ADR 0052).
 - **`release_golden_suite` sets environment variables on a program** its
   `RELEASE_GOLDEN_ENV` table names, as it gives standard input:
   `process_env` reads one.
+- **A glob import's E0002s come out in declaration order.** When `import
+  m` brings in names the importing module already has, the "is already
+  defined or imported in this module" errors follow the order `m`
+  declared those names in, its values first, then its types, then its
+  traits, where they followed a hash map's order before (spec
+  `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+  §3.1). The errors themselves are unchanged.
 
 ### Fixed
 
diff --git a/docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md b/docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md
new file mode 100644
index 0000000000000000000000000000000000000000..c4442b9b67893aa95effcf708637c315b08d74cd
--- /dev/null
+++ b/docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md
@@ -0,0 +1,754 @@
+# ADR 0053 — The self-hosted loader and resolver, and the resolution dump
+
+## Status
+
+Accepted (2026-11-02). Branch `claude/loving-archimedes-frtf5q-52a`
+(`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+§3). §1 is the resolution dump, its grammar and S0's `nova check
+--dump-resolution`; §2 departs from the spec on its 11 codes; §3 to §5
+are N's loader, resolver and dump, and each Rust idiom's translation; §6
+std on disk; §7 S0's insertion order; §8 the fuzzer; §9 the comparison
+and its time; §10 the speed; §11 the rule that a change to resolution
+changes both compilers. 5.2b (packages, the WASM target, test mode, the
+live heap) writes ADR 0054.
+
+## Decision
+
+### 1. The resolution dump
+
+`nova_resolver::dump::dump_program` writes a program's part,
+`nova_resolver::dump::dump_std` std's part; `nova_driver::ResolutionDumps`
+runs `check`'s pipeline up to the resolver for each root and dumps it,
+with std's modules lexed and parsed once for all of them (§9), and
+`nova_driver::dump_std_resolution` resolves a program of no module of its
+own and dumps std's part. The hidden `nova check --dump-resolution FILE…`
+prints, for each file, `== FILE` (the path as given) and its program's
+part, or `IOERR` and the error's kind for a root it cannot read, then
+exits 2 after the rest, as `parse --dump-ast` does (ADR 0052 §1). `nova
+check --dump-resolution --std` prints std's part. Both take `--target`;
+5.2a compares the native target. With neither a file nor `--std`, or with
+both, it prints one error line to stderr, nothing to stdout, and exits 2.
+
+**Hidden, and `check` as before.** The form is taken only when
+`--dump-resolution` is the first argument after `check`: `main` looks
+for it before clap parses the command line, and parses what follows with
+a parser of its own (`DumpResolutionCmd`), while `check`'s `CheckCmd` is
+the base's, field for field. So every other `nova check` command line,
+under any `argv[0]` and in a terminal, gives what the base gives, byte for
+byte on both streams and in the exit code: its errors, tips ("a similar
+argument exists" names no hidden flag), usage lines, the order in which
+clap reports a fault, and `--help` and `-h`, which `check`'s help never
+lists the flags in. The first version declared the flags in `CheckCmd`
+and re-parsed the command line without them to refuse; clap then
+suggested them, parsed `--std=1` and `--std --std` as their own errors,
+reported `--target bogus` before a second file, and printed help for
+`check --std --help`, which the first fact-check found in 17 of 24 shapes.
+After the flag, the form takes `--std`, `--target native|wasm` (or
+`--target=…`) and files, each option once, `--` before a file that starts
+with `-`, an empty file as one that is not there; it refuses anything
+else, `--help`, `-h` and `--dump-resolution` again included, with one
+clap error under the usage `nova check --dump-resolution …` and the exit
+code 2. `--dump-resolution=…` is not the form, so the base's refusal of
+it stands. 5.1's hidden `parse --dump-tokens` and `--dump-ast` are still
+arguments of `parse` itself, so `parse --dump` is told "a similar argument
+exists: '--dump-ast'": a known 5.1 leak (ADR 0052), in the base, left as
+it is; this mechanism does not reach `parse`.
+
+**What runs.** `check`'s pipeline for a loose program: the loader
+(`load_program`, breadth first from the root, sibling files), then, if no
+error so far, the `@test` functions stripped, then `resolve_program_for`
+with the target's std. The type checker does not run. A program the
+loader stops leaves the dump with its modules and the diagnostics so far
+(`resolved=false`). A root directly in a package's `src/` or `tests/`
+prints the one line `UNSUPPORTED package`: packages are 5.2b's.
+
+**Two parts.** std's modules come after the user's, and their resolution
+is the same in every program for a target: std's scopes hold no user
+name. So std's part is printed once, and a program's part prints only
+what is its own. A unit test resolves a program of two modules and checks
+that std's part of its resolution is std's part of a program of none.
+
+**Ids.** Within a part, modules and merged items are numbered from 0: a
+program's part numbers the user's modules, std's part std's. A user
+definition is `u<n>`, its `DefId`; a std definition is `s<n>`, its
+`DefId` less the first std definition's. A program's part says where std
+begins in its numbering, `(StdStart module= def= item=)`, so the global
+`DefId` of `s<n>` is `def + n`, and a std definition's `item=` in std's
+part plus the program's `item` is its merged item index.
+
+**Labels.** A place, a span in a file, names the file by a label: a user
+module's label (a loose file's stem), or a std module's name in the file
+database, `<std/core>`. A file with no label prints `?`.
+
+**The grammar.** Every line ends with `\n`, the last included. `I(d)` is
+`d` times two spaces. As in ADR 0052, a line's `)` comes right after its
+last child's.
+
+```
+output     = { "== " path "\n" part }            -- nova check --dump-resolution FILE…
+           | std diagnostics                     -- nova check --dump-resolution --std
+part       = "IOERR " kind "\n" | "UNSUPPORTED package\n" | program diagnostics
+
+program    = "(Program target=" target " resolved=" bool
+             { "\n" module(1) }
+             [ "\n" stdstart { "\n" def } { "\n" scope } { "\n" exports }
+               { "\n" items } { "\n" split } { "\n" test } ]  -- when resolved
+             ")\n"
+std        = "(Std target=" target " modules=" uint " defs=" uint " items=" uint
+             { "\n" module(1) } { "\n" def } { "\n" scope } { "\n" exports }
+             { "\n" items } { "\n" missing } ")\n"
+target     = "native" | "wasm"
+
+module(1)  = I(1) "(Module " uint " " string " package=" ("None" | uint)
+             { "\n" import } { "\n" importable } ")"
+import     = I(2) "(Import " string " " (uint | "Reported") ")"   -- sorted by name
+importable = I(2) "(Importable " string " " uint ")"              -- in the loader's order
+stdstart   = I(1) "(StdStart module=" uint " def=" uint " item=" uint ")"
+
+def        = I(1) "(Def." kind " " id " " string " " place fields { "\n" variant } ")"
+kind       = "Fn" | "Sum" | "Record" | "Const" | "Trait"          -- fields: " item=" uint
+           | "Method"      -- " item=" uint " method=" uint " owner=" ("Impl" | "TraitDefault")
+           | "ExternFn"    -- " item=" uint " fn=" uint
+           | "AssocType"   -- " trait=" id
+variant    = I(2) "(Variant " string " " place " arity=" uint ")" -- a Sum's, in order
+
+scope      = I(1) "(Scope " uint { "\n" binding } ")"   -- values, types, traits, each
+                                                        -- sorted by name (bytes)
+exports    = I(1) "(Exports " uint { "\n" binding } ")" -- values, types, traits, each
+                                                        -- in the order exported
+binding    = I(2) "(Value.Def " string " " id ")"
+           | I(2) "(Value.Variant " string " " id " " uint ")"   -- the sum, the index
+           | I(2) "(Value.Builtin " string " " word ")"          -- Builtin's variant
+           | I(2) "(Type " string " " id ")"
+           | I(2) "(Trait " string " " id ")"
+items      = I(1) "(Items " uint ".." uint " " uint ")"  -- a run of merged items of
+                                                        -- one module: start..end module
+split      = I(1) "(Split " place " kind=" ("Call" | "Index" | "Minus" | "Value") ")"
+test       = I(1) "(Test " id " " string " should_panic=" bool ")"
+missing    = I(1) "(Missing " string " " string ")"      -- sorted by name
+
+diagnostics = "(Diagnostics" { "\n" diagnostic } ")\n"   -- every one, in order
+diagnostic = I(1) "(" severity " " string " " string     -- code, message
+             { "\n" label } { "\n" note } { "\n" fix } ")"
+severity   = "Error" | "Warning" | "NoteDiagnostic" | "Help"
+label      = I(2) "(" ("Primary" | "Secondary") " " place " " string ")"  -- in order
+note       = I(2) "(Note " string ")"
+fix        = I(2) "(Fix " string { "\n" edit } ")"
+edit       = I(3) "(Edit " place " " string ")"
+
+id         = "u" uint | "s" uint
+place      = (string | "?") ":" uint ".." uint            -- a file's label, a span
+string     = ADR 0052's: in double quotes, each char below 0x20, 0x7F,
+             '"' and '\' as "\x" hh
+bool       = "true" | "false"
+```
+
+A program's part holds only its own: its modules' scopes and exports, the
+definitions before `StdStart`'s `def`, the items before its `item`, the
+user's newline splits (what the type checker's E0103 reads), and its
+tests (none under `check`, which strips them). `missing` is the target's,
+the same in every program, so std's part holds it. Every diagnostic is in
+the program's part, the loader's first, then the resolver's in S0's order:
+std's parse errors, pass 1 module by module, E0090, pass 2 (imports),
+E0102.
+
+**One example of every form**, each S0's dump of the sources after it,
+the first file the root, reduced to the lines of those forms and their
+parents, each line closed as if the others were not there:
+
+```
+;; Program, Module, StdStart, Def.Fn, Scope, Value.Def, Exports, Items, Diagnostics:
+;; hello.nova: fn main() {}\n
+(Program target=native resolved=true
+  (Module 0 "hello" package=None)
+  (StdStart module=1 def=1 item=1)
+  (Def.Fn u0 "main" "hello":3..7 item=0)
+  (Scope 0
+    (Value.Def "main" u0))
+  (Exports 0)
+  (Items 0..1 0))
+(Diagnostics)
+
+;; Import, Importable, Def.Sum, Variant, Def.Trait, Value.Variant, Type, Trait:
+;; main.nova: import geometry\nfn main() {}\n
+;; geometry.nova: pub fn area() -> Int { 1 }\n
+;;     pub type Shape = | Circle(Float) | Dot\npub trait Show {}\n
+(Program target=native resolved=true
+  (Module 0 "main" package=None
+    (Import "geometry" 1)
+    (Importable "geometry" 1))
+  (Module 1 "geometry" package=None
+    (Importable "main" 0))
+  (Def.Sum u2 "Shape" "geometry":36..41 item=3
+    (Variant "Circle" "geometry":46..52 arity=1)
+    (Variant "Dot" "geometry":62..65 arity=0))
+  (Def.Trait u3 "Show" "geometry":76..80 item=4)
+  (Exports 1
+    (Value.Def "area" u1)
+    (Value.Variant "Circle" u2 0)
+    (Value.Variant "Dot" u2 1)
+    (Type "Shape" u2)
+    (Trait "Show" u3))
+  (Items 0..2 0)
+  (Items 2..5 1))
+
+;; Def.Record, Def.Const, Def.AssocType, Def.Method, Def.ExternFn:
+;; defs.nova: type Shape = | Circle(Float) | Dot\nrecord P { x: Int }\n
+;;     const K: Int = 1\ntrait Show {\n    type Item\n    fn twice(self) {}\n
+;;     }\nimpl Show for P {\n    fn twice(self) {}\n}\nextern "C" {\n
+;;         fn abs(x: Int) -> Int\n}\nfn main() {}\n
+(Program target=native resolved=true
+  (Def.Record u1 "P" "defs":42..43 item=1)
+  (Def.Const u2 "K" "defs":61..62 item=2)
+  (Def.AssocType u4 "Item" "defs":94..98 trait=u3)
+  (Def.Method u5 "Show::twice$default" "defs":106..111 item=3 method=1 owner=TraitDefault)
+  (Def.Method u6 "P.Show.twice" "defs":148..153 item=4 method=0 owner=Impl)
+  (Def.ExternFn u7 "abs" "defs":185..188 item=5 fn=0))
+
+;; Value.Builtin, and std's definitions by their ids in a program's scope:
+;; hello.nova: fn main() {}\n
+(Program target=native resolved=true
+  (Scope 0
+    (Value.Variant "Some" s0 0)
+    (Value.Builtin "println" Println)
+    (Type "Option" s0)))
+
+;; Split:
+;; split.nova: fn f() {\n    g\n    (1)\n}\nfn main() {}\n
+(Program target=native resolved=true
+  (Split "split":19..20 kind=Call))
+
+;; Error, Primary, Secondary:
+;; dup.nova: fn dup() {}\nfn dup() {}\nfn main() {}\n
+(Diagnostics
+  (Error "E0002" "duplicate definition of `dup`"
+    (Primary "dup":15..18 "redefined here")
+    (Secondary "dup":3..6 "first defined here")))
+
+;; Note:
+;; main.nova: fn println() {}\nfn main() {}\n
+(Diagnostics
+  (Error "E0002" "duplicate definition of `println`"
+    (Primary "main":3..10 "redefined here")
+    (Note "`println` is a compiler builtin")))
+
+;; Fix, Edit:
+;; main.nova: @tset\nfn main() {}\n
+(Diagnostics
+  (Error "E0082" "unknown attribute `@tset`; known attributes are: test"
+    (Primary "main":1..5 "unknown attribute")
+    (Fix "change `tset` to `test`"
+      (Edit "main":1..5 "test"))))
+
+;; Warning:
+;; main.nova: type Shade = | Red | Ok(Int)\nfn main() {}\n
+(Diagnostics
+  (Warning "E0102" "variant `Ok` hides std's `Result::Ok`"
+    (Primary "main":21..23 "this variant")
+    (Note "in this module the bare name `Ok` now means this variant; std's is `Result::Ok`")))
+
+;; Program resolved=false: a program the loader stops:
+;; broken.nova: fn main( {\n
+(Program target=native resolved=false
+  (Module 0 "broken" package=None))
+(Diagnostics
+  (Error "P0001" "expected identifier (in parameter name), found `{`"
+    (Primary "broken":9..10 "here"))
+  (Error "P0001" "expected `)` (in function parameters), found `{`"
+    (Primary "broken":9..10 "here")))
+
+;; Import Reported: an import the loader refused (E0093):
+;; main.nova: import ui\nfn main() {}\n
+;; ui.nova: pub fn f() {}\n
+(Program target=native resolved=false
+  (Module 0 "main" package=None
+    (Import "ui" Reported)))
+(Diagnostics
+  (Error "E0093" "`ui` is the name of a std module, so no module may take it: rename ui.nova"
+    (Primary "main":7..9 "std's module, not this one")))
+
+;; Std, from nova check --dump-resolution --std:
+(Std target=native modules=21 defs=713 items=548
+  (Module 0 "$std.core" package=None)
+  (Def.Sum s0 "Option" "<std/core>":1717..1723 item=0
+    (Variant "Some" "<std/core>":1758..1762 arity=1)
+    (Variant "None" "<std/core>":1796..1800 arity=0))
+  (Def.Method s2 "Option_T.is_some" "<std/core>":2101..2108 item=2 method=0 owner=Impl)
+  (Items 0..56 0))
+(Diagnostics)
+
+;; IOERR and UNSUPPORTED: a root that is not there, and a package's src/main.nova:
+;; pkg/src/main.nova: fn main() {}\n
+;; pkg/nova.toml: (empty)
+== missing.nova
+IOERR NotFound
+== pkg/src/main.nova
+UNSUPPORTED package
+```
+
+The forms a loose native program never prints are 5.2b's: a module's
+package key comes with packages, `(Missing` with the WASM target, `(Test`
+with test mode; `NoteDiagnostic` and `Help` are severities the resolver
+never gives. An import the loader reported, `(Import "x" Reported)`, is a
+loose program's too, when E0093 refuses a sibling named for a std module;
+a package's E0004 and dev-dependency print it too. `nova_resolver::dump::KINDS`
+lists the 43 forms 5.2a's programs print and `LATER_KINDS` the others; a
+unit test checks that each kind of definition, of value and of diagnostic
+has a form in one of them, and the differential checks that every form
+of `KINDS`, and none of `LATER_KINDS`, is in S0's dump of std, a loose
+corpus program or a resolver case, and that every form those dumps print
+is in one of the lists (`listed`: a field whose value is a number or an
+id stands for itself), so a form added to the dump must be listed.
+
+**Sizes.** std's native part is 7,728 lines (320 KB), 713 definitions
+and 548 items, and S0 debug prints it in 0.082–0.142 s (§10). A typical program's part
+is about 180 lines (6 KB), almost all of it its module's bindings of
+std's names and the builtins; the corpus's largest loose program,
+`tests/runtime/modules/main.nova` with its two siblings, is 561 lines.
+S0 debug dumped the 251 loose corpus programs in about 17.8 s in one
+process when it parsed std again for each, about 0.065 s of each (at
+part A's head); with std parsed once per process (§9) it takes 3.6–4.3 s
+(§10).
+
+### 2. Eight of the 11 codes occur in 5.2a (differs from the spec)
+
+The spec (§3.5, §3.7) asks that all of the resolver's 11 codes occur among
+5.2a's inputs. Only 8 can: 5.2a compares `check`'s pipeline on the native
+target, and of the 11,
+
+- E0084 (a `@test` function's shape) and E0085 (an unknown `@test`
+  argument) are given only for a function that keeps its `@test`, and
+  `check` strips every `@test` function before resolving
+  (`strip_test_functions`), so they come with 5.2b's test mode;
+- E0090 (a user `extern` block) is given only on the WASM target, so it
+  comes with 5.2b's `--target wasm`.
+
+The user's split (the spec's decision 4) put both the 11 codes in 5.2a
+and test mode and the WASM target in 5.2b, which the prototype found
+cannot both hold; asked, the user chose on 2026-11-03 to move the three
+codes to 5.2b with test mode and the WASM target.
+
+N ports all three paths (`validate_test_function`'s E0084 and E0085, and
+`resolve_program_for`'s E0090), unreachable natively. The harness asserts
+that the other 8 (E0001, E0002, E0082, E0083, E0089, E0092, E0102, E0900)
+occur among the name fuzzer's inputs from its fixed seeds, each at least
+twice (`NATIVE_RESOLVER_CODES`; §8 gives the counts), and that none of the three does
+(`LATER_RESOLVER_CODES`) there or in the dumps of the corpus and the
+cases, though cases hold the programs that would give them; 5.2b moves
+them to the first list. The loader's
+codes on the loose path are L0001, the P-codes and E0093; E0004 and the
+two noted E0001s are a package's, 5.2b's too.
+
+### 3. N's loader
+
+`selfhost/src/loader.nova` ports the driver's `load_program` and `Loader`
+on the loose path, `strip_test_functions` (with `keep_root_tests` false,
+as `check` calls it) and `package_of`. From the root it reads, lexes and
+parses each module breadth first, with 5.1's lexer and parser; each
+single-segment import is a file `<name>.nova` in the root's directory,
+found by one listing of the directory per load (`has`: the name exactly,
+not when only a name differing in case is listed, otherwise a read of the
+path), and refused with E0093 when the name is `dom` or `ui`
+(`RESERVED_MODULE_NAMES`). The root's directory is `Path::parent`'s:
+the root less its last component, less the `/`s and `.` components that
+end what is left, but not a leading `/` or `./` (`sub/./main.nova` is in
+`sub`, `./main.nova` in `.`), and a sibling's path is that directory
+joined with its file name, which E0093 prints. A root with no directory
+part has the directory `""`, which both sides list as `.`; so does
+`package_of` (`in_package`) for `src/main.nova`, whose package's
+directory is `""`: a root is in a package when its directory is named
+`src` or `tests` and the directory above holds a file `nova.toml`. A
+module that cannot be read is skipped, so its
+import is the resolver's "cannot find module"; a root that cannot be read
+is `IOERR`. `ImportTarget`, what an import names, lives here, beside
+the loader that decides it (S0 keeps it in `nova-resolver`), so the
+loader builds and is tested without the resolver. Each module's label is
+its file's stem; each module's
+importable list is every loaded module beside it, by stem, in byte order,
+less itself and the reserved names, as `find` itself decides it. The
+diagnostics are the lex errors (L0001, "here"), the parse errors with
+their codes and notes, and E0093, in load order; any error stops the
+program before the resolver, as in S0.
+
+### 4. N's resolver
+
+`selfhost/src/resolver.nova` ports `nova-resolver`'s
+`resolve_program_for` function by function, with S0's names:
+`std_module`, `resolve_program_for`, `collect_item` (definitions and
+their ids by `Vec` length, `type_full_name`'s mangling, the trait
+defaults' `Trait::m$default`, the associated types, the attribute checks,
+the tests, the merged item list), `reject_reserved_type_name`,
+`unknown_attribute`, `validate_attrs_reject_test`,
+`validate_test_function`, `test_shape_violations`, E0090, `resolve_import`,
+`make_public`, `would_receive`, `binds`, the fix-dropping step,
+`import_std_module` (std's glob, last and lowest-priority; `IMPORTED_STD`
+nowhere, `STD_INTERNAL` into std alone), `std_globbed_variants`,
+`variant_shadowing` (E0102), `missing_names`, `insert_value`,
+`insert_type`, `bind_value`, `bind_type`, `bind_trait`, `unsupported`,
+`path_tail`. Every message, label, note and fix is S0's word for word.
+`builtins.nova` holds the builtin table (the 118 variants' Rust names,
+which the dump prints, and their names in a program; `GLOBAL`, `STD_ONLY`
+and `WASM_ONLY` as lists of variant names) and the std tables
+(`STD_MODULES` and `WASM_STD_MODULES` with each source's path under std's
+directory, `IMPORTED_STD`, `RESERVED_MODULE_NAMES`, `STD_INTERNAL`,
+`STD_TEST_MODULE`, `RESERVED_TYPE_NAMES`); `diagnostic.nova` the
+`Diagnostic` record, `suggest.rs`'s `distance`, `closest` and `change_to`,
+and `lines.rs`'s `keyword_start`. `resolver.nova` is 1,486 lines (1,224 code lines), under
+the 1,500 past which the brief asked for a split, so it is one file.
+
+**Sizes,** in code lines as 5.1 counted them (a line that is neither
+blank nor a comment line, one starting with `//`, and nothing from a Rust
+file's first `#[cfg(test)]` on): N's are
+`diagnostic.nova` 189, `builtins.nova` 473, `loader.nova` 341,
+`resolver.nova` 1,224 and `resolution_dump.nova` 369, 2,596 in all, and
+about 100 more in `main.nova`'s command; S0's are `nova-resolver`'s
+`lib.rs` 1,962 (1,921 before 5.2a; the builtin and std tables about 465 of
+them), `dump.rs` 552, and about 250 of `program.rs`'s 646 for the loose
+path (its `load_program`, `Loader` and their types, less the package and
+doc-test arms). N's tests are 277 more.
+
+**Each Rust idiom's translation**, one way everywhere (ADR 0052's table
+holds; these are new):
+
+| Rust | Nova |
+|---|---|
+| `DefId(u32)`, `ModuleId(u32)`, `Option<u32>` package keys | `Int`s; -1 for none |
+| `Builtin`, a 118-variant enum, and `name()` | its index in `BUILTIN_VARIANTS`, the macro's declaration order; `BUILTIN_NAMES[i]` its name |
+| `Res`, `DefKind`, `MethodOwner`, `ImportTarget` | sums with prefixed variants (`ResDef`, `DefKindMethod`, `ImportTargetReported`), named fields positional |
+| a `Span`'s `FileId` | the module's file label, passed beside the tree's spans (`file`) and kept in each `Def`, `VariantDef`, `Label` and `Edit` |
+| `FxHashMap<String, _>` namespaces, looked up | `Map<String, _>`, never iterated into output; the dump sorts its keys (`sort_strings`, a merge sort) |
+| `Exports`' maps and S0's new name `Vec`s (§7) | the same: maps for lookups, `value_names`/`type_names`/`trait_names` for every loop |
+| `IndexMap<String, Span>` first definitions | `Map<String, Span>`: only looked up |
+| `&mut definitions.modules[mid]` | `definitions.modules.at(mid)` held in a `let mut`: a record is a reference |
+| `Diagnostic::error(…).with_primary_label(…)` | `error_at(code, message, file, span, label)`, then `d.add_note(…)` on a variable: a builder's `mut self` call on a temporary is E0060 |
+| `diagnostics[before..].iter_mut().for_each(\|d\| d.fixes.clear())` | `clear_fixes(diagnostics, before)` |
+| a constant array read in a loop (`BUILTIN_VARIANTS[i]`) | read once into `BuiltinTable` (`builtin_table()`): a constant array is built again each time it is read, which made the first build 9× slower than this one |
+| std parsed in every `resolve_program_for` | parsed once per process (`load_std`, `std_module`), handed in as `StdSet`; its diagnostics are given first, as S0 gives them |
+| `Path::parent`, `Path::join`, `file_name`, `file_stem` | `path_parent`, `path_join`, `file_name`, `module_name` over `/`, trimming `.` components and repeated `/`s as `Components` does |
+| `VecDeque` of places | a `Vec` and its head's index; a loose place is its stem, since every module of a loose program is in the root's directory |
+
+Not ported: the language server's import occurrences (`imports`,
+`Occurrence`), which no dump prints, and `ResolveResult`/`resolve`, the
+one-file wrapper only S0's tests call.
+
+### 5. N's dump and command
+
+`selfhost/src/resolution_dump.nova` ports `dump.rs`'s `dump_program`,
+`dump_std`, `first_std_def` and the `Dump` writer, and prints §1's grammar
+byte for byte, its strings through 5.1's `push_escaped`. `selfhost resolve
+--std DIR FILE…` prints `== FILE` and each root's part (`IOERR` and the
+kind for a root it cannot read, then exit code 2 after the rest; `UNSUPPORTED
+package` for a root directly in a package's `src/` or `tests/`), and
+`selfhost resolve --std DIR --std-part` std's part; without `--std DIR`,
+or with neither a root nor `--std-part`, or with both, it prints its
+usage line to stderr, nothing to stdout, and exits 2, as S0's form does
+with its own line. `main.nova`'s `dump_resolution` is the
+driver's: load, stop on any error, strip the `@test` functions, resolve.
+N's own tests (`loader_tests.nova`, `resolver_tests.nova`, in `src/` as
+5.1's are): a loose program's breadth-first order, its imports, E0093 and
+the importable lists; what `check` strips; the paths; a glob's clashes in
+declaration order; the mangled method names; a builtin's name redefined;
+`make_public`'s edits; the attribute fix and `closest`'s edges. `nova test`
+in `selfhost/` runs 25. N takes no `--target`: 5.2a resolves natively, and
+`resolver.nova`'s WASM paths (E0090, E0092's WASM arm, `missing`) wait for
+5.2b's flag. Its command line is a test tool's: an empty first FILE or a
+`--` is a root named so (`IOERR NotFound`), where S0's clap refuses the
+one and skips the other; the harness passes neither.
+
+**Differences found while building.** None in output on the inputs:
+N's first build equalled S0 on std's part, all 251 loose corpus programs
+and the cases, and no input of the fixed seeds or of the longer searches
+(`NOVA_SELFHOST_FUZZ=7:1500` and `11:5000`, 6,500 name-fuzzed programs
+beside the soup and the mutations) has shown one. Before the split, a
+probe of the root's spellings found one, and two more are settled or
+recorded here:
+
+- **A root with a `.` component** (`sub/./main.nova`): `Path::parent`
+  gives `sub`, and N gave `sub/.`, so E0093 named `sub/./ui.nova` where
+  S0 names `sub/ui.nova`. N's `path_parent` and `file_name` now trim as
+  `Components` does (§3), and `a_root_spelled_any_way_names_its_siblings_alike`
+  compares eleven spellings: bare, `./`, `.//`, `sub/`, `./sub/`,
+  `sub/./`, `sub//`, `./sub/.//`, `sub/../sub/`, `../sub/` and an absolute
+  path.
+- **Windows paths.** S0's resolution output holds one path: E0093's
+  message names the sibling it refuses as `Path::join` writes it, the
+  root's directory and the file name, `dir\ui.nova` on Windows, where N
+  writes `dir/ui.nova`. (The `== FILE` header is the root as given on both
+  sides when the root is UTF-8, below, and modules are labelled by stem.)
+  Settled in the harness, not in
+  either compiler: a case directory holding a `dom.nova` or `ui.nova` runs
+  from inside that directory, its root named `main.nova`, so the path has
+  no directory part and is `ui.nova` on every system, and the spellings
+  test compares only the bare root on Windows. The other ways were worse:
+  S0 printing `/` would change `nova check`'s message on Windows against
+  the base, which this phase must not; N cannot tell the system it runs
+  on (Nova has no such query); and running every case from its own
+  directory costs about 10 s of CPU for N's std per process. So
+  `selfhost resolve` given a root with a directory part still prints
+  `/` on Windows: 5.5's driver, which owns paths and the command line,
+  settles it. This was reasoned from `std::path` (on Windows `Path::join`
+  pushes `\` after a directory that does not end in a separator, and
+  `Path::parent` splits at `/` and `\` alike); the pull request's Windows
+  leg is the proof. CRLF is the other Windows difference: git gives
+  `cases/` no `.gitattributes` rule, as for 5.1's cases, so a Windows
+  checkout compares the cases with `\r\n`, which both sides read alike;
+  `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`
+  compares a CRLF copy of them on every system. The same checkout gives
+  `\r\n` to any checked-in text a test compares byte for byte: a
+  test that reads a fixture with `include_str!` or `read_to_string` and
+  compares it with output fails there. `nova-resolver`'s
+  `dump_test_program.txt` did (the first fact-check reproduced it with a
+  CRLF copy); its test now reads each `\r\n` in the fixture as `\n`, and
+  `the_expected_program_part_may_have_windows_line_endings` checks that
+  with a CRLF copy on every system. No other fixture that 5.2a reads is
+  compared so: the harness's other reads normalise (`names_of`) or feed
+  both sides the same bytes.
+- **A file name that is not UTF-8** beside a root: S0's listing drops it
+  and N's `read_dir` keeps it with U+FFFD in place of its bad bytes, so an
+  import written with U+FFFD would find a module on N's side alone. No
+  identifier holds U+FFFD, so no input reaches it; the spellings test puts
+  one beside its roots on Linux (macOS refuses such names) and the dumps
+  agree. A root that is a directory is `IOERR Other` on both sides on
+  Linux (macOS reads it the same way); on Windows opening a directory
+  fails with `PermissionDenied` instead, for S0 at least, and the harness
+  compares no such root. An import of a directory named `x.nova` is
+  "cannot find module" on both sides
+  (`an_import_of_a_directory_finds_no_module`).
+- **A root whose path is not UTF-8** (`r\xff.nova`, `sub\xfe/main.nova`,
+  on Linux): S0 opens it and dumps its program, under a header with
+  U+FFFD in place of the bad bytes; N's arguments reach it as UTF-8 with
+  U+FFFD, so it opens another name, prints `IOERR NotFound` and exits 2.
+  ADR 0051 records the same for 5.1's dumps. Nova has no byte paths
+  before 5.5's driver, so it is recorded, not settled; no input of the
+  differential has such a root, and
+  `a_root_whose_path_is_not_utf8_is_dumped_by_s0_and_not_found_by_n` pins
+  today's behaviour on Linux. A `nova.toml` that is a directory makes no
+  package on either side: the header test compares such a root.
+
+### 6. std on disk
+
+N reads std's sources from `--std DIR` (the spec's decision 2), at the
+paths `builtins.nova`'s tables give (`core/lib.nova`,
+`ui/backend/native.nova`, …), labels each module as the driver registers
+it (`<std/core>`), and parses the target's modules once per process; the
+harness passes the checkout's `std/`, which is what S0 embeds
+(`std/lib.rs`'s `include_str!`s), CRLF and all on a Windows checkout. A
+file it cannot read stops `selfhost resolve` with a message and exit
+code 2. In 5.5 the driver finds std next to its executable or in
+`$NOVA_HOME`, as the language server does.
+
+### 7. S0's insertion order
+
+A glob import's E0002s for the names it clashes with followed `FxHash`
+order (`beta, delta, epsilon, alpha, gamma` for five functions), which N's
+seeded `Map` cannot reproduce. `Exports` keeps each namespace's names in a
+`Vec` beside its map, in the order exported, which is the order declared,
+and `resolve_import`'s glob follows them (the spec's decision 3, §3.1); no
+crate is added. No other output changed: `import_std_module` and
+`missing_names` bind with `or_insert`, whose result does not depend on
+order, and the namespaces are looked up, never iterated into output. The
+test `a_glob_imports_clashes_are_reported_in_declaration_order` pins it,
+and the CHANGELOG's Changed records it.
+
+### 8. The name fuzzer
+
+`fuzz::name_mutate` (`crates/nova-cli/tests/selfhost/fuzz.rs`) mutates a
+loose corpus program that parses, one to three times, at the level of
+names: a definition renamed to collide with another, a builtin, a std
+name, a built-in type's name or a std-only builtin's; an item duplicated
+or deleted; `pub` toggled; an import of a missing module, a sibling whole
+or by a list, `dom`, `ui`, a nested path, an alias or itself; an unknown
+attribute or a malformed `@test`; an item that is an alias, a sum hiding
+std's variants, a trait twice, an `extern` block, a record named `Bytes`,
+a `@test` impl block, impls for `()` and a function type. A mutation
+that would not parse is skipped, so every input reaches the resolver,
+which the harness asserts. Input `i` is written as `f{i:05}.nova` beside
+four siblings (`NAME_SIBLINGS`): `shapes` and `helpers`, and, since the
+first fact-check, `vault`, which keeps a private item of each kind a fix
+can make public and one name twice, the second with its keyword on a
+line of its own, and `keeper`, which imports `vault` whole and declares a
+record named as one of `vault`'s private functions. Three imports reach
+them: `vault::{hidden}`, whose fix is dropped since one item's keyword
+cannot be found; `vault::{SEALED, Cell, Hue, Veiled}`, a fix per kind;
+and `vault::{Kept}` with `keeper`, whose fix is dropped because it would
+bind `Kept` in `keeper`'s types as well. The fact-check's mutants of N
+that dropped one item instead of the whole fix, lost `make_public`'s
+`const` arm, or made `binds` ignore the types namespace all passed: the
+two siblings held one private function and nothing else, so no input
+offered `make_public` another kind of item or a clash in another
+namespace. Now the fuzzer alone catches those three, and the mutants that
+reword E0083's "an impl block" or rename `type_full_name`'s `anon`
+fallbacks; the resolver cases catch all of them too (§9). One kind it
+cannot reach is a file system's shape (a `nova.toml` that is a
+directory): the inputs are files beside fixed siblings, so the header
+test holds that one. The fixed seeds 301–303 give 50 inputs each (150 in
+all; the codes they give: E0001 46, E0002 63, E0082 14, E0083 8, E0089
+10, E0092 13, E0102 50, E0900 18), and the harness asserts each of the 8
+at least twice, a floor the rarest, E0083, clears with 6 to spare. The inputs are the same on
+every system: the files are picked from their sorted list, read with each
+`\r\n` as `\n`, and the siblings are written from the harness's own
+text; a test checks that a Windows checkout's corpus, listed in reverse,
+gives the same inputs. `NOVA_SELFHOST_FUZZ=SEED:COUNT` adds COUNT
+name-fuzzed programs to its longer search.
+
+### 9. The comparison, and its time
+
+`selfhost.rs` compares S0's and N's resolution dumps byte for byte, with
+their exit codes, through the same `compare_batch` as 5.1's dumps:
+
+- std's part, once (`std_resolution`);
+- every loose corpus program, 251 at 5.2a: the corpus is every `.nova`
+  file of the repository outside `selfhost/` and `cases/`, then N's own
+  sources, and a loose program is every one of them but those directly in
+  a package's `src/` or `tests/`, which are 5.2b's; in batches of 50 roots
+  run side by side (`agree_on_resolution`), each root's part there;
+- beside them, the 103 resolver cases in
+  `crates/nova-cli/tests/selfhost/cases/resolve/` (`agree_on_resolve_cases`):
+  88 the inline sources of `nova-resolver`'s and the type checker's tests
+  (`<testfile>__<test>[_n]`), 15 written by hand (names without `__`) for
+  the loader's edges and some of the resolver's: a sibling named for a std
+  module, a sibling that fails to lex or parse, a parse error with a note,
+  an import cycle, an import of a directory, an import whose case differs
+  from its file's (which only a case-insensitive disk, macOS's or
+  Windows', could find), a root that imports itself, a `@test` function
+  that would collide, an empty file, and a `pub` fix dropped because it
+  would bind a name twice, once through a glob import and once through a
+  listed one; and, from the first fact-check, a fix dropped because one
+  item's keyword is not on its name's line (a `const` beside a function),
+  a fix dropped because it would bind a type twice, a `@test` impl block
+  (E0083's "an impl block"), and impls for a function type and `()`
+  (methods named `anon.…`). The note, the two imports and these four close
+  gaps that breaking N's code on purpose found, where no input told the
+  two sides apart. A
+  file is a program, a directory one whose root is `main.nova`, and a
+  directory holding a `dom.nova` or `ui.nova` runs from inside itself
+  (§5); the same cases with `\r\n` line endings, on every system, the
+  copy checked to hold them;
+- the name fuzzer's 150 inputs, through `agree_on_inputs` with the third
+  kind of dump, `Dumps::Resolution`, which writes them beside the
+  siblings;
+- the headers: a missing root, a package's root, a root that does not
+  parse, a root beside a `nova.toml` that is a directory, and a root that
+  is not UTF-8, exit code 2; and no root at all, refused by both sides
+  with nothing on stdout; S0's form given an empty root, `== ` and
+  `IOERR NotFound`;
+- roots whose directory is listed as `.`: a root with no directory part
+  beside a directory named `ui.nova` (E0093), a package's `src/main.nova`
+  and `tests/main.nova` named from the package's directory, and an empty
+  root;
+- eleven spellings of a root's path (§5), and, but on Windows, a root
+  named `q"uo\te.nova`, whose label the dump escapes; on Linux, a root
+  whose path is not UTF-8, which pins today's difference (§5).
+
+The corpus files in a package's `src/` or `tests/` are skipped: the test
+asserts a floor on their count (66 at 5.2a) and reports it with
+`eprintln!`, which libtest shows only under `--nocapture`, so CI's log
+does not.
+
+Every form of `KINDS` must appear in the dumps of std, the corpus and the
+cases, and no form of `LATER_KINDS`; the dumps are both sides', byte for
+byte, so this holds for N's as for S0's.
+
+**Time.** On a 4-core Linux machine, debug build, with the target and the
+temporary directory on the same disk (not tmpfs), `selfhost.rs` ran in
+about 34 s (124 s of CPU) at part A's head, where S0 alone dumped the
+corpus, the cases and 450 fuzz inputs, parsing std again for each, and
+in 47 s with N's side added at 450 inputs. Three changes bring it down.
+The machine's speed varies from day to day, so 5.2a's tests are measured
+against 5.1's, run alone in the same build, interleaved: after the
+second fact-check's fixes, three runs of the final tree's 36 tests took
+31.9, 32.3 and 30.9 s (95 to 100 s of CPU), against 20.9, 20.0 and
+20.3 s (49 to 51 s of CPU) for the 27 tests 5.1 had, 1.48 to 1.61
+times; after the first fact-check's fixes the same runs took 24.8 to
+26.5 s against 16.7 to 17.6 s, 1.40 to 1.58 times, and the replay's
+slowest run then, 29.5 s, was 1.77 times 5.1's fastest. (The first
+fact-check measured 24.3 s, 72 s of CPU, and 24 to 26 s over about 20
+runs, with the target on tmpfs; part B's 34.5 to 39.9 s against 25.4 s
+for 5.1's tests came from a slower state of the same machine.)
+
+- **S0 parses std once per `--dump-resolution` process** (differs from
+  the spec, which leaves S0's resolver as it was but for §7).
+  `nova_resolver::parse_std` lexes and parses a target's std modules and
+  `resolve_program_with_std` resolves a program with them;
+  `resolve_program_for` is the two in one, as before, for every other
+  caller (`check`, `build`, `run`, `test`, the language server), whose
+  output is unchanged: `nova check` over every file the base tracks
+  prints what the base's does, and a unit test checks that
+  `resolve_program_for` and the two give the same file, definitions,
+  diagnostics, tests and import occurrences for two programs sharing one
+  `ParsedStd`. `nova_driver::ResolutionDumps` parses std once and, for
+  each root, registers std's modules first in the program's file
+  database, so their ids, and the parsed trees' spans, are the same in
+  every program. The dumps are unchanged, byte for byte; S0 dumps the
+  corpus in 3.6–4.3 s where it took 17.8 s. The cost: two public
+  functions and a struct in `nova-resolver`, and a second path into the
+  resolver that only the dump takes.
+- **The two sides of a batch run at once** (`compare_batch`), which 5.1's
+  dumps share, and the corpus's batches beside the cases'.
+- **The fixed seeds give 50 inputs each, not 150**: every one of the 8
+  codes still occurs at least twice, E0083 the rarest with 8.
+
+**CI.** On `main`'s CI at `1fa7cd3f` (5.1b's merge), `selfhost.rs` took
+19.45 s on Windows and 22.50 s on macOS, and about 19.3 s on Linux on
+5.1b's pull request: 5.1's tests. Here 5.2a's tests take 1.4 to 1.8
+times 5.1's in the same build (1.40 to 1.77 in every measurement above),
+so on CI they should take about 27 s (19.3 × 1.4) to 40 s (22.50 × 1.8)
+on each system, under the spec's
+minute (risk 4), and no count is cut; the pull request's three legs give
+the figures. (An estimate from 5.1a's older ratio, 1.4 to 2.1 times
+local on macOS and Windows, gave 46 to 75 s; `main`'s run replaces it.)
+
+### 10. The speed
+
+Measured locally, five runs, N built by `nova build` with the debug
+runtime (what CI uses), S0's debug build:
+
+| | N | S0 |
+|---|---:|---:|
+| std's part alone (`--std-part` / `--std`) | 0.46–0.52 s | 0.082–0.142 s |
+| the 251 loose corpus programs, one process | 6.3–6.7 s | 3.6–4.3 s (17.8 s parsing std per program, at part A's head) |
+
+N parses std once in about 0.45 s and then resolves and dumps a program
+in about 0.024 s, against S0's 0.015 s with std parsed once: 1.5 to 2.0×
+S0 debug over the corpus (1.53 to 1.81 in five runs here after the
+second fact-check's fixes, 1.69 to 1.84 in three after the first; the
+second fact-check measured 1.48, 2.01 and 1.86), and about 4 to 6× on
+std's part (3.6 to 6.3 here), whose 21 scopes of about 450
+names each N sorts with a merge sort over `String.cmp`. Not gated, and
+well under the ~10× past which the user would be told. The first build was
+9× slower (139 s over the corpus): it read the builtin table's constant
+arrays in a loop, and Nova builds a constant array again each time it is
+read (§4's idiom).
+
+### 11. A change to resolution changes both compilers
+
+From 5.2a the loader of loose programs and the item-level resolver exist
+twice. A change to how a loose program loads, how names are bound, what
+the builtin or std tables hold, how methods are mangled, or what the
+resolver says changes `nova-resolver` (or the driver's loader) and
+`selfhost/` in one pull request, with the dump's grammar here if it
+changes; the differential in every pull request holds the two equal
+(phase plan decision 6).
+
+## Consequences
+
+- N has its third and fourth stages for loose programs: it loads a root
+  and its siblings and resolves them with all of std, as S0 does, byte
+  for byte in the dump. 5.2b adds packages, the WASM target and test mode
+  (and the three codes §2 defers), and the live heap's figures; 5.3 ports
+  the type checker and HIR on top of `Definitions`.
+- The dump pins `DefId` numbering and mangled method names, which reach
+  symbols in 5.5.
+- S0's resolver gained `parse_std` and `resolve_program_with_std`, and
+  its glob import's E0002s follow declaration order; nothing else in S0's
+  behaviour changed. `nova check` gained a form only when
+  `--dump-resolution` comes first after `check`; every other `nova
+  check` command line, `--help` included, gives what it gave (§1).
+- `selfhost.rs` takes 1.4 to 1.8 times as long locally as 5.1's tests
+  alone in the same build, and should take about 27 to 40 s on CI's
+  three systems (§9), under a minute.
+- `selfhost resolve` writes `/` in E0093's path on Windows, where S0
+  writes `\`, for a root with a directory part; the harness keeps such
+  paths out of what it compares, and 5.5's driver settles it (§5).
+- Nova's constant arrays cost an allocation per read: N reads them once,
+  and 5.3's tables should too.
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index 9e46adf3bf0890e6c6f3b3f7027618b6b9167ca3..85679e0a15c0fede1ec64b13f4adc7b046353e98 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -412,6 +412,31 @@ arguments, a call's arguments being a level deeper than the call).
   soup); how deep the recursion goes before N's stack runs out.
 
 ### 5.2 — The resolver and HIR
+
+**Amended 2026-11-02 and 2026-11-03 (branch
+`claude/loving-archimedes-frtf5q-52a`):** by the user's decisions of
+2026-11-02
+(`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`)
+and their ruling of 2026-11-03 on three of the resolver's codes, 5.2 is
+**the loader and the resolver**, in two parts, and **HIR moves to 5.3**.
+HIR is the type checker's output in S0, built where paths, locals
+and types are resolved, so it is ported, and dumped, with the type
+checker; the gate below that names HIR dumps and E0001-class diagnostics
+(most of which are the type checker's "cannot find `x`") is 5.3's. 5.2
+ports what runs before the type checker: loading a program's modules,
+and the item-level resolver (definitions, imports, std's globs, the
+builtin table, std-only names and the resolver's codes), compared
+through a canonical resolution dump on both sides.
+- **5.2a** (ADR 0053): the dump; N's loader for loose programs (a root
+  and the sibling files it imports) and its resolver, with std read from
+  `--std DIR`; S0's glob-clash E0002s in declaration order; the
+  resolver's error cases and a name-level fuzzer. Of the resolver's 11
+  codes, the 8 that `check`'s native pipeline can give occur in 5.2a's
+  inputs; E0084 and E0085 (test mode) and E0090 (WASM) wait for 5.2b,
+  as the user chose on 2026-11-03.
+- **5.2b:** packages (`nova.toml`'s name and path dependencies, `src/`
+  and `tests/`, a dependency), the WASM target, test mode, and the live
+  heap's figures, recorded (decision 8).
 - **Modules:** loading a package's flat directory, std's 21 modules, the
   imports, scopes, and the builtin table.
 - **HIR,** and a canonical dump of resolution and HIR.
@@ -427,6 +452,13 @@ arguments, a call's arguments being a level deeper than the call).
   std-only names are tabled.
 
 ### 5.3 — The type checker
+
+**Amended 2026-11-02 (branch `claude/loving-archimedes-frtf5q-52a`):**
+5.3 also ports HIR (`nova-hir`, 810 code lines), which the type checker
+builds, with its dump, the type checker's own E0001s ("cannot find `x`
+in this scope"), and the type checker's method order, which follows a
+hash map in S0 (5.2's spec §2, §10).
+
 The largest part, 7,404 code lines; its spec may split it, for example:
 - **5.3a:** inference, expressions, patterns and exhaustiveness;
 - **5.3b:** traits and associated types, generics and bounds, and async
@@ -452,6 +484,12 @@ The largest part, 7,404 code lines; its spec may split it, for example:
   numbering of locals and blocks is deterministic enough to compare.
 
 ### 5.5 — The `.ll` emitter, the driver and linking
+
+**Amended 2026-11-02 (branch `claude/loving-archimedes-frtf5q-52a`):**
+5.2 ports the module loader (loose programs in 5.2a, packages in 5.2b),
+so the driver's item narrows to what 5.2 leaves: the command line and
+its exit codes, finding std next to the executable or in `$NOVA_HOME`,
+`clang` and `cc`, finding `libnova_runtime.a`, and linking.
 - **The emitter,** a port of `nova-codegen-llvm`, and `--emit-ll`.
 - **The driver:** the module loader, `clang` and `cc` through 5.0b's
   spawn, finding `libnova_runtime.a` as `link.rs` does, and the commands
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 3b52b45488a6574809a16b651711fcdf7ca17832..6e09e868affa2eb2b42bdc022d53090f99e29bff 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -784,6 +784,19 @@ levels with P0002, and S0 runs its pipeline on a 64 MB thread. N lexes and
 parses std about 6× as slowly as release S0 with a release runtime. From
 here a change to the grammar changes both compilers in one pull request.
 
+**Amended 2026-11-02 (branch `claude/loving-archimedes-frtf5q-52a`):**
+step 3 is split: the loader and the item-level resolver are ported
+first, as 5.2
+(`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`),
+and HIR, which the type checker builds, moves to step 4's 5.3. 5.2a (ADR
+0053): `selfhost/` loads a loose program and resolves it with all of
+std, read from disk, and `selfhost resolve` prints the resolution dump
+S0's hidden `nova check --dump-resolution` prints, which the
+differential compares on std's part, every loose corpus program, the
+resolver's error cases and a name-level fuzzer's programs. From here a
+change to how names resolve changes both compilers in one pull request.
+5.2b adds packages, the WASM target and test mode.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/nova-spec/11-PARSER.md b/nova-spec/11-PARSER.md
index 5f193bb1c986ab1782f315b2771b79acbb1988d5..3263ce4b46e08873d33ec8345b2473fe6efdf153 100644
--- a/nova-spec/11-PARSER.md
+++ b/nova-spec/11-PARSER.md
@@ -493,6 +493,22 @@ FILE…` prints the same bytes, which the differential compares on every
 input; like the token dump, it changes with the tree, in both compilers
 at once.
 
+**Amended 2026-11-02 (branch `claude/loving-archimedes-frtf5q-52a`):**
+**the resolution dump**
+(`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+§3.4; ADR 0053, which writes its grammar out with an example of every
+form) joins the two, one stage on: `nova_resolver::dump` renders what
+the loader and the resolver built for a program in the AST dump's
+S-expression style: the modules and their imports, the definitions with
+their ids and mangled names, each module's namespaces sorted by name,
+the exports, the items and newline splits, then every diagnostic with
+its labels, notes and fixes. std's part, the same in every native
+program, is printed once, and a program's part names std's definitions
+by std-relative ids. The hidden `nova check --dump-resolution FILE…`
+(and `--std`) prints it, and the self-hosted resolver's `selfhost
+resolve --std DIR FILE…` (and `--std-part`) prints the same bytes; it
+changes with resolution, in both compilers at once.
+
 ---
 
 ## 5. Error Recovery
diff --git a/nova-spec/12-TYPESYSTEM.md b/nova-spec/12-TYPESYSTEM.md
index 6c1be3dc0bcffa201a3bfc067f9bf35ec5de2478..8da5104bdf568369cf53dd089154fa09c300d0b1 100644
--- a/nova-spec/12-TYPESYSTEM.md
+++ b/nova-spec/12-TYPESYSTEM.md
@@ -30,6 +30,18 @@ AST → [Resolver] → resolved AST → [TypeCheck] → typed AST (HIR)
 - Reports `unresolved name`, `duplicate definition`, `private item access`
 - Outputs: `ResolvedAst` where every `Path` is fully qualified
 
+**Amended 2026-11-02 (branch `claude/loving-archimedes-frtf5q-52a`):**
+what the resolver builds is item-level only: one merged item list, the
+definitions with their ids (methods named `Type.Trait.method`), each
+module's three namespaces (values, types, traits) with the builtins
+seeded and std's exports globbed in last, the exports and the tests.
+Paths, locals and types are resolved inside the type checker, which
+reports "cannot find `x`", and HIR is the type checker's output. The
+self-hosted compiler ports the loader and this resolver first (Phase
+5.2,
+`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`;
+ADR 0053) and HIR with the type checker (5.3).
+
 **TypeChecker** (`nova-typeck`):
 - Walks ResolvedAst
 - Generates type variables and constraints
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
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-52a.txt" 2>&1
  grep '^test result' "$P/full-52a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-52a.txt"
  cargo test --locked --workspace -- --list > "$P/final-list-52a.txt" 2>&1
  python3 "$P/names.py" "$P/base-list-52a.txt" "$P/final-list-52a.txt"
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
  - the suite, in 4 min 56 s: `2857 1 10`, the baseline's figures and
    16 more tests;
  - the one failure is Task 0's IPv6 test;
  - `names.py` prints `binaries: base 112, now 112` and `tests: base 2852, now 2868`, and
    `net added: 16`, 16 added and 0 gone:

    - `nova: tests/selfhost.rs (selfhost)`, 27 to 36:
      `a_root_spelled_any_way_names_its_siblings_alike`;
      `a_root_whose_name_needs_escapes_is_printed_alike`;
      `a_root_whose_path_is_not_utf8_is_dumped_by_s0_and_not_found_by_n`;
      `a_root_with_no_directory_part_or_in_a_package_loads_alike`;
      `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`;
      `the_resolution_dump_flags_are_hidden_and_refused_as_before`;
      `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`;
      `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`;
      `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`;
    - `nova_resolver (unittests src/lib.rs)`, 60 to 67:
      `dump::tests::a_programs_part_prints_its_modules_definitions_items_and_diagnostics`;
      `dump::tests::every_kind_of_definition_value_and_diagnostic_is_listed`;
      `dump::tests::parsing_std_once_resolves_as_resolve_program_for_does`;
      `dump::tests::std_part_is_the_same_in_every_program`;
      `dump::tests::the_expected_program_part_may_have_windows_line_endings`;
      `dump::tests::the_program_prints_every_form`;
      `tests::a_glob_imports_clashes_are_reported_in_declaration_order`.
- [ ] **Step 2: CI's Linux leg, as CI runs it.** The `test` job's
  `ubuntu-24.04` leg runs the whole suite with the golden suite required,
  with CI's own flags, `--locked --workspace --all-features --no-fail-fast`:

  ```sh
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast > "$P/ci-52a.txt" 2>&1
  grep '^test result' "$P/ci-52a.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/ci-52a.txt"
  grep -c 'release_golden_suite ... ok' "$P/ci-52a.txt"
  ```

  Expected, in 5 min 20 s: `2857 1 10`, Step 1's figures, the one failure the
  IPv6 test, and `1`: the golden suite ran and passed inside the whole run,
  as on CI, where Step 1's run skips it.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in
  29 s of wall time: `release golden: 147 passed, 9 excluded` and `1 passed`: unchanged,
  since 5.2a adds no runtime program.
- [ ] **Step 4: Parsing and checking every file the base tracks.** Two
  scripts, as the replay ran them, compare the base's `nova` (Task 0's
  copy, `$P/nova-base-52a`) with the tree's on every `.nova` file
  `git ls-tree -r --name-only 7878b9bb` lists, 589 files, each
  run's output, both streams, and exit status compared byte for byte.
  5.2a changes neither the lexer nor the parser, and no file the base
  tracks has a glob import that clashes, so nothing here changes. Both
  run from the repository's root after the build, and read `$P`.
  - `parsecmp.sh`: `nova parse --check` and `nova parse --ast` of each
    file, both on the file as `7878b9bb` has it, from a
    `git archive 7878b9bb` unpacked in `$P/pc-src-52a`, each passed the
    path relative to that root:

```sh
cat > "$P/parsecmp.sh" <<'EOF'
#!/bin/bash
# parsecmp.sh: run from the repository's root, after the build, with $P set.
# For every `.nova` file `git ls-tree -r --name-only 7878b9bb` lists (the
# base's tracked files, so none 5.2a adds), `nova parse --check` and
# `nova parse --ast`, by the base's `nova` ($P/nova-base-52a, Task 0) and
# by the tree's `target/debug/nova`, both on the file as 7878b9bb has it,
# from a `git archive 7878b9bb` unpacked in $P/pc-src-52a: 5.2a adds
# seven files to N's sources and changes its `main.nova`, which the base's
# text keeps out of the comparison. Each run passes the path relative to that root and
# keeps its output, both streams, and its exit status; every pair must be
# byte-identical, since 5.2a changes neither the lexer nor the parser.
# Prints each file that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
src="$P/pc-src-52a"
rm -rf "$src"; mkdir -p "$src"
git archive 7878b9bb | tar -x -C "$src"
n=0; differ=0; nz_base=0; nz_new=0
while read -r f; do
  for mode in --check --ast; do
    a=$( (cd "$src" && "$P/nova-base-52a" parse $mode "$f") 2>&1; echo "exit=$?")
    b=$( (cd "$src" && "$nova" parse $mode "$f") 2>&1; echo "exit=$?")
    n=$((n + 1))
    if [ "$mode" = --check ]; then
      [ "$(tail -1 <<< "$a")" != exit=0 ] && nz_base=$((nz_base + 1))
      [ "$(tail -1 <<< "$b")" != exit=0 ] && nz_new=$((nz_new + 1))
    fi
    if [ "$a" != "$b" ]; then differ=$((differ + 1)); echo "differs: $mode $f"; fi
  done
done < <(git ls-tree -r --name-only 7878b9bb | grep '\.nova$')
echo "parse: $n runs over $((n / 2)) files, $differ differ; --check exits nonzero on $nz_base at the base, $nz_new now"
rm -rf "$src"
[ "$differ" = 0 ]
EOF
```

  - `checkcmp.sh`: `nova check` by both, on the tree's files, which shows
    that `check`, through `resolve_program_for`, prints what the base
    prints (decision 26). A file's package root is the nearest directory
    above it, in the tree, that holds a `nova.toml`, below the
    repository's root: each package is checked once, by `nova check` run
    in its root, and every other file alone, by `nova check FILE` from
    the repository's root:

```sh
cat > "$P/checkcmp.sh" <<'EOF'
#!/bin/bash
# checkcmp.sh: run from the repository's root, after the build, with $P set.
# `nova check` by the base's `nova` ($P/nova-base-52a, Task 0) and by the
# tree's `target/debug/nova`, both on the tree's files, for every `.nova`
# file `git ls-tree -r --name-only 7878b9bb` lists. A file's package root
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
    a=$( (cd "$pkg" && "$P/nova-base-52a" check) 2>&1; echo "exit=$?")
    b=$( (cd "$pkg" && "$nova" check) 2>&1; echo "exit=$?")
    what="$pkg/ (package)"
  else
    a=$("$P/nova-base-52a" check "$f" 2>&1; echo "exit=$?")
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
done < <(git ls-tree -r --name-only 7878b9bb | grep '\.nova$')
echo "check: $n checks, $differ outside std differ, $stddiff std files differ"
[ "$differ" = 0 ] && [ "$stddiff" = 0 ]
EOF
```

  - Then:

    ```sh
    P="$P" bash "$P/parsecmp.sh"
    P="$P" bash "$P/checkcmp.sh"
    ```

    Expected: `parse: 1178 runs over 589 files, 0 differ; --check exits nonzero on 116 at the base, 116 now`, then `check: 541 checks, 0 outside std differ, 0 std files differ`. Both exit 0.
- [ ] **Step 5: The hidden flags stay hidden.**
  - `helpcmp.sh` runs `--help` of `nova` and of each of its 21 commands,
    `help` included, by the base's `nova` and the tree's, both through a
    link named `nova`, and checks that neither `nova parse --help` nor
    `nova check --help` names a hidden flag, `MORE` or `--std` (decisions
    13 and 31):

```sh
cat > "$P/helpcmp.sh" <<'EOF'
#!/bin/bash
# helpcmp.sh: run from the repository's root, after the build, with $P set.
# `--help` of `nova` and of every command, by the base's `nova`
# ($P/nova-base-52a, Task 0) and the tree's `target/debug/nova`, both run
# through a link named `nova` so the usage lines agree: both streams and the
# exit status must be byte-identical, the hidden flags and `more` absent
# from `parse --help` and `check --help`.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
d="$P/help-52a"
rm -rf "$d"; mkdir -p "$d/base" "$d/tree"
ln -s "$P/nova-base-52a" "$d/base/nova"
ln -s "$PWD/target/debug/nova" "$d/tree/nova"
n=0; same=0
for cmd in "" parse run build bundle dev check test fmt doc lsp new init add remove fetch update login package publish version help; do
  a=$("$d/base/nova" $cmd --help 2>&1; echo "exit=$?")
  b=$("$d/tree/nova" $cmd --help 2>&1; echo "exit=$?")
  n=$((n + 1))
  if [ "$a" == "$b" ]; then same=$((same + 1)); else echo "differs: nova $cmd --help"; fi
done
hidden=$( ("$d/tree/nova" parse --help; "$d/tree/nova" check --help) | grep -c 'dump-\|MORE\|--std')
echo "help: $same of $n identical; hidden flags shown: $hidden"
rm -rf "$d"
[ "$same" = "$n" ] && [ "$hidden" = 0 ]
EOF
```

  - 29 `parse` and `check` shapes, refusals and the help beside them,
    from the base's `nova` and the tree's, each run with the same first
    argument: `nova`, a link's name, `othernova`, a renamed binary, and
    four with no file name in UTF-8, `""`, `/`, `..` and the bytes ff fe;
    both streams and the exit code must be the base's, byte for byte
    (decisions 13 and 31); eight of them in a terminal, through
    `script`'s pseudo-terminal, colours included; then, on the tree alone,
    5.1's `--dump-ast` against `--dump-tokens`, and the form's own
    misuses, which the base has no form of, each one error and exit 2.
    Before the first fact-check the refusals were held only to "one
    error, exit 2"; run on that tree, this script reports 90 of 174 runs
    different and 4 of 8 in a terminal, and exits 1. `argv0.py`
    runs a program by `execv` with the first argument it is given, empty
    included, which Python's `os.execv` refuses:

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
# and $P/argv0.py written. The errors `parse` and `check` give for
# arguments they refuse, and what they print for the shapes beside them,
# from the base's `nova` ($P/nova-base-52a, Task 0) and the tree's
# `target/debug/nova`, each run by execv with the same first argument:
# `nova` (a link's name), `othernova` (a renamed binary), and four with no
# file name in UTF-8, "", "/", ".." and the bytes ff fe. Both streams and
# the exit status must be byte-identical, tips, usage lines and `--help`
# included: every `check` command line whose first argument after `check`
# is not `--dump-resolution` is parsed as the base parses it. Then the
# same in a terminal, colours included, through `script`'s pseudo-terminal
# with NO_COLOR unset. Then 5.1's `--dump-ast` against the tree's own
# `--dump-tokens`. Then the shapes the hidden `check --dump-resolution`
# refuses, which the base lacks: each must exit 2 with nothing on stdout
# and one error on stderr, no panic.
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
  "check --help --dump-resolution" "check -h --std")
runs=0; same=0
for arg0 in nova othernova "" / .. '\xff\xfe'; do
  for args in "${shapes[@]}"; do
    eval "python3 -I '$P/argv0.py' '$arg0' '$P/nova-base-52a' $args" > "$P/usage-base.txt" 2>&1
    echo "exit $?" >> "$P/usage-base.txt"
    eval "python3 -I '$P/argv0.py' '$arg0' '$nova' $args" > "$P/usage-tree.txt" 2>&1
    echo "exit $?" >> "$P/usage-tree.txt"
    runs=$((runs + 1))
    if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then same=$((same + 1)); else
      echo "differs: [$arg0] $args"; diff "$P/usage-base.txt" "$P/usage-tree.txt" | head -12; fi
  done
  echo "[$arg0] $(grep -a '^Usage:' "$P/usage-tree.txt" | head -1), $(tail -1 "$P/usage-tree.txt")"
done
tty=0; ttys=("parse a b" "parse -- a b" "check a b" "check --std a" "check --st a.nova"
  "check --std=1 a.nova" "check --std --help" "check a b --bogus")
for args in "${ttys[@]}"; do
  env -u NO_COLOR script -qc "python3 -I '$P/argv0.py' othernova '$P/nova-base-52a' $args" /dev/null > "$P/usage-base.txt"
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
res=0; resok=0
for arg0 in nova othernova "" / .. '\xff\xfe'; do
  for args in "check --dump-resolution" "check --dump-resolution --std a" "check --dump-resolution --dump-resolution a" \
              "check --dump-resolution --target bogus a" "check --dump-resolution --std --std" \
              "check --dump-resolution --stdd" "check --dump-resolution --help" "check --dump-resolution -h"; do
    eval "python3 -I '$P/argv0.py' '$arg0' '$nova' $args" > "$P/usage-tree.txt" 2> "$P/usage-err.txt"
    code=$?
    res=$((res + 1))
    if [ "$code" = 2 ] && [ ! -s "$P/usage-tree.txt" ] && [ "$(grep -c '^error' "$P/usage-err.txt")" = 1 ] \
       && ! grep -q panicked "$P/usage-err.txt"; then resok=$((resok + 1)); else
      echo "--dump-resolution: [$arg0] $args: exit $code"; cat "$P/usage-tree.txt" "$P/usage-err.txt"; fi
  done
done
echo "usage: $same of $runs runs identical; in a terminal, $tty of ${#ttys[@]}; --dump-ast as --dump-tokens: $astsame of $asts; --dump-resolution refused: $resok of $res"
rm -f "$P/usage-base.txt" "$P/usage-tree.txt" "$P/usage-err.txt"; cd / && rmdir "$work"
[ "$same" = "$runs" ] && [ "$tty" = "${#ttys[@]}" ] && [ "$astsame" = "$asts" ] && [ "$resok" = "$res" ]
EOF
```

    ```sh
    P="$P" bash "$P/helpcmp.sh"
    P="$P" bash "$P/usagecmp.sh"
    ```

    Expected: `help: 22 of 22 identical; hidden flags shown: 0`, then

    ```
[nova] Usage: nova check [OPTIONS] [FILE], exit 0
[othernova] Usage: othernova check [OPTIONS] [FILE], exit 0
[] Usage: check [OPTIONS] [FILE], exit 0
[/] Usage: check [OPTIONS] [FILE], exit 0
[..] Usage: check [OPTIONS] [FILE], exit 0
[\xff\xfe] Usage: check [OPTIONS] [FILE], exit 0
usage: 174 of 174 runs identical; in a terminal, 8 of 8; --dump-ast as --dump-tokens: 25 of 25; --dump-resolution refused: 48 of 48
    ```

    and both exit 0.
- [ ] **Step 6: The lockfile and Rust 1.78.**
  - `git diff --numstat 7878b9bb -- Cargo.lock`. Expected: nothing.
  - Then:

    ```sh
    RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-52a cargo check --locked --workspace
    rm -rf /dev/shm/msrv-52a
    ```

    Expected: clean, `Finished` in 32 s. It checks the libraries
    and binaries, as CI's MSRV job does, not the tests: `cargo check
    --workspace` compiles no test target, so the tests' Rust meets 1.78 by
    review alone. Its target directory is in `/dev/shm`, memory, so the
    disk stays above 6G.
- [ ] **Step 7: The differential's time, N's tests and the fuzzer's
  codes.** `SH --no-run`, then three times, each timed, `SH` and then
  `SH -- --exact` followed by the 27 tests `selfhost.rs` had at `$BASE`
  (`git show $BASE:crates/nova-cli/tests/selfhost.rs | grep -A3
  '^#\[test\]' | grep -o '^fn [a-z0-9_]*'`, less the `fn `), then
  `SH the_name_fuzzer --nocapture 2>&1 | grep 'codes over'`, then
  `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 1`. Expected:
  `36 passed; 0 failed` each time, in 27.2, 26.8 and 28.5 s (82 to 87 s of CPU) of wall time,
  building N included, and `27 passed; 0 failed; …; 9 filtered out` in
  18.6, 18.9 and 17.9 s: 1.42 to 1.59 times, within decision 23's 1.4 to 1.8;
  then `codes over seeds [301, 302, 303]: {"E0001": 46, "E0002": 63, "E0082": 14, "E0083": 8, "E0089": 10, "E0092": 13, "E0102": 50, "E0900": 18}`; then `test result: ok. 25 passed; 0 failed; 0 trapped; 25 total`. Record them in the ledger beside CI's, once the PR's
  run gives each system's.
- [ ] **Step 8: The mutants.** One at a time, each edit is made, its
  command run, and the file restored with `git checkout -- <file>`, all on
  the last commit, in a shell under the process limit.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it. Mutant
    91 makes a second such edit in another file
    ("And in …"), and both files are restored.
  - Each command starts with `cargo build --locked --workspace`. A mutant
    of N needs no Rust rebuilt: `selfhost.rs` builds N from `selfhost/`
    each run.
  - The replay's run took 58 min in all; a mutant of S0 about
    46 s, one of N about 26 s.
  - Each caught mutant must fail as written: the tests named.
    Mutants 3, 22, 31, 32, 40, 62, 76, 77, 78, 95 pass by design, each for the reason it gives. Any other mutant that passes is a gap: record
    it in the ledger, name it in the PR, and give it to the review (Step
    10), which says whether a check is added.
  - At the end, `git status --short` prints nothing, and
    `cargo build --locked --workspace` rebuilds the last commit's binaries
    (decision 39).

  1. **The glob's values back in the map's order.** `crates/nova-resolver/src/lib.rs`:
     `for (n, r) in exports[target].values_in_order() {` becomes `for (n, r) in exports[target].values.iter().map(|(k, v)| (k.clone(), *v)).collect::<Vec<_>>() {`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- a_glob_imports_clashes_are_reported_in_declaration_order` and `SH`, each to its end.
     - Expected: fails: `tests::a_glob_imports_clashes_are_reported_in_declaration_order`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  2. **The glob's types back in the map's order.** `crates/nova-resolver/src/lib.rs`:
     `for (n, id) in exports[target].types_in_order() {` becomes `for (n, id) in exports[target].types.iter().map(|(k, v)| (k.clone(), *v)).collect::<Vec<_>>() {`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- a_glob_imports_clashes_are_reported_in_declaration_order` and `SH`, each to its end.
     - Expected: fails: `tests::a_glob_imports_clashes_are_reported_in_declaration_order`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  3. **A name exported twice keeps its place twice.** `crates/nova-resolver/src/lib.rs`:
     replace

     ```
             if self.values.insert(name.clone(), res).is_none() {
                 self.value_names.push(name);
             }
     ```

     with

     ```
             self.values.insert(name.clone(), res);
             self.value_names.push(name);
     ```

     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- a_glob_imports_clashes_are_reported_in_declaration_order` and `SH`, each to its end.
     - Expected: passes, accepted: equivalent: `insert_value` exports a name only after finding it unbound in the module's scope, so no name is exported twice and the guard never fires (decision 39).
  4. **Std's ids not std-relative.** `crates/nova-resolver/src/dump.rs`:
     `format!("s{}", id.0 - self.std_def)` becomes `format!("s{}", id.0)`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
     - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `dump::tests::std_part_is_the_same_in_every_program`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  5. **A user id off by one.** `crates/nova-resolver/src/dump.rs`:
     `format!("u{}", id.0)` becomes `format!("u{}", id.0 + 1)`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
     - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `dump::tests::a_programs_part_prints_its_modules_definitions_items_and_diagnostics`, `dump::tests::the_expected_program_part_may_have_windows_line_endings`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  6. **StdStart's def off by one.** `crates/nova-resolver/src/dump.rs`:
     `d.field("def", &std_def.to_string());` becomes `d.field("def", &(std_def + 1).to_string());`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
     - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `dump::tests::a_programs_part_prints_its_modules_definitions_items_and_diagnostics`, `dump::tests::the_expected_program_part_may_have_windows_line_endings`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  7. **A scope's values unsorted.** `crates/nova-resolver/src/dump.rs`:
     `values.sort_by(|a, b| a.0.cmp(b.0));`: the line goes.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
     - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `dump::tests::a_programs_part_prints_its_modules_definitions_items_and_diagnostics`, `dump::tests::the_expected_program_part_may_have_windows_line_endings`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  8. **The exports in the map's order.** `crates/nova-resolver/src/dump.rs`:
     `for (name, res) in e.values_in_order() {` becomes `for (name, res) in e.values.iter().map(|(k, v)| (k.clone(), *v)) {`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
     - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  9. **A variant's arity off by one.** `crates/nova-resolver/src/dump.rs`:
     `self.field("arity", &v.arity.to_string());` becomes `self.field("arity", &(v.arity + 1).to_string());`.
     - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
     - Expected: fails: `dump::tests::a_programs_part_prints_its_modules_definitions_items_and_diagnostics`, `dump::tests::the_expected_program_part_may_have_windows_line_endings`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  10. **Strings printed unquoted.** `crates/nova-resolver/src/dump.rs`:
      `quote(&mut self.out, text);` becomes `self.out.push_str(text);`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `dump::tests::a_programs_part_prints_its_modules_definitions_items_and_diagnostics`, `dump::tests::std_part_is_the_same_in_every_program`, `dump::tests::the_expected_program_part_may_have_windows_line_endings`, `dump::tests::the_program_prints_every_form`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  11. **A secondary label printed as primary.** `crates/nova-resolver/src/dump.rs`:
      `"Secondary"` becomes `"Primary"`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_programs_part_prints_its_modules_definitions_items_and_diagnostics`, `dump::tests::the_expected_program_part_may_have_windows_line_endings`, `dump::tests::the_program_prints_every_form`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  12. **Notes dropped.** `crates/nova-resolver/src/dump.rs`:
      replace

      ```
                      self.open(2, "Note");
                      self.string(note);
                      self.close();
      ```

      with

      ```
                      let _ = note;
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_programs_part_prints_its_modules_definitions_items_and_diagnostics`, `dump::tests::the_expected_program_part_may_have_windows_line_endings`, `dump::tests::the_program_prints_every_form`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  13. **An edit's text dropped.** `crates/nova-resolver/src/dump.rs`:
      `self.string(&edit.text);`: the line goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::a_programs_part_prints_its_modules_definitions_items_and_diagnostics`, `dump::tests::the_expected_program_part_may_have_windows_line_endings`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  14. **An import the loader reported printed as module 0.** `crates/nova-resolver/src/dump.rs`:
      `ImportTarget::Reported => d.out.push_str(" Reported"),` becomes `ImportTarget::Reported => d.out.push_str(" 0"),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `dump::tests::the_program_prints_every_form`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  15. **A package's root not marked.** `crates/nova-driver/src/lib.rs`:
      `return Ok("UNSUPPORTED package\n".to_string());` becomes `return Ok("UNSUPPORTED\n".to_string());`.
      - Run `cargo build --locked --workspace`, `SH the_resolution_dump` and `SH`, each to its end.
      - Expected: fails: `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`.
  16. **The header without its space.** `crates/nova-cli/src/cmd/run.rs`:
      `writeln!(out, "== {}", file.display())?;` becomes `writeln!(out, "=={}", file.display())?;`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  17. **An unreadable root exits 0.** `crates/nova-cli/src/cmd/run.rs`:
      `code = 2;`: the line goes.
      - Run `cargo build --locked --workspace` and `SH the_resolution_dump`, each to its end.
      - Expected: fails: `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`.
  18. **--std with a FILE accepted.** `crates/nova-cli/src/cmd/run.rs`:
      `if !cmd.files.is_empty() {` becomes `if false {`.
      - Run `cargo build --locked --workspace` and `SH the_resolution_dump`, each to its end.
      - Expected: fails: `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`.
  19. **@test functions kept in the dump.** `crates/nova-driver/src/lib.rs`:
      `strip_test_functions(&mut modules, false);`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  20. **The resolver run after a load error.** `crates/nova-driver/src/lib.rs`:
      replace

      ```
              let resolved = (!failed)
                  .then(|| nova_resolver::resolve_program_with_std(self.target, &sources, &self.std));
      ```

      with

      ```
              let resolved = Some(nova_resolver::resolve_program_with_std(self.target, &sources, &self.std));
      ```

      - Run `cargo build --locked --workspace`, `SH the_resolution_dump` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  21. **Std parsed once for the wrong target.** `crates/nova-driver/src/lib.rs`:
      `std: nova_resolver::parse_std(target, &std_files, None),` becomes `std: nova_resolver::parse_std(Target::Wasm, &std_files, None),`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  22. **Std registered after the program's files.** `crates/nova-driver/src/lib.rs`:
      replace

      ```
              let (_, mut labels) = register_std(&mut db, self.target);
              let load = program::load_program(None, &roots, &DiskSources, &mut db)?;
      ```

      with

      ```
              let load = program::load_program(None, &roots, &DiskSources, &mut db)?;
              let (_, mut labels) = register_std(&mut db, self.target);
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: equivalent for the dump: the program's part prints no span into std, so std's file ids, which this order sets, never show; the type checker, which would read them, does not run under the dump (decision 39).
  23. **A hidden `--std` declared in `check`'s own arguments again.** `crates/nova-cli/src/cmd/run.rs`:
      replace

      ```
          target: TargetArg,
      }

      /// What `--target` names
      ```

      with

      ```
          target: TargetArg,

          #[arg(long, hide = true)]
          std: bool,
      }

      /// What `--target` names
      ```

      - Run `cargo build --locked --workspace` and `SH the_resolution_dump`, each to its end.
      - Expected: fails: `the_resolution_dump_flags_are_hidden_and_refused_as_before`.
  24. **The hidden form taken for any argument starting `--dump-res`.** `crates/nova-cli/src/cmd/run.rs`:
      `if args.next()? != "check" || args.next()? != "--dump-resolution" {` becomes `if args.next()? != "check" || !args.next()?.to_string_lossy().starts_with("--dump-res") {`.
      - Run `cargo build --locked --workspace` and `SH the_resolution_dump`, each to its end.
      - Expected: fails: `the_resolution_dump_flags_are_hidden_and_refused_as_before`.
  25. **The hidden form given a `--help`.** `crates/nova-cli/src/cmd/run.rs`:
      `disable_help_flag = true,`: the line goes.
      - Run `cargo build --locked --workspace` and `SH the_resolution_dump`, each to its end.
      - Expected: fails: `the_resolution_dump_flags_are_hidden_and_refused_as_before`.
  26. **The expected program part compared with its `\r\n`s.** `crates/nova-resolver/src/dump.rs`:
      `assert_eq!(own.join("\n") + "\n", want.replace("\r\n", "\n"));` becomes `assert_eq!(own.join("\n") + "\n", want);`.
      - Run `cargo build --locked --workspace` and `cargo test --locked --workspace --lib -- dump::tests`, each to its end.
      - Expected: fails: `dump::tests::the_expected_program_part_may_have_windows_line_endings`.
  27. **The resolution pass compares nothing.** `crates/nova-cli/tests/selfhost.rs`:
      replace

      ```
              let runs: Vec<_> = files
                  .chunks(BATCH)
                  .map(|chunk| {
                      let nova = &nova;
      ```

      with

      ```
              let runs: Vec<_> = files[..0]
                  .chunks(BATCH)
                  .map(|chunk| {
                      let nova = &nova;
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  28. **A form missing from KINDS.** `crates/nova-resolver/src/dump.rs`:
      `"(Split",`: the line goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::the_program_prints_every_form`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  29. **A later form listed as 5.2a's.** `crates/nova-resolver/src/dump.rs`:
      replace

      ```
          "(Edit",
      ];
      ```

      with

      ```
          "(Edit",
          "(Missing",
      ];
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked --workspace --lib -- dump::tests` and `SH`, each to its end.
      - Expected: fails: `dump::tests::the_program_prints_every_form`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  30. **The fuzzer's seeds cut to 5 inputs.** `crates/nova-cli/tests/selfhost.rs`:
      `const NAME_COUNT: usize = 50;` becomes `const NAME_COUNT: usize = 5;`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`.
  31. **A name mutation that does not parse kept.** `crates/nova-cli/tests/selfhost/fuzz.rs`:
      `if named_items(&next).is_some() {` becomes `if true {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: equivalent under the fixed seeds: none of their mutations fails to parse, so the guard drops none; it keeps longer searches' inputs resolvable, and the test's count of `resolved=true` would fail on a seed whose mutation did not parse (decision 39).
  32. **A std-named sibling's case run from the cases' directory.** `crates/nova-cli/tests/selfhost.rs`:
      `.any(|std| path.join(format!("{std}.nova")).is_file())` becomes `.any(|std| path.join(format!("{std}.none")).is_file())`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: Windows-only: on Linux and macOS `Path::join` writes `/` as N does, so the case agrees from either directory; CI's Windows leg is the check (decision 39).
  33. **The CRLF copy written with LF.** `crates/nova-cli/tests/selfhost.rs`:
      replace

      ```
          if text.contains(&b'\r') {
              return text;
      ```

      with

      ```
          if true {
              return text;
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  34. **The queue taken from its tail.** `selfhost/src/loader.nova`:
      `let (place, path) = queue.at(head)` becomes `let (place, path) = queue.at(queue.len() - 1)`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_loose_program_loads_breadth_first_from_its_root`.
  35. **A sibling's path without the root's directory.** `selfhost/src/loader.nova`:
      `FoundModule(name, path_join(self.dir, "${name}.nova"))` becomes `FoundModule(name, "${name}.nova")`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_loose_program_loads_breadth_first_from_its_root`.
  36. **E0093 never given.** `selfhost/src/loader.nova`:
      `if !holds(RESERVED_MODULE_NAMES, name) { return found }` becomes `if true { return found }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_loose_program_loads_breadth_first_from_its_root`.
  37. **E0093 names the import, not the file.** `selfhost/src/loader.nova`:
      `rename ${what}` becomes `rename ${name}`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_loose_program_loads_breadth_first_from_its_root`.
  38. **A lex error's label.** `selfhost/src/loader.nova`:
      `diagnostics.push(error_at("L0001", "${e}", label, e.span(), "here"))` becomes `diagnostics.push(error_at("L0001", "${e}", label, e.span(), "there"))`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  39. **A parse error's notes dropped.** `selfhost/src/loader.nova`:
      `for n in e.notes().iter() { d.add_note(n) }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  40. **A name differing in case read from the disk.** `selfhost/src/loader.nova`:
      `if eq_ignore_ascii_case(n, name) { folded = true }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: Linux-only: on a case-sensitive disk the read of `Shapes.nova` fails as the listing's answer would; CI's macOS and Windows legs run `an_import_differing_in_case_finds_no_module`, where it would succeed (decision 39).
  41. **A module importable from itself.** `selfhost/src/loader.nova`:
      `Some(j) => { if j != i { importable.push((name, j)) } }` becomes `Some(j) => importable.push((name, j))`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_loose_program_loads_breadth_first_from_its_root`.
  42. **A `.` component kept before the file name.** `selfhost/src/loader.nova`:
      `more = e > lo && path.byte_at(e - 1) == 46 && (e - 1 == lo || path.byte_at(e - 2) == 47)` becomes `more = false`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `paths_split_and_join_as_rusts_do`.
  43. **A package's root not seen.** `selfhost/src/loader.nova`:
      `for e in entries.iter() { if e.name == "nova.toml" && e.is_file { return true } }` becomes `for e in entries.iter() { if e.name == "nova.tom" && e.is_file { return true } }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`.
  44. **N resolves after a load error.** `selfhost/src/main.nova`:
      `if failed { return Ok(dump_program(target, sources, None, diagnostics, std_set.builtins)) }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  45. **N keeps the @test functions.** `selfhost/src/main.nova`:
      `if !failed { strip_test_functions(modules) }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  46. **Std's scopes without STD_ONLY.** `selfhost/src/resolver.nova`:
      `seed(scope, table, table.std_only)`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  47. **The user's scopes with STD_ONLY.** `selfhost/src/resolver.nova`:
      replace

      ```
              if mid >= std_start {
                  seed(scope, table, table.std_only)
      ```

      with

      ```
              if mid >= 0 {
                  seed(scope, table, table.std_only)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_builtins_name_redefined_is_e0002_with_a_note`.
  48. **An impl's method mangled trait first.** `selfhost/src/resolver.nova`:
      `Some(tr) => "${self_name}.${path_tail(tr.value)}.${f.name.value}"` becomes `Some(tr) => "${path_tail(tr.value)}.${self_name}.${f.name.value}"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `an_impls_type_names_its_methods`.
  49. **A default method without `$default`.** `selfhost/src/resolver.nova`:
      `name: "${t.name.value}::${f.name.value}$" + "default",` becomes `name: "${t.name.value}::${f.name.value}",`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `an_impls_type_names_its_methods`.
  50. **A generic type's arguments left out of its name.** `selfhost/src/resolver.nova`:
      `"${head}_${"_".join(parts.to_array())}"` becomes `head`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `an_impls_type_names_its_methods`.
  51. **An array's type named otherwise.** `selfhost/src/resolver.nova`:
      `TypeArray(elem) => "Arr_${type_full_name(elem.value)}"` becomes `TypeArray(elem) => "Array_${type_full_name(elem.value)}"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `an_impls_type_names_its_methods`.
  52. **A tuple's type named otherwise.** `selfhost/src/resolver.nova`:
      `"Tup${items.len()}_${"_".join(parts.to_array())}"` becomes `"Tuple${items.len()}_${"_".join(parts.to_array())}"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `an_impls_type_names_its_methods`.
  53. **An associated type's trait off by one.** `selfhost/src/resolver.nova`:
      `kind: DefKindAssocType(id),` becomes `kind: DefKindAssocType(id + 1),`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `an_impls_type_names_its_methods`.
  54. **An impl's methods all index 0.** `selfhost/src/resolver.nova`:
      replace

      ```
                              kind: DefKindMethod(item_index, method_index, MethodOwnerImpl),
                          },
                      )
                      method_index += 1
      ```

      with

      ```
                              kind: DefKindMethod(item_index, method_index, MethodOwnerImpl),
                          },
                      )
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  55. **An extern block's functions all index 0.** `selfhost/src/resolver.nova`:
      `fn_index += 1`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  56. **A pub sum's variants not exported.** `selfhost/src/resolver.nova`:
      replace

      ```
                                  ResVariant(id, vi),
                                  pubv,
      ```

      with

      ```
                                  ResVariant(id, vi),
                                  false,
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  57. **A private value exported.** `selfhost/src/resolver.nova`:
      `if is_pub { exp.export_value(name, res) }` becomes `exp.export_value(name, res)`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `importing_a_private_item_offers_pub_where_its_keyword_starts`.
  58. **`@test` on an item not a function is E0082.** `selfhost/src/resolver.nova`:
      replace

      ```
              if attr.name.value == "test" {
                  diagnostics.push(
      ```

      with

      ```
              if attr.name.value == "tests" {
                  diagnostics.push(
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  59. **E0082's fix dropped.** `selfhost/src/resolver.nova`:
      replace

      ```
                      Some(fix) => d.add_fix(fix)
                      None => ()
                  }
                  diagnostics.push(d)
                  continue
      ```

      with

      ```
                      Some(_) => ()
                      None => ()
                  }
                  diagnostics.push(d)
                  continue
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `an_unknown_attribute_on_a_function_offers_the_nearest_known_one`.
  60. **E0089 never given.** `selfhost/src/resolver.nova`:
      `if !holds(RESERVED_TYPE_NAMES, name) { return false }` becomes `return false`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  61. **A two-segment import taken as one.** `selfhost/src/resolver.nova`:
      `if segments.len() > 1 && !is_imported_std_path(joined) {` becomes `if segments.len() > 2 && !is_imported_std_path(joined) {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  62. **An import the loader reported binds module 0.** `selfhost/src/resolver.nova`:
      `Some(ImportTargetReported) => return` becomes `Some(ImportTargetReported) => 0`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: unreachable natively in 5.2a: the loader reports an import only with E0093, an error that stops the program before the resolver; a package's E0004 and dev-dependency reach it in 5.2b (decision 39).
  63. **E0092's message.** `selfhost/src/resolver.nova`:
      `is a std module, which target ${t} does not have` becomes `is a std module, which target ${t} lacks`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  64. **A self-import not seen.** `selfhost/src/resolver.nova`:
      `if target == mid {` becomes `if target == mid + 1 {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test`s `a_glob_imports_clashes_are_reported_in_declaration_order`, `importing_a_private_item_offers_pub_where_its_keyword_starts`.
  65. **A glob's values in the map's order.** `selfhost/src/resolver.nova`:
      replace

      ```
                  for (n, r) in e.values_in_order().iter() {
                      bind_value(scope, diagnostics, n, span, r, file)
      ```

      with

      ```
                  for n in e.values.keys().iter() {
                      bind_value(scope, diagnostics, n, span, e.value(n), file)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_glob_imports_clashes_are_reported_in_declaration_order`.
  66. **A listed type not bound.** `selfhost/src/resolver.nova`:
      `bind_type(scope, diagnostics, name, n.span, id, file)` becomes `let _ = id`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  67. **The `pub` fix withheld in the same package.** `selfhost/src/resolver.nova`:
      `if !same_package { return None }` becomes `if same_package { return None }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `importing_a_private_item_offers_pub_where_its_keyword_starts`.
  68. **The `pub` fix's text.** `selfhost/src/resolver.nova`:
      `edits.push(Edit::insert(at, m.label, "pub "))` becomes `edits.push(Edit::insert(at, m.label, "pub"))`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `importing_a_private_item_offers_pub_where_its_keyword_starts`.
  69. **A `pub` fix kept that would bind a name twice.** `selfhost/src/resolver.nova`:
      `if g != fix_target` becomes `if g == fix_target`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  70. **A glob import not seen to receive the name.** `selfhost/src/resolver.nova`:
      `ImportKindSimple => return true` becomes `ImportKindSimple => ()`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  71. **A list import not seen to receive the name.** `selfhost/src/resolver.nova`:
      `for n in names.iter() { if n.value == name { return true } }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  72. **E0102 never given.** `selfhost/src/resolver.nova`:
      `match std_variants.get(v.name.value) {` becomes `match std_variants.get("${v.name.value}_") {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  73. **STD_INTERNAL's names globbed into the user's modules.** `selfhost/src/resolver.nova`:
      `let from = if holds(STD_INTERNAL, name) { std_start } else { 0 }` becomes `let from = 0`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  74. **An imported std module globbed everywhere.** `selfhost/src/resolver.nova`:
      replace

      ```
              if !is_imported_std_module(name) {
                  let from
      ```

      with

      ```
              if true {
                  let from
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  75. **Std's glob overriding a module's own value.** `selfhost/src/resolver.nova`:
      `if !scope.values.contains_key(n) { let _ = scope.values.insert(n, r) }` becomes `let _ = scope.values.insert(n, r)`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  76. **Fixes kept in std's modules.** `selfhost/src/resolver.nova`:
      `mid < std_start && (package == -1 || package == 0)` becomes `true`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: unreachable in 5.2a: std's modules give no diagnostic, and a loose program is one package; a dependency's modules, whose fixes are dropped, come with 5.2b (decision 39).
  77. **The missing names' module.** `selfhost/src/resolver.nova`:
      replace

      ```
                  if !missing.contains_key(n) { let _ = missing.insert(n, s.name) }
              }
              for n in exp.type_names.iter() {
      ```

      with

      ```
                  if !missing.contains_key(n) { let _ = missing.insert(n, s.label) }
              }
              for n in exp.type_names.iter() {
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: WASM-only: natively every std module compiles, so `missing` is empty; 5.2b's WASM part prints it (decision 39).
  78. **E0084's async note.** `selfhost/src/resolver.nova`:
      `` an `async fn` returns a future, `` becomes `an async fn returns a future,`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: test mode only: `check` strips every `@test` function before resolving, so E0084 and E0085 come with 5.2b's test mode (decision 39).
  79. **A fix kept, without the item whose keyword is not found (O1).** `selfhost/src/resolver.nova`:
      replace

      ```
              let at = keyword_start(text, item_name.span.start)?
              edits.push(Edit::insert(at, m.label, "pub "))
      ```

      with

      ```
              match keyword_start(text, item_name.span.start) {
                  Some(at) => edits.push(Edit::insert(at, m.label, "pub "))
                  None => ()
              }
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  80. **`make_public` without its `const` arm (O2).** `selfhost/src/resolver.nova`:
      `ItemConst(c) => (c.vis, c.name)`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  81. **`binds` blind to the types namespace (O3).** `selfhost/src/resolver.nova`:
      replace

      ```
      scope.values.contains_key(name)
              || scope.types.contains_key(name)
      ```

      with

      ```
      scope.values.contains_key(name)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  82. **E0083's "an impl block" reworded (O4).** `selfhost/src/resolver.nova`:
      `"an impl block"` becomes `"an impl"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  83. **`()` named `unit` in mangled names (O5).** `selfhost/src/resolver.nova`:
      replace

      ```
                  if items.is_empty() {
                      "anon"
      ```

      with

      ```
                  if items.is_empty() {
                      "unit"
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  84. **`type_full_name`'s `anon` fallback renamed (O6).** `selfhost/src/resolver.nova`:
      replace

      ```
              _ => "anon"
          }
      }
      ```

      with

      ```
              _ => "anon_"
          }
      }
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  85. **`keyword_start` not past `async` (O7).** `selfhost/src/diagnostic.nova`:
      `if text.byte_slice(word, end) == "async" { return Some(word) }` becomes `let _ = word`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `importing_a_private_item_offers_pub_where_its_keyword_starts`.
  86. **A `nova.toml` directory taken for a package's (O8).** `selfhost/src/loader.nova`:
      `e.name == "nova.toml" && e.is_file` becomes `e.name == "nova.toml"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`.
  87. **A root with no directory part lists `""`, not `.` (G1).** `selfhost/src/loader.nova`:
      `let listed = if self.dir == "" { "." } else { self.dir }` becomes `let listed = self.dir`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_with_no_directory_part_or_in_a_package_loads_alike`.
  88. **A package's directory `""` listed as is (G1).** `selfhost/src/loader.nova`:
      `let listed = if root == "" { "." } else { root }` becomes `let listed = root`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_with_no_directory_part_or_in_a_package_loads_alike`.
  89. **A package's `tests/` not seen (G1).** `selfhost/src/loader.nova`:
      `if dir_name != "src" && dir_name != "tests" { return false }` becomes `if dir_name != "src" { return false }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_with_no_directory_part_or_in_a_package_loads_alike`.
  90. **The form's files on clap's `PathBuf` parser, which refuses `""` (G2).** `crates/nova-cli/src/cmd/run.rs`:
      replace

      ```
          #[arg(value_parser = OsStringValueParser::new().map(PathBuf::from))]
          files: Vec<PathBuf>,
      ```

      with

      ```
          #[arg()]
          files: Vec<PathBuf>,
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_with_no_directory_part_or_in_a_package_loads_alike`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`.
  91. **Both sides keep the `@test` functions, so E0084 and E0085 reach the corpus's and the cases' dumps (G3).** `crates/nova-driver/src/lib.rs`:
      replace

      ```
              if !failed {
                  strip_test_functions(&mut modules, false);
      ```

      with

      ```
              if failed {
                  strip_test_functions(&mut modules, false);
      ```

      And in `selfhost/src/main.nova`, `if !failed { strip_test_functions(modules) }` becomes `if failed { strip_test_functions(modules) }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  92. **A STD_ONLY builtin dropped.** `selfhost/src/builtins.nova`:
      replace

      ```
      pub const STD_ONLY: [String] = [
          "StrCmp",
      ```

      with

      ```
      pub const STD_ONLY: [String] = [
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  93. **A GLOBAL builtin swapped for a STD_ONLY one.** `selfhost/src/builtins.nova`:
      `"EPrintln", "Panic"]` becomes `"EPrintln", "StrCmp"]`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_builtins_name_redefined_is_e0002_with_a_note`.
  94. **A builtin's name.** `selfhost/src/builtins.nova`:
      `"println",` becomes `"printline",`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`, `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's `@test` `a_builtins_name_redefined_is_e0002_with_a_note`.
  95. **A WASM_ONLY builtin dropped.** `selfhost/src/builtins.nova`:
      replace

      ```
      pub const WASM_ONLY: [String] = [
          "MemLoad8",
      ```

      with

      ```
      pub const WASM_ONLY: [String] = [
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: WASM-only: natively no scope holds WASM_ONLY's builtins; 5.2b's WASM part prints them (decision 39).
  96. **`Bytes` no reserved type name.** `selfhost/src/builtins.nova`:
      replace

      ```
          "Future",
          "Bytes",
      ]
      ```

      with

      ```
          "Future",
      ]
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  97. **Two std modules swapped.** `selfhost/src/builtins.nova`:
      replace

      ```
          ("$std.io", "io/lib.nova"),
          ("$std.fs", "fs/lib.nova"),
      ```

      with

      ```
          ("$std.fs", "fs/lib.nova"),
          ("$std.io", "io/lib.nova"),
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  98. **N's scopes unsorted.** `selfhost/src/resolution_dump.nova`:
      `for name in sort_strings(Vec::from_array(scope.values.keys())).iter() {` becomes `for name in scope.values.keys().iter() {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  99. **N's imports unsorted.** `selfhost/src/resolution_dump.nova`:
      `for name in sort_strings(Vec::from_array(m.imports.keys())).iter() {` becomes `for name in m.imports.keys().iter() {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  100. **N's StdStart def off by one.** `selfhost/src/resolution_dump.nova`:
       `d.field("def", "${std_def}")` becomes `d.field("def", "${std_def + 1}")`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `a_root_spelled_any_way_names_its_siblings_alike`, `a_root_whose_name_needs_escapes_is_printed_alike`, `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  101. **N's exports in the map's order.** `selfhost/src/resolution_dump.nova`:
       `for name in e.value_names.iter() { self.value(name, e.value(name)) }` becomes `for name in e.values.keys().iter() { self.value(name, e.value(name)) }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  102. **N's arity off by one.** `selfhost/src/resolution_dump.nova`:
       `self.field("arity", "${v.arity}")` becomes `self.field("arity", "${v.arity + 1}")`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  103. **N's secondary label printed as primary.** `selfhost/src/resolution_dump.nova`:
       `if label.primary { "Primary" } else { "Secondary" }` becomes `if label.primary { "Primary" } else { "Primary" }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  104. **N's edit text dropped.** `selfhost/src/resolution_dump.nova`:
       `self.string(edit.text)`: the line goes.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  105. **N's strings unescaped.** `selfhost/src/resolution_dump.nova`:
       `push_escaped(self.out, text, true)` becomes `self.out.push(text)`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `a_root_whose_name_needs_escapes_is_printed_alike`.
  106. **A warning printed as an error.** `selfhost/src/resolution_dump.nova`:
       `SeverityWarning => "Warning"` becomes `SeverityWarning => "Error"`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`, `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`, `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
- [ ] **Step 9: The review's inputs.** The full run's figures, the new
  tests by binary, the golden suite's line and time, the parse, check,
  help and usage comparisons, the differential's times and the codes, and
  the mutants' outcome, with each gap, go in the ledger.
- [ ] **Step 10: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0053 and this plan, and is told to hunt for:
    - a Windows-only difference: a path S0 forms with `Path::join` or
      `display` that reaches a dump or a message N compares, a test that
      compares a directory part on Windows, a test skipped on Windows that
      hides a failure, a line ending, a case-insensitive disk (decisions
      27 and 32); a fixture read by `include_str!` or `read_to_string`
      and compared byte for byte, which a Windows checkout gives `\r\n`
      (Review Focus 1);
    - an input on which N's loader or resolver and S0's part that no case
      or fuzz input makes: a message, label, note or fix worded otherwise;
      a builtin or std table's entry; a mangled name; a definition's id or
      item index; the glob's order; the fix-dropping step; an attribute
      check; the paths no native input reaches (E0084, E0085, E0090,
      `missing`, `owned`, a reported import), read against the Rust
      (decisions 18, 25 and 39);
    - anything `parse_std` and `resolve_program_with_std` change for
      `check`, `build`, `run`, `test` or the language server: an order of
      diagnostics, std's parse errors, an id (decision 26);
    - an E0002 order changed beyond the glob's, or a map's order reaching
      any output (decision 11);
    - the dump on either side against ADR 0053 §1: an id, `StdStart`, a
      sort, an escape, a reported import, `UNSUPPORTED package`, a
      header, `IOERR`'s exit code; an example the ADR holds that S0 no
      longer prints (decisions 12 and 14);
    - a way for the harness to pass when the two sides differ: an input
      set that skips the resolution pass, a form not listed or not read,
      an exit code not checked, the codes' floor, a code ruling 8 defers
      not asserted absent, a case not compared, an input that never
      reaches the resolver, a root's directory part or package the tests
      never vary (decisions 8, 14, 16, 21 and 30);
    - argument handling: the hidden flags shown anywhere, a tip or usage
      line naming them, any command's `--help` changed, any `nova check`
      command line not starting with `--dump-resolution` that differs
      from the base's in a byte, under any first argument or in a
      terminal; the form's own refusals; a conflict with `--target`
      (decisions 13 and 31);
    - something that breaks only under repetition or another order: the
      fuzzer's inputs differing by system or listing, a fixed directory
      shared, a stale N, a stale record figure;
    - a commit between Task 1 and Task 7 that builds wrong code or fails
      the suite (decisions 34 to 37);
    - the records: ADR 0053, the CHANGELOG, the dated notes, the branch
      name in each, every count (251, 103, 43, 2,596) and line count
      (by `loc.py`'s method) against the tree;
    - each mutant of Step 8 that passes, the accepted ones included, and
      whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock`, a test that
      touches the internet, a temporary name that is not fixed or missing
      from the Global Constraints' list.
  - Each finding is fixed in one commit, `5.2a: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Steps 1 and 2 run again.
- [ ] **Step 11: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-52a`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 5.2a adds: N's loader, resolver and resolution dump, `selfhost
      resolve`, N's `@test`s; S0's `nova_resolver::dump`, `parse_std`,
      `ResolutionDumps` and the hidden `nova check --dump-resolution`; the
      cases, the name fuzzer and the resolution comparison; ADR 0053;
    - the change a user sees: a glob import's E0002s in declaration
      order; every `.nova` file the base tracks parses and checks as
      before, every command's `--help` is unchanged, and every `nova
      check` command line but the hidden form's gives what it gave;
    - the departures: eight of the 11 codes; S0 parses std once for the
      dump; Windows paths settled in the harness; `ImportTarget` beside
      N's loader;
    - the figures: the suite's `2857 1 10`, the golden suite's
      `147 passed, 9 excluded`, the differential's time here and the
      estimate from `main`'s CI (27 to 40 s), N's speed (1.5 to 2.0× S0
      debug over the corpus), the fuzzer's codes, and that `Cargo.lock` is unchanged;
    - the user's decisions, and this plan's decisions, numbered as here,
      with decisions 25 to 28 marked as differing from the spec;
    - the mutants, those that pass by design and any gap they found, and
      the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems; on the
    `test` job's `ubuntu-24.04` leg, `cargo test`'s log holds
    `test release_golden_suite ... ok`, and on each system `selfhost.rs`'s
    tests pass; record each system's time for it, and the run's address,
    in the ledger.
  - **If the differential fails on CI and not here,** record its report,
    stop and ask the user: a Windows path or line ending is the likeliest
    (decision 27). **If `selfhost.rs` takes more than a minute on a
    system,** ask the user before lowering the counts (spec §9 risk 4;
    decision 23).
- [ ] **Step 12: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 11's.
