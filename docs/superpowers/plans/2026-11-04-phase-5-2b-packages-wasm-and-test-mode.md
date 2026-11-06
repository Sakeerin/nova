# Phase 5.2b, "Packages, the WASM target and test mode", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** The second of Phase 5.2's two parts. S0's hidden resolution
dump (`nova check --dump-resolution`, ADR 0053) gains packages (a root
directly in a package's `src/` or `tests/` is a module of that package,
with its dependencies' libraries), the WASM target (`--target wasm`) and
test mode (`--test`, as `nova test` resolves). N (`selfhost/`) gains a
minimal manifest reader and the package graph, the package loader, the
WASM target and test mode in `selfhost resolve`, and prints the same dump
for the same roots. The differential (`crates/nova-cli/tests/selfhost.rs`)
compares the two on std's three parts, the corpus in three modes (318
programs natively: 251 loose and 67 package files; 39 for WASM; 9 with
tests), 145 / 30 / 126 cases (with Windows line endings too), the name
fuzzer's check, test, WASM and package inputs, and the refusals, on Linux,
macOS and Windows. Both sides resolve std once per process for the dump
(option B, the user's choice when CI's Windows leg measured 76.59 s).
Nothing a user runs changes: `nova check`, `nova build`, `nova test` and
the language server keep their path. ADR 0054 records the dump's new
forms, N's reader, the departures and the figures, the live heap among
them.

**Architecture:**
- **S0** (Task 1): `crates/nova-driver/src/program.rs`'s
  `Program::for_file_with` (std's files first in the database);
  `crates/nova-driver/src/lib.rs`'s `ResolutionDumps::new(target, test)`
  with the package graph's M-codes first, manifest labels, test stripping
  and naming, `register_std_for`; `crates/nova-resolver/src/dump.rs`'s
  package key, ` mode=test`, `(Test`, `(Missing`;
  `crates/nova-resolver/src/lib.rs`'s `SharedStd`,
  `resolve_program_with_shared_std`, `Exports::rebased`,
  `variant_shadowing_of`; `crates/nova-diagnostics/src/files.rs`'s
  `FileDb: Clone`; `crates/nova-cli/src/cmd/run.rs`'s hidden `--test`.
- **N** (Task 2): `selfhost/src/manifest.nova` (new), `loader.nova`'s
  graph and package loader, `resolver.nova`'s `share_std` and
  `resolve_program_shared`, `resolution_dump.nova`'s ` mode=test`,
  `main.nova`'s `selfhost resolve --target … --test`; N's `@test`s in
  `loader_tests.nova`.
- **The differential** (Task 3): `selfhost.rs`'s modes, case
  directories, package corpus, refusals and the speed-up;
  `tests/selfhost/fuzz.rs`'s WASM and package mutations;
  `tests/selfhost/compare.rs`'s `settle_paths`; 331 new files under
  `tests/selfhost/cases/resolve/`, `resolve-wasm/` and `resolve-test/`;
  `dump.rs`'s `KINDS` with 5.2b's forms.
- **Records** (Task 4): ADR 0054 (new); a dated note in ADR 0053;
  `CHANGELOG.md`; dated notes in `docs/phase-5-plan.md`,
  `nova-spec/00-MASTER-SPEC.md` and `nova-spec/11-PARSER.md`.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §6).
- N is built by `nova build` through Cranelift. `clang-18` (CI's pin) for
  the golden suite, and Node 22 for the WASM tests, as before.

**Spec:**
`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`,
approved by the user on 2026-11-02, on this branch's base.
- Read its header, §4 (5.2b), §§1, 2 and 5 to 10 before Task 1.
- The user's decisions are decisions 1 to 8 here, with the dated lines
  of 2026-11-04 (decisions 9 to 11) and of 2026-11-05 (decision 24); the
  defaults taken for part B, "not asked", are decision 12.

**Base:** `2d38ad7a` ("5.2a: the review's fixes"), `main` after 5.2a,
whose full suite gives `2858 1 10` (the IPv6 test the only failure here).
The branch is `claude/loving-archimedes-frtf5q-52b`, from `2d38ad7a`.

**Checked on 2026-11-05, before execution:**
- **A prototype built 5.2b whole** on a local branch from `2d38ad7a`, in
  this tree: part A, S0's side, `4ea4913c` (the dump of packages, the
  WASM target and test mode, and the cases), `6d7d60e4` (the fuzzer's WASM
  and package siblings, every code's floor), `77145653` (ADR 0054's part
  A); part B, N's side, `78220638` (N's packages, WASM target and test
  mode, and the differential on both sides), `c0c9ad1a` (the fuzzer's
  seeds and the CRLF cases side by side), `4d6d26e9` (ADR 0054's part B,
  the live heap); then step 0, `56ed80c5`, "5.2b prototype: before the
  split" (decisions 14 to 17 and 19), and `0f7c01d8`, "5.2b prototype: before
  the split, std resolved once per process" (decision 18), and
  `d8dae46a`, "5.2b prototype: before the split, the mutants' gaps"
  (decision 21), and `5d48e977`, "5.2b prototype: before the split, the
  live heap measured again" (decision 22), and `bffca307`, "5.2b
  prototype: the first fact-check's fixes" (decision 23), and
  `3e0c5099`, "5.2b prototype: the second fact-check's fixes" (decision
  25). No difference
  between the sides was left: N's first full run agreed on every input
  but the manifests it refused, now read (decision 16).
- **Step 0, before the split:**
  - **CI time** (decision 17, and 18 **(differs)**): one process a side
    per set and mode, the fuzzer's seeds in one process a mode, 5.1's
    comparisons shared, CRLF copies only where the bytes change: 51.6,
    49.5, 51.9 s became 49.2, 47.3, 48.7 s locally. CI then measured
    `56ed80c5` at Linux 35.59 s, macOS 29.54 s, Windows 76.59 s; the user
    chose option B, std resolved once per process on both sides: 40.6,
    40.3, 40.8 s became 28.5, 27.3, 27.0 s on one machine state (5.1's 27
    tests alone 18.1, 18.2, 18.0 s). The projection from these local
    times, Linux 24-25 s, macOS 20-21 s, Windows 51-54 s, was wrong: CI
    measured more (decision 24).
  - **Windows and macOS, the edges** (decisions 14 **(differs)** and
    15): every edge
    the plan's review asked for probed on both sides, alike; a dependency on the
    package itself, two entries of one path and a dev-dependency that is
    also a dependency each an S0 M-code and an N refusal (exit 1);
    `both_sides_take_test_in_every_position_alike` (6 shapes, 10 after
    the first fact-check added `--target=VALUE`, 15 after the second's
    empty and repeated `--target` and `--test=VALUE`);
    `usagecmp.sh` extended for `--test`; a root typed `src\main.nova` on
    Windows left to 5.5.
  - **N's reader against S0's** (decision 16 **(differs)**): 211
    distinct manifests
    (the 100 tracked and those the suite writes), 131 read alike, 80
    refused where S0 gives an M-code, 0 that S0 accepts refused by N,
    after four fixes; a manifest ending in a bare `\r` or holding a
    control character now refused, as toml_edit refuses it; the reader's
    table pinned in N's tests. After the first fact-check: 229 distinct
    manifests (the 104 tracked and those the suite writes), 146 read
    alike, 83 refused where S0 gives an M-code, none that S0 accepts
    refused; the table 16 read and 51 refused.
  - **The records** (decision 19): ADR 0054 names the branch, one corpus
    count (318), the CI figures; the CHANGELOG says nothing user-visible
    changed, which `checkcmp.sh` and `pkgcmp.sh` check.
- **The split** (decision 20 **(differs)**): four tasks, not the seven
  of the split's brief, each the nearest clean cut: Task 1, S0's dump of
  packages, the WASM target and test mode with option B's S0 part; Task
  2, N's reader, loader, WASM target and test mode with N's sharing;
  Task 3, the cases, the fuzzer's new inputs and the differential with
  the harness's parallelism; Task 4, the records.
  `split.py` checks that the last commit's tree is `3e0c5099`'s
  (`git diff --stat 3e0c5099 split/5-2b` empty) and that every patch
  applies with plain `git apply`, in order, on `2d38ad7a`, with no
  warning, each applied tree its commit's, and that no added line holds
  trailing whitespace, a CR or a backslash-u escape.
- **The replay** (`verify.sh`, under the process limit) ran each step as
  written here: each `tN-tests` failed as Steps 2 record it (Task 1's
  library tests do not compile and its three harness tests fail; Task
  2's N tests do not compile; Task 3's harness does not compile), and
  each `tN-code` passed with the whole suite, `2862 1 10`, `2862 1 10`
  and `2869 1 10`, the IPv6 test the one failure, its formatters and
  clippy clean; Task 4's ADR examples were identical. Logs:
  `verify.sh`'s, one a step, at `da9b88c0`.
- **At the last commit:**
  - the full suite: `2869 1 10`, the base's `2858 1 10` and 11
    tests more: 12 added and 1 gone, the fuzzer's test renamed (Task 5,
    Step 1); the one failure is
    the IPv6 test;
  - CI's Linux leg with `--all-features`: `2869 1 10` (Task 5, Step 2);
  - the golden suite: `release golden: 147 passed, 9 excluded, Ubuntu
    clang version 18.1.3 (1ubuntu1)`, in 17 s (Task 5, Step 3);
  - `nova parse` over every base-tracked `.nova` file: `parse: 1470 runs
    over 735 files, 0 differ; --check exits nonzero on 119 at the base,
    119 now`;
    `nova check`: `check: 746 checks, 0 outside std differ, 0 std files
    differ`; `nova test` and `nova build` on every
    package: `packages: 22 runs, 0 differ` (Task 5, Step 4);
  - `--help`: `help: 22 of 22 identical; hidden flags shown: 0`;
    `usagecmp.sh`: `usage: 246 of 246 runs identical; in a terminal, 10
    of 10; --dump-ast as --dump-tokens: 25 of 25; --dump-resolution
    refused: 114 of 114` (Task 5, Step 5);
  - Rust 1.78: clean, `Finished` in 58 s; `Cargo.lock` and every
    `Cargo.toml` unchanged
    (Task 5, Step 6);
  - `selfhost.rs` three times: 20.2, 20.2 and 20.5 s (user CPU 64 to 65
    s; the machine was faster after a container restart than on the days
    of decisions 18 and 25); 5.1's 27 tests alone:
    13.3, 12.8 and 12.9 s; N's `nova test`: `test result: ok. 32 passed;
    0 failed; 0 trapped; 32 total` (Task 5, Step 7);
  - the live heap, rerun: `selfhost/src/main.nova` 36.9 MB peak and a
    93.5 ms longest pause with the debug runtime, 36.1 MB and 29.0 ms
    with a release one; std's parts 16.0 to 19.0 MB. The peak is about 4
    MB above part B's (32.8 and 31.8 MB): std's state, kept for every
    root under option B; ADR 0054 §10 now holds these figures; the
    plan's `heap.sh` run again at the split's last commit gave 37.0 MB
    and 63.5 ms, 35.9 MB and 19.1 ms (the pauses shorter on the faster
    machine state of the replay) (Task 5, Step 7).
- **The mutants** (decision 21): 140 breakages of what 5.2b adds, each a
  text replaced in one file, run one at a time at the split's last
  commit under the process limit (Task 5, Step 8): S0's dump forms,
  labels, key, M-code stop, std registered first, test mode's keeping,
  stripping, naming and `std/test`, `--test`'s parsing and refusal,
  option B's rebasing, fallback, `@test` check, `missing` and mode (22);
  the harness's modes, case directories, `settle_paths` and `run`'s
  settling, floors, process split and fuzzer, its manifest forms, form
  3's line endings and modes included (19); and N (99): the manifest
  reader's forms, refusals, spans, u64 limit and requirements, the
  graph's checks and version entries, identity's message and its
  climbing and one-sided checks, the loader's labels, roots, visibility,
  E0004, both notes and stripping, the WASM target (each of the 13
  WASM-only builtins, `ui` modules, E0090, `missing`), test mode,
  `--target=`, an empty or repeated `--target` and `--test=VALUE`, the
  refusal's exit code, option B's rebasing (the trait offset included),
  fallback and `@test` check. 135 were caught and 5 pass by design, in
  56 minutes. The first run, at the split before `d8dae46a`, found 11
  gaps among those that passed: the process split dropping a root (the
  dealt-back parts were counted, not named), S0's M-code stop and a
  dependency's manifest label, N's graph refusing a package with no
  entry, a keyword library and a dependency on itself only by luck of
  another check, a dependency's dev-dependencies, and three manifest
  refusals another check backed up. `d8dae46a` closes them: each
  dealt-back part must be its root's and none left over;
  `a_manifest_n_cannot_read_stops_n_with_a_message` checks that an error
  stops S0 before the loader; the case
  `a_dependencys_e0004_names_its_own_manifest` (E0004 inside a
  dependency, whose manifest also has a dev-dependency the graph never
  reads); N's graph test checks each refusal's words, with three more
  packages; N's reader test pins three refusals' words. The first
  fact-check added 20 (the u64 limit, the requirement reader, the
  dev-dependency version entries, identity's message, `--target=`, the
  std `@test` sharing on both sides, the checker's four reader mutants,
  and the fuzzer's manifest forms and modes) and made the two fallback
  mutants catchable, each caught now; its run at the split found one
  gap, the manifest forms written in one form passing, which the
  fuzzer's check that E0004 names its entry on more than one line of a
  manifest closes (`bffca307`). The second fact-check added 15 (`run`
  settling no resolution dump, S0's only, N's only or every dump; the
  identity clause without the climbing check, reading one package's path
  only, taking `..` alone, or one-sided for an absolute and a relative
  path; N taking `--target ""`, marking no `--target VALUE` given,
  taking a second `--target=VALUE`, taking `--test=VALUE`; N's rebasing
  without the trait offset; the fuzzer's form 3 without `\r\n`), each
  caught; all 140 ran again at the split's last commit, `da9b88c0`,
  after the second fact-check's fixes (`3e0c5099`). The rest pass by
  design (decision 21).
- **The first fact-check** (decision 23), of the plan and the split at
  `5d48e977`, ran 35,955 inputs through both sides and found: N reading a
  version whose number is past `u64::MAX`; N identifying a package by its
  folded path where S0 reads `real_path` (one directory spelled relative
  and absolute, or through a link); the std-diagnostic fallback untested
  on both sides, and `shareable` blind to a std `@test`; four reader
  mutants surviving (DEL, a 65-character name, `1.0.0.0`, `[package]`'s
  span); N refusing a dependency's dev-dependency version entry, which
  S0 never resolves; the fuzzer's package inputs in one manifest form and
  one mode; and record errors (the added-and-gone count, `names-301` to
  `303`'s phase, the tracked manifests' count, Review Focus 3, the MSRV
  sentence, decision 20's count, the driver's doc comments), and nits
  (the `?` label, Task 1 Step 2's build order, `--target=wasm`, the CI
  projection). `bffca307` fixes each; decision 23 says how, and what
  stays (differs).
- **Real CI** (decision 24): the tree after the first fact-check's fixes
  (`bffca307`, the split's tree then) ran CI run 42006844810 from a
  separate branch with a draft pull request, closed after:
  `selfhost.rs` on Linux 37.90 s (42 tests), macOS 35.77 s (40), Windows
  60.48 s (36). The local projection had said Linux 24-25 s: it was
  wrong. One test failed on Windows only:
  `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`
  compared S0's E0004, whose paths S0 writes with `\` on Windows, with
  N's without settling them. The user accepted about 60 s on Windows,
  with no cut, to be read again on the PR's CI and asked again past 70 s.
- **The second fact-check** (decision 25), of the plan and the split at
  `bffca307`, found: N's package identity differing also on a path that
  climbs above the current directory and comes back down (`run`,
  `../../run/geom`: S0 one package, N M0011 without the "may be one
  directory" clause), and decision 23's statement of which inputs differ
  wrong; N taking `--target ""` (and then a second `--target`) and
  `--test=VALUE`, which clap refuses for S0; three surviving mutants (N's
  rebasing without the trait offset, the identity clause made
  one-sided, the `--target=VALUE` repeat check removed) and a reach gap
  (the fuzzer's form 3 losing `\r\n`); and record errors (the suites,
  the case-file counts, which head the mutants ran at, ADR 0054's
  Status against the plan, Task 4's manifest count, Review Focus and
  the reviewer's list without package identity), and nits (Task 1 Step
  4's Expected, `settle_paths`' citation, Task 2's patch size, the scaled
  projection's arithmetic, spec §4.5's WASM corpus). `3e0c5099` fixes
  each and the CI failure, and every mutant ran again at the split's
  last commit; decision 25 says how.
- **What the checks left standing,** each recorded or handed on:
  - macOS and Windows are reasoned from the code here and were run once
    on CI (decision 24), before the second fact-check's fixes; the PR's
    runs are the check (decisions 14, 18 and 24);
  - a root typed with `\` on Windows, which N reads as one name, and a
    drive in N's paths (5.5, decisions 14 and 23);
  - package identity through a link, or, under a relative root, an
    absolute dependency path or one climbing above the current
    directory, or, on a case-insensitive disk, one directory spelled two
    ways by case: N refuses where S0 reads one package, and `x/..` after
    a link, which N folds (decision 23, ADR 0054 §5); no comparison
    input has any of the three;
  - the mutants that pass by design (decision 21).
## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` under `RUSTUP_TOOLCHAIN=1.78.0`
  with `RUSTFLAGS=-D warnings`. No `std::sync::LazyLock` (1.80), no
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79). 5.2b's Rust
  adds `Option::filter` (1.27), `Iterator::step_by` and `slice::from_ref`
  (1.28), `str::strip_prefix` and `strip_suffix` (1.45), `bool::then` and
  `Ord::clamp` (1.50), `str::split_inclusive` (1.51), `std::thread::scope`
  (1.63), `let`-`else` (1.65), `Option::is_some_and`, `Result::is_ok_and`
  and `std::sync::OnceLock` (1.70), and a derived `Clone`: each at most
  1.78 (a `grep` over the diff's added lines).
- **No new crate, and `Cargo.lock` does not change** (spec §6). The order
  stops the task: ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`. 5.2b's Rust outside tests adds none; N's
  `unwrap()`s are on values its own code has checked.
- **No `unsafe`.** 5.2b adds none.
- **Tests never touch the internet,** bind no port, and need no `clang`
  (spec §6). The differential runs `nova` and the N it builds, nothing
  else.
- **Temporary names are fixed** (spec §6): under `std::env::temp_dir()`,
  `nova-selfhost-<name>`; 5.2b adds `resolution-refused`, `names` and
  `wasm-names` (the fuzzer's seeds, which replace 5.2a's `names-301` to
  `names-303`), `package-names-321` to `package-names-323`,
  `resolution-identity`,
  `altered-std-0` and `altered-std-1`, `trait-std` and `trait-roots`
  (the trait-offset test); the driver's
  unit tests' `nova-driver-resolution-<name>`; and N's own tests', under
  Nova's `temp_dir()`, `nova-selfhost-package-test`,
  `-package-clash-test`, `-package-of-test` and `-graph-test`
  (`selfhost/src/loader_tests.nova`). These are every fixed temporary
  name in the diff (`grep -n 'fresh_dir(\|write_.*_inputs(\|temp_dir'`
  over its added lines). Each is removed and made afresh by the test that
  uses it. A test that needs `$NOVA_HOME` sets a fresh one.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and the golden suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the suite here with `NOVA_REQUIRE_NODE=1`. No test of 5.2b needs Node.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and
  passes. 5.2b adds no runtime program, so it stays at 147.
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
  `/tmp/nova-todo-*`, which is never cleaned: this plan's six runs take
  about 1.8 GB, so begin with room for them (the replay began with
  7.7G and ended with 7.1G). Every `cargo` command here
  selects the whole workspace (Conventions), so cargo keeps one copy of
  each test executable. Task 5's Rust 1.78 check uses its own target
  directory, in `/dev/shm`, removed after.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.2b's
  patches holds one. Apply the patches with `patch_block`, from the
  committed plan.
- **No patch adds a CR, a NUL or trailing whitespace.** The inputs that
  need them are written by the tests at run time (the CRLF cases, the
  manifests with a BOM or a bare `\r`), not kept as files. The cases'
  `nova.toml` files are LF in the repository; a Windows checkout may give
  them CRLF, which both readers take.
- **Nothing writes into the repository but the patches.** `nova build -o`
  writes N under the system's temporary directory and `nova test` writes
  nothing into the package; `git status --short` is empty after every run.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.2b.
- **This machine is Linux.** macOS and Windows results come from CI; what
  differs there is reasoned from the code (decision 14).

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-52b.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-52b-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-52b-<task>.txt"`. A task with a tests
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
  mutants ran under it on 4 CPUs, and peaked at 153. Test threads
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
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-52b"
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
  copy. `S` gives each code step the whole suite, about five
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
  PLAN=docs/superpowers/plans/2026-11-04-phase-5-2b-packages-wasm-and-test-mode.md   # this file, committed
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
  nothing: no 5.2b patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. Task 3's tests patch
  adds 331 case files and Task 2's code patch 1,556 added lines (a
  2,049-line patch):
  each is one block all the same. The non-ASCII characters the patches add are
  `§` (Tasks 1 to 4), and `—` and `…` (Task 4). A block that does not
  apply means the tree differs from
  the plan's: stop, and record why in the ledger.
- **Each task** but Task 4 is test first: its tests' patch, their run and
  the expected failure, then its code's patch, its run and the expected
  pass. Task 4's patch is the records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy,
    `cargo clippy --locked --all-targets --all-features -- -D warnings`, as
    CI runs it, which ends with `Finished` and no warning;
  - `target/debug/nova fmt --check std examples tests/wasm tests/dom
    tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost`,
    `selfhost` included, as CI runs it since 5.1a, with the `nova` the
    step's build made, which prints nothing.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost
  if wrong.


## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
check in the task that owns the code:

1. **Windows paths and case-insensitive disks** (`program.rs`'s
   `package_of` and N's `package_of`, `path_parent`, `path_join`; the
   paths in E0004's message, package E0093's, the refusal line
   `cannot dump ROOT: <manifest>: why`, the manifest labels, `tests/stem`
   and `import_name/stem`): labels are built from names, never from
   paths, so they are `/` on every system; E0004's and E0093's messages
   hold a path, which S0 writes with `\` on Windows and N with `/`, and
   the harness reads `\` as `/` in those two messages only, and only on
   Windows (`settle_paths`, decision 14); every comparison of S0's and
   N's output takes it from the harness's `run`, which settles every
   resolution dump, so a comparison built another way must settle too:
   CI's Windows leg failed on one that did not, which no Linux run could
   see (decision 25); `Nova.toml` and `SRC/` on a case-folding disk are
   found alike, the manifest by its path, the directory by its listed
   name (decision 14) →
   `the_paths_of_e0004_and_e0093_are_settled_on_windows_alone`,
   `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
   `a_package_is_found_by_its_manifests_path_on_both_sides` (Task 3)
   (mutants 22, 23,
   126 to 129). A root typed
   `src\main.nova` on Windows is one name to N: 5.5's (decision 14). The
   PR's Windows and macOS legs are the runs.
2. **N's messages word for word** (`manifest.nova`'s refusals,
   `loader.nova`'s E0004, the two E0001 notes, the graph's M-code
   refusals; `main.nova`'s usage line and refusal line) against S0's
   `nova-pm` and `program.rs` → the differential on the cases and the
   fuzzer, `a_manifest_n_cannot_read_stops_n_with_a_message`, N's
   `@test`s (Tasks 2 and 3) (mutants 65 to
   70).
3. **The manifest reader's refusals** (`parse_manifest`, `is_version`,
   `is_u64`, `is_requirement`, `check_name`, the graph's checks): N
   refuses exactly ADR 0054 §5's list, and no input of the comparison
   that S0 accepts is refused by N; the 229 manifests the suite parses
   were run through both, 146 read alike and 83 refused where S0 gives an
   M-code (decisions 16 and 23) →
   `the_manifest_readers_accepted_and_refused_forms_are_pinned`,
   `the_graph_refuses_what_nova_pm_gives_an_m_code` (Task 2) (mutants
   34 to 51). The reviewer reads the
   reader's table in ADR 0054 §5 against `nova-pm`'s `manifest.rs`.
4. **The speed-up keeps every input** (`selfhost.rs`'s
   `agree_on_resolution_in_parts`, the seeds' directories, `Compared`,
   `copy_crlf`; option B's `SharedStd` on both sides): every root is still
   compared, each in its mode, and each part counted by its header
   (decisions 17 and 18) → the part counts in
   `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
   `std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`
   (Tasks 1 and 3) (mutants 15 to
   21, 30, 31,
   100 to 105). The reviewer
   reads `resolve_program_with_shared_std` against `resolve_program_for`
   step by step: the shared path must be the full one with std's work
   done once.
5. **Nothing a user runs changes** (`resolve_program_for`, `FileDb`,
   `variant_shadowing`, `Program::for_file`): only the dump calls the new
   functions → `checkcmp.sh`, `pkgcmp.sh`, `helpcmp.sh`, `usagecmp.sh`
   (Task 5, Steps 4 and 5).
6. **Test mode** (`strip_test_functions`'s `keep_root_tests`,
   `name_tests`, `std/test` joined): the root package's and a loose
   program's tests kept, a dependency's stripped, a `tests/` file's
   named `<stem>::<function>`, as `nova test` collects them →
   `test_mode_dumps_the_tests_nova_test_collects` (Task 1),
   `test_mode_keeps_the_root_packages_tests_and_strips_a_dependencys`
   (Task 2) (mutants 8 to
   11, 72, 73,
   78 to 82).
7. **Argument handling** (`--test` in every position, `--test --target
   wasm`, repeated options, `--` as a root name) on both sides and against
   the base → `both_sides_take_test_in_every_position_alike`,
   `the_resolution_dump_flags_are_hidden_and_refused_as_before` (Task 3),
   `usagecmp.sh` (Task 5).
8. **Package identity** (`loader.nova`'s `normal_path`, `climbs_above`
   and the graph's M0011; decision 23 **(differs)**): N folds the paths
   as spelled, where S0 reads `real_path`. They can differ only on a
   path through a link, or, under a relative root, an absolute
   dependency path or one climbing above the current directory, or, on
   a case-insensitive disk, one directory spelled two ways by case; N
   then refuses as M0011 (with the "may be one directory" clause for the
   second kind) where S0 reads one package, or, after a link, dumps
   another package. No comparison input reaches any of the three →
   `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`
   (Task 3, Unix) (mutants 117, 130 to
   134). The reviewer checks the list of inputs
   in ADR 0054 §5 against `normal_path` and nova-pm's `real_path`.

## Decisions

The user's, from the spec's header (2026-11-02) and the ruling of
2026-11-03:

1. **5.2 is the loader and the resolver, HIR 5.3's** (the user's decision
   1).
2. **N reads std from disk, through `--std DIR`** (the user's decision 2):
   the WASM target's and test mode's std files too.
3. **S0 keeps insertion order where an error order depended on a hash
   map** (the user's decision 3): 5.2a's; nothing here reorders.
4. **Two pull requests** (the user's decision 4): this is 5.2b, packages
   (`nova.toml`, sibling modules, the path dependency), the WASM target,
   test mode and the live heap's figures.
5. **The dump is 5.1's S-expression form, std's part dumped once** (the
   user's decision 5): std's part once per target and mode (`--std`,
   `--std --target wasm`, `--std --test`); a program's part prints ids
   relative to std.
6. **The inputs come from a name-level fuzzer and the Rust tests'
   sources** (the user's decision 6): the fuzzer's WASM and package
   siblings, and the cases extracted from the driver's, the resolver's,
   `run_tests`', `wasm_tests`' and `project`'s tests (ADR 0054 §§2, 3).
7. **The live heap is measured with outside tools** (the user's decision
   7): ADR 0054 §10, no runtime change.
8. **E0084, E0085 and E0090 are 5.2b's** (the user's decision 8,
   2026-11-03): E0084 and E0085 under test mode, E0090 for the WASM
   target, each at least twice in the fuzzer's dumps and in the cases,
   and absent from every other mode's dumps.

The dated lines of 2026-11-04:

9. **The credentials** (the user's lines of 2026-11-04): the crates.io
   token revoked, `CARGO_REGISTRY_TOKEN` deleted, the Open VSX token
   deleted. Nothing in 5.2b publishes.
10. **CI measured before choosing** (the user's line of 2026-11-04): the
    prototype's `56ed80c5` was pushed to a separate branch with a draft
    pull request, never merged and closed after; CI run 41813500363 (all
    16 jobs green) measured `selfhost.rs` at Linux 35.59 s (40 tests),
    macOS 29.54 s (38), Windows 76.59 s (35). Windows was over the
    minute of spec risk 4, so the user was asked.
11. **Option B** (the user's choice, 2026-11-04): decision 18.

The defaults taken for part B, "not asked" (2026-11-04):

12. **(differs)** Five points the spec leaves open or states otherwise,
    each recorded in ADR 0054 §8:
    - **Roots are files:** a file directly in a package's `src/` or
      `tests/`, or a loose program. `Program::for_file` already makes the
      first a module of its package; `nova test`'s whole-package program
      is not a root.
    - **M-codes stay S0's:** the package graph's errors come first in
      S0's dump (`resolved=false`, before the loader); N's reader is
      minimal and refuses what it does not read, on stderr with exit 1,
      and reads dev-dependencies, a library's existence and the entries'
      spans, which E0004 and the notes need. No comparison input has an
      M-code.
    - **Spec §4.1's note "a dependency without `src/lib.nova`"** is not
      what happens: the graph refuses such a dependency first (M0009);
      E0001's note comes only when a package with no library is imported
      by its own name from its own `tests/`. ADR 0054 corrects it; the
      spec is left as approved.
    - **Windows paths are compared with `\` read as `/`** in E0004's and
      E0093's messages (decision 14).
    - **Test names** are `name_tests`', `<file stem>::<function>` for a
      `tests/` file's.

The spec, as built:

13. **The dump's new forms** (spec §4.1-§4.3; ADR 0054 §1): `package=<n>`
    on a module, the graph's id; `target=wasm`; `(Missing` std's names the
    WASM target does not compile; ` mode=test` on the header under test
    mode only, so every 5.2a dump is byte for byte as it was; `(Test` with
    `should_panic`; labels `stem`, `tests/stem`, a dependency's import
    name for its library, `import_name/stem`; manifest labels
    `nova.toml`, `<import>/nova.toml`. `Program::for_file_with` registers
    std first, so std's ids match in every program. `--test --target wasm`
    is refused, one stderr line, exit 2, as `nova test` has no target.
    `KINDS` gains the seven forms; `LATER_KINDS` keeps `(Help` and
    `(NoteDiagnostic`. A driver test checks that the dump's tests are
    `FrontendContext::check(true)`'s.
14. **Windows and macOS** (step 0; ADR 0054 §7): labels are built from
    names, so `/` everywhere; E0004's and E0093's messages hold paths,
    settled by `settle_paths` on Windows alone; the refusal line names
    the root as given; `Nova.toml` on a case-folding disk is a manifest
    on both sides (found by its path), and `SRC/` is not `src/` on
    either (found by its listed name); the cases' `nova.toml` files are
    plain text both readers take with CRLF; the threads `c0c9ad1a` added
    join in a fixed order, so output order does not depend on the cores.
    **(differs)** A root typed `src\main.nova` on Windows is one name to
    N's `path_parent`, where S0 splits it: the harness writes only `/`,
    and 5.5, whose driver takes roots from users, settles it.
15. **The edges, probed on both sides** (step 0): a dependency through
    `..`, an absolute one, E0004 inside a dependency reached through `..`;
    a `src/` subdirectory root (a loose program on both); an empty
    `tests/`; a library-only package; `tests/` importing the library, a
    dependency and a dev-dependency; `--test` on a package with four
    dependencies (theirs stripped); colliding `@test`s; each the same
    dump. A dependency on the package itself, two entries of one path, a
    dev-dependency also a dependency: an S0 M-code and an N refusal.
    `--test` in every position (an empty argument, `--`, repeated,
    without `--dump-resolution`) alike on both sides and, where the base
    has the shape, as the base: `both_sides_take_test_in_every_position_alike`,
    `usagecmp.sh` (246 runs, 10 in a terminal, 114 refused).
16. **N's reader against S0's** (step 0; ADR 0054 §5): the 211 distinct
    manifests the repository holds or the suite writes, each in a package
    through both sides: 131 read alike, 80 refused where S0 gives an
    M-code, 0 accepted by S0 and refused by N. Four used to be refused
    and are now read: `nova add`'s inline dependency table (2), a BOM, a
    pre-release version. **(differs)** A manifest ending in a bare `\r`
    or holding a control character other than a tab was read by N and
    refused by toml_edit: N now refuses it. Build metadata stays refused,
    as `nova-pm` refuses it. The table is pinned in N's tests: 11 read,
    33 refused (30 S0 M-codes, 3 forms S0 reads that no manifest here
    uses). After the first fact-check (decision 23), at `bffca307`: 229
    distinct manifests (the 104 tracked and those the suite writes), 146
    read alike, 83 refused where S0 gives an M-code, none accepted by S0
    and refused by N; the table 16 read (14 with no M-code from S0, and 2
    holding version entries among the root's dev-dependencies, which the
    graph refuses and S0 gives an M-code) and 51 refused (48 S0 M-codes,
    the same 3 forms S0 reads).
17. **The harness's speed-up, every input kept** (step 0, `56ed80c5`;
    ADR 0054 §9): one process a side for each set and mode; the fuzzer's
    seeds in one process a mode, each seed in its own directory, counted
    seed by seed; 5.1's corpus and cases compared once and shared
    (`Compared`); CRLF copies compared only where the copy changes a file.
    51.6, 49.5, 51.9 s became 49.2, 47.3, 48.7 s; no count or input set
    cut.
18. **Option B: std resolved once per process** **(differs)** (the
    user's choice after decision 10; ADR 0054 §9): S0's dump resolves std
    once per process and mode into `SharedStd` (std's exports, the
    variants std globs, `missing`); each program reads std's exports
    rebased past its own ids (`Exports::rebased`) and never writes the
    shared state. Only `ResolutionDumps` uses it; `nova check`, `nova
    build`, `nova test` and the language server keep
    `resolve_program_with_std`. If std alone gives a diagnostic, or holds
    a `@test` function (decision 23), every program takes the full path.
    `variant_shadowing` is split into
    `variant_shadowing_of`, a pure refactor; `FileDb` derives `Clone`. N
    mirrors it (`share_std`, `resolve_program_shared`). The harness runs
    std's three parts side by side and the package corpus in 3 processes
    a side. `std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`
    compares shared with unshared over 463 / 69 / 135 roots, in order and
    reversed. On one machine state: before 40.6, 40.3, 40.8 s, after
    28.5, 27.3, 27.0 s (user CPU 126-128 s, then 83-87 s), 5.1's 27 tests
    alone 18.1, 18.2, 18.0 s. The projection from these local times
    (Linux 24-25 s, macOS 20-21 s, Windows 51-54 s) was wrong: CI
    measured the tree after the first fact-check at Linux 37.90 s, macOS
    35.77 s, Windows 60.48 s (decision 24).
19. **The records** (step 0; Task 4): ADR 0054 `Accepted (2026-11-04)`
    in its Status, as ADR 0053 was in 5.2a's pull request, with the branch
    `claude/loving-archimedes-frtf5q-52b`, every departure above, the CI
    figures and the live heap; one corpus count, 318 (251 loose programs
    and 67 package files), everywhere; ADR 0053 a dated note; the
    CHANGELOG says nothing a user runs changed, which `checkcmp.sh` and
    `pkgcmp.sh` check (Task 5, Step 4); every ADR example byte for byte
    what S0 prints (`adrcheck.sh`, Task 4).

How this plan is cut and checked:

20. **Four tasks, three test first, then the records** **(differs)**: the
    split's brief drew seven. The nearest clean cut merges three pairs:
    - S0's package dump with its test mode (Task 1): `test` threads
      through every signature the package work changes
      (`ResolutionDumps::new`, `dump_program`, `dump_std`, `std_part_of`,
      `register_std_for`), so a cut would need a version of each that the
      head never has;
    - N's reader and loader with N's WASM target and test mode (Task 2):
      both rewrite `main.nova`'s `resolve_command` and `dump_resolution`,
      and `strip_test_functions`, line by line;
    - the cases with the differential (Task 3): the base harness already
      compares S0 with N on every case in `cases/resolve/`, so a package
      case there compares a half-ported N unless the differential comes
      with it, and the head's case passes are both-sided.
    Option B goes where its code is: S0's `SharedStd`, the
    `variant_shadowing_of` refactor, `FileDb: Clone` and the equivalence
    test in Task 1; N's sharing in Task 2; the harness's parallelism in
    Task 3. Three files have versions the head does not: `selfhost.rs`
    checks a package's root on S0 alone in Tasks 1 and 2
    (`s0_resolution`), until Task 3 compares both sides; and the
    equivalence test takes a missing directory of cases as empty, with
    the corpus's floors, until Task 3 adds the cases. `dump.rs` keeps
    5.2b's forms (and ` mode=test`) in `LATER_KINDS` until Task 3's cases
    print them. Every commit builds and passes the whole suite.
21. **The mutants** (Task 5, Step 8): 140 mutants (S0 22, the harness
    19, N 99), 135 caught, 5 accepted, each with its reason in Task 5,
    Step 8: mutants 19, 29, 32, 44 and 105. The first run, at the split
    before `d8dae46a`, found 11 gaps among those that passed: the
    process split dropping a root (the dealt-back parts were counted,
    not named), S0's M-code stop and a dependency's manifest label, N's
    graph refusing a package with no entry, a keyword library and a
    dependency on itself only by luck of another check, a dependency's
    dev-dependencies, and three manifest refusals another check backed
    up. `d8dae46a` closes them: each dealt-back part must be its root's
    and none left over;
    `a_manifest_n_cannot_read_stops_n_with_a_message` checks that an
    error stops S0 before the loader; the case
    `a_dependencys_e0004_names_its_own_manifest` (E0004 inside a
    dependency, whose manifest also has a dev-dependency the graph never
    reads); N's graph test checks each refusal's words, with three more
    packages; N's reader test pins three refusals' words. The first
    fact-check added 20 (the u64 limit, the requirement reader, the
    dev-dependency version entries, identity's message, `--target=`, the
    std `@test` sharing on both sides, the checker's four reader
    mutants, and the fuzzer's manifest forms and modes) and made the two
    fallback mutants catchable, each caught now; its run at the split
    found one gap, the manifest forms written in one form passing, which
    the fuzzer's check that E0004 names its entry on more than one line
    of a manifest closes (`bffca307`). The second fact-check added 15
    (`run` settling no resolution dump, S0's only, N's only or every
    dump; the identity clause without the climbing check, reading one
    package's path only, taking `..` alone, or one-sided for an absolute
    and a relative path; N taking `--target ""`, marking no `--target
    VALUE` given, taking a second `--target=VALUE`, taking
    `--test=VALUE`; N's rebasing without the trait offset; the fuzzer's
    form 3 without `\r\n`), each caught; all 140 ran again at the
    split's last commit, `da9b88c0`, after the second fact-check's fixes
    (`3e0c5099`). The rest pass by design.
22. **The figures at the last commit** (Task 5): `selfhost.rs` 20.2,
    20.2 and 20.5 s (user CPU 64 to 65 s; the machine was faster after a
    container restart than on the days of decisions 18 and 25)
    locally, 5.1's 27 tests alone 13.3, 12.8 and 12.9 s; the live heap
    `selfhost/src/main.nova` 36.9 MB peak and a 93.5 ms longest pause
    with the debug runtime, 36.1 MB and 29.0 ms with a release one;
    std's parts 16.0 to 19.0 MB. The peak is about 4 MB above part B's
    (32.8 and 31.8 MB): std's state, kept for every root under option B;
    ADR 0054 §10 now holds these figures; the plan's `heap.sh` run again
    at the split's last commit gave 37.0 MB and 63.5 ms, 35.9 MB and
    19.1 ms (the pauses shorter on the faster machine state of the
    replay). These
    are local figures only; CI's are decision 24's, measured, not
    projected.
23. **The first fact-check** (`bffca307`; ADR 0054 §§1, 3, 5, 7, 9):
    - **A version's numbers** are each at most `u64::MAX` in N's reader
      (`is_u64`), as semver reads them, in a root's and a dependency's
      manifest; the table pins each place one past it, `1.0.0.0`, DEL and
      a 65-character name, and the graph's test a dependency's.
    - **Package identity** **(differs)**: N's std and runtime can neither
      name the current directory, follow a link nor read a disk's case
      folding, so N cannot read
      `real_path`; it folds the path as the root and the entries spell
      it. Exactly three kinds of input can differ: any path through a
      link, whatever the root; unless the root is given by an
      absolute path, a dependency path that is absolute or whose folded
      form climbs above the current directory (starts with `..`); and,
      on a case-insensitive disk (Windows, usually macOS), one
      directory spelled two ways by case **(differs)** (the review's
      finding), `../Geom` and `../geom`, one package to S0 and two to
      N, which refuses as M0011 without the clause. Where
      they differ, none the comparison's: one directory spelled relative
      and absolute under a relative root, or by a path climbing above the
      current directory and back and one that does not (the second
      fact-check's input: in `run`, `climb` depends on `../../run/geom`),
      N refuses as M0011, saying the two paths may be one directory, and
      S0 reads one package; one directory through a link and without, N
      refuses as M0011 with no such clause, and S0 reads one; and `x/..`
      after a link `x`, N folds and S0 follows the link, so S0 may give
      M0011 where N dumps. No case, corpus file or fuzz input holds a
      link, an absolute dependency path under a relative root, a path
      climbing above the current directory or a path spelled two ways by
      case. A `..` at an absolute path's leading `/` is dropped, as
      `real_path` drops it (the review's finding: `/../x` is `/x`; a
      drive is not read, so `C:/../x` folds to `x`, decision 14). The
      Unix test
      `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`
      pins both sides reading one package under absolute roots, a path
      climbing past `/` among them, and N's
      refusals under relative ones, each pair of paths each way round
      (decision 25). On Windows N's paths read no drive, so the test is
      Unix's (5.5, with decision 14).
    - **std's sharing** stays safe for all std could add to a program's
      part: `shareable` also requires that std alone hold no `@test`
      function, which the full path lists and the shared one would drop,
      on both sides (falling back rather than listing it, as for a
      diagnostic: both are std's own, and the fallback is the path the
      equivalence test already proves). nova-resolver's
      `std_with_a_diagnostic_or_a_test_is_resolved_for_each_program`
      and the harness's
      `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`
      pin both fallbacks with an altered std (`core` defining a name
      twice, `ui` marking a function `@test`), so mutants
      18 and 104 are
      caught now.
    - **A dependency's dev-dependencies**: N's reader takes a version
      entry in `[dev-dependencies]` (`"REQ"` or `{ version = "REQ" }`,
      one to three numbers after `^`, `~`, `=` or nothing), which nova-pm
      reads and never resolves in a dependency's manifest; the graph
      refuses one in the root's, which needs the registry. Any other
      requirement is refused (S0 takes more), as ADR 0054 §5 lists.
    - **`--target=VALUE`** is read by N as clap reads it for S0; an empty,
      repeated or unknown value is refused (exit 2).
    - **The span case**
      `a_tests_module_named_for_the_package_is_e0004_at_its_package_table`:
      `[package]`'s last value line ends in a comment, and a `tests/`
      module named for the package gives E0004 at `0..57`.
    - **The fuzzer's package inputs** are written in four manifest forms
      (`fuzz::package_manifests`), and seed 321's also run under test
      mode and seed 322's for the WASM target, in the process of that
      mode's loose inputs: no measurable time (`selfhost.rs` 31.8, 30.9
      and 31.7 s with them against 32.1, 31.6 and 31.3 s without, on one
      machine state). Manifests
      N refuses stay out of the fuzzer: a refusal stops N's process, so
      each would need processes of its own; the reader's table, the 229
      manifests and `a_manifest_n_cannot_read_stops_n_with_a_message` pin
      them.
    - **Records**: ADR 0054 §1 names the `?` label; the added-and-gone
      count, `names-301` to `303` (5.2a's), the manifest counts, Review
      Focus 3, the MSRV sentence, decision 20 and the driver's doc
      comments are corrected; Task 1 Step 2 states both crates' errors,
      whichever cargo builds first. (The CI projection it then corrected
      is replaced by CI's measured figures, decision 24.)

The dated lines of 2026-11-05:

24. **CI measured, and the user's choice** **(differs)** (the user's
    lines of 2026-11-05; ADR 0054 §9): spec §9's risk 4 asks the user
    when CI exceeds a minute on a system. Pushed to a separate branch
    with a draft pull request, closed after, the tree after the first
    fact-check's fixes (`bffca307`) ran CI run 42006844810:
    `selfhost.rs` on Linux 37.90 s (42 tests), macOS 35.77 s (40),
    Windows 60.48 s (36; one test failed early, decision 25, so the full
    run is likely slightly longer). Before option B, run 41813500363
    measured 35.59 / 29.54 / 76.59 s. The local projection was wrong:
    it put Linux at 24-25 s, and CI measured 37.9 s. The user accepted
    about 60 s on Windows, with no coverage cut; the PR's CI is read on
    all three systems and compared with these figures, and the user is
    asked again if `selfhost.rs` takes more than 70 s on Windows (Task
    5, Step 11).
25. **The second fact-check and CI's Windows failure** (`3e0c5099`; ADR
    0054 §§1, 4, 5, 7, 9):
    - **Every resolution dump settled where it is run** (CI run
      42006844810's one failure, Windows only):
      `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`
      compared S0's dump of a dependency's E0004, which printed
      its paths with `\` (`…\geo\nova.toml:7`), with N's `/`,
      without `settle_paths`. The harness's `run`, which every S0/N
      comparison goes through, now settles every resolution dump, S0's
      and N's (`settled_stdout`), and `compare_batch` no longer does it
      apart. Audited: every S0/N comparison in `selfhost.rs`,
      `selfhost/compare.rs`, `selfhost/fuzz.rs` and nova-driver's
      equivalence test; that test was the only one comparing a
      path-bearing dump unsettled (the others compare one side with
      fixed text, or S0 with S0). The new test
      `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`
      runs on every system: S0's E0004 written as Windows writes it
      differs from N's dump raw and equals it settled under S0's and N's
      arguments.
    - **Package identity:** N's M0011 says the two paths may be one
      directory also when a folded path climbs above the current
      directory (`climbs_above`); the identity test gains that input,
      spelled both ways round (`up`, `up2`), and the absolute and the
      relative path both ways round (`app`, `app3`); decision 23 and
      ADR 0054 §5 state exactly which inputs can differ.
    - **`--target ""`, a repeated `--target`, `--test=VALUE`:** N refuses
      an empty target in either spelling, marks the option given in the
      space-separated form so a second is refused, and refuses
      `--test=VALUE`, each with exit 2 and nothing on stdout, as clap
      refuses them for S0; `both_sides_take_test_in_every_position_alike`
      gains five shapes (15).
    - **A trait's rebasing:** no std module a root imports exports a
      trait, so dropping the trait offset in N's `Exports::rebased`
      passed; `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`
      gives a copy of std a `pub trait` at the end of `ui` and checks
      each of two roots lists it with the id N's std part gives it, and
      is otherwise S0's dump.
    - **The fuzzer's form 3** must end every line of both manifests with
      `\r\n` (an assertion in `write_package_inputs_in`).
    - **Spec §4.5's "every corpus program under `--target wasm`"**
      **(differs)**: the harness runs the 39 WASM programs, not all 334
      roots, for CI's time on Windows, now about 60 s (decision 24); the
      second fact-check ran all 334 for the WASM target through both
      sides, and they agreed.
    - **Records:** suites, case-file counts, which head each mutant ran
      at, ADR 0054's Status (`Accepted`, as ADR 0053's in 5.2a), Task 4's
      manifest count, Task 1 Step 4's per-crate Expected, the citations
      of `settle_paths` (ADR 0054 §7, and decision 4 of its §8) and Task
      2's patch size. No measurable time: `selfhost.rs` 32.6, 33.4 and
      34.7 s before and 33.0, 33.4 and 34.6 s after, three runs each on
      one machine state.
### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-52b`: from `2d38ad7a`
(`main` after 5.2a), then this plan's commit, which adds this file alone,
so the patches apply to it as to `2d38ad7a`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-52b`.
  - `git log --format='%h %s' -1 HEAD~1` prints
    `2d38ad7a 5.2a: the review's fixes`.
  - `git diff --stat 2d38ad7a HEAD` lists only
    `docs/superpowers/plans/2026-11-04-phase-5-2b-packages-wasm-and-test-mode.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 5's review reads from it.
  - `df -h /` shows at least 6G free; if not, remove older duplicate test
    executables in `target/debug/deps` first. The replay began with
    7.7G free.
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
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-52b.txt" 2>&1
  grep '^test result' "$P/base-52b.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-52b.txt"
  cp target/debug/nova "$P/nova-base-52b"
  cargo test --locked --workspace -- --list > "$P/base-list-52b.txt" 2>&1
  grep -c ': test$' "$P/base-list-52b.txt"
  ```

  Expected, from 5.2a's last full run on `2d38ad7a`: `2858 1 10`, and
  one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. The replay built `2d38ad7a` and listed its tests, and
  found `2869` names, the run's 2858 + 1 + 10; its `nova` was
  byte for byte the one the prototype's base run kept. The copy of `nova`
  is the base's, for Task 5's comparisons; the list is the base's names.
  Task 5 expects `2869 1 10`: 5.2b's tests make 11 more,
  12 added and 1 gone, the fuzzer's test renamed.
- [ ] **Step 4: The ledger.** Start `$P/ledger-52b.md`, headed with the
  base commit, the baseline's figures and the process limit's form.


### Task 1: S0's resolution dump of packages, the WASM target and test mode

Spec §4.1-§4.3; decisions 5, 8, 12, 13, 18, 20 and 23.

**Files:**
- Modify: `crates/nova-driver/src/program.rs` (`for_file_with`),
  `crates/nova-driver/src/lib.rs` (`ResolutionDumps`, `register_std_for`,
  `manifest_labels`, the unit tests `resolution_dumps`),
  `crates/nova-resolver/src/dump.rs`, `crates/nova-resolver/src/lib.rs`
  (`SharedStd`), `crates/nova-diagnostics/src/files.rs`,
  `crates/nova-cli/src/cmd/run.rs`, `crates/nova-cli/tests/selfhost.rs`
  (a package's root checked on S0 alone)

- [ ] **Step 1: The tests.** `patch_block t1-tests`:
  - `dump.rs`'s tests take the `test` argument, and
    `std_part_is_the_same_in_every_test_mode_program` and
    `the_5_2b_forms` (the program every form test reads) are added, and
    `std_with_a_diagnostic_or_a_test_is_resolved_for_each_program` (std
    built from altered text, `core` defining a name twice and `ui`
    marking a function `@test`: `SharedStd` is not shareable, and the
    shared dump is the full one under `check` and test mode; decision
    23);
  - the driver's `resolution_dumps` module:
    `test_mode_dumps_the_tests_nova_test_collects` (the dump's `(Test`
    lines are `FrontendContext::check(true)`'s, a dependency's stripped,
    a `tests/` file's named) and
    `std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`
    (decision 18; until Task 3 adds the cases, a missing directory of
    cases is empty and the floors are the corpus's);
  - `selfhost.rs`: `MANIFEST`, `s0_resolution`, and the three tests that
    expected `UNSUPPORTED package` now expect S0's package part, and
    compare N only on the roots in no package.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`.
  - `cargo test --locked --workspace --lib -- dump::tests`, then
    `cargo test --locked --workspace --lib -- resolution_dumps`. Expected:
    no test runs, as two crates' library tests do not compile:
    `nova-resolver`'s, 15 errors: `E0061` (`dump_program`, `dump_std`
    and `std_part_of` take the `test` argument they do not yet have),
    `E0308`, and `E0425` and `E0433` (`resolve_program_with_shared_std`
    and `SharedStd`, which the new test reads); and `nova-driver`'s, 4
    errors: `E0061` (`ResolutionDumps::new` takes one argument) and
    `E0599` (no `ResolutionDumps::unshared`). Each run builds every
    library's
    tests, so it reports one crate's errors or both, as the order cargo
    builds them in falls (the replay's first run gave `nova-resolver`'s,
    its second `nova-driver`'s); what holds either way is that each run
    ends with `error: could not compile` for one of the two crates and
    neither runs a test.
  - `SH the_resolution_dump_has_a_header_per_root
    a_root_with_no_directory_part a_package_is_found_by`.
    Expected: the three tests fail, `0 passed; 3 failed`: S0 prints
    `UNSUPPORTED package`
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`program.rs`:** `Program::for_file_with(path, db)`, which
    `for_file` calls with an empty database: the dump passes one holding
    std's files, so std's ids are the same in every program;
  - **`dump.rs`:** `test` in `dump_program`, `dump_std`, `std_part_of`
    and `std_part`, ` mode=test` (`Dump::mode`), the package key,
    `(Test`, `(Missing`; `KINDS` and `LATER_KINDS` keep 5.2a's lists,
    ` mode=test` among the later forms, until Task 3;
  - **`lib.rs` (resolver):** `Exports::rebased`, `SharedStd`,
    `resolve_program_with_shared_std`, and `variant_shadowing_of`, which
    `variant_shadowing` now calls;
  - **`files.rs`:** `FileDb` derives `Clone`;
  - **`lib.rs` (driver):** `ResolutionDumps::new(target, test)` and
    `unshared`, the graph's M-codes first, `manifest_labels`, test
    stripping and naming as `nova test`'s, `register_std_for` (`std/test`
    joined under test mode), `dump_std_resolution(target, test)`;
  - **`run.rs`:** the hidden `--test`, refused with `--target wasm`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - The two library runs of Step 2. Expected: three `test result` lines,
    one for each crate with tests the filter `dump::tests` matches:
    `nova_ast`'s `4 passed; 0 failed`, `nova_parser`'s `2 passed; 0
    failed` and `nova_resolver`'s `8 passed; 0 failed; …; 61 filtered
    out`, the last 5.2b's (the first two are 5.1's AST and token dumps,
    unchanged) and
    one line, `nova_driver`'s `2 passed; 0 failed; …; 37 filtered out`.
  - The `SH` run of Step 2. Expected: `3 passed; 0 failed`.
  - `S` (221 s). Expected: `2862 1 10`, the IPv6 test the one
    failure.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `nova-driver: the resolution dump of packages,
      the WASM target and test mode`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 37f94f1c8e1c56f377d13d46a7d17315280f1ca4..155eb61365fafa5e9f7426e5f5d75662c43f8c03 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -1426,6 +1426,10 @@ fn nova_run_runs_the_program_on_the_main_thread() {
 /// program's root.
 const RESOLUTION_S0: &[&str] = &["check", "--dump-resolution"];
 
+/// A package's `nova.toml` with what the package graph requires and
+/// nothing else: the tests' packages (5.2b, ADR 0054).
+const MANIFEST: &str = "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";
+
 /// The error cases of the resolver (spec §3.5): the inline sources of
 /// `nova-resolver`'s tests and of the type checker's tests that reach the
 /// resolver, each a file, or a directory whose `main.nova` is the root of
@@ -1566,6 +1570,21 @@ fn agree_on_resolution(dir: &Path, files: &[String], code: i32) -> String {
     all
 }
 
+/// S0's resolution dump of `files` from `dir`, which must exit with
+/// `code`: what a package's roots are checked by until N dumps packages
+/// (ADR 0054).
+fn s0_resolution(dir: &Path, files: &[String], code: i32) -> String {
+    let out = Command::new(nova())
+        .current_dir(dir)
+        .args(RESOLUTION_S0)
+        .args(files)
+        .output()
+        .expect("run nova check --dump-resolution");
+    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
+    assert_eq!(out.status.code(), Some(code), "{stdout}");
+    stdout
+}
+
 /// `nova check`'s help names neither hidden flag, and every `nova check`
 /// command line whose first argument after `check` is not
 /// `--dump-resolution` is parsed as before the flags (ADR 0053 §1): each
@@ -1694,16 +1713,18 @@ fn the_resolution_dump_flags_are_hidden_and_refused_as_before() {
 
 /// Each root's part follows a header with its path as given; a root that
 /// cannot be read is `IOERR` and its kind, the rest are dumped, and the
-/// exit code is 2; a root in a package is `UNSUPPORTED package`; a program
-/// that fails to load holds the diagnostics so far; `--std` prints std's
-/// part for the target. N prints the same for the same roots.
+/// exit code is 2; a root in a package is a module of the package (ADR
+/// 0054); a program that fails to load holds the diagnostics so far;
+/// `--std` prints std's part for the target. N prints the same for the
+/// same roots but the package's, whose part S0 alone is checked for until
+/// N dumps packages.
 #[test]
 fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
     let dir = fresh_dir("resolution-headers");
     std::fs::write(dir.join("a.nova"), "fn main() {}\n").expect("write a.nova");
     std::fs::write(dir.join("b.nova"), "fn g( {").expect("write b.nova");
     std::fs::create_dir_all(dir.join("pkg").join("src")).expect("create pkg/src");
-    std::fs::write(dir.join("pkg").join("nova.toml"), "").expect("write nova.toml");
+    std::fs::write(dir.join("pkg").join("nova.toml"), MANIFEST).expect("write nova.toml");
     std::fs::write(
         dir.join("pkg").join("src").join("main.nova"),
         "fn main() {}\n",
@@ -1729,7 +1750,14 @@ fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
         stdout.contains(
             "  (Items 0..1 0))\n(Diagnostics)\n\
              == missing.nova\nIOERR NotFound\n\
-             == pkg/src/main.nova\nUNSUPPORTED package\n\
+             == pkg/src/main.nova\n(Program target=native resolved=true\n  \
+             (Module 0 \"main\" package=0)\n"
+        ),
+        "{stdout}"
+    );
+    assert!(
+        stdout.contains(
+            "  (Items 0..1 0))\n(Diagnostics)\n\
              == b.nova\n(Program target=native resolved=false\n  \
              (Module 0 \"b\" package=None))\n(Diagnostics\n  \
              (Error \"P0001\" \"expected identifier (in parameter name), found `{`\"\n    \
@@ -1750,7 +1778,6 @@ fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
     let files = [
         "a.nova",
         "missing.nova",
-        "pkg/src/main.nova",
         "b.nova",
         "dirpkg/src/main.nova",
         "latin1.nova",
@@ -1975,8 +2002,9 @@ fn a_root_spelled_any_way_names_its_siblings_alike() {
 /// directory both sides list for the root's siblings is then `.`, so a
 /// directory named `ui.nova` beside `main.nova` is E0093's, though it
 /// cannot be read, and a root directly in a package's `src/` or `tests/`,
-/// named from the package's directory, is `UNSUPPORTED package`, as one
-/// named from outside it is. An empty root is a file that is not there.
+/// named from the package's directory, is a module of the package, as one
+/// named from outside it is (ADR 0054; S0's part alone until N dumps
+/// packages). An empty root is a file that is not there.
 #[test]
 fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
     let dir = fresh_dir("resolution-roots");
@@ -1993,19 +2021,30 @@ fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
         std::fs::create_dir_all(pkg.join(sub)).expect("create a package's directory");
         std::fs::write(pkg.join(sub).join("main.nova"), "fn main() {}\n").expect("write main.nova");
     }
-    std::fs::write(pkg.join("nova.toml"), "").expect("write nova.toml");
+    std::fs::write(pkg.join("nova.toml"), MANIFEST).expect("write nova.toml");
     let roots = ["src/main.nova", "tests/main.nova"].map(String::from);
-    let out = agree_on_resolution(&pkg, &roots, 0);
-    assert_eq!(
-        out,
-        "== src/main.nova\nUNSUPPORTED package\n== tests/main.nova\nUNSUPPORTED package\n"
-    );
+    let out = s0_resolution(&pkg, &roots, 0);
+    for (root, label) in [("src/main.nova", "main"), ("tests/main.nova", "tests/main")] {
+        assert!(
+            out.contains(&format!(
+                "== {root}\n(Program target=native resolved=true\n  \
+                 (Module 0 \"{label}\" package=0)\n"
+            )),
+            "{out}"
+        );
+    }
     let roots = ["pkg/tests/main.nova", ""].map(String::from);
-    let out = agree_on_resolution(&dir, &roots, 2);
-    assert_eq!(
-        out,
-        "== pkg/tests/main.nova\nUNSUPPORTED package\n== \nIOERR NotFound\n"
+    let out = s0_resolution(&dir, &roots, 2);
+    assert!(
+        out.starts_with(
+            "== pkg/tests/main.nova\n(Program target=native resolved=true\n  \
+             (Module 0 \"tests/main\" package=0)\n"
+        ),
+        "{out}"
     );
+    assert!(out.ends_with("\n== \nIOERR NotFound\n"), "{out}");
+    let out = agree_on_resolution(&dir, &["".to_string()], 2);
+    assert_eq!(out, "== \nIOERR NotFound\n");
 }
 
 /// A root's package is found as S0's `package_of` finds it (ADR 0053 §3):
@@ -2017,6 +2056,8 @@ fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
 /// link none, and a directory that can be searched but not listed still
 /// holds its `nova.toml` for both (as root, which lists it anyway, that
 /// part is any other package). The test asks the disk whether it folds case.
+/// S0's parts are checked, and N's of the roots in no package until N
+/// dumps packages.
 #[test]
 fn a_package_is_found_by_its_manifests_path_on_both_sides() {
     #[cfg(unix)]
@@ -2035,15 +2076,15 @@ fn a_package_is_found_by_its_manifests_path_on_both_sides() {
         .expect("write main.nova");
         dir.join(name)
     };
-    std::fs::write(package("cased").join("Nova.toml"), "").expect("write Nova.toml");
+    std::fs::write(package("cased").join("Nova.toml"), MANIFEST).expect("write Nova.toml");
     let folds = dir.join("cased").join("nova.toml").is_file();
     #[cfg(unix)]
     let (more, _relisted) = {
         use std::os::unix::fs::{symlink, PermissionsExt};
-        std::fs::write(dir.join("real.toml"), "").expect("write real.toml");
+        std::fs::write(dir.join("real.toml"), MANIFEST).expect("write real.toml");
         symlink("../real.toml", package("linked").join("nova.toml")).expect("link nova.toml");
         symlink("missing.toml", package("dangling").join("nova.toml")).expect("link nova.toml");
-        std::fs::write(package("locked").join("nova.toml"), "").expect("write nova.toml");
+        std::fs::write(package("locked").join("nova.toml"), MANIFEST).expect("write nova.toml");
         let relisted = Relisted(dir.join("locked"));
         std::fs::set_permissions(dir.join("locked"), std::fs::Permissions::from_mode(0o311))
             .expect("make locked/ unlistable");
@@ -2059,12 +2100,19 @@ fn a_package_is_found_by_its_manifests_path_on_both_sides() {
         .iter()
         .map(|(name, _)| format!("{name}/src/main.nova"))
         .collect();
-    let out = agree_on_resolution(&dir, &roots, 0);
+    let loose: Vec<String> = roots
+        .iter()
+        .zip(&cases)
+        .filter(|(_, (_, package))| !*package)
+        .map(|(root, _)| root.clone())
+        .collect();
+    agree_on_resolution(&dir, &loose, 0);
+    let out = s0_resolution(&dir, &roots, 0);
     for (root, (_, package)) in roots.iter().zip(&cases) {
         let part = if *package {
-            "UNSUPPORTED package\n"
+            "(Program target=native resolved=true\n  (Module 0 \"main\" package=0)\n"
         } else {
-            "(Program target=native resolved=true\n"
+            "(Program target=native resolved=true\n  (Module 0 \"main\" package=None)\n"
         };
         assert!(
             out.contains(&format!("== {root}\n{part}")),
diff --git a/crates/nova-driver/src/lib.rs b/crates/nova-driver/src/lib.rs
index a0d536771c8793c83b5d0e33e0e24ab7d268ff1e..5aa9b911bda9ef6dd2111b17f86da24e9905b753 100644
--- a/crates/nova-driver/src/lib.rs
+++ b/crates/nova-driver/src/lib.rs
@@ -1135,6 +1135,286 @@ fn strip_test_functions(modules: &mut [Loaded], keep_root_tests: bool) {
     }
 }
 
+/// The resolution dumps of packages and under test mode (5.2b, ADR 0054):
+/// the tests a dump lists are `nova test`'s, and std resolved once dumps
+/// what std resolved for each program dumps.
+#[cfg(test)]
+mod resolution_dumps {
+    use super::*;
+
+    /// Write each `(path, text)` under a fresh `dir`, a fixed name under the
+    /// system temp directory.
+    fn package(name: &str, files: &[(&str, &str)]) -> PathBuf {
+        let dir = std::env::temp_dir().join(format!("nova-driver-resolution-{name}"));
+        let _ = std::fs::remove_dir_all(&dir);
+        for (path, text) in files {
+            let path = dir.join(path);
+            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
+            std::fs::write(path, text).unwrap();
+        }
+        dir
+    }
+
+    /// The `(Test` lines of a dump: each test's id, name and
+    /// `should_panic`, without the `)` that closes the program after the
+    /// last.
+    fn tests_of(dump: &str) -> Vec<String> {
+        dump.lines()
+            .filter(|l| l.starts_with("  (Test "))
+            .map(|l| {
+                l.strip_suffix("))")
+                    .map_or(l.to_string(), |l| format!("{l})"))
+            })
+            .collect()
+    }
+
+    /// A [`TestFn`] as the dump prints it.
+    fn line(test: &TestFn) -> String {
+        format!(
+            "  (Test u{} \"{}\" should_panic={})",
+            test.def_id.0, test.name, test.should_panic
+        )
+    }
+
+    /// Under test mode the dump shows what `nova test`'s own path resolves
+    /// (`FrontendContext::check(true)`, which `build_test_program` runs):
+    /// the same tests, by id, name and `should_panic`, in the same order,
+    /// for a `tests/` file of a package with a dependency, whose `@test`
+    /// functions are stripped while the root package's are kept, and for a
+    /// loose file; and its `std/test` makes `assert_eq` resolve, which
+    /// `check`'s mode does not have.
+    #[test]
+    fn test_mode_dumps_the_tests_nova_test_collects() {
+        let manifest = |name: &str, extra: &str| {
+            format!(
+                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}"
+            )
+        };
+        let dir = package(
+            "test-mode",
+            &[
+                (
+                    "app/nova.toml",
+                    &manifest("app", "\n[dependencies]\ngeom = { path = \"../geom\" }\n"),
+                ),
+                (
+                    "app/src/lib.nova",
+                    "import geom\n\npub fn twice(n: Int) -> Int {\n    n * width()\n}\n\n\
+                     @test\nfn twice_doubles() {\n    assert_eq(twice(2), 4)\n}\n",
+                ),
+                (
+                    "app/tests/api.nova",
+                    "import app\n\n@test\nfn first() {\n    assert_eq(twice(1), 2)\n}\n\n\
+                     @test(should_panic)\nfn second() {\n    panic(\"no\")\n}\n",
+                ),
+                ("geom/nova.toml", &manifest("geom", "")),
+                (
+                    "geom/src/lib.nova",
+                    "pub fn width() -> Int {\n    2\n}\n\n@test\nfn stripped() {}\n",
+                ),
+                (
+                    "loose.nova",
+                    "@test\nfn alone() {\n    assert_eq(1, 1)\n}\n\nfn main() {}\n",
+                ),
+            ],
+        );
+        let dumps = ResolutionDumps::new(Target::Native, true);
+        for root in [dir.join("app/tests/api.nova"), dir.join("loose.nova")] {
+            let mut ctx = FrontendContext::new(Program::for_file(&root)).unwrap();
+            let (_, tests, _) = ctx.check(true).unwrap().expect("the program checks");
+            let dump = dumps.dump(&root).unwrap();
+            assert!(dump.starts_with("(Program target=native mode=test resolved=true\n"));
+            assert!(dump.ends_with("\n(Diagnostics)\n"), "{dump}");
+            let want: Vec<String> = tests.iter().map(line).collect();
+            assert!(!want.is_empty());
+            assert_eq!(tests_of(&dump), want, "{}", root.display());
+        }
+        let api = dumps.dump(&dir.join("app/tests/api.nova")).unwrap();
+        assert_eq!(
+            tests_of(&api)
+                .iter()
+                .map(|l| l.split('"').nth(1).unwrap())
+                .collect::<Vec<_>>(),
+            ["api::first", "api::second", "twice_doubles"]
+        );
+        // `check`'s mode strips every test and has no `assert_eq`.
+        let check = ResolutionDumps::new(Target::Native, false)
+            .dump(&dir.join("app/tests/api.nova"))
+            .unwrap();
+        assert!(tests_of(&check).is_empty());
+        assert!(!check.contains("(Value.Def \"assert_eq\""));
+        assert!(api.contains("(Value.Def \"assert_eq\""));
+    }
+
+    /// The repository, two levels above this crate.
+    fn repository() -> std::path::PathBuf {
+        Path::new(env!("CARGO_MANIFEST_DIR"))
+            .ancestors()
+            .nth(2)
+            .expect("the repository")
+            .to_path_buf()
+    }
+
+    /// Every `.nova` file under `dir`, sorted, skipping `target`,
+    /// `node_modules`, directories whose names begin with `.`, and `skip`.
+    fn nova_files(dir: &Path, skip: &Path, out: &mut Vec<std::path::PathBuf>) {
+        let mut entries: Vec<_> = std::fs::read_dir(dir)
+            .expect("read a directory")
+            .map(|e| e.expect("a directory entry").path())
+            .collect();
+        entries.sort();
+        for path in entries {
+            let name = path.file_name().unwrap().to_string_lossy().into_owned();
+            if path.is_dir() {
+                if name != "target"
+                    && name != "node_modules"
+                    && !name.starts_with('.')
+                    && path != skip
+                {
+                    nova_files(&path, skip, out);
+                }
+            } else if name.ends_with(".nova") {
+                out.push(path);
+            }
+        }
+    }
+
+    /// The programs of a directory of the differential's cases (the
+    /// harness's `case_roots`): each `.nova` file, each `.nova` file directly
+    /// in a package's `src/` and `tests/`, and each other directory's
+    /// `main.nova`.
+    fn case_roots(cases: &Path) -> Vec<std::path::PathBuf> {
+        // Task 3 adds the WASM and test-mode cases' directories.
+        let Ok(listing) = std::fs::read_dir(cases) else {
+            return Vec::new();
+        };
+        let mut entries: Vec<_> = listing
+            .map(|e| e.expect("a directory entry").path())
+            .collect();
+        entries.sort();
+        let mut roots = Vec::new();
+        for path in entries {
+            if path.join("nova.toml").is_file() {
+                for sub in ["src", "tests"] {
+                    if let Ok(dir) = std::fs::read_dir(path.join(sub)) {
+                        let mut files: Vec<_> = dir
+                            .map(|e| e.expect("a directory entry").path())
+                            .filter(|p| p.extension().is_some_and(|e| e == "nova"))
+                            .collect();
+                        files.sort();
+                        roots.extend(files);
+                    }
+                }
+            } else if path.is_dir() {
+                roots.push(path.join("main.nova"));
+            } else if path.extension().is_some_and(|e| e == "nova") {
+                roots.push(path);
+            }
+        }
+        roots
+    }
+
+    /// std resolved once and shared by every program of a process (5.2b,
+    /// ADR 0054 §9) dumps what resolving it again for each program dumps,
+    /// byte for byte, under `check`, for the WASM target and under test
+    /// mode: on the differential's inputs of each mode, the corpus's and
+    /// the cases', in their order and again in reverse, so that nothing a
+    /// program leaves in the shared state reaches the next.
+    #[test]
+    fn std_resolved_once_dumps_what_std_resolved_for_each_program_dumps() {
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
+        let sets: Vec<(Target, bool, Vec<std::path::PathBuf>)> = vec![
+            (
+                Target::Native,
+                false,
+                corpus
+                    .iter()
+                    .cloned()
+                    .chain(case_roots(&cases.join("resolve")))
+                    .collect(),
+            ),
+            (
+                Target::Wasm,
+                false,
+                corpus
+                    .iter()
+                    .filter(|p| under(p, "tests/dom") || under(p, "tests/wasm"))
+                    .cloned()
+                    .chain(case_roots(&cases.join("resolve-wasm")))
+                    .collect(),
+            ),
+            (
+                Target::Native,
+                true,
+                corpus
+                    .iter()
+                    .filter(|p| has_test(p))
+                    .cloned()
+                    .chain(case_roots(&cases.join("resolve-test")))
+                    .collect(),
+            ),
+        ];
+        std::thread::scope(|scope| {
+            let runs: Vec<_> = sets
+                .iter()
+                .map(|(target, test, roots)| {
+                    scope.spawn(move || {
+                        let dump_all = |dumps: &ResolutionDumps, roots: &[std::path::PathBuf]| {
+                            roots
+                                .iter()
+                                .map(|r| dumps.dump(r).map_err(|e| e.kind()))
+                                .collect::<Vec<_>>()
+                        };
+                        let unshared = dump_all(&ResolutionDumps::unshared(*target, *test), roots);
+                        let shared = ResolutionDumps::new(*target, *test);
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
+                        let resolved = unshared
+                            .iter()
+                            .filter(|d| d.as_ref().is_ok_and(|d| d.contains(" resolved=true")))
+                            .count();
+                        (roots.len(), resolved)
+                    })
+                })
+                .collect();
+            // The corpus's floors, until Task 3 adds the cases of each mode.
+            for (run, floor) in runs.into_iter().zip([400, 35, 8]) {
+                let (roots, resolved) = run.join().expect("a mode's thread");
+                assert!(
+                    roots >= floor && resolved * 2 > roots,
+                    "{roots} roots, {resolved} resolved"
+                );
+            }
+        });
+    }
+}
 /// What doc tests need of a build (spec
 /// `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
 /// §7, §11.2), where only this crate can see it.
diff --git a/crates/nova-resolver/src/dump.rs b/crates/nova-resolver/src/dump.rs
index d1c714638f1ad3249711419e98f200481b47b358..af0999799482a6501705522061ae6ae97f65ede7 100644
--- a/crates/nova-resolver/src/dump.rs
+++ b/crates/nova-resolver/src/dump.rs
@@ -757,12 +757,13 @@ mod tests {
         labels.push((lib_id, "lib".to_string()));
         let program = dump_program(
             CompileTarget::Native,
+            false,
             &sources,
             &labels,
             Some(&r),
             &r.diagnostics,
         );
-        let std = std_part_of(CompileTarget::Native, &labels, &r);
+        let std = std_part_of(CompileTarget::Native, false, &labels, &r);
         (program, std, std_labels, std_ids)
     }
 
@@ -819,14 +820,20 @@ mod tests {
             .imports
             .insert("ui".to_string(), ImportTarget::Reported);
         let labels = [(id, "main".to_string())];
-        let stopped = dump_program(CompileTarget::Native, &[module], &labels, None, &[]);
+        let stopped = dump_program(CompileTarget::Native, false, &[module], &labels, None, &[]);
         assert_eq!(
             stopped,
             "(Program target=native resolved=false\n  (Module 0 \"main\" package=None\n    \
              (Import \"ui\" Reported)))\n(Diagnostics)\n"
         );
+        let later = the_5_2b_forms();
         let mut found = std::collections::BTreeSet::new();
-        for line in program.lines().chain(std.lines()).chain(stopped.lines()) {
+        for line in program
+            .lines()
+            .chain(std.lines())
+            .chain(stopped.lines())
+            .chain(later.lines())
+        {
             found.extend(forms_on(line));
         }
         let missing: Vec<&&str> = KINDS.iter().filter(|k| !found.contains(**k)).collect();
@@ -835,6 +842,55 @@ mod tests {
         assert!(unlisted.is_empty(), "{unlisted:?}");
     }
 
+    /// What only 5.2b's programs print (ADR 0054): a package's module keys,
+    /// in a test-mode program whose module keeps two tests, then std's part
+    /// for the WASM target, with its `missing` names.
+    fn the_5_2b_forms() -> String {
+        let src = "@test\nfn plain() {}\n@test(should_panic)\nfn panics() {}\n";
+        let mut db = FileDb::new();
+        let main_id = db.add("main.nova", src);
+        let (mut std_ids, mut labels) = std_files(&mut db);
+        let (name, text) = crate::STD_TEST_MODULE;
+        let test_id = db.add("<std/test>", text);
+        labels.push((test_id, "<std/test>".to_string()));
+        let main = parse(src, main_id);
+        let mut module = ModuleSource::new("main", &main);
+        module.package = Some(0);
+        let r = resolve_program_for(
+            CompileTarget::Native,
+            std::slice::from_ref(&module),
+            &std_ids,
+            Some(((name, text), test_id)),
+        );
+        labels.push((main_id, "main".to_string()));
+        let mut out = dump_program(
+            CompileTarget::Native,
+            true,
+            &[module],
+            &labels,
+            Some(&r),
+            &r.diagnostics,
+        );
+        assert!(
+            out.starts_with(
+                "(Program target=native mode=test resolved=true\n  (Module 0 \"main\" package=0)"
+            ),
+            "{out}"
+        );
+        std_ids.clear();
+        let mut db = FileDb::new();
+        let mut labels = Vec::new();
+        for &(name, src) in &crate::WASM_STD_MODULES {
+            let label = format!("<std/{}>", name.strip_prefix("$std.").unwrap_or(name));
+            let id = db.add(label.clone(), src);
+            std_ids.push(id);
+            labels.push((id, label));
+        }
+        let wasm = resolve_program_for(CompileTarget::Wasm, &[], &std_ids, None);
+        out.push_str(&dump_std(CompileTarget::Wasm, false, &labels, &wasm));
+        out
+    }
+
     /// std's part of a program's resolution is std's part of a program of
     /// no module of its own: the user's modules change no std scope, and
     /// the ids, modules and items are std-relative.
@@ -842,8 +898,11 @@ mod tests {
     fn std_part_is_the_same_in_every_program() {
         let (_, std, std_labels, std_ids) = the_program();
         let alone = resolve_program_for(CompileTarget::Native, &[], &std_ids, None);
-        assert_eq!(std, std_part_of(CompileTarget::Native, &std_labels, &alone));
-        let full = dump_std(CompileTarget::Native, &std_labels, &alone);
+        assert_eq!(
+            std,
+            std_part_of(CompileTarget::Native, false, &std_labels, &alone)
+        );
+        let full = dump_std(CompileTarget::Native, false, &std_labels, &alone);
         assert_eq!(full, std.clone() + "(Diagnostics)\n");
         assert!(
             std.starts_with("(Std target=native modules=21 "),
@@ -853,6 +912,45 @@ mod tests {
         assert!(!std.contains(" u"), "std's part names a user definition");
     }
 
+    /// Under test mode too, std's part of a program's resolution is std's
+    /// part of a program of no module of its own: `std/test` joins std,
+    /// after its other modules, and its names are globbed into theirs, but
+    /// a user module's are not.
+    #[test]
+    fn std_part_is_the_same_in_every_test_mode_program() {
+        let src = "@test\nfn plain() {}\n";
+        let mut db = FileDb::new();
+        let main_id = db.add("main.nova", src);
+        let (std_ids, mut labels) = std_files(&mut db);
+        let (name, text) = crate::STD_TEST_MODULE;
+        let test_id = db.add("<std/test>", text);
+        labels.push((test_id, "<std/test>".to_string()));
+        let extra = Some(((name, text), test_id));
+        let main = parse(src, main_id);
+        let module = ModuleSource::new("main", &main);
+        let r = resolve_program_for(
+            CompileTarget::Native,
+            std::slice::from_ref(&module),
+            &std_ids,
+            extra,
+        );
+        let alone = resolve_program_for(CompileTarget::Native, &[], &std_ids, extra);
+        let std = std_part_of(CompileTarget::Native, true, &labels, &r);
+        assert_eq!(
+            std,
+            std_part_of(CompileTarget::Native, true, &labels, &alone)
+        );
+        let full = dump_std(CompileTarget::Native, true, &labels, &alone);
+        assert_eq!(full, std.clone() + "(Diagnostics)\n");
+        assert!(
+            std.starts_with("(Std target=native mode=test modules=22 "),
+            "{}",
+            &std[..80]
+        );
+        assert!(std.contains("\n  (Module 21 \"$std.test\" package=None)"));
+        assert!(!std.contains(" u"), "std's part names a user definition");
+    }
+
     /// [`resolve_program_for`] is [`crate::parse_std`] then
     /// [`crate::resolve_program_with_std`], which the resolution dumps call
     /// with std parsed once for every root (ADR 0053 §9): one `ParsedStd`
@@ -949,4 +1047,90 @@ mod tests {
             assert!(listed(severity_kind(severity)), "{severity:?}");
         }
     }
+
+    /// std resolved once ([`crate::SharedStd`], 5.2b, ADR 0054 §9) is
+    /// shared only when std alone gives no diagnostic and holds no `@test`
+    /// function, either of which the shared path would drop: with a std
+    /// whose `core` defines a name twice (E0002), or whose `ui` marks a
+    /// function `@test`, each program is resolved in full, and its dump,
+    /// under `check` and under test mode, is the full path's, std's E0002
+    /// or std's test kept.
+    #[test]
+    fn std_with_a_diagnostic_or_a_test_is_resolved_for_each_program() {
+        let alterations = [
+            (
+                "$std.core",
+                "fn mix64(x: Int) -> Int {",
+                "fn escape_common(x: Int) -> Int {",
+                "(Error \"E0002\" \"duplicate definition of `escape_common`\"",
+            ),
+            (
+                "$std.ui",
+                "\nfn flush() {",
+                "\n@test\nfn flush() {",
+                " \"flush\" should_panic=false)",
+            ),
+        ];
+        for (module, old, new, mark) in alterations {
+            let mut db = FileDb::new();
+            let main_id = db.add("main.nova", MAIN);
+            let lib_id = db.add("lib.nova", LIB);
+            let mut labels = vec![(main_id, "main".to_string()), (lib_id, "lib".to_string())];
+            let mut texts = Vec::new();
+            for &(name, src) in &STD_MODULES {
+                let text = if name == module {
+                    assert_eq!(src.matches(old).count(), 1, "{module}: {old}");
+                    src.replacen(old, new, 1)
+                } else {
+                    src.to_string()
+                };
+                let label = format!("<std/{}>", name.strip_prefix("$std.").unwrap_or(name));
+                let id = db.add(label.clone(), text.clone());
+                labels.push((id, label));
+                texts.push((name, text, id));
+            }
+            let mut diagnostics = Vec::new();
+            let modules = texts
+                .iter()
+                .map(|(name, text, id)| {
+                    let (file, diags) = crate::std_module(text, *id);
+                    diagnostics.extend(diags);
+                    (name.to_string(), file)
+                })
+                .collect();
+            let parsed = crate::ParsedStd {
+                modules,
+                diagnostics,
+            };
+            let shared = crate::SharedStd::new(CompileTarget::Native, parsed);
+            assert!(!shared.shareable, "{module}");
+            let main = parse(MAIN, main_id);
+            let lib = parse(LIB, lib_id);
+            let mut sources = [
+                ModuleSource::new("main", &main),
+                ModuleSource::new("lib", &lib),
+            ];
+            name_imports(&mut sources);
+            sources[0].text = Some(MAIN);
+            sources[1].text = Some(LIB);
+            let full =
+                crate::resolve_program_with_std(CompileTarget::Native, &sources, shared.parsed());
+            let once = crate::resolve_program_with_shared_std(&sources, &shared);
+            for test in [false, true] {
+                let dump = |r: &crate::ProgramResolution| {
+                    dump_program(
+                        CompileTarget::Native,
+                        test,
+                        &sources,
+                        &labels,
+                        Some(r),
+                        &r.diagnostics,
+                    )
+                };
+                let (want, got) = (dump(&full), dump(&once));
+                assert!(want.contains(mark), "{module}: {want}");
+                assert_eq!(got, want, "{module}, test mode {test}");
+            }
+        }
+    }
 }
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index ba8bb73ba30a785afaff67ba05f4e925a2d84577..199dabe8e50f7bc5c78fd3ecc687dc370528e3e1 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -486,10 +486,11 @@ pub fn check(cmd: CheckCmd) -> Result<()> {
 /// ([`DumpResolutionCmd::from_args`]), and then parsed by this parser of
 /// its own: so [`CheckCmd`] stays as it was, and every other `nova check`
 /// command line is parsed, refused, suggested and helped exactly as
-/// before the flag. After the flag it takes `--std`, `--target native|wasm`
-/// and files, `--` before a file that starts with `-`, each option once;
-/// it refuses everything else, `--help` and `-h` included, in clap's words
-/// under the usage `nova check --dump-resolution`.
+/// before the flag. After the flag it takes `--std`, `--target native|wasm`,
+/// `--test` (5.2b, ADR 0054) and files, `--` before a file that starts
+/// with `-`, each option once; it refuses everything else, `--help` and
+/// `-h` included, in clap's words under the usage `nova check
+/// --dump-resolution`.
 #[derive(clap::Parser)]
 #[command(
     name = "nova check --dump-resolution",
@@ -508,6 +509,11 @@ pub struct DumpResolutionCmd {
     #[arg(long, value_enum, default_value_t = TargetArg::Native)]
     target: TargetArg,
 
+    /// Resolve as `nova test` does: the root package's `@test` functions
+    /// kept, and `std/test` in std. Native only, as `nova test` is.
+    #[arg(long)]
+    test: bool,
+
     /// Each a program's root. Taken as they are, so that an empty one is a
     /// file that is not there.
     #[arg(value_parser = OsStringValueParser::new().map(PathBuf::from))]
@@ -532,9 +538,10 @@ impl DumpResolutionCmd {
 /// otherwise, for each file, `== FILE` (the path as given), then its
 /// program's part, or `IOERR` and the error's kind for a root that cannot
 /// be read, after which the rest are dumped and the exit code is 2, as
-/// `parse --dump-ast` does. With neither a file nor `--std`, or with both,
-/// it prints one line to stderr, nothing to stdout, and exits 2, as N
-/// does.
+/// `parse --dump-ast` does; with `--test`, each as `nova test` resolves
+/// it. With `--test` and `--target wasm`, with neither a file nor `--std`,
+/// or with both, it prints one line to stderr, nothing to stdout, and
+/// exits 2, as N does.
 pub fn dump_resolution(cmd: DumpResolutionCmd) -> Result<()> {
     let code = dump_resolution_code(&cmd)?;
     if code != 0 {
@@ -547,12 +554,16 @@ fn dump_resolution_code(cmd: &DumpResolutionCmd) -> Result<i32> {
     let target: nova_driver::Target = cmd.target.into();
     let stdout = std::io::stdout();
     let mut out = stdout.lock();
+    if cmd.test && target == nova_driver::Target::Wasm {
+        eprintln!("error: --dump-resolution --test is native only, as nova test is");
+        return Ok(2);
+    }
     if cmd.std {
         if !cmd.files.is_empty() {
             eprintln!("error: --dump-resolution --std takes no FILE");
             return Ok(2);
         }
-        out.write_all(nova_driver::dump_std_resolution(target).as_bytes())?;
+        out.write_all(nova_driver::dump_std_resolution(target, cmd.test).as_bytes())?;
         out.flush()?;
         return Ok(0);
     }
@@ -561,7 +572,7 @@ fn dump_resolution_code(cmd: &DumpResolutionCmd) -> Result<i32> {
         return Ok(2);
     }
     // std is parsed once for every program, as N parses it.
-    let dumps = nova_driver::ResolutionDumps::new(target);
+    let dumps = nova_driver::ResolutionDumps::new(target, cmd.test);
     let mut code = 0;
     for file in &cmd.files {
         writeln!(out, "== {}", file.display())?;
diff --git a/crates/nova-diagnostics/src/files.rs b/crates/nova-diagnostics/src/files.rs
index 76be15bc70d16120b63f5388ef9356a8315aa250..5bb529d1230051ba148a57350e44d4f2941c5ae6 100644
--- a/crates/nova-diagnostics/src/files.rs
+++ b/crates/nova-diagnostics/src/files.rs
@@ -12,13 +12,13 @@ impl FileId {
 }
 
 /// Database of source files, keyed by `FileId`.
-#[derive(Debug, Default)]
+#[derive(Debug, Default, Clone)]
 pub struct FileDb {
     files: Vec<SourceFile>,
     by_name: HashMap<String, FileId>,
 }
 
-#[derive(Debug)]
+#[derive(Debug, Clone)]
 struct SourceFile {
     name: String,
     source: String,
diff --git a/crates/nova-driver/src/lib.rs b/crates/nova-driver/src/lib.rs
index 5aa9b911bda9ef6dd2111b17f86da24e9905b753..9adb4ce6beacf182b04152489254b3ce9d0bb287 100644
--- a/crates/nova-driver/src/lib.rs
+++ b/crates/nova-driver/src/lib.rs
@@ -116,45 +116,105 @@ pub fn check_program_counted_for(program: Program, target: Target) -> Result<Che
 /// The resolution dumps of programs for one target (`nova_resolver::dump`;
 /// spec
 /// `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
-/// §3.4, ADR 0053), with std's modules lexed and parsed once for them all,
-/// as the self-hosted resolver parses them: each program's own file
-/// database registers std's modules first, so their ids, and the parsed
-/// trees' spans, are the same in every program.
+/// §3.4 and §4, ADRs 0053 and 0054), under `check`'s pipeline or `nova
+/// test`'s, with std's modules lexed and parsed once for them all, as the
+/// self-hosted resolver parses them: each program's own file database
+/// registers std's modules first, so their ids, and the parsed trees'
+/// spans, are the same in every program.
 pub struct ResolutionDumps {
     target: Target,
-    std: nova_resolver::ParsedStd,
+    test: bool,
+    std: nova_resolver::SharedStd,
+    /// A file database holding std's files alone, each program's starting
+    /// point, and their labels: registered once, as std is parsed once.
+    std_db: FileDb,
+    std_labels: Vec<(FileId, String)>,
+    /// Whether each program reads std resolved once
+    /// ([`nova_resolver::resolve_program_with_shared_std`]), or resolves
+    /// it again, as `check` does ([`ResolutionDumps::unshared`]).
+    shared: bool,
 }
 
 impl ResolutionDumps {
-    /// std's modules for `target`, parsed.
-    pub fn new(target: Target) -> ResolutionDumps {
+    /// std's modules for `target`, parsed, and `std/test` after them under
+    /// test mode (`test`), as `nova test` registers it.
+    /// std is also resolved once, and each program reads it (5.2b, ADR
+    /// 0054 §9).
+    pub fn new(target: Target, test: bool) -> ResolutionDumps {
         let mut db = FileDb::new();
-        let (std_files, _) = register_std(&mut db, target);
+        let (std_files, extra, labels) = register_std_for(&mut db, target, test);
+        let parsed = nova_resolver::parse_std(target, &std_files, extra);
         ResolutionDumps {
             target,
-            std: nova_resolver::parse_std(target, &std_files, None),
+            test,
+            std: nova_resolver::SharedStd::new(target, parsed),
+            std_db: db,
+            std_labels: labels,
+            shared: true,
         }
     }
 
-    /// The resolution dump of the program whose root is `path`: `check`'s
-    /// pipeline up to the resolver, rendering nothing. It loads the
-    /// program, stops there on any error, as `check` does, strips the
-    /// `@test` functions, and resolves; the dump holds what was built and
-    /// every diagnostic so far. A root directly in a package's `src/` or
-    /// `tests/` is `UNSUPPORTED package`: 5.2a dumps loose programs alone.
-    /// `Err` when the root cannot be read.
+    /// [`ResolutionDumps::new`], each program resolving std again, as
+    /// `check` resolves it: the dumps are the same, which a test checks.
+    pub fn unshared(target: Target, test: bool) -> ResolutionDumps {
+        ResolutionDumps {
+            shared: false,
+            ..ResolutionDumps::new(target, test)
+        }
+    }
+
+    /// The resolution dump of the program whose root is `path`, as `nova
+    /// check FILE` takes it (`Program::for_file`): a loose file and the
+    /// files beside it that it imports, or, for a file directly in a
+    /// package's `src/` or `tests/`, a module of that package, with its
+    /// graph read from the `nova.toml`s. The pipeline is `check`'s up to the
+    /// resolver, rendering nothing, and syncing nothing: the graph's
+    /// diagnostics, the program stopped there on any error; the loader, the
+    /// program stopped there on any error; then the `@test` functions
+    /// stripped, and the resolver. Under test mode it is `nova test`'s
+    /// (`FrontendContext::check(true)`): the root package's and a loose
+    /// program's `@test` functions are kept, `std/test` joins std, and the
+    /// tests of a `tests/` file are named `<file stem>::<function>`. A
+    /// registry package's warnings are dropped, as `check` hides them. The
+    /// dump holds what was built and every diagnostic so far. `Err` when the
+    /// root cannot be read.
     pub fn dump(&self, path: &Path) -> std::io::Result<String> {
-        if package_of(path).is_some() {
-            return Ok("UNSUPPORTED package\n".to_string());
+        let db = self.std_db.clone();
+        let mut labels = self.std_labels.clone();
+        let Program {
+            mut db,
+            graph,
+            roots,
+            diagnostics: graph_diagnostics,
+            ..
+        } = Program::for_file_with(path, db);
+        labels.extend(manifest_labels(graph.as_ref(), &graph_diagnostics));
+        let has_error = |ds: &[Diagnostic]| ds.iter().any(|d| d.severity == Severity::Error);
+        if has_error(&graph_diagnostics) {
+            return Ok(nova_resolver::dump::dump_program(
+                self.target,
+                self.test,
+                &[],
+                &labels,
+                None,
+                &graph_diagnostics,
+            ));
         }
-        let Program { mut db, roots, .. } = Program::loose(path);
-        let (_, mut labels) = register_std(&mut db, self.target);
-        let load = program::load_program(None, &roots, &DiskSources, &mut db)?;
+        let load = program::load_program(graph.as_ref(), &roots, &DiskSources, &mut db)?;
         let mut modules = load.modules;
-        let mut diagnostics = load.diagnostics;
-        let failed = diagnostics.iter().any(|d| d.severity == Severity::Error);
+        let registry = program::registry_files(graph.as_ref(), &modules);
+        let shown = |ds: &[Diagnostic]| -> Vec<Diagnostic> {
+            ds.iter()
+                .filter(|d| !program::a_dependencys_warning(d, &registry))
+                .cloned()
+                .collect()
+        };
+        let mut diagnostics = graph_diagnostics;
+        let loaded = shown(&load.diagnostics);
+        let failed = has_error(&loaded);
+        diagnostics.extend(loaded);
         if !failed {
-            strip_test_functions(&mut modules, false);
+            strip_test_functions(&mut modules, self.test);
         }
         labels.extend(modules.iter().map(|m| (m.file, m.name.clone())));
         let sources: Vec<ModuleSource> = modules
@@ -168,13 +228,20 @@ impl ResolutionDumps {
                 importable: m.importable.clone(),
             })
             .collect();
-        let resolved = (!failed)
-            .then(|| nova_resolver::resolve_program_with_std(self.target, &sources, &self.std));
-        if let Some(resolved) = &resolved {
-            diagnostics.extend(resolved.diagnostics.iter().cloned());
+        let mut resolved = (!failed).then(|| {
+            if self.shared {
+                nova_resolver::resolve_program_with_shared_std(&sources, &self.std)
+            } else {
+                nova_resolver::resolve_program_with_std(self.target, &sources, self.std.parsed())
+            }
+        });
+        if let Some(resolved) = &mut resolved {
+            name_tests(&mut resolved.tests, &resolved.definitions, &modules);
+            diagnostics.extend(shown(&resolved.diagnostics));
         }
         Ok(nova_resolver::dump::dump_program(
             self.target,
+            self.test,
             &sources,
             &labels,
             resolved.as_ref(),
@@ -183,13 +250,52 @@ impl ResolutionDumps {
     }
 }
 
-/// std's part of the resolution dump for `target`: the resolution of a
-/// program of no module of its own, which is std's in every program.
-pub fn dump_std_resolution(target: Target) -> String {
+/// The label of each manifest a program's graph read (ADR 0054 §1):
+/// `nova.toml` for the root package's, and `<import name>/nova.toml` for a
+/// dependency's, the name its modules' labels begin with. Without a graph
+/// only the root's manifest was read, and every file the graph's
+/// diagnostics name is it.
+fn manifest_labels(
+    graph: Option<&nova_pm::Graph>,
+    diagnostics: &[Diagnostic],
+) -> Vec<(FileId, String)> {
+    match graph {
+        Some(graph) => graph
+            .packages
+            .iter()
+            .enumerate()
+            .map(|(i, p)| {
+                let label = if i == 0 {
+                    nova_pm::MANIFEST.to_string()
+                } else {
+                    format!("{}/{}", p.import_name, nova_pm::MANIFEST)
+                };
+                (p.manifest_file, label)
+            })
+            .collect(),
+        None => {
+            let mut files: Vec<FileId> = Vec::new();
+            for label in diagnostics.iter().flat_map(|d| &d.labels) {
+                if !files.contains(&label.span.file) {
+                    files.push(label.span.file);
+                }
+            }
+            files
+                .into_iter()
+                .map(|file| (file, nova_pm::MANIFEST.to_string()))
+                .collect()
+        }
+    }
+}
+
+/// std's part of the resolution dump for `target`, under test mode with
+/// `std/test` (`test`): the resolution of a program of no module of its
+/// own, which is std's in every program.
+pub fn dump_std_resolution(target: Target, test: bool) -> String {
     let mut db = FileDb::new();
-    let (std_files, labels) = register_std(&mut db, target);
-    let resolved = nova_resolver::resolve_program_for(target, &[], &std_files, None);
-    nova_resolver::dump::dump_std(target, &labels, &resolved)
+    let (std_files, extra, labels) = register_std_for(&mut db, target, test);
+    let resolved = nova_resolver::resolve_program_for(target, &[], &std_files, extra);
+    nova_resolver::dump::dump_std(target, test, &labels, &resolved)
 }
 
 /// Register each of `target`'s std modules in `db` as `check` does, as
@@ -207,6 +313,31 @@ fn register_std(db: &mut FileDb, target: Target) -> (Vec<FileId>, Vec<(FileId, S
     (ids, labels)
 }
 
+/// [`register_std`], then, under test mode, `std/test` as `nova test`
+/// registers it (`FrontendContext::check`): the target's std ids, the
+/// extra module the resolver takes, and every id with its name.
+#[allow(clippy::type_complexity)]
+fn register_std_for(
+    db: &mut FileDb,
+    target: Target,
+    test: bool,
+) -> (
+    Vec<FileId>,
+    Option<((&'static str, &'static str), FileId)>,
+    Vec<(FileId, String)>,
+) {
+    let (ids, mut labels) = register_std(db, target);
+    let extra = test.then(|| {
+        let (name, src) = nova_resolver::STD_TEST_MODULE;
+        let short = name.strip_prefix("$std.").unwrap_or(name);
+        let label = format!("<std/{short}>");
+        let id = db.add(label.clone(), src);
+        labels.push((id, label));
+        (nova_resolver::STD_TEST_MODULE, id)
+    });
+    (ids, extra, labels)
+}
+
 /// The stack of the threads the pipeline runs on: every command's (`nova`'s
 /// `main`), the language server's checker's and `nova dev`'s rebuilds'
 /// (spec
@@ -1415,6 +1546,7 @@ mod resolution_dumps {
         });
     }
 }
+
 /// What doc tests need of a build (spec
 /// `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
 /// §7, §11.2), where only this crate can see it.
diff --git a/crates/nova-driver/src/program.rs b/crates/nova-driver/src/program.rs
index 308fffe63173bd5d3b3efc7ef4e24faf5b827753..6d292037f695ce245125cbf1db1608efb2432e36 100644
--- a/crates/nova-driver/src/program.rs
+++ b/crates/nova-driver/src/program.rs
@@ -89,10 +89,20 @@ impl Program {
     /// package when it is directly in a package's `src/` or `tests/`, and
     /// otherwise loose, reading no manifest.
     pub fn for_file(path: &Path) -> Program {
+        Program::for_file_with(path, FileDb::new())
+    }
+
+    /// [`Program::for_file`], adding the manifests it reads to `db`, which
+    /// may hold files already: the resolution dump registers std's modules
+    /// first, so that their ids are the same in every program (ADR 0053
+    /// §9).
+    pub fn for_file_with(path: &Path, mut db: FileDb) -> Program {
         let Some((root, kind)) = package_of(path) else {
-            return Program::loose(path);
+            return Program {
+                db,
+                ..Program::loose(path)
+            };
         };
-        let mut db = FileDb::new();
         let (graph, diagnostics) = nova_pm::graph(&root, &mut db);
         let runs = kind == RootKind::Src && path.file_stem().is_some_and(|stem| stem == "main");
         Program {
diff --git a/crates/nova-resolver/src/dump.rs b/crates/nova-resolver/src/dump.rs
index af0999799482a6501705522061ae6ae97f65ede7..f9dab67949445f8aea064de45930b5259d0e665a 100644
--- a/crates/nova-resolver/src/dump.rs
+++ b/crates/nova-resolver/src/dump.rs
@@ -7,7 +7,8 @@
 //! It carries no stability promise: it changes whenever resolution does, on
 //! both sides in one pull request. `nova check --dump-resolution FILE…`
 //! prints a program's part after a `== FILE` header, and `nova check
-//! --dump-resolution --std` std's part.
+//! --dump-resolution --std` std's part; `--target wasm` resolves for the
+//! WASM target, and `--test` as `nova test` resolves (ADR 0054).
 //!
 //! - It follows the AST dump's conventions (ADR 0052 §1): a line is its
 //!   indentation, two spaces per depth, `(` and its kind, then its values
@@ -99,11 +100,12 @@ pub const KINDS: &[&str] = &[
 
 /// The forms a loose program does not print natively outside test mode:
 /// 5.2b's packages (a module's package key), its WASM target (`missing`)
-/// and its test mode (`@test` functions are stripped under `check`), and
-/// the two severities the resolver never gives.
+/// and its test mode (` mode=test`; `@test` functions are stripped under
+/// `check`), and the two severities the resolver never gives.
 pub const LATER_KINDS: &[&str] = &[
     "target=wasm",
     "package=<n>",
+    "mode=test",
     "(Test",
     "(Test should_panic=true",
     "(Test should_panic=false",
@@ -116,12 +118,14 @@ pub const LATER_KINDS: &[&str] = &[
 /// label, and std's modules' names in the file database, as `<std/core>`.
 pub type Labels = [(FileId, String)];
 
-/// A program's part (module docs) for `target`: its `modules` as the loader
-/// gave them, the files' `labels`, its resolution once it got that far, and
-/// every diagnostic, the loader's and the resolver's, in order. Without a
+/// A program's part (module docs) for `target`, under test mode when
+/// `test`: its `modules` as the loader gave them, the files' `labels`, its
+/// resolution once it got that far, and every diagnostic, the package
+/// graph's, the loader's and the resolver's, in order. Without a
 /// resolution the part holds the modules loaded and the diagnostics.
 pub fn dump_program(
     target: CompileTarget,
+    test: bool,
     modules: &[ModuleSource],
     labels: &Labels,
     resolution: Option<&ProgramResolution>,
@@ -130,6 +134,7 @@ pub fn dump_program(
     let mut d = Dump::new(labels, u32::MAX);
     d.open(0, "Program");
     d.field("target", target.name());
+    d.mode(test);
     d.bool_field("resolved", resolution.is_some());
     for (index, m) in modules.iter().enumerate() {
         d.open(1, "Module");
@@ -197,11 +202,17 @@ pub fn dump_program(
     d.out
 }
 
-/// std's part (module docs) for `target`, from the resolution of a program
-/// of no module of its own, whose std files have `labels`.
-pub fn dump_std(target: CompileTarget, labels: &Labels, resolution: &ProgramResolution) -> String {
+/// std's part (module docs) for `target`, under test mode when `test`,
+/// from the resolution of a program of no module of its own, whose std
+/// files have `labels`.
+pub fn dump_std(
+    target: CompileTarget,
+    test: bool,
+    labels: &Labels,
+    resolution: &ProgramResolution,
+) -> String {
     let mut d = Dump::new(labels, 0);
-    d.std_part(target, &resolution.definitions, 0, 0, 0);
+    d.std_part(target, test, &resolution.definitions, 0, 0, 0);
     d.out.push('\n');
     d.diagnostics(&resolution.diagnostics);
     d.out
@@ -212,6 +223,7 @@ pub fn dump_std(target: CompileTarget, labels: &Labels, resolution: &ProgramReso
 /// every program for one target, which a test checks.
 pub fn std_part_of(
     target: CompileTarget,
+    test: bool,
     labels: &Labels,
     resolution: &ProgramResolution,
 ) -> String {
@@ -224,7 +236,7 @@ pub fn std_part_of(
         .unwrap_or(defs.item_module.len());
     let std_def = first_std_def(defs, std_item);
     let mut d = Dump::new(labels, std_def);
-    d.std_part(target, defs, std_module, std_def, std_item);
+    d.std_part(target, test, defs, std_module, std_def, std_item);
     d.out.push('\n');
     d.out
 }
@@ -407,6 +419,14 @@ impl<'a> Dump<'a> {
         let _ = write!(self.out, " {key}={value}");
     }
 
+    /// ` mode=test` under test mode, and nothing under `check`'s, so that
+    /// 5.2a's dumps are as they were.
+    fn mode(&mut self, test: bool) {
+        if test {
+            self.field("mode", "test");
+        }
+    }
+
     fn bool_field(&mut self, key: &str, value: bool) {
         let _ = write!(self.out, " {key}={value}");
     }
@@ -444,6 +464,7 @@ impl<'a> Dump<'a> {
     fn std_part(
         &mut self,
         target: CompileTarget,
+        test: bool,
         defs: &Definitions,
         std_module: usize,
         std_def: u32,
@@ -451,6 +472,7 @@ impl<'a> Dump<'a> {
     ) {
         self.open(0, "Std");
         self.field("target", target.name());
+        self.mode(test);
         let modules = defs.modules.len() - std_module;
         self.field("modules", &modules.to_string());
         self.field("defs", &(defs.defs.len() as u32 - std_def).to_string());
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 7f9ab24798059a1a4063065d7fbba6b34255477b..2df4e736ae0fef4e7d3f689e05731d02241a3c72 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -1875,6 +1875,38 @@ impl Exports {
         }
     }
 
+    /// These exports with every definition's id `offset` higher, as they
+    /// are when `offset` definitions come before their module's
+    /// ([`resolve_program_with_shared_std`]).
+    fn rebased(&self, offset: u32) -> Exports {
+        let def = |id: DefId| DefId(id.0 + offset);
+        let res = |r: Res| match r {
+            Res::Def(id) => Res::Def(def(id)),
+            Res::Variant(sum, index) => Res::Variant(def(sum), index),
+            Res::Builtin(b) => Res::Builtin(b),
+        };
+        Exports {
+            values: self
+                .values
+                .iter()
+                .map(|(k, r)| (k.clone(), res(*r)))
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
+            value_names: self.value_names.clone(),
+            type_names: self.type_names.clone(),
+            trait_names: self.trait_names.clone(),
+        }
+    }
+
     /// The exported values, in the order they were exported.
     fn values_in_order(&self) -> Vec<(String, Res)> {
         self.value_names
@@ -2296,6 +2328,234 @@ pub fn resolve_program_with_std(
     }
 }
 
+/// std resolved once for many programs, for the hidden resolution dump
+/// alone (5.2b, ADR 0054 §9): the resolution of a program of no module of
+/// its own, from which each program takes std's exports, its globbed
+/// variants and its `missing` names, unchanged. A program's resolution
+/// never changes it.
+pub struct SharedStd {
+    target: CompileTarget,
+    std: ParsedStd,
+    /// Each std module's exports, its definitions numbered from 0.
+    exports: Vec<Exports>,
+    /// [`std_globbed_variants`] of std.
+    variants: FxHashMap<String, String>,
+    missing: FxHashMap<String, &'static str>,
+    /// Whether resolving std gave no diagnostic of its own, beyond parsing
+    /// it, and found no `@test` function: a diagnostic would come between
+    /// a program's, and a test would be listed with a program's, which the
+    /// shared path drops, so a program is then resolved by
+    /// [`resolve_program_with_std`].
+    shareable: bool,
+}
+
+impl SharedStd {
+    /// std as parsed, which [`resolve_program_with_std`] takes.
+    pub fn parsed(&self) -> &ParsedStd {
+        &self.std
+    }
+
+    /// `std`, parsed for `target`, resolved once.
+    pub fn new(target: CompileTarget, std: ParsedStd) -> SharedStd {
+        let alone = resolve_program_with_std(target, &[], &std);
+        let all: Vec<ModuleSource> = std
+            .modules
+            .iter()
+            .filter_map(|(name, file)| {
+                file.as_ref()
+                    .map(|file| ModuleSource::new(name.clone(), file))
+            })
+            .collect();
+        let variants =
+            std_globbed_variants(&alone.definitions, &alone.definitions.exports, &all, 0);
+        let shareable = alone.diagnostics.len() == std.diagnostics.len() && alone.tests.is_empty();
+        SharedStd {
+            target,
+            exports: alone.definitions.exports,
+            variants,
+            missing: alone.definitions.missing,
+            shareable,
+            std,
+        }
+    }
+}
+
+/// [`resolve_program_with_std`] with std resolved once ([`SharedStd`]),
+/// for the hidden resolution dump alone (5.2b, ADR 0054 §9): the user's
+/// modules go through the same steps, in the same order, and read std's
+/// exports, rebased past the user's definitions, where the full
+/// resolution reads std's own; std's modules are not resolved again, and
+/// the result holds the user's definitions, scopes and items, and every
+/// module's exports, which is what the dump prints. Not for the type
+/// checker: std's definitions, scopes, items and import occurrences are
+/// not in it. A program whose resolution would differ, because std's gave
+/// a diagnostic or holds a `@test` function, is resolved in full.
+pub fn resolve_program_with_shared_std(
+    modules: &[ModuleSource],
+    shared: &SharedStd,
+) -> ProgramResolution {
+    let target = shared.target;
+    if !shared.shareable {
+        return resolve_program_with_std(target, modules, &shared.std);
+    }
+    let mut definitions = Definitions::default();
+    let mut diagnostics = shared.std.diagnostics.clone();
+    let mut merged = Vec::new();
+    let mut tests = Vec::new();
+    let mut imports: Vec<Occurrence> = Vec::new();
+    let user_count = modules.len();
+    let all: Vec<ModuleSource> = modules
+        .iter()
+        .map(|m| ModuleSource {
+            name: m.name.clone(),
+            file: m.file,
+            imports: m.imports.clone(),
+            text: m.text,
+            package: m.package,
+            importable: m.importable.clone(),
+        })
+        .chain(shared.std.modules.iter().filter_map(|(name, file)| {
+            file.as_ref()
+                .map(|file| ModuleSource::new(name.clone(), file))
+        }))
+        .collect();
+    let std_start = user_count;
+    let mut exports: Vec<Exports> = Vec::new();
+    for m in &all[..std_start] {
+        let mut scope = ModuleScope {
+            name: m.name.clone(),
+            ..Default::default()
+        };
+        for b in Builtin::GLOBAL {
+            scope.values.insert(b.name().to_string(), Res::Builtin(b));
+        }
+        definitions.modules.push(scope);
+        exports.push(Exports::default());
+    }
+    let owned = |mid: usize| mid < std_start && matches!(all[mid].package, None | Some(0));
+    for (mid, m) in all.iter().enumerate().take(std_start) {
+        let mut first_value: IndexMap<String, Span> = IndexMap::new();
+        let mut first_type: IndexMap<String, Span> = IndexMap::new();
+        let before = diagnostics.len();
+        for item in &m.file.items {
+            let item_index = merged.len();
+            definitions.item_module.push(mid as u32);
+            collect_item(
+                &mut definitions.defs,
+                &mut definitions.modules[mid],
+                &mut exports[mid],
+                &mut first_value,
+                &mut first_type,
+                &mut diagnostics,
+                &mut tests,
+                item_index,
+                &item.value,
+            );
+            merged.push(item.clone());
+        }
+        if !owned(mid) {
+            diagnostics[before..]
+                .iter_mut()
+                .for_each(|d| d.fixes.clear());
+        }
+    }
+    // std's definitions come after the user's.
+    let offset = definitions.defs.len() as u32;
+    exports.extend(shared.exports.iter().map(|e| e.rebased(offset)));
+    if target == CompileTarget::Wasm {
+        for m in &all[..std_start] {
+            for item in &m.file.items {
+                if let Item::Extern(_) = &item.value {
+                    diagnostics.push(
+                        Diagnostic::error(
+                            "E0090",
+                            "extern functions are not available on target wasm",
+                        )
+                        .with_primary_label(item.span, "this `extern` block"),
+                    );
+                }
+            }
+        }
+    }
+    let mut public_fixes: Vec<(usize, usize, String)> = Vec::new();
+    for (mid, m) in all.iter().enumerate().take(std_start) {
+        let before = diagnostics.len();
+        for item in &m.file.items {
+            if let Item::Import(imp) = &item.value {
+                resolve_import(
+                    &mut definitions,
+                    &exports,
+                    &all,
+                    target,
+                    mid,
+                    imp,
+                    &mut diagnostics,
+                    &mut imports,
+                    &mut public_fixes,
+                );
+            }
+        }
+        if !owned(mid) {
+            diagnostics[before..]
+                .iter_mut()
+                .for_each(|d| d.fixes.clear());
+        }
+    }
+    // No std module imports a user's, so none would receive a name.
+    for (at, fix_target, name) in public_fixes {
+        let clash = all[..std_start].iter().enumerate().any(|(g, m)| {
+            g != fix_target
+                && would_receive(m, fix_target, &name)
+                && binds(&definitions.modules[g], &name)
+        });
+        if clash {
+            diagnostics[at].fixes.clear();
+        }
+    }
+    // std's glob into the user's modules; `definitions.modules` holds no
+    // std module's scope, so a glob into std's alone binds nothing.
+    for (mid, std_exports) in exports.iter().enumerate().skip(std_start) {
+        let name = all[mid].name.as_str();
+        if IMPORTED_STD.iter().any(|&(_, m)| m == name) {
+            continue;
+        }
+        let from = if STD_INTERNAL.contains(&name) {
+            std_start
+        } else {
+            0
+        };
+        import_std_module(&mut definitions, std_exports, mid, from);
+    }
+    diagnostics.extend(variant_shadowing_of(&shared.variants, &all[..std_start]));
+    definitions.importable = all
+        .iter()
+        .map(|m| {
+            m.importable
+                .iter()
+                .map(|(n, i)| (n.clone(), ModuleId(*i as u32)))
+                .collect()
+        })
+        .collect();
+    definitions.packages = all.iter().map(|m| m.package).collect();
+    definitions.std_start = std_start;
+    definitions.exports = exports;
+    definitions.target = target;
+    definitions.missing = shared.missing.clone();
+    ProgramResolution {
+        file: File {
+            items: merged,
+            newline_splits: all[..std_start]
+                .iter()
+                .flat_map(|m| m.file.newline_splits.iter().copied())
+                .collect(),
+        },
+        definitions,
+        diagnostics,
+        tests,
+        imports,
+    }
+}
+
 /// The variants std brings into every module, by name: each public variant
 /// of a std module whose names glob into user modules, with its sum's name.
 /// The first module to export a name wins, as std's glob does.
@@ -2333,8 +2593,18 @@ fn variant_shadowing(
     std_start: usize,
 ) -> Vec<Diagnostic> {
     let std_variants = std_globbed_variants(definitions, exports, all, std_start);
+    variant_shadowing_of(&std_variants, &all[..std_start.min(all.len())])
+}
+
+/// [`variant_shadowing`]'s warnings for `user`, the program's own modules,
+/// from `std_variants`, the variants std globs into them, each with its
+/// sum's name ([`std_globbed_variants`]).
+fn variant_shadowing_of(
+    std_variants: &FxHashMap<String, String>,
+    user: &[ModuleSource],
+) -> Vec<Diagnostic> {
     let mut out = Vec::new();
-    for m in &all[..std_start.min(all.len())] {
+    for m in user {
         for item in &m.file.items {
             let Item::Type(t) = &item.value else {
                 continue;
````

### Task 2: N's manifest reader, package loader, WASM target and test mode

Spec §4.1-§4.3; decisions 2, 12, 14 to 16, 18, 20, 23 and 25.

**Files:**
- Create: `selfhost/src/manifest.nova`
- Modify: `selfhost/src/loader.nova`, `selfhost/src/loader_tests.nova`,
  `selfhost/src/main.nova`, `selfhost/src/resolver.nova`,
  `selfhost/src/resolution_dump.nova`

- [ ] **Step 1: The tests.** `patch_block t2-tests`: `loader_tests.nova`'s
  seven new `@test`s:
  `a_package_loads_its_siblings_and_its_dependencies_libraries` (the
  modules' order, labels, keys), `e0004_and_e0001s_notes_are_the_loaders`,
  `test_mode_keeps_the_root_packages_tests_and_strips_a_dependencys`,
  `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`,
  `the_manifest_readers_accepted_and_refused_forms_are_pinned` (16 read,
  51 refused, decisions 16 and 23: each number of a version at most
  `u64::MAX`, a dev-dependency's version entries, DEL, a 65-character
  name, `1.0.0.0`), `the_graph_refuses_what_nova_pm_gives_an_m_code`
  (with a dependency's version past `u64::MAX`, the root's dev-dependency
  version entry refused and a dependency's read)
  and `a_root_is_a_packages_module_directly_in_its_src_or_tests`.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`.
  - `SH the_selfhost_tests_pass`. Expected: `0 passed; 1 failed`: N's
    tests do not compile.
  - `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 3`, stderr
    too, where a compile error goes. Expected: `Error: could not compile
    due to 1 previous error`, the import of `manifest`: `no such
    module`.
- [ ] **Step 3: The code.** `patch_block t2-code`:
  - **`manifest.nova`** (new): `parse_manifest`, which reads what ADR 0054
    §5 lists and refuses the rest with a message; `import_name`,
    `is_keyword`;
  - **`loader.nova`:** the package graph (`read_graph`, `read_package`,
    `expand`, `visit`), each M-code S0 would give a refusal, M0011 saying
    two paths may be one directory where folding cannot tell
    (`climbs_above`, decisions 23 and 25); the package
    loader (`Place`, `root_place`, `label`, `find_place` with E0004 and
    E0001's two notes, `visible_edges`, `clash`); `strip_test_functions`'s
    `keep_root_tests`; `package_of`;
  - **`resolver.nova`:** `Exports::rebased`, `SharedStd`, `share_std`,
    `resolve_program_shared`, `variant_shadowing_of` (decision 18);
  - **`resolution_dump.nova`:** ` mode=test` and `test` threaded through;
  - **`main.nova`:** `selfhost resolve --std DIR [--target native|wasm]
    [--test] (--std-part | FILE...)`, each option once, an empty target
    and `--test=VALUE` refused as clap refuses them, `--test` with
    `--target wasm` refused (exit 2); `load_std` with `std/test` under
    test mode and `STD_MODULES` parsed once for `missing`; a refused
    manifest's line on stderr and exit 1; `name_tests`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH the_selfhost_tests_pass`. Expected: `1 passed; 0 failed`.
  - `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 3`.
    Expected: `test result: ok. 32 passed; 0 failed; 0 trapped; 32 total`.
  - `SH`. Expected: `37 passed; 0 failed`.
  - `git status --short`. Expected: nothing.
  - `S` (207 s). Expected: `2862 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `selfhost: N's manifest reader, package loader,
      WASM target and test mode`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/selfhost/src/loader_tests.nova b/selfhost/src/loader_tests.nova
index f83ae74e8e1ad379587ac73dec2d662bb438940e..8d3493f6eb738a854822e551be3090653c1b28eb 100644
--- a/selfhost/src/loader_tests.nova
+++ b/selfhost/src/loader_tests.nova
@@ -2,8 +2,11 @@
 // first from its root, a missing sibling left to the resolver, a sibling
 // under a std module's name refused with E0093, what each module could
 // import, the `@test` functions `check` strips, and the paths and the sort
-// the loader leans on. Each expected value is what S0's driver gives for
-// the same files; the differential compares the corpus and the cases.
+// the loader leans on; and, from 5.2b (ADR 0054), a package's modules, its
+// dependencies' and their labels, E0004, E0001's two notes, the tests test
+// mode keeps, the manifest reader and the package graph, and what they
+// refuse. Each expected value is what S0's driver gives for the same
+// files; the differential compares the corpus and the cases.
 //
 // They live in `src/`, imported by `main.nova`, for the reason
 // lexer_tests.nova gives.
@@ -12,6 +15,7 @@ import ast
 import diagnostic
 import lexer
 import loader
+import manifest
 import parser
 import token
 
@@ -34,7 +38,7 @@ fn a_loose_program_loads_breadth_first_from_its_root() {
         ("ui.nova", "pub fn fu() {}\n"),
     ]
     for (name, text) in files.iter() { let _ = block_on(write_string("${dir}/${name}", text)) }
-    let load = match block_on(load_program("${dir}/main.nova")) {
+    let load = match block_on(load_program(None, "${dir}/main.nova", RootLoose)) {
         Ok(l) => l
         Err(_) => {
             assert(false, "unreachable")
@@ -69,7 +73,7 @@ fn a_loose_program_loads_breadth_first_from_its_root() {
     assert_eq(d.labels.at(0).span.start, 39)
     assert_eq(d.labels.at(0).span.end, 41)
     // A root that is not there is an error; a sibling that is not is not.
-    match block_on(load_program("${dir}/missing.nova")) {
+    match block_on(load_program(None, "${dir}/missing.nova", RootLoose)) {
         Ok(_) => assert(false, "unreachable")
         Err(e) => {
             match e.kind {
@@ -95,11 +99,13 @@ fn check_strips_test_functions_and_nothing_else() {
             path: "main.nova",
             ast: file,
             text: source,
+            package: -1,
+            test_file: false,
             imports: Map::new(),
             importable: Vec::new(),
         },
     )
-    strip_test_functions(modules)
+    strip_test_functions(modules, false)
     let items = modules.at(0).ast.items
     assert_eq(items.len(), 2)
     match items.at(0).value {
@@ -141,3 +147,453 @@ fn paths_split_and_join_as_rusts_do() {
     let sorted = sorted_dedup(Vec::from_array(["b", "B", "a", "ab", "b", "_"]))
     assert_eq(lines(sorted.to_array()), lines(["B", "_", "a", "ab", "b"]))
 }
+
+// Write `files`, each a path under `dir` and its text, into a fresh `dir`.
+fn write_tree(dir: String, files: [(String, String)]) {
+    let _ = block_on(remove_dir_all(dir))
+    for (path, text) in files.iter() {
+        let full = "${dir}/${path}"
+        let _ = block_on(create_dir_all(path_parent(full).unwrap_or(dir)))
+        let _ = block_on(write_string(full, text))
+    }
+}
+
+// The manifest of the package `name`, and `more` after it.
+fn manifest_of(name: String, more: String) -> String {
+    "[package]\nname = \"${name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n${more}"
+}
+
+// The program whose root is `root`, a module of the package in `dir`, or
+// why its graph or its root could not be read. A helper is built with N,
+// where `assert` is not in scope, so it panics with the reason itself.
+fn load_package(dir: String, root: String, kind: RootKind) -> Load {
+    let graph = match block_on(read_graph(dir)) {
+        Ok(g) => g
+        Err(why) => panic("the graph of ${dir}: ${why}")
+    }
+    match block_on(load_program(Some(graph), "${dir}/${root}", kind)) {
+        Ok(l) => l
+        Err(_) => panic("${dir}/${root} cannot be read")
+    }
+}
+
+// Each module's label, package key and whether its tests are named for
+// its file, one to a line.
+fn module_lines(load: Load) -> String {
+    let mut out: Vec<String> = Vec::new()
+    for m in load.modules.iter() { out.push("${m.name} ${m.package} ${m.test_file}") }
+    lines(out.to_array())
+}
+
+@test
+fn a_package_loads_its_siblings_and_its_dependencies_libraries() {
+    // app/src/main imports util, a sibling, and geom, a path dependency,
+    // whose library imports its own `shapes`. Breadth first: main, util,
+    // geom, geom/shapes; the dependency's modules are labelled by its
+    // import name, its library by the name alone, and its package key is 1.
+    let dir = "${temp_dir()}/nova-selfhost-package-test"
+    write_tree(
+        dir,
+        [
+            (
+                "nova.toml",
+                manifest_of("app", "\n[dependencies]\ngeom = { path = \"pkgs/geom\" }\n"),
+            ),
+            ("src/main.nova", "import util\nimport geom\nfn main() {}\n"),
+            ("src/util.nova", "pub fn u() {}\n"),
+            ("tests/api.nova", "import util\nimport app\n"),
+            ("src/lib.nova", "pub fn l() {}\n"),
+            ("pkgs/geom/nova.toml", manifest_of("geom", "")),
+            ("pkgs/geom/src/lib.nova", "import shapes\npub fn g() {}\n"),
+            ("pkgs/geom/src/shapes.nova", "pub fn s() {}\n"),
+        ],
+    )
+    let load = load_package(dir, "src/main.nova", RootSrc)
+    assert_eq(load.diagnostics.len(), 0)
+    assert_eq(
+        module_lines(load),
+        lines(["main 0 false", "util 0 false", "geom 1 false", "geom/shapes 1 false"]),
+    )
+    let mut importable: Vec<String> = Vec::new()
+    for (name, index) in load.modules.at(0).importable.iter() {
+        importable.push("${name} ${index}")
+    }
+    assert_eq(lines(importable.to_array()), lines(["geom 2", "util 1"]))
+    match load.modules.at(0).imports.get("geom") {
+        Some(ImportTargetModule(t)) => assert_eq(t, 2)
+        _ => assert(false, "unreachable")
+    }
+    // A `tests/` module sees `src/`'s files only through the package's
+    // library, by its import name, which is the root package's module
+    // `lib`; `util` is no file of `tests/`.
+    let tests = load_package(dir, "tests/api.nova", RootTests)
+    assert_eq(module_lines(tests), lines(["tests/api 0 true", "lib 0 false"]))
+    assert(!tests.modules.at(0).imports.contains_key("util"), "util is src/'s")
+    let _ = block_on(remove_dir_all(dir))
+}
+
+@test
+fn e0004_and_e0001s_notes_are_the_loaders() {
+    // `shapes` is both a sibling and a dependency (E0004, its secondary
+    // label the entry, in the manifest labelled `nova.toml`); `helper` is
+    // a dev-dependency imported from src/ (E0001 and a note); from tests/,
+    // the package itself, which has no library, is E0001 with a note.
+    let dir = "${temp_dir()}/nova-selfhost-package-clash-test"
+    let toml = manifest_of(
+        "app",
+        "[dependencies]\nshapes = { path = \"shapes\" }\n[dev-dependencies]\nhelper = { path = \"helper\" }\n",
+    )
+    write_tree(
+        dir,
+        [
+            ("nova.toml", toml),
+            ("src/main.nova", "import shapes\nimport helper\nfn main() {}\n"),
+            ("src/shapes.nova", "pub fn a() {}\n"),
+            ("tests/api.nova", "import app\nimport helper\n"),
+            ("shapes/nova.toml", manifest_of("shapes", "")),
+            ("shapes/src/lib.nova", "pub fn b() {}\n"),
+            ("helper/nova.toml", manifest_of("helper", "")),
+            ("helper/src/lib.nova", "pub fn h() {}\n"),
+        ],
+    )
+    let load = load_package(dir, "src/main.nova", RootSrc)
+    assert_eq(load.diagnostics.len(), 2)
+    let clash = load.diagnostics.at(0)
+    assert_eq(clash.code, "E0004")
+    assert_eq(
+        clash.message,
+        "`shapes` is both a module of this package and a dependency: ${dir}/src/shapes.nova, and the entry at ${dir}/nova.toml:6",
+    )
+    assert_eq(clash.labels.at(1).file, "nova.toml")
+    assert_eq(clash.labels.at(1).span.start, 73)
+    assert_eq(clash.labels.at(1).span.end, 79)
+    let dev = load.diagnostics.at(1)
+    assert_eq(dev.code, "E0001")
+    assert_eq(dev.notes.at(0), "`helper` is a dev-dependency, which only `tests/` files can import")
+    let tests = load_package(dir, "tests/api.nova", RootTests)
+    assert_eq(tests.diagnostics.len(), 1)
+    assert_eq(tests.diagnostics.at(0).notes.at(0), "`app` has no src/lib.nova")
+    assert_eq(module_lines(tests), lines(["tests/api 0 true", "helper 2 false"]))
+    let _ = block_on(remove_dir_all(dir))
+}
+
+@test
+fn test_mode_keeps_the_root_packages_tests_and_strips_a_dependencys() {
+    let source = "@test\nfn t() {}\nfn main() {}\n"
+    let mut modules: Vec<Loaded> = Vec::new()
+    for key in [-1, 0, 1].iter() {
+        let (tokens, _) = lex(source)
+        let (file, _) = parse(tokens)
+        modules.push(
+            Loaded {
+                name: "m",
+                path: "m.nova",
+                ast: file,
+                text: source,
+                package: key,
+                test_file: false,
+                imports: Map::new(),
+                importable: Vec::new(),
+            },
+        )
+    }
+    strip_test_functions(modules, true)
+    assert_eq(modules.at(0).ast.items.len(), 2)
+    assert_eq(modules.at(1).ast.items.len(), 2)
+    assert_eq(modules.at(2).ast.items.len(), 1)
+}
+
+@test
+fn the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest() {
+    let text = "# app\n[package]\nname = \"json-api\"   # a name\nversion = \"1.2.3\"\nedition = '2026'\nauthors = [\"a\", \"b\",]\n\n[dependencies]\ngeom = { path = \"../geom\" }\n[dev-dependencies]\nhelper = {path=\"h\"}\n"
+    match parse_manifest(text) {
+        Ok(m) => {
+            assert_eq(m.name, "json-api")
+            assert_eq(import_name(m.name), "json_api")
+            // From `[package]`'s `[` to its last value's end.
+            assert_eq(m.span.start, 6)
+            assert_eq(m.span.end, 101)
+            assert_eq(m.dependencies.len(), 1)
+            assert_eq(m.dependencies.at(0).path, "../geom")
+            assert_eq(m.dependencies.at(0).span.start, 118)
+            assert_eq(m.dependencies.at(0).span.end, 122)
+            assert_eq(m.dev_dependencies.at(0).name, "helper")
+        }
+        Err(why) => assert(false, why)
+    }
+    // A line break may be `\r\n`; spans count the `\r`s.
+    match parse_manifest(
+        "[package]\r\nname = \"a\"\r\nversion = \"0.1.0\"\r\nedition = \"2026\"\r\n",
+    ) {
+        Ok(m) => assert_eq(m.span.end, 58)
+        Err(why) => assert(false, why)
+    }
+    let refused = [
+        manifest_of("app", "color = \"red\"\n"),
+        manifest_of("app", "[dependencies]\njson = \"1.0\"\n"),
+        manifest_of("app", "[dependencies]\njson = { version = \"1.0\", path = \"j\" }\n"),
+        manifest_of("app", "[dependencies.geom]\npath = \"geom\"\n"),
+        manifest_of("app", "[package]\n"),
+        manifest_of("app", "[tool]\n"),
+        manifest_of("app", "description = \"a\\tb\"\n"),
+        manifest_of("app", "description = \"\"\"x\"\"\"\n"),
+        manifest_of("ui", ""),
+        manifest_of("1app", ""),
+        manifest_of("con", ""),
+        "[package]\nname = \"app\"\nversion = \"01.0.0\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2027\"\n",
+        "[package]\nname = \"app\"\nversion = \"0.1.0\"\n",
+        "name = \"app\"\n",
+    ]
+    for text in refused.iter() {
+        match parse_manifest(text) {
+            Ok(_) => assert(false, "refused: ${text}")
+            Err(_) => ()
+        }
+    }
+}
+
+// The reader's table (ADR 0054 §5), pinned: each text here is read, and
+// each in the second list refused, whatever else S0's reader would say
+// of it. S0 accepts every text of the first list too; for the second it
+// gives an M-code, or reads what N's reader does not (a version entry
+// with the registry, an escape, a number).
+@test
+fn the_manifest_readers_accepted_and_refused_forms_are_pinned() {
+    let accepted = [
+        manifest_of("app", ""),
+        "# a comment\n\n[ package ]\nname = 'app'   # a name\nversion = \"10.20.30\"\nedition = '2026'\n",
+        manifest_of(
+            "my-app_2",
+            "description = \"d\"\nlicense = 'MIT'\nrepository = \"r\"\nauthors = []\nkeywords = [\"a\", 'b',]\ncategories = [\"c\"]\n",
+        ),
+        manifest_of("app", "[dependencies]\n[dev-dependencies]\n"),
+        manifest_of(
+            "app",
+            "[dependencies]\ngeom = {path='g'}\n[dev-dependencies]\nhelp = { path = \"../h\" }\n",
+        ),
+        "[package]\r\nname = \"app\"\r\nversion = \"0.1.0\"\r\nedition = \"2026\"\r\n\r\n[dependencies]\r\ng = { path = \"g\" }\r\n",
+        "[package]\nedition = \"2026\"\nversion = \"0.1.0\"\nname = \"app\"\n",
+        // As `nova add` writes into an inline table; a byte-order mark;
+        // semver's pre-release.
+        "dependencies = {}\n\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+        "dev-dependencies = { geom = { path = \"../geom\" }, h = {path='h'} }\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+        "${char_from_int(65279).unwrap()}[package]\nname = \"app\"\nversion = \"1.2.3-alpha.1\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"1.0.0-x-y.0.a\"\nedition = \"2026\"\n",
+        // Each number at most `u64::MAX`, a pre-release's unbounded, as
+        // semver reads them; a name of 64 characters.
+        "[package]\nname = \"app\"\nversion = \"18446744073709551615.18446744073709551615.18446744073709551615\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"1.0.0-99999999999999999999\"\nedition = \"2026\"\n",
+        manifest_of("a123456789012345678901234567890123456789012345678901234567890123", ""),
+        // A version entry in `[dev-dependencies]`, which nova-pm reads and,
+        // in a dependency's manifest, never resolves.
+        manifest_of(
+            "app",
+            "[dev-dependencies]\na = \"1\"\nb = '^1.2'\nc = { version = \"~0.1.2\" }\nd = \"=18446744073709551615\"\n",
+        ),
+        "dev-dependencies = { a = \"1.0\", b = { version = \"2\" } }\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+    ]
+    for text in accepted.iter() {
+        match parse_manifest(text) {
+            Ok(_) => ()
+            Err(why) => assert(false, "accepted: ${text}: ${why}")
+        }
+    }
+    let refused = [
+        "",
+        manifest_of("app", "[workspace]\n"),
+        manifest_of("app", "[[bin]]\n"),
+        manifest_of("app", "readme = \"r\"\n"),
+        manifest_of("app", "\"license\" = \"MIT\"\n"),
+        manifest_of("app", "package.license = \"MIT\"\n"),
+        manifest_of("app", "license = 7\n"),
+        manifest_of("app", "license = true\n"),
+        manifest_of("app", "authors = [\n\"a\"]\n"),
+        manifest_of("app", "description = '''x'''\n"),
+        manifest_of("app", "[dependencies]\ngeom = { path = \"g\" }\ngeom = { path = \"g\" }\n"),
+        manifest_of("app", "[dependencies]\n[dependencies]\n"),
+        manifest_of("app", "[dependencies]\ngeom = { path = \"g\", features = [] }\n"),
+        manifest_of("app", "[dependencies]\ngeom = {}\n"),
+        manifest_of("app", "[dependencies]\ngeom = { path = 1 }\n"),
+        "[package]\nname = \"app\"\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n[package]\n",
+        "[package]\nname = \"app\"\nversion = \"1.0.0-01\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"1.0.0-a..b\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"1.0.0+\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"1.0.0+build\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"1.0\"\nedition = \"2026\"\n",
+        "dependencies = {}\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n[dependencies]\n",
+        "dependencies = { g = { path = \"g\" }, g = { path = \"g\" } }\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+        "dependencies = { g = \"1.0\" }\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+        "[package]\nname = \"a\\tp\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+        manifest_of("dom", ""),
+        manifest_of("a.b", ""),
+        manifest_of("aux", ""),
+        manifest_of("a12345678901234567890123456789012345678901234567890123456789012345", ""),
+        // A carriage return not before a line feed, as the last byte or
+        // in a comment, and another control character.
+        "[package]\r\nname = \"app\"\r\nversion = \"0.1.0\"\r\nedition = \"2026\"\r",
+        manifest_of("app", "# a\rb\n"),
+        manifest_of("app", "# a\0b\n"),
+        manifest_of("app", "# a${char_from_int(127).unwrap()}b\n"),
+        // A name of 65 characters; four numbers; a number over `u64::MAX`,
+        // in each place.
+        manifest_of("a1234567890123456789012345678901234567890123456789012345678901234", ""),
+        "[package]\nname = \"app\"\nversion = \"1.0.0.0\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"18446744073709551616.0.0\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"0.18446744073709551616.0\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"0.0.18446744073709551616\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"99999999999999999999.0.0\"\nedition = \"2026\"\n",
+        "[package]\nname = \"app\"\nversion = \"0.0.100000000000000000000\"\nedition = \"2026\"\n",
+        // A version entry in `[dependencies]`, or one beside a path, or a
+        // requirement outside the reader's.
+        manifest_of("app", "[dependencies]\na = \"1\"\n"),
+        manifest_of("app", "[dependencies]\na = { version = \"1\" }\n"),
+        manifest_of("app", "[dev-dependencies]\na = { version = \"1\", path = \"a\" }\n"),
+        manifest_of("app", "[dev-dependencies]\na = \"01\"\n"),
+        manifest_of("app", "[dev-dependencies]\na = \"1.2.3.4\"\n"),
+        manifest_of("app", "[dev-dependencies]\na = \"18446744073709551616\"\n"),
+        manifest_of("app", "[dev-dependencies]\na = \"\"\n"),
+        manifest_of("app", "[dev-dependencies]\na = \"*\"\n"),
+        manifest_of("app", "[dev-dependencies]\na = \">=1\"\n"),
+        manifest_of("app", "[dev-dependencies]\na = []\n"),
+    ]
+    for text in refused.iter() {
+        match parse_manifest(text) {
+            Ok(_) => assert(false, "refused: ${text}")
+            Err(_) => ()
+        }
+    }
+    // Some refusals' words, as N's refusal line prints them: a check
+    // another would back up still says what it refuses.
+    let worded = [
+        ("", "there is no `[package]` table"),
+        ("[package]\nname = \"app\"\nedition = \"2026\"\n", "`[package]` has no `version`"),
+        (
+            "x = { path = \"g\" }\n[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+            "the key `x` is outside every table",
+        ),
+    ]
+    for (text, message) in worded.iter() {
+        match parse_manifest(text) {
+            Ok(_) => assert(false, "refused: ${text}")
+            Err(why) => assert_eq(why, message)
+        }
+    }
+}
+
+@test
+fn the_graph_refuses_what_nova_pm_gives_an_m_code() {
+    // Each would be an M-code in S0's graph, which N does not give, so
+    // N's graph refuses it, saying which: a dependency's directory with no
+    // manifest (M0007), an entry whose key is not the package's name
+    // (M0008), a dependency with no library (M0009), a cycle and a
+    // dependency on the package itself (M0010), and an import name taken
+    // twice (M0012); and as the reader would refuse it: a dependency's
+    // version over `u64::MAX` (M0003), and a version entry among the
+    // root's dev-dependencies, which needs the registry.
+    let dir = "${temp_dir()}/nova-selfhost-graph-test"
+    let cases = [
+        ("missing", "[dependencies]\ngeom = { path = \"nowhere\" }\n", "(M0007)"),
+        ("renamed", "[dependencies]\nother = { path = \"geom\" }\n", "(M0008)"),
+        ("nolib", "[dependencies]\nbare = { path = \"bare\" }\n", "(M0009)"),
+        ("cycle", "[dependencies]\nloop = { path = \"loop\" }\n", "a dependency cycle"),
+        ("self", "[dependencies]\nme = { path = \".\" }\n", "is its own package (M0010)"),
+        (
+            "twice",
+            "[dependencies]\ngeom = { path = \"geom\" }\n[dev-dependencies]\ngeom = { path = \"geom\" }\n",
+            "two entries are imported as `geom` (M0012)",
+        ),
+        (
+            "huge",
+            "[dependencies]\nhuge = { path = \"huge\" }\n",
+            "huge/nova.toml: `0.0.18446744073709551616` is not a version this reader takes",
+        ),
+        (
+            "registry",
+            "[dev-dependencies]\njson = \"1.0\"\n",
+            "dev-dependency `json` is a version entry, which needs the registry",
+        ),
+    ]
+    for (name, deps, why) in cases.iter() {
+        let root = "${dir}/${name}"
+        write_tree(
+            root,
+            [
+                ("nova.toml", manifest_of("app", deps)),
+                ("src/lib.nova", "pub fn a() {}\n"),
+                ("geom/nova.toml", manifest_of("geom", "")),
+                ("geom/src/lib.nova", "pub fn g() {}\n"),
+                ("bare/nova.toml", manifest_of("bare", "")),
+                ("bare/src/main.nova", "fn main() {}\n"),
+                (
+                    "loop/nova.toml",
+                    manifest_of("loop", "[dependencies]\nback = { path = \"back\" }\n"),
+                ),
+                ("loop/src/lib.nova", "pub fn l() {}\n"),
+                (
+                    "loop/back/nova.toml",
+                    manifest_of("back", "[dependencies]\nloop = { path = \"..\" }\n"),
+                ),
+                ("loop/back/src/lib.nova", "pub fn b() {}\n"),
+                (
+                    "huge/nova.toml",
+                    "[package]\nname = \"huge\"\nversion = \"0.0.18446744073709551616\"\nedition = \"2026\"\n",
+                ),
+                ("huge/src/lib.nova", "pub fn h() {}\n"),
+            ],
+        )
+        match block_on(read_graph(root)) {
+            Ok(_) => assert(false, "refused: ${name}")
+            Err(message) => assert(message.contains(why), "${name}: ${message}")
+        }
+    }
+    // The root package alone: neither src/lib.nova nor src/main.nova
+    // (M0013), and a library imported as a keyword (M0012).
+    let alone = [
+        ("noentry", "app", "src/other.nova", "has neither src/lib.nova nor src/main.nova (M0013)"),
+        ("keyword", "fn", "src/lib.nova", "would be imported as a keyword (M0012)"),
+    ]
+    for (name, package, file, why) in alone.iter() {
+        let root = "${dir}/${name}"
+        write_tree(root, [("nova.toml", manifest_of(package, "")), (file, "pub fn a() {}\n")])
+        match block_on(read_graph(root)) {
+            Ok(_) => assert(false, "refused: ${name}")
+            Err(message) => assert(message.contains(why), "${name}: ${message}")
+        }
+    }
+    // A dependency's own version entries, which nova-pm reads and never
+    // resolves, are read and left alone.
+    let root = "${dir}/devversions"
+    write_tree(
+        root,
+        [
+            ("nova.toml", manifest_of("app", "[dependencies]\ngeom = { path = \"geom\" }\n")),
+            ("src/main.nova", "fn main() {}\n"),
+            ("geom/nova.toml", manifest_of("geom", "[dev-dependencies]\njson = \"1.0\"\n")),
+            ("geom/src/lib.nova", "pub fn g() {}\n"),
+        ],
+    )
+    match block_on(read_graph(root)) {
+        Ok(g) => assert_eq(g.packages.len(), 2)
+        Err(message) => assert(false, "devversions: ${message}")
+    }
+    let _ = block_on(remove_dir_all(dir))
+}
+
+@test
+fn a_root_is_a_packages_module_directly_in_its_src_or_tests() {
+    let dir = "${temp_dir()}/nova-selfhost-package-of-test"
+    write_tree(dir, [("nova.toml", manifest_of("app", "")), ("src/main.nova", "fn main() {}\n")])
+    match block_on(package_of("${dir}/src/main.nova")) {
+        Some((root, RootSrc)) => assert_eq(root, dir)
+        _ => assert(false, "unreachable")
+    }
+    match block_on(package_of("${dir}/tests/x.nova")) {
+        Some((_, RootTests)) => ()
+        _ => assert(false, "unreachable")
+    }
+    assert(block_on(package_of("${dir}/src/sub/x.nova")).is_none(), "not directly in src/")
+    assert(block_on(package_of("${dir}/main.nova")).is_none(), "not in src/")
+    let _ = block_on(remove_dir_all(dir))
+}
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/selfhost/src/loader.nova b/selfhost/src/loader.nova
index 033d7a24eacc359102558f25aa3febdb4a86d608..ca590cdfbc53f5b646ad56c4155a501b5beaffa0 100644
--- a/selfhost/src/loader.nova
+++ b/selfhost/src/loader.nova
@@ -1,39 +1,68 @@
-// The loader of a loose program: a port of
-// crates/nova-driver/src/program.rs's `load_program` and `Loader` on the
-// loose path, and of the driver's `strip_test_functions` and `package_of`
-// (spec
+// The loader: a port of crates/nova-driver/src/program.rs's
+// `load_program` and `Loader`, of nova-pm's package graph as far as the
+// loader reads it, and of the driver's `strip_test_functions`, `name_tests`
+// and `package_of` (spec
 // docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
-// §3.2, ADR 0053). From the root it reads, lexes and parses every module
-// it reaches, breadth first, finding each single-segment import as a file
-// beside the root; the order of the modules is the order of that search,
-// and each module's label is its file's stem, as in S0. Packages
-// (`nova.toml`, `src/`, `tests/`, a dependency) are 5.2b's.
+// §3.2 and §4.1, ADRs 0053 and 0054). From the root it reads, lexes and
+// parses every module it reaches, breadth first: a loose program's
+// imports are files beside the root; a package's module's are files beside
+// it in its package's `src/` or `tests/`, or its package's dependencies'
+// `src/lib.nova`, which its `nova.toml` names (manifest.nova reads it).
+// The order of the modules is the order of that search, and each module's
+// label is S0's: its file's stem, `tests/<stem>` in the root package's
+// `tests/`, or a dependency's import name for its library and
+// `<import name>/<stem>` for its other modules.
 //
 // The Rust's idioms, translated:
 //
-// - A loose `Place` is a directory and a stem, and every module of a loose
-//   program is in the root's directory, so a place is its stem.
+// - A `Place` is a package (-1 for a loose file), whether it is in its
+//   package's `tests/`, and a stem; every module of a loose program is in
+//   the root's directory, so a loose place needs no directory. It is
+//   kept in maps by its `key`.
 // - `VecDeque` is a `Vec` and the index of its head.
 // - A path is a `String` with `/` between its parts, as `Path::join` and
 //   `Path::parent` treat one on Linux and macOS: `Path::join` writes `\`
-//   on Windows, which the harness keeps out of what it compares (ADR 0053
-//   §5).
+//   on Windows, where the harness reads `\` as `/` in the two messages
+//   that print a path, E0004's and E0093's (ADR 0054).
+// - A package's identity, `real_path` of its directory in nova-pm, is its
+//   directory as the root's path and the entries' paths spell it, with
+//   `.` and `x/..` folded away (`normal_path`). Neither N's std nor its
+//   runtime names the current directory or follows a link, so this is
+//   `real_path`'s identity except on two kinds of input: any path through
+//   a link, whatever the root, where `x/..` after a link `x` is folded
+//   where `real_path` follows the link; and, unless the root is given by
+//   an absolute path, an absolute dependency path or one whose folded
+//   form climbs above the current directory (starts with `..`), which N
+//   cannot compare with the other paths without the current directory's
+//   name. There one directory spelled two ways is two packages, which N
+//   refuses as M0011, saying, where the paths are of the second kind,
+//   that they may be one directory. No input of the differential holds a
+//   link or, under a relative root, an absolute path or one that climbs
+//   above the current directory (ADR 0054 §5).
+// - The graph's diagnostics, the M-codes, are S0's alone (ADR 0054 §5):
+//   where nova-pm would give one, or where the manifest reader refuses a
+//   manifest, `read_graph` is `Err`, and the driver prints the message and
+//   exits non-zero.
 
 import ast
 import builtins
 import diagnostic
 import lexer
+import manifest
 import parser
 import token
 
 /// One module the loader read (`Loaded`): its label, path, tree and
-/// source, what each of its imports names by first segment, and what it
-/// could import.
+/// source, its package (-1 for a loose module), whether its tests are
+/// named for its file (a module of a package's `tests/`), what each of
+/// its imports names by first segment, and what it could import.
 pub record Loaded {
     pub name: String,
     pub path: String,
     pub ast: File,
     pub text: String,
+    pub package: Int,
+    pub test_file: Bool,
     pub imports: Map<String, ImportTarget>,
     pub importable: Vec<(String, Int)>,
 }
@@ -47,36 +76,290 @@ pub record Load { pub modules: Vec<Loaded>, pub diagnostics: Vec<Diagnostic> }
 /// decides it, so the loader builds without the resolver.
 pub type ImportTarget = | ImportTargetModule(Int) | ImportTargetReported
 
+/// Where a root is (`RootKind`): directly in its package's `src/` or
+/// `tests/`, or in no package.
+pub type RootKind = | RootSrc | RootTests | RootLoose
+
+/// A dependency that resolved (nova-pm's `Edge`): the name its dependent
+/// imports it by, its package, the entry in the declaring manifest, and
+/// the declaring package, whose manifest that span is in.
+pub record Edge { pub import_name: String, pub package: Int, pub span: Span, pub declarer: Int }
+
+/// One package of a `Graph` (nova-pm's `GraphPackage`): its name and
+/// import name, its directory as the root's path and the entries spell
+/// it, its identity, its manifest (its path, its label in the dump, its
+/// text and where `[package]` is in it), whether it has a library, what
+/// the manifest reader read of it, and its edges; the dev-dependencies are
+/// read for the root alone.
+pub record GraphPackage {
+    pub name: String,
+    pub import_name: String,
+    pub dir: String,
+    pub canonical: String,
+    pub manifest_path: String,
+    pub label: String,
+    pub text: String,
+    pub span: Span,
+    pub has_lib: Bool,
+    pub manifest: Manifest,
+    pub dependencies: Vec<Edge>,
+    pub dev_dependencies: Vec<Edge>,
+}
+
+/// The root package, and every package its dependencies reach, the root
+/// first (nova-pm's `Graph`).
+pub record Graph { pub packages: Vec<GraphPackage> }
+
+// A module's identity (`Place`): its package, -1 for a loose file;
+// whether it is in its package's `tests/`; and its stem.
+record Place { package: Int, tests: Bool, stem: String }
+
 // What an import names before every module is loaded (`Pending`).
-type Pending = | PendingPlace(String) | PendingReported
+type Pending = | PendingPlace(Place) | PendingReported
 
 // What `find` found for one import (`Found`): a module, by its place and
 // path; an error, reported here; or nothing, which the resolver reports.
-type Found = | FoundModule(String, String) | FoundReported(Diagnostic) | FoundNothing
+type Found = | FoundModule(Place, String) | FoundReported(Diagnostic) | FoundNothing
+
+// The loader's state (`Loader`): the graph, if the program is a package's;
+// the loose root's directory; each directory's listing, read once; and
+// each `has` answered, by directory and stem.
+record Loader {
+    graph: Option<Graph>,
+    dir: String,
+    listings: Map<String, Vec<String>>,
+    known: Map<String, Bool>,
+}
+
+// ---------------------------------------------------------------------------
+// The package graph
+// ---------------------------------------------------------------------------
+
+/// Read the package whose `nova.toml` is in `root`, "" for the current
+/// directory, and every package its path dependencies reach, breadth
+/// first (nova-pm's `graph`, and its `check_root`, `expand`, `edge`,
+/// `admit` and `cycles`). `Err`, saying why, wherever nova-pm would give
+/// an M-code or the manifest reader refuses a manifest.
+pub async fn read_graph(root: String) -> Result<Graph, String> {
+    let first = read_package(root, true).await?
+    if !first.has_lib && !is_file(path_join(path_join(root, "src"), "main.nova")).await {
+        return Err("package `${first.name}` has neither src/lib.nova nor src/main.nova (M0013)")
+    }
+    if first.has_lib && is_keyword(first.import_name) {
+        return Err("library `${first.name}` would be imported as a keyword (M0012)")
+    }
+    let mut graph = Graph { packages: Vec::new() }
+    graph.packages.push(first)
+    let mut id = 0
+    while id < graph.packages.len() {
+        expand(graph, id).await?
+        id += 1
+    }
+    // A cycle of `[dependencies]` edges (M0010), found depth first from
+    // every package: 0 not visited, 1 on the path, 2 done.
+    let mut state: Vec<Int> = Vec::new()
+    for _ in graph.packages.iter() { state.push(0) }
+    id = 0
+    while id < graph.packages.len() {
+        if state.at(id) == 0 { visit(graph, id, state)? }
+        id += 1
+    }
+    Ok(graph)
+}
+
+// The package whose manifest is `dir/nova.toml`, with no edges yet
+// (nova-pm's `read`), its manifest labelled as the dump labels it:
+// `nova.toml` for the root, `<import name>/nova.toml` for a dependency.
+async fn read_package(dir: String, root: Bool) -> Result<GraphPackage, String> {
+    let path = path_join(dir, "nova.toml")
+    let text = match read_to_string(path).await {
+        Ok(t) => t
+        Err(_) => return Err("cannot read ${path} (M0001)")
+    }
+    let m = match parse_manifest(text) {
+        Ok(m) => m
+        Err(why) => return Err("${path}: ${why}")
+    }
+    let import_as = import_name(m.name)
+    Ok(
+        GraphPackage {
+            name: m.name,
+            import_name: import_as,
+            dir: dir,
+            canonical: normal_path(if dir == "" { "." } else { dir }),
+            manifest_path: path,
+            label: if root { "nova.toml" } else { "${import_as}/nova.toml" },
+            text: text,
+            span: m.span,
+            has_lib: is_file(path_join(path_join(dir, "src"), "lib.nova")).await,
+            manifest: m,
+            dependencies: Vec::new(),
+            dev_dependencies: Vec::new(),
+        },
+    )
+}
+
+// Resolve package `id`'s entries, and for the root its dev-dependencies,
+// reading each package named for the first time and adding it to the
+// graph, whose loop then expands it (nova-pm's `expand`, `names_ok`,
+// `edge` and `admit`).
+async fn expand(mut graph: Graph, id: Int) -> Result<(), String> {
+    let mut p = graph.packages.at(id)
+    let mut entries: Vec<(Dependency, Bool)> = Vec::new()
+    for d in p.manifest.dependencies.iter() { entries.push((d, false)) }
+    if id == 0 { for d in p.manifest.dev_dependencies.iter() { entries.push((d, true)) } }
+    // A version entry needs the registry, which N does not read; the
+    // reader takes one in `[dev-dependencies]` alone, which nova-pm never
+    // resolves but in the root's manifest.
+    for (d, _) in entries.iter() {
+        if d.version != "" {
+            return Err(
+                "${p.manifest_path}: dev-dependency `${d.name}` is a version entry, which needs the registry",
+            )
+        }
+    }
+    // M0012: the entries' import names, together.
+    let mut seen: Vec<String> = Vec::new()
+    for (d, _) in entries.iter() {
+        let name = import_name(d.name)
+        if is_keyword(name) {
+            return Err("dependency `${d.name}` would be imported as a keyword (M0012)")
+        }
+        for s in seen.iter() {
+            if s == name { return Err("two entries are imported as `${name}` (M0012)") }
+        }
+        if id == 0 && name == p.import_name {
+            return Err(
+                "dependency `${d.name}` would be imported as this package's own name (M0012)",
+            )
+        }
+        seen.push(name)
+    }
+    for (d, dev) in entries.iter() {
+        let dir = path_join(p.dir, d.path)
+        if !is_file(path_join(dir, "nova.toml")).await {
+            return Err(
+                "dependency `${d.name}`: ${dir} is not a directory holding a nova.toml (M0007)",
+            )
+        }
+        let canonical = normal_path(dir)
+        if canonical == p.canonical {
+            return Err("dependency `${d.name}` is its own package (M0010)")
+        }
+        let mut target = -1
+        let mut i = 0
+        while i < graph.packages.len() {
+            if graph.packages.at(i).canonical == canonical { target = i }
+            i += 1
+        }
+        if target >= 0 {
+            let found = graph.packages.at(target)
+            if found.name != d.name {
+                return Err("dependency `${d.name}` is the package `${found.name}` (M0008)")
+            }
+            if !found.has_lib { return Err("dependency `${d.name}` is not a library (M0009)") }
+        } else {
+            let new = read_package(dir, false).await?
+            if new.name != d.name {
+                return Err("dependency `${d.name}` is the package `${new.name}` (M0008)")
+            }
+            if !new.has_lib { return Err("dependency `${d.name}` is not a library (M0009)") }
+            for other in graph.packages.iter() {
+                if other.name == new.name {
+                    // Under a relative root, an absolute path and a
+                    // relative one may name one directory, and so may a
+                    // path that climbs above the current directory and
+                    // one that does not.
+                    let mixed = other.canonical.starts_with("/") != new.canonical.starts_with("/")
+                    let climbs = climbs_above(other.canonical) || climbs_above(new.canonical)
+                    let or_one = if mixed || climbs {
+                        ", or one directory spelled by two paths, which N cannot tell apart without the current directory"
+                    } else {
+                        ""
+                    }
+                    return Err("two packages named `${new.name}` in one build (M0011)${or_one}")
+                }
+            }
+            target = graph.packages.len()
+            graph.packages.push(new)
+        }
+        let edge = Edge {
+            import_name: import_name(d.name),
+            package: target,
+            span: d.span,
+            declarer: id,
+        }
+        if dev { p.dev_dependencies.push(edge) } else { p.dependencies.push(edge) }
+    }
+    Ok(())
+}
+
+// Visit package `id` depth first along its `[dependencies]` edges: `Err`
+// on an edge back to a package on the path (nova-pm's `visit`, M0010).
+fn visit(graph: Graph, id: Int, mut state: Vec<Int>) -> Result<(), String> {
+    state.set(id, 1)
+    for edge in graph.packages.at(id).dependencies.iter() {
+        let s = state.at(edge.package)
+        if s == 1 {
+            return Err("a dependency cycle through `${graph.packages.at(id).name}` (M0010)")
+        }
+        if s == 0 { visit(graph, edge.package, state)? }
+    }
+    state.set(id, 2)
+    Ok(())
+}
+
+// Whether `path`, folded by `normal_path`, climbs above the directory it
+// is relative to, where folding cannot tell whether it comes back down
+// into a directory another path names.
+fn climbs_above(path: String) -> Bool { path == ".." || path.starts_with("../") }
 
-// The loader's state (`Loader`): the root's directory, and each stem's
-// answer to `has`, from one listing of the directory read once.
-record Loader { dir: String, listing: Vec<String>, listed: Bool, known: Map<String, Bool> }
+// `path` with its `.` components dropped and each `x/..` folded, where
+// `x` is not `..`, as `real_path` names a directory when no link is on the
+// way; a leading `/` stays.
+fn normal_path(path: String) -> String {
+    let absolute = path.starts_with("/")
+    let mut parts: Vec<String> = Vec::new()
+    for part in path.split("/").iter() {
+        if part == "" || part == "." { continue }
+        if part == ".." && !parts.is_empty() && parts.last() != ".." {
+            let _ = parts.pop()
+            continue
+        }
+        parts.push(part)
+    }
+    let joined = "/".join(parts.to_array())
+    if absolute { "/${joined}" } else { joined }
+}
 
-/// Read, lex and parse every module the loose program whose root is
-/// `root` reaches, breadth first (`load_program`). A module that cannot be
-/// read is skipped, so an import of it is "cannot find module"; `Err` only
-/// when the root cannot be read.
-pub async fn load_program(root: String) -> Result<Load, IoError> {
+// ---------------------------------------------------------------------------
+// The loader
+// ---------------------------------------------------------------------------
+
+/// Read, lex and parse every module the program whose root is `root`, of
+/// kind `kind`, reaches, breadth first (`load_program`), with `graph` when
+/// the root is a package's module. A module that cannot be read is skipped,
+/// so an import of it is "cannot find module"; `Err` only when the root
+/// cannot be read.
+pub async fn load_program(
+    graph: Option<Graph>,
+    root: String,
+    kind: RootKind,
+) -> Result<Load, IoError> {
     let mut loader = Loader {
+        graph: graph,
         dir: path_parent_or_empty(root),
-        listing: Vec::new(),
-        listed: false,
+        listings: Map::new(),
         known: Map::new(),
     }
     let mut modules: Vec<Loaded> = Vec::new()
     let mut diagnostics: Vec<Diagnostic> = Vec::new()
     let mut pending: Vec<Vec<(String, Pending)>> = Vec::new()
     let mut places: Map<String, Int> = Map::new()
-    let mut place_order: Vec<String> = Vec::new()
+    let mut place_order: Vec<Place> = Vec::new()
     let mut tried: Set<String> = Set::new()
-    let mut queue: Vec<(String, String)> = Vec::new()
-    queue.push((module_name(root), root))
+    let mut queue: Vec<(Place, String)> = Vec::new()
+    queue.push((loader.root_place(root, kind), root))
     let mut head = 0
     let mut is_entry = true
 
@@ -85,7 +368,7 @@ pub async fn load_program(root: String) -> Result<Load, IoError> {
         head += 1
         let entry = is_entry
         is_entry = false
-        if !tried.insert(place) { continue }
+        if !tried.insert(place_key(place)) { continue }
         let source = match read_to_string(path).await {
             Ok(s) => s
             Err(e) => {
@@ -93,7 +376,7 @@ pub async fn load_program(root: String) -> Result<Load, IoError> {
                 continue
             }
         }
-        let label = place
+        let label = loader.label(place)
         let (tokens, lex_errors) = lex(source)
         for e in lex_errors.iter() {
             diagnostics.push(error_at("L0001", "${e}", label, e.span(), "here"))
@@ -128,7 +411,7 @@ pub async fn load_program(root: String) -> Result<Load, IoError> {
                 _ => ()
             }
         }
-        let _ = places.insert(place, modules.len())
+        let _ = places.insert(place_key(place), modules.len())
         place_order.push(place)
         modules.push(
             Loaded {
@@ -136,6 +419,8 @@ pub async fn load_program(root: String) -> Result<Load, IoError> {
                 path: path,
                 ast: file,
                 text: source,
+                package: place.package,
+                test_file: place.tests,
                 imports: Map::new(),
                 importable: Vec::new(),
             },
@@ -149,7 +434,7 @@ pub async fn load_program(root: String) -> Result<Load, IoError> {
         for (name, target) in pending.at(i).iter() {
             match target {
                 PendingPlace(place) => {
-                    match places.get(place) {
+                    match places.get(place_key(place)) {
                         Some(index) => { let _ = m.imports.insert(name, ImportTargetModule(index)) }
                         // It could not be read.
                         None => ()
@@ -162,18 +447,30 @@ pub async fn load_program(root: String) -> Result<Load, IoError> {
     }
 
     // What each loaded module could import, by the name an import of it
-    // writes, through `find` itself: every module beside it, by stem.
-    let mut names: Vec<String> = Vec::new()
-    for p in place_order.iter() { names.push(p) }
-    names = sorted_dedup(names)
+    // writes, through `find` itself: every module beside it, by stem, and
+    // each dependency it sees, by its import name.
     i = 0
     while i < modules.len() {
         let from = place_order.at(i)
+        let mut names: Vec<String> = Vec::new()
+        for p in place_order.iter() {
+            if p.package == from.package && p.tests == from.tests { names.push(p.stem) }
+        }
+        match loader.graph {
+            Some(graph) => {
+                if from.package >= 0 {
+                    for e in visible_edges(graph, from.package, from.tests).iter() {
+                        names.push(e.import_name)
+                    }
+                }
+            }
+            None => ()
+        }
         let mut importable: Vec<(String, Int)> = Vec::new()
-        for name in names.iter() {
+        for name in sorted_dedup(names).iter() {
             match loader.find(from, name, Span::point(0)).await {
                 FoundModule(target, _) => {
-                    match places.get(target) {
+                    match places.get(place_key(target)) {
                         Some(j) => { if j != i { importable.push((name, j)) } }
                         None => ()
                     }
@@ -188,12 +485,36 @@ pub async fn load_program(root: String) -> Result<Load, IoError> {
     Ok(Load { modules: modules, diagnostics: diagnostics })
 }
 
+// A place's key in the loader's maps.
+fn place_key(p: Place) -> String { "${p.package} ${p.tests} ${p.stem}" }
+
 impl Loader {
+    // The root's place (`root_place`): a module of the root package for a
+    // root in its `src/` or `tests/`, else a loose file.
+    fn root_place(self, root: String, kind: RootKind) -> Place {
+        let stem = module_name(root)
+        match kind {
+            RootSrc => Place { package: 0, tests: false, stem: stem }
+            RootTests => Place { package: 0, tests: true, stem: stem }
+            RootLoose => Place { package: -1, tests: false, stem: stem }
+        }
+    }
+
+    // A module's label (`label`).
+    fn label(self, place: Place) -> String {
+        if place.package < 0 { return place.stem }
+        if place.package == 0 {
+            return if place.tests { "tests/${place.stem}" } else { place.stem }
+        }
+        let import_as = self.graph.unwrap().packages.at(place.package).import_name
+        if place.stem == "lib" { import_as } else { "${import_as}/${place.stem}" }
+    }
+
     // What `import name`, at `at` in the module `from`, names (`find`): a
-    // module under a std module's name is E0093; a std module's name that
-    // names no file is left to the resolver.
-    async fn find(mut self, from: String, name: String, at: Span) -> Found {
-        let found = self.find_place(from, name).await
+    // module or a dependency under a std module's name is E0093; a std
+    // module's name that names neither is left to the resolver.
+    async fn find(mut self, from: Place, name: String, at: Span) -> Found {
+        let found = self.find_place(from, name, at).await
         if !holds(RESERVED_MODULE_NAMES, name) { return found }
         let what = match found {
             FoundModule(_, path) => path
@@ -204,59 +525,180 @@ impl Loader {
             error_at(
                 "E0093",
                 "`${name}` is the name of a std module, so no module may take it: rename ${what}",
-                from,
+                self.label(from),
                 at,
                 "std's module, not this one",
             ),
         )
     }
 
-    // `find`, before std's names are checked (`find_place`): on the loose
-    // path, the file `name.nova` beside the root, or nothing.
-    async fn find_place(mut self, from: String, name: String) -> Found {
-        if !self.has(name).await { return FoundNothing }
-        FoundModule(name, path_join(self.dir, "${name}.nova"))
+    // `find`, before std's names are checked (`find_place`): for a loose
+    // module, the file `name.nova` beside the root, or nothing; for a
+    // package's module, a file beside it in its package's `src/` or
+    // `tests/`, or a dependency it sees, E0004 for both, E0001 with a note
+    // for its own package with no library or for a dev-dependency seen
+    // from `src/`.
+    async fn find_place(mut self, from: Place, name: String, at: Span) -> Found {
+        if from.package < 0 {
+            if !self.has(self.dir, name).await { return FoundNothing }
+            return FoundModule(
+                Place { package: -1, tests: false, stem: name },
+                path_join(self.dir, "${name}.nova"),
+            )
+        }
+        let graph = self.graph.unwrap()
+        let dir = module_dir(graph.packages.at(from.package), from.tests)
+        let mut edge: Option<Edge> = None
+        for e in visible_edges(graph, from.package, from.tests).iter() {
+            if edge.is_none() && e.import_name == name { edge = Some(e) }
+        }
+        let path = path_join(dir, "${name}.nova")
+        let file = self.has(dir, name).await
+        match edge {
+            Some(e) => {
+                if file { return FoundReported(clash(graph, name, path, e, self.label(from), at)) }
+                let target = graph.packages.at(e.package)
+                if !target.has_lib {
+                    // The package itself, seen from its own tests.
+                    let mut d = error_at(
+                        "E0001",
+                        "cannot find module `${name}`",
+                        self.label(from),
+                        at,
+                        "no such module",
+                    )
+                    d.add_note("`${target.name}` has no src/lib.nova")
+                    return FoundReported(d)
+                }
+                FoundModule(
+                    Place { package: e.package, tests: false, stem: "lib" },
+                    path_join(module_dir(target, false), "lib.nova"),
+                )
+            }
+            None => {
+                if file {
+                    return FoundModule(
+                        Place { package: from.package, tests: from.tests, stem: name },
+                        path,
+                    )
+                }
+                let mut dev = false
+                if !from.tests && from.package == 0 {
+                    for e in graph.packages.at(0).dev_dependencies.iter() {
+                        if e.import_name == name { dev = true }
+                    }
+                }
+                if !dev { return FoundNothing }
+                let mut d = error_at(
+                    "E0001",
+                    "cannot find module `${name}`",
+                    self.label(from),
+                    at,
+                    "no such module",
+                )
+                d.add_note("`${name}` is a dev-dependency, which only `tests/` files can import")
+                FoundReported(d)
+            }
+        }
     }
 
-    // Whether the root's directory holds `<stem>.nova` (`has`, `look`),
-    // answered once per load: its name exactly in the directory's listing,
-    // read once; not if only a name differing in case is; otherwise
-    // whether a file of that path reads.
-    async fn has(mut self, stem: String) -> Bool {
-        match self.known.get(stem) {
+    // Whether `dir` holds `<stem>.nova` (`has`, `look`), answered once per
+    // load: its name exactly in the directory's listing, read once; not if
+    // only a name differing in case is; otherwise whether a file of that
+    // path reads. An empty `dir` is the current directory.
+    async fn has(mut self, dir: String, stem: String) -> Bool {
+        let key = "${dir}\n${stem}"
+        match self.known.get(key) {
             Some(known) => return known
             None => ()
         }
-        if !self.listed {
-            let listed = if self.dir == "" { "." } else { self.dir }
-            match read_dir(listed).await {
-                Ok(entries) => { for e in entries.iter() { self.listing.push(e.name) } }
-                Err(_) => ()
+        let listing = match self.listings.get(dir) {
+            Some(l) => l
+            None => {
+                let mut l: Vec<String> = Vec::new()
+                let listed = if dir == "" { "." } else { dir }
+                match read_dir(listed).await {
+                    Ok(entries) => { for e in entries.iter() { l.push(e.name) } }
+                    Err(_) => ()
+                }
+                let _ = self.listings.insert(dir, l)
+                l
             }
-            self.listed = true
         }
         let name = "${stem}.nova"
         let mut found = false
         let mut folded = false
-        for n in self.listing.iter() {
+        for n in listing.iter() {
             if n == name { found = true }
             if eq_ignore_ascii_case(n, name) { folded = true }
         }
         if !found && !folded {
-            found = match read_to_string(path_join(self.dir, name)).await {
+            found = match read_to_string(path_join(dir, name)).await {
                 Ok(_) => true
                 Err(_) => false
             }
         }
-        let _ = self.known.insert(stem, found)
+        let _ = self.known.insert(key, found)
         found
     }
 }
 
-/// Remove every `@test` function, as the driver does before resolving
-/// under `check` (`strip_test_functions` with `keep_root_tests` false).
-pub fn strip_test_functions(modules: Vec<Loaded>) {
+// A package's `src/`, or `tests/` when `tests` (`module_dir`).
+fn module_dir(p: GraphPackage, tests: Bool) -> String {
+    path_join(p.dir, if tests { "tests" } else { "src" })
+}
+
+// The dependencies a module of package `id` sees (`visible_edges`): a test
+// module also sees the root's dev-dependencies, and the package itself by
+// its import name, its entry its `[package]` table.
+fn visible_edges(graph: Graph, id: Int, tests: Bool) -> Vec<Edge> {
+    let p = graph.packages.at(id)
+    let mut edges: Vec<Edge> = Vec::new()
+    for e in p.dependencies.iter() { edges.push(e) }
+    if tests {
+        for e in p.dev_dependencies.iter() { edges.push(e) }
+        edges.push(Edge { import_name: p.import_name, package: id, span: p.span, declarer: id })
+    }
+    edges
+}
+
+// E0004 (`clash`): `name` is both the file at `path` and the dependency
+// `edge`, imported at `at` in the module labelled `from`. The message
+// names the file and the entry's manifest and line, its secondary label
+// is the entry, in that manifest.
+fn clash(
+    graph: Graph,
+    name: String,
+    path: String,
+    edge: Edge,
+    from: String,
+    at: Span,
+) -> Diagnostic {
+    let declarer = graph.packages.at(edge.declarer)
+    let mut line = 1
+    let mut i = 0
+    while i < edge.span.start && i < declarer.text.byte_len() {
+        if declarer.text.byte_at(i) == 10 { line += 1 }
+        i += 1
+    }
+    let mut d = error_at(
+        "E0004",
+        "`${name}` is both a module of this package and a dependency: ${path}, and the entry at ${declarer.manifest_path}:${line}",
+        from,
+        at,
+        "this import is ambiguous",
+    )
+    d.add_secondary(declarer.label, edge.span, "the dependency")
+    d
+}
+
+/// Remove the `@test` functions before resolving (`strip_test_functions`):
+/// every one under `check` (`keep_root_tests` false); under `nova test`'s
+/// resolution, a dependency's, keeping the root package's and a loose
+/// program's.
+pub fn strip_test_functions(modules: Vec<Loaded>, keep_root_tests: Bool) {
     for loaded in modules.iter() {
+        let root = loaded.package <= 0
         let mut kept: Vec<Spanned<Item>> = Vec::new()
         for item in loaded.ast.items.iter() {
             let test = match item.value {
@@ -267,31 +709,38 @@ pub fn strip_test_functions(modules: Vec<Loaded>) {
                 }
                 _ => false
             }
-            if !test { kept.push(item) }
+            if !test || (keep_root_tests && root) { kept.push(item) }
         }
         let mut file = loaded.ast
         file.items = kept
     }
 }
 
-/// The package `path` is a module of, when it is directly in the `src/` or
-/// `tests/` of a directory holding a `nova.toml` (`package_of`): whether it
-/// is. The manifest is asked for by its path, as `package_of` asks
+/// The package `path` is a module of, and where in it, when `path` is
+/// directly in the `src/` or `tests/` of a directory holding a
+/// `nova.toml` (`package_of`): the package's directory and the root's
+/// kind. The manifest is asked for by its path, as `package_of` asks
 /// `root.join("nova.toml").is_file()`, not looked for in a listing, so a
 /// case-insensitive disk finds `Nova.toml` on both sides, and a directory
 /// that can be searched but not listed still holds its manifest.
-pub async fn in_package(path: String) -> Bool {
+pub async fn package_of(path: String) -> Option<(String, RootKind)> {
     let parent = match path_parent(path) {
         Some(p) => p
-        None => return false
+        None => return None
     }
     let dir_name = file_name(parent)
-    if dir_name != "src" && dir_name != "tests" { return false }
+    let kind = if dir_name == "src" {
+        RootSrc
+    } else if dir_name == "tests" {
+        RootTests
+    } else {
+        return None
+    }
     let root = match path_parent(parent) {
         Some(r) => r
-        None => return false
+        None => return None
     }
-    is_file(path_join(root, "nova.toml")).await
+    if is_file(path_join(root, "nova.toml")).await { Some((root, kind)) } else { None }
 }
 
 // Whether something that is not a directory is at `path`, a link followed
@@ -392,9 +841,10 @@ fn path_parent_or_empty(path: String) -> String {
 }
 
 /// `dir` joined with `name` (`Path::join`): `name` alone for an empty
-/// `dir`, and no second `/` after one `dir` ends with.
+/// `dir` or an absolute `name`, and no second `/` after one `dir` ends
+/// with.
 pub fn path_join(dir: String, name: String) -> String {
-    if dir == "" { return name }
+    if dir == "" || name.starts_with("/") { return name }
     if dir.ends_with("/") { "${dir}${name}" } else { "${dir}/${name}" }
 }
 
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index fe9c3bb965eed0038e7cfe5e9ebb1476ee575d57..3681b00d96a34467fe078c15a57b724d3f048d88 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -1,9 +1,9 @@
 // N, the self-hosted Nova compiler, so far its lexer, its parser, its
-// loader of loose programs and its resolver (spec
+// loader of programs and packages and its resolver (spec
 // docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md
 // §3.2 and §4.3, ADR 0051 and ADR 0052; spec
 // docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
-// §3.4, ADR 0053).
+// §3.4 and §4, ADRs 0053 and 0054).
 //
 //   selfhost tokens FILE...   prints each file's token dump, after a header
 //                             line `== FILE`, as `nova parse --dump-tokens`
@@ -11,12 +11,16 @@
 //   selfhost ast FILE...      prints each file's AST dump, its tree and its
 //                             errors, after the same header, as
 //                             `nova parse --dump-ast` does
-//   selfhost resolve --std DIR FILE...
+//   selfhost resolve --std DIR [--target native|wasm] [--test] FILE...
 //                             prints the resolution dump of each program
 //                             whose root is FILE, after the same header,
 //                             as `nova check --dump-resolution` does, with
-//                             std's modules read from DIR once
-//   selfhost resolve --std DIR --std-part
+//                             std's modules read from DIR once: a loose
+//                             program, or a module of the package whose
+//                             `src/` or `tests/` FILE is directly in; for
+//                             the WASM target, or under test mode (`nova
+//                             test`'s resolution)
+//   selfhost resolve --std DIR [--target native|wasm] [--test] --std-part
 //                             prints std's part, as `nova check
 //                             --dump-resolution --std` does
 //
@@ -33,6 +37,7 @@ import lexer
 import lexer_tests
 import loader
 import loader_tests
+import manifest
 import parser_tests
 import resolution_dump
 import resolver
@@ -72,33 +77,84 @@ async fn main() {
     if failed { exit(2) }
 }
 
-// `selfhost resolve --std DIR (--std-part | FILE...)`: std's modules are
-// read from DIR and parsed once, then std's part, or each program's part
-// after its header. Without `--std DIR`, or with neither `--std-part` nor
-// a file, or with both, nothing is printed and the exit code is 2, as S0's
-// `check --dump-resolution` gives.
+// `selfhost resolve --std DIR [--target native|wasm] [--test]
+// (--std-part | FILE...)`: std's modules are read from DIR and parsed
+// once, then std's part, or each program's part after its header, for
+// the target, natively by default, under test mode with `--test`. Each
+// option is taken once, and the target also as `--target=VALUE`, as
+// clap takes it for S0. Without `--std DIR`, with a target other than
+// `native` or `wasm`, or with neither `--std-part` nor a file, or with
+// both, nothing is printed and the exit code is 2, as S0's `check
+// --dump-resolution` gives; `--test` with `--target wasm` too, which S0
+// refuses with one line, as `nova test` has no target. A root whose
+// package's manifest N's reader refuses, or whose package graph would
+// give an M-code, S0's alone, stops the run: N says why on stderr and
+// exits 1, printing nothing of that root's part.
 async fn resolve_command(argv: Vec<String>) {
+    let usage = "usage: selfhost resolve --std DIR [--target native|wasm] [--test] (--std-part | FILE...)"
     let mut std_dir = ""
+    let mut target_name = ""
+    let mut target_given = false
+    let mut test = false
     let mut std_part = false
+    let mut bad = false
     let mut files: Vec<String> = Vec::new()
     let mut i = 2
     while i < argv.len() {
         let a = argv.at(i)
-        if a == "--std" && i + 1 < argv.len() {
-            std_dir = argv.at(i + 1)
+        if a.starts_with("--target=") {
+            // `--target=VALUE`, as clap takes it for S0; an empty value or
+            // a second `--target` is refused.
+            if target_given { bad = true }
+            target_given = true
+            target_name = a.byte_slice(9, a.byte_len())
+            if target_name == "" { bad = true }
+            i += 1
+            continue
+        }
+        if (a == "--std" || a == "--target") && i + 1 < argv.len() {
+            let value = argv.at(i + 1)
+            if a == "--std" {
+                if std_dir != "" { bad = true }
+                std_dir = value
+            } else {
+                if target_given { bad = true }
+                target_given = true
+                target_name = value
+                if target_name == "" { bad = true }
+            }
             i += 2
             continue
         }
-        if a == "--std-part" { std_part = true } else { files.push(a) }
+        if a == "--std-part" {
+            if std_part { bad = true }
+            std_part = true
+        } else if a == "--test" {
+            if test { bad = true }
+            test = true
+        } else if a.starts_with("--test=") {
+            // A flag takes no value, as clap refuses one for S0.
+            bad = true
+        } else if a == "--target" {
+            bad = true
+        } else {
+            files.push(a)
+        }
         i += 1
     }
-    if std_dir == "" || std_part == !files.is_empty() {
-        eprintln("usage: selfhost resolve --std DIR (--std-part | FILE...)")
+    let target = if target_name == "wasm" { CompileTargetWasm } else { CompileTargetNative }
+    if target_name != "" && target_name != "native" && target_name != "wasm" { bad = true }
+    if bad || std_dir == "" || std_part == !files.is_empty() {
+        eprintln(usage)
         exit(2)
         return
     }
-    let target = CompileTargetNative
-    let modules = match load_std(std_dir, target).await {
+    if test && target_name == "wasm" {
+        eprintln("error: selfhost resolve --test is native only, as nova test is")
+        exit(2)
+        return
+    }
+    let std_set = match load_std(std_dir, target, test).await {
         Ok(s) => s
         Err(message) => {
             eprintln(message)
@@ -106,34 +162,82 @@ async fn resolve_command(argv: Vec<String>) {
             return
         }
     }
-    // Natively the target's std modules are `STD_MODULES`.
-    let std_set = StdSet { modules: modules, natives: modules, builtins: builtin_table() }
     if std_part {
         let resolved = resolve_program_for(target, Vec::new(), std_set)
-        print(dump_std(target, resolved, std_set.builtins))
+        print(dump_std(target, test, resolved, std_set.builtins))
         return
     }
+    // std is resolved once, and every root reads it (ADR 0054 §9).
+    let shared = share_std(target, std_set)
     let mut failed = false
     for path in files.iter() {
-        match dump_resolution(path, target, std_set).await {
-            Ok(out) => print("== ${path}\n${out}")
-            Err(e) => {
+        match dump_resolution(path, target, test, shared).await {
+            DumpedText(out) => print("== ${path}\n${out}")
+            DumpedIoError(e) => {
                 print("== ${path}\nIOERR ${io_kind_name(e.kind)}\n")
                 failed = true
             }
+            DumpedRefused(why) => {
+                eprintln("error: selfhost resolve cannot dump ${path}: ${why}")
+                exit(1)
+                return
+            }
         }
     }
     if failed { exit(2) }
 }
 
-// The target's std modules, each read from under `dir` and parsed once,
-// in order, labelled as the driver registers them (`<std/core>`).
-async fn load_std(dir: String, target: CompileTarget) -> Result<Vec<StdModule>, String> {
+// What `dump_resolution` gives for one root: its part; the error that
+// kept its root from being read, `IOERR`; or why N refuses its package.
+type Dumped = | DumpedText(String) | DumpedIoError(IoError) | DumpedRefused(String)
+
+// What every program's resolution reads for `target`, under test mode
+// when `test`: the target's std modules, each read from under `dir` and
+// parsed once, labelled as the driver registers them (`<std/core>`), with
+// `std/test` after them under test mode, as `nova test` registers it;
+// `STD_MODULES`, parsed, which the WASM target's `missing` reads (a file
+// both lists name is parsed once); and the builtin table.
+async fn load_std(dir: String, target: CompileTarget, test: Bool) -> Result<StdSet, String> {
+    let mut parsed: Map<String, StdModule> = Map::new()
+    let natives = read_std_modules(dir, STD_MODULES, parsed).await?
+    let mut modules = match target {
+        CompileTargetNative => natives
+        CompileTargetWasm => read_std_modules(dir, WASM_STD_MODULES, parsed).await?
+    }
+    if test {
+        let mut with_test: Vec<StdModule> = Vec::new()
+        for m in modules.iter() { with_test.push(m) }
+        for m in read_std_modules(dir, [STD_TEST_MODULE], parsed).await?.iter() {
+            with_test.push(m)
+        }
+        modules = with_test
+    }
+    Ok(StdSet { modules: modules, natives: natives, builtins: builtin_table() })
+}
+
+// The std modules `list` names, in order, each read from under `dir` and
+// parsed, or taken from `parsed`, where each is kept by its file.
+async fn read_std_modules(
+    dir: String,
+    list: [(String, String)],
+    mut parsed: Map<String, StdModule>,
+) -> Result<Vec<StdModule>, String> {
     let mut out: Vec<StdModule> = Vec::new()
-    for (name, rel) in std_modules(target).iter() {
+    for (name, rel) in list.iter() {
+        match parsed.get(rel) {
+            Some(m) => {
+                out.push(m)
+                continue
+            }
+            None => ()
+        }
         let path = "${dir}/${rel}"
         match read_to_string(path).await {
-            Ok(source) => out.push(std_module(name, std_label(name), source))
+            Ok(source) => {
+                let m = std_module(name, std_label(name), source)
+                let _ = parsed.insert(rel, m)
+                out.push(m)
+            }
             Err(e) => return Err("error: cannot read std's ${path}: ${io_kind_name(e.kind)}")
         }
     }
@@ -141,23 +245,43 @@ async fn load_std(dir: String, target: CompileTarget) -> Result<Vec<StdModule>,
 }
 
 // The resolution dump of the program whose root is `path`
-// (nova-driver's `dump_resolution`): `check`'s pipeline up to the
-// resolver. It loads the program, stops there on any error, strips the
-// `@test` functions and resolves; the dump holds what was built and every
-// diagnostic so far. A root directly in a package's `src/` or `tests/` is
-// `UNSUPPORTED package`, 5.2b's. `Err` when the root cannot be read.
+// (nova-driver's `ResolutionDumps::dump`): `check`'s pipeline up to the
+// resolver, or under test mode `nova test`'s. A root directly in a
+// package's `src/` or `tests/` is a module of that package, whose graph
+// is read from the manifests first; any other root is a loose program.
+// It loads the program, stops there on any error, strips the `@test`
+// functions, but under test mode the root package's and a loose
+// program's, and resolves; under test mode a `tests/` file's tests are
+// named for it; the resolver reads std resolved once (`SharedStd`). The
+// dump holds what was built and every diagnostic so far.
 async fn dump_resolution(
     path: String,
     target: CompileTarget,
-    std_set: StdSet,
-) -> Result<String, IoError> {
-    if in_package(path).await { return Ok("UNSUPPORTED package\n") }
-    let load = load_program(path).await?
+    test: Bool,
+    shared: SharedStd,
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
     let modules = load.modules
     let mut diagnostics = load.diagnostics
     let mut failed = false
     for d in diagnostics.iter() { if d.is_error() { failed = true } }
-    if !failed { strip_test_functions(modules) }
+    if !failed { strip_test_functions(modules, test) }
     let mut sources: Vec<ModuleSource> = Vec::new()
     for m in modules.iter() {
         sources.push(
@@ -167,15 +291,38 @@ async fn dump_resolution(
                 file: m.ast,
                 imports: m.imports,
                 text: Some(m.text),
-                package: -1,
+                package: m.package,
                 importable: m.importable,
             },
         )
     }
-    if failed { return Ok(dump_program(target, sources, None, diagnostics, std_set.builtins)) }
-    let resolved = resolve_program_for(target, sources, std_set)
+    if failed {
+        return DumpedText(dump_program(target, test, sources, None, diagnostics, std_set.builtins))
+    }
+    let resolved = resolve_program_shared(sources, shared)
+    name_tests(resolved.tests, resolved.definitions, modules)
     for d in resolved.diagnostics.iter() { diagnostics.push(d) }
-    Ok(dump_program(target, sources, Some(resolved), diagnostics, std_set.builtins))
+    DumpedText(dump_program(target, test, sources, Some(resolved), diagnostics, std_set.builtins))
+}
+
+// Name each test of a module of a package's `tests/`
+// `<file stem>::<function>` (the driver's `name_tests`), as `nova test`'s
+// inventory names it.
+fn name_tests(tests: Vec<TestFn>, definitions: Definitions, modules: Vec<Loaded>) {
+    for t in tests.iter() {
+        if t.def_id < 0 || t.def_id >= definitions.defs.len() { continue }
+        let item = match definitions.defs.at(t.def_id).kind {
+            DefKindFn(i) => i
+            _ => continue
+        }
+        let mid = definitions.item_module.at(item)
+        if mid >= modules.len() { continue }
+        let m = modules.at(mid)
+        if m.test_file {
+            let mut named = t
+            named.name = "${module_name(m.path)}::${t.name}"
+        }
+    }
 }
 
 // The kind's name, as S0's dump prints it.
diff --git a/selfhost/src/manifest.nova b/selfhost/src/manifest.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8f4a734b249a88046c346bc846e50a99cc7b62b3
--- /dev/null
+++ b/selfhost/src/manifest.nova
@@ -0,0 +1,548 @@
+// N's minimal `nova.toml` reader (spec
+// docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md
+// §4.1, ADR 0054 §5): it reads what the loader needs of a manifest and
+// nothing more, the package's name and where its `[package]` table is,
+// and each path entry of `[dependencies]` and `[dev-dependencies]` with
+// where its key is. S0 reads `nova.toml` through nova-pm (`toml_edit`),
+// whose diagnostics (the M-codes) stay S0's alone: no input of the
+// differential has one. So this reader accepts a small part of TOML, the
+// part every manifest of the corpus, the cases and the fuzzer is written
+// in, and refuses the rest with a message, which the driver prints on
+// stderr before it exits non-zero, never printing a partial dump:
+//
+// - comments, blank lines, a byte-order mark, and the tables
+//   `[package]`, `[dependencies]` and `[dev-dependencies]`, each once, the
+//   two dependency tables also as one inline table of inline tables
+//   before every header (`dependencies = { geom = { path = "g" } }`, as
+//   `nova add` writes into one);
+// - in `[package]`, the keys nova-pm reads (`PACKAGE_KEYS`): `name`,
+//   `version` and `edition`, which must be there, `description`,
+//   `license` and `repository` as strings, `authors`, `keywords` and
+//   `categories` as arrays of strings on one line;
+// - in the dependency tables, `key = { path = "..." }` on one line, and
+//   in `[dev-dependencies]` also a version entry, `key = "REQ"` or
+//   `key = { version = "REQ" }`, whose requirement is a version of one to
+//   three numbers after `^`, `~`, `=` or nothing, which only a
+//   dependency's manifest may hold, where nova-pm reads it and never
+//   resolves it (the graph refuses one in the root's manifest);
+// - bare keys, and strings in `"..."` without an escape or in `'...'`.
+//
+// Anything else, among it what S0 would warn about (an unknown key,
+// M0006) or reject (a version entry in `[dependencies]`, which needs the
+// registry; a name or a version nova-pm refuses; an edition other than
+// 2026), is refused, and so is any other requirement. A version is any
+// semver takes with no build metadata, a pre-release included, each of
+// its three numbers at most `u64::MAX`, as nova-pm takes it.
+
+import builtins
+import lexer
+import token
+
+/// An entry of `[dependencies]` or `[dev-dependencies]` (nova-pm's
+/// `Dependency`): its key, its `path`, or "" for a version entry, its
+/// requirement, or "" for a path entry, and where its key is.
+pub record Dependency { pub name: String, pub path: String, pub version: String, pub span: Span }
+
+/// What the loader reads of a `nova.toml` (nova-pm's `Manifest`): the
+/// package's name, where `[package]` is, from its header to its last
+/// value, as toml_edit spans a table, and its path entries in the order
+/// they are written.
+pub record Manifest {
+    pub name: String,
+    pub span: Span,
+    pub dependencies: Vec<Dependency>,
+    pub dev_dependencies: Vec<Dependency>,
+}
+
+/// The keys of `[package]` nova-pm reads (`PACKAGE_KEYS`).
+const PACKAGE_KEYS: [String] = [
+    "name",
+    "version",
+    "edition",
+    "description",
+    "license",
+    "repository",
+    "authors",
+    "keywords",
+    "categories",
+]
+
+/// Names Windows reserves for devices, which no package may take
+/// (nova-pm's `WINDOWS_DEVICES`).
+const WINDOWS_DEVICES: [String] = [
+    "con",
+    "prn",
+    "aux",
+    "nul",
+    "com1",
+    "com2",
+    "com3",
+    "com4",
+    "com5",
+    "com6",
+    "com7",
+    "com8",
+    "com9",
+    "lpt1",
+    "lpt2",
+    "lpt3",
+    "lpt4",
+    "lpt5",
+    "lpt6",
+    "lpt7",
+    "lpt8",
+    "lpt9",
+]
+
+// A value this reader accepts: a string, an array of strings, or an
+// inline table of strings by key.
+type Value = | ValueString(String) | ValueArray(Vec<String>) | ValueTable(Vec<(String, String)>)
+
+/// The manifest `text` holds, or why this reader refuses it.
+pub fn parse_manifest(text: String) -> Result<Manifest, String> {
+    let n = text.byte_len()
+    let mut table = ""
+    let mut tables: Vec<String> = Vec::new()
+    let mut keys: Vec<String> = Vec::new()
+    let mut name = ""
+    let mut version = ""
+    let mut edition = ""
+    let mut package_start = -1
+    let mut package_end = -1
+    let mut dependencies: Vec<Dependency> = Vec::new()
+    let mut dev_dependencies: Vec<Dependency> = Vec::new()
+    // TOML allows no control character but a tab, a line feed, and a
+    // carriage return right before one (toml_edit's M0001).
+    let mut c = 0
+    while c < n {
+        let b = text.byte_at(c)
+        let crlf = b == 13 && c + 1 < n && text.byte_at(c + 1) == 10
+        if (b < 32 && b != 9 && b != 10 && !crlf) || b == 127 {
+            return Err("a control character other than a tab or a line break is not TOML")
+        }
+        c += 1
+    }
+    // A byte-order mark is skipped, as toml_edit skips it; spans still
+    // count its three bytes.
+    let bom = n >= 3 && text.byte_at(0) == 239 && text.byte_at(1) == 187 && text.byte_at(2) == 191
+    let mut line_start = if bom { 3 } else { 0 }
+    while line_start < n {
+        let mut line_end = line_start
+        while line_end < n && text.byte_at(line_end) != 10 { line_end += 1 }
+        let mut end = line_end
+        if end > line_start && text.byte_at(end - 1) == 13 { end -= 1 }
+        let i = skip_space(text, line_start, end)
+        if i < end && text.byte_at(i) != 35 {
+            if text.byte_at(i) == 91 {
+                // A table's header.
+                let k = skip_space(text, i + 1, end)
+                let k_end = bare_key_end(text, k, end)
+                let close = skip_space(text, k_end, end)
+                if k == k_end || close >= end || text.byte_at(close) != 93 {
+                    return Err("only a `[table]` header of one bare key is read")
+                }
+                if !line_ends(text, close + 1, end) {
+                    return Err("only a comment may follow a table's header")
+                }
+                table = text.byte_slice(k, k_end)
+                if table != "package" && table != "dependencies" && table != "dev-dependencies" {
+                    return Err("the table `${table}` is not read")
+                }
+                if contains(tables, table) { return Err("the table `${table}` is written twice") }
+                tables.push(table)
+                if table == "package" { package_start = i }
+            } else {
+                // A key and its value.
+                let k_end = bare_key_end(text, i, end)
+                let eq = skip_space(text, k_end, end)
+                if k_end == i || eq >= end || text.byte_at(eq) != 61 {
+                    return Err("only `key = value` with a bare key is read")
+                }
+                let key = text.byte_slice(i, k_end)
+                if table == "" && (key == "dependencies" || key == "dev-dependencies") {
+                    // The table as one inline table of inline tables, before
+                    // every header, as `nova add` writes it into one.
+                    if contains(tables, key) { return Err("the table `${key}` is written twice") }
+                    tables.push(key)
+                    let (entries, entries_end) = parse_dependency_table(
+                        text,
+                        skip_space(text, eq + 1, end),
+                        end,
+                        key == "dev-dependencies",
+                    )?
+                    if !line_ends(text, entries_end, end) {
+                        return Err("only a comment may follow the value of `${key}`")
+                    }
+                    for d in entries.iter() {
+                        if key == "dependencies" {
+                            dependencies.push(d)
+                        } else {
+                            dev_dependencies.push(d)
+                        }
+                    }
+                    line_start = line_end + 1
+                    continue
+                }
+                let (value, value_end) = parse_value(text, skip_space(text, eq + 1, end), end)?
+                if !line_ends(text, value_end, end) {
+                    return Err("only a comment may follow the value of `${key}`")
+                }
+                if table == "" { return Err("the key `${key}` is outside every table") }
+                if contains(keys, "${table}.${key}") {
+                    return Err("the key `${table}.${key}` is written twice")
+                }
+                keys.push("${table}.${key}")
+                if table == "package" {
+                    package_end = value_end
+                    if !holds(PACKAGE_KEYS, key) {
+                        return Err("the key `package.${key}` is not read")
+                    }
+                    let is_array = key == "authors" || key == "keywords" || key == "categories"
+                    match value {
+                        ValueString(s) => {
+                            if is_array {
+                                return Err("`package.${key}` must be an array of strings")
+                            }
+                            if key == "name" { name = s }
+                            if key == "version" { version = s }
+                            if key == "edition" { edition = s }
+                        }
+                        ValueArray(_) => {
+                            if !is_array { return Err("`package.${key}` must be a string") }
+                        }
+                        ValueTable(_) => return Err("`package.${key}` may not be a table")
+                    }
+                } else {
+                    let d = entry(
+                        key,
+                        value,
+                        table == "dev-dependencies",
+                        Span { start: i, end: k_end },
+                    )?
+                    if table == "dependencies" {
+                        dependencies.push(d)
+                    } else {
+                        dev_dependencies.push(d)
+                    }
+                }
+            }
+        }
+        line_start = line_end + 1
+    }
+    if package_start < 0 { return Err("there is no `[package]` table") }
+    for key in ["name", "version", "edition"].iter() {
+        if !contains(keys, "package.${key}") { return Err("`[package]` has no `${key}`") }
+    }
+    check_name(name)?
+    if !is_version(version) { return Err("`${version}` is not a version this reader takes") }
+    if edition != "2026" { return Err("unknown edition `${edition}`") }
+    Ok(
+        Manifest {
+            name: name,
+            span: Span { start: package_start, end: package_end },
+            dependencies: dependencies,
+            dev_dependencies: dev_dependencies,
+        },
+    )
+}
+
+/// A package's import name, its name with each `-` as `_` (nova-pm's
+/// `import_name`).
+pub fn import_name(name: String) -> String {
+    let mut out = StringBuilder::new()
+    let mut i = 0
+    while i < name.byte_len() {
+        let c = name.byte_at(i)
+        if c == 45 { out.push("_") } else { out.push(name.byte_slice(i, i + 1)) }
+        i += 1
+    }
+    out.to_string()
+}
+
+/// Whether `name` is one of Nova's keywords, which no import name may be
+/// (nova-pm's `is_keyword`).
+pub fn is_keyword(name: String) -> Bool { holds(KEYWORDS, name) }
+
+// The entry `key = value` at `span`: a `path` alone in an inline table,
+// or, where `versions` (in `[dev-dependencies]`), a requirement
+// [`is_requirement`] takes, as a string or a `version` alone in an inline
+// table. A `version` in `[dependencies]` needs the registry, a `version`
+// beside a `path` is M0003, and any other key is M0006.
+fn entry(key: String, value: Value, versions: Bool, span: Span) -> Result<Dependency, String> {
+    let (field, text) = match value {
+        ValueString(s) => ("version", s)
+        ValueTable(fields) => {
+            if fields.len() != 1 {
+                return Err("dependency `${key}` must be `{ path = \"...\" }` alone")
+            }
+            fields.at(0)
+        }
+        ValueArray(_) => ("", "")
+    }
+    if field == "path" { return Ok(Dependency { name: key, path: text, version: "", span: span }) }
+    if field == "version" && versions && is_requirement(text) {
+        return Ok(Dependency { name: key, path: "", version: text, span: span })
+    }
+    Err("dependency `${key}` must be `{ path = \"...\" }`")
+}
+
+// Whether `text` is a requirement this reader takes, one semver's
+// `VersionReq::parse` surely takes: `^`, `~`, `=` or nothing, then one to
+// three numbers between dots, each without a leading zero and at most
+// `u64::MAX`.
+fn is_requirement(text: String) -> Bool {
+    let mut rest = text
+    if rest.starts_with("^") || rest.starts_with("~") || rest.starts_with("=") {
+        rest = rest.byte_slice(1, rest.byte_len())
+    }
+    let parts = rest.split(".")
+    if parts.len() > 3 { return false }
+    for p in parts.iter() { if !is_u64(p) { return false } }
+    true
+}
+
+// nova-pm's `check_name`: ASCII letters, digits, `-` and `_`, starting
+// with a letter, at most 64 characters, not a Windows device name, and
+// not a std module's name.
+fn check_name(name: String) -> Result<(), String> {
+    let n = name.byte_len()
+    if n == 0 || n > 64 || !is_letter(name.byte_at(0)) {
+        return Err("package name `${name}` is not one this reader takes")
+    }
+    let mut i = 0
+    while i < n {
+        let c = name.byte_at(i)
+        if !(is_letter(c) || is_digit(c) || c == 45 || c == 95) {
+            return Err("package name `${name}` is not one this reader takes")
+        }
+        i += 1
+    }
+    if holds(WINDOWS_DEVICES, name.to_lower()) || name == "dom" || name == "ui" {
+        return Err("package name `${name}` is not one this reader takes")
+    }
+    Ok(())
+}
+
+// Whether `text` is a version nova-pm takes: one `semver::Version::parse`
+// takes, `MAJOR.MINOR.PATCH`, each a number without a leading zero, then
+// optionally `-` and a pre-release, identifiers of ASCII letters, digits
+// and `-` between dots, none empty, a numeric one without a leading zero;
+// and with no build metadata (`+...`), which nova-pm refuses (M0003).
+fn is_version(text: String) -> Bool {
+    if byte_index(text, 43) >= 0 { return false }
+    let mut rest = text
+    let minus = byte_index(rest, 45)
+    if minus >= 0 {
+        if !identifiers(rest.byte_slice(minus + 1, rest.byte_len())) { return false }
+        rest = rest.byte_slice(0, minus)
+    }
+    let parts = rest.split(".")
+    if parts.len() != 3 { return false }
+    for p in parts.iter() { if !is_u64(p) { return false } }
+    true
+}
+
+// Whether `text` is a number without a leading zero that fits `u64`, as
+// semver reads a version's and a requirement's numbers: at most 20
+// digits, and 20 only up to `18446744073709551615`.
+fn is_u64(text: String) -> Bool {
+    if !is_number(text) { return false }
+    let n = text.byte_len()
+    if n < 20 { return true }
+    if n > 20 { return false }
+    let max = "18446744073709551615"
+    let mut i = 0
+    while i < n {
+        let a = text.byte_at(i)
+        let b = max.byte_at(i)
+        if a != b { return a < b }
+        i += 1
+    }
+    true
+}
+
+// Whether `text` is a pre-release: dot-separated identifiers of ASCII
+// letters, digits and `-`, none empty, each that is all digits a number
+// without a leading zero.
+fn identifiers(text: String) -> Bool {
+    for id in text.split(".").iter() {
+        let n = id.byte_len()
+        if n == 0 { return false }
+        let mut digits = true
+        let mut i = 0
+        while i < n {
+            let c = id.byte_at(i)
+            if !(is_letter(c) || is_digit(c) || c == 45) { return false }
+            if !is_digit(c) { digits = false }
+            i += 1
+        }
+        if digits && !is_number(id) { return false }
+    }
+    true
+}
+
+// Where the first byte `c` of `text` is, or -1.
+fn byte_index(text: String, c: Int) -> Int {
+    let mut i = 0
+    while i < text.byte_len() {
+        if text.byte_at(i) == c { return i }
+        i += 1
+    }
+    -1
+}
+
+// Whether `text` is a number without a leading zero.
+fn is_number(text: String) -> Bool {
+    let n = text.byte_len()
+    if n == 0 || (n > 1 && text.byte_at(0) == 48) { return false }
+    let mut i = 0
+    while i < n {
+        if !is_digit(text.byte_at(i)) { return false }
+        i += 1
+    }
+    true
+}
+
+// The inline table of dependencies starting at `i`, before `end`, and
+// where it ends: `{}`, or `{ key = { path = "..." }, ... }` on one line,
+// each key bare and once, and where `versions` each entry also as
+// [`entry`] takes it.
+fn parse_dependency_table(
+    text: String,
+    i: Int,
+    end: Int,
+    versions: Bool,
+) -> Result<(Vec<Dependency>, Int), String> {
+    let mut entries: Vec<Dependency> = Vec::new()
+    if i >= end || text.byte_at(i) != 123 {
+        return Err("a dependency table outside its header must be an inline table")
+    }
+    let mut j = skip_space(text, i + 1, end)
+    if j < end && text.byte_at(j) == 125 { return Ok((entries, j + 1)) }
+    while true {
+        let k_end = bare_key_end(text, j, end)
+        let eq = skip_space(text, k_end, end)
+        if k_end == j || eq >= end || text.byte_at(eq) != 61 {
+            return Err("only `key = { path = \"...\" }` with a bare key is read")
+        }
+        let key = text.byte_slice(j, k_end)
+        for d in entries.iter() {
+            if d.name == key { return Err("the key `${key}` is written twice") }
+        }
+        let (value, after) = parse_value(text, skip_space(text, eq + 1, end), end)?
+        entries.push(entry(key, value, versions, Span { start: j, end: k_end })?)
+        j = skip_space(text, after, end)
+        if j < end && text.byte_at(j) == 125 { return Ok((entries, j + 1)) }
+        if j >= end || text.byte_at(j) != 44 {
+            return Err("only an inline table on one line is read")
+        }
+        j = skip_space(text, j + 1, end)
+    }
+    Err("unreachable")
+}
+
+// The value starting at `i`, before `end`, and where it ends: a string,
+// an array of strings, or an inline table of strings.
+fn parse_value(text: String, i: Int, end: Int) -> Result<(Value, Int), String> {
+    if i >= end { return Err("a key has no value") }
+    let c = text.byte_at(i)
+    if c == 34 || c == 39 {
+        let (s, after) = parse_string(text, i, end)?
+        return Ok((ValueString(s), after))
+    }
+    if c == 91 {
+        let mut items: Vec<String> = Vec::new()
+        let mut j = skip_space(text, i + 1, end)
+        while j < end && text.byte_at(j) != 93 {
+            let (s, after) = parse_string(text, j, end)?
+            items.push(s)
+            j = skip_space(text, after, end)
+            if j < end && text.byte_at(j) == 44 {
+                j = skip_space(text, j + 1, end)
+            } else if j >= end || text.byte_at(j) != 93 {
+                return Err("only an array of strings on one line is read")
+            }
+        }
+        if j >= end { return Err("only an array of strings on one line is read") }
+        return Ok((ValueArray(items), j + 1))
+    }
+    if c == 123 {
+        let mut fields: Vec<(String, String)> = Vec::new()
+        let mut j = skip_space(text, i + 1, end)
+        if j < end && text.byte_at(j) == 125 { return Ok((ValueTable(fields), j + 1)) }
+        while true {
+            let k_end = bare_key_end(text, j, end)
+            let eq = skip_space(text, k_end, end)
+            if k_end == j || eq >= end || text.byte_at(eq) != 61 {
+                return Err("only an inline table of bare keys and strings is read")
+            }
+            let key = text.byte_slice(j, k_end)
+            for (k, _) in fields.iter() {
+                if k == key { return Err("the key `${key}` is written twice") }
+            }
+            let (s, after) = parse_string(text, skip_space(text, eq + 1, end), end)?
+            fields.push((key, s))
+            j = skip_space(text, after, end)
+            if j < end && text.byte_at(j) == 125 { return Ok((ValueTable(fields), j + 1)) }
+            if j >= end || text.byte_at(j) != 44 {
+                return Err("only an inline table on one line is read")
+            }
+            j = skip_space(text, j + 1, end)
+        }
+    }
+    Err("only strings, arrays of strings and inline tables are read")
+}
+
+// The string starting at `i`, a quote, and where it ends: `"..."` with no
+// escape, or `'...'`, on one line, holding no control character.
+fn parse_string(text: String, i: Int, end: Int) -> Result<(String, Int), String> {
+    if i >= end { return Err("a string is missing") }
+    let quote = text.byte_at(i)
+    if quote != 34 && quote != 39 { return Err("only a string is read here") }
+    if i + 2 < end && text.byte_at(i + 1) == quote && text.byte_at(i + 2) == quote {
+        return Err("a multi-line string is not read")
+    }
+    let mut j = i + 1
+    while j < end && text.byte_at(j) != quote {
+        let c = text.byte_at(j)
+        if (c < 32 && c != 9) || c == 127 { return Err("a string holds a control character") }
+        if c == 92 && quote == 34 { return Err("a string's escape is not read") }
+        j += 1
+    }
+    if j >= end { return Err("a string is not closed on its line") }
+    Ok((text.byte_slice(i + 1, j), j + 1))
+}
+
+// `i` moved past spaces and tabs, but not to `end`.
+fn skip_space(text: String, i: Int, end: Int) -> Int {
+    let mut j = i
+    while j < end && (text.byte_at(j) == 32 || text.byte_at(j) == 9) { j += 1 }
+    j
+}
+
+// Where the bare key starting at `i` ends: past ASCII letters, digits,
+// `-` and `_`.
+fn bare_key_end(text: String, i: Int, end: Int) -> Int {
+    let mut j = i
+    while j < end {
+        let c = text.byte_at(j)
+        if !(is_letter(c) || is_digit(c) || c == 45 || c == 95) { break }
+        j += 1
+    }
+    j
+}
+
+// Whether only spaces, and then a comment, if any, are left of the line.
+fn line_ends(text: String, i: Int, end: Int) -> Bool {
+    let j = skip_space(text, i, end)
+    j >= end || text.byte_at(j) == 35
+}
+
+fn is_letter(c: Int) -> Bool { (c >= 65 && c <= 90) || (c >= 97 && c <= 122) }
+
+fn is_digit(c: Int) -> Bool { c >= 48 && c <= 57 }
+
+fn contains(list: Vec<String>, name: String) -> Bool {
+    for n in list.iter() { if n == name { return true } }
+    false
+}
diff --git a/selfhost/src/resolution_dump.nova b/selfhost/src/resolution_dump.nova
index 9b7d44d1228eb87874183b89fc5cc6a2f4012445..cb2c9921ee7b5a34f7107e5cf6aa56d0e9163ddd 100644
--- a/selfhost/src/resolution_dump.nova
+++ b/selfhost/src/resolution_dump.nova
@@ -23,12 +23,13 @@ import loader
 import resolver
 import token
 
-/// A program's part for `target`: its `modules` as the loader gave them,
-/// its resolution once it got that far, and every diagnostic, the
-/// loader's and the resolver's, in order (`dump_program`); `table` names
-/// the builtins.
+/// A program's part for `target`, under test mode when `test`: its
+/// `modules` as the loader gave them, its resolution once it got that far,
+/// and every diagnostic, the loader's and the resolver's, in order
+/// (`dump_program`); `table` names the builtins.
 pub fn dump_program(
     target: CompileTarget,
+    test: Bool,
     modules: Vec<ModuleSource>,
     resolution: Option<ProgramResolution>,
     diagnostics: Vec<Diagnostic>,
@@ -42,6 +43,7 @@ pub fn dump_program(
     }
     d.open(0, "Program")
     d.field("target", compile_target_name(target))
+    d.mode(test)
     d.bool_field("resolved", resolution.is_some())
     let mut index = 0
     for m in modules.iter() {
@@ -105,10 +107,11 @@ pub fn dump_program(
     d.out.to_string()
 }
 
-/// std's part for `target`, from the resolution of a program of no module
-/// of its own (`dump_std`).
+/// std's part for `target`, under test mode when `test`, from the
+/// resolution of a program of no module of its own (`dump_std`).
 pub fn dump_std(
     target: CompileTarget,
+    test: Bool,
     resolution: ProgramResolution,
     table: BuiltinTable,
 ) -> String {
@@ -118,7 +121,7 @@ pub fn dump_std(
         fresh: true,
         variants: table.variants,
     }
-    d.std_part(target, resolution.definitions, 0, 0, 0)
+    d.std_part(target, test, resolution.definitions, 0, 0, 0)
     d.newline()
     d.diagnostics(resolution.diagnostics)
     d.out.to_string()
@@ -226,6 +229,10 @@ impl ResDump {
     // ` key=value`.
     fn field(mut self, key: String, value: String) { self.out.push(" ${key}=${value}") }
 
+    // ` mode=test` under test mode, and nothing under `check`'s, so that
+    // a part `check` prints is 5.2a's (`mode`).
+    fn mode(mut self, test: Bool) { if test { self.out.push(" mode=test") } }
+
     fn bool_field(mut self, key: String, value: Bool) { self.out.push(" ${key}=${value}") }
 
     // ` ` and `text` quoted.
@@ -251,6 +258,7 @@ impl ResDump {
     fn std_part(
         mut self,
         target: CompileTarget,
+        test: Bool,
         defs: Definitions,
         std_module: Int,
         std_def: Int,
@@ -258,6 +266,7 @@ impl ResDump {
     ) {
         self.open(0, "Std")
         self.field("target", compile_target_name(target))
+        self.mode(test)
         self.field("modules", "${defs.modules.len() - std_module}")
         self.field("defs", "${defs.defs.len() - std_def}")
         self.field("items", "${defs.item_module.len() - std_item}")
diff --git a/selfhost/src/resolver.nova b/selfhost/src/resolver.nova
index 4e9ebcb716ac2c4b924dcc40d7d6add225d2aee8..cd64d6a6de6f5b23ad5e14c283f71f04789b9fba 100644
--- a/selfhost/src/resolver.nova
+++ b/selfhost/src/resolver.nova
@@ -219,6 +219,24 @@ impl Exports {
         out
     }
 
+    /// A copy of these exports with every definition's id `offset` higher,
+    /// as they are when `offset` definitions come before their module's
+    /// (`Exports::rebased`); these are left as they are.
+    pub fn rebased(self, offset: Int) -> Exports {
+        let mut out = Exports::new()
+        for n in self.value_names.iter() {
+            let r = match self.value(n) {
+                ResDef(id) => ResDef(id + offset)
+                ResVariant(sum, index) => ResVariant(sum + offset, index)
+                ResBuiltin(b) => ResBuiltin(b)
+            }
+            out.export_value(n, r)
+        }
+        for n in self.type_names.iter() { out.export_type(n, self.type_of(n) + offset) }
+        for n in self.trait_names.iter() { out.export_trait(n, self.trait_of(n) + offset) }
+        out
+    }
+
     /// What the exported value `name` resolves to; it is exported.
     pub fn value(self, name: String) -> Res { self.values.get(name).unwrap() }
 
@@ -490,6 +508,251 @@ pub fn resolve_program_for(
     }
 }
 
+/// std resolved once for every root of a `selfhost resolve` run (5.2b,
+/// ADR 0054 §9, nova-resolver's `SharedStd`): the resolution of a program
+/// of no module of its own, from which each program takes std's exports,
+/// its globbed variants and its `missing` names. A program's resolution
+/// only reads it, through copies.
+pub record SharedStd {
+    pub target: CompileTarget,
+    pub std_set: StdSet,
+    pub exports: Vec<Exports>,
+    pub variants: Map<String, String>,
+    pub missing: Map<String, String>,
+    // What std's glob binds in every user module (`import_std_module`
+    // from 0): each such std module's exported values, types and traits
+    // in order, their definitions numbered from 0.
+    pub globs: Vec<(Vec<(String, Res)>, Vec<(String, Int)>, Vec<(String, Int)>)>,
+    // Whether resolving std gave no diagnostic beyond parsing it and found
+    // no `@test` function, which the shared path would drop; else a program
+    // is resolved in full.
+    pub shareable: Bool,
+}
+
+/// std for `target`, resolved once (`SharedStd::new`).
+pub fn share_std(target: CompileTarget, std_set: StdSet) -> SharedStd {
+    let alone = resolve_program_for(target, Vec::new(), std_set)
+    let mut parsed = 0
+    let mut all: Vec<ModuleSource> = Vec::new()
+    for s in std_set.modules.iter() {
+        parsed += s.diagnostics.len()
+        match s.file {
+            Some(file) => all.push(module_source(s.name, s.label, file))
+            None => ()
+        }
+    }
+    let exports = alone.definitions.exports
+    let mut globs: Vec<(Vec<(String, Res)>, Vec<(String, Int)>, Vec<(String, Int)>)> = Vec::new()
+    let mut k = 0
+    while k < exports.len() {
+        let name = all.at(k).name
+        if !is_imported_std_module(name) && !holds(STD_INTERNAL, name) {
+            let e = exports.at(k)
+            globs.push((e.values_in_order(), e.types_in_order(), e.traits_in_order()))
+        }
+        k += 1
+    }
+    SharedStd {
+        target: target,
+        std_set: std_set,
+        exports: exports,
+        variants: std_globbed_variants(alone.definitions, exports, all, 0),
+        missing: alone.definitions.missing,
+        globs: globs,
+        shareable: alone.diagnostics.len() == parsed && alone.tests.is_empty(),
+    }
+}
+
+/// `resolve_program_for` with std resolved once (`SharedStd`), for the
+/// resolution dump alone (nova-resolver's
+/// `resolve_program_with_shared_std`): the user's modules go through the
+/// same steps in the same order, reading std's exports rebased past the
+/// user's definitions; std's modules are not resolved again, and the
+/// result holds the user's definitions, scopes and items and every
+/// module's exports, which the dump prints.
+pub fn resolve_program_shared(modules: Vec<ModuleSource>, shared: SharedStd) -> ProgramResolution {
+    let target = shared.target
+    let std_set = shared.std_set
+    if !shared.shareable { return resolve_program_for(target, modules, std_set) }
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
+        missing: shared.missing,
+    }
+    let mut diagnostics: Vec<Diagnostic> = Vec::new()
+    let mut merged: Vec<Spanned<Item>> = Vec::new()
+    let mut tests: Vec<TestFn> = Vec::new()
+    for s in std_set.modules.iter() { for d in s.diagnostics.iter() { diagnostics.push(d) } }
+    let user_count = modules.len()
+    let mut all: Vec<ModuleSource> = Vec::new()
+    for m in modules.iter() { all.push(m) }
+    for s in std_set.modules.iter() {
+        match s.file {
+            Some(file) => all.push(module_source(s.name, s.label, file))
+            None => ()
+        }
+    }
+    let std_start = user_count
+    let mut exports: Vec<Exports> = Vec::new()
+    let mut mid = 0
+    while mid < std_start {
+        let mut scope = new_scope(all.at(mid).name)
+        seed(scope, table, table.global)
+        definitions.modules.push(scope)
+        exports.push(Exports::new())
+        mid += 1
+    }
+    mid = 0
+    while mid < std_start {
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
+    // std's definitions come after the user's.
+    let offset = definitions.defs.len()
+    for e in shared.exports.iter() { exports.push(e.rebased(offset)) }
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
+    let mut public_fixes: Vec<(Int, Int, String)> = Vec::new()
+    mid = 0
+    while mid < std_start {
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
+    // No std module imports a user's, so none would receive a name.
+    for (at, fix_target, name) in public_fixes.iter() {
+        let mut clash = false
+        let mut g = 0
+        while g < std_start && !clash {
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
+    // std's glob into the user's modules, as `import_std_module` binds it,
+    // each std module's in turn, a name a module has kept: an imported std
+    // module's binds into none, and `STD_INTERNAL`'s into std's alone.
+    for (values, types, traits) in shared.globs.iter() {
+        mid = 0
+        while mid < std_start {
+            let mut scope = definitions.modules.at(mid)
+            for (n, r) in values.iter() {
+                if !scope.values.contains_key(n) {
+                    let shifted = match r {
+                        ResDef(id) => ResDef(id + offset)
+                        ResVariant(sum, index) => ResVariant(sum + offset, index)
+                        ResBuiltin(b) => ResBuiltin(b)
+                    }
+                    let _ = scope.values.insert(n, shifted)
+                }
+            }
+            for (n, id) in types.iter() {
+                if !scope.types.contains_key(n) { let _ = scope.types.insert(n, id + offset) }
+            }
+            for (n, id) in traits.iter() {
+                if !scope.traits.contains_key(n) { let _ = scope.traits.insert(n, id + offset) }
+            }
+            mid += 1
+        }
+    }
+    for d in variant_shadowing_of(shared.variants, all, std_start).iter() { diagnostics.push(d) }
+    for m in all.iter() {
+        definitions.importable.push(m.importable)
+        definitions.packages.push(m.package)
+    }
+    definitions.std_start = std_start
+    definitions.exports = exports
+    let mut splits: Vec<(String, NewlineSplit)> = Vec::new()
+    mid = 0
+    while mid < std_start {
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
 // Bind each builtin of `builtins` by its name in `scope`'s values.
 fn seed(mut scope: ModuleScope, table: BuiltinTable, builtins: Vec<Int>) {
     for b in builtins.iter() { let _ = scope.values.insert(table.names[b], ResBuiltin(b)) }
@@ -550,7 +813,17 @@ fn variant_shadowing(
     all: Vec<ModuleSource>,
     std_start: Int,
 ) -> Vec<Diagnostic> {
-    let std_variants = std_globbed_variants(definitions, exports, all, std_start)
+    variant_shadowing_of(std_globbed_variants(definitions, exports, all, std_start), all, std_start)
+}
+
+/// `variant_shadowing`'s warnings for the user's modules, the first
+/// `std_start` of `all`, from `std_variants`, the variants std globs into
+/// them with their sums' names (`variant_shadowing_of`).
+fn variant_shadowing_of(
+    std_variants: Map<String, String>,
+    all: Vec<ModuleSource>,
+    std_start: Int,
+) -> Vec<Diagnostic> {
     let mut out: Vec<Diagnostic> = Vec::new()
     let mut mid = 0
     while mid < std_start && mid < all.len() {
````

### Task 3: The cases, the fuzzer's WASM and package inputs, and the differential on both sides

Spec §4.1-§4.3, §5; decisions 6, 8, 13 to 18, 20, 23 and 25.

**Files:**
- Create: 331 files under `crates/nova-cli/tests/selfhost/cases/`:
  102 in `resolve/` (packages beside 5.2a's loose cases),
  36 in `resolve-wasm/`, 193 in `resolve-test/`
- Modify: `crates/nova-cli/tests/selfhost.rs`,
  `crates/nova-cli/tests/selfhost/fuzz.rs`,
  `crates/nova-cli/tests/selfhost/compare.rs`,
  `crates/nova-resolver/src/dump.rs` (`KINDS`),
  `crates/nova-driver/src/lib.rs` (the equivalence test's floors)

- [ ] **Step 1: The tests.** `patch_block t3-tests`:
  - **the cases:** a directory's name gives the mode (`resolve/` check,
    `resolve-wasm/`, `resolve-test/`); a directory with `nova.toml` is a
    package, each `.nova` file directly in its `src/` then `tests/` a
    root; any other directory a loose program rooted at `main.nova`.
    145 / 30 / 126 programs, 14 of them written by hand (ADR 0054 §2;
    `a_tests_module_named_for_the_package_is_e0004_at_its_package_table`,
    `[package]`'s last value before a comment, decision 23);
  - **`selfhost.rs`:** `Mode`, the case directories (`case_roots`), the
    package, WASM and test corpora, std's three parts side by side, each
    root's part counted by its header, the refusals
    (`a_manifest_n_cannot_read_stops_n_with_a_message`), `--test` in
    every position, `--test=VALUE`, an empty or repeated `--target` and
    `--target=VALUE` (`both_sides_take_test_in_every_position_alike`, 15
    shapes), std's fallback in N
    (`n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`),
    a trait's rebasing in N
    (`a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`),
    package identity on Unix, each pair of paths each way round
    (`a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`),
    `settle_paths` (`the_paths_of_e0004_and_e0093_are_settled_on_windows_alone`)
    and `run` settling every resolution dump, S0's and N's
    (`every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
    decision 25),
    the fuzzer's seeds and floors, its package inputs in four manifest
    forms (form 3's every line ending `\r\n`) and two seeds' also under
    test mode and for the WASM target
    (`the_name_fuzzer_agrees_and_gives_every_resolver_code`, renamed from
    5.2a's `…every_native_resolver_code`), the CRLF cases per mode, and
    the speed-up (decision 17); the three tests of Task 1 compare both
    sides again;
  - **`dump.rs`:** `KINDS` gains 5.2b's seven forms, and `LATER_KINDS`
    keeps `(Help` and `(NoteDiagnostic`;
  - **the driver's equivalence test:** the cases of every mode, and the
    floors 400 / 60 / 120.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. `SH`. Expected: `selfhost.rs` does not compile: 5 errors,
  `E0425` (`fuzz::PACKAGE_FILES`, `package_manifests`,
  `wasm_name_mutate`, `package_name_mutate`) and `E0432`
  (`compare::settle_paths`).
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`fuzz.rs`:** `PACKAGE_FILES`, `package_manifests`, `WASM_ADDITIONS`,
    `PACKAGE_ADDITIONS`, `add_one`, `wasm_name_mutate`,
    `package_name_mutate` (ADR 0054 §3);
  - **`compare.rs`:** `settle_paths`, `\` read as `/` in E0004's and
    E0093's messages on Windows alone (decision 14).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH`. Expected: `44 passed; 0 failed`.
  - `git status --short`. Expected: nothing.
  - `S` (210 s). Expected: `2869 1 10`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `selfhost: the resolution differential on
      packages, the WASM target and test mode`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 155eb61365fafa5e9f7426e5f5d75662c43f8c03..280fbbf67510b4184968530478df182cc4314bbf 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -39,6 +39,20 @@
 //! from fixed seeds, among which every resolver code `check` can give
 //! natively occurs. Every form of the dump appears in those dumps.
 //!
+//! 5.2b (spec §4; ADR 0054): S0's hidden dump also takes a package's
+//! module as a root (a file directly in a package's `src/` or `tests/`),
+//! `--target wasm`, and `--test`, `nova test`'s resolution; std's WASM and
+//! test-mode parts, the corpus's packages, its WASM programs and its files
+//! with tests, the cases of each mode (`cases/resolve-wasm/`,
+//! `cases/resolve-test/`) and the name fuzzer's package, WASM and
+//! test-mode inputs go through it, among which every one of the resolver's
+//! 11 codes, E0004 and E0001's two notes occur. N's `selfhost resolve`
+//! takes the same roots, `--target wasm` and `--test`, and must print the
+//! same dump of each, byte for byte, with `\` read as `/` in the paths of
+//! E0004's and E0093's messages on Windows; a manifest N's minimal reader
+//! refuses stops it, with a message, and no input of the differential has
+//! one.
+//!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
 //!
@@ -60,7 +74,7 @@ use std::sync::OnceLock;
 use std::thread::JoinHandle;
 use std::time::{Duration, Instant};
 
-use compare::{first_difference, report, Dump, AST, TOKENS};
+use compare::{first_difference, report, settle_paths, Dump, AST, TOKENS};
 use fuzz::SplitMix64;
 
 /// How many files one process lexes.
@@ -489,7 +503,10 @@ fn batch_name(program: &Path, args: &[&str], files: &[String]) -> String {
     )
 }
 
-/// `program` with `args` on `files`, within [`CHILD_LIMIT`].
+/// `program` with `args` on `files`, within [`CHILD_LIMIT`]. A resolution
+/// dump's stdout, S0's or N's, is settled here ([`settled_stdout`]), so
+/// every comparison of the two sides that runs them through `run` takes
+/// it settled.
 fn run(program: &Path, args: &[&str], dir: &Path, files: &[String]) -> Result<Run, String> {
     let out = output_within(
         Command::new(program)
@@ -501,18 +518,32 @@ fn run(program: &Path, args: &[&str], dir: &Path, files: &[String]) -> Result<Ru
         &batch_name(program, args, files),
     )?;
     Ok(Run {
-        stdout: out.stdout,
+        stdout: settled_stdout(args, out.stdout, cfg!(windows)),
         code: out.status.code(),
         stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
     })
 }
 
+/// `stdout` of a run with `args` as S0's and N's are compared: a
+/// resolution dump's, S0's `check --dump-resolution` or N's `resolve`,
+/// with `\` read as `/` in the paths of E0004's and E0093's messages on
+/// `windows` ([`settle_paths`], ADR 0054 §7); any other as it is.
+fn settled_stdout(args: &[&str], stdout: Vec<u8>, windows: bool) -> Vec<u8> {
+    if args.starts_with(RESOLUTION_S0) || args.first() == Some(&"resolve") {
+        settle_paths(&stdout, windows)
+    } else {
+        stdout
+    }
+}
+
 /// One batch, `files` relative to `dir`, through `nova` and `selfhost` with
 /// `dump`'s arguments: S0's output, or the report of where the two part,
 /// in their dumps or in their exit codes, both of which must be `code`. The
 /// report gives both exit codes and the start of anything either side
 /// wrote to stderr, so a side that crashed or printed nothing says so; a
 /// side that runs past the limit is killed and reported with the batch.
+/// A resolution dump's paths in E0004's and E0093's messages are compared
+/// with `\` read as `/` on Windows, as [`run`] settles them.
 fn compare_batch(
     dump: &Dump,
     dir: &Path,
@@ -669,24 +700,76 @@ fn crlf(text: Vec<u8>) -> Vec<u8> {
     out
 }
 
+/// The corpus's and the fixed cases' dumps, each compared once a run
+/// ([`agree`], from the repository): their files and S0's AST dump, which
+/// is N's. `the_corpus_dumps_agree`, `the_fixed_cases_agree` and
+/// `every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases` read the
+/// same dumps, so whichever test asks first compares them and the others
+/// wait for it (5.2b, ADR 0054 §9).
+struct Compared {
+    files: Vec<String>,
+    ast: String,
+}
+
+fn compared(files: Vec<String>) -> Compared {
+    let root = root();
+    agree_on(&TOKENS, &root, &files, BATCH, 0);
+    let ast = agree_on(&AST, &root, &files, BATCH, 0);
+    Compared { files, ast }
+}
+
+/// [`Compared`] of the corpus.
+fn corpus_compared() -> &'static Compared {
+    static DUMPS: OnceLock<Compared> = OnceLock::new();
+    DUMPS.get_or_init(|| compared(corpus()))
+}
+
+/// [`Compared`] of the fixed cases: every `.nova` file under [`cases_dir`].
+fn cases_compared() -> &'static Compared {
+    static DUMPS: OnceLock<Compared> = OnceLock::new();
+    DUMPS.get_or_init(|| {
+        let mut files = Vec::new();
+        nova_files(&root(), &cases_dir(), &[], &mut files);
+        compared(files)
+    })
+}
+
 #[test]
 fn the_corpus_dumps_agree() {
-    let files = corpus();
+    let compared = corpus_compared();
+    let files = &compared.files;
     // 281 under std/, tests/ and examples/ alone at 5.0c.
     assert!(files.len() >= 290, "only {} corpus files", files.len());
     assert!(files.iter().any(|f| f == "selfhost/src/lexer.nova"));
-    let dumps = agree(&root(), &files, BATCH, 0);
     // Each file's tree, so the AST dumps were compared too.
-    let trees = dumps.lines().filter(|l| l.starts_with("(File")).count();
+    let trees = compared
+        .ast
+        .lines()
+        .filter(|l| l.starts_with("(File"))
+        .count();
     assert_eq!(trees, files.len());
 }
 
 /// Both lexers must agree on the corpus as a Windows checkout has it
-/// ([`windows_corpus`]) on every system, not on Windows alone.
+/// ([`windows_corpus`]) on every system, not on Windows alone: on each
+/// file the checkout changes. On a Windows checkout, which already has
+/// each `\r\n`, the copy changes no file, and `the_corpus_dumps_agree`
+/// has compared those bytes (5.2b, ADR 0054 §9).
 #[test]
 fn the_corpus_dumps_agree_with_windows_line_endings() {
     let (dir, files) = windows_corpus("crlf");
-    let dumps = agree(&dir, &files, BATCH, 0);
+    let root = root();
+    let read = |path: PathBuf| std::fs::read(path).expect("read a corpus file");
+    let changed: Vec<String> = files
+        .into_iter()
+        .filter(|f| read(root.join(f)) != read(dir.join(f)))
+        .collect();
+    if changed.is_empty() {
+        let text = read(root.join("selfhost/src/lexer.nova"));
+        assert!(text.windows(2).any(|w| w == b"\r\n"), "no file changed");
+        return;
+    }
+    let dumps = agree(&dir, &changed, BATCH, 0);
     assert!(dumps.contains(" NL\n"), "the corpus has line breaks");
 }
 
@@ -714,15 +797,30 @@ fn the_mutations_are_the_same_with_windows_line_endings() {
             "seed {seed} name-mutates the Windows checkout otherwise"
         );
     }
+    // And its WASM and package siblings' (5.2b).
+    for seed in PACKAGE_NAME_SEEDS {
+        assert!(
+            package_names_of(&dir, &reversed, seed, PACKAGE_NAME_COUNT)
+                == package_names_of(&root(), &loose, seed, PACKAGE_NAME_COUNT),
+            "package seed {seed} name-mutates the Windows checkout otherwise"
+        );
+    }
+    let wasm = loose_wasm_corpus();
+    let mut reversed = wasm.clone();
+    reversed.reverse();
+    for seed in WASM_NAME_SEEDS {
+        assert!(
+            wasm_names_of(&dir, &reversed, seed, WASM_NAME_COUNT)
+                == wasm_names_of(&root(), &wasm, seed, WASM_NAME_COUNT),
+            "WASM seed {seed} name-mutates the Windows checkout otherwise"
+        );
+    }
 }
 
 #[test]
 fn the_fixed_cases_agree() {
-    let root = root();
-    let mut files = Vec::new();
-    nova_files(&root, &cases_dir(), &[], &mut files);
+    let files = &cases_compared().files;
     assert!(files.len() >= 10, "only {} fixed cases", files.len());
-    agree(&root, &files, BATCH, 0);
 }
 
 #[test]
@@ -831,13 +929,499 @@ fn unreadable_files_are_reported_alike() {
 }
 
 /// No file at all: both sides refuse it, print nothing on stdout and exit
-/// 2, for each dump: a resolution dump needs a root (or std's part).
+/// 2, for each dump: a resolution dump needs a root (or std's part), under
+/// every mode. Test mode for the WASM target is refused alike, with a root
+/// or std's part (5.2b, ADR 0054): `nova test` has no target.
 #[test]
 fn both_sides_refuse_no_file() {
-    for dump in [&TOKENS, &AST, resolution()] {
+    let wasm_test = Dump {
+        s0: leak_args(&[RESOLUTION_S0, &["--test", "--target", "wasm"]].concat()),
+        n: leak_args(&["resolve", "--std", &std_dir(), "--test", "--target", "wasm"]),
+    };
+    let wasm_test_std = Dump {
+        s0: leak_args(&[wasm_test.s0, &["--std"]].concat()),
+        n: leak_args(&[wasm_test.n, &["--std-part"]].concat()),
+    };
+    for dump in [
+        &TOKENS,
+        &AST,
+        resolution(),
+        resolution_in(Mode::Wasm),
+        resolution_in(Mode::Test),
+    ] {
         let out = compare_batch(dump, &root(), &[], 2, &nova(), selfhost());
         assert_eq!(out, Ok(String::new()));
     }
+    let main = ["tests/runtime/array_helpers.nova".to_string()];
+    assert!(root().join(&main[0]).is_file());
+    for (dump, files) in [(&wasm_test, &main[..]), (&wasm_test_std, &[])] {
+        let out = compare_batch(dump, &root(), files, 2, &nova(), selfhost());
+        assert_eq!(out, Ok(String::new()));
+    }
+}
+
+/// Both hidden forms take `--test` wherever an option may stand, and agree
+/// on what each shape gives, stdout and exit code (5.2b, ADR 0054 §1):
+/// before or after a root, beside an empty root (not found), twice
+/// (refused), and with std's part for the WASM target in either order
+/// (refused, as `nova test` has no target), and with a value (refused, as
+/// clap refuses one for a flag); `--target VALUE` and `--target=VALUE`, as
+/// clap takes them, an empty, repeated (either way round) or unknown value
+/// refused. `--std` is N's
+/// `--std-part`. A `--` is left out: N reads it as a root of that name, where S0's form
+/// skips it, and the harness passes none (ADR 0053 §5).
+#[test]
+fn both_sides_take_test_in_every_position_alike() {
+    let file = "tests/runtime/array_helpers.nova";
+    assert!(root().join(file).is_file());
+    let std = std_dir();
+    let shapes: [(&[&str], i32); 15] = [
+        (&["--test", file], 0),
+        (&[file, "--test"], 0),
+        (&["--test", ""], 2),
+        (&["--test", "--test", file], 2),
+        (&["--test=true", file], 2),
+        (&["--target", "", file], 2),
+        (&["--target", "", "--target", "wasm", file], 2),
+        (&["--target", "wasm", "--target=native", file], 2),
+        (&["--target", "wasm", "--target", "native", file], 2),
+        (&["--test", "--std", "--target", "wasm"], 2),
+        (&["--target", "wasm", "--std", "--test"], 2),
+        (&["--target=wasm", "--std"], 0),
+        (&["--target=", file], 2),
+        (&["--target=wasm", "--target", "wasm", "--std"], 2),
+        (&["--target=bogus", file], 2),
+    ];
+    for (args, code) in shapes {
+        let n: Vec<&str> = ["resolve", "--std", std.as_str()]
+            .into_iter()
+            .chain(
+                args.iter()
+                    .map(|a| if *a == "--std" { "--std-part" } else { a }),
+            )
+            .collect();
+        let dump = Dump {
+            s0: leak_args(&[RESOLUTION_S0, args].concat()),
+            n: leak_args(&n),
+        };
+        if let Err(report) = compare_batch(&dump, &root(), &[], code, &nova(), selfhost()) {
+            panic!("{args:?}: {report}");
+        }
+    }
+}
+
+/// A root whose package's manifest N's minimal reader refuses (ADR 0054
+/// §5): S0 dumps it with the package graph's M-code, an error stopping
+/// it before the loader, and N, which gives no M-code, prints nothing of
+/// it, says why on stderr, naming the manifest, and exits 1. No input
+/// of the differential has such a manifest; this pins what N does
+/// instead of printing a partial dump.
+#[test]
+fn a_manifest_n_cannot_read_stops_n_with_a_message() {
+    let dir = fresh_dir("resolution-refused");
+    for (name, extra) in [
+        ("unknown", "color = \"red\"\n"),
+        ("registry", "[dependencies]\njson = \"1.0\"\n"),
+    ] {
+        let pkg = dir.join(name);
+        std::fs::create_dir_all(pkg.join("src")).expect("create src/");
+        std::fs::write(pkg.join("nova.toml"), format!("{MANIFEST}{extra}"))
+            .expect("write nova.toml");
+        std::fs::write(pkg.join("src").join("main.nova"), "fn main() {}\n")
+            .expect("write main.nova");
+        let root = format!("{name}/src/main.nova");
+        let s0 = Command::new(nova())
+            .current_dir(&dir)
+            .args(RESOLUTION_S0)
+            .arg(&root)
+            .output()
+            .expect("run nova check --dump-resolution");
+        let stdout = String::from_utf8_lossy(&s0.stdout);
+        assert_eq!(s0.status.code(), Some(0), "{name}: {stdout}");
+        assert!(stdout.contains("\"M00"), "{name}: {stdout}");
+        // An error among the graph's M-codes stops S0 before the loader,
+        // with no module; the unknown key's warning does not.
+        let stopped = format!("== {root}\n(Program target=native resolved=false)\n(Diagnostics\n");
+        assert_eq!(
+            stdout.starts_with(&stopped),
+            name == "registry",
+            "{name}: {stdout}"
+        );
+        let n = Command::new(selfhost())
+            .current_dir(&dir)
+            .args(resolution().n)
+            .arg(&root)
+            .output()
+            .expect("run selfhost resolve");
+        let stderr = String::from_utf8_lossy(&n.stderr);
+        assert_eq!(n.status.code(), Some(1), "{name}: {stderr}");
+        assert!(n.stdout.is_empty(), "{name}");
+        assert!(
+            stderr.starts_with(&format!(
+                "error: selfhost resolve cannot dump {root}: {name}/nova.toml: "
+            )),
+            "{name}: {stderr}"
+        );
+    }
+}
+
+/// A copy of std's sources in a fresh directory named for `name`.
+fn std_copy(name: &str) -> PathBuf {
+    let std = fresh_dir(name);
+    let from = root().join("std");
+    let mut sources = Vec::new();
+    nova_files(&from, &from, &[], &mut sources);
+    for rel in &sources {
+        let to = std.join(rel);
+        std::fs::create_dir_all(to.parent().expect("a parent")).expect("create a std directory");
+        std::fs::copy(from.join(rel), &to).expect("copy a std file");
+    }
+    std
+}
+
+/// A trait that an imported std module exports reaches each program that
+/// shares std through std's exports rebased past the program's own ids
+/// (ADR 0054 §9), as a function and a type do; std's `ui`, which a root
+/// may import, exports none, so a copy of std whose `ui` ends with a
+/// `pub trait` stands in: two loose roots of one and of three definitions
+/// import `ui`, and N's dump of each lists the trait in its scope with
+/// the id N's std part gives it, and is otherwise S0's, whose std is
+/// built in and lacks it (the trait is std's last definition, so no
+/// other id moves).
+#[test]
+fn a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program() {
+    let std = std_copy("trait-std");
+    let ui = std.join("ui").join("lib.nova");
+    let text = std::fs::read_to_string(&ui).expect("read ui");
+    std::fs::write(&ui, format!("{text}\npub trait Shade {{}}\n")).expect("alter ui");
+    let std = std.display().to_string();
+    let dir = fresh_dir("trait-roots");
+    let roots = ["a.nova".to_string(), "b.nova".to_string()];
+    std::fs::write(dir.join(&roots[0]), "import ui\n\nfn main() {}\n").expect("write a.nova");
+    std::fs::write(
+        dir.join(&roots[1]),
+        "import ui\n\nrecord R { n: Int }\nrecord Q { n: Int }\n\nfn main() {}\n",
+    )
+    .expect("write b.nova");
+    let part = run(
+        selfhost(),
+        &["resolve", "--std", &std, "--std-part"],
+        &dir,
+        &[],
+    )
+    .expect("run N's std part");
+    let part = String::from_utf8_lossy(&part.stdout);
+    let id = part
+        .split("(Def.Trait ")
+        .find_map(|d| {
+            d.split_once(' ')
+                .filter(|(_, rest)| rest.starts_with("\"Shade\" "))
+                .map(|(id, _)| id.to_string())
+        })
+        .unwrap_or_else(|| panic!("no Shade in std's part: {part}"));
+    let line = format!("    (Trait \"Shade\" {id})\n");
+    let s0 = run(&nova(), RESOLUTION_S0, &dir, &roots).expect("run S0");
+    let n = run(selfhost(), &["resolve", "--std", &std], &dir, &roots).expect("run N");
+    assert_eq!((s0.code, n.code), (Some(0), Some(0)), "{}", n.stderr);
+    let n = String::from_utf8_lossy(&n.stdout);
+    for part in parts_of(&n) {
+        assert_eq!(part.matches(&line).count(), 1, "{line}{part}");
+    }
+    assert_eq!(n.replace(&line, ""), String::from_utf8_lossy(&s0.stdout));
+}
+
+/// N shares std resolved once only when std alone gives no diagnostic and
+/// holds no `@test` function, either of which the shared path would drop
+/// (ADR 0054 §9): with a std whose `core` defines a name twice, or whose
+/// `ui` marks a function `@test` (under test mode), N resolves std again
+/// for each program, so each program that reaches the resolver keeps
+/// std's E0002, or lists std's test, and its dump is otherwise S0's, whose
+/// std is built in (nova-resolver's own test pins S0's side); the rest
+/// are S0's. The roots are the first six loose programs and the first four
+/// package roots of the mode's cases.
+#[test]
+fn n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test() {
+    let alterations = [
+        (
+            Mode::Check,
+            "core/lib.nova",
+            "fn mix64(x: Int) -> Int {",
+            "fn escape_common(x: Int) -> Int {",
+            "(Error \"E0002\" \"duplicate definition of `escape_common`\"",
+        ),
+        (
+            Mode::Test,
+            "ui/lib.nova",
+            "\nfn flush() {",
+            "\n@test\nfn flush() {",
+            " \"flush\" should_panic=false)",
+        ),
+    ];
+    std::thread::scope(|scope| {
+        let runs: Vec<_> = alterations
+            .into_iter()
+            .enumerate()
+            .map(|(k, (mode, file, old, new, mark))| {
+                scope.spawn(move || {
+                    // A copy of std's sources, `file` altered.
+                    let std = std_copy(&format!("altered-std-{k}"));
+                    let text = std::fs::read_to_string(std.join(file))
+                        .expect("read the std file")
+                        .replace("\r\n", "\n");
+                    assert_eq!(text.matches(old).count(), 1, "{file}: {old}");
+                    std::fs::write(std.join(file), text.replacen(old, new, 1))
+                        .expect("alter the std file");
+                    let cases = mode_cases_dir(mode);
+                    let found = case_roots(&cases);
+                    let roots: Vec<String> = found.loose[..6]
+                        .iter()
+                        .chain(&found.packages[..4])
+                        .cloned()
+                        .collect();
+                    let std = std.display().to_string();
+                    let n_args: Vec<&str> = ["resolve", "--std", std.as_str()]
+                        .into_iter()
+                        .chain(mode.flags().iter().copied())
+                        .collect();
+                    let s0 = run(&nova(), resolution_in(mode).s0, &cases, &roots).expect("run S0");
+                    let n = run(selfhost(), &n_args, &cases, &roots).expect("run N");
+                    assert_eq!((s0.code, n.code), (Some(0), Some(0)), "{}", n.stderr);
+                    let (s0, n) = (
+                        String::from_utf8_lossy(&s0.stdout).into_owned(),
+                        String::from_utf8_lossy(&n.stdout).into_owned(),
+                    );
+                    let (s0_parts, n_parts) = (parts_of(&s0), parts_of(&n));
+                    assert_eq!(s0_parts.len(), roots.len(), "{s0}");
+                    assert_eq!(n_parts.len(), roots.len(), "{n}");
+                    let mut resolved = 0;
+                    for (s0_part, n_part) in s0_parts.iter().zip(&n_parts) {
+                        if !s0_part.contains(" resolved=true\n") {
+                            assert_eq!(n_part, s0_part);
+                            continue;
+                        }
+                        resolved += 1;
+                        assert_eq!(n_part.matches(mark).count(), 1, "{file}: {n_part}");
+                        assert_eq!(
+                            without_parens(&without_block(n_part, mark)),
+                            without_parens(s0_part),
+                            "{file}: {n_part}"
+                        );
+                    }
+                    assert!(resolved >= 6, "{resolved} of {} resolved", roots.len());
+                })
+            })
+            .collect();
+        joined(runs);
+    });
+}
+
+/// `part` without the line holding `mark` and the lines below it indented
+/// further, a diagnostic and its labels or a test's line.
+fn without_block(part: &str, mark: &str) -> String {
+    let indent = |l: &str| l.len() - l.trim_start().len();
+    let mut out = String::new();
+    let mut inside: Option<usize> = None;
+    for line in part.split_inclusive('\n') {
+        if let Some(depth) = inside {
+            if indent(line) > depth {
+                continue;
+            }
+            inside = None;
+        }
+        if line.contains(mark) {
+            inside = Some(indent(line));
+            continue;
+        }
+        out.push_str(line);
+    }
+    out
+}
+
+/// `text` with the `)`s that end each line taken off, so that a part
+/// with a line taken out compares with one that never had it.
+fn without_parens(text: &str) -> String {
+    text.lines()
+        .map(|l| l.trim_end_matches(')'))
+        .collect::<Vec<_>>()
+        .join("\n")
+}
+
+/// A package's identity (ADR 0054 §5), in a directory named `run`: `app`
+/// depends on `../geom` and `../b`, and `b` on `geom` by its absolute
+/// path; `app3` on `geom` by its absolute path and on `../b3`, and `b3` on
+/// `../geom`, so the absolute path comes first; `up` on `../geom` and
+/// `../climb`, and `climb` on `../../run/geom`, which climbs above the
+/// current directory and comes back down; `up2` on `../../run/geom` and
+/// on `../b3`, so the climbing path comes first. Under a root given by its
+/// absolute path both sides read one `geom` in each. N cannot name the
+/// current directory, so under a relative root it refuses each graph as
+/// M0011, saying the two paths may name one directory, where S0, which
+/// reads the real path, resolves it; and through a link to `geom` N
+/// refuses the graph as M0011 under any root, where S0 resolves it. No
+/// other input of the differential holds an absolute dependency path, a
+/// path climbing above the current directory or a link; on Windows N's
+/// paths read no drive, which 5.5 takes up with a root written with `\`,
+/// so the test is Unix's.
+#[cfg(unix)]
+#[test]
+fn a_directory_spelled_two_ways_is_one_package_under_an_absolute_root() {
+    let dir = fresh_dir("resolution-identity").join("run");
+    let geom = dir.join("geom").display().to_string();
+    let manifest = |name: &str, deps: &[(&str, &str)]| {
+        let mut text = MANIFEST.replace("pkg", name);
+        if !deps.is_empty() {
+            text.push_str("[dependencies]\n");
+        }
+        for (dep, path) in deps {
+            text.push_str(&format!("{dep} = {{ path = \"{path}\" }}\n"));
+        }
+        text
+    };
+    let main = |deps: &[&str]| {
+        let imports: String = deps.iter().map(|d| format!("import {d}\n")).collect();
+        format!("{imports}\nfn main() {{}}\n")
+    };
+    let lib = |name: &str| format!("import geom\n\npub fn {name}() -> Int {{ area() }}\n");
+    let files = [
+        (
+            "app/nova.toml",
+            manifest("app", &[("geom", "../geom"), ("b", "../b")]),
+        ),
+        ("app/src/main.nova", main(&["geom", "b"])),
+        (
+            "app3/nova.toml",
+            manifest("app3", &[("geom", &geom), ("b3", "../b3")]),
+        ),
+        ("app3/src/main.nova", main(&["geom", "b3"])),
+        (
+            "up/nova.toml",
+            manifest("up", &[("geom", "../geom"), ("climb", "../climb")]),
+        ),
+        ("up/src/main.nova", main(&["geom", "climb"])),
+        (
+            "up2/nova.toml",
+            manifest("up2", &[("geom", "../../run/geom"), ("b3", "../b3")]),
+        ),
+        ("up2/src/main.nova", main(&["geom", "b3"])),
+        ("geom/nova.toml", manifest("geom", &[])),
+        (
+            "geom/src/lib.nova",
+            "pub fn area() -> Int { 1 }\n".to_string(),
+        ),
+        ("b/nova.toml", manifest("b", &[("geom", &geom)])),
+        ("b/src/lib.nova", lib("twice")),
+        ("b3/nova.toml", manifest("b3", &[("geom", "../geom")])),
+        ("b3/src/lib.nova", lib("thrice")),
+        (
+            "climb/nova.toml",
+            manifest("climb", &[("geom", "../../run/geom")]),
+        ),
+        ("climb/src/lib.nova", lib("climb")),
+        (
+            "linked/nova.toml",
+            manifest("linked", &[("geom", "../geom"), ("c", "../c")]),
+        ),
+        ("linked/src/main.nova", main(&["geom", "c"])),
+        ("c/nova.toml", manifest("c", &[("geom", "../link")])),
+        ("c/src/lib.nova", lib("c")),
+    ];
+    for (path, text) in &files {
+        let path = dir.join(path);
+        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
+        std::fs::write(path, text).expect("write a package's file");
+    }
+    std::os::unix::fs::symlink("geom", dir.join("link")).expect("link geom");
+    let absolute = |root: &str| dir.join(root).display().to_string();
+    let roots: Vec<String> = ["app", "app3", "up", "up2"]
+        .iter()
+        .map(|p| absolute(&format!("{p}/src/main.nova")))
+        .collect();
+    let out = agree_on_resolution(&dir, &roots, 0);
+    assert_eq!(parts_of(&out).len(), roots.len(), "{out}");
+    for (root, part) in roots.iter().zip(parts_of(&out)) {
+        assert!(
+            part.starts_with(&format!("{root}\n(Program target=native resolved=true\n")),
+            "{part}"
+        );
+        assert_eq!(part.matches("(Module ").count(), 3, "{part}");
+    }
+    let refused = |root: &str, why: &str| {
+        let s0 = run(&nova(), RESOLUTION_S0, &dir, &[root.to_string()]).expect("run S0");
+        let s0_out = String::from_utf8_lossy(&s0.stdout);
+        assert_eq!(s0.code, Some(0), "{root}: {s0_out}");
+        assert!(s0_out.contains(" resolved=true\n"), "{root}: {s0_out}");
+        assert_eq!(s0_out.matches("(Module ").count(), 3, "{root}: {s0_out}");
+        let n = run(selfhost(), resolution().n, &dir, &[root.to_string()]).expect("run N");
+        assert_eq!(n.code, Some(1), "{root}: {}", n.stderr);
+        assert!(n.stdout.is_empty(), "{root}");
+        assert_eq!(
+            n.stderr,
+            format!(
+                "error: selfhost resolve cannot dump {root}: two packages named `geom` in one \
+                 build (M0011){why}\n"
+            )
+        );
+    };
+    let or_one = ", or one directory spelled by two paths, which N cannot tell apart without \
+                  the current directory";
+    for root in ["app", "app3", "up", "up2"] {
+        refused(&format!("{root}/src/main.nova"), or_one);
+    }
+    refused(&absolute("linked/src/main.nova"), "");
+}
+
+/// On Windows a resolution dump's E0004 and E0093 messages are compared
+/// with `\` read as `/` ([`settle_paths`], ADR 0054 §7, and decision 4 of
+/// its §8), and nothing else is; elsewhere nothing is changed.
+#[test]
+fn the_paths_of_e0004_and_e0093_are_settled_on_windows_alone() {
+    let dump =
+        "(Diagnostics\n  (Error \"E0004\" \"`a` is both: app\\x5csrc\\x5ca.nova, and the entry at \
+                app\\x5cnova.toml:6\"\n    (Primary \"a\\x5cb\":0..1 \"x\"))\n  \
+                (Error \"E0093\" \"rename sub\\x5cui.nova\"\n  (Error \"E0001\" \"a\\x5cb\"\n";
+    let settled =
+        "(Diagnostics\n  (Error \"E0004\" \"`a` is both: app/src/a.nova, and the entry at \
+                   app/nova.toml:6\"\n    (Primary \"a\\x5cb\":0..1 \"x\"))\n  \
+                   (Error \"E0093\" \"rename sub/ui.nova\"\n  (Error \"E0001\" \"a\\x5cb\"\n";
+    assert_eq!(settle_paths(dump.as_bytes(), true), settled.as_bytes());
+    assert_eq!(settle_paths(dump.as_bytes(), false), dump.as_bytes());
+}
+
+/// The fault CI's Windows leg found in 5.2b (ADR 0054 §7): a test
+/// compared S0's and N's resolution dumps of E0004 without settling its
+/// paths, which S0 writes with `\` on Windows. [`run`] settles every
+/// resolution dump, S0's and N's, so no comparison built on it can miss
+/// that: here S0's dump of a dependency's E0004, each `/` of its message
+/// written as Windows writes it, differs from N's as it is and equals it
+/// settled as `run` settles it on Windows, under S0's arguments and N's;
+/// a dump that is not a resolution dump is kept as it is.
+#[test]
+fn every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows() {
+    let cases = mode_cases_dir(Mode::Check);
+    let root = ["a_dependencys_e0004_names_its_own_manifest/src/main.nova".to_string()];
+    let s0 = run(&nova(), RESOLUTION_S0, &cases, &root).expect("run S0");
+    let n = run(selfhost(), resolution().n, &cases, &root).expect("run N");
+    assert_eq!((s0.code, n.code), (Some(0), Some(0)), "{}", n.stderr);
+    assert_eq!(s0.stdout, n.stdout);
+    let text = String::from_utf8_lossy(&s0.stdout);
+    let windows: String = text
+        .split_inclusive('\n')
+        .map(|line| {
+            if line.starts_with("  (Error \"E0004\" ") {
+                line.replace('/', "\\x5c")
+            } else {
+                line.to_string()
+            }
+        })
+        .collect();
+    assert!(windows.contains("geo\\x5cnova.toml:7"), "{windows}");
+    assert_ne!(windows.as_bytes(), &n.stdout[..]);
+    for args in [RESOLUTION_S0, resolution().n] {
+        let settled = settled_stdout(args, windows.clone().into_bytes(), true);
+        assert_eq!(settled, n.stdout, "{args:?}");
+    }
+    let kept = settled_stdout(TOKENS.s0, windows.clone().into_bytes(), true);
+    assert_eq!(kept, windows.as_bytes());
 }
 
 #[test]
@@ -1126,34 +1710,25 @@ fn kinds_on(line: &str, found: &mut std::collections::HashSet<String>) {
 }
 
 /// Every kind of node, field-less value and literal the dump can print
-/// appears in S0's dump of some corpus file or fixed case, and in N's
-/// (spec §4.3): it follows from the dumps' being equal, which
-/// `the_corpus_dumps_agree` and `the_fixed_cases_agree` check, and each
-/// side is checked on its own all the same.
+/// appears in the AST dump of some corpus file or fixed case (spec §4.3),
+/// which both sides print byte for byte: `the_corpus_dumps_agree` and
+/// `the_fixed_cases_agree` compare the same dumps ([`Compared`]), so each
+/// side's holds every kind. 5.1 ran both sides over the files again here;
+/// from 5.2b the dumps compared are read (ADR 0054 §9).
 #[test]
 fn every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases() {
-    let root = root();
-    let mut files = corpus();
-    nova_files(&root, &cases_dir(), &[], &mut files);
-    for (side, program, args) in [("S0", nova(), AST.s0), ("N", selfhost().into(), AST.n)] {
-        let mut found = std::collections::HashSet::new();
-        for chunk in files.chunks(BATCH) {
-            let out = run(&program, args, &root, chunk).unwrap_or_else(|e| panic!("{e}"));
-            assert_eq!(out.code, Some(0), "{side}: {}", out.stderr);
-            for line in String::from_utf8_lossy(&out.stdout).lines() {
-                kinds_on(line, &mut found);
-            }
+    let mut found = std::collections::HashSet::new();
+    for compared in [corpus_compared(), cases_compared()] {
+        for line in compared.ast.lines() {
+            kinds_on(line, &mut found);
         }
-        let missing: Vec<&str> = nova_ast::dump::KINDS
-            .iter()
-            .copied()
-            .filter(|k| !found.contains(*k))
-            .collect();
-        assert!(
-            missing.is_empty(),
-            "no file's dump by {side} holds {missing:?}"
-        );
     }
+    let missing: Vec<&str> = nova_ast::dump::KINDS
+        .iter()
+        .copied()
+        .filter(|k| !found.contains(*k))
+        .collect();
+    assert!(missing.is_empty(), "no file's dump holds {missing:?}");
 }
 
 #[test]
@@ -1430,35 +2005,106 @@ const RESOLUTION_S0: &[&str] = &["check", "--dump-resolution"];
 /// nothing else: the tests' packages (5.2b, ADR 0054).
 const MANIFEST: &str = "[package]\nname = \"pkg\"\nversion = \"0.1.0\"\nedition = \"2026\"\n";
 
-/// The error cases of the resolver (spec §3.5): the inline sources of
+/// What a program is resolved under (5.2b, ADR 0054): `check`'s pipeline
+/// natively, `check`'s for the WASM target, or `nova test`'s.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+enum Mode {
+    Check,
+    Wasm,
+    Test,
+}
+
+impl Mode {
+    /// The arguments for the mode after [`RESOLUTION_S0`] on S0's side, and
+    /// after `resolve --std DIR` on N's: the same on both.
+    fn flags(self) -> &'static [&'static str] {
+        match self {
+            Mode::Check => &[],
+            Mode::Wasm => &["--target", "wasm"],
+            Mode::Test => &["--test"],
+        }
+    }
+}
+
+/// The error cases of the resolver (spec §3.5, §4): the inline sources of
 /// `nova-resolver`'s tests and of the type checker's tests that reach the
 /// resolver, each a file, or a directory whose `main.nova` is the root of
-/// a program of several modules.
-fn resolve_cases_dir() -> PathBuf {
-    cases_dir().join("resolve")
-}
-
-/// [`agree_on_resolution`] the resolver's cases in `cases`
-/// ([`resolve_cases_dir`], or a copy of it), from `cases`: each file in it,
-/// and each directory's `main.nova`, [`BATCH`] roots to a process; but a
-/// directory holding a module named for a std module (`dom.nova`,
-/// `ui.nova`, `RESERVED_MODULE_NAMES`) from inside it, its root named
-/// `main.nova`. E0093 names that module by its path, the root's directory
-/// joined with its file name: S0 joins with `Path::join`, which writes `\`
-/// on Windows, and N with `/`, so the path must have no directory part
-/// (ADR 0053 §5). Returns S0's output and how many programs it holds.
-fn agree_on_resolve_cases(cases: &Path) -> (String, usize) {
+/// a program of several modules; and, from 5.2b (ADR 0054), the programs
+/// the driver's, `run_tests.rs`'s and `wasm_tests.rs`'s tests resolve as
+/// packages, for the WASM target or under test mode, packages each a
+/// directory holding a `nova.toml` ([`case_roots`]). They are where the
+/// mode they are resolved under puts them: `cases/resolve/` under `check`,
+/// `cases/resolve-wasm/` for the WASM target, and `cases/resolve-test/`
+/// under test mode. The directory names the mode, so a case needs no
+/// marker of its own.
+fn mode_cases_dir(mode: Mode) -> PathBuf {
+    cases_dir().join(mode_cases_name(mode))
+}
+
+/// [`mode_cases_dir`]'s last component.
+fn mode_cases_name(mode: Mode) -> &'static str {
+    match mode {
+        Mode::Check => "resolve",
+        Mode::Wasm => "resolve-wasm",
+        Mode::Test => "resolve-test",
+    }
+}
+
+/// The programs of a directory of cases, each a root named from that
+/// directory but those in `apart` (ADR 0053 §9, ADR 0054):
+/// - a `.nova` file is a program;
+/// - a directory holding a `nova.toml` is a package, and each `.nova` file
+///   directly in its `src/`, then in its `tests/`, sorted, is a program's
+///   root, a module of that package (`packages`);
+/// - any other directory is a loose program whose root is its `main.nova`,
+///   but one holding a module named for a std module (`dom.nova`,
+///   `ui.nova`, `RESERVED_MODULE_NAMES`) runs from inside it, its root
+///   named `main.nova` (`apart`): E0093 names that module by its path, the
+///   root's directory joined with its file name, which S0 writes with `\`
+///   on Windows and N with `/`, so the path must have no directory part
+///   (ADR 0053 §5).
+struct CaseRoots {
+    loose: Vec<String>,
+    apart: Vec<PathBuf>,
+    packages: Vec<String>,
+}
+
+fn case_roots(cases: &Path) -> CaseRoots {
     let mut entries: Vec<_> = std::fs::read_dir(cases)
         .expect("read the resolver's cases")
         .map(|e| e.expect("a directory entry"))
         .collect();
     entries.sort_by_key(|e| e.file_name());
-    let mut roots = Vec::new();
-    let mut apart = Vec::new();
+    let mut roots = CaseRoots {
+        loose: Vec::new(),
+        apart: Vec::new(),
+        packages: Vec::new(),
+    };
     for e in entries {
         let path = e.path();
         let name = e.file_name().to_string_lossy().into_owned();
-        if path.is_dir() {
+        if path.is_dir() && path.join("nova.toml").is_file() {
+            let before = roots.packages.len();
+            for sub in ["src", "tests"] {
+                let Ok(dir) = std::fs::read_dir(path.join(sub)) else {
+                    continue;
+                };
+                let mut files: Vec<String> = dir
+                    .map(|e| e.expect("a directory entry").file_name())
+                    .map(|n| n.to_string_lossy().into_owned())
+                    .filter(|n| n.ends_with(".nova"))
+                    .collect();
+                files.sort();
+                roots
+                    .packages
+                    .extend(files.into_iter().map(|f| format!("{name}/{sub}/{f}")));
+            }
+            assert!(
+                roots.packages.len() > before,
+                "{} has no root",
+                path.display()
+            );
+        } else if path.is_dir() {
             assert!(
                 path.join("main.nova").is_file(),
                 "{} has no main.nova",
@@ -1468,61 +2114,130 @@ fn agree_on_resolve_cases(cases: &Path) -> (String, usize) {
                 .iter()
                 .any(|std| path.join(format!("{std}.nova")).is_file())
             {
-                apart.push(path);
+                roots.apart.push(path);
             } else {
-                roots.push(format!("{name}/main.nova"));
+                roots.loose.push(format!("{name}/main.nova"));
             }
         } else if name.ends_with(".nova") {
-            roots.push(name);
+            roots.loose.push(name);
         }
     }
-    let mut all = agree_on_resolution(cases, &roots, 0);
-    for dir in &apart {
-        all.push_str(&agree_on_resolution(dir, &["main.nova".to_string()], 0));
+    roots
+}
+
+/// [`agree_on_resolution_in`] `mode` on the resolver's cases in `cases`
+/// ([`mode_cases_dir`], or a copy of it) ([`case_roots`]): the loose
+/// programs and the packages' roots from `cases`, in one process a side,
+/// then each program apart from its own directory. Returns S0's output,
+/// which is N's, and how many programs it holds.
+fn agree_on_resolve_cases(mode: Mode, cases: &Path) -> (String, usize) {
+    let roots = case_roots(cases);
+    let count = roots.loose.len() + roots.apart.len() + roots.packages.len();
+    let from_cases: Vec<String> = roots.loose.into_iter().chain(roots.packages).collect();
+    let mut all = agree_on_resolution_in(mode, cases, &from_cases, 0);
+    for dir in &roots.apart {
+        all.push_str(&agree_on_resolution_in(
+            mode,
+            dir,
+            &["main.nova".to_string()],
+            0,
+        ));
     }
-    (all, roots.len() + apart.len())
+    (all, count)
 }
 
-/// The programs of the corpus that the resolution dump compares in 5.2a:
-/// every corpus file but those directly in a package's `src/` or
-/// `tests/`, which are 5.2b's.
+/// Whether `file`, a corpus file relative to the repository, is directly
+/// in a package's `src/` or `tests/` (`nova_driver::package_of`).
+fn in_package(file: &str) -> bool {
+    let path = root().join(file);
+    let parent = path.parent().expect("a parent");
+    matches!(
+        parent.file_name().and_then(|n| n.to_str()),
+        Some("src" | "tests")
+    ) && parent
+        .parent()
+        .is_some_and(|p| p.join("nova.toml").is_file())
+}
+
+/// The loose programs of the corpus: every corpus file but those directly
+/// in a package's `src/` or `tests/` ([`package_corpus`]), each a file run
+/// alone or a root with the sibling files it imports.
 fn loose_corpus() -> Vec<String> {
+    corpus().into_iter().filter(|f| !in_package(f)).collect()
+}
+
+/// The corpus files directly in a package's `src/` or `tests/` (5.2b):
+/// each a root, a module of its package.
+fn package_corpus() -> Vec<String> {
+    corpus().into_iter().filter(|f| in_package(f)).collect()
+}
+
+/// The corpus files resolved for the WASM target (spec §4.2): those under
+/// `tests/dom/` and `tests/wasm/`, packages included.
+fn wasm_corpus() -> Vec<String> {
+    corpus()
+        .into_iter()
+        .filter(|f| f.starts_with("tests/dom/") || f.starts_with("tests/wasm/"))
+        .collect()
+}
+
+/// The corpus files resolved under test mode (spec §4.3): those that hold
+/// a line beginning with `@test`, after its indentation.
+fn test_corpus() -> Vec<String> {
     let root = root();
     corpus()
         .into_iter()
         .filter(|f| {
-            let path = root.join(f);
-            let parent = path.parent().expect("a parent");
-            let in_package = matches!(
-                parent.file_name().and_then(|n| n.to_str()),
-                Some("src" | "tests")
-            ) && parent
-                .parent()
-                .is_some_and(|p| p.join("nova.toml").is_file());
-            !in_package
+            let text = std::fs::read_to_string(root.join(f)).expect("read a corpus file");
+            text.lines().any(|l| l.trim_start().starts_with("@test"))
         })
         .collect()
 }
 
-/// The resolution dump through both sides (ADR 0053): S0's `check
-/// --dump-resolution`, and N's `resolve --std DIR`, DIR the repository's
-/// `std/`, each followed by the roots.
+/// The resolution dump through both sides under `check` (ADR 0053):
+/// [`resolution_in`] [`Mode::Check`].
 fn resolution() -> &'static Dump {
-    static DUMP: OnceLock<Dump> = OnceLock::new();
-    DUMP.get_or_init(|| Dump {
-        s0: RESOLUTION_S0,
-        n: leak_args(&["resolve", "--std", &std_dir()]),
-    })
+    resolution_in(Mode::Check)
 }
 
-/// std's part through both sides: S0's `check --dump-resolution --std`,
-/// and N's `resolve --std DIR --std-part`.
-fn std_resolution() -> &'static Dump {
-    static DUMP: OnceLock<Dump> = OnceLock::new();
-    DUMP.get_or_init(|| Dump {
-        s0: &["check", "--dump-resolution", "--std"],
-        n: leak_args(&["resolve", "--std", &std_dir(), "--std-part"]),
-    })
+/// The resolution dump through both sides under `mode` (ADR 0053, ADR
+/// 0054): S0's `check --dump-resolution`, and N's `resolve --std DIR`, DIR
+/// the repository's `std/`, each with the mode's flags and followed by the
+/// roots.
+fn resolution_in(mode: Mode) -> &'static Dump {
+    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
+    DUMPS[mode as usize].get_or_init(|| dump_with(mode, &[]))
+}
+
+/// std's part through both sides under `mode`: S0's `check
+/// --dump-resolution --std`, and N's `resolve --std DIR --std-part`, each
+/// with the mode's flags.
+fn std_resolution(mode: Mode) -> &'static Dump {
+    static DUMPS: [OnceLock<Dump>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
+    DUMPS[mode as usize].get_or_init(|| dump_with(mode, &["--std"]))
+}
+
+/// The resolution dump under `mode`, with `extra` after S0's flags: `--std`
+/// is N's `--std-part`, and any other is passed to both as it is.
+fn dump_with(mode: Mode, extra: &[&str]) -> Dump {
+    let s0: Vec<&str> = RESOLUTION_S0
+        .iter()
+        .chain(mode.flags())
+        .chain(extra)
+        .copied()
+        .collect();
+    let std_dir = std_dir();
+    let mut n = vec!["resolve", "--std", std_dir.as_str()];
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
 }
 
 /// The repository's `std/`, which N reads its std modules from.
@@ -1539,52 +2254,82 @@ fn leak_args(args: &[&str]) -> &'static [&'static str] {
     Box::leak(owned.into_boxed_slice())
 }
 
-/// [`agree_on`] the resolution dump: `files`, relative to `dir`, each a
-/// program's root, through both sides, [`BATCH`] roots to a process and
-/// the batches side by side, each side exiting with `code`; fail with the
-/// report of the first difference. Returns S0's output, which is N's.
-/// S0 parses std again for each program (about 0.065 s, debug) and N once
-/// per process (about 0.4 s with the debug runtime), so the batches run at
-/// once.
+/// [`agree_on_resolution_in`] [`Mode::Check`].
 fn agree_on_resolution(dir: &Path, files: &[String], code: i32) -> String {
-    let (nova, selfhost) = (nova(), selfhost());
-    let outs: Vec<Result<String, String>> = std::thread::scope(|scope| {
-        let runs: Vec<_> = files
-            .chunks(BATCH)
-            .map(|chunk| {
-                let nova = &nova;
-                scope.spawn(move || compare_batch(resolution(), dir, chunk, code, nova, selfhost))
-            })
+    agree_on_resolution_in(Mode::Check, dir, files, code)
+}
+
+/// [`agree_on`] the resolution dump under `mode`: `files`, relative to
+/// `dir`, each a program's root, through both sides, all in one process a
+/// side, each side exiting with `code`; fail with the report of the first
+/// difference. Returns S0's output, which is N's. N reads and parses std
+/// once per process, about 0.4 s with the debug runtime (0.55 s for the
+/// WASM target, which also parses `STD_MODULES` for `missing`), so a set
+/// of roots goes to one process, not to batches (5.2b, ADR 0054 §9), and
+/// the callers run their sets side by side.
+fn agree_on_resolution_in(mode: Mode, dir: &Path, files: &[String], code: i32) -> String {
+    if files.is_empty() {
+        return String::new();
+    }
+    compare_batch(resolution_in(mode), dir, files, code, &nova(), selfhost())
+        .unwrap_or_else(|report| panic!("{report}"))
+}
+
+/// [`agree_on_resolution_in`] `files` in `parts` processes a side, side by
+/// side, root `i` in part `i % parts`: for a set whose roots are slow to
+/// load, the corpus's packages (`selfhost/src/`'s each reach most of N),
+/// which in one process would make the longest wait of the file. Returns
+/// S0's output, each root's part in the roots' order, each checked to be
+/// its root's, and none left over, so no root is dropped or dealt twice.
+fn agree_on_resolution_in_parts(
+    mode: Mode,
+    dir: &Path,
+    files: &[String],
+    code: i32,
+    parts: usize,
+) -> String {
+    let parts = parts.clamp(1, files.len().max(1));
+    let dealt: Vec<Vec<String>> = (0..parts)
+        .map(|k| files.iter().skip(k).step_by(parts).cloned().collect())
+        .collect();
+    let outs: Vec<String> = std::thread::scope(|scope| {
+        let runs: Vec<_> = dealt
+            .iter()
+            .map(|part| scope.spawn(move || agree_on_resolution_in(mode, dir, part, code)))
             .collect();
-        runs.into_iter()
-            .map(|r| r.join().expect("a batch's thread"))
-            .collect()
+        joined(runs)
     });
+    // Each output's parts, by its roots' headers, dealt back in order.
+    let mut split: Vec<std::collections::VecDeque<String>> = outs
+        .iter()
+        .map(|out| {
+            let mut roots = std::collections::VecDeque::new();
+            for line in out.split_inclusive('\n') {
+                if line.starts_with("== ") || roots.is_empty() {
+                    roots.push_back(String::new());
+                }
+                roots.back_mut().expect("a root's part").push_str(line);
+            }
+            roots
+        })
+        .collect();
     let mut all = String::new();
-    for out in outs {
-        match out {
-            Ok(out) => all.push_str(&out),
-            Err(report) => panic!("{report}"),
-        }
+    for (i, root) in files.iter().enumerate() {
+        let part = split[i % parts].pop_front().expect("each root's part");
+        assert!(
+            part.starts_with(&format!("== {root}\n")),
+            "root {i}, {root}, was dealt the part of {}",
+            part.lines().next().unwrap_or("")
+        );
+        all.push_str(&part);
     }
+    assert!(
+        split.iter().all(|s| s.is_empty()),
+        "a part no root was dealt"
+    );
     all
 }
 
-/// S0's resolution dump of `files` from `dir`, which must exit with
-/// `code`: what a package's roots are checked by until N dumps packages
-/// (ADR 0054).
-fn s0_resolution(dir: &Path, files: &[String], code: i32) -> String {
-    let out = Command::new(nova())
-        .current_dir(dir)
-        .args(RESOLUTION_S0)
-        .args(files)
-        .output()
-        .expect("run nova check --dump-resolution");
-    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
-    assert_eq!(out.status.code(), Some(code), "{stdout}");
-    stdout
-}
-
 /// `nova check`'s help names neither hidden flag, and every `nova check`
 /// command line whose first argument after `check` is not
 /// `--dump-resolution` is parsed as before the flags (ADR 0053 §1): each
@@ -1602,6 +2347,7 @@ fn the_resolution_dump_flags_are_hidden_and_refused_as_before() {
     assert!(help.contains("--target"), "{help}");
     assert!(!help.contains("dump-resolution"), "{help}");
     assert!(!help.contains("--std"), "{help}");
+    assert!(!help.contains("--test"), "{help}");
     let out = Command::new(nova())
         .args(["check", "--help", "--dump-resolution"])
         .output()
@@ -1626,6 +2372,14 @@ fn the_resolution_dump_flags_are_hidden_and_refused_as_before() {
         (&["check", "--st", "a.nova"], "--st", opts),
         (&["check", "--stdd", "a.nova"], "--stdd", opts),
         (&["check", "--std-part", "a.nova"], "--std-part", opts),
+        (&["check", "--test", "a.nova"], "--test", opts),
+        (&["check", "--test"], "--test", opts),
+        (&["check", "a.nova", "--test"], "--test", file),
+        (
+            &["check", "--test", "--dump-resolution", "a.nova"],
+            "--test",
+            opts,
+        ),
         (&["check", "--dump", "a.nova"], "--dump", opts),
         (&["check", "--dump-res", "a.nova"], "--dump-res", opts),
         (
@@ -1681,7 +2435,8 @@ fn the_resolution_dump_flags_are_hidden_and_refused_as_before() {
         );
         assert!(out.stdout.is_empty(), "{args:?}");
     }
-    // The dump's misuses, which the base has no form of.
+    // The dump's misuses, which the base has no form of; `--test` is
+    // native only, as `nova test` is (5.2b).
     for args in [
         &["--stdd"][..],
         &["--std", "--std"],
@@ -1689,6 +2444,13 @@ fn the_resolution_dump_flags_are_hidden_and_refused_as_before() {
         &["--help"],
         &["-h"],
         &["--dump-resolution", "a.nova"],
+        &["--test", "--test", "a.nova"],
+        &["--test=1", "a.nova"],
+        &["--tests", "a.nova"],
+        &["--test", "--target", "wasm", "a.nova"],
+        &["--target=wasm", "--test", "--std"],
+        &["--test", "--std", "a.nova"],
+        &["--test"],
     ] {
         let out = Command::new(nova())
             .args(RESOLUTION_S0)
@@ -1715,9 +2477,8 @@ fn the_resolution_dump_flags_are_hidden_and_refused_as_before() {
 /// cannot be read is `IOERR` and its kind, the rest are dumped, and the
 /// exit code is 2; a root in a package is a module of the package (ADR
 /// 0054); a program that fails to load holds the diagnostics so far;
-/// `--std` prints std's part for the target. N prints the same for the
-/// same roots but the package's, whose part S0 alone is checked for until
-/// N dumps packages.
+/// `--std` prints std's part for the target and mode. N prints the same
+/// for the same roots.
 #[test]
 fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
     let dir = fresh_dir("resolution-headers");
@@ -1765,8 +2526,9 @@ fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
         ),
         "{stdout}"
     );
-    // N prints the same, a file that is not UTF-8 included, and a root
-    // beside a `nova.toml` that is a directory, which makes no package.
+    // N prints the same, a package's root, a file that is not UTF-8 and a
+    // root beside a `nova.toml` that is a directory, which makes no package,
+    // included.
     std::fs::write(dir.join("latin1.nova"), b"fn \xe9() {}\n").expect("write latin1.nova");
     std::fs::create_dir_all(dir.join("dirpkg").join("nova.toml")).expect("create nova.toml/");
     std::fs::create_dir_all(dir.join("dirpkg").join("src")).expect("create dirpkg/src");
@@ -1778,6 +2540,7 @@ fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
     let files = [
         "a.nova",
         "missing.nova",
+        "pkg/src/main.nova",
         "b.nova",
         "dirpkg/src/main.nova",
         "latin1.nova",
@@ -1817,13 +2580,19 @@ fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
         "== \nIOERR NotFound\n"
     );
     assert!(out.stderr.is_empty());
-    for (target, head) in [
-        ("native", "(Std target=native modules=21 "),
-        ("wasm", "(Std target=wasm modules=16 "),
+    for (args, head) in [
+        (
+            &["--target", "native"][..],
+            "(Std target=native modules=21 ",
+        ),
+        (&["--target", "wasm"], "(Std target=wasm modules=16 "),
+        (&["--test"], "(Std target=native mode=test modules=22 "),
     ] {
+        let target = args.join(" ");
         let out = Command::new(nova())
             .args(RESOLUTION_S0)
-            .args(["--std", "--target", target])
+            .arg("--std")
+            .args(args)
             .output()
             .expect("run nova check --dump-resolution --std");
         let stdout = String::from_utf8_lossy(&out.stdout);
@@ -1839,52 +2608,148 @@ fn the_resolution_dump_has_a_header_per_root_and_std_its_own_part() {
 
 /// std's part, every loose corpus program and every resolver case give the
 /// same resolution dump on both sides, byte for byte (spec §3.5, §3.7):
-/// std's part once, then the programs, [`BATCH`] to a process. Every form
-/// of the dump's `KINDS` appears in these dumps, every form they print is
-/// listed, and no form of `LATER_KINDS` appears, nor any of the three
-/// codes 5.2b's test mode and WASM target give: the dumps are S0's and
-/// N's alike, so each holds them.
+/// std's part once, then the programs, [`BATCH`] to a process. 5.2b (spec
+/// §4, §4.5; ADR 0054) adds std's WASM and test-mode parts, the corpus's
+/// packages' files natively, its WASM programs (`tests/dom/`,
+/// `tests/wasm/`) for the WASM target and its files with tests under test
+/// mode, and the cases of every mode, each compared on both sides. Every
+/// form of the dump's `KINDS` appears in these dumps, every form they print
+/// is listed, and no form of `LATER_KINDS` appears; E0084 and E0085 occur
+/// only under test mode, which keeps `@test` functions, and E0090 only for
+/// the WASM target. The dumps are S0's and N's alike, so each holds them.
 #[test]
 fn the_resolution_dumps_agree_on_std_the_corpus_and_the_cases() {
     let root = root();
-    let std = compare_batch(std_resolution(), &root, &[], 0, &nova(), selfhost())
-        .unwrap_or_else(|report| panic!("{report}"));
-    assert!(
-        std.starts_with("(Std target=native modules=21 "),
-        "{}",
-        &std[..80]
-    );
+    // std's parts side by side, while the corpus and the cases are listed.
+    let std_parts: Vec<String> = std::thread::scope(|scope| {
+        let parts: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
+            .into_iter()
+            .map(|mode| {
+                let root = &root;
+                scope.spawn(move || {
+                    compare_batch(std_resolution(mode), root, &[], 0, &nova(), selfhost())
+                        .unwrap_or_else(|report| panic!("{mode:?}: {report}"))
+                })
+            })
+            .collect();
+        joined(parts)
+    });
+    for (part, mode) in std_parts.iter().zip([Mode::Check, Mode::Wasm, Mode::Test]) {
+        assert!(part.ends_with("))\n(Diagnostics)\n"), "{mode:?}");
+    }
+    for (part, head) in std_parts.iter().zip([
+        "(Std target=native modules=21 ",
+        "(Std target=wasm modules=16 ",
+        "(Std target=native mode=test modules=22 ",
+    ]) {
+        assert!(part.starts_with(head), "{}", &part[..80]);
+    }
     let loose = loose_corpus();
     // 251 loose programs at 5.2a.
     assert!(loose.len() >= 240, "only {} loose programs", loose.len());
-    // The files in a package, which are 5.2b's, are skipped, and said to
-    // be: 66 at 5.2a.
-    let skipped = corpus().len() - loose.len();
+    // 5.2b: the 67 files directly in a package's src/ or tests/, in 11
+    // packages (the nine the spec counts, `05-geometry`'s dependency and
+    // the VS Code extension's fixture), each a root, which 5.2a skipped.
+    let packages = package_corpus();
+    let package_dirs: std::collections::BTreeSet<&str> = packages
+        .iter()
+        .filter_map(|f| f.rsplitn(3, '/').nth(2))
+        .collect();
+    assert!(
+        packages.len() >= 60,
+        "only {} package files",
+        packages.len()
+    );
+    assert!(
+        package_dirs.contains("examples/05-geometry/shapes") && package_dirs.len() >= 10,
+        "{package_dirs:?}"
+    );
+    // 39 WASM programs at 5.2b, six of them a package's; 9 files with tests.
+    let wasm = wasm_corpus();
+    assert!(wasm.len() >= 35, "only {} WASM programs", wasm.len());
+    let tests = test_corpus();
+    assert!(tests.len() >= 8, "only {} files with tests", tests.len());
     eprintln!(
-        "the resolution pass skips {skipped} corpus files in a package's src/ or tests/ (5.2b's)"
+        "the resolution pass compares {} loose programs, {} package files in {} packages, \
+         {} WASM programs and {} files with tests",
+        loose.len(),
+        packages.len(),
+        package_dirs.len(),
+        wasm.len(),
+        tests.len()
     );
-    assert!(skipped >= 60, "only {skipped} corpus files in packages");
     // The corpus and the cases side by side.
-    let (mut dumps, (cases, count)) = std::thread::scope(|scope| {
-        let cases = scope.spawn(|| agree_on_resolve_cases(&resolve_cases_dir()));
-        let corpus = agree_on_resolution(&root, &loose, 0);
-        (corpus, cases.join().expect("the cases' thread"))
+    let (check, wasm_dumps, test_dumps, counts) = std::thread::scope(|scope| {
+        let cases: Vec<_> = [Mode::Check, Mode::Wasm, Mode::Test]
+            .into_iter()
+            .map(|mode| scope.spawn(move || agree_on_resolve_cases(mode, &mode_cases_dir(mode))))
+            .collect();
+        let package_dumps =
+            scope.spawn(|| agree_on_resolution_in_parts(Mode::Check, &root, &packages, 0, 3));
+        let wasm_dumps = scope.spawn(|| agree_on_resolution_in(Mode::Wasm, &root, &wasm, 0));
+        let test_dumps = scope.spawn(|| agree_on_resolution_in(Mode::Test, &root, &tests, 0));
+        let mut check = agree_on_resolution(&root, &loose, 0);
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
     });
-    // 103 cases at 5.2a.
-    assert!(count >= 90, "only {count} resolver cases");
-    dumps.push_str(&cases);
+    // 103 cases at 5.2a; at 5.2b 142 programs under `check` (39 of them a
+    // package's roots), 30 for the WASM target and 126 under test mode.
+    assert!(counts[0] >= 130, "only {} resolver cases", counts[0]);
+    assert!(counts[1] >= 25, "only {} WASM cases", counts[1]);
+    assert!(counts[2] >= 115, "only {} test-mode cases", counts[2]);
     // Each root's part, so every program was compared.
-    let parts = dumps.lines().filter(|l| l.starts_with("(Program ")).count();
-    assert_eq!(parts, loose.len() + count);
-    // None of the codes that only 5.2b's test mode and WASM target give
-    // (decision 8), though cases hold the programs that would give them.
-    let codes = codes_in(&dumps);
-    for code in LATER_RESOLVER_CODES {
-        assert!(!codes.contains_key(code), "{code} in {codes:?}");
+    let parts = |dumps: &str, head: &str| dumps.lines().filter(|l| l.starts_with(head)).count();
+    assert_eq!(
+        parts(&check, "(Program target=native resolved="),
+        loose.len() + packages.len() + counts[0]
+    );
+    assert_eq!(
+        parts(&wasm_dumps, "(Program target=wasm resolved="),
+        wasm.len() + counts[1]
+    );
+    assert_eq!(
+        parts(&test_dumps, "(Program target=native mode=test resolved="),
+        tests.len() + counts[2]
+    );
+    // E0084 and E0085 only under test mode, E0090 only for the WASM
+    // target (decision 8; spec §4.2, §4.3), though cases of every mode hold
+    // the programs that would give them; and test mode gives both.
+    for (dumps, absent) in [
+        (&check, &["E0084", "E0085", "E0090"][..]),
+        (&wasm_dumps, &["E0084", "E0085"]),
+        (&test_dumps, &["E0090"]),
+    ] {
+        let codes = codes_in(dumps);
+        for code in absent {
+            assert!(!codes.contains_key(*code), "{code} in {codes:?}");
+        }
     }
+    let test_codes = codes_in(&test_dumps);
+    assert!(
+        test_codes.contains_key("E0084") && test_codes.contains_key("E0085"),
+        "{test_codes:?}"
+    );
+    assert!(codes_in(&wasm_dumps).contains_key("E0090"));
     let mut found = std::collections::HashSet::new();
-    for line in dumps.lines().chain(std.lines()) {
-        found.extend(nova_driver::resolution_dump::forms_on(line));
+    for dumps in [&check, &wasm_dumps, &test_dumps]
+        .into_iter()
+        .chain(&std_parts)
+    {
+        for line in dumps.lines() {
+            found.extend(nova_driver::resolution_dump::forms_on(line));
+        }
     }
     let missing: Vec<&str> = nova_driver::resolution_dump::KINDS
         .iter()
@@ -1904,35 +2769,77 @@ fn the_resolution_dumps_agree_on_std_the_corpus_and_the_cases() {
         .collect();
     assert!(
         later.is_empty(),
-        "{later:?} appear, which LATER_KINDS says 5.2a's programs never print"
+        "{later:?} appear, which LATER_KINDS says no program prints"
     );
 }
 
-/// The resolver's cases as a Windows checkout has them ([`crlf`]): git
-/// gives no `.gitattributes` rule for `cases/`, so CI's Windows runner
-/// compares them with `\r\n`, and every system does here.
+/// The resolver's cases of every mode as a Windows checkout has them
+/// ([`crlf`]): git gives no `.gitattributes` rule for `cases/`, so CI's
+/// Windows runner compares them with `\r\n`, `nova.toml`s included, and
+/// every system does here. A mode whose copy changes no file, as on a
+/// Windows checkout, which already has each `\r\n`, is not compared
+/// again: `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`
+/// has compared those bytes (5.2b, ADR 0054 §9).
 #[test]
 fn the_resolution_dumps_agree_on_the_cases_with_windows_line_endings() {
     let dir = fresh_dir("crlf-resolve");
-    let mut files = Vec::new();
-    nova_files(&resolve_cases_dir(), &resolve_cases_dir(), &[], &mut files);
-    for file in &files {
-        let text = std::fs::read(resolve_cases_dir().join(file)).expect("read a case");
-        let path = dir.join(file);
-        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
-        std::fs::write(&path, crlf(text)).expect("write a case");
+    let mut crlf_files = 0;
+    let mut changed_modes = Vec::new();
+    for mode in [Mode::Check, Mode::Wasm, Mode::Test] {
+        let from = mode_cases_dir(mode);
+        let to = dir.join(mode_cases_name(mode));
+        let (held, changed) = copy_crlf(&from, &to);
+        crlf_files += held;
+        if changed > 0 {
+            changed_modes.push(mode);
+        }
     }
-    let crlf_files = files
-        .iter()
-        .filter(|f| std::fs::read(dir.join(f)).is_ok_and(|t| t.contains(&b'\r')))
-        .count();
     assert!(
-        crlf_files > 90,
+        crlf_files > 300,
         "only {crlf_files} cases have CRLF line endings"
     );
-    let (dumps, count) = agree_on_resolve_cases(&dir);
-    let parts = dumps.lines().filter(|l| l.starts_with("(Program ")).count();
-    assert_eq!(parts, count);
+    std::thread::scope(|scope| {
+        let modes: Vec<_> = changed_modes
+            .into_iter()
+            .map(|mode| {
+                let cases = dir.join(mode_cases_name(mode));
+                scope.spawn(move || (mode, agree_on_resolve_cases(mode, &cases)))
+            })
+            .collect();
+        for handle in modes {
+            let (mode, (dumps, count)) = handle.join().expect("a mode's thread");
+            let parts = dumps.lines().filter(|l| l.starts_with("(Program ")).count();
+            assert_eq!(parts, count, "{mode:?}");
+        }
+    });
+}
+
+/// Copy every file under `from` to `to`, as a Windows checkout has it
+/// ([`crlf`]); returns how many copies hold a `\r`, and how many differ
+/// from their files.
+fn copy_crlf(from: &Path, to: &Path) -> (usize, usize) {
+    let (mut held, mut changed) = (0, 0);
+    let mut entries: Vec<_> = std::fs::read_dir(from)
+        .expect("read a directory of cases")
+        .map(|e| e.expect("a directory entry"))
+        .collect();
+    entries.sort_by_key(|e| e.file_name());
+    std::fs::create_dir_all(to).expect("create a directory");
+    for e in entries {
+        let target = to.join(e.file_name());
+        if e.path().is_dir() {
+            let (h, c) = copy_crlf(&e.path(), &target);
+            held += h;
+            changed += c;
+        } else {
+            let original = std::fs::read(e.path()).expect("read a case");
+            let text = crlf(original.clone());
+            held += usize::from(text.contains(&b'\r'));
+            changed += usize::from(text != original);
+            std::fs::write(&target, text).expect("write a case");
+        }
+    }
+    (held, changed)
 }
 
 /// However its root is spelled, both sides form the same paths from it:
@@ -2003,8 +2910,8 @@ fn a_root_spelled_any_way_names_its_siblings_alike() {
 /// directory named `ui.nova` beside `main.nova` is E0093's, though it
 /// cannot be read, and a root directly in a package's `src/` or `tests/`,
 /// named from the package's directory, is a module of the package, as one
-/// named from outside it is (ADR 0054; S0's part alone until N dumps
-/// packages). An empty root is a file that is not there.
+/// named from outside it is (ADR 0054). An empty root is a file that is
+/// not there.
 #[test]
 fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
     let dir = fresh_dir("resolution-roots");
@@ -2023,7 +2930,7 @@ fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
     }
     std::fs::write(pkg.join("nova.toml"), MANIFEST).expect("write nova.toml");
     let roots = ["src/main.nova", "tests/main.nova"].map(String::from);
-    let out = s0_resolution(&pkg, &roots, 0);
+    let out = agree_on_resolution(&pkg, &roots, 0);
     for (root, label) in [("src/main.nova", "main"), ("tests/main.nova", "tests/main")] {
         assert!(
             out.contains(&format!(
@@ -2034,7 +2941,7 @@ fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
         );
     }
     let roots = ["pkg/tests/main.nova", ""].map(String::from);
-    let out = s0_resolution(&dir, &roots, 2);
+    let out = agree_on_resolution(&dir, &roots, 2);
     assert!(
         out.starts_with(
             "== pkg/tests/main.nova\n(Program target=native resolved=true\n  \
@@ -2043,12 +2950,10 @@ fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
         "{out}"
     );
     assert!(out.ends_with("\n== \nIOERR NotFound\n"), "{out}");
-    let out = agree_on_resolution(&dir, &["".to_string()], 2);
-    assert_eq!(out, "== \nIOERR NotFound\n");
 }
 
-/// A root's package is found as S0's `package_of` finds it (ADR 0053 §3):
-/// by asking whether the path `<dir>/nova.toml` is a file, or a link to
+/// A root's package is found as S0's `package_of` finds it (ADR 0053 §3,
+/// ADR 0054), on both sides: by asking whether the path `<dir>/nova.toml` is a file, or a link to
 /// one, not by looking for that exact name in a listing of `<dir>`. So a
 /// `Nova.toml` makes a package on a case-insensitive disk (macOS's and
 /// Windows' by default) on both sides, and on a case-sensitive one on
@@ -2056,8 +2961,6 @@ fn a_root_with_no_directory_part_or_in_a_package_loads_alike() {
 /// link none, and a directory that can be searched but not listed still
 /// holds its `nova.toml` for both (as root, which lists it anyway, that
 /// part is any other package). The test asks the disk whether it folds case.
-/// S0's parts are checked, and N's of the roots in no package until N
-/// dumps packages.
 #[test]
 fn a_package_is_found_by_its_manifests_path_on_both_sides() {
     #[cfg(unix)]
@@ -2100,14 +3003,7 @@ fn a_package_is_found_by_its_manifests_path_on_both_sides() {
         .iter()
         .map(|(name, _)| format!("{name}/src/main.nova"))
         .collect();
-    let loose: Vec<String> = roots
-        .iter()
-        .zip(&cases)
-        .filter(|(_, (_, package))| !*package)
-        .map(|(root, _)| root.clone())
-        .collect();
-    agree_on_resolution(&dir, &loose, 0);
-    let out = s0_resolution(&dir, &roots, 0);
+    let out = agree_on_resolution(&dir, &roots, 0);
     for (root, (_, package)) in roots.iter().zip(&cases) {
         let part = if *package {
             "(Program target=native resolved=true\n  (Module 0 \"main\" package=0)\n"
@@ -2195,18 +3091,38 @@ fn a_root_whose_path_is_not_utf8_is_dumped_by_s0_and_not_found_by_n() {
     }
 }
 
-/// The name fuzzer's fixed seeds and counts every run checks (spec §3.5).
+/// The name fuzzer's fixed seeds and counts every run checks (spec §3.5),
+/// and, from 5.2b (spec §4.5, ADR 0054), its WASM and package siblings'.
+/// The loose inputs are also resolved under test mode, which keeps their
+/// `@test` functions; the package's under `check`, each in a package whose
+/// manifests are one of four forms ([`fuzz::package_manifests`]), and the
+/// first seed's also under test mode and the second's for the WASM target,
+/// in the process of that mode's loose inputs, which keeps the fuzzer's
+/// time in proportion (ADR 0054).
 const NAME_SEEDS: [u64; 3] = [301, 302, 303];
 const NAME_COUNT: usize = 50;
-
-/// The resolver's codes that `check`'s pipeline gives a loose program on
-/// the native target. Of its 11, E0084 and E0085 are a `@test` function's,
-/// which `check` strips before resolving, and E0090 is WASM's: they come
-/// with 5.2b's test mode and target.
-const NATIVE_RESOLVER_CODES: [&str; 8] = [
-    "E0001", "E0002", "E0082", "E0083", "E0089", "E0092", "E0102", "E0900",
+const WASM_NAME_SEEDS: [u64; 3] = [311, 312, 313];
+const WASM_NAME_COUNT: usize = 20;
+const PACKAGE_NAME_SEEDS: [u64; 3] = [321, 322, 323];
+const PACKAGE_NAME_COUNT: usize = 30;
+
+/// The resolver's 11 codes (spec §2). `check` gives a loose native program
+/// 8 of them; E0084 and E0085 are a `@test` function's, which only test
+/// mode keeps, and E0090 is the WASM target's (decision 8).
+const RESOLVER_CODES: [&str; 11] = [
+    "E0001", "E0002", "E0082", "E0083", "E0084", "E0085", "E0089", "E0090", "E0092", "E0102",
+    "E0900",
+];
+const TEST_MODE_CODES: [&str; 2] = ["E0084", "E0085"];
+const WASM_CODES: [&str; 1] = ["E0090"];
+
+/// The two notes the loader puts on E0001 in a package (spec §4.1): a
+/// package with no library imported from its own `tests/`, and a
+/// dev-dependency imported from `src/`.
+const E0001_NOTES: [&str; 2] = [
+    " has no src/lib.nova\")",
+    " is a dev-dependency, which only `tests/` files can import\")",
 ];
-const LATER_RESOLVER_CODES: [&str; 3] = ["E0084", "E0085", "E0090"];
 
 /// `count` name-mutated programs from `seed` ([`fuzz::name_mutate`]), each
 /// one of `files`, under `root`, that the seed picks and that parses. As
@@ -2215,6 +3131,18 @@ const LATER_RESOLVER_CODES: [&str; 3] = ["E0084", "E0085", "E0090"];
 /// system. Input `i` is written as `f{i:05}.nova`, which a self-import
 /// names.
 fn names_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<String> {
+    names_of_with(root, files, seed, count, fuzz::name_mutate)
+}
+
+/// [`names_of`], each input made by `mutate`, which takes the rng, the
+/// source and the input's stem.
+fn names_of_with<T>(
+    root: &Path,
+    files: &[String],
+    seed: u64,
+    count: usize,
+    mutate: impl Fn(&mut SplitMix64, &str, &str) -> T,
+) -> Vec<T> {
     let mut files = files.to_vec();
     files.sort();
     let mut rng = SplitMix64::new(seed);
@@ -2225,7 +3153,7 @@ fn names_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<Strin
                 .expect("read a corpus file")
                 .replace("\r\n", "\n");
             if fuzz::parses(&source) {
-                break fuzz::name_mutate(&mut rng, &source, &format!("f{i:05}"));
+                break mutate(&mut rng, &source, &format!("f{i:05}"));
             }
         })
         .collect()
@@ -2236,11 +3164,37 @@ fn names(seed: u64, count: usize) -> Vec<String> {
     names_of(&root(), &loose_corpus(), seed, count)
 }
 
+/// The loose WASM programs of the corpus, which [`wasm_names`] mutates.
+fn loose_wasm_corpus() -> Vec<String> {
+    wasm_corpus()
+        .into_iter()
+        .filter(|f| !in_package(f))
+        .collect()
+}
+
+/// The name-mutated loose WASM programs ([`fuzz::wasm_name_mutate`]).
+fn wasm_names_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<String> {
+    names_of_with(root, files, seed, count, fuzz::wasm_name_mutate)
+}
+
+/// The name-mutated loose corpus as modules of [`fuzz::PACKAGE_FILES`]'
+/// package ([`fuzz::package_name_mutate`]): whether each is in `tests/`,
+/// and its text.
+fn package_names_of(root: &Path, files: &[String], seed: u64, count: usize) -> Vec<(bool, String)> {
+    names_of_with(root, files, seed, count, fuzz::package_name_mutate)
+}
+
 /// Write `inputs` to a fixed directory named after `name`, as
 /// `f00000.nova` and on, beside [`fuzz::NAME_SIBLINGS`]: the directory, and
 /// the inputs' files, each a program's root.
 fn write_name_inputs(name: &str, inputs: &[String]) -> (PathBuf, Vec<String>) {
     let dir = fresh_dir(name);
+    let files = write_name_inputs_in(&dir, inputs);
+    (dir, files)
+}
+
+/// [`write_name_inputs`] into `dir`, which exists: the inputs' files.
+fn write_name_inputs_in(dir: &Path, inputs: &[String]) -> Vec<String> {
     for (stem, source) in fuzz::NAME_SIBLINGS {
         std::fs::write(dir.join(format!("{stem}.nova")), source).expect("write a sibling");
     }
@@ -2250,7 +3204,89 @@ fn write_name_inputs(name: &str, inputs: &[String]) -> (PathBuf, Vec<String>) {
         std::fs::write(dir.join(&file), input).expect("write a name-fuzz input");
         files.push(file);
     }
-    (dir, files)
+    files
+}
+
+/// Each seed's inputs, [`write_name_inputs`] into a directory named for
+/// the seed in a fixed directory named after `name`: that directory, and
+/// every seed's roots, relative to it (`301/f00000.nova`), so that one
+/// process a side resolves all the seeds' inputs ([`seed_parts`]).
+fn write_seeded_inputs(name: &str, seeded: &[(u64, Vec<String>)]) -> (PathBuf, Vec<String>) {
+    let dir = fresh_dir(name);
+    let mut roots = Vec::new();
+    for (seed, inputs) in seeded {
+        let sub = dir.join(seed.to_string());
+        std::fs::create_dir_all(&sub).expect("create a seed's directory");
+        let files = write_name_inputs_in(&sub, inputs);
+        roots.extend(files.into_iter().map(|f| format!("{seed}/{f}")));
+    }
+    (dir, roots)
+}
+
+/// The parts of `dump`, a resolution dump of [`write_seeded_inputs`]'
+/// roots, whose roots are `seed`'s, each from its `== ` header on.
+fn seed_parts(dump: &str, seed: u64) -> String {
+    parts_under(dump, &seed.to_string())
+}
+
+/// The parts of `dump` whose roots are in the directory `dir`, each from
+/// its `== ` header on.
+fn parts_under(dump: &str, dir: &str) -> String {
+    let header = format!("== {dir}/");
+    let mut out = String::new();
+    let mut keep = false;
+    for line in dump.split_inclusive('\n') {
+        if line.starts_with("== ") {
+            keep = line.starts_with(&header);
+        }
+        if keep {
+            out.push_str(line);
+        }
+    }
+    out
+}
+
+/// Write [`fuzz::PACKAGE_FILES`] to `dir`, which exists, once for each
+/// form of [`fuzz::package_manifests`], `v0/` to `v3/`, each with its
+/// manifests in that form and [`fuzz::NAME_SIBLINGS`] in its `app/src/` and
+/// `app/tests/`, and input `i` as `f{i:05}.nova` in the `app/tests/` or
+/// `app/src/` of form `i % 4`: the inputs' roots, relative to `dir`.
+fn write_package_inputs_in(dir: &Path, inputs: &[(bool, String)]) -> Vec<String> {
+    let forms = fuzz::package_manifests();
+    // Form 3's manifests end every line with `\r\n`, which no other input
+    // reaching N's manifest reader through the fuzzer does.
+    for text in [&forms[3].0, &forms[3].1] {
+        assert_eq!(text.matches("\r\n").count(), text.matches('\n').count());
+        assert!(text.contains("\r\n"), "{text:?}");
+    }
+    for (k, (app_manifest, dep_manifest)) in forms.iter().enumerate() {
+        let form = dir.join(format!("v{k}"));
+        for (path, text) in fuzz::PACKAGE_FILES {
+            let text = match path {
+                "app/nova.toml" => app_manifest.as_str(),
+                "dep/nova.toml" => dep_manifest.as_str(),
+                _ => text,
+            };
+            let path = form.join(path);
+            std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
+            std::fs::write(path, text).expect("write a package's file");
+        }
+        for sub in ["src", "tests"] {
+            let app = form.join("app").join(sub);
+            std::fs::create_dir_all(&app).expect("create a package's directory");
+            for (stem, source) in fuzz::NAME_SIBLINGS {
+                std::fs::write(app.join(format!("{stem}.nova")), source).expect("write a sibling");
+            }
+        }
+    }
+    let mut roots = Vec::with_capacity(inputs.len());
+    for (i, (tests, input)) in inputs.iter().enumerate() {
+        let sub = if *tests { "tests" } else { "src" };
+        let root = format!("v{}/app/{sub}/f{i:05}.nova", i % forms.len());
+        std::fs::write(dir.join(&root), input).expect("write a name-fuzz input");
+        roots.push(root);
+    }
+    roots
 }
 
 /// How many diagnostics of each code `dump`, a resolution dump, holds.
@@ -2267,34 +3303,181 @@ fn codes_in(dump: &str) -> std::collections::BTreeMap<String, usize> {
     codes
 }
 
+/// What each of `handles` gave, in order: a thread that panicked fails the
+/// test with its panic.
+fn joined<T>(handles: Vec<std::thread::ScopedJoinHandle<'_, T>>) -> Vec<T> {
+    handles
+        .into_iter()
+        .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
+        .collect()
+}
+
+/// Each program's part of `dump`, by its root's header.
+fn parts_of(dump: &str) -> Vec<&str> {
+    dump.split("== ").filter(|p| !p.is_empty()).collect()
+}
+
 /// Both sides give the same resolution dump of each of the name fuzzer's
-/// inputs from the fixed seeds, every input reaches the resolver, and among
-/// them every resolver code `check` gives a loose native program occurs
-/// twice or more, and none of the three only test mode and WASM give.
+/// inputs from the fixed seeds, and every input reaches the resolver; and,
+/// from 5.2b, its inputs under test mode, its WASM sibling's for the WASM
+/// target and its package sibling's, with the package's manifests in four
+/// forms, one seed's also under test mode and another's for the WASM
+/// target. A package's input reaches the resolver, or the loader stops it
+/// with E0004 or a noted E0001, and E0004 names its entry on more than one
+/// line of a manifest, as the forms place it. Among
+/// them every one of the resolver's 11 codes, E0004 and E0001's two notes
+/// occur twice or more; E0084 and E0085 only under test mode, E0090 only
+/// for the WASM target. The loose inputs of every seed are written to
+/// `names/<seed>/` and the WASM ones to `wasm-names/<seed>/`, each set
+/// resolved in one process a side and mode ([`write_seeded_inputs`]).
 #[test]
-fn the_name_fuzzer_agrees_and_gives_every_native_resolver_code() {
+fn the_name_fuzzer_agrees_and_gives_every_resolver_code() {
+    let root = root();
     let mut all = std::collections::BTreeMap::new();
-    for seed in NAME_SEEDS {
-        let inputs = names(seed, NAME_COUNT);
-        let dump = agree_on_inputs(&format!("names-{seed}"), &inputs, Dumps::Resolution);
+    let mut add = |dump: &str, mode: Mode, what: &str| {
+        let codes = codes_in(dump);
+        let absent: &[&str] = match mode {
+            Mode::Check => &["E0084", "E0085", "E0090"],
+            Mode::Wasm => &TEST_MODE_CODES,
+            Mode::Test => &WASM_CODES,
+        };
+        for code in absent {
+            assert!(!codes.contains_key(*code), "{what}: {code} in {codes:?}");
+        }
+        for (code, n) in codes {
+            *all.entry(code).or_insert(0) += n;
+        }
+        for note in E0001_NOTES {
+            *all.entry(note.to_string()).or_insert(0) += dump.matches(note).count();
+        }
+    };
+    let loose = loose_corpus();
+    let loose_wasm = loose_wasm_corpus();
+    // Every seed's loose inputs in one process a side under `check`, again
+    // under test mode, and the WASM inputs in one, each mode in its own
+    // thread, as each seed's package inputs, which run from inside their
+    // own package; then counted in order, seed by seed.
+    let names_inputs: Vec<(u64, Vec<String>)> = NAME_SEEDS
+        .iter()
+        .map(|&seed| (seed, names(seed, NAME_COUNT)))
+        .collect();
+    let (names_dir, names_roots) = write_seeded_inputs("names", &names_inputs);
+    let wasm_inputs: Vec<(u64, Vec<String>)> = WASM_NAME_SEEDS
+        .iter()
+        .map(|&seed| {
+            let inputs = wasm_names_of(&root, &loose_wasm, seed, WASM_NAME_COUNT);
+            (seed, inputs)
+        })
+        .collect();
+    let (wasm_dir, mut wasm_roots) = write_seeded_inputs("wasm-names", &wasm_inputs);
+    let package_inputs: Vec<(u64, Vec<(bool, String)>)> = PACKAGE_NAME_SEEDS
+        .iter()
+        .map(|&seed| {
+            (
+                seed,
+                package_names_of(&root, &loose, seed, PACKAGE_NAME_COUNT),
+            )
+        })
+        .collect();
+    // One seed's package inputs under test mode, and another's for the WASM
+    // target, in the process of the loose inputs of that mode.
+    let mut test_roots = names_roots.clone();
+    for ((seed, inputs), (dir, roots)) in package_inputs[..2]
+        .iter()
+        .zip([(&names_dir, &mut test_roots), (&wasm_dir, &mut wasm_roots)])
+    {
+        let sub = format!("package-{seed}");
+        std::fs::create_dir_all(dir.join(&sub)).expect("create a package seed's directory");
+        let found = write_package_inputs_in(&dir.join(&sub), inputs);
+        roots.extend(found.into_iter().map(|r| format!("{sub}/{r}")));
+    }
+    let (check, test, wasm, package_dumps) = std::thread::scope(|scope| {
+        let test = scope.spawn(|| agree_on_resolution_in(Mode::Test, &names_dir, &test_roots, 0));
+        let wasm = scope.spawn(|| agree_on_resolution_in(Mode::Wasm, &wasm_dir, &wasm_roots, 0));
+        let packages: Vec<_> = package_inputs
+            .iter()
+            .map(|(seed, inputs)| {
+                let dir = fresh_dir(&format!("package-names-{seed}"));
+                let roots = write_package_inputs_in(&dir, inputs);
+                scope.spawn(move || (*seed, inputs.len(), agree_on_resolution(&dir, &roots, 0)))
+            })
+            .collect();
+        let check = agree_on_resolution(&names_dir, &names_roots, 0);
+        let [test, wasm] =
+            [test, wasm].map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)));
+        (check, test, wasm, joined(packages))
+    });
+    for (seed, inputs) in &names_inputs {
+        let count = inputs.len();
+        let dump = seed_parts(&check, *seed);
         let resolved = dump.matches("resolved=true").count();
         assert_eq!(
-            resolved,
-            inputs.len(),
+            resolved, count,
             "seed {seed}: an input did not reach the resolver"
         );
-        for (code, n) in codes_in(&dump) {
-            *all.entry(code).or_insert(0) += n;
+        add(&dump, Mode::Check, &format!("seed {seed}"));
+        let test = seed_parts(&test, *seed);
+        assert_eq!(test.matches("resolved=true").count(), count);
+        add(&test, Mode::Test, &format!("seed {seed} under test mode"));
+    }
+    for (seed, inputs) in &wasm_inputs {
+        let dump = seed_parts(&wasm, *seed);
+        assert_eq!(dump.matches("resolved=true").count(), inputs.len());
+        add(&dump, Mode::Wasm, &format!("WASM seed {seed}"));
+    }
+    let under_modes = [
+        (
+            Mode::Test,
+            parts_under(&test, &format!("package-{}", package_inputs[0].0)),
+        ),
+        (
+            Mode::Wasm,
+            parts_under(&wasm, &format!("package-{}", package_inputs[1].0)),
+        ),
+    ];
+    let package_parts = package_dumps
+        .into_iter()
+        .map(|(seed, count, dump)| (Mode::Check, seed, count, dump))
+        .chain(
+            under_modes
+                .into_iter()
+                .zip(&package_inputs)
+                .map(|((mode, dump), (seed, inputs))| (mode, *seed, inputs.len(), dump)),
+        );
+    // The line of each E0004's entry in a manifest, which the manifests'
+    // forms put on different lines.
+    let mut entry_lines = std::collections::BTreeSet::new();
+    for (mode, seed, count, dump) in package_parts {
+        for at in dump.split("and the entry at ").skip(1) {
+            let entry = at.split('"').next().unwrap_or("");
+            entry_lines.insert(entry.rsplit(':').next().unwrap_or("").to_string());
         }
+        let parts = parts_of(&dump);
+        assert_eq!(parts.len(), count);
+        for part in parts {
+            let stopped_by_the_loader = part.contains("\n  (Error \"E0004\"")
+                || E0001_NOTES.iter().any(|n| part.contains(n));
+            assert!(
+                part.contains(" resolved=true\n") || stopped_by_the_loader,
+                "package seed {seed}: an input did not reach the resolver: {part}"
+            );
+            assert!(part.contains(" package=0"), "{part}");
+        }
+        add(
+            &dump,
+            mode,
+            &format!("package seed {seed} ({})", mode_cases_name(mode)),
+        );
     }
-    eprintln!("codes over seeds {NAME_SEEDS:?}: {all:?}");
-    // At 5.2a: E0001 46, E0002 63, E0082 14, E0083 8, E0089 10, E0092 13,
-    // E0102 50, E0900 18.
-    for code in NATIVE_RESOLVER_CODES {
-        let n = all.get(code).copied().unwrap_or(0);
+    assert!(
+        entry_lines.len() >= 2,
+        "E0004's entries all on one line of a manifest: {entry_lines:?}"
+    );
+    eprintln!(
+        "codes over seeds {NAME_SEEDS:?}, {WASM_NAME_SEEDS:?} and {PACKAGE_NAME_SEEDS:?}: {all:?}"
+    );
+    for code in RESOLVER_CODES.iter().chain(&["E0004"]).chain(&E0001_NOTES) {
+        let n = all.get(*code).copied().unwrap_or(0);
         assert!(n >= 2, "{code} occurs {n} times, under 2, in {all:?}");
     }
-    for code in LATER_RESOLVER_CODES {
-        assert!(!all.contains_key(code), "{code} in {all:?}");
-    }
 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ef75ce070b7d336c47dbbc62f48c2b5b4eeea952
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/geom/src/lib.nova
@@ -0,0 +1,6 @@
+pub fn width() -> Int {
+    2
+}
+
+@test
+fn stripped() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..88a9626966df764710a16cd216d46a27b0b2195d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..62cc7f2bbab74bc1f5a513b945b91ec4740f49a6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/src/lib.nova
@@ -0,0 +1,10 @@
+import geom
+
+pub fn twice(n: Int) -> Int {
+    n * width()
+}
+
+@test
+fn twice_doubles() {
+    assert_eq(twice(2), 4)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/tests/api.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/tests/api.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ae47b96927a1d03b980698d6a8e6ba2fa4222117
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_dependencys_tests_are_stripped_and_a_tests_file_names_its_own/tests/api.nova
@@ -0,0 +1,11 @@
+import app
+
+@test
+fn first() {
+    assert_eq(twice(1), 2)
+}
+
+@test(should_panic)
+fn second() {
+    panic("no")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_loose_siblings_tests_are_kept/helpers.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_loose_siblings_tests_are_kept/helpers.nova
new file mode 100644
index 0000000000000000000000000000000000000000..84403ea7d26de5dfca823cd2bf21a4360fa6e6ef
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_loose_siblings_tests_are_kept/helpers.nova
@@ -0,0 +1,6 @@
+pub fn helper() -> Int {
+    0
+}
+
+@test
+fn in_helpers() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_loose_siblings_tests_are_kept/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_loose_siblings_tests_are_kept/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ac6309036c9d638a4d4d10d1a9b71b24749be8bc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_loose_siblings_tests_are_kept/main.nova
@@ -0,0 +1,8 @@
+import helpers
+
+@test
+fn in_main() {
+    assert_eq(helper(), 0)
+}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_test_attribute_on_an_impl_block_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_test_attribute_on_an_impl_block_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c4f1f359b443965cd3dbd33bfe08d2776bb68305
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_test_attribute_on_an_impl_block_is_e0083.nova
@@ -0,0 +1,7 @@
+trait Tr {}
+record R { x: Int }
+
+@test
+impl Tr for R {}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_test_function_that_would_collide_is_stripped.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_test_function_that_would_collide_is_stripped.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fc0d40d1b01f914ebeba1ff2c710436c30910225
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_test_function_that_would_collide_is_stripped.nova
@@ -0,0 +1,6 @@
+fn helper() {}
+
+@test
+fn helper() {}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/helper/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/helper/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..52ab7f309d4fff6effe0444f5d1a37b4bcb96117
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/helper/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "helper"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/helper/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/helper/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..be0dd19b48b05af24addfc67133919d43197ec20
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/helper/src/lib.nova
@@ -0,0 +1,6 @@
+pub fn expected() -> Int {
+    42
+}
+
+@test
+fn not_kept() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..175e46be7b375aa5e482859295c4f90ecdd6c2e1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "my-app"
+version = "0.1.0"
+edition = "2026"
+
+[dev-dependencies]
+helper = { path = "helper" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a43e803d8573008727ccd2ff8e31d62f7810b082
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn answer() -> Int {
+    42
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/tests/answers.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/tests/answers.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e3afee1436d17351c98396bd48ad13cc50b1bca9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/tests/answers.nova
@@ -0,0 +1,8 @@
+import my_app
+import helper
+import support
+
+@test
+fn the_answer() {
+    assert_eq(answer(), expected() + offset())
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/tests/support.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/tests/support.nova
new file mode 100644
index 0000000000000000000000000000000000000000..493acdf924a58fb9a46735c16d14ea175ed881c8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/a_tests_file_sees_the_dev_dependencies_and_its_siblings/tests/support.nova
@@ -0,0 +1,6 @@
+pub fn offset() -> Int {
+    0
+}
+
+@test
+fn a_sibling_test() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/analyze__a_dropped_name_raises_no_e0001_at_any_site/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/analyze__a_dropped_name_raises_no_e0001_at_any_site/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..40c64e35a05cd0520a5bf3550050355852a55ea4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/analyze__a_dropped_name_raises_no_e0001_at_any_site/lib.nova
@@ -0,0 +1,2 @@
+pub fn area(r: Int {
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/analyze__a_dropped_name_raises_no_e0001_at_any_site/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/analyze__a_dropped_name_raises_no_e0001_at_any_site/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b29546328929c4fa8922d858be3911b38b77840a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/analyze__a_dropped_name_raises_no_e0001_at_any_site/main.nova
@@ -0,0 +1,11 @@
+import lib::{area}
+fn f(x: Int {
+}
+record P %
+trait T %
+fn main() {
+    f(1)
+    let g = f
+    let p: P = P { x: 1 }
+}
+impl T for Int {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/analyze__with_tests_a_test_body_is_checked.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/analyze__with_tests_a_test_body_is_checked.nova
new file mode 100644
index 0000000000000000000000000000000000000000..92e71f17c63b0ef9b089c6a360cf3b32cb20203c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/analyze__with_tests_a_test_body_is_checked.nova
@@ -0,0 +1,5 @@
+@test
+fn t() {
+    let x: Int = "s"
+}
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/check__an_unknown_test_argument_is_e0085.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__an_unknown_test_argument_is_e0085.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f936da3e4bfd81fc1c51ecf702962609a385bc7e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__an_unknown_test_argument_is_e0085.nova
@@ -0,0 +1,3 @@
+@test(shuold_panic)
+fn t() { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_before_a_module_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_before_a_module_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..47e5719946db0b30117de1c2152fe81681235ff7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_before_a_module_is_e0083.nova
@@ -0,0 +1,3 @@
+@test
+module foo;
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_before_an_extern_block_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_before_an_extern_block_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8067c3b69cbb5fb23c496c1d0c0753ec9aa968dc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_before_an_extern_block_is_e0083.nova
@@ -0,0 +1,3 @@
+@test
+extern { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_before_an_import_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_before_an_import_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0e0ef1cd536b587eb1fe1fbdb81e6c47e4b52663
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_before_an_import_is_e0083.nova
@@ -0,0 +1,3 @@
+@test
+import core
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_function_returning_a_value_is_e0084.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_function_returning_a_value_is_e0084.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0d4d655977da09d11c1b4d2d135e0e4337c57f3e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_function_returning_a_value_is_e0084.nova
@@ -0,0 +1,3 @@
+@test
+fn t() -> Int { 1 }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_function_with_parameters_is_e0084.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_function_with_parameters_is_e0084.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2386927bc67d2badcb6edf02a5b31b9100a67d4a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_function_with_parameters_is_e0084.nova
@@ -0,0 +1,3 @@
+@test
+fn t(x: Int) { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_generic_function_is_e0084.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_generic_function_is_e0084.nova
new file mode 100644
index 0000000000000000000000000000000000000000..627b38dbf20ad9fb1f2be9a2ed5f653ae76157f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_generic_function_is_e0084.nova
@@ -0,0 +1,3 @@
+@test
+fn t<T>() { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_non_function_is_e0083.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_non_function_is_e0083.nova
new file mode 100644
index 0000000000000000000000000000000000000000..537074e237de5c908cadab98f0fc12feba2f68c0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_a_non_function_is_e0083.nova
@@ -0,0 +1,3 @@
+@test
+record R { n: Int }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_an_async_function_is_e0084.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_an_async_function_is_e0084.nova
new file mode 100644
index 0000000000000000000000000000000000000000..93afb6873bc5fcd9e977b5b89d077ab3daca89fa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/check__test_on_an_async_function_is_e0084.nova
@@ -0,0 +1,3 @@
+@test
+async fn t() { }
+fn main() { }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__compile_error/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__compile_error/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__compile_error/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__compile_error/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__compile_error/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d5f4a0a52f914dae7c803b39cd14cbd0aae469a5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__compile_error/src/lib.nova
@@ -0,0 +1,17 @@
+//! Arithmetic, checked.
+//!
+//! ```
+//! assert_eq(twice(2) + 1, 5)
+//! ```
+
+import units
+
+/// Twice `x`.
+///
+/// ```
+/// assert_eq(twice(3), nope)
+/// ```
+pub fn twice(x: Int) -> Int { x * 2 + zero() }
+
+@test
+fn twice_doubles() { assert_eq(twice(1), 2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__compile_error/src/units.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__compile_error/src/units.nova
new file mode 100644
index 0000000000000000000000000000000000000000..85dbeec886b886fd416179bbd9367c37b6194ac9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__compile_error/src/units.nova
@@ -0,0 +1,8 @@
+//! Units of nothing.
+//!
+//! ```
+//! assert_eq(twice(0), 0)
+//! ```
+
+/// Zero.
+pub fn zero() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__failing/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__failing/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__failing/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__failing/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__failing/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3146524ebbd0f67410d9856925b3123c22b6761c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__failing/src/lib.nova
@@ -0,0 +1,17 @@
+//! Arithmetic, checked.
+//!
+//! ```
+//! assert_eq(twice(2) + 1, 5)
+//! ```
+
+import units
+
+/// Twice `x`.
+///
+/// ```
+/// assert_eq(twice(3), 7)
+/// ```
+pub fn twice(x: Int) -> Int { x * 2 + zero() }
+
+@test
+fn twice_doubles() { assert_eq(twice(1), 2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__failing/src/units.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__failing/src/units.nova
new file mode 100644
index 0000000000000000000000000000000000000000..85dbeec886b886fd416179bbd9367c37b6194ac9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__failing/src/units.nova
@@ -0,0 +1,8 @@
+//! Units of nothing.
+//!
+//! ```
+//! assert_eq(twice(0), 0)
+//! ```
+
+/// Zero.
+pub fn zero() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__filter_one/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__filter_one/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__filter_one/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__filter_one/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__filter_one/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..44be0bdbf248671eddad7d4620513df6302f071d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__filter_one/src/lib.nova
@@ -0,0 +1,17 @@
+//! Arithmetic, checked.
+//!
+//! ```
+//! assert_eq(twice(2) + 1, 5)
+//! ```
+
+import units
+
+/// Twice `x`.
+///
+/// ```
+/// assert_eq(twice(3), 6)
+/// ```
+pub fn twice(x: Int) -> Int { x * 2 + zero() }
+
+@test
+fn twice_doubles() { assert_eq(twice(1), 2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__filter_one/src/units.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__filter_one/src/units.nova
new file mode 100644
index 0000000000000000000000000000000000000000..85dbeec886b886fd416179bbd9367c37b6194ac9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__filter_one/src/units.nova
@@ -0,0 +1,8 @@
+//! Units of nothing.
+//!
+//! ```
+//! assert_eq(twice(0), 0)
+//! ```
+
+/// Zero.
+pub fn zero() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_1/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_1/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_1/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5736e31197154cab62767d51b201ed676bc19940
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_1/src/lib.nova
@@ -0,0 +1,6 @@
+pub fn double(x: Int) -> Int { x + x }
+
+fn check_double(x: Int) { assert_eq(double(x), x + x) }
+
+@test
+fn doubles() { check_double(2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_2/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e77c696ea14ab718c99494ada86f65493654f5ff
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__helper_2/src/lib.nova
@@ -0,0 +1,11 @@
+/// Twice `x`.
+///
+/// ```
+/// assert_eq(double(3), 6)
+/// ```
+pub fn double(x: Int) -> Int { x + x }
+
+fn check_double(x: Int) { assert_eq(double(x), x + x) }
+
+@test
+fn doubles() { check_double(2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__main_value/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__main_value/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__main_value/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__main_value/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__main_value/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a009ac35fb3d875e1aaea4c4123de331af6bae52
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__main_value/src/lib.nova
@@ -0,0 +1,6 @@
+/// Twice `x`.
+///
+/// ```
+/// fn main() -> Int { 1 }
+/// ```
+pub fn twice(x: Int) -> Int { x * 2 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_1/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_1/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_1/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..88e80d410454fda835414fc593286e1929cc1ea9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_1/src/lib.nova
@@ -0,0 +1,13 @@
+/// Twice `x`:
+///
+/// ```text
+/// twice(3) is 6
+/// ```
+///
+/// ```ignore
+/// assert_eq(twice(3), 7)
+/// ```
+pub fn twice(x: Int) -> Int { x * 2 }
+
+@test
+fn twice_doubles() { assert_eq(twice(1), 2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_2/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f138360648f36247802be517970b7985443b5000
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__no_doc_tests_2/src/lib.nova
@@ -0,0 +1,13 @@
+/// Twice `x`:
+///
+/// ```text
+/// twice(3) is 6
+/// ```
+///
+/// ```ignore
+/// assert_eq(twice(3), 7)
+/// ```
+pub fn twice(x: Int) -> Int { x * 2 }
+
+@test
+fn twice_doubles() { assert_eq(twice(1), 3) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__program/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__program/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__program/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__program/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__program/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4fde644933ed72391e53f3c1f2541046015b11d0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__program/src/main.nova
@@ -0,0 +1,6 @@
+/// Starts.
+///
+/// ```
+/// assert(false, "never run")
+/// ```
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__unknown_word/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__unknown_word/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3e05c9751d5cecc9edf3b7659f66a226dbeafa02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__unknown_word/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "calc"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__unknown_word/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__unknown_word/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1c8ee5366c71940d320becf110d85ba997b3c871
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__unknown_word/src/lib.nova
@@ -0,0 +1,17 @@
+//! Arithmetic, checked.
+//!
+//! ```
+//! assert_eq(twice(2) + 1, 5)
+//! ```
+
+import units
+
+/// Twice `x`.
+///
+/// ```nova,should_panic
+/// assert_eq(twice(3), 6)
+/// ```
+pub fn twice(x: Int) -> Int { x * 2 + zero() }
+
+@test
+fn twice_doubles() { assert_eq(twice(1), 2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__unknown_word/src/units.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__unknown_word/src/units.nova
new file mode 100644
index 0000000000000000000000000000000000000000..85dbeec886b886fd416179bbd9367c37b6194ac9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_test__unknown_word/src/units.nova
@@ -0,0 +1,8 @@
+//! Units of nothing.
+//!
+//! ```
+//! assert_eq(twice(0), 0)
+//! ```
+
+/// Zero.
+pub fn zero() -> Int { 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_map_sees_every_diagnostic_and_cannot_hide_an_error.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_map_sees_every_diagnostic_and_cannot_hide_an_error.nova
new file mode 100644
index 0000000000000000000000000000000000000000..18b588c348a65d6dd87dcbab5ba6becc071e7546
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_map_sees_every_diagnostic_and_cannot_hide_an_error.nova
@@ -0,0 +1,9 @@
+@test
+fn t() {
+    let x = 1
+    match x {
+        _ => {}
+        1 => {}
+    }
+    let y: Int = "s"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_mapped_diagnostic_renders_at_its_source_line.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_mapped_diagnostic_renders_at_its_source_line.nova
new file mode 100644
index 0000000000000000000000000000000000000000..44a1bcf348b0e94a487a3f1faffd124205dd95f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_mapped_diagnostic_renders_at_its_source_line.nova
@@ -0,0 +1,4 @@
+@test
+fn uses_nope() {
+    assert_eq(nope(2), 4)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_test_binary_builds_from_sources_not_on_disk/helper.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_test_binary_builds_from_sources_not_on_disk/helper.nova
new file mode 100644
index 0000000000000000000000000000000000000000..16d97d16be9a90826e297b8202ee082cd2681cf5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_test_binary_builds_from_sources_not_on_disk/helper.nova
@@ -0,0 +1,3 @@
+pub fn add(a: Int, b: Int) -> Int {
+    a + b
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_test_binary_builds_from_sources_not_on_disk/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_test_binary_builds_from_sources_not_on_disk/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ac1552921178f1fcdb6f1f078de68c50ee1f8561
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doc_tests__a_test_binary_builds_from_sources_not_on_disk/main.nova
@@ -0,0 +1,11 @@
+import helper
+
+@test
+fn adds() {
+    assert_eq(add(1, 2), 3)
+}
+
+@test
+fn fails() {
+    assert_eq(add(1, 2), 4)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__doc_tests_are_named_by_the_sites_paths/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__doc_tests_are_named_by_the_sites_paths/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..1d6b53c37b1da370f272bb553c8a35ea2089f732
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__doc_tests_are_named_by_the_sites_paths/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "shapes"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__doc_tests_are_named_by_the_sites_paths/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__doc_tests_are_named_by_the_sites_paths/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..24855ce6568a34f2840b2ad4a9a35811dac64704
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__doc_tests_are_named_by_the_sites_paths/src/lib.nova
@@ -0,0 +1,47 @@
+//! Shapes.
+//!
+//! ```
+//! let a = circle(1.0)
+//! ```
+
+import units
+
+/// A circle's area.
+///
+/// ```nova,should_panic
+/// let a = circle(2.0)
+/// ```
+///
+/// ```text
+/// not a test
+/// ```
+pub fn circle(r: Float) -> Float { 3.0 * r * r }
+
+/// A rectangle.
+pub record Rect {
+    /// Its width.
+    ///
+    /// ```
+    /// let r = Rect { width: 1.0 }
+    /// ```
+    pub width: Float,
+}
+
+/// Rect's own methods.
+///
+///     let r = Rect { width: 2.0 }
+impl Rect {
+    /// Its width, twice.
+    ///
+    /// ```ignore
+    /// let w = Rect { width: 1.0 }.twice()
+    /// ```
+    pub fn twice(self) -> Float { self.width * 2.0 }
+}
+
+/// Not exported.
+///
+/// ```
+/// let s = secret()
+/// ```
+fn secret() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__doc_tests_are_named_by_the_sites_paths/src/units.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__doc_tests_are_named_by_the_sites_paths/src/units.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f53bb0bce02d63d5e493bf65470ff9cbc1b6244e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__doc_tests_are_named_by_the_sites_paths/src/units.nova
@@ -0,0 +1,21 @@
+//! Lengths.
+//!
+//! ```
+//! let l = Length { metres: 1.0 }
+//! ```
+
+/// A length.
+pub record Length {
+    pub metres: Float,
+}
+
+impl Length {
+    /// The sum of two lengths.
+    ///
+    /// ```
+    /// let l = Length { metres: 1.0 }.plus(Length { metres: 2.0 })
+    /// ```
+    pub fn plus(self, other: Length) -> Length {
+        Length { metres: self.metres + other.metres }
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3fb3fb39ab78265dadc26948f5813666e2a3f3f2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_1/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_1/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_1/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0c5ea8039dd3470535b76d89e72d37ad515fdd00
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_1/src/main.nova
@@ -0,0 +1,6 @@
+/// Runs.
+///
+/// ```
+/// main()
+/// ```
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..a12bfa2fbe097c89fdad7889f5ebd33ba40c7fef
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_2/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "quiet"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ed7cf12144a0cb18c3eba22b991faf412fa8081e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/doctest__only_a_librarys_documented_items_give_doc_tests_2/src/lib.nova
@@ -0,0 +1,14 @@
+/// ```
+/// let a = secret()
+/// ```
+fn secret() -> Int { 1 }
+
+/// ```text
+/// shown()
+/// ```
+pub fn shown() -> Int { secret() }
+
+/// ```ignore
+/// skipped()
+/// ```
+pub fn skipped() -> Int { 2 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c9d4b28c018293a1a0bd583f8f54d471a745fe33
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/pkgs/geom/src/lib.nova
@@ -0,0 +1,8 @@
+@tset
+fn helper() {}
+
+pub fn area() -> Int {
+    let x = 1
+    x = 2
+    x
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..12e4c6dafd220fb2c59252762c59f07d212c30ed
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__a_dependencys_errors_offer_no_fix/src/main.nova
@@ -0,0 +1,5 @@
+import geom
+
+fn main() {
+    println("${area()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/geometry.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/geometry.nova
new file mode 100644
index 0000000000000000000000000000000000000000..62bdc829aec739c6ae71bf8c6f6c0a89baaaaf56
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/geometry.nova
@@ -0,0 +1,17 @@
+pub record Point { x: Int, y: Int }
+
+pub fn origin() -> Point {
+    Point { x: 0, y: 0 }
+}
+
+pub fn manhattan(p: Point) -> Int {
+    p.x + p.y
+}
+
+pub trait Shape {
+    fn area(self) -> Int
+}
+
+pub fn one() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4ecc04f9c0f1daed31e7e00e74f9730f2be91c77
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/lib.nova
@@ -0,0 +1,6 @@
+import geometry
+import shapes
+
+pub fn name() -> String {
+    "demo"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..02645abbaf8193edcdaf6e10ca0a975187b58671
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/main.nova
@@ -0,0 +1,6 @@
+// A header.
+
+// What main does.
+fn main() {
+    println("${twice()} ${one()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/shapes.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/shapes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c52f489dbab41f9e7ab7d46b7708544ab042a1e4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_1/src/shapes.nova
@@ -0,0 +1,3 @@
+pub fn twice() -> Int {
+    2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/geometry.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/geometry.nova
new file mode 100644
index 0000000000000000000000000000000000000000..62bdc829aec739c6ae71bf8c6f6c0a89baaaaf56
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/geometry.nova
@@ -0,0 +1,17 @@
+pub record Point { x: Int, y: Int }
+
+pub fn origin() -> Point {
+    Point { x: 0, y: 0 }
+}
+
+pub fn manhattan(p: Point) -> Int {
+    p.x + p.y
+}
+
+pub trait Shape {
+    fn area(self) -> Int
+}
+
+pub fn one() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4ecc04f9c0f1daed31e7e00e74f9730f2be91c77
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/lib.nova
@@ -0,0 +1,6 @@
+import geometry
+import shapes
+
+pub fn name() -> String {
+    "demo"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5c948aad857c90b7c9151e7e07bee16cb3ef1ac1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/main.nova
@@ -0,0 +1,8 @@
+// A header.
+
+import geometry::{one}
+
+// What main does.
+fn main() {
+    println("${twice()} ${one()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/shapes.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/shapes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c52f489dbab41f9e7ab7d46b7708544ab042a1e4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_13_import_goes_above_the_first_item_below_a_header_2/src/shapes.nova
@@ -0,0 +1,3 @@
+pub fn twice() -> Int {
+    2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..37f1168a4d14f8c3b858b5cb930c047712d3a382
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/pkgs/geom/src/lib.nova
@@ -0,0 +1,7 @@
+pub fn area() -> Int {
+    1
+}
+
+pub fn side() -> Int {
+    2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..816e8f6097dd8a575a4ba31a3d56aa93e7c21e4b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/src/main.nova
@@ -0,0 +1,6 @@
+import geom
+import shapes
+
+fn main() {
+    println("${area()} ${twice()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/src/shapes.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/src/shapes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f472d738019db66f771de8dc0c9007bf6e0b8bc6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_1/src/shapes.nova
@@ -0,0 +1,3 @@
+pub fn twice() -> Int {
+    side() * 2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..37f1168a4d14f8c3b858b5cb930c047712d3a382
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/pkgs/geom/src/lib.nova
@@ -0,0 +1,7 @@
+pub fn area() -> Int {
+    1
+}
+
+pub fn side() -> Int {
+    2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..816e8f6097dd8a575a4ba31a3d56aa93e7c21e4b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/src/main.nova
@@ -0,0 +1,6 @@
+import geom
+import shapes
+
+fn main() {
+    println("${area()} ${twice()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/src/shapes.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/src/shapes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..433c9dca63fe686af3d3fbbfb99d96e87f61c9d4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_14_import_from_a_dependencys_library_2/src/shapes.nova
@@ -0,0 +1,5 @@
+import geom::{side}
+
+pub fn twice() -> Int {
+    side() * 2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0de0c50f2c352c3ed94bd7b4cf8179f5d29b6996
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_1.nova
@@ -0,0 +1,4 @@
+@tset
+fn t() {}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bd4ab99270e2f90d91978063cf0fd5b9bdef3b8a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_2.nova
@@ -0,0 +1,4 @@
+@test
+fn t() {}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_3.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fef8e099d72cee311b437fc3c37b5f2cd59f5595
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_3.nova
@@ -0,0 +1,6 @@
+@test(should_panik)
+fn t() {
+    panic("x")
+}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_4.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c6582a221a98b4464aef8e1ba3c8579cc04face5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_24_did_you_mean_an_attribute_and_a_test_argument_4.nova
@@ -0,0 +1,6 @@
+@test(should_panic)
+fn t() {
+    panic("x")
+}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_27_make_public_edits_the_sibling_module/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_27_make_public_edits_the_sibling_module/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d35aae543e970c1b8671fb9d9271ffa0a9783a26
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_27_make_public_edits_the_sibling_module/lib.nova
@@ -0,0 +1,4 @@
+/// The secret.
+fn hidden() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_27_make_public_edits_the_sibling_module/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_27_make_public_edits_the_sibling_module/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..345efd7f1a03cac410ecfd3f8f60e93b12e4416d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_27_make_public_edits_the_sibling_module/main.nova
@@ -0,0 +1,5 @@
+import lib::{hidden}
+
+fn main() {
+    println("${hidden()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_1/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_1/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_1/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a8a5ff773cb313f8eb846fc83cb214eabd922f5c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_1/src/lib.nova
@@ -0,0 +1,3 @@
+fn name() -> String {
+    "app"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_1/tests/t.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_1/tests/t.nova
new file mode 100644
index 0000000000000000000000000000000000000000..368358388dd8a9b64850f033b70333c3b87dca67
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_1/tests/t.nova
@@ -0,0 +1,6 @@
+import demo::{name}
+
+@test
+fn t() {
+    assert_eq(name(), "app")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_2/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cbc4b58db30c8bdbc7ef32750a36f4016f5ad707
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_2/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn name() -> String {
+    "app"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_2/tests/t.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_2/tests/t.nova
new file mode 100644
index 0000000000000000000000000000000000000000..368358388dd8a9b64850f033b70333c3b87dca67
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_28_make_public_from_tests_edits_the_packages_library_2/tests/t.nova
@@ -0,0 +1,6 @@
+import demo::{name}
+
+@test
+fn t() {
+    assert_eq(name(), "app")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3780509b9a41b372197059f19b9927212b30a10a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/pkgs/geom/src/lib.nova
@@ -0,0 +1,7 @@
+fn secret() -> Int {
+    1
+}
+
+pub fn area() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9f52f171775f1e657e9a43c9e2c9edab648f4281
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__case_29_no_make_public_for_a_dependencys_item/src/main.nova
@@ -0,0 +1,3 @@
+import geom::{secret}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..945d422f3f28125289cb140dafed5ac363ce3d17
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/pkgs/geom/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn area() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cbc4b58db30c8bdbc7ef32750a36f4016f5ad707
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn name() -> String {
+    "app"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..74113fad0d40458abb6e89c4ae7af1a97f5b3ac5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/src/main.nova
@@ -0,0 +1,4 @@
+import geom
+import shapes
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/src/shapes.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/src/shapes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7b5642d9bf5de9a19d24a5a56873aeacf1800c39
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/src/shapes.nova
@@ -0,0 +1,3 @@
+pub fn side() -> Int {
+    2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/tests/check.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/tests/check.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b0e63b41fadadc456bfbb2e3487f72db5bf0c28e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__every_loaded_module_knows_what_it_could_import/tests/check.nova
@@ -0,0 +1,4 @@
+import app
+
+@test
+fn t() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__make_public_is_not_offered_when_a_glob_of_the_module_would_clash/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__make_public_is_not_offered_when_a_glob_of_the_module_would_clash/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3411a5a31d3eed4413f5550321ec262f849e25ce
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__make_public_is_not_offered_when_a_glob_of_the_module_would_clash/lib.nova
@@ -0,0 +1,3 @@
+fn helper() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__make_public_is_not_offered_when_a_glob_of_the_module_would_clash/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__make_public_is_not_offered_when_a_glob_of_the_module_would_clash/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f4f1eadd5e05115719945824dfff6b55c4700647
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__make_public_is_not_offered_when_a_glob_of_the_module_would_clash/main.nova
@@ -0,0 +1,6 @@
+import lib::{helper}
+import other
+
+fn main() {
+    println("${helper()} ${twice()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__make_public_is_not_offered_when_a_glob_of_the_module_would_clash/other.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__make_public_is_not_offered_when_a_glob_of_the_module_would_clash/other.nova
new file mode 100644
index 0000000000000000000000000000000000000000..21964d50bcd8514c6a5706a7eaf5e85059fd03c1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/fixes__make_public_is_not_offered_when_a_glob_of_the_module_would_clash/other.nova
@@ -0,0 +1,9 @@
+import lib
+
+fn helper() -> Int {
+    2
+}
+
+pub fn twice() -> Int {
+    helper() * 2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_file_nested_too_deeply_elsewhere_keeps_unused_imports/geometry.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_file_nested_too_deeply_elsewhere_keeps_unused_imports/geometry.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3d2a8fd5b802f125949d509b1102fa352d60ebdf
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_file_nested_too_deeply_elsewhere_keeps_unused_imports/geometry.nova
@@ -0,0 +1,7 @@
+pub fn origin() -> Int {
+    1
+}
+
+pub fn manhattan() -> Int {
+    a.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x.x
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_file_nested_too_deeply_elsewhere_keeps_unused_imports/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_file_nested_too_deeply_elsewhere_keeps_unused_imports/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5406b9ea490e93df7542974047e454c592f12642
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_file_nested_too_deeply_elsewhere_keeps_unused_imports/main.nova
@@ -0,0 +1,6 @@
+import geometry::{manhattan, origin}
+
+fn main() {
+    let o = origin()
+    let m = manhattan()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_parse_error_elsewhere_keeps_unused_imports/geometry.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_parse_error_elsewhere_keeps_unused_imports/geometry.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6704539db3ec2625772c453e994c8a4dd673e61c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_parse_error_elsewhere_keeps_unused_imports/geometry.nova
@@ -0,0 +1,7 @@
+pub fn origin() -> Int {
+    1
+}
+
+pub fn manhattan( -> Int {
+    2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_parse_error_elsewhere_keeps_unused_imports/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_parse_error_elsewhere_keeps_unused_imports/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5406b9ea490e93df7542974047e454c592f12642
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/organize__a_parse_error_elsewhere_keeps_unused_imports/main.nova
@@ -0,0 +1,6 @@
+import geometry::{manhattan, origin}
+
+fn main() {
+    let o = origin()
+    let m = manhattan()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e1f5c7f4af9734f4b6588408edd8c7a74deef540
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/pkgs/geom/src/lib.nova
@@ -0,0 +1,8 @@
+pub fn area() -> Int {
+    9
+}
+
+@test
+fn geom_checks_its_area() {
+    assert_eq(area(), 9)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..60749d55d0c5d9e780be764ea9b141b2719b53ea
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__dependency_tests/src/main.nova
@@ -0,0 +1,10 @@
+import geom
+
+fn main() {
+    println("${area()}")
+}
+
+@test
+fn app_checks_the_area() {
+    assert_eq(area(), 9)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3fb3fb39ab78265dadc26948f5813666e2a3f3f2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7a515373d7b10b0e49d48e8efcb9c293d90cffa6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/src/main.nova
@@ -0,0 +1,5 @@
+import utils
+
+fn main() {
+    println(label())
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..649e10265a872dc70afac3888a6de3f92cfb6bfe
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/src/utils.nova
@@ -0,0 +1,3 @@
+pub fn label() -> String {
+    "src"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/tests/api.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/tests/api.nova
new file mode 100644
index 0000000000000000000000000000000000000000..98b0422642f4c6bdcfe0dd7161e0ef5ca3eb3a3e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/tests/api.nova
@@ -0,0 +1,6 @@
+import utils
+
+@test
+fn the_tests_utils_is_seen() {
+    assert_eq(helper(), "tests")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/tests/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/tests/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3d2de1d7a97e048ea12ee2d65e34679c71aa4324
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__src_utils_and_tests_utils_are_two_modules/tests/utils.nova
@@ -0,0 +1,3 @@
+pub fn helper() -> String {
+    "tests"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1ddebc440e03ecf9d048a76a1ba27e693b94c84b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/src/lib.nova
@@ -0,0 +1,10 @@
+import utils
+
+pub fn area() -> Int {
+    width() * width()
+}
+
+@test
+fn area_is_nine() {
+    assert_eq(area(), 9)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2131f4df757f47f89aa6aff9949c52c75322ebf6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/src/utils.nova
@@ -0,0 +1,3 @@
+pub fn width() -> Int {
+    3
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/tests/api.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/tests/api.nova
new file mode 100644
index 0000000000000000000000000000000000000000..caf164ce73a566fcd749b0f94eb1ef32d35477e8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_1/tests/api.nova
@@ -0,0 +1,6 @@
+import geom
+
+@test
+fn parses() {
+    assert_eq(area(), 9)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_2/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1ddebc440e03ecf9d048a76a1ba27e693b94c84b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_2/src/lib.nova
@@ -0,0 +1,10 @@
+import utils
+
+pub fn area() -> Int {
+    width() * width()
+}
+
+@test
+fn area_is_nine() {
+    assert_eq(area(), 9)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_2/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_2/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2131f4df757f47f89aa6aff9949c52c75322ebf6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_2/src/utils.nova
@@ -0,0 +1,3 @@
+pub fn width() -> Int {
+    3
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..1d6b53c37b1da370f272bb553c8a35ea2089f732
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "shapes"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3addec5a3c6a9d6126df1c49907f793e54105014
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/src/lib.nova
@@ -0,0 +1,8 @@
+pub fn area() -> Int {
+    9
+}
+
+@test
+fn in_the_library() {
+    assert_eq(area(), 9)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/tests/a.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/tests/a.nova
new file mode 100644
index 0000000000000000000000000000000000000000..501ea5b0579ecd7da67f0bc45f9653da2af1a94a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/tests/a.nova
@@ -0,0 +1,6 @@
+import shapes
+
+@test
+fn first() {
+    assert_eq(area(), 9)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/tests/b.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/tests/b.nova
new file mode 100644
index 0000000000000000000000000000000000000000..74286b855e4bfe9e24035ade0145d42cb8293fc6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_1/tests/b.nova
@@ -0,0 +1,6 @@
+import shapes
+
+@test
+fn second() {
+    assert_eq(area(), 9)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..1d6b53c37b1da370f272bb553c8a35ea2089f732
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_2/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "shapes"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3addec5a3c6a9d6126df1c49907f793e54105014
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__test_names_2/src/lib.nova
@@ -0,0 +1,8 @@
+pub fn area() -> Int {
+    9
+}
+
+@test
+fn in_the_library() {
+    assert_eq(area(), 9)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..d11c577cb15bdc50d6a41b9df2125e3adda357b6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/nova.toml
@@ -0,0 +1,10 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
+
+[dev-dependencies]
+helper = { path = "pkgs/helper" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d076f590d495542783f674a686ed1cdf45962d3c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/geom/src/lib.nova
@@ -0,0 +1,5 @@
+import utils
+
+pub fn area() -> Int {
+    width() * width()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/geom/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/geom/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2131f4df757f47f89aa6aff9949c52c75322ebf6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/geom/src/utils.nova
@@ -0,0 +1,3 @@
+pub fn width() -> Int {
+    3
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/helper/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/helper/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..52ab7f309d4fff6effe0444f5d1a37b4bcb96117
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/helper/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "helper"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/helper/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/helper/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..de4fe7f2852fdd6977506c09aa713037fcefa395
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/pkgs/helper/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn twice(n: Int) -> Int {
+    n * 2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cbc4b58db30c8bdbc7ef32750a36f4016f5ad707
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn name() -> String {
+    "app"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/tests/api.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/tests/api.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3991857f6e8026872414a4ddb380207e296a6ce1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/tests/api.nova
@@ -0,0 +1,12 @@
+import app
+import geom
+import helper
+import util
+
+@test
+fn sees_all_four() {
+    assert_eq(name(), "app")
+    assert_eq(area(), 9)
+    assert_eq(twice(2), 4)
+    assert_eq(one(), 1)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/tests/util.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/tests/util.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f1feea4b6a5686960e50adbaaea3e282580a17f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/packages__tests_files_import_the_package_a_dev_dependency_and_each_other/tests/util.nova
@@ -0,0 +1,3 @@
+pub fn one() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..bff845301e4d60a8606fb8534efed579c8489ca6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/nova.toml
@@ -0,0 +1,6 @@
+[package]
+name = "both"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dacd5fd262699c7977ae5d30c5631b9bea91dd5b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/src/lib.nova
@@ -0,0 +1,4 @@
+pub fn greeting() -> String { "Hello, Nova!" }
+
+@test
+fn greeting_says_hello() { assert_eq(greeting(), "Hello, Nova!") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9aab0c6ab9df68ace482d23d2560d59b9c90687c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/src/main.nova
@@ -0,0 +1,3 @@
+fn main() {
+    println("hello from the project")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/tests/both_test.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/tests/both_test.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4cc034178c15695b105ea29ef311fb8857a5b230
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_1/tests/both_test.nova
@@ -0,0 +1,4 @@
+import both
+
+@test
+fn greeting_is_public() { assert_eq(greeting(), "Hello, Nova!") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..bff845301e4d60a8606fb8534efed579c8489ca6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_2/nova.toml
@@ -0,0 +1,6 @@
+[package]
+name = "both"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dacd5fd262699c7977ae5d30c5631b9bea91dd5b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__init_lib_2/src/lib.nova
@@ -0,0 +1,4 @@
+pub fn greeting() -> String { "Hello, Nova!" }
+
+@test
+fn greeting_says_hello() { assert_eq(greeting(), "Hello, Nova!") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..581c572cc2e9d5f056622cf211b68c4362ff23d3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new/nova.toml
@@ -0,0 +1,6 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7d88ad02656f691e12690303da221ab8c79fbaf5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new/src/main.nova
@@ -0,0 +1,6 @@
+fn greeting() -> String { "Hello, Nova!" }
+
+fn main() { println(greeting()) }
+
+@test
+fn greeting_says_hello() { assert_eq(greeting(), "Hello, Nova!") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..8ea2a2f2b2badc4d5bcb9c9f3496e607b659b5bb
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_1/nova.toml
@@ -0,0 +1,6 @@
+[package]
+name = "json-api"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_1/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_1/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dacd5fd262699c7977ae5d30c5631b9bea91dd5b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_1/src/lib.nova
@@ -0,0 +1,4 @@
+pub fn greeting() -> String { "Hello, Nova!" }
+
+@test
+fn greeting_says_hello() { assert_eq(greeting(), "Hello, Nova!") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_1/tests/json_api_test.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_1/tests/json_api_test.nova
new file mode 100644
index 0000000000000000000000000000000000000000..834be50e0a347afb87918cc42aeb20776df4a123
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_1/tests/json_api_test.nova
@@ -0,0 +1,4 @@
+import json_api
+
+@test
+fn greeting_is_public() { assert_eq(greeting(), "Hello, Nova!") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..8ea2a2f2b2badc4d5bcb9c9f3496e607b659b5bb
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_2/nova.toml
@@ -0,0 +1,6 @@
+[package]
+name = "json-api"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dacd5fd262699c7977ae5d30c5631b9bea91dd5b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__new_lib_2/src/lib.nova
@@ -0,0 +1,4 @@
+pub fn greeting() -> String { "Hello, Nova!" }
+
+@test
+fn greeting_says_hello() { assert_eq(greeting(), "Hello, Nova!") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__spaces/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__spaces/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__spaces/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/project__spaces/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__spaces/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b61fde942559060d8ba2ab7ab95c7affbfc45bae
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/project__spaces/src/main.nova
@@ -0,0 +1,8 @@
+fn main() {
+    println("main")
+}
+
+@test
+fn passes() {
+    assert_eq(1 + 1, 2)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5f5375c1147a617cad80a5c638a46e700a4e8cb5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/lib.nova
@@ -0,0 +1,5 @@
+import lib
+pub fn alpha() -> Int { 1 }
+fn secret() -> Int { 2 }
+pub record P {}
+pub trait T {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a6a094393b9189099f7cea2e1c3344d360f9884c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/resolver_dump__a_programs_part_prints_its_modules_definitions_items_and_diagnostics/main.nova
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
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_hard_trap_is_reported_as_a_trap_and_does_not_satisfy_should_panic.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_hard_trap_is_reported_as_a_trap_and_does_not_satisfy_should_panic.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4ad3ed4b165d25bde7a915142cd5d286d85cc848
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_hard_trap_is_reported_as_a_trap_and_does_not_satisfy_should_panic.nova
@@ -0,0 +1,2 @@
+@test(should_panic)
+fn divides_by_zero() { let _ = 1 / 0 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_test_binary_runs_exactly_the_selected_test.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_test_binary_runs_exactly_the_selected_test.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2b8e086bab8b1526f3d3de9c1f744daac5c71bac
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_test_binary_runs_exactly_the_selected_test.nova
@@ -0,0 +1,4 @@
+@test
+fn alpha() { println("A") }
+@test
+fn beta() { println("B") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_test_binary_runs_the_selected_test_not_the_users_own_main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_test_binary_runs_the_selected_test_not_the_users_own_main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..190054a4b8da3ad227f707f1dc6d53681d9e4f6a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_test_binary_runs_the_selected_test_not_the_users_own_main.nova
@@ -0,0 +1,3 @@
+@test
+fn alpha() { println("TEST-RAN") }
+fn main() { println("USER-MAIN-RAN") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_test_binary_with_no_index_prints_its_inventory.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_test_binary_with_no_index_prints_its_inventory.nova
new file mode 100644
index 0000000000000000000000000000000000000000..eda917509617ea8949d88a1c4b241168c2ab4b92
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_test_binary_with_no_index_prints_its_inventory.nova
@@ -0,0 +1,4 @@
+@test
+fn alpha() { }
+@test
+fn beta() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_trapping_tests_captured_output_is_shown_not_discarded.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_trapping_tests_captured_output_is_shown_not_discarded.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2fbb92c0a8ffe2d1484a96555c6382a25453a116
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__a_trapping_tests_captured_output_is_shown_not_discarded.nova
@@ -0,0 +1,5 @@
+@test(should_panic)
+fn prints_then_traps() {
+    println("about to trap")
+    let _ = 1 / 0
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_eq_failure_names_both_values.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_eq_failure_names_both_values.nova
new file mode 100644
index 0000000000000000000000000000000000000000..baf245fb497364b49154a6ffc9b3139ccfed01dc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_eq_failure_names_both_values.nova
@@ -0,0 +1,2 @@
+@test
+fn t() { assert_eq(1, 3) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_eq_works_at_bool_and_float_not_only_int.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_eq_works_at_bool_and_float_not_only_int.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4a3ea6c8db8cfb1f1e404392bde52c0547cb8a72
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_eq_works_at_bool_and_float_not_only_int.nova
@@ -0,0 +1,16 @@
+@test
+fn int_pass() { assert_eq(1, 1) }
+@test
+fn int_fail() { assert_eq(1, 2) }
+@test
+fn bool_pass() { assert_eq(true, true) }
+@test
+fn bool_fail() { assert_eq(true, false) }
+@test
+fn float_pass() { assert_eq(0.5, 0.5) }
+@test
+fn float_fail() { assert_eq(0.5, 1.5) }
+@test
+fn string_pass() { assert_eq("a", "a") }
+@test
+fn string_fail() { assert_eq("a", "b") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_ne_panics_when_equal_and_is_silent_when_different.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_ne_panics_when_equal_and_is_silent_when_different.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f923b316ac14f42939bc09a58af160e066fe38dd
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_ne_panics_when_equal_and_is_silent_when_different.nova
@@ -0,0 +1,4 @@
+@test
+fn ne_pass() { assert_ne(1, 2) }
+@test
+fn ne_fail() { assert_ne(1, 1) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_panics_with_its_message_and_is_silent_when_true.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_panics_with_its_message_and_is_silent_when_true.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1555a931777772e28a0de001a1fd1170809a5c98
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__assert_panics_with_its_message_and_is_silent_when_true.nova
@@ -0,0 +1,4 @@
+@test
+fn assert_pass() { assert(true, "boom") }
+@test
+fn assert_fail() { assert(false, "boom") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__process_test_reentry.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__process_test_reentry.nova
new file mode 100644
index 0000000000000000000000000000000000000000..73f9cbb50452c53aa07fd3b13b33c9575fbc4a3b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__process_test_reentry.nova
@@ -0,0 +1,26 @@
+fn count(name: String) -> Int {
+    let path = "${temp_dir()}/${name}"
+    let before = match block_on(read_to_string(path)) {
+        Ok(s) => s.parse_int().unwrap_or(0)
+        Err(_) => 0
+    }
+    block_on(write_string(path, "${before + 1}")).unwrap()
+    before + 1
+}
+
+@test
+fn run_starts_its_binary_once() {
+    if count("run-count") < 3 {
+        let out = run(current_exe().unwrap(), ["child"]).unwrap()
+        assert_eq(out.status, 0)
+        let listed = out.stdout.to_string().unwrap_or("")
+        assert(listed.contains("status_starts_its_binary_once"), listed)
+    }
+}
+
+@test
+fn status_starts_its_binary_once() {
+    if count("status-count") < 3 {
+        assert_eq(status(current_exe().unwrap(), ["child"]).unwrap(), 0)
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_async_block_on.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_async_block_on.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7377c5aa69e3a2ee70bd230ca57c913baf2eadfa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_async_block_on.nova
@@ -0,0 +1,8 @@
+async fn helper() -> Int {
+  yield_now().await
+  41
+}
+@test
+fn an_async_body_runs_under_block_on() { assert_eq(block_on(helper()), 41) }
+@test
+fn a_wrong_answer_inside_block_on_fails() { assert_eq(block_on(helper()), 999) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_async_rejected.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_async_rejected.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ee41faa5fa2be5b77e28cff12b973dcc040a989d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_async_rejected.nova
@@ -0,0 +1,3 @@
+async fn helper() -> Int { 41 }
+@test
+async fn t() { assert_eq(helper().await, 999) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_binary_output_paths_do_not_collide_across_same_stem_sources_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_binary_output_paths_do_not_collide_across_same_stem_sources_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..13dc74fc850491a7b210090561b4791b01db873d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_binary_output_paths_do_not_collide_across_same_stem_sources_1.nova
@@ -0,0 +1,2 @@
+@test
+fn from_a() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_binary_output_paths_do_not_collide_across_same_stem_sources_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_binary_output_paths_do_not_collide_across_same_stem_sources_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1f27249a9d6f6c9900c58888e63e123b6ab55f41
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_binary_output_paths_do_not_collide_across_same_stem_sources_2.nova
@@ -0,0 +1,2 @@
+@test
+fn from_b() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_captured_stdout_on_failure.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_captured_stdout_on_failure.nova
new file mode 100644
index 0000000000000000000000000000000000000000..403c9e9dd2e93362a101548603163e15dc5fe6fd
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_captured_stdout_on_failure.nova
@@ -0,0 +1,5 @@
+@test
+fn prints_then_fails() {
+    println("about to fail")
+    assert_eq(1, 2)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_critical_nontest_compile.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_critical_nontest_compile.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c616ca7c2cdf115dab21d8b2d74721bf1f0217f6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_critical_nontest_compile.nova
@@ -0,0 +1,5 @@
+@test
+fn addition_is_correct() { assert_eq(2 + 2, 4) }
+@test
+fn addition_is_wrong() { assert_eq(2 + 2, 5) }
+fn main() { println("hello") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_filter.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_filter.nova
new file mode 100644
index 0000000000000000000000000000000000000000..abc6e820e19cb48155df2978f7ef1a4085f2c723
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_filter.nova
@@ -0,0 +1,10 @@
+@test
+fn keep_a() { assert_eq(1, 1) }
+@test
+fn keep_b() { assert_eq(2, 2) }
+@test
+fn skip_c() {
+    let mut i = 0
+    while i < 40000000000 { i = i + 1 }
+    assert_eq(i, 40000000000)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_filter_matches_nothing.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_filter_matches_nothing.nova
new file mode 100644
index 0000000000000000000000000000000000000000..34c890df254390c2157d2b66ca50236cd653b75d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_filter_matches_nothing.nova
@@ -0,0 +1,2 @@
+@test
+fn addition_is_correct() { assert_eq(1 + 1, 2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_filter_substring_not_prefix.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_filter_substring_not_prefix.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ca3efb1bc491231bb96e0bcd1cc9183ee2455442
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_filter_substring_not_prefix.nova
@@ -0,0 +1,6 @@
+@test
+fn zzz_alpha_zzz() { assert_eq(1, 1) }
+@test
+fn keep_alpha() { assert_eq(2, 2) }
+@test
+fn unrelated_beta() { assert_eq(3, 3) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_function_values.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_function_values.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bb0560b0eac63e0aceb485e9484a8787bf620d90
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_function_values.nova
@@ -0,0 +1,12 @@
+fn inc(x: Int) -> Int { x + 1 }
+fn apply(f: fn(Int) -> Int, x: Int) -> Int { f(x) }
+@test
+fn a_closure() {
+    let f = |x: Int| x + 1
+    assert_eq(f(1), 2)
+}
+@test
+fn a_named_function_as_a_value() {
+    assert_eq(apply(inc, 1), 2)
+    assert_eq(Some(1).map(inc).unwrap_or(0), 2)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_pass_fail.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_pass_fail.nova
new file mode 100644
index 0000000000000000000000000000000000000000..49f8172436997502ee72c43b7314436c766019fa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_pass_fail.nova
@@ -0,0 +1,4 @@
+@test
+fn addition_works() { assert_eq(1 + 1, 2) }
+@test
+fn addition_is_broken() { assert_eq(1 + 1, 3) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_should_panic_by_index.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_should_panic_by_index.nova
new file mode 100644
index 0000000000000000000000000000000000000000..09491508d62b8c3ee62f97ec6d3af17af6ccf6c1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_should_panic_by_index.nova
@@ -0,0 +1,7 @@
+@test
+fn alpha_ok() { assert_eq(1, 1) }
+@test(should_panic)
+fn beta_panics_checked() { let xs = [1, 2, 3]
+ let _ = xs[9] }
+@test
+fn gamma_ok() { assert_eq(2, 2) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_should_panic_checked.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_should_panic_checked.nova
new file mode 100644
index 0000000000000000000000000000000000000000..327f92ea847e9ac3e23a1eb13cca2f3c9e0ce17d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_should_panic_checked.nova
@@ -0,0 +1,3 @@
+@test(should_panic)
+fn out_of_bounds_panics() { let xs = [1, 2, 3]
+ let _ = xs[7] }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_should_panic_not_panicking.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_should_panic_not_panicking.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0e27e823807f79a5c2ace9a42a6aed443a447c1d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_cli_should_panic_not_panicking.nova
@@ -0,0 +1,2 @@
+@test(should_panic)
+fn does_not_actually_panic() { assert_eq(1, 1) }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_gate_e0082.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_gate_e0082.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bc787347407761aac441a89f7ac6f058cba7e8d7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_gate_e0082.nova
@@ -0,0 +1,2 @@
+@tset
+fn foo() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_join_handle_double_spawn_guard.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_join_handle_double_spawn_guard.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f83867df5e50d09c90d86df4d6446f8057ddab68
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_join_handle_double_spawn_guard.nova
@@ -0,0 +1,13 @@
+async fn spin() -> Int { 1 }
+async fn run_two_spawns() -> Int {
+  let a = spawn(spin())
+  let b = spawn(spin())
+  let ra = a.join().await
+  let rb = b.join().await
+  ra + rb
+}
+
+@test
+fn spawning_two_distinct_futures_still_works() {
+  assert_eq(block_on(run_two_spawns()), 2)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_join_handle_forgery_and_duplicate.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_join_handle_forgery_and_duplicate.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3d24ed9a8949710b5031fc847cffdad99382ceca
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_join_handle_forgery_and_duplicate.nova
@@ -0,0 +1,14 @@
+async fn spin() -> Int { 1 }
+
+@test(should_panic)
+fn joining_a_never_spawned_handle_aborts() {
+  let h = JoinHandle { fut: spin() }
+  let _ = block_on(h.join())
+}
+
+@test(should_panic)
+fn spawning_the_same_future_twice_aborts() {
+  let f = spin()
+  spawn(f)
+  let _ = spawn(f)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_join_handle_join_twice.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_join_handle_join_twice.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5daae766bc2b1c5928e71d7a17421efcb47cd40c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/run_tests__test_join_handle_join_twice.nova
@@ -0,0 +1,12 @@
+async fn spin() -> Int { 1 }
+async fn run_join_twice() -> Int {
+  let h = spawn(spin())
+  let a = h.join().await
+  let b = h.join().await
+  a + b
+}
+
+@test
+fn joining_a_handle_twice_returns_the_same_value_twice() {
+  assert_eq(block_on(run_join_twice()), 2)
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-test/test_attributes_in_every_shape.nova b/crates/nova-cli/tests/selfhost/cases/resolve-test/test_attributes_in_every_shape.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d26e4473970f40126bca9af3bd34ea12eba655d5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-test/test_attributes_in_every_shape.nova
@@ -0,0 +1,37 @@
+@test
+fn plain() {}
+
+@test(should_panic)
+fn panics() {
+    panic("no")
+}
+
+@test
+fn takes(n: Int) {}
+
+@test
+fn gives() -> Int {
+    1
+}
+
+@test
+fn generic<T>() {}
+
+@test
+async fn later() {}
+
+@test(should_panc)
+fn misspelt() {}
+
+@test(should_panic, again)
+fn twice() {}
+
+@test
+record Probe {
+    n: Int,
+}
+
+@tset
+fn close() {}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/native/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/native/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..5de47e50071e3cbd88baac9d26dfebfac982b1a2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/native/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "native"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/native/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/native/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d91a3116920ee817705fd963b8798151b6ea598b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/native/src/lib.nova
@@ -0,0 +1,7 @@
+extern "C" {
+    fn abs(x: Int) -> Int
+}
+
+pub fn one() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..aeb7f1abfdfd953fa36cd0c231114eb8327820cf
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "page"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+native = { path = "native" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f3370bce6268478d7d094a4d9610c89a38d7b4e8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/a_wasm_package_whose_dependency_has_an_extern_block/src/main.nova
@@ -0,0 +1,4 @@
+import native
+import dom
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/an_extern_block_is_e0090_and_a_native_module_e0092.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/an_extern_block_is_e0090_and_a_native_module_e0092.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dd84345dde1738c89141b06532b27181963aa057
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/an_extern_block_is_e0090_and_a_native_module_e0092.nova
@@ -0,0 +1,7 @@
+import ui::ssr
+
+extern "C" {
+    fn abs(x: Int) -> Int
+}
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/dom_and_ui_are_imported_on_wasm.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/dom_and_ui_are_imported_on_wasm.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6960f5ec6db7f32d7591e7ccdb4e37623045a65f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/dom_and_ui_are_imported_on_wasm.nova
@@ -0,0 +1,6 @@
+import dom
+import ui
+import ui::html::{div}
+import ui::router
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..fbd3b610e911050c295074159c6e3a756fb33f5e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_1.nova
@@ -0,0 +1,2 @@
+import ui::router::{link}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9697bb6d9a5498868693e4d7508d577484a9ea98
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_2.nova
@@ -0,0 +1,2 @@
+import ui
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_3.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b1e3a58db64316012a16f7b8fa89f94517ad7038
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_3.nova
@@ -0,0 +1,2 @@
+import ui::html::{div}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_4.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_4.nova
new file mode 100644
index 0000000000000000000000000000000000000000..06a2ca2d0b95ccb100973b6dfc31b8ba3c54f2ff
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_4.nova
@@ -0,0 +1,2 @@
+import ui::html
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_5.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_5.nova
new file mode 100644
index 0000000000000000000000000000000000000000..3f90aa91ddabd395732913a54a0bf2c206f4b88c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_5.nova
@@ -0,0 +1,2 @@
+import a::b
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_6.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_6.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a2ac1f9607a89f273855cd5d74ecca9fdb5afe36
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__a_nested_import_path_names_only_stds_modules_6.nova
@@ -0,0 +1,2 @@
+import ui::nope
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__an_extern_block_is_e0090_on_wasm.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__an_extern_block_is_e0090_on_wasm.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5a967529c7b14d22a9a92b5719f4e2bdd5dc446a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__an_extern_block_is_e0090_on_wasm.nova
@@ -0,0 +1,4 @@
+extern "C" {
+    fn abs(x: Int) -> Int
+}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__dom_is_seen_only_through_an_import.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__dom_is_seen_only_through_an_import.nova
new file mode 100644
index 0000000000000000000000000000000000000000..424868c66db74cf7850e7b527432c805e46444f8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__dom_is_seen_only_through_an_import.nova
@@ -0,0 +1,2 @@
+import dom::{Element}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__ssr_is_imported_by_name_and_only_natively.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__ssr_is_imported_by_name_and_only_natively.nova
new file mode 100644
index 0000000000000000000000000000000000000000..beabd0fdc502bd14ed62da93c516a23b262fa6c5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__ssr_is_imported_by_name_and_only_natively.nova
@@ -0,0 +1,2 @@
+import ui::ssr
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__ui_is_seen_only_through_an_import.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__ui_is_seen_only_through_an_import.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c8df1ae026087e2eba13c4dbb9fb7068dd6fb852
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/resolver__ui_is_seen_only_through_an_import.nova
@@ -0,0 +1,2 @@
+import ui::{signal}
+fn main() { }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_project/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_project/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..a4d7c503672f571f70763c1b83559617d247c5fa
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_project/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "site"
+version = "1.2.3"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_project/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_project/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..af2f27783ddee62fe156e9cbdf6089705abc586a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_project/src/main.nova
@@ -0,0 +1 @@
+fn main() { println("hi") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_public_outside/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_public_outside/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..76ad0349a60f5297950ed1d1a2f2026cfb851300
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_public_outside/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "site"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_public_outside/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_public_outside/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..af2f27783ddee62fe156e9cbdf6089705abc586a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__bundle_public_outside/src/main.nova
@@ -0,0 +1 @@
+fn main() { println("hi") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__call_export_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__call_export_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e8c53dd85f7d113659b39532ed54740ea64bc5f0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__call_export_1.nova
@@ -0,0 +1,2 @@
+import dom
+fn main() { body().set_text("x") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__call_export_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__call_export_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b0d6a2f4da52adc0a08edb7a7901617133846ea7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__call_export_2.nova
@@ -0,0 +1,4 @@
+import dom
+fn main() {
+    let _ = body().add_event_listener("click", |e: Event| println("x"))
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__check_extern.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__check_extern.nova
new file mode 100644
index 0000000000000000000000000000000000000000..da50afea3365b4350d02124fa2e567b18d882ad3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__check_extern.nova
@@ -0,0 +1,4 @@
+extern "C" {
+    fn abs(x: Int) -> Int
+}
+fn main() { println("${abs(-3)}") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__check_extern_js.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__check_extern_js.nova
new file mode 100644
index 0000000000000000000000000000000000000000..96f87c2b90b2c78a8ef77e792414fcf34f117c90
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__check_extern_js.nova
@@ -0,0 +1,4 @@
+extern "js" {
+    fn now() -> Int
+}
+fn main() { println("${now()}") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__dom_forged.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__dom_forged.nova
new file mode 100644
index 0000000000000000000000000000000000000000..84caf0ff949d65a6063283baea62e66cf106309e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__dom_forged.nova
@@ -0,0 +1,6 @@
+import dom
+fn main() {
+    let e = Element { box: 0 }
+    println("before")
+    e.set_text("x")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__dom_interval_panic.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__dom_interval_panic.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ef5a2c6ec66f2b0125bf32cc88dfcaad4b4600e0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__dom_interval_panic.nova
@@ -0,0 +1,5 @@
+import dom
+fn main() {
+    let _ = set_interval(1, | | panic("tick"))
+    println("main done")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__hydrate_panics_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__hydrate_panics_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5ce366d4cc5df4ddbdf498395e8e28c96fadf918
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__hydrate_panics_1.nova
@@ -0,0 +1,7 @@
+import ui
+import ui::html::{text}
+
+fn main() {
+    hydrate("#nope", | | text("x"))
+    println("not reached")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__hydrate_panics_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__hydrate_panics_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2fe0e7ae10a735bf20445855169336c5ce6df19f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__hydrate_panics_2.nova
@@ -0,0 +1,7 @@
+import ui
+import ui::html::{text}
+
+fn main() {
+    hydrate_with("#nope", "none", |s: String| text(s))
+    println("not reached")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__hydrate_panics_3.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__hydrate_panics_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d0474ae087738d9e4389523aa688fcd3dca110a0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__hydrate_panics_3.nova
@@ -0,0 +1,7 @@
+import ui
+import ui::html::{text}
+
+fn main() {
+    hydrate_with("#app", "none", |s: String| text(s))
+    println("not reached")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__import_dom.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__import_dom.nova
new file mode 100644
index 0000000000000000000000000000000000000000..63b14324f676259b35b3ac72fbbb6032035f27f0
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__import_dom.nova
@@ -0,0 +1,3 @@
+import dom::{Element}
+fn f(e: Element) -> Int { 0 }
+fn main() { println("x") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__import_ssr.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__import_ssr.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2a12c72e2fdb94cd4a6de24ce86764b345f8a70b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__import_ssr.nova
@@ -0,0 +1,4 @@
+import ui
+import ui::ssr::{Page, page}
+fn f(p: Page) -> Int { 0 }
+fn main() { println("x") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__project/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__project/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__project/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__project/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__project/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..af2f27783ddee62fe156e9cbdf6089705abc586a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__project/src/main.nova
@@ -0,0 +1 @@
+fn main() { println("hi") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__ui_imports_1.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__ui_imports_1.nova
new file mode 100644
index 0000000000000000000000000000000000000000..63857faf0d50c3da32a33d2a580a38c2f4dacf02
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__ui_imports_1.nova
@@ -0,0 +1,6 @@
+import ui
+import ui::html::{div, text}
+
+fn main() {
+    println(render_to_string(| | div([], [text("x")])))
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__ui_imports_2.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__ui_imports_2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..db117f2b734ee19fc6fe2022e718395346ba6b3c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__ui_imports_2.nova
@@ -0,0 +1,3 @@
+import a::b
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__ui_imports_3.nova b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__ui_imports_3.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bd31f3229bee86e6e950349e34a312ac3f249ca7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve-wasm/wasm_tests__ui_imports_3.nova
@@ -0,0 +1,3 @@
+import ui::nope
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..938487575b02fa2b7e2ac7cf5e9ea854a814e6d4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/nova.toml
@@ -0,0 +1,10 @@
+[package]
+name = "geo"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+units = { path = "units" }
+
+[dev-dependencies]
+ghost = { path = "ghost" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..65930aa84870d655f26f049a926d8ae748ecb73f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/src/lib.nova
@@ -0,0 +1,5 @@
+import units
+
+pub fn area() -> Int {
+    one()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/src/units.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/src/units.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f1feea4b6a5686960e50adbaaea3e282580a17f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/src/units.nova
@@ -0,0 +1,3 @@
+pub fn one() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/units/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/units/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..a3f03b356b3375f082dc9da459851f42c971c94d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/units/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "units"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/units/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/units/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c2c3919e78d06e733aa2bed1531983e533fe9f82
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/geo/units/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn one() -> Int {
+    2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..d90fd066dc9c137eb6c4b68e39fd3f76c46f9c13
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geo = { path = "geo" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ea4ce97341a6876a621aaf3cf8b4b055afb3d57b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_e0004_names_its_own_manifest/src/main.nova
@@ -0,0 +1,5 @@
+import geo
+
+fn main() {
+    println("${area()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/base/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/base/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..b6799c9d079e366146c40b5780a3b346e988adf1
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/base/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "base"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/base/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/base/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d942d69d048788e233b8dcd1789738e7c4edc883
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/base/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn zero() -> Int {
+    0
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/geo/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/geo/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..d9947a25a61073eb54fd311c72328883abbfedc5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/geo/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "geo-metry"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+base = { path = "../base" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/geo/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/geo/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b388f2b56eaafa839d5238ecf9ae2a8733b5c43c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/geo/src/lib.nova
@@ -0,0 +1,11 @@
+import units
+import base
+
+pub type Shape = | Dot | Ok(Int)
+
+pub fn area(s: Shape) -> Int {
+    unit() + zero()
+}
+
+@tset
+fn hidden() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/geo/src/units.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/geo/src/units.nova
new file mode 100644
index 0000000000000000000000000000000000000000..71b2e087f678dd9944127b1f58f7f4e43eafb1b2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/deps/geo/src/units.nova
@@ -0,0 +1,3 @@
+pub fn unit() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..758de5c1812932a6efe5a442d5d514061429e37b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geo-metry = { path = "deps/geo" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/src/helpers.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/src/helpers.nova
new file mode 100644
index 0000000000000000000000000000000000000000..de4fe7f2852fdd6977506c09aa713037fcefa395
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/src/helpers.nova
@@ -0,0 +1,3 @@
+pub fn twice(n: Int) -> Int {
+    n * 2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ff69d76ca24fe004a1a632c2665018791fd75745
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dependencys_modules_are_labelled_by_its_import_name/src/main.nova
@@ -0,0 +1,6 @@
+import geo_metry::{area, Shape}
+import helpers
+
+fn main() {
+    println("${area(Shape::Dot)} ${twice(1)}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/helper/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/helper/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..52ab7f309d4fff6effe0444f5d1a37b4bcb96117
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/helper/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "helper"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/helper/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/helper/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..8b108a0458d5e48fa1bc935d7ea7aab9489e4b0c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/helper/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn expected() -> Int {
+    42
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..50e5fb01c22b0d3a240227cabb0646770444705e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dev-dependencies]
+helper = { path = "helper" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a43e803d8573008727ccd2ff8e31d62f7810b082
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn answer() -> Int {
+    42
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..830d6bbbcfdffc88e6ef8f862fed7e9af3e2b2df
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/src/main.nova
@@ -0,0 +1,3 @@
+import helper
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/tests/answers.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/tests/answers.nova
new file mode 100644
index 0000000000000000000000000000000000000000..50d440b72cddc5d06850dcc061f22119ba835f71
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_dev_dependency_from_src_is_e0001_with_a_note/tests/answers.nova
@@ -0,0 +1,6 @@
+import app
+import helper
+
+fn check() -> Bool {
+    answer() == expected()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..d02d19ce00ad891090dde4ba91852cb25d41792a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+shapes = { path = "shapes" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/shapes/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/shapes/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..1d6b53c37b1da370f272bb553c8a35ea2089f732
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/shapes/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "shapes"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/shapes/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/shapes/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0bd3a0c74f9af6fd4e08bd3272c6249f7aa090d4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/shapes/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn area() -> Int {
+    2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..11c126a207c6026f1ed68f01e2ac10c0c37645bf
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/src/main.nova
@@ -0,0 +1,5 @@
+import shapes
+
+fn main() {
+    println("${area()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/src/shapes.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/src/shapes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..945d422f3f28125289cb140dafed5ac363ce3d17
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_module_that_is_also_a_dependency_is_e0004/src/shapes.nova
@@ -0,0 +1,3 @@
+pub fn area() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_package_module_named_for_a_std_module_is_e0093/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_module_named_for_a_std_module_is_e0093/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3fb3fb39ab78265dadc26948f5813666e2a3f3f2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_module_named_for_a_std_module_is_e0093/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_package_module_named_for_a_std_module_is_e0093/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_module_named_for_a_std_module_is_e0093/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5543aaedc0d029b29243ed345c678ef1825e79a3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_module_named_for_a_std_module_is_e0093/src/main.nova
@@ -0,0 +1,3 @@
+import ui
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_package_module_named_for_a_std_module_is_e0093/src/ui.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_module_named_for_a_std_module_is_e0093/src/ui.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cdc6c27d800bdf61531058e432b2217e45a34dc7
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_module_named_for_a_std_module_is_e0093/src/ui.nova
@@ -0,0 +1 @@
+pub fn f() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_package_without_a_library_seen_from_its_tests_is_e0001_with_a_note/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_without_a_library_seen_from_its_tests_is_e0001_with_a_note/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3fb3fb39ab78265dadc26948f5813666e2a3f3f2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_without_a_library_seen_from_its_tests_is_e0001_with_a_note/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_package_without_a_library_seen_from_its_tests_is_e0001_with_a_note/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_without_a_library_seen_from_its_tests_is_e0001_with_a_note/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f328e4d9d04c31d0d70d16d21a07d1613be9d577
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_without_a_library_seen_from_its_tests_is_e0001_with_a_note/src/main.nova
@@ -0,0 +1 @@
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_package_without_a_library_seen_from_its_tests_is_e0001_with_a_note/tests/api.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_without_a_library_seen_from_its_tests_is_e0001_with_a_note/tests/api.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b456f897f0720421b727633222820e5d7932a462
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_package_without_a_library_seen_from_its_tests_is_e0001_with_a_note/tests/api.nova
@@ -0,0 +1,3 @@
+import app
+
+fn check() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7e050a29e6aea4b7302f51b2eaa26fa608338e42
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026" # the last value, then a comment
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..945d422f3f28125289cb140dafed5ac363ce3d17
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn area() -> Int {
+    1
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/tests/app.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/tests/app.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0bd3a0c74f9af6fd4e08bd3272c6249f7aa090d4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/tests/app.nova
@@ -0,0 +1,3 @@
+pub fn area() -> Int {
+    2
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/tests/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/tests/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..c84563de74189b6a9bbf1e4d9a111c46c8447857
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/a_tests_module_named_for_the_package_is_e0004_at_its_package_table/tests/main.nova
@@ -0,0 +1,5 @@
+import app
+
+fn main() {
+    println("${area()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__a_package_named_std_stops_it/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_package_named_std_stops_it/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..5deab22c170b10420c2c4b899e8b0cd2e93a2301
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_package_named_std_stops_it/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "std"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__a_package_named_std_stops_it/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_package_named_std_stops_it/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f328e4d9d04c31d0d70d16d21a07d1613be9d577
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_package_named_std_stops_it/src/main.nova
@@ -0,0 +1 @@
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a6d4c3ef570fafd568da0ec57c43d1db5f449291
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/pkgs/geom/src/lib.nova
@@ -0,0 +1,20 @@
+import utils
+
+/// A point.
+pub record Point {
+    /// Across.
+    pub x: Int,
+    y: Int,
+}
+
+/// The area of a square of side `width()`.
+pub fn area() -> Int { width() * width() }
+
+fn secret() -> Int { 1 }
+
+record Hidden {
+    n: Int,
+}
+
+/// Makes a thing of a private type.
+pub fn make() -> Hidden { Hidden { n: 1 } }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/pkgs/geom/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/pkgs/geom/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5a5f2ffa92e38dbf5358d80c75cd1c32c63506f2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/pkgs/geom/src/utils.nova
@@ -0,0 +1,2 @@
+/// Three.
+pub fn width() -> Int { 3 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cbe504512be9a1114d1dd285c228855adf35fea5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__a_program_documents_every_item_and_its_library_only_its_exports/src/main.nova
@@ -0,0 +1,13 @@
+import geom
+
+/// Shows `p`.
+fn show(p: Point) -> String { "${p.x}" }
+
+/// A record of the app's own.
+record Local {
+    hidden: Int,
+}
+
+fn main() {
+    println(show(Point { x: area(), y: 0 }))
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_goes_with_its_type_and_a_trait_lists_its_impls/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_goes_with_its_type_and_a_trait_lists_its_impls/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..1d6b53c37b1da370f272bb553c8a35ea2089f732
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_goes_with_its_type_and_a_trait_lists_its_impls/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "shapes"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_goes_with_its_type_and_a_trait_lists_its_impls/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_goes_with_its_type_and_a_trait_lists_its_impls/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..ff702941f6260e249ced28fefa362303ca71cc7b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_goes_with_its_type_and_a_trait_lists_its_impls/src/lib.nova
@@ -0,0 +1,27 @@
+/// A rectangle.
+pub record Rect {
+    pub w: Int,
+    pub h: Int,
+}
+
+impl Rect {
+    /// Its area.
+    pub fn area(self) -> Int { self.w * self.h }
+
+    fn hidden(self) -> Int { 0 }
+}
+
+/// What can be measured.
+pub trait Measure {
+    /// The size.
+    fn size(self) -> Int
+}
+
+impl Measure for Rect {
+    fn size(self) -> Int { self.area() }
+}
+
+impl Int {
+    /// Twice it.
+    pub fn doubled(self) -> Int { self * 2 }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_on_arrays_names_its_methods_after_array/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_on_arrays_names_its_methods_after_array/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..e048efdc55372f8841515bebea8ecb5096376ef8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_on_arrays_names_its_methods_after_array/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "arrays"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_on_arrays_names_its_methods_after_array/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_on_arrays_names_its_methods_after_array/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..165c15cc339b92e38adc1ade7417e206b6980b94
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__an_impl_on_arrays_names_its_methods_after_array/src/lib.nova
@@ -0,0 +1,4 @@
+impl<T> [T] {
+    /// The second element.
+    pub fn second(self) -> T { self[1] }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__two_impls_of_one_type_number_a_method_they_share/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/model__two_impls_of_one_type_number_a_method_they_share/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3107880c4b6d38d37336cc68de5cb5cab6833c61
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__two_impls_of_one_type_number_a_method_they_share/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "pairs"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/model__two_impls_of_one_type_number_a_method_they_share/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/model__two_impls_of_one_type_number_a_method_they_share/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..5837ea144f0ab8aac50d4d8e9cc8d06a2074b476
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/model__two_impls_of_one_type_number_a_method_they_share/src/lib.nova
@@ -0,0 +1,15 @@
+/// Two of a kind.
+pub record Pair<T> {
+    pub a: T,
+    pub b: T,
+}
+
+impl Pair<Int> {
+    /// Their sum.
+    pub fn total(self) -> Int { self.a + self.b }
+}
+
+impl Pair<String> {
+    /// The two, joined.
+    pub fn total(self) -> String { "${self.a}${self.b}" }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d076f590d495542783f674a686ed1cdf45962d3c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/pkgs/geom/src/lib.nova
@@ -0,0 +1,5 @@
+import utils
+
+pub fn area() -> Int {
+    width() * width()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/pkgs/geom/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/pkgs/geom/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2131f4df757f47f89aa6aff9949c52c75322ebf6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/pkgs/geom/src/utils.nova
@@ -0,0 +1,3 @@
+pub fn width() -> Int {
+    3
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..12e4c6dafd220fb2c59252762c59f07d212c30ed
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_file_argument_in_src_sees_its_packages_dependencies/src/main.nova
@@ -0,0 +1,5 @@
+import geom
+
+fn main() {
+    println("${area()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d076f590d495542783f674a686ed1cdf45962d3c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/pkgs/geom/src/lib.nova
@@ -0,0 +1,5 @@
+import utils
+
+pub fn area() -> Int {
+    width() * width()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/pkgs/geom/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/pkgs/geom/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2131f4df757f47f89aa6aff9949c52c75322ebf6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/pkgs/geom/src/utils.nova
@@ -0,0 +1,3 @@
+pub fn width() -> Int {
+    3
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/src/a.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/src/a.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b39e929139215ffb51751dc544d782d5a436e6f4
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/src/a.nova
@@ -0,0 +1 @@
+pub fn x() -> Int { 1 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/src/b.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/src/b.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f444c0cc6df8b91084abd6f5a1366c4409ae3de8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/src/b.nova
@@ -0,0 +1 @@
+pub fn y() -> Int { 2 }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d32d57369e9c3338c0671a87368c6a8be809859b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__a_missing_sibling_is_probed_once_per_directory/src/main.nova
@@ -0,0 +1,5 @@
+import geom
+import a
+import b
+
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..11a626c01a2112d0ecc6b5d9dd1e169c2577cd0d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/pkgs/geom/src/lib.nova
@@ -0,0 +1,7 @@
+pub fn area() -> Int {
+    9
+}
+
+fn main() {
+    println("geom's demo")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..aa38713a6725c97173a79a1aca0a6ca22494251c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_entry_without_main_is_e0601_even_when_a_dependency_has_one/src/main.nova
@@ -0,0 +1,5 @@
+import geom
+
+fn run() {
+    println("${area()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cb4f02b745859ec317cdfed0ec0bb5e1fdcd5af8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/pkgs/geom/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn area() -> Int {
+    "nine"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..12e4c6dafd220fb2c59252762c59f07d212c30ed
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__an_error_in_a_dependency_is_reported_in_its_own_file/src/main.nova
@@ -0,0 +1,5 @@
+import geom
+
+fn main() {
+    println("${area()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_1/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_1/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee495fbbb3174af18ac2e1bbf9b1e1c81050fc2
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_1/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "both"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_1/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_1/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cb4f02b745859ec317cdfed0ec0bb5e1fdcd5af8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_1/src/lib.nova
@@ -0,0 +1,3 @@
+pub fn area() -> Int {
+    "nine"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_1/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_1/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f328e4d9d04c31d0d70d16d21a07d1613be9d577
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_1/src/main.nova
@@ -0,0 +1 @@
+fn main() {}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_2/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_2/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_2/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_2/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_2/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d076f590d495542783f674a686ed1cdf45962d3c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_2/src/lib.nova
@@ -0,0 +1,5 @@
+import utils
+
+pub fn area() -> Int {
+    width() * width()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_2/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_2/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2131f4df757f47f89aa6aff9949c52c75322ebf6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__check_2/src/utils.nova
@@ -0,0 +1,3 @@
+pub fn width() -> Int {
+    3
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..d076f590d495542783f674a686ed1cdf45962d3c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/pkgs/geom/src/lib.nova
@@ -0,0 +1,5 @@
+import utils
+
+pub fn area() -> Int {
+    width() * width()
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/pkgs/geom/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/pkgs/geom/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..2131f4df757f47f89aa6aff9949c52c75322ebf6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/pkgs/geom/src/utils.nova
@@ -0,0 +1,3 @@
+pub fn width() -> Int {
+    3
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f540a9b29d455f72d4779e56a632dafc274cd4c8
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/src/main.nova
@@ -0,0 +1,7 @@
+import geom
+import utils
+
+fn main() {
+    println("area ${area()}")
+    println(label())
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/src/utils.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/src/utils.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a0589a4c35a8b3eedd907e2a9027555809abf696
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__file_argument/src/utils.nova
@@ -0,0 +1,3 @@
+pub fn label() -> String {
+    "app utils"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..7ee1487b26709c7154540e059823e2458ff66895
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/nova.toml
@@ -0,0 +1,7 @@
+[package]
+name = "app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
+geom = { path = "pkgs/geom" }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/pkgs/geom/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/pkgs/geom/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..55c1b5342e91282058c6b8111722f45d87411907
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/pkgs/geom/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "geom"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/pkgs/geom/src/lib.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/pkgs/geom/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..11a626c01a2112d0ecc6b5d9dd1e169c2577cd0d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/pkgs/geom/src/lib.nova
@@ -0,0 +1,7 @@
+pub fn area() -> Int {
+    9
+}
+
+fn main() {
+    println("geom's demo")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..067ffd7ee44b3b64a9fff79fbb7f63f94885e4c3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/packages__two_mains/src/main.nova
@@ -0,0 +1,5 @@
+import geom
+
+fn main() {
+    println("app ${area()}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__build_output/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/project__build_output/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__build_output/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__build_output/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/project__build_output/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9aab0c6ab9df68ace482d23d2560d59b9c90687c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__build_output/src/main.nova
@@ -0,0 +1,3 @@
+fn main() {
+    println("hello from the project")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__import/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/project__import/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__import/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__import/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/project__import/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4d54cfac28433d14c23c993c5c8114c017e4ee41
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__import/src/main.nova
@@ -0,0 +1,5 @@
+import util::{greeting}
+
+fn main() {
+    println(greeting())
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__import/src/util.nova b/crates/nova-cli/tests/selfhost/cases/resolve/project__import/src/util.nova
new file mode 100644
index 0000000000000000000000000000000000000000..cba4f9c454eedc281047745e575444028ac69d41
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__import/src/util.nova
@@ -0,0 +1,3 @@
+pub fn greeting() -> String {
+    "from util"
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__init/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/project__init/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..da4e90da51f5816ec4f0f368c4a3dcc467c99a4a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__init/nova.toml
@@ -0,0 +1,6 @@
+[package]
+name = "my-app"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__init/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/project__init/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7d88ad02656f691e12690303da221ab8c79fbaf5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__init/src/main.nova
@@ -0,0 +1,6 @@
+fn greeting() -> String { "Hello, Nova!" }
+
+fn main() { println(greeting()) }
+
+@test
+fn greeting_says_hello() { assert_eq(greeting(), "Hello, Nova!") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__init_keeps/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/project__init_keeps/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..e4e51dfc5adc67bff303c8aa3cd34919b9fcfaad
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__init_keeps/nova.toml
@@ -0,0 +1,6 @@
+[package]
+name = "keeper"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__init_keeps/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/project__init_keeps/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9aab0c6ab9df68ace482d23d2560d59b9c90687c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__init_keeps/src/main.nova
@@ -0,0 +1,3 @@
+fn main() {
+    println("hello from the project")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__init_lib/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/project__init_lib/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..bff845301e4d60a8606fb8534efed579c8489ca6
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__init_lib/nova.toml
@@ -0,0 +1,6 @@
+[package]
+name = "both"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__init_lib/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/project__init_lib/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9aab0c6ab9df68ace482d23d2560d59b9c90687c
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__init_lib/src/main.nova
@@ -0,0 +1,3 @@
+fn main() {
+    println("hello from the project")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__new/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/project__new/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..581c572cc2e9d5f056622cf211b68c4362ff23d3
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__new/nova.toml
@@ -0,0 +1,6 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
+
+[dependencies]
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__new/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/project__new/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7d88ad02656f691e12690303da221ab8c79fbaf5
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__new/src/main.nova
@@ -0,0 +1,6 @@
+fn greeting() -> String { "Hello, Nova!" }
+
+fn main() { println(greeting()) }
+
+@test
+fn greeting_says_hello() { assert_eq(greeting(), "Hello, Nova!") }
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__paths/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/project__paths/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__paths/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__paths/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/project__paths/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..69ab06197cfab5c8bd590cd623c23ca963fcedb9
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__paths/src/main.nova
@@ -0,0 +1,7 @@
+fn main() {
+    let argv = args()
+    match argv.get(0) {
+        Some(s) => println(s)
+        None => println("no arguments")
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__spaces/nova.toml b/crates/nova-cli/tests/selfhost/cases/resolve/project__spaces/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..36d7bed7b3d1eaa9f173f7c3e97bdab3a845730a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__spaces/nova.toml
@@ -0,0 +1,4 @@
+[package]
+name = "demo"
+version = "0.1.0"
+edition = "2026"
diff --git a/crates/nova-cli/tests/selfhost/cases/resolve/project__spaces/src/main.nova b/crates/nova-cli/tests/selfhost/cases/resolve/project__spaces/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b61fde942559060d8ba2ab7ab95c7affbfc45bae
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/resolve/project__spaces/src/main.nova
@@ -0,0 +1,8 @@
+fn main() {
+    println("main")
+}
+
+@test
+fn passes() {
+    assert_eq(1 + 1, 2)
+}
diff --git a/crates/nova-driver/src/lib.rs b/crates/nova-driver/src/lib.rs
index 9adb4ce6beacf182b04152489254b3ce9d0bb287..199a4cedede019fc047f9ca3fa5c36ce560a6743 100644
--- a/crates/nova-driver/src/lib.rs
+++ b/crates/nova-driver/src/lib.rs
@@ -1415,11 +1415,8 @@ mod resolution_dumps {
     /// in a package's `src/` and `tests/`, and each other directory's
     /// `main.nova`.
     fn case_roots(cases: &Path) -> Vec<std::path::PathBuf> {
-        // Task 3 adds the WASM and test-mode cases' directories.
-        let Ok(listing) = std::fs::read_dir(cases) else {
-            return Vec::new();
-        };
-        let mut entries: Vec<_> = listing
+        let mut entries: Vec<_> = std::fs::read_dir(cases)
+            .expect("read the cases")
             .map(|e| e.expect("a directory entry").path())
             .collect();
         entries.sort();
@@ -1535,8 +1532,7 @@ mod resolution_dumps {
                     })
                 })
                 .collect();
-            // The corpus's floors, until Task 3 adds the cases of each mode.
-            for (run, floor) in runs.into_iter().zip([400, 35, 8]) {
+            for (run, floor) in runs.into_iter().zip([400, 60, 120]) {
                 let (roots, resolved) = run.join().expect("a mode's thread");
                 assert!(
                     roots >= floor && resolved * 2 > roots,
diff --git a/crates/nova-resolver/src/dump.rs b/crates/nova-resolver/src/dump.rs
index f9dab67949445f8aea064de45930b5259d0e665a..d3cea0a33a706caf930e52ccd472444c0fca068f 100644
--- a/crates/nova-resolver/src/dump.rs
+++ b/crates/nova-resolver/src/dump.rs
@@ -44,14 +44,15 @@ use crate::{
     ProgramResolution, Res,
 };
 
-/// Every form the dump can print, by which the differential checks that
+/// Every form the dump prints, by which the differential checks that std,
 /// the corpus and the cases hold each of them
 /// (`crates/nova-cli/tests/selfhost.rs`): a line's kind as `(Kind`, and a
 /// field's word values as `key=Value` or, for a `bool`, after its line's
-/// kind as `(Kind key=value` ([`forms_on`]). [`LATER_KINDS`] are the
-/// forms only a package, the WASM target or test mode prints, which 5.2b
-/// compares. An import the loader reported, `(Import Reported`, is a
-/// loose program's too: E0093 reports one.
+/// kind as `(Kind key=value` ([`forms_on`]). 5.2a's loose native programs
+/// print the first 43; a package's module key, the WASM target with its
+/// `missing` names, and test mode with its tests are 5.2b's (ADR 0054). An
+/// import the loader reported, `(Import Reported`, is a loose program's
+/// too: E0093 reports one, as a package's E0004 and noted E0001s do.
 pub const KINDS: &[&str] = &[
     "(Program",
     "(Program resolved=true",
@@ -96,24 +97,20 @@ pub const KINDS: &[&str] = &[
     "(Note",
     "(Fix",
     "(Edit",
-];
-
-/// The forms a loose program does not print natively outside test mode:
-/// 5.2b's packages (a module's package key), its WASM target (`missing`)
-/// and its test mode (` mode=test`; `@test` functions are stripped under
-/// `check`), and the two severities the resolver never gives.
-pub const LATER_KINDS: &[&str] = &[
-    "target=wasm",
+    // 5.2b (ADR 0054).
     "package=<n>",
+    "target=wasm",
+    "(Missing",
     "mode=test",
     "(Test",
     "(Test should_panic=true",
     "(Test should_panic=false",
-    "(Missing",
-    "(Help",
-    "(NoteDiagnostic",
 ];
 
+/// The forms no program prints: the two severities neither the package
+/// graph, the loader nor the resolver gives.
+pub const LATER_KINDS: &[&str] = &["(Help", "(NoteDiagnostic"];
+
 /// The label each file of a program is printed under: a user module's
 /// label, and std's modules' names in the file database, as `<std/core>`.
 pub type Labels = [(FileId, String)];
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-cli/tests/selfhost/compare.rs b/crates/nova-cli/tests/selfhost/compare.rs
index 4c1506ab4f358738f0a86ef4504099ca64894729..7bb656c3d525bf68b061fcfc7b797b47d65ff202 100644
--- a/crates/nova-cli/tests/selfhost/compare.rs
+++ b/crates/nova-cli/tests/selfhost/compare.rs
@@ -160,3 +160,35 @@ pub fn report(m: &Mismatch, dump: &Dump, dir: &Path, nova: &Path, selfhost: &Pat
         selfhost = selfhost.display(),
     )
 }
+
+/// A resolution dump as both sides are compared on `windows` (ADR 0054
+/// §7, and decision 4 of its §8): each `\` of E0004's and E0093's messages, the
+/// two that print a path, read as `/`. S0 writes such a path as
+/// `Path::join` does, with `\` on Windows, which the dump escapes as
+/// `\x5c`, where N writes `/`. Every other line, and every line elsewhere,
+/// is kept as it is.
+pub fn settle_paths(dump: &[u8], windows: bool) -> Vec<u8> {
+    if !windows {
+        return dump.to_vec();
+    }
+    let mut out = Vec::with_capacity(dump.len());
+    for line in dump.split_inclusive(|&b| b == b'\n') {
+        let path_message =
+            line.starts_with(b"  (Error \"E0004\" ") || line.starts_with(b"  (Error \"E0093\" ");
+        if !path_message {
+            out.extend_from_slice(line);
+            continue;
+        }
+        let mut rest = line;
+        while !rest.is_empty() {
+            if rest.starts_with(b"\\x5c") {
+                out.push(b'/');
+                rest = &rest[4..];
+            } else {
+                out.push(rest[0]);
+                rest = &rest[1..];
+            }
+        }
+    }
+    out
+}
diff --git a/crates/nova-cli/tests/selfhost/fuzz.rs b/crates/nova-cli/tests/selfhost/fuzz.rs
index 79c68ea7a1e6b047f7dbe1b37c059fbb07b83017..71a01748ce11702ea2134facaa7b7344ee8c2862 100644
--- a/crates/nova-cli/tests/selfhost/fuzz.rs
+++ b/crates/nova-cli/tests/selfhost/fuzz.rs
@@ -24,7 +24,12 @@
 //!    `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
 //!    §3.5): a loose corpus program that parses, mutated at the level of
 //!    names, beside four sibling modules ([`NAME_SIBLINGS`]), so that the
-//!    resolver's codes occur among the fixed seeds' inputs.
+//!    resolver's codes occur among the fixed seeds' inputs. 5.2b (spec
+//!    §4.5, ADR 0054) adds two siblings, each `name_mutate` then one edit
+//!    of its own: [`wasm_name_mutate`], for the WASM target's programs, and
+//!    [`package_name_mutate`], for a module of the package
+//!    [`PACKAGE_FILES`] lays out, with its sibling modules, a dependency,
+//!    a dev-dependency and a module named as a dependency.
 //!
 //! Beside them, [`edges`]: fixed inputs that no file under `cases/` can
 //! hold as it is, such as a lone `\r` or a NUL.
@@ -706,6 +711,136 @@ const NAME_ITEMS: [&str; 11] = [
     "trait Marked {}\nimpl Marked for () { fn mark(self) {} }\nimpl Marked for fn(Int) -> Int { fn mark(self) {} }\n",
 ];
 
+/// The package every [`package_name_mutate`] input is a module of, `(path,
+/// text)` under the fuzzer's directory: `app`, whose `src/` and `tests/`
+/// hold [`NAME_SIBLINGS`] and the inputs, and `src/` a module named
+/// `clash`, as a dependency is (E0004), and a `main.nova`, which the
+/// package graph requires; `app` has no `src/lib.nova`, so a `tests/`
+/// module's `import app` is E0001 with a note. Its dependencies:
+/// `dep`, whose library imports its own `inner` and keeps a private
+/// function and a `@test` (stripped under test mode, being a
+/// dependency's); `clash`; and the dev-dependency `devdep`, which a `src/`
+/// module may not import (E0001 with a note).
+pub const PACKAGE_FILES: [(&str, &str); 10] = [
+    (
+        "app/nova.toml",
+        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
+         [dependencies]\ndep = { path = \"../dep\" }\nclash = { path = \"../clash\" }\n\n\
+         [dev-dependencies]\ndevdep = { path = \"../devdep\" }\n",
+    ),
+    ("app/src/main.nova", "fn main() {}\n"),
+    ("app/src/clash.nova", "pub fn twin() -> Int { 0 }\n"),
+    (
+        "dep/nova.toml",
+        "[package]\nname = \"dep\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+    ),
+    (
+        "dep/src/lib.nova",
+        "import inner\npub fn lift() -> Int { ground() }\npub record Gauge { n: Int }\n\
+         fn sunk() -> Int { 0 }\n@test\nfn a_dependencys_test() {}\n",
+    ),
+    ("dep/src/inner.nova", "pub fn ground() -> Int { 0 }\n"),
+    (
+        "clash/nova.toml",
+        "[package]\nname = \"clash\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+    ),
+    ("clash/src/lib.nova", "pub fn twin() -> Int { 1 }\n"),
+    (
+        "devdep/nova.toml",
+        "[package]\nname = \"devdep\"\nversion = \"0.1.0\"\nedition = \"2026\"\n",
+    ),
+    ("devdep/src/lib.nova", "pub fn expect() -> Int { 2 }\n"),
+];
+
+/// The forms [`PACKAGE_FILES`]' manifests are written in, `app`'s and
+/// `dep`'s, input `i` of a seed in a package whose manifests are form `i %
+/// 4` (5.2b, ADR 0054 §5): each a form N's manifest reader takes and
+/// nova-pm reads with no M-code, so that each of the reader's forms meets
+/// the loader, E0004's line and span in a manifest included. Form 0 is
+/// [`PACKAGE_FILES`]' own. Form 1 has comments, a header with spaces, a
+/// name in `'...'`, a pre-release, more keys, the dependency tables in
+/// another order and `[package]`'s last value before a comment; form 2 the
+/// dependency tables as inline tables before the header, the keys in
+/// another order and the largest version; form 3 a byte-order mark and
+/// `\r\n`. `dep`'s forms add dev-dependencies, which nova-pm reads and,
+/// in a dependency's manifest, never resolves: a version entry, and a
+/// path to no package.
+pub fn package_manifests() -> [(String, String); 4] {
+    let app = PACKAGE_FILES[0].1.to_string();
+    let dep = PACKAGE_FILES[3].1.to_string();
+    let bom = char::from_u32(0xFEFF)
+        .expect("the byte-order mark")
+        .to_string();
+    [
+        (app.clone(), dep.clone()),
+        (
+            "# The fuzzer's package, written another way.\n\n[ package ]\nname = 'app'   # its name\n\
+             version = \"1.2.3-alpha.1\"\ndescription = \"d\"\nauthors = [\"a\", 'b',]\n\
+             edition = '2026' # the last value\n\n[dev-dependencies]\ndevdep = {path='../devdep'}\n\n\
+             [dependencies]\nclash = { path = \"../clash\" }   # a module's name too\n\
+             dep = { path = \"../dep\" }\n"
+                .to_string(),
+            format!("{dep}\n[dev-dependencies]\njson = \"^1.2\"\n"),
+        ),
+        (
+            "dependencies = { dep = { path = \"../dep\" }, clash = { path = \"../clash\" } }\n\
+             dev-dependencies = { devdep = { path = \"../devdep\" } }\n\n[package]\nedition = \"2026\"\n\
+             version = \"18446744073709551615.0.0\"\nname = \"app\"\nlicense = \"MIT\"\n"
+                .to_string(),
+            dep.replace("0.1.0", "0.0.18446744073709551615")
+                + "\n[dev-dependencies]\nlocal = { path = \"../nowhere\" }\nreg = { version = \"=1\" }\n",
+        ),
+        (
+            format!("{bom}{}", app.replace('\n', "\r\n")),
+            dep.replace('\n', "\r\n"),
+        ),
+    ]
+}
+
+/// What [`wasm_name_mutate`] adds to a WASM program, one of them, after
+/// [`name_mutate`]'s mutations: at the top (`true`) an import of `ui::ssr`,
+/// which only the native target has (E0092), of `ui::router`, or of
+/// `dom`, which only WASM has; or at the end an `extern "C"` or `extern
+/// "JS"` block (E0090).
+const WASM_ADDITIONS: [(bool, &str); 5] = [
+    (true, "import ui::ssr"),
+    (true, "import ui::router"),
+    (true, "import dom"),
+    (false, "extern \"C\" {\n    fn abs(x: Int) -> Int\n}\n"),
+    (false, "extern \"JS\" {\n    fn now() -> Float\n}\n"),
+];
+
+/// What [`package_name_mutate`] adds to a module of [`PACKAGE_FILES`]'
+/// `app`, one of them, at the top, after [`name_mutate`]'s mutations: an
+/// import of the dependency whole, by a list and by a private name, of
+/// the dev-dependency, of `clash`, a module and a dependency at once
+/// (E0004), and of the package itself, which a `tests/` module sees by
+/// its name but which has no library.
+const PACKAGE_ADDITIONS: [(bool, &str); 6] = [
+    (true, "import dep"),
+    (true, "import dep::{lift, Gauge}"),
+    (true, "import dep::{sunk}"),
+    (true, "import devdep"),
+    (true, "import clash"),
+    (true, "import app"),
+];
+
+/// `text` with one of `additions` that the rng picks, at the top or at the
+/// end, when the result still parses.
+fn add_one(rng: &mut SplitMix64, text: String, additions: &[(bool, &str)]) -> String {
+    let (top, addition) = *rng.pick(additions);
+    let next = if top {
+        format!("{addition}\n{text}")
+    } else {
+        format!("{}\n{addition}", text.trim_end_matches('\n'))
+    };
+    if named_items(&next).is_some() {
+        next
+    } else {
+        text
+    }
+}
+
 /// One top-level item, as the name fuzzer edits it: its span, and its
 /// name's span, its name and whether it is `pub` when it has one, with
 /// whether it introduces a type name (a record or a sum) and whether it is
@@ -854,6 +989,22 @@ pub fn name_mutate(rng: &mut SplitMix64, source: &str, stem: &str) -> String {
     text
 }
 
+/// [`name_mutate`] for a WASM program (5.2b), then one of
+/// [`WASM_ADDITIONS`].
+pub fn wasm_name_mutate(rng: &mut SplitMix64, source: &str, stem: &str) -> String {
+    let text = name_mutate(rng, source, stem);
+    add_one(rng, text, &WASM_ADDITIONS)
+}
+
+/// [`name_mutate`] for a module of [`PACKAGE_FILES`]' `app` (5.2b), then
+/// one of [`PACKAGE_ADDITIONS`]: whether the module goes in `tests/` (one
+/// in three) rather than `src/`, and its text.
+pub fn package_name_mutate(rng: &mut SplitMix64, source: &str, stem: &str) -> (bool, String) {
+    let tests = rng.one_in(3);
+    let text = name_mutate(rng, source, stem);
+    (tests, add_one(rng, text, &PACKAGE_ADDITIONS))
+}
+
 /// Whether `source` lexes and parses without an error: what the name
 /// fuzzer starts from.
 pub fn parses(source: &str) -> bool {
````

### Task 4: ADR 0054 and the records

Spec §8; decisions 9 to 19, 24 and 25.

**Files:**
- Create:
  `docs/adr/0054-packages-the-wasm-target-and-test-mode-in-the-resolution-dump.md`
- Modify:
  `docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md`,
  `CHANGELOG.md`, `docs/phase-5-plan.md`, `nova-spec/00-MASTER-SPEC.md`,
  `nova-spec/11-PARSER.md`

- [ ] **Step 1: The records.** `patch_block t4-code`:
  - **ADR 0054,** packages, the WASM target and test mode in the
    resolution dump: `Accepted (2026-11-04)`, with the branch (decision
    19); §1 the dump's new forms
    and one example of every form; §2 the cases; §3 the fuzzer and its
    floors; §4 the corpus, 318 natively (251 loose, 67 package files), 39
    for WASM, 9 with tests; §5 N's manifest reader and package graph, the
    reader's table and the 229 manifests (decisions 16 and 23), package
    identity and exactly which inputs can differ (decisions 23 and 25);
    §6 what N ports;
    §7 the differential on both sides and Windows' paths (decision 14);
    §8 part A's defaults and the spec's §4.1 correction (decision 12); §9
    CI's time, the speed-up and option B with every local figure, CI's
    measured figures and the user's acceptance of about 60 s on Windows
    (decisions 10, 17, 18 and 24); §10 the live heap; its consequences.
  - **ADR 0053:** a dated note that ADR 0054 extends the dump.
  - **`CHANGELOG.md`,** Unreleased: nothing a user runs changes; the
    hidden dump and N gain packages, the WASM target and test mode.
  - **Dated notes,** each dated 2026-11-04, naming the branch
    `claude/loving-archimedes-frtf5q-52b`: `docs/phase-5-plan.md`'s 5.2;
    `00-MASTER-SPEC.md`'s Phase 5; `11-PARSER.md` §4.
- [ ] **Step 2: Read them against the code.** Each record's claim against
  the tree: the ADR's decisions against Tasks 1 to 3 and this plan's
  decisions; its examples against `nova check --dump-resolution` of their
  sources, by the prototype's script, which writes the ADR's block
  afresh, and a comparison:

```sh
mkdir -p "$(dirname "$P/plan52b/adr_examples.py")"
cat > "$P/plan52b/adr_examples.py" <<'EOF'
#!/usr/bin/env python3
"""ADR 0054's examples, one of every form 5.2b adds to the resolution dump:
for each, small sources written to a fresh directory, S0's resolution dump
of the root named (`nova check --dump-resolution [FLAGS] ROOT`, or `--std
[FLAGS]` for std's part), and the lines a list of selectors keeps, with
every parent of a kept line, each line then closed as a tree of its own.
A selector is a regex a line must match, or a (parent, child) pair of
regexes. Prints Markdown. Usage: adr_examples.py NOVA"""
import os, re, subprocess, sys, tempfile

NOVA = sys.argv[1]
ALL = [r"."]

def manifest(name, extra=""):
    return f'[package]\nname = "{name}"\nversion = "0.1.0"\nedition = "2026"\n{extra}'

EXAMPLES = [
    ("package=<n>: a package's modules, its dependency's labelled by its import name",
     [("app/nova.toml", manifest("app", '[dependencies]\nshapes = { path = "shapes" }\n')),
      ("app/src/main.nova", "import shapes\nfn main() {}\n"),
      ("app/shapes/nova.toml", manifest("shapes")),
      ("app/shapes/src/lib.nova", "import units\npub fn area() -> Int { unit() }\n"),
      ("app/shapes/src/units.nova", "pub fn unit() -> Int { 1 }\n")],
     [], "app/src/main.nova",
     [r"^\(Program", r"^  \(Module ", r"\(Import ", r"\(Importable ", r"\(StdStart"]),
    ("A tests/ module, and E0001 with a note: the package itself, which has no library",
     [("app/nova.toml", manifest("app")),
      ("app/src/main.nova", "fn main() {}\n"),
      ("app/tests/api.nova", "import app\n")],
     [], "app/tests/api.nova", ALL),
    ("E0001 with a note: a dev-dependency imported from src/",
     [("app/nova.toml", manifest("app", '[dev-dependencies]\nhelper = { path = "helper" }\n')),
      ("app/src/main.nova", "import helper\nfn main() {}\n"),
      ("app/helper/nova.toml", manifest("helper")),
      ("app/helper/src/lib.nova", "pub fn h() {}\n")],
     [], "app/src/main.nova", [r"^\(Diagnostics", (r"\(Error", r".")]),
    ("E0004, its Secondary in the manifest, labelled nova.toml",
     [("app/nova.toml", manifest("app", '[dependencies]\nshapes = { path = "shapes" }\n')),
      ("app/src/main.nova", "import shapes\nfn main() {}\n"),
      ("app/src/shapes.nova", "pub fn area() -> Int { 1 }\n"),
      ("app/shapes/nova.toml", manifest("shapes")),
      ("app/shapes/src/lib.nova", "pub fn area() -> Int { 2 }\n")],
     [], "app/src/main.nova", [r"^\(Diagnostics", (r"\(Error", r".")]),
    ("A package graph's diagnostics: the program stops before the loader",
     [("app/nova.toml", "[package]\nname = \"app\"\n"),
      ("app/src/main.nova", "fn main() {}\n")],
     [], "app/src/main.nova", [r"^\(Program", r"^\(Diagnostics", (r"\(Error", r".")]),
    ("mode=test, Test should_panic=false and true: nova test's resolution of a tests/ file",
     [("app/nova.toml", manifest("app")),
      ("app/src/lib.nova", "pub fn one() -> Int { 1 }\n@test\nfn in_lib() {}\n"),
      ("app/tests/api.nova", "import app\n@test\nfn works() {}\n@test(should_panic)\nfn panics() { panic(\"no\") }\n")],
     ["--test"], "app/tests/api.nova",
     [r"^\(Program", r"^  \(Module ", r"\(Test ", r"^\(Diagnostics"]),
    ("E0084 and E0085, under test mode",
     [("main.nova", "@test\nfn takes(n: Int) {}\n@test(nope)\nfn t() {}\nfn main() {}\n")],
     ["--test"], "main.nova", [r"^\(Program", (r"\(Error", r"."), r"^\(Diagnostics"]),
    ("Std mode=test, from nova check --dump-resolution --std --test",
     None, ["--test"], None,
     [r"^\(Std", r'^  \(Module 21 ', r'\(Scope 0', (r"\(Scope 0", r'"assert_eq"'), r"^\(Diagnostics"]),
    ("target=wasm, and E0090 and E0092",
     [("page.nova", "import ui::ssr\nextern \"C\" {\n    fn abs(x: Int) -> Int\n}\nfn main() {}\n")],
     ["--target", "wasm"], "page.nova",
     [r"^\(Program", r"^\(Diagnostics", (r"\(Error", r".")]),
    ("Std target=wasm, Missing, from nova check --dump-resolution --std --target wasm",
     None, ["--target", "wasm"], None,
     [r"^\(Std", r'^  \(Module 10 ', r'\(Missing "args" ', r"^\(Diagnostics"]),
]


def dump(files, flags, root):
    with tempfile.TemporaryDirectory() as d:
        if files is None:
            args = ["--std", *flags]
        else:
            for name, src in files:
                path = os.path.join(d, name)
                os.makedirs(os.path.dirname(path), exist_ok=True)
                open(path, "w").write(src)
            args = [*flags, root]
        out = subprocess.run([NOVA, "check", "--dump-resolution", *args], cwd=d,
                             capture_output=True, text=True).stdout
    lines = out.splitlines()
    if files is not None:
        assert lines[0] == "== " + root, lines[0]
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


def header(forms, files, flags, root):
    out = [";; " + forms + ":"]
    if root:
        out.append(";; nova check --dump-resolution " + " ".join([*flags, root]))
    for name, src in files or []:
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
for i, (forms, files, flags, root, selectors) in enumerate(EXAMPLES):
    lines = select(dump(files, flags, root), selectors)
    if not lines:
        sys.exit("nothing kept for " + forms)
    if i:
        print()
    print("\n".join(header(forms, files, flags, root)))
    print("\n".join(lines))
print("```")
EOF
```

```sh
mkdir -p "$(dirname "$P/plan52b/adrcheck.sh")"
cat > "$P/plan52b/adrcheck.sh" <<'EOF'
#!/bin/bash
# adrcheck.sh: run from the repository's root, after the build, with $P set
# and $P/plan52b/adr_examples.py written. Regenerates ADR 0054's examples, one of
# every form, from S0's `nova check --dump-resolution` of their sources,
# and compares them with the ADR's block, byte for byte.
set -u
: "${P:?set P to the scratch directory}"
adr=docs/adr/0054-packages-the-wasm-target-and-test-mode-in-the-resolution-dump.md
python3 -I "$P/plan52b/adr_examples.py" "$PWD/target/debug/nova" > "$P/plan52b/adr-examples.txt" || exit 1
awk '/^\*\*One example of every new form\*\*/ { on = 1 } on && /^```$/ { n++; print; if (n == 2) exit; next } on && n == 1 { print }' "$adr" > "$P/plan52b/adr-block.txt"
n=$(grep -c '^;; ' "$P/plan52b/adr-block.txt")
if cmp -s "$P/plan52b/adr-examples.txt" "$P/plan52b/adr-block.txt"; then echo "adr: $n header lines, identical"; else
  echo "adr: the examples differ"; diff "$P/plan52b/adr-block.txt" "$P/plan52b/adr-examples.txt" | head -20; exit 1; fi
EOF
```

  ```sh
  cargo build --locked --workspace
  git diff --check HEAD
  P="$P" bash "$P/plan52b/adrcheck.sh"
  ```

  Expected: `git diff --check` prints nothing, and `adr: 53 header
  lines, identical`.
- [ ] **Step 3: The checks,** as the conventions list them.
- [ ] **Step 4: Commit**
  `docs: ADR 0054, packages, the WASM target and test mode, and the
  notes for 5.2b`.

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index c3e2410a0a3106ee6ec82b78e8b11edc0e8f453c..a6967561b943775d6a1d66cea97f69b87bca03b6 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -197,7 +197,10 @@ resolver for loose programs and the resolution dump (spec
   part, the same in every native program, is printed once by `--std`, and
   a program's part names std's definitions by std-relative ids. A root
   that cannot be read is `IOERR` and its kind, and the exit code 2; a root
-  in a package's `src/` or `tests/` is `UNSUPPORTED package` until 5.2b.
+  in a package's `src/` or `tests/` is a module of its package, with the
+  package graph's diagnostics first, and `--target wasm` and `--test` (as
+  `nova test` resolves) dump the WASM target's and test mode's resolution
+  (5.2b, ADR 0054).
   std is parsed once for all the roots. It is absent from `--help` and
   carries no stability promise. It is taken only when `--dump-resolution`
   comes first after `check`, and parsed apart, so every other `nova check`
@@ -217,7 +220,13 @@ resolver for loose programs and the resolution dump (spec
   `check` gives natively occurs. A change to how names resolve now
   changes both compilers in one pull request. With the debug runtime N
   resolves and dumps the corpus's loose programs 1.5 to 2.0× as slowly as
-  S0's debug build (ADR 0053).
+  S0's debug build (ADR 0053). From 5.2b N also loads a package's module
+  with its path dependencies, reading `nova.toml` with a minimal reader
+  of its own, and resolves for `--target wasm` and under `--test`, compared
+  with S0 on the corpus's packages, WASM programs and tests, the cases of
+  each mode and the fuzzer's (ADR 0054). Nothing a user sees changes with
+  5.2b: `nova check`, `nova test` and `nova build` give what they gave,
+  byte for byte, on every file and package of the repository.
 
 ### Changed
 
diff --git a/docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md b/docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md
index 73599cac5da8613256d4dcd497123d0859ff5819..5cb217a8f2899475b02f1d9e55357bee2ba08905 100644
--- a/docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md
+++ b/docs/adr/0053-the-self-hosted-loader-and-resolver-and-the-resolution-dump.md
@@ -81,6 +81,11 @@ part plus the program's `item` is its merged item index.
 module's label (a loose file's stem), or a std module's name in the file
 database, `<std/core>`. A file with no label prints `?`.
 
+*(2026-11-04: 5.2b extends this grammar, ADR 0054 §1: a package's root is
+dumped, not `UNSUPPORTED package`; ` mode=test` and `(Test` come with
+`--test`; `target=wasm` and `(Missing` are compared. The example of
+`UNSUPPORTED` below is 5.2a's.)*
+
 **The grammar.** Every line ends with `\n`, the last included. `I(d)` is
 `d` times two spaces. As in ADR 0052, a line's `)` comes right after its
 last child's.
diff --git a/docs/adr/0054-packages-the-wasm-target-and-test-mode-in-the-resolution-dump.md b/docs/adr/0054-packages-the-wasm-target-and-test-mode-in-the-resolution-dump.md
new file mode 100644
index 0000000000000000000000000000000000000000..90bc9c8649b2ef91ab01cd4c2d5a9a896d0c7370
--- /dev/null
+++ b/docs/adr/0054-packages-the-wasm-target-and-test-mode-in-the-resolution-dump.md
@@ -0,0 +1,803 @@
+# ADR 0054 — Packages, the WASM target and test mode in the resolution dump
+
+## Status
+
+Accepted (2026-11-04). Branch `claude/loving-archimedes-frtf5q-52b`
+(`docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
+§4). It extends ADR 0053, whose §1 grammar and examples stay as they were
+for 5.2a's forms: §1 here is what 5.2b adds to S0's dump and to the hidden
+form; §2 the cases; §3 the fuzzer; §4 the corpus (part A, S0's side);
+§5 N's manifest reader and package graph; §6 what N ports and what it
+does not; §7 the differential on both sides; §8 the decisions part A took
+and the spec's correction; §9 CI's time; §10 the live heap (part B).
+
+**Why a new ADR, not ADR 0053 amended:** ADR 0053 records 5.2a's state,
+accepted and merged, and promises this one for 5.2b (its §2 and its
+Status); the spec's records (§8) name ADR 0054 for packages, WASM, test
+mode and the live heap. A reader porting the dump reads ADR 0053 §1, then
+§1 here, which only adds forms and changes one: `UNSUPPORTED package`,
+which S0 no longer prints. ADR 0053 §1 gains a dated note that says so.
+
+## Decision
+
+### 1. The dump of packages, the WASM target and test mode
+
+**Roots.** A root is what `nova check FILE` takes (`Program::for_file`): a
+file directly in the `src/` or `tests/` of a directory holding a
+`nova.toml` (`package_of`, ADR 0053 §3) is a module of that package, any
+other a loose program. 5.2a printed `UNSUPPORTED package` for the first;
+S0 now dumps its program: the package graph read from the manifests, as
+`nova_pm::graph` reads it, then the loader from that one root, breadth
+first, with `src/` and `tests/` siblings, path dependencies' `src/lib.nova`
+and their modules, E0004, E0093 and E0001's two notes, then `check`'s
+stop, strip and resolver. A whole package (`nova check` with no file in a
+project, `nova test`'s `src/main.nova`, `src/lib.nova` and `tests/*` in
+one program) is not a root: the hidden form takes files, as 5.2a's did.
+
+**Not synced.** `nova check FILE` syncs a package first (`project::sync`,
+which may write `nova.lock` and fetch registry packages); the dump syncs
+nothing and writes nothing, and reads registry packages only from what is
+cached, as the language server does. A registry package's warnings are
+dropped, as `check` hides them; no input of the differential has one.
+
+**The graph's diagnostics** come first in `(Diagnostics`; with an error
+among them the program stops before the loader, `resolved=false` with no
+module, as `check` stops. They are nova-pm's (M-codes), which N's minimal
+manifest reader (§5) does not give: no case, corpus file or fuzz input has a manifest the graph
+warns about, so they are S0's alone, and the extraction drops such
+programs (§2).
+
+**Labels.** A module's label is the loader's (spec §4.1): its stem, or
+`tests/<stem>` for the root package's `tests/`, or a dependency's import
+name for its `lib.nova` and `<import name>/<stem>` for its other modules.
+A manifest is labelled `nova.toml` for the root package's and `<import
+name>/nova.toml` for a dependency's, so E0004's secondary label, in the
+manifest, names a place without a path; without a graph (a root manifest
+that does not parse) every file the graph's diagnostics name is the root's
+`nova.toml`. A dependency's manifest that gives an M-code is not in the
+graph, so its file has no label, and its M-code's labels print `?` in
+place of one (`(Primary ?:82..83 ...)`); N refuses such a manifest (§5).
+
+**The package key** is each module's `package=`, the graph's `PackageId`:
+0 for the root package, then the dependencies in the graph's breadth-first
+order; `None` for a loose module and for std's. The graph itself is not
+printed: the resolver sees only the key (`owned`, `same_package`), and the
+labels show which package a module came from.
+
+**Paths in messages.** E0004's message names the sibling and the
+manifest by their paths, and a package module's E0093 its file, each as
+`Path::join` writes it from the root as given, with `\` on Windows, as
+5.2a's E0093 does (ADR 0053 §5). The differential compares these two
+messages with `\` read as `/` on Windows (§7).
+
+**The WASM target.** `--target wasm` parsed in 5.2a and resolved with the
+WASM std, its 13 WASM-only builtins in std's scopes, `IMPORTED_STD`'s
+modules, E0090 and E0092; it was complete, and is now in `KINDS`:
+`target=wasm`, and std's part's `(Missing` lines.
+
+**Test mode.** `--test` resolves as `nova test` does
+(`FrontendContext::check(true)`, which `build_test_program` runs): the root
+package's and a loose program's `@test` functions are kept and a
+dependency's stripped (`strip_test_functions` with `keep_root_tests`),
+`std/test` joins std after its other modules, registered as `<std/test>`
+and globbed into every module, std's own included, and a `tests/` file's
+tests are named `<file stem>::<function>` (`name_tests`). The tests are
+listed as `(Test` lines, in the resolver's order; E0083 to E0085 apply. The
+part's header says ` mode=test`, after its target, and says nothing under
+`check`, so 5.2a's dumps are unchanged byte for byte. `--std --test`
+prints std's test-mode part, which `std/test` changes: one more module,
+its definitions, and its names in every std module's scope (22 modules,
+716 definitions, 551 items, 8,098 lines). A driver test checks that the
+dump's tests are `nova test`'s, by id, name and `should_panic`, for a
+package's `tests/` file and a loose file.
+
+**What the hidden form takes.** After `check --dump-resolution`: `--std`,
+`--target native|wasm`, `--test` and files, each option once, `--` before a
+file that starts with `-`. `--test` with `--target wasm` is refused with
+one line, `error: --dump-resolution --test is native only, as nova test
+is`, nothing on stdout and the exit code 2, as `--std` with a file is
+(ADR 0053 §1): `nova test` has no `--target`. `--test=…`, `--test --test`,
+`--tests` and the rest are clap's errors under the form's own usage line.
+`check --test …` without `--dump-resolution` first is the base's error,
+byte for byte.
+
+**The grammar**, ADR 0053 §1's with these changes:
+
+```
+part       = "IOERR " kind "\n" | program diagnostics      -- "UNSUPPORTED package" is gone
+program    = "(Program target=" target [" mode=test"] " resolved=" bool
+             { "\n" module(1) } [ ... ] ")\n"              -- as in ADR 0053 §1
+std        = "(Std target=" target [" mode=test"] " modules=" uint " defs=" uint
+             " items=" uint ... ")\n"                       -- as in ADR 0053 §1
+module(1)  = I(1) "(Module " uint " " string " package=" ("None" | uint) ...
+             -- a package's module: its key in the graph, 0 for the root package
+place      = (string | "?") ":" uint ".." uint
+             -- a manifest's label is "nova.toml" or "<import name>/nova.toml"
+```
+
+`KINDS` gains `package=<n>`, `target=wasm`, `(Missing`, `mode=test`,
+`(Test`, `(Test should_panic=true` and `(Test should_panic=false`;
+`LATER_KINDS` keeps the two severities no program prints, `(Help` and
+`(NoteDiagnostic`.
+
+**One example of every new form**, each S0's dump of the sources after
+it, from the directory that holds them, reduced as ADR 0053's are:
+
+```
+;; package=<n>: a package's modules, its dependency's labelled by its import name:
+;; nova check --dump-resolution app/src/main.nova
+;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
+;;     edition = "2026"\n[dependencies]\nshapes = { path = "shapes" }\n
+;; app/src/main.nova: import shapes\nfn main() {}\n
+;; app/shapes/nova.toml: [package]\nname = "shapes"\nversion = "0.1.0"\n
+;;     edition = "2026"\n
+;; app/shapes/src/lib.nova: import units\npub fn area() -> Int { unit() }\n
+;; app/shapes/src/units.nova: pub fn unit() -> Int { 1 }\n
+(Program target=native resolved=true
+  (Module 0 "main" package=0
+    (Import "shapes" 1)
+    (Importable "shapes" 1))
+  (Module 1 "shapes" package=1
+    (Import "units" 2)
+    (Importable "units" 2))
+  (Module 2 "shapes/units" package=1
+    (Importable "lib" 1))
+  (StdStart module=3 def=3 item=5))
+
+;; A tests/ module, and E0001 with a note: the package itself, which has no library:
+;; nova check --dump-resolution app/tests/api.nova
+;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
+;;     edition = "2026"\n
+;; app/src/main.nova: fn main() {}\n
+;; app/tests/api.nova: import app\n
+(Program target=native resolved=false
+  (Module 0 "tests/api" package=0
+    (Import "app" Reported)))
+(Diagnostics
+  (Error "E0001" "cannot find module `app`"
+    (Primary "tests/api":7..10 "no such module")
+    (Note "`app` has no src/lib.nova")))
+
+;; E0001 with a note: a dev-dependency imported from src/:
+;; nova check --dump-resolution app/src/main.nova
+;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
+;;     edition = "2026"\n[dev-dependencies]\nhelper = { path = "helper" }\n
+;; app/src/main.nova: import helper\nfn main() {}\n
+;; app/helper/nova.toml: [package]\nname = "helper"\nversion = "0.1.0"\n
+;;     edition = "2026"\n
+;; app/helper/src/lib.nova: pub fn h() {}\n
+(Diagnostics
+  (Error "E0001" "cannot find module `helper`"
+    (Primary "main":7..13 "no such module")
+    (Note "`helper` is a dev-dependency, which only `tests/` files can import")))
+
+;; E0004, its Secondary in the manifest, labelled nova.toml:
+;; nova check --dump-resolution app/src/main.nova
+;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
+;;     edition = "2026"\n[dependencies]\nshapes = { path = "shapes" }\n
+;; app/src/main.nova: import shapes\nfn main() {}\n
+;; app/src/shapes.nova: pub fn area() -> Int { 1 }\n
+;; app/shapes/nova.toml: [package]\nname = "shapes"\nversion = "0.1.0"\n
+;;     edition = "2026"\n
+;; app/shapes/src/lib.nova: pub fn area() -> Int { 2 }\n
+(Diagnostics
+  (Error "E0004" "`shapes` is both a module of this package and a dependency: app/src/shapes.nova, and the entry at app/nova.toml:6"
+    (Primary "main":7..13 "this import is ambiguous")
+    (Secondary "nova.toml":73..79 "the dependency")))
+
+;; A package graph's diagnostics: the program stops before the loader:
+;; nova check --dump-resolution app/src/main.nova
+;; app/nova.toml: [package]\nname = "app"\n
+;; app/src/main.nova: fn main() {}\n
+(Program target=native resolved=false)
+(Diagnostics
+  (Error "M0002" "`[package]` has no `version`"
+    (Primary "nova.toml":0..22 "add `version = \x22...\x22` to this table"))
+  (Error "M0002" "`[package]` has no `edition`"
+    (Primary "nova.toml":0..22 "add `edition = \x22...\x22` to this table")))
+
+;; mode=test, Test should_panic=false and true: nova test's resolution of a tests/ file:
+;; nova check --dump-resolution --test app/tests/api.nova
+;; app/nova.toml: [package]\nname = "app"\nversion = "0.1.0"\n
+;;     edition = "2026"\n
+;; app/src/lib.nova: pub fn one() -> Int { 1 }\n@test\nfn in_lib() {}\n
+;; app/tests/api.nova: import app\n@test\nfn works() {}\n@test(should_panic)\n
+;;     fn panics() { panic("no") }\n
+(Program target=native mode=test resolved=true
+  (Module 0 "tests/api" package=0)
+  (Module 1 "lib" package=0)
+  (Test u0 "api::works" should_panic=false)
+  (Test u1 "api::panics" should_panic=true)
+  (Test u3 "in_lib" should_panic=false))
+(Diagnostics)
+
+;; E0084 and E0085, under test mode:
+;; nova check --dump-resolution --test main.nova
+;; main.nova: @test\nfn takes(n: Int) {}\n@test(nope)\nfn t() {}\n
+;;     fn main() {}\n
+(Program target=native mode=test resolved=true)
+(Diagnostics
+  (Error "E0084" "a `@test` function may not have parameters"
+    (Primary "main":9..14 "invalid test signature")
+    (Secondary "main":15..21 "parameters here"))
+  (Error "E0085" "unknown `@test` argument `nope`; the accepted arguments are: should_panic"
+    (Primary "main":32..36 "unknown argument")))
+
+;; Std mode=test, from nova check --dump-resolution --std --test:
+(Std target=native mode=test modules=22 defs=716 items=551
+  (Module 21 "$std.test" package=None)
+  (Scope 0
+    (Value.Def "assert_eq" s714)))
+(Diagnostics)
+
+;; target=wasm, and E0090 and E0092:
+;; nova check --dump-resolution --target wasm page.nova
+;; page.nova: import ui::ssr\nextern "C" {\n    fn abs(x: Int) -> Int\n}\n
+;;     fn main() {}\n
+(Program target=wasm resolved=true)
+(Diagnostics
+  (Error "E0090" "extern functions are not available on target wasm"
+    (Primary "page":15..55 "this `extern` block"))
+  (Error "E0092" "`ui::ssr` is a std module, which target wasm does not have"
+    (Primary "page":7..14 "not on target wasm")))
+
+;; Std target=wasm, Missing, from nova check --dump-resolution --std --target wasm:
+(Std target=wasm modules=16 defs=781 items=563
+  (Module 10 "$std.dom" package=None)
+  (Missing "args" "$std.process"))
+(Diagnostics)
+```
+
+**Sizes.** std's WASM part is 5,260 lines (233 KB): 16 modules, 781
+definitions, 563 items and 92 `missing` names; its test-mode part 8,098
+lines (333 KB), against the native part's 7,728 (320 KB). A package's
+program holds every module its root reaches: `examples/05-geometry`'s
+`src/main.nova` is 587 lines (19 KB), `examples/07-fullstack-blog`'s 2,184
+(71 KB), and `selfhost/src/main.nova`, which reaches most of N, 11,202
+lines (411 KB); the corpus's 67 package files dump to 152,084 lines (5.4
+MB) in 2.7 s, S0 debug, one process.
+
+### 2. The cases
+
+`crates/nova-cli/tests/selfhost/cases/` holds a directory per mode, which
+the directory's name says, so a case needs no marker: `resolve/` under
+`check`, `resolve-wasm/` for the WASM target and `resolve-test/` under test
+mode. In each, a `.nova` file is a program; a directory holding a
+`nova.toml` is a package, and each `.nova` file directly in its `src/`,
+then its `tests/`, sorted, is a root, named from the mode's directory;
+any other directory is a loose program rooted at its `main.nova` (one
+holding `dom.nova` or `ui.nova` runs from inside itself, ADR 0053 §5). A
+package's dependencies are inside it, at the `path`s its manifest gives.
+
+The cases come from the programs the tests resolve, recorded by a
+temporary instrumentation of the resolver and the loader while the
+workspace's library tests, the driver's and the CLI's package, fix,
+doc-test, analysis, index and project tests, `wasm_tests.rs` and
+`run_tests.rs` ran (a script, not committed): every
+program resolved as a package, for the WASM target or under test mode,
+kept when it is a package's, gives a resolver diagnostic, holds a
+`@test` (under test mode) or imports `dom` or `ui` (for the WASM target);
+dropped when it is a copy of a tracked file, a repeat, a program of doc
+tests, a registry package's, or one whose graph gives a diagnostic; each
+text with `\r\n` read as `\n`. A package whose dependencies were outside
+it has them moved under `pkgs/`, its manifests' `path`s rewritten. Named
+`<file>__<test>[_n]` as 5.2a's; a test that ran `nova` is named by its
+temporary directory. The 5.2a cases holding a `@test` are copied to
+`resolve-test/`, since `check` strips what they test. Hand cases (names
+without `__`): E0004, E0004 inside a dependency (its secondary label
+that dependency's manifest, which also has a dev-dependency the graph
+never reads), E0004 for a `tests/` module named for the package (its
+secondary label `[package]`'s span, whose last value is followed by a
+comment; the first fact-check), a package module named `ui` (E0093), E0001's two
+notes, a dependency's modules labelled by its import name with a dropped
+fix and an E0102 in it, a dependency's tests stripped while a `tests/`
+file names its own, a `tests/` file's dev-dependency and siblings, every
+`@test` shape (E0083 to E0085), a loose sibling's kept test, E0090 with
+E0092, `dom` and `ui` imported on WASM, and a WASM package whose dependency
+has an `extern` block.
+
+At the split: `resolve/` 145 programs (103 of 5.2a, and 28 packages with
+42 roots), `resolve-wasm/` 30, `resolve-test/` 126 (37 packages with 71
+roots). 5.1's token and AST differentials read them too, `nova fmt`'s gate
+skips `cases/`, and the CRLF test copies all three directories,
+`nova.toml`s included.
+
+### 3. The fuzzer
+
+`fuzz::wasm_name_mutate` and `fuzz::package_name_mutate` are
+`name_mutate`, whose fixed seeds 301 to 303 give the inputs they gave,
+then one edit of their own. The WASM sibling adds an import of `ui::ssr`
+(E0092 there), `ui::router` or `dom`, or an `extern "C"` or `extern "JS"`
+block (E0090); its inputs are the loose WASM corpus programs, beside
+`NAME_SIBLINGS`, seeds 311 to 313, 20 each. The package sibling puts its
+input in `src/` or, one in three, `tests/` of `PACKAGE_FILES`' package
+`app` (beside `NAME_SIBLINGS` in both, a `src/clash.nova`, a dependency
+`dep` whose library imports its own `inner` and keeps a private item and a
+`@test`, a dependency `clash`, a dev-dependency `devdep`, and no library
+of its own), and adds an import of `dep`, of its items, of a private one,
+of `devdep`, of `clash` (E0004) or of `app` (from `tests/`, E0001 with a
+note); seeds 321 to 323, 30 each, from the loose corpus. The package is
+written four times, `v0/` to `v3/`, each with its manifests in one of four
+forms N's reader takes and nova-pm reads with no M-code
+(`fuzz::package_manifests`: the plain one; comments, a header with
+spaces, `'...'`, a pre-release, more keys, the tables in another order
+and a comment after `[package]`'s last value; the dependency tables as
+inline tables before the header and the largest version; a byte-order
+mark and `\r\n`; `dep`'s adding a version entry and a path to nothing
+among its dev-dependencies), input `i` in form `i % 4`; the harness
+asserts that E0004 names its entry on more than one line of a manifest
+over them. The loose name
+inputs are also resolved under test mode; the package's under `check`,
+and seed 321's also under test mode and seed 322's for the WASM target,
+each in the process of that mode's loose inputs, which costs no
+measurable time (the first fact-check; §9). Manifests N refuses are not
+among the fuzzer's: a refusal stops N's whole process, so each would
+need two processes of its own (about 0.5 s of N's debug runtime each),
+and the reader's table, the repository's manifests (§5) and a test of
+the refusal pin them instead. The inputs are the same on every system,
+which the Windows-checkout test checks for the two siblings too.
+
+On the fixed seeds (S0's dump; part A): under `check` E0001 46, E0002 63,
+E0082 14, E0083 8, E0089 10, E0092 13, E0102 50, E0900 18 (5.2a's); under
+test mode E0084 8 and E0085 5 more, among the same; the WASM inputs
+E0090 28 and E0092 10; the package's E0004 9, E0001's note on a package
+with no library 5 and on a dev-dependency 6. With the manifest forms and
+the two modes (the first fact-check), all the dumps together hold E0004
+16 times and each note 9. The harness asserts each of
+the 11 codes, E0004 and the two notes at least twice over all of them,
+E0084 and E0085 only under test mode and E0090 only for WASM.
+
+### 4. The corpus
+
+Natively, every corpus file is now a root, 318: the 251 loose programs and
+the 67 files directly in a package's `src/` or `tests/`, in 11 directories
+(the spec's nine packages, `05-geometry`'s dependency `shapes`, and the VS
+Code extension's fixture), which 5.2a skipped; for the WASM target, the 39
+files under `tests/dom/` and `tests/wasm/`, six of them a package's; under
+test mode, the 9 files that hold a `@test`. 5.2a's floor of 66 skipped
+files is replaced by floors on these counts. The spec (§4.5) asks for
+every corpus program under the WASM target; the harness runs these 39
+(**differs**), since the rest are native programs and CI's Windows leg,
+at about 60 s (§9), has no room for them; the second fact-check ran all
+334 roots for the WASM target through both sides, and they agreed.
+
+**Time (part A, S0 alone).** `selfhost.rs` ran in 36.7 and 38.0 s
+locally, against 20.8 and 21.0 s for the 27 tests 5.1 had, interleaved:
+1.76 and 1.81 times, where 5.2a measured 1.40 to 1.77. Part B adds N's
+side of every 5.2b input (§9).
+
+### 5. N's manifest reader and package graph
+
+`selfhost/src/manifest.nova` reads what the loader needs of a
+`nova.toml` and nothing more (decision 2 of §8): the package's name and
+where its `[package]` table is, and each entry of `[dependencies]` and
+`[dev-dependencies]` with its `path` and where its key is. That is more
+than the spec's "name and path dependencies" (§4.1): the dev-dependencies
+(a `tests/` module sees them, and `src/` gives E0001's note for one), the
+spans E0004 prints (its secondary label is the entry's key; its message
+names the entry's manifest and line), `[package]`'s span (the entry of the
+package itself, which a `tests/` module sees by its import name, so
+E0004 there labels the whole table, from its `[` to the end of its last
+value, as toml_edit spans it), and whether each package's `src/lib.nova`
+(and the root's `src/main.nova`) is a file. The M-codes stay S0's.
+
+**It accepts** comments, blank lines, `\r\n`, a byte-order mark, and the
+tables `[package]`, `[dependencies]` and `[dev-dependencies]`, each once,
+headers with spaces inside the brackets, the two dependency tables also
+as one inline table of inline tables before every header (`dependencies
+= { geom = { path = "../geom" } }`, as `nova add` writes into one); bare keys; strings in `"..."` without an
+escape or in `'...'`; in `[package]` the keys nova-pm reads (`name`,
+`version` and `edition`, which must be there, `description`, `license`,
+`repository` as strings, `authors`, `keywords`, `categories` as arrays of
+strings on one line, a trailing comma allowed); in the dependency tables
+`key = { path = "..." }` on one line; and in `[dev-dependencies]` a
+version entry, `key = "REQ"` or `key = { version = "REQ" }`, whose
+requirement is one to three numbers between dots after `^`, `~`, `=` or
+nothing, each without a leading zero and at most `u64::MAX`, which
+semver's `VersionReq` takes. nova-pm reads every manifest's
+dev-dependencies but resolves only the root's, so a dependency's version
+entries are read and left alone, as S0 leaves them, and the graph refuses
+one in the root's manifest, which would need the registry. **It
+refuses**, with a message, everything else: another table or key (which
+S0 warns about, M0006), a dotted or quoted key, an array of tables, a
+multi-line string, an escape, a number or a boolean, a version entry in
+`[dependencies]` (which needs the registry), a `path` beside a `version`
+(M0003), any other requirement (S0 takes more: `*`, `>=1`, a
+pre-release, a list), a key written twice, a name nova-pm refuses
+(`check_name`: a letter first, ASCII letters, digits, `-` and `_`, at
+most 64, not a Windows device, not `dom` or `ui`), a version semver
+refuses (three numbers, each without a leading zero and at most
+`u64::MAX`, a pre-release's unbounded) or one with build metadata
+(`+...`, M0003; a pre-release is read), an edition other than 2026, and
+a control character other than a tab, a line feed and a `\r` before one
+(DEL included), as TOML refuses it. N's test
+`the_manifest_readers_accepted_and_refused_forms_are_pinned` pins this
+table: 16 texts read and 51 refused, and three refusals' words. S0 reads
+14 of the 16 with no M-code; the other two hold version entries among the
+root's dev-dependencies, which the graph refuses and S0 gives an M-code
+(the registry). S0 gives 48 of the 51 an M-code, and reads the other
+three (a quoted key, an array over two lines, a multi-line string),
+which no manifest of the repository holds. Among the 51 are a name of 65
+characters, `1.0.0.0` and each number of a version one past `u64::MAX`
+(the first fact-check found N reading those three, and DEL), and the
+graph's test refuses a dependency's version past `u64::MAX`.
+
+**Every manifest of the repository** (at the split, the 100 tracked `nova.toml` files
+and every manifest the workspace's tests parse, recorded by a temporary
+instrumentation of nova-pm while the whole suite ran: 211 distinct
+texts), each in a package with `src/main.nova` and a library at each of
+its relative paths, through both sides: 131 read as S0 reads them, the
+same dump; 80 refused where S0 gives an M-code; none refused that S0
+reads. Before the split four were: the inline dependency table `nova
+add` writes (two), a byte-order mark, and a pre-release version; the
+reader takes them now. After the first fact-check, at the 104 tracked
+`nova.toml` files and every manifest the whole suite parses: 229 distinct texts, 146 read as S0
+reads them, 83 refused where S0 gives an M-code, none refused that S0
+reads.
+
+**The graph** (`loader.nova`'s `read_graph`) is nova-pm's walk: the
+root, then breadth first each package's entries, and the root's
+dev-dependencies after its dependencies, a package read once by its
+identity, its key its index. nova-pm's identity is `real_path` of the
+package's directory, the absolute path with every link followed. N's
+std and runtime have no way to name the current directory or follow a
+link, so N's identity is the directory as the root's path and the
+entries' paths spell it, with `.` and `x/..` folded (**differs**). The
+two can differ on exactly two kinds of input: any path through a link,
+whatever the root; and, unless the root is given by an absolute path,
+any dependency path that is absolute or whose folded form climbs above
+the current directory (starts with `..`), since N cannot compare it
+with the other paths without the current directory's name. The two are
+the same package for every input of the differential: no input holds a
+link, every input that spells a dependency by its absolute path gives
+its root by its absolute path too, and no folded path climbs above the
+current directory. Where they differ: one directory spelled by an
+absolute path and a relative one under a relative root (the first
+fact-check's case (a)), or by a path that climbs above the current
+directory and comes back down and one that does not (the second
+fact-check's: in a directory `run`, `b` depends on `../../run/geom`), is
+two packages of one name to N, which refuses the graph as M0011, saying
+that the two may be one directory, where S0 reads one package; one
+directory reached through a link and without it (case (b)) is refused
+as M0011 too, with no such clause, since N cannot see a link; and
+`x/..` after a link `x` (case (c)), which N folds and `real_path`
+follows, makes N dump a package other than S0's, or S0 give an M-code
+where N dumps. A Unix test pins the first three, each pair of paths
+each way round, beside a root given absolutely, where both read one
+package; on Windows N's paths read no drive, which 5.5
+takes up with a root written with `\`.
+Where nova-pm gives an error the graph refuses instead: a dependency's
+directory with no manifest (M0007), a key that is not the package's name
+(M0008), a dependency with no library (M0009), its own directory or a
+cycle of `[dependencies]` (M0010), two packages of one name (M0011), an
+import name that is a keyword, taken twice or the root's own (M0012),
+a root with neither a library nor a program (M0013). A refusal stops
+`selfhost resolve`: it prints `error: selfhost resolve cannot dump ROOT:
+WHY` on stderr, the manifest's path in WHY, and exits 1 (not 2, which
+is a root that cannot be read or a misuse, as in S0's form), having printed
+nothing of that root's part, never a partial dump; the roots before it
+were printed. A test pins it beside S0's M-code for the same root.
+
+### 6. What N ports, and what it does not
+
+N's loader (`load_program`, `Loader::find`, `has`, `visible_edges`,
+`clash`, `module_dir`, the labels and `root_place`) is
+`program.rs`'s, function for function, for the roots §1 takes: a loose
+file, or a file directly in a package's `src/` or `tests/`
+(`package_of`). A `Place` is a package (-1 for a loose file), whether it
+is in `tests/`, and a stem; modules are found in the same order, labelled
+as S0 labels them, keyed as S0 keys them (`package=`), and each import's
+E0004, E0093 and E0001 with its two notes is S0's, word for word. The
+driver's `strip_test_functions` takes `keep_root_tests`, and
+`name_tests` names a `tests/` file's tests `<file stem>::<function>`.
+`selfhost resolve` takes `--target native|wasm` (also as
+`--target=VALUE`, as clap takes it for S0, since the first fact-check)
+and `--test`, each once, as the hidden form does; like clap, it refuses
+an empty target, a second `--target` in either spelling and a value
+given to `--test` (since the second fact-check), and `--test --target
+wasm` with one line on stderr, each with the exit code 2, as S0 does. For the WASM target it parses
+`WASM_STD_MODULES` and also `STD_MODULES`, which `missing` reads, a file
+both name parsed once; under test mode `std/test` follows std's modules,
+labelled `<std/test>`. The resolver needed nothing new: 5.2a had ported
+the WASM builtins, `IMPORTED_STD`, E0090, E0092, `missing`, the `@test`
+checks (E0083 to E0085) and `TestFn`s, which no comparison had reached.
+
+**Not ported**, all S0's alone: the M-codes and every manifest the reader
+refuses (§5); registry packages, `nova.lock` and `project::sync`, which
+the dump never runs; doc tests (`RootKind::Doc`) and `nova test`'s
+whole-package program of several roots (decision 1); a registry
+package's dropped warnings (no input has one); `real_path`'s identity
+through a link or the current directory (§5).
+
+### 7. The differential on both sides
+
+Every input part A ran through S0 alone is now compared byte for byte
+with N, exit codes included: std's native, WASM and test-mode parts; the
+corpus natively, 251 loose programs and 67 files directly in a package's
+`src/` or `tests/` (318, §4), its 39 WASM programs for the WASM target
+and its 9 files with tests under test mode; the cases, `resolve/` 145
+programs (42 a package's roots), `resolve-wasm/` 30 and `resolve-test/`
+126, and all of them again with `\r\n`; and the name fuzzer's fixed
+seeds: 150 loose inputs under `check` and under test mode, 60 WASM
+inputs and 90 package inputs, 30 of them also under test mode and 30
+for the WASM target. The floors part A set on these counts stay
+(240 loose, 60 package files in 10 directories, 35 WASM, 8 with tests;
+130, 25 and 115 cases), and so do the floors on codes, now over dumps
+both sides print: every one of the 11 codes, E0004 and both notes twice
+or more, E0084 and E0085 only under test mode, E0090 only for the WASM
+target, every `KINDS` form somewhere and no `LATER_KINDS` form. 5.2a's
+tests of refusals, roots and manifests compare N too: a package's root
+in the header test, roots in a package named from inside and outside it,
+`Nova.toml` on a case-folding disk, a linked, a dangling and an
+unlistable manifest; and both sides refuse no root under each mode, and
+`--test --target wasm` with a root or `--std`.
+
+**What the comparison found:** nothing. The first build of N's package
+loader and its WASM and test-mode switches agreed with S0 on every input
+above, so neither N nor S0's dump was changed by it; N's own tests found
+one wrong expectation in a test (the root package's library seen from
+`tests/` is labelled `lib`, as S0 labels it).
+
+**Paths on Windows** (decision 4). E0004's message names the sibling and
+the manifest by path, and E0093's the module it refuses, each as
+`Path::join` writes it, with `\` on Windows, which the dump escapes as
+`\x5c`, where N writes `/`. The harness compares these two messages with
+`\x5c` read as `/` on Windows (`compare::settle_paths`), and nothing else;
+a test checks the rule on every system. So the package inputs are
+compared on Windows too, not skipped. CI's Windows leg found one test
+that compared S0's and N's dumps of E0004 without the rule (the
+altered-std test of §9): the harness now settles every resolution dump,
+S0's and N's, in the one function that runs either side, so no
+comparison can miss it, and a test on every system feeds that function
+S0's E0004 as Windows writes it and checks it equals N's.
+
+**Edges probed before the split**, each through both sides:
+
+- A dependency reached through `..`, an absolute one, and E0004 inside a
+  dependency reached through `..`: the same dump; both print a path as it
+  is joined, `..` kept.
+- A dependency that is the package itself (M0012, its import name the
+  package's), two entries with one path (M0008), and a dev-dependency
+  that is also a dependency (M0012): S0's M-code, N's refusal, exit 1.
+- A root in a subdirectory of `src/` (a loose program on both:
+  `package_of` looks directly in `src/` and `tests/` only), an empty
+  `tests/`, a package whose only root is `src/lib.nova`, a `tests/` file
+  importing the library, a dependency and a dev-dependency, `--test` on
+  a package of four dependencies whose tests (one named as the root's)
+  are stripped, and colliding `@test`s (E0002 under test mode, stripped
+  under `check`), each under `check` and test mode: the same dump.
+- **One difference found:** N's reader read a manifest ending in a `\r`
+  with no line feed, or with a control character in a comment, which
+  toml_edit refuses (M0001). It now refuses every control character but
+  a tab, a line feed and a `\r` before one, as TOML does.
+- Case-insensitive disks: both sides ask for `nova.toml` by its path and
+  compare the parent's name with `src` and `tests` exactly, so `Nova.toml`
+  is a manifest on a case-folding disk on both (5.2a's test, now through
+  N too) and `SRC/main.nova` is a loose program on both; a dependency's
+  manifest is asked for by its path as well.
+- Windows: `src\main.nova` typed as a root is a package's module to S0 and
+  a loose program to N, whose paths split at `/` only (ADR 0053 §5); the
+  harness writes every root with `/`, and 5.5 settles both. macOS (a
+  temporary directory behind a link): a package is identified by
+  `real_path` in S0 and by its folded path in N, which differ only on
+  the inputs §5 lists, none of the differential's; a temporary directory
+  behind a link is behind it in every spelling, and no printed path is
+  canonical.
+- CRLF: `cases/` has no `.gitattributes` rule, so CI's Windows checkout
+  gives every case's `nova.toml` `\r\n`, and the CRLF copy reads them on
+  every system.
+- The harness's threads are scoped and joined in a fixed order, and each
+  output is joined in order: no count of cores changes what is compared
+  or printed.
+- `--test` before or after a root, beside an empty root, twice, and with
+  `--std --target wasm` in either order: the same stdout and exit code on
+  both sides (a test). A `--` is still a root of that name to N, where
+  S0's form skips it (ADR 0053 §5); the harness passes none.
+
+### 8. The decisions part A took, and the spec's correction
+
+1. **Roots are files:** a file directly in a package's `src/` or
+   `tests/`, or a loose program; `nova test`'s whole-package program is
+   not a root. If 5.5 needs it, the loader already takes one root per
+   call and the queue would start from several.
+2. **M-codes stay S0's.** N's reader is minimal, reads what §5 lists, and
+   refuses the rest; no comparison input has an M-code.
+3. **The spec's §4.1 note "a dependency without `src/lib.nova`"** is not
+   what happens: the graph refuses a dependency with no library first
+   (M0009). E0001's note "`<name>` has no src/lib.nova" comes only when a
+   package with no library is imported, by its own import name, from its
+   own `tests/`. The spec is left as approved; this is the correction.
+4. **Windows paths** are compared with `\` read as `/` (§7), as ADR 0053
+   settles paths, not skipped.
+5. **Test names** are `name_tests`'s, `<file stem>::<function>` for a
+   `tests/` file's.
+
+### 9. CI's time
+
+**Before the split** (the prototype as part B left it; 4 cores, Linux,
+the test binaries prebuilt, three interleaved rounds): `selfhost.rs` 51.6,
+49.5 and 51.9 s, 165 to 174 s of user CPU; 5.1's 27 tests alone 22.4,
+24.1 and 21.1 s. Each test alone, N's build (3.7 s of CPU) in each: the
+corpus's and the cases' resolution 67 s of CPU, the name fuzzer 36 s, the
+cases again with `\r\n` 22 s, 5.1's mutated corpus 15 s, its corpus and
+its CRLF corpus 9 s each, and 5.1's every-kind test 9 s, which ran both
+sides over the corpus and the cases again. The processes are nearly all
+the CPU (the harness 1 to 2 s): N, with the debug runtime, about two
+thirds of it. Where N's time goes (callgrind): the debug runtime is about
+five sixths of N's instructions; of N's own work on a package's root,
+lexing and parsing are about half; on a small root, resolving std again
+for each program (seeding std's scopes, collecting its items) is about
+half, as it is S0's (61% of S0's per-root time). N read and parsed std
+once per process, 0.40 s (0.57 s for the WASM target, which parses
+`STD_MODULES` too), and 61 N processes took roots, 50 to a batch.
+
+**The speed-up**, every input kept:
+
+- One process a side for each set and mode (`agree_on_resolution_in`):
+  the loose corpus, the package corpus, the WASM corpus, the files with
+  tests, each mode's cases (loose and packages together, the case apart
+  from its own directory), each of 5.2a's small tests' sets; the sets of
+  a test still run side by side.
+- The name fuzzer's three seeds in one process a mode, each seed's inputs
+  in a directory named for it (`names/301/f00000.nova`), counted seed by
+  seed from the headers (`seed_parts`); the package inputs, each seed
+  from inside its own package, as before.
+- 5.1's corpus and cases dumps compared once a run (`Compared`), and the
+  every-kind test reads them, as it read both sides' equal output again.
+- The CRLF copies compared only where the copy changes a file: on a
+  Windows checkout, which already has each `\r\n`, the cases' and the
+  corpus's copies change nothing, and the first comparison was of those
+  bytes; on Linux and macOS every file with a line break changes.
+
+**After:** 49.2, 47.3 and 48.7 s, 145 to 151 s of user CPU, 2.0 to 2.3
+times 5.1's tests. CI then measured `selfhost.rs` at 35.59 s on Linux,
+29.54 s on macOS and 76.59 s on Windows (all 16 jobs green), Windows about
+2.1 times Linux in every test, 5.1's included: past the spec's minute on
+Windows (risk 4). Of the options measured (an optimized debug runtime;
+std resolved once and shared; part B's cuts; none), the user chose the
+second, with no input cut.
+
+**std resolved once, shared by every root of a process (differs).** The
+spec has each program resolved as `check` resolves it, std included.
+The hidden dump's path now resolves std once per process and mode and
+shares it: S0's `ResolutionDumps` holds a `nova_resolver::SharedStd` (the
+resolution of a program of no module of its own: std's exports, the
+variants std globs, `missing`) and resolves each program with
+`resolve_program_with_shared_std`, and N's `selfhost resolve` holds a
+`SharedStd` (`share_std`) and resolves with `resolve_program_shared`. The
+user's modules go through the same steps in the same order; where the
+full resolution reads std's own state they read std's exports, rebased
+past the user's definitions (`Exports::rebased`), and std's glob binds
+the same names in each user module, std module by std module; std's own
+modules are not resolved again. The result holds the user's definitions,
+scopes, items and tests and every module's exports, all a program's part
+prints, and is not for the type checker. A program reads the shared
+state and never writes it: S0's is borrowed immutably, and N's is read
+through copies. Should resolving std alone give a diagnostic beyond
+parsing it, which would come between a program's own, or find a `@test`
+function, which the full resolution lists with a program's tests and
+the shared one would drop, every program is resolved in full on both
+sides (no std file holds either; a std's diagnostic and its test are all
+std could add to a program's part that the shared path leaves out, its
+definitions, scopes and items being std's part's). nova-resolver's test
+`std_with_a_diagnostic_or_a_test_is_resolved_for_each_program` builds std
+from altered text (`core` defining a name twice, `ui` marking a function
+`@test`) and checks the shared dump is the full one under `check` and
+test mode; the harness runs N with `--std` pointed at such a copy and
+checks each program keeps std's E0002 or lists std's test and is
+otherwise S0's; with a copy whose `ui` ends with a `pub trait`, which
+no std module exports, it checks each program importing `ui` lists the
+trait with the id N's std part gives it and is otherwise S0's, so a
+trait's rebasing is compared as a function's and a type's are. `nova check`, `nova build`, `nova test` and the language
+server keep `resolve_program_with_std`; the resolver's only other change
+is `variant_shadowing` split around the variants it reads
+(`variant_shadowing_of`), and `FileDb` derives `Clone`, so that S0's dump
+registers std's files once and copies them for each root. The driver's
+test `std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`
+compares S0's shared and unshared dumps byte for byte over the corpus
+and the cases of each mode (`check`, the WASM target, test mode; 463,
+69 and 135 roots), in order and reversed; the differential compares N's
+shared dumps with S0's. The harness also runs std's three parts side by
+side and the corpus's packages in three processes a side, root `i` in
+process `i % 3`.
+
+Per root, S0 resolves the loose corpus about six times as fast (4.7 s of
+CPU to 0.5 s), N about three times (9.6 s to 3.0 s); the packages, whose
+roots are slow to parse, little faster. `selfhost.rs` locally, measured
+on one machine state with the binaries of both trees built: before, 40.6,
+40.3 and 40.8 s (126 to 128 s of user CPU); after, 28.5, 27.3 and 27.0 s
+(83 to 87 s), 1.5 times 5.1's 27 tests alone (18.1, 18.2 and 18.0 s). (An
+earlier, slower state of the machine gave 47.3 to 49.2 s before and 34.0
+to 37.4 s after.) The rest is mostly N's debug runtime parsing: std's 21
+modules once per process (0.36 s, 49 processes) and the package roots
+that reach most of N. A projection from these local times by the ratios
+of CI to the local time before (Linux 0.88, macOS 0.73, Windows 1.89)
+put CI at 24 to 25 s on Linux, 20 to 21 s on macOS and 51 to 54 s on
+Windows. It was wrong: CI measured more, below.
+
+**After the first fact-check** (its fixes, two tests and a case, and the
+fuzzer's manifest forms and package inputs under test mode and for the
+WASM target), locally, on a slower machine state than the figures above:
+`selfhost.rs` took 30.7, 31.8 and 30.8 s before and 31.8, 30.9 and
+31.7 s after; with the fuzzer as it was, 32.1, 31.6 and 31.3 s, so the
+fuzzer's new inputs cost no measurable time.
+
+**CI, measured** (the figures that count; the local ones above are only
+local). Run 41813500363, before std was shared: `selfhost.rs` 35.59 s
+on Linux, 29.54 s on macOS, 76.59 s on Windows. Run 42006844810, on the
+tree after the first fact-check's fixes: 37.90 s on Linux (42 tests),
+35.77 s on macOS (40) and 60.48 s on Windows (36: one test failed early,
+the comparison §7 now settles, so the full run is likely slightly
+longer). Linux measured 37.9 s where the local projection said 24 to 25
+s, and Windows about 1.6 times Linux, not the 1.89 times the local time
+it was projected by. Sharing std took Windows from 76.59 s to about 60 s.
+
+**The user's decision (differs from the spec's risk 4, which asks the
+user when CI exceeds a minute on a system):** asked with these figures,
+the user accepted about 60 s on Windows, with no coverage cut. The real
+PR's CI is read for all three systems and compared with these figures;
+should `selfhost.rs` exceed 70 s on Windows, the user is asked again.
+
+**After the second fact-check** (its fixes: every resolution dump settled
+where it is run, three tests, five argument shapes and one assertion),
+locally, one machine state, three runs each: `selfhost.rs` 32.6, 33.4 and
+34.7 s before, 33.0, 33.4 and 34.6 s after (100.6 to 106.0 s and 102.8
+to 106.4 s of user CPU): no measurable time added.
+
+### 10. The live heap (spec §4.4, phase plan decision 8)
+
+N resolving its largest inputs, measured locally as the research did
+(facts §6), with the debug runtime CI uses and a release runtime
+(`libnova_runtime.a` built `--release` into an isolated target dir and
+passed as `NOVA_RUNTIME_LIB`); peak RSS and wall time are the best of
+three by `wait4`; the pauses are timed with gdb from the entry of
+`nova_gc_collect_roots` to the return of `collect`, which ends with the
+mark and `Pages::sweep` (inlined in the release runtime, so the pause
+ends at the return of the allocation that triggered it); the live heap
+is the largest `NOVA_GC_DEBUG` reports after a collection. The script is
+in the 5.2b plan's last task.
+
+| input | runtime | wall | peak RSS | collections | median pause | p95 | max | total GC | max live |
+|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
+| `selfhost/src/main.nova`, a package root, all 23 of N's modules | debug | 1.08 s | 36.9 MB | 15 | 15.3 ms | 70.3 ms | 93.5 ms | 396 ms | 10.0 MB |
+| std's native part (`--std-part`) | debug | 0.48 s | 16.8 MB | 12 | 10.7 ms | 32.1 ms | 47.1 ms | 185 ms | 6.7 MB |
+| std's WASM part (`--target wasm --std-part`) | debug | 0.55 s | 19.0 MB | 12 | 9.7 ms | 30.1 ms | 44.3 ms | 170 ms | 5.5 MB |
+| `selfhost/src/main.nova` | release | 0.26 s | 36.1 MB | 15 | 3.9 ms | 18.1 ms | 29.0 ms | 105 ms | 10.0 MB |
+| std's native part | release | 0.11 s | 16.0 MB | 12 | 2.4 ms | 8.0 ms | 11.1 ms | 47 ms | 6.7 MB |
+| std's WASM part | release | 0.13 s | 18.2 MB | 12 | 2.5 ms | 7.2 ms | 9.9 ms | 41 ms | 5.5 MB |
+
+These are the split's figures, with std resolved once (§9). Before it,
+part B measured the root at 32.8 MB of peak RSS and a 102.3 ms longest
+pause with the debug runtime, 31.8 MB and 29.8 ms with the release one:
+std's state, resolved alone and kept for every root, adds about 4 MB of
+peak to a run of one root, and the pauses are the same within noise.
+
+No threshold is set. The pauses grow with the live heap, about 9 ms a
+live MB at the largest with the debug runtime and 3 ms with the release
+one, as the research found (7 to 8 and 2 to 3), and the collector takes
+31 to 41% of N's time on these inputs. For 5.3, which holds a type on
+every node of std as well, a live heap five to ten times this one would
+give pauses of about 0.5 to 1 s with the debug runtime and 0.15 to 0.3 s
+with a release one: slower runs, not a failure, since N is a batch
+compiler; the cost to watch is CI's time, not the pause.
+
+## Consequences
+
+- S0's hidden dump covers every program `nova check FILE` and `nova test`'s
+  path take, packages, the WASM target and test mode; `nova check`, its
+  help and every command line without `--dump-resolution` first are the
+  base's.
+- N loads and resolves the same roots as S0, packages, the WASM target and
+  test mode included, byte for byte in the dump, on every input the
+  differential holds; 5.2 is done, and 5.3 ports the type checker and HIR
+  on top of it.
+- N reads `nova.toml` through a reader of its own that refuses what it
+  does not read; the package graph's diagnostics stay S0's until a driver
+  needs them (5.5).
+- The hidden dump resolves std once per process and mode, on both sides,
+  a departure from the spec's per-program resolution that a test shows
+  changes no byte (§9); with every input kept, CI measured `selfhost.rs`
+  at 37.90 s on Linux, 35.77 s on macOS and about 60 s on Windows, which
+  the user accepted; the real PR's CI is compared with these, and the
+  user is asked again past 70 s on Windows (§9).
+- The live heap is recorded (§10), with no threshold.
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index 85679e0a15c0fede1ec64b13f4adc7b046353e98..0a55065fb3f6d07e7894112498da863049655e74 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -437,6 +437,12 @@ through a canonical resolution dump on both sides.
 - **5.2b:** packages (`nova.toml`'s name and path dependencies, `src/`
   and `tests/`, a dependency), the WASM target, test mode, and the live
   heap's figures, recorded (decision 8).
+  **Done 2026-11-04 (branch `claude/loving-archimedes-frtf5q-52b`, ADR
+  0054):** N's reader of `nova.toml` reads also the dev-dependencies, the
+  entries' spans and whether each library exists, and refuses what would
+  give an M-code, which stays S0's; the differential compares packages,
+  the WASM target and test mode on both sides; the live heap is recorded
+  in ADR 0054 §10.
 - **Modules:** loading a package's flat directory, std's 21 modules, the
   imports, scopes, and the builtin table.
 - **HIR,** and a canonical dump of resolution and HIR.
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 6e09e868affa2eb2b42bdc022d53090f99e29bff..5e4ffb62c8ce5e8933414d3eba854b25d12d871a 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -797,6 +797,17 @@ resolver's error cases and a name-level fuzzer's programs. From here a
 change to how names resolve changes both compilers in one pull request.
 5.2b adds packages, the WASM target and test mode.
 
+**Amended 2026-11-04 (branch `claude/loving-archimedes-frtf5q-52b`, ADR
+0054):** `selfhost/` also loads a package's module, a file directly in its
+`src/` or `tests/`, with its path dependencies' libraries, reading
+`nova.toml` through a minimal reader of its own (the package graph's
+M-codes stay S0's), and resolves for the WASM target (`--target wasm`) and
+under test mode (`--test`, as `nova test` resolves); the differential
+compares both compilers on the corpus's packages, WASM programs and files
+with tests, the cases of each mode and the name fuzzer's package, WASM and
+test-mode programs. The live heap of N resolving its largest inputs is
+recorded in ADR 0054 §10.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/nova-spec/11-PARSER.md b/nova-spec/11-PARSER.md
index 3263ce4b46e08873d33ec8345b2473fe6efdf153..46f6712af28365fe0206b990787300be9c6cba4a 100644
--- a/nova-spec/11-PARSER.md
+++ b/nova-spec/11-PARSER.md
@@ -509,6 +509,11 @@ by std-relative ids. The hidden `nova check --dump-resolution FILE…`
 resolve --std DIR FILE…` (and `--std-part`) prints the same bytes; it
 changes with resolution, in both compilers at once.
 
+**Amended 2026-11-04 (branch `claude/loving-archimedes-frtf5q-52b`, ADR
+0054):** both forms also take a package's module as a root, `--target
+wasm` and `--test` (`nova test`'s resolution); ADR 0054 §1 writes the
+forms they add (`package=<n>`, `mode=test`, `(Test`, `(Missing`).
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
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-52b.txt" 2>&1
  grep '^test result' "$P/full-52b.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-52b.txt"
  cargo test --locked --workspace -- --list > "$P/final-list-52b.txt" 2>&1
  python3 "$P/names.py" "$P/base-list-52b.txt" "$P/final-list-52b.txt"
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
mkdir -p "$(dirname "$P/plan52b/names.py")"
cat > "$P/plan52b/names.py" <<'EOF'
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
  - the suite, in 191 s: `2869 1 10`, the baseline's figures and
    11 more tests;
  - the one failure is Task 0's IPv6 test;
  - `names.py` prints `binaries: base 112, now 112` and `tests: base
    2869, now 2880`, and
    `net added: 11`: 12 added and 1 gone, the fuzzer's test renamed:

    - `nova: tests/selfhost.rs (selfhost)`, 37 to 44:
      `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`;
      `a_manifest_n_cannot_read_stops_n_with_a_message`;
      `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`;
      `both_sides_take_test_in_every_position_alike`;
      `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`;
      `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`;
      `the_name_fuzzer_agrees_and_gives_every_resolver_code`;
      `the_paths_of_e0004_and_e0093_are_settled_on_windows_alone`;
      gone: `the_name_fuzzer_agrees_and_gives_every_native_resolver_code`;
    - `nova_driver (unittests src/lib.rs)`, 37 to 39:
      `resolution_dumps::std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`;
      `resolution_dumps::test_mode_dumps_the_tests_nova_test_collects`;
    - `nova_resolver (unittests src/lib.rs)`, 67 to 69:
      `dump::tests::std_part_is_the_same_in_every_test_mode_program`;
      `dump::tests::std_with_a_diagnostic_or_a_test_is_resolved_for_each_program`.
- [ ] **Step 2: CI's Linux leg, as CI runs it.** The `test` job's
  `ubuntu-24.04` leg runs the whole suite with the golden suite required,
  with CI's own flags, `--locked --workspace --all-features --no-fail-fast`:

  ```sh
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast > "$P/ci-52b.txt" 2>&1
  grep '^test result' "$P/ci-52b.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/ci-52b.txt"
  grep -c 'release_golden_suite ... ok' "$P/ci-52b.txt"
  ```

  Expected, in 205 s: `2869 1 10`, Step 1's figures, the one failure the
  IPv6 test, and `1`: the golden suite ran and passed inside the whole run,
  as on CI, where Step 1's run skips it.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in
  17 s of wall time: ``release golden: 147 passed, 9 excluded, Ubuntu
  clang version 18.1.3 (1ubuntu1)`, in 17 s` and `1 passed`: unchanged,
  since 5.2b adds no runtime program.
- [ ] **Step 4: Parsing and checking every file the base tracks.** Two
  scripts, as the replay ran them, compare the base's `nova` (Task 0's
  copy, `$P/nova-base-52b`) with the tree's on every `.nova` file
  `git ls-tree -r --name-only 2d38ad7a` lists, each run's output, both
  streams, and exit status compared byte for byte. 5.2b changes neither
  the lexer, the parser, nor any command's path, so nothing here changes.
  A third runs `nova test` and `nova build` on every package. All run
  from the repository's root after the build, and read `$P`.
  - `parsecmp.sh`: `nova parse --check` and `nova parse --ast` of each
    file, both on the file as `2d38ad7a` has it, from a
    `git archive 2d38ad7a` unpacked in `$P/pc-src-52b`, each passed the
    path relative to that root:

```sh
mkdir -p "$(dirname "$P/plan52b/parsecmp.sh")"
cat > "$P/plan52b/parsecmp.sh" <<'EOF'
#!/bin/bash
# parsecmp.sh: run from the repository's root, after the build, with $P set.
# For every `.nova` file `git ls-tree -r --name-only 2d38ad7a` lists (the
# base's tracked files, so none 5.2a adds), `nova parse --check` and
# `nova parse --ast`, by the base's `nova` ($P/nova-base-52b, Task 0) and
# by the tree's `target/debug/nova`, both on the file as 2d38ad7a has it,
# from a `git archive 2d38ad7a` unpacked in $P/pc-src-52b: 5.2a adds
# seven files to N's sources and changes its `main.nova`, which the base's
# text keeps out of the comparison. Each run passes the path relative to that root and
# keeps its output, both streams, and its exit status; every pair must be
# byte-identical, since 5.2a changes neither the lexer nor the parser.
# Prints each file that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
src="$P/pc-src-52b"
rm -rf "$src"; mkdir -p "$src"
git archive 2d38ad7a | tar -x -C "$src"
n=0; differ=0; nz_base=0; nz_new=0
while read -r f; do
  for mode in --check --ast; do
    a=$( (cd "$src" && "$P/nova-base-52b" parse $mode "$f") 2>&1; echo "exit=$?")
    b=$( (cd "$src" && "$nova" parse $mode "$f") 2>&1; echo "exit=$?")
    n=$((n + 1))
    if [ "$mode" = --check ]; then
      [ "$(tail -1 <<< "$a")" != exit=0 ] && nz_base=$((nz_base + 1))
      [ "$(tail -1 <<< "$b")" != exit=0 ] && nz_new=$((nz_new + 1))
    fi
    if [ "$a" != "$b" ]; then differ=$((differ + 1)); echo "differs: $mode $f"; fi
  done
done < <(git ls-tree -r --name-only 2d38ad7a | grep '\.nova$')
echo "parse: $n runs over $((n / 2)) files, $differ differ; --check exits nonzero on $nz_base at the base, $nz_new now"
rm -rf "$src"
[ "$differ" = 0 ]
EOF
```

  - `checkcmp.sh`: `nova check` by both, on the tree's files, which shows
    that `check`, through `resolve_program_for`, prints what the base
    prints (decision 18). A file's package root is the nearest directory
    above it, in the tree, that holds a `nova.toml`, below the
    repository's root: each package is checked once, by `nova check` run
    in its root, and every other file alone, by `nova check FILE` from
    the repository's root:

```sh
mkdir -p "$(dirname "$P/plan52b/checkcmp.sh")"
cat > "$P/plan52b/checkcmp.sh" <<'EOF'
#!/bin/bash
# checkcmp.sh (5.2b part A, from 5.2a's): run from the repository's root, after the build, with $P set.
# `nova check` by the base's `nova` ($P/nova-base-52b, Task 0) and by the
# tree's `target/debug/nova`, both on the tree's files, for every `.nova`
# file `git ls-tree -r --name-only 2d38ad7a` lists. A file's package root
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
    # 5.2b: each file of a package by `nova check FILE` too, from the
    # repository's root, as the dump takes a package's root.
    a=$("$P/nova-base-52b" check "$f" 2>&1; echo "exit=$?")
    b=$("$nova" check "$f" 2>&1; echo "exit=$?")
    n=$((n + 1))
    if [ "$a" != "$b" ]; then
      differ=$((differ + 1)); echo "differs: $f (nova check FILE)"
    fi
    [ -n "${seen[$pkg]:-}" ] && continue
    seen[$pkg]=1
    a=$( (cd "$pkg" && "$P/nova-base-52b" check) 2>&1; echo "exit=$?")
    b=$( (cd "$pkg" && "$nova" check) 2>&1; echo "exit=$?")
    what="$pkg/ (package)"
  else
    a=$("$P/nova-base-52b" check "$f" 2>&1; echo "exit=$?")
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
done < <(git ls-tree -r --name-only 2d38ad7a | grep '\.nova$')
echo "check: $n checks, $differ outside std differ, $stddiff std files differ"
[ "$differ" = 0 ] && [ "$stddiff" = 0 ]
EOF
```

  - `pkgcmp.sh`: `nova test` and `nova build -o OUT` in a copy of every
    package the base tracks, by both, both streams and the exit status
    (decision 19):

```sh
mkdir -p "$(dirname "$P/plan52b/pkgcmp.sh")"
cat > "$P/plan52b/pkgcmp.sh" <<'EOF'
#!/bin/bash
# pkgcmp.sh: run from the repository's root, after the build, with $P set. For every package of the
# base's tree (a directory holding a nova.toml, of `git ls-tree -r --name-only 2d38ad7a`), a copy in a
# fresh directory for each side, then `nova test` and `nova build -o OUT` in it by the base's `nova`
# ($P/nova-base-52b) and by the tree's: both streams and the exit status must be byte-identical
# (the built program's bytes are not compared), both with the tree's runtime library, which 5.2b does not change. Prints each
# that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
nova="$PWD/target/debug/nova"; repo=$PWD
work=$P/plan52b/pkgcmp; rm -rf "$work"; mkdir -p "$work"
n=0; differ=0
for m in $(git ls-tree -r --name-only 2d38ad7a | grep '/nova.toml$' | grep -v '^crates/'); do
  pkg=$(dirname "$m")
  for side in base tree; do
    rm -rf "$work/$side"; mkdir -p "$work/$side"; cp -r "$pkg" "$work/$side/p"; rm -rf "$work/$side/p/target"
  done
  for cmd in test "build -o $work/OUT"; do
    a=$(cd "$work/base/p" && NOVA_RUNTIME_LIB=$repo/target/debug/libnova_runtime.a NOVA_HOME=$work/home-base "$P/nova-base-52b" $cmd 2>&1; echo "exit=$?")
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

  - Then:

    ```sh
    P="$P" bash "$P/plan52b/parsecmp.sh"
    P="$P" bash "$P/plan52b/checkcmp.sh"
    P="$P" bash "$P/plan52b/pkgcmp.sh"
    ```

    Expected: `parse: 1470 runs over 735 files, 0 differ; --check exits
    nonzero on 119 at the base, 119 now`, then `check: 746 checks, 0
    outside std differ, 0 std files differ`, then `packages: 22 runs, 0
    differ`. Each exits 0.
- [ ] **Step 5: The hidden flags stay hidden.**
  - `helpcmp.sh` runs `--help` of `nova` and of each of its 21 commands,
    `help` included, by the base's `nova` and the tree's, both through a
    link named `nova`, and checks that neither `nova parse --help` nor
    `nova check --help` names a hidden flag, `--std` or `--test`:

```sh
mkdir -p "$(dirname "$P/plan52b/helpcmp.sh")"
cat > "$P/plan52b/helpcmp.sh" <<'EOF'
#!/bin/bash
# helpcmp.sh: run from the repository's root, after the build, with $P set.
# `--help` of `nova` and of every command, by the base's `nova`
# ($P/nova-base-52b, Task 0) and the tree's `target/debug/nova`, both run
# through a link named `nova` so the usage lines agree: both streams and the
# exit status must be byte-identical, the hidden flags and `more` absent
# from `parse --help` and `check --help`.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
d="$P/help-52b"
rm -rf "$d"; mkdir -p "$d/base" "$d/tree"
ln -s "$P/nova-base-52b" "$d/base/nova"
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

  - The `parse` and `check` shapes, refusals and the help beside them,
    from the base's `nova` and the tree's, each run with the same first
    argument: `nova`, a link's name, `othernova`, a renamed binary, and
    four with no file name in UTF-8, `""`, `/`, `..` and the bytes ff fe;
    both streams and the exit code must be the base's, byte for byte;
    some in a terminal, colours included; then, on the tree alone, the
    hidden form's own misuses, `--test` in every position among them,
    each one error and exit 2 (decision 15). `argv0.py` runs a program by
    `execv` with the first argument it is given, empty included:

```sh
mkdir -p "$(dirname "$P/plan52b/argv0.py")"
cat > "$P/plan52b/argv0.py" <<'EOF'
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
mkdir -p "$(dirname "$P/plan52b/usagecmp.sh")"
cat > "$P/plan52b/usagecmp.sh" <<'EOF'
#!/bin/bash
# usagecmp.sh (5.2b, from 5.2a's, with `--test`'s shapes in every position): run from the repository's root, after the build, with $P set
# and $P/argv0.py written. The errors `parse` and `check` give for
# arguments they refuse, and what they print for the shapes beside them,
# from the base's `nova` ($P/nova-base-52b, Task 0) and the tree's
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
  "check --help --dump-resolution" "check -h --std"
  "check --test a" "check --test" "check a --test" "check --test --dump-resolution a" "check --tests a"
  "check --test=1 a" "check -- --test" "check --test ''" "check --test -- a" "check '' --test"
  "check --test --test a" "check a --test --dump-resolution")
runs=0; same=0
for arg0 in nova othernova "" / .. '\xff\xfe'; do
  for args in "${shapes[@]}"; do
    eval "python3 -I '$P/argv0.py' '$arg0' '$P/nova-base-52b' $args" > "$P/usage-base.txt" 2>&1
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
  "check --std=1 a.nova" "check --std --help" "check a b --bogus" "check --test a" "check a --test")
for args in "${ttys[@]}"; do
  env -u NO_COLOR script -qc "python3 -I '$P/argv0.py' othernova '$P/nova-base-52b' $args" /dev/null > "$P/usage-base.txt"
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
              "check --dump-resolution --stdd" "check --dump-resolution --help" "check --dump-resolution -h" \
              "check --dump-resolution --test --test a" "check --dump-resolution --test=1 a" \
              "check --dump-resolution --tests a" "check --dump-resolution --test --target wasm a" \
              "check --dump-resolution --target=wasm --test --std" "check --dump-resolution --test --std a" \
              "check --dump-resolution --test" "check --dump-resolution --test -h" \
              "check --dump-resolution --test --std --target native a" "check --dump-resolution --target wasm --test" \
              "check --dump-resolution --test --std --test"; do
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
    P="$P" bash "$P/plan52b/helpcmp.sh"
    P="$P" bash "$P/plan52b/usagecmp.sh"
    ```

    Expected: `help: 22 of 22 identical; hidden flags shown: 0`, then

    ```
    [nova] Usage: nova check <FILE>, exit 2
    [othernova] Usage: othernova check <FILE>, exit 2
    [] Usage: check <FILE>, exit 2
    [/] Usage: check <FILE>, exit 2
    [..] Usage: check <FILE>, exit 2
    [\xff\xfe] Usage: check <FILE>, exit 2
    usage: 246 of 246 runs identical; in a terminal, 10 of 10; --dump-ast as --dump-tokens: 25 of 25; --dump-resolution refused: 114 of 114
    ```

    and both exit 0.
- [ ] **Step 6: The lockfile and Rust 1.78.**
  - `git diff --numstat 2d38ad7a -- Cargo.lock`, and `git diff --stat
    2d38ad7a -- '*Cargo.toml'`. Expected: nothing.
  - Then:

    ```sh
    RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-52b cargo check --locked --workspace
    rm -rf /dev/shm/msrv-52b
    ```

    Expected: clean, `Finished` in 58 s. It checks the libraries
    and binaries, as CI's MSRV job does, not the tests: `cargo check
    --workspace` compiles no test target, so the tests' Rust meets 1.78 by
    review alone. Its target directory is in `/dev/shm`, memory, so the
    disk stays above 6G.
- [ ] **Step 7: The differential's time, N's tests, the fuzzer's codes
  and the live heap.** `SH --no-run`, then three times, each timed, `SH`
  and then `SH -- --exact` followed by the 27 tests `selfhost.rs` had at
  5.1 (`$P/plan52b/t51.txt`, one name a line, written from the list
  below), then `SH the_name_fuzzer --nocapture 2>&1 | grep -E
  'codes|floor' | head -n 8`,
  then `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 1`:

  ```sh
  printf '%s\n' the_token_dump_flag_is_hidden \
    the_token_dump_has_a_header_per_file_and_ioerr_for_a_missing_one \
    the_selfhost_tests_pass the_corpus_dumps_agree \
    the_corpus_dumps_agree_with_windows_line_endings \
    the_mutations_are_the_same_with_windows_line_endings \
    the_fixed_cases_agree the_edge_inputs_agree \
    the_soup_agrees_and_holds_every_lex_error the_mutated_corpus_agrees \
    a_longer_search_from_the_environment \
    unreadable_files_are_reported_alike both_sides_refuse_no_file \
    the_mismatch_report_names_the_file_and_line \
    a_side_that_fails_is_reported_with_its_exit_code_and_stderr \
    a_process_past_its_limit_is_killed_and_its_batch_named \
    a_failed_build_is_made_once_and_reported_to_each_test \
    a_pipe_still_open_at_the_deadline_is_reported \
    a_missing_final_line_break_is_marked \
    the_ast_dump_flag_is_hidden_and_dumps_each_file_after_a_header \
    every_kind_appears_in_the_dumps_of_the_corpus_and_the_cases \
    the_deep_inputs_agree \
    n_parses_128_levels_of_every_kind_on_a_1_mb_main_thread \
    s0_meets_the_depth_limit_on_the_deep_inputs_not_its_stack \
    a_panic_on_the_pipelines_thread_exits_101_with_its_message \
    a_long_decimal_number_lexes_with_a_1_mb_main_thread \
    nova_run_runs_the_program_on_the_main_thread > "$P/plan52b/t51.txt"
  cargo test --locked --workspace --test selfhost --no-run
  for i in 1 2 3; do
    TIMEFORMAT='wall %R s, user %U s, sys %S s'; time (SH 2>&1 | grep 'test result')
    TIMEFORMAT='wall %R s, user %U s, sys %S s'; time (SH --exact $(tr '\n' ' ' < "$P/plan52b/t51.txt") 2>&1 | grep 'test result')
  done
  SH the_name_fuzzer --nocapture 2>&1 | grep -E 'codes|floor' | head -n 8
  (cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 1
  ```

  Expected: `44 passed; 0 failed` each time, in 20.2, 20.2 and 20.5 s
  (user CPU 64 to 65 s; the machine was faster after a container restart
  than on the days of decisions 18 and 25) of wall time,
  building N included, and `27 passed; 0 failed; …; 17 filtered out` in
  13.3, 12.8 and 12.9 s; then `codes over seeds [301, 302, 303], [311,
  312, 313] and [321, 322, 323]: {" has no src/lib.nova\")": 9, " is a
  dev-dependency, which only `tests/` files can import\")": 9, "E0001":
  199, "E0002": 497, "E0004": 16, "E0082": 45, "E0083": 28, "E0084": 9,
  "E0085": 6, "E0089": 29, "E0090": 29, "E0092": 39, "E0102": 146,
  "E0900": 44}`; then `test result: ok. 32 passed; 0 failed; 0 trapped;
  32 total`. Record them in the
  ledger beside CI's: run 42006844810 measured Linux 37.90 s, macOS
  35.77 s, Windows 60.48 s (decision 24), and the PR's run gives each
  system's again (Step 11).

  Then the live heap once (ADR 0054 §10): N built with the debug runtime
  and with a release one, resolving `selfhost/src/main.nova` and std's
  native and WASM parts, peak RSS and wall (best of 3), GC pauses (gdb),
  and the runtime's last live figure. The release runtime is built in a
  copy of the workspace, without `target/` or `.git/`, so the tree's own
  `target/` is untouched, then the scripts are written as the replay ran
  them:

  ```sh
  mkdir -p "$P/proto52b/b-ws" && git archive HEAD | tar -x -C "$P/proto52b/b-ws"
  (cd "$P/proto52b/b-ws" && cargo build --release --locked -p nova-runtime --target-dir target)
  ```


```sh
mkdir -p "$(dirname "$P/r52/meas.py")"
cat > "$P/r52/meas.py" <<'EOF'
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
mkdir -p "$(dirname "$P/proto52b/b-gcpause.py")"
cat > "$P/proto52b/b-gcpause.py" <<'EOF'
# gdb -batch -nx -x b-gcpause.py --args PROGRAM ARGS...: each GC pause, timed as the research
# timed them (facts-52 §6): from the entry of nova_gc_collect_roots (the stack scan, the
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
mkdir -p "$(dirname "$P/plan52b/heap.sh")"
cat > "$P/plan52b/heap.sh" <<'EOF'
#!/bin/bash
# heap.sh (the prototype's b-heap.sh): run from the repository's root, after the build, with
# $P set. The live heap of N resolving its largest inputs (spec §4.4, facts-52 §6), with the
# debug runtime and a release one: peak RSS and wall (best of 3, wait4, r52/meas.py), GC pauses
# (gdb, b-gcpause.py: from nova_gc_collect_roots to the end of Pages::sweep), and NOVA_GC_DEBUG's
# last live figure. Results in b-heap.txt.
#   N debug:   nova build -o b-heap/selfhost-debug             (selfhost/, the debug runtime CI uses)
#   N release: NOVA_RUNTIME_LIB=b-ws/target/release/libnova_runtime.a nova build -o b-heap/selfhost-release
#   the release runtime: cargo build --release --locked -p nova-runtime --target-dir b-ws/target,
#   in b-ws, a copy of the workspace without target/ or .git/
: "${P:?set P to the scratch directory}"
H=$P/proto52b/b-heap; mkdir -p $H
cd selfhost
../target/debug/nova build -o $H/selfhost-debug > /dev/null
NOVA_RUNTIME_LIB=$P/proto52b/b-ws/target/release/libnova_runtime.a ../target/debug/nova build -o $H/selfhost-release > /dev/null
cd ..
out=$P/proto52b/b-heap.txt; : > $out
for rt in debug release; do
  N=$H/selfhost-$rt
  for input in "selfhost/src/main.nova" "--std-part" "--target wasm --std-part"; do
    args="resolve --std std $input"
    m=$(python3 $P/r52/meas.py 3 $N $args)
    g=$(gdb -batch -nx -x $P/proto52b/b-gcpause.py --args $N $args 2>/dev/null | grep GCPAUSE)
    live=$(NOVA_GC_DEBUG=1 $N $args 2>&1 >/dev/null | awk -F'[(]' '/nova-gc: collection/ {n++; split($2,a," "); if (a[1]>m) m=a[1]} END {printf "%d collections, max live %.1f MB", n, m/1048576}')
    echo "$rt | $input | $m | $g | $live" >> $out
  done
done
cat $out
EOF
```

  ```sh
  P="$P" bash "$P/plan52b/heap.sh"
  ```

  Expected, within noise of ADR 0054 §10's figures (debug 36.9 MB peak,
  longest pause 93.5 ms; release 36.1 MB, 29.0 ms):

  ```
  debug | selfhost/src/main.nova | 0.800s 37.0MB exit=0  selfhost-debug resolve --std std selfhost/src/main.nova | GCPAUSE n=15 median=11.3ms p95=50.1ms max=63.5ms total=283ms | 15 collections, max live 9.9 MB
  debug | --std-part | 0.346s 16.9MB exit=0  selfhost-debug resolve --std std --std-part | GCPAUSE n=12 median=7.9ms p95=23.8ms max=39.0ms total=137ms | 12 collections, max live 6.7 MB
  debug | --target wasm --std-part | 0.388s 19.1MB exit=0  selfhost-debug resolve --std std --target wasm --std-part | GCPAUSE n=12 median=7.8ms p95=25.6ms max=35.3ms total=137ms | 12 collections, max live 5.5 MB
  release | selfhost/src/main.nova | 0.206s 35.9MB exit=0  selfhost-release resolve --std std selfhost/src/main.nova | GCPAUSE n=15 median=3.2ms p95=14.8ms max=19.1ms total=82ms | 15 collections, max live 9.9 MB
  release | --std-part | 0.080s 15.9MB exit=0  selfhost-release resolve --std std --std-part | GCPAUSE n=12 median=2.2ms p95=6.8ms max=9.0ms total=37ms | 12 collections, max live 6.7 MB
  release | --target wasm --std-part | 0.090s 18.3MB exit=0  selfhost-release resolve --std std --target wasm --std-part | GCPAUSE n=12 median=2.1ms p95=6.8ms max=9.6ms total=37ms | 12 collections, max live 5.5 MB
  ```

- [ ] **Step 8: The mutants.** One at a time, each edit is made, its
  command run, and the file restored with `git checkout -- <file>`, all on
  the last commit, in a shell under the process limit.
  - Each edit replaces one exact text, which occurs once in its file
    unless the mutant names which occurrence ("the second of two"); "the
    line goes" removes the whole line holding it.
  - Each command starts with `cargo build --locked --workspace`. A mutant
    of N needs no Rust rebuilt: `selfhost.rs` builds N from `selfhost/`
    each run.
  - The replay's run took 56 minutes in all; a mutant of S0 about
    42 s, one of N about 20 s.
  - Each caught mutant must fail as written: the tests named.
    Five pass by design, each with its reason below: mutants 19, 29, 32,
    44 and 105. Any other mutant that passes is a gap: record
    it in the ledger, name it in the PR, and give it to the review (Step
    10), which says whether a check is added.
  - At the end, `git status --short` prints nothing, and
    `cargo build --locked --workspace` rebuilds the last commit's binaries
    (decision 21).

  1. **` mode=test` never printed.** `crates/nova-resolver/src/dump.rs`:
     replace

     ```
     fn mode(&mut self, test: bool) {
             if test {
     ```

     with

     ```
     fn mode(&mut self, test: bool) {
             if false {
     ```

     - Run `cargo build --locked --workspace`, `cargo test --locked
       --workspace --lib -- dump::tests`, `cargo test --locked
       --workspace --lib -- resolution_dumps` and `SH`, each to its end.
     - Expected: fails: `both_sides_take_test_in_every_position_alike`,
       `dump::tests::std_part_is_the_same_in_every_test_mode_program`,
       `dump::tests::the_program_prints_every_form`,
       `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
       `resolution_dumps::test_mode_dumps_the_tests_nova_test_collects`,
       `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
       `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
       `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
       `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  2. **A module's package key always None.** `crates/nova-resolver/src/dump.rs`:
     `d.field("package", &package(m.package));` becomes
     `d.field("package", &package(None));`.
     - Run `cargo build --locked --workspace`, `cargo test --locked
       --workspace --lib -- dump::tests`, `cargo test --locked
       --workspace --lib -- resolution_dumps` and `SH`, each to its end.
     - Expected: fails:
       `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
       `a_package_is_found_by_its_manifests_path_on_both_sides`,
       `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
       `dump::tests::the_program_prints_every_form`,
       `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
       `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
       `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
       `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
       `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
       `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  3. **A test's should_panic always false.** `crates/nova-resolver/src/dump.rs`:
     `d.bool_field("should_panic", test.should_panic);` becomes
     `d.bool_field("should_panic", false);`.
     - Run `cargo build --locked --workspace`, `cargo test --locked
       --workspace --lib -- dump::tests`, `cargo test --locked
       --workspace --lib -- resolution_dumps` and `SH`, each to its end.
     - Expected: fails: `dump::tests::the_program_prints_every_form`,
       `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
       `resolution_dumps::test_mode_dumps_the_tests_nova_test_collects`,
       `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
       `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
       `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  4. **Std's `missing` names not printed.** `crates/nova-resolver/src/dump.rs`:
     `self.open(1, "Missing");` becomes `self.open(1, "Absent");`.
     - Run `cargo build --locked --workspace`, `cargo test --locked
       --workspace --lib -- dump::tests`, `cargo test --locked
       --workspace --lib -- resolution_dumps` and `SH`, each to its end.
     - Expected: fails: `both_sides_take_test_in_every_position_alike`,
       `dump::tests::the_program_prints_every_form`,
       `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  5. **A dependency's manifest labelled as the root's.**
     `crates/nova-driver/src/lib.rs`:
     `format!("{}/{}", p.import_name, nova_pm::MANIFEST)` becomes
     `nova_pm::MANIFEST.to_string()`.
     - Run `cargo build --locked --workspace`, `cargo test --locked
       --workspace --lib -- dump::tests`, `cargo test --locked
       --workspace --lib -- resolution_dumps` and `SH`, each to its end.
     - Expected: fails:
       `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
       `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
       `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
       `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  6. **An M-code no longer stops before the loader.**
     `crates/nova-driver/src/lib.rs`:
     replace

     ```
             if has_error(&graph_diagnostics) {
                 return Ok(
     ```

     with

     ```
             if false && has_error(&graph_diagnostics) {
                 return Ok(
     ```

     - Run `cargo build --locked --workspace`, `cargo test --locked
       --workspace --lib -- dump::tests`, `cargo test --locked
       --workspace --lib -- resolution_dumps` and `SH`, each to its end.
     - Expected: fails: `a_manifest_n_cannot_read_stops_n_with_a_message`.
  7. **Std not registered before the program.** `crates/nova-driver/src/lib.rs`:
     `let db = self.std_db.clone();` becomes `let db = FileDb::new();`.
     - Run `cargo build --locked --workspace`, `cargo test --locked
       --workspace --lib -- dump::tests`, `cargo test --locked
       --workspace --lib -- resolution_dumps` and `SH`, each to its end.
     - Expected: fails:
       `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
       `a_package_is_found_by_its_manifests_path_on_both_sides`,
       `a_root_spelled_any_way_names_its_siblings_alike`,
       `a_root_whose_name_needs_escapes_is_printed_alike`,
       `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
       `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`,
       `both_sides_take_test_in_every_position_alike`,
       `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
       `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
       `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
       `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
       `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
       `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  8. **Test mode strips the root's tests too.** `crates/nova-driver/src/lib.rs`:
     `strip_test_functions(&mut modules, self.test);` becomes
     `strip_test_functions(&mut modules, false);`.
     - Run `cargo build --locked --workspace`, `cargo test --locked
       --workspace --lib -- dump::tests`, `cargo test --locked
       --workspace --lib -- resolution_dumps` and `SH`, each to its end.
     - Expected: fails:
       `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
       `resolution_dumps::test_mode_dumps_the_tests_nova_test_collects`,
       `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
       `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
       `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  9. **Test mode keeps a dependency's tests.** `crates/nova-driver/src/lib.rs`:
     replace

     ```
                             root
                         }
                 }
     ```

     with

     ```
                             true
                         }
                 }
     ```

     - Run `cargo build --locked --workspace`, `cargo test --locked
       --workspace --lib -- dump::tests`, `cargo test --locked
       --workspace --lib -- resolution_dumps` and `SH`, each to its end.
     - Expected: fails:
       `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
       `resolution_dumps::test_mode_dumps_the_tests_nova_test_collects`,
       `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
       `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
       `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  10. **A `tests/` file's tests not named for it.**
      `crates/nova-driver/src/lib.rs`:
      `name_tests(&mut resolved.tests, &resolved.definitions,
      &modules);`: the line goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `resolution_dumps::test_mode_dumps_the_tests_nova_test_collects`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  11. **Std/test not joined under test mode.** `crates/nova-driver/src/lib.rs`:
      `let extra = test.then(|| {` becomes `let extra = false.then(|| {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `resolution_dumps::test_mode_dumps_the_tests_nova_test_collects`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  12. **`--test --target wasm` accepted.** `crates/nova-cli/src/cmd/run.rs`:
      `if cmd.test && target == nova_driver::Target::Wasm {` becomes `if
      false && cmd.test && target == nova_driver::Target::Wasm {`.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails: `both_sides_refuse_no_file`,
        `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dump_flags_are_hidden_and_refused_as_before`.
  13. **`--std --test` dumps check mode's std.**
      `crates/nova-cli/src/cmd/run.rs`:
      `nova_driver::dump_std_resolution(target, cmd.test)` becomes
      `nova_driver::dump_std_resolution(target, false)`.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails:
        `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  14. **`--test` ignored for programs.** `crates/nova-cli/src/cmd/run.rs`:
      `nova_driver::ResolutionDumps::new(target, cmd.test)` becomes
      `nova_driver::ResolutionDumps::new(target, false)`.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  15. **Std's exports not rebased.** `crates/nova-resolver/src/lib.rs`:
      `exports.extend(shared.exports.iter().map(|e|
      e.rebased(offset)));` becomes
      `exports.extend(shared.exports.iter().map(|e| e.rebased(0)));`.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails:
        `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
        `a_package_is_found_by_its_manifests_path_on_both_sides`,
        `a_root_spelled_any_way_names_its_siblings_alike`,
        `a_root_whose_name_needs_escapes_is_printed_alike`,
        `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
        `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`,
        `both_sides_take_test_in_every_position_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `resolution_dumps::std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  16. **Std's exports rebased one too far.** `crates/nova-resolver/src/lib.rs`:
      `let offset = definitions.defs.len() as u32;` becomes `let offset
      = definitions.defs.len() as u32 + 1;`.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails:
        `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
        `a_package_is_found_by_its_manifests_path_on_both_sides`,
        `a_root_spelled_any_way_names_its_siblings_alike`,
        `a_root_whose_name_needs_escapes_is_printed_alike`,
        `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
        `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`,
        `both_sides_take_test_in_every_position_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `resolution_dumps::std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  17. **A variant's sum not rebased.** `crates/nova-resolver/src/lib.rs`:
      `Res::Variant(sum, index) => Res::Variant(def(sum), index),`
      becomes `Res::Variant(sum, index) => Res::Variant(sum, index),`.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails:
        `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
        `a_package_is_found_by_its_manifests_path_on_both_sides`,
        `a_root_spelled_any_way_names_its_siblings_alike`,
        `a_root_whose_name_needs_escapes_is_printed_alike`,
        `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
        `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`,
        `both_sides_take_test_in_every_position_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `resolution_dumps::std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  18. **No fallback when std alone gives a diagnostic.**
      `crates/nova-resolver/src/lib.rs`:
      `let shareable = alone.diagnostics.len() == std.diagnostics.len()
      && alone.tests.is_empty();` becomes `let shareable = true;`.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails:
        `dump::tests::std_with_a_diagnostic_or_a_test_is_resolved_for_each_program`.
  19. **The shared `missing` names dropped.** `crates/nova-resolver/src/lib.rs`:
      `definitions.missing = shared.missing.clone();`: the line goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: passes, accepted: a program's part does not print
        `missing`; std's part, which does, comes from the full path
        (`dump_std`), so no dump reads it; 5.3's type checker will, and
        its differential will see it (decision 21).
  20. **Variant shadowing from the shared state skipped.**
      `crates/nova-resolver/src/lib.rs`:
      `diagnostics.extend(variant_shadowing_of(&shared.variants,
      &all[..std_start]));`: the line goes.
      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `resolution_dumps::std_resolved_once_dumps_what_std_resolved_for_each_program_dumps`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  21. **Test mode's dumps read check mode's shared std.**
      `crates/nova-driver/src/lib.rs`:
      replace

      ```
              let (std_files, extra, labels) = register_std_for(&mut db, target, test);
              let parsed = nova_resolver::parse_std(target, &std_files, extra);
              ResolutionDumps {
      ```

      with

      ```
              let (std_files, extra, labels) = register_std_for(&mut db, target, test);
              let parsed = nova_resolver::parse_std(target, &std_files, extra.filter(|_| false));
              ResolutionDumps {
      ```

      - Run `cargo build --locked --workspace`, `cargo test --locked
        --workspace --lib -- dump::tests`, `cargo test --locked
        --workspace --lib -- resolution_dumps` and `SH`, each to its
        end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `resolution_dumps::test_mode_dumps_the_tests_nova_test_collects`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  22. **Settle_paths applied off Windows too.**
      `crates/nova-cli/tests/selfhost/compare.rs`:
      replace

      ```
          if !windows {
              return dump.to_vec();
      ```

      with

      ```
          if false {
              return dump.to_vec();
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_paths_of_e0004_and_e0093_are_settled_on_windows_alone`.
  23. **Settle_paths applied to E0005, not E0004.**
      `crates/nova-cli/tests/selfhost/compare.rs`:
      `line.starts_with(b"  (Error \"E0004\" ")` becomes
      `line.starts_with(b"  (Error \"E0005\" ")`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `the_paths_of_e0004_and_e0093_are_settled_on_windows_alone`.
  24. **The WASM corpus compared natively.**
      `crates/nova-cli/tests/selfhost.rs`:
      `let wasm_dumps = scope.spawn(||
      agree_on_resolution_in(Mode::Wasm, &root, &wasm, 0));` becomes
      `let wasm_dumps = scope.spawn(||
      agree_on_resolution_in(Mode::Check, &root, &wasm, 0));`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  25. **The test corpus compared under check.**
      `crates/nova-cli/tests/selfhost.rs`:
      `let test_dumps = scope.spawn(||
      agree_on_resolution_in(Mode::Test, &root, &tests, 0));` becomes
      `let test_dumps = scope.spawn(||
      agree_on_resolution_in(Mode::Check, &root, &tests, 0));`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  26. **Test mode passes no flag.** `crates/nova-cli/tests/selfhost.rs`:
      `Mode::Test => &["--test"],` becomes `Mode::Test => &[],`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  27. **A case directory with nova.toml read as a loose program.**
      `crates/nova-cli/tests/selfhost.rs`:
      `if path.is_dir() && path.join("nova.toml").is_file() {` becomes
      `if path.is_dir() && path.join("nova.tom").is_file() {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  28. **A case package's tests/ roots skipped.**
      `crates/nova-cli/tests/selfhost.rs`:
      replace

      ```
                  for sub in ["src", "tests"] {
                      let Ok(dir) = std::fs::read_dir(path.join(sub)) else {
      ```

      with

      ```
                  for sub in ["src"] {
                      let Ok(dir) = std::fs::read_dir(path.join(sub)) else {
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  29. **The floor of 2 per code dropped to 0.**
      `crates/nova-cli/tests/selfhost.rs`:
      `assert!(n >= 2, "{code} occurs {n} times, under 2, in {all:?}");`
      becomes `assert!(n >= 0, "{code} occurs {n} times, under 2, in
      {all:?}");`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: a floor guards the fuzzer's reach,
        not a comparison: removing it fails nothing at this head
        (decision 21).
  30. **The process split drops a root and duplicates another.**
      `crates/nova-cli/tests/selfhost.rs`:
      `.map(|k| files.iter().skip(k).step_by(parts).cloned().collect())`
      becomes `.map(|k|
      files.iter().skip(k.saturating_sub(1)).step_by(parts).cloned().collect())`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  31. **The process split deals the parts back out of order.**
      `crates/nova-cli/tests/selfhost.rs`:
      `let part = split[i % parts].pop_front().expect("each root's
      part");` becomes `let part = split[(i + 1) %
      parts].pop_front().expect("each root's part");`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  32. **The WASM fuzzer adds nothing.**
      `crates/nova-cli/tests/selfhost/fuzz.rs`:
      `add_one(rng, text, &WASM_ADDITIONS)` becomes `text`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: the additions raise the WASM seeds'
        counts, but the corpus's own `extern` blocks and imports already
        meet every floor (decision 21).
  33. **The package fuzzer never writes a tests/ module.**
      `crates/nova-cli/tests/selfhost/fuzz.rs`:
      `let tests = rng.one_in(3);` becomes `let tests = false;`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_name_fuzzer_agrees_and_gives_every_resolver_code`.
  34. **A BOM not skipped.** `selfhost/src/manifest.nova`:
      `let mut line_start = if bom { 3 } else { 0 }` becomes `let mut
      line_start = 0`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  35. **A CRLF line end not trimmed.** `selfhost/src/manifest.nova`:
      `if end > line_start && text.byte_at(end - 1) == 13 { end -= 1 }`:
      the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`s
        `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`,
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  36. **A control character accepted.** `selfhost/src/manifest.nova`:
      `return Err("a control character other than a tab or a line break
      is not TOML")` becomes `c += 0`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  37. **An unknown table accepted.** `selfhost/src/manifest.nova`:
      `if table != "package" && table != "dependencies" && table !=
      "dev-dependencies" {` becomes `if table != "package" && table !=
      "dependencies" && table != "dev-dependencies" && false {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`s
        `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`,
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  38. **A key written twice accepted.** `selfhost/src/manifest.nova`:
      `if contains(keys, "${table}.${key}") {` becomes `if false {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  39. **A key outside every table accepted.** `selfhost/src/manifest.nova`:
      `` if table == "" { return Err("the key `${key}` is outside every
      table") } ``: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  40. **An unknown [package] key accepted.** `selfhost/src/manifest.nova`:
      `if !holds(PACKAGE_KEYS, key) {` becomes `if false {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `a_manifest_n_cannot_read_stops_n_with_a_message`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`s
        `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`,
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  41. **No [package] table accepted.** `selfhost/src/manifest.nova`:
      `` if package_start < 0 { return Err("there is no `[package]`
      table") } ``: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  42. **A missing name/version/edition accepted.** `selfhost/src/manifest.nova`:
      `` if !contains(keys, "package.${key}") { return Err("`[package]`
      has no `${key}`") } ``: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  43. **Any version accepted.** `selfhost/src/manifest.nova`:
      `` if !is_version(version) { return Err("`${version}` is not a
      version this reader takes") } ``: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`s
        `the_graph_refuses_what_nova_pm_gives_an_m_code`,
        `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`,
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  44. **Build metadata accepted.** `selfhost/src/manifest.nova`:
      `if byte_index(text, 43) >= 0 { return false }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: passes, accepted: an early refusal: with it gone, the
        `+` still fails the number and identifier checks, so every such
        version is refused with the same words (decision 21).
  45. **A pre-release refused.** `selfhost/src/manifest.nova`:
      `if !identifiers(rest.byte_slice(minus + 1, rest.byte_len())) {
      return false }` becomes `return false`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  46. **Any edition accepted.** `selfhost/src/manifest.nova`:
      `` if edition != "2026" { return Err("unknown edition
      `${edition}`") } ``: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`.
  47. **`dom` and `ui` and device names accepted.**
      `selfhost/src/manifest.nova`:
      `if holds(WINDOWS_DEVICES, name.to_lower()) || name == "dom" ||
      name == "ui" {` becomes `if false {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`s
        `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`,
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  48. **`dependencies = { ... }` refused.** `selfhost/src/manifest.nova`:
      `if table == "" && (key == "dependencies" || key ==
      "dev-dependencies") {` becomes `if false {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`
        `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  49. **A dev-dependency read as a dependency.** `selfhost/src/manifest.nova`:
      replace

      ```
                          if table == "dependencies" {
                              dependencies.push(d)
      ```

      with

      ```
                          if true {
                              dependencies.push(d)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`s `e0004_and_e0001s_notes_are_the_loaders`,
        `the_graph_refuses_what_nova_pm_gives_an_m_code`,
        `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`.
  50. **The [package] span starts at its header's end.**
      `selfhost/src/manifest.nova`:
      `if table == "package" { package_start = i }` becomes `if table ==
      "package" { package_start = close + 1 }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`
        `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`.
  51. **A dependency's span is its whole line.** `selfhost/src/manifest.nova`:
      replace

      ```
                              Span { start: i, end: k_end },
                          )?
      ```

      with

      ```
                              Span { start: i, end: end },
                          )?
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`s `e0004_and_e0001s_notes_are_the_loaders`,
        `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`.
  52. **A package with neither lib nor main accepted.**
      `selfhost/src/loader.nova`:
      `if !first.has_lib && !is_file(path_join(path_join(root, "src"),
      "main.nova")).await {` becomes `if false {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_graph_refuses_what_nova_pm_gives_an_m_code`.
  53. **A library imported as a keyword accepted.** `selfhost/src/loader.nova`:
      `if first.has_lib && is_keyword(first.import_name) {` becomes `if
      false {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_graph_refuses_what_nova_pm_gives_an_m_code`.
  54. **A dependency on the package itself accepted.**
      `selfhost/src/loader.nova`:
      `` return Err("dependency `${d.name}` is its own package (M0010)")
      `` becomes `()`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_graph_refuses_what_nova_pm_gives_an_m_code`.
  55. **A dependency with no library accepted.** `selfhost/src/loader.nova`:
      `` if !new.has_lib { return Err("dependency `${d.name}` is not a
      library (M0009)") } ``: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_graph_refuses_what_nova_pm_gives_an_m_code`.
  56. **A dependency cycle accepted.** `selfhost/src/loader.nova`:
      `if state.at(id) == 0 { visit(graph, id, state)? }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_graph_refuses_what_nova_pm_gives_an_m_code`.
  57. **Two entries imported as one name accepted.** `selfhost/src/loader.nova`:
      `` if s == name { return Err("two entries are imported as
      `${name}` (M0012)") } ``: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `the_selfhost_tests_pass`; inside
        `the_selfhost_tests_pass`, N's `@test`
        `the_graph_refuses_what_nova_pm_gives_an_m_code`.
  58. **A dependency's dev-dependencies read.** `selfhost/src/loader.nova`:
      `if id == 0 { for d in p.manifest.dev_dependencies.iter() {
      entries.push((d, true)) } }` becomes `for d in
      p.manifest.dev_dependencies.iter() { entries.push((d, true)) }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test` `the_graph_refuses_what_nova_pm_gives_an_m_code`.
  59. **A dependency's manifest labelled as the root's.**
      `selfhost/src/loader.nova`:
      `label: if root { "nova.toml" } else { "${import_as}/nova.toml"
      },` becomes `label: "nova.toml",`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  60. **A tests/ module labelled as src/'s.** `selfhost/src/loader.nova`:
      `return if place.tests { "tests/${place.stem}" } else { place.stem
      }` becomes `return place.stem`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`s
        `a_package_loads_its_siblings_and_its_dependencies_libraries`,
        `e0004_and_e0001s_notes_are_the_loaders`.
  61. **A dependency's library labelled `name/lib`.**
      `selfhost/src/loader.nova`:
      `if place.stem == "lib" { import_as } else {
      "${import_as}/${place.stem}" }` becomes
      `"${import_as}/${place.stem}"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`s
        `a_package_loads_its_siblings_and_its_dependencies_libraries`,
        `e0004_and_e0001s_notes_are_the_loaders`.
  62. **A tests/ root placed in src/.** `selfhost/src/loader.nova`:
      `RootTests => Place { package: 0, tests: true, stem: stem }`
      becomes `RootTests => Place { package: 0, tests: false, stem: stem
      }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`s
        `a_package_loads_its_siblings_and_its_dependencies_libraries`,
        `e0004_and_e0001s_notes_are_the_loaders`.
  63. **A test module sees no dev-dependency.** `selfhost/src/loader.nova`:
      `for e in p.dev_dependencies.iter() { edges.push(e) }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test` `e0004_and_e0001s_notes_are_the_loaders`.
  64. **A test module does not see its own package.**
      `selfhost/src/loader.nova`:
      `edges.push(Edge { import_name: p.import_name, package: id, span:
      p.span, declarer: id })`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`s
        `a_package_loads_its_siblings_and_its_dependencies_libraries`,
        `e0004_and_e0001s_notes_are_the_loaders`.
  65. **A file and a dependency of one name: the file wins.**
      `selfhost/src/loader.nova`:
      `if file { return FoundReported(clash(graph, name, path, e,
      self.label(from), at)) }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test` `e0004_and_e0001s_notes_are_the_loaders`.
  66. **E0004's manifest line counted from 0.** `selfhost/src/loader.nova`:
      replace

      ```
          let mut line = 1
          let mut i = 0
          while i < edge.span.start
      ```

      with

      ```
          let mut line = 0
          let mut i = 0
          while i < edge.span.start
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test` `e0004_and_e0001s_notes_are_the_loaders`.
  67. **E0004's secondary label dropped.** `selfhost/src/loader.nova`:
      `d.add_secondary(declarer.label, edge.span, "the dependency")`:
      the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test` `e0004_and_e0001s_notes_are_the_loaders`.
  68. **E0001's no-lib note dropped.** `selfhost/src/loader.nova`:
      `` d.add_note("`${target.name}` has no src/lib.nova") ``: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test` `e0004_and_e0001s_notes_are_the_loaders`.
  69. **E0001's dev-dependency note dropped.** `selfhost/src/loader.nova`:
      `` d.add_note("`${name}` is a dev-dependency, which only `tests/`
      files can import") ``: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test` `e0004_and_e0001s_notes_are_the_loaders`.
  70. **A dev-dependency from src/ is no error.** `selfhost/src/loader.nova`:
      `if !dev { return FoundNothing }` becomes `return FoundNothing`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test` `e0004_and_e0001s_notes_are_the_loaders`.
  71. **A dependency's library read from its tests/.**
      `selfhost/src/loader.nova`:
      `path_join(module_dir(target, false), "lib.nova"),` becomes
      `path_join(module_dir(target, true), "lib.nova"),`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
        `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`s
        `a_package_loads_its_siblings_and_its_dependencies_libraries`,
        `e0004_and_e0001s_notes_are_the_loaders`.
  72. **Test mode keeps a dependency's tests.** `selfhost/src/loader.nova`:
      `let root = loaded.package <= 0` becomes `let root = true`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`
        `test_mode_keeps_the_root_packages_tests_and_strips_a_dependencys`.
  73. **Test mode strips the root's tests.** `selfhost/src/loader.nova`:
      `if !test || (keep_root_tests && root) { kept.push(item) }`
      becomes `if !test { kept.push(item) }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`
        `test_mode_keeps_the_root_packages_tests_and_strips_a_dependencys`.
  74. **A tests/ root is a loose program.** `selfhost/src/loader.nova`:
      replace

      ```
          } else if dir_name == "tests" {
              RootTests
      ```

      with

      ```
          } else if dir_name == "tests_" {
              RootTests
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`,
        `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`, N's
        `@test`
        `a_root_is_a_packages_module_directly_in_its_src_or_tests`.
  75. **`--target wasm` resolves natively.** `selfhost/src/main.nova`:
      `let target = if target_name == "wasm" { CompileTargetWasm } else
      { CompileTargetNative }` becomes `let target =
      CompileTargetNative`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  76. **`--test --target wasm` accepted.** `selfhost/src/main.nova`:
      `if test && target_name == "wasm" {` becomes `if false {`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_refuse_no_file`,
        `both_sides_take_test_in_every_position_alike`.
  77. **`--test` twice accepted.** `selfhost/src/main.nova`:
      `if test { bad = true }`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`.
  78. **Std/test not joined under test mode.** `selfhost/src/main.nova`:
      replace

      ```
          if test {
              let mut with_test
      ```

      with

      ```
          if false {
              let mut with_test
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  79. **A tests/ file's tests not named.** `selfhost/src/main.nova`:
      `name_tests(resolved.tests, resolved.definitions, modules)`: the
      line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  80. **A test named `stem.name`.** `selfhost/src/main.nova`:
      `named.name = "${module_name(m.path)}::${t.name}"` becomes
      `named.name = "${module_name(m.path)}.${t.name}"`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  81. **` mode=test` never printed.** `selfhost/src/resolution_dump.nova`:
      `fn mode(mut self, test: Bool) { if test { self.out.push("
      mode=test") } }` becomes `fn mode(mut self, test: Bool) { }`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  82. **A test's should_panic always false.**
      `selfhost/src/resolution_dump.nova`:
      `d.bool_field("should_panic", test.should_panic)` becomes
      `d.bool_field("should_panic", false)`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  83. **A refused manifest exits 2.** `selfhost/src/main.nova`:
      replace

      ```
                      eprintln("error: selfhost resolve cannot dump ${path}: ${why}")
                      exit(1)
      ```

      with

      ```
                      eprintln("error: selfhost resolve cannot dump ${path}: ${why}")
                      exit(2)
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
        `a_manifest_n_cannot_read_stops_n_with_a_message`.
  84. **The WASM target without `ui/html`.** `selfhost/src/builtins.nova`:
      replace

      ```
          ("$std.ui.html", "ui/html/lib.nova"),
          ("$std.ui.backend", "ui/backend/wasm.nova"),
      ```

      with

      ```
          ("$std.ui.backend", "ui/backend/wasm.nova"),
      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  85. **E0090 never given.** `selfhost/src/resolver.nova`, the second of 2:
      `"E0090",` becomes `"E0091",`.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails:
        `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
        `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  86. **`missing` holds no type names.** `selfhost/src/resolver.nova`:
      replace

      ```
              for n in exp.type_names.iter() {
                  if !missing.contains_key(n) { let _ = missing.insert(n, s.name) }
              }
      ```

      with

      ```

      ```

      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  87. **The WASM-only builtin MemLoad8 dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"MemLoad8",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  88. **The WASM-only builtin MemStore8 dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"MemStore8",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  89. **The WASM-only builtin MemLoad64 dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"MemLoad64",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  90. **The WASM-only builtin MemStore64 dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"MemStore64",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  91. **The WASM-only builtin MemCopy dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"MemCopy",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  92. **The WASM-only builtin MemFill dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"MemFill",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  93. **The WASM-only builtin MemPages dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"MemPages",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  94. **The WASM-only builtin MemGrow dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"MemGrow",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  95. **The WASM-only builtin HeapBase dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"HeapBase",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  96. **The WASM-only builtin FuturePoll dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"FuturePoll",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  97. **The WASM-only builtin HostAbort dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"HostAbort",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  98. **The WASM-only builtin ClosureWord dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"ClosureWord",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  99. **The WASM-only builtin ClosureCall dropped.**
      `selfhost/src/builtins.nova`, the second of 2:
      `"ClosureCall",`: the line goes.
      - Run `cargo build --locked --workspace` and `SH`, each to its end.
      - Expected: fails: `both_sides_take_test_in_every_position_alike`,
        `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`.
  100. **Std's exports not rebased.** `selfhost/src/resolver.nova`:
       `for e in shared.exports.iter() { exports.push(e.rebased(offset))
       }` becomes `for e in shared.exports.iter() { exports.push(e) }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`,
         `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
         `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
         `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  101. **Std's exports rebased one too far.** `selfhost/src/resolver.nova`:
       `let offset = definitions.defs.len()` becomes `let offset =
       definitions.defs.len() + 1`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
         `a_package_is_found_by_its_manifests_path_on_both_sides`,
         `a_root_spelled_any_way_names_its_siblings_alike`,
         `a_root_whose_name_needs_escapes_is_printed_alike`,
         `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
         `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`,
         `both_sides_take_test_in_every_position_alike`,
         `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
         `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
         `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
         `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  102. **A variant's sum not rebased.** `selfhost/src/resolver.nova`,
       the first of 2:
       `ResVariant(sum, index) => ResVariant(sum + offset, index)`
       becomes `ResVariant(sum, index) => ResVariant(sum, index)`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`,
         `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
         `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
         `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  103. **The precomputed std glob's ids not rebased.**
       `selfhost/src/resolver.nova`:
       replace

       ```
                           let shifted = match r {
                               ResDef(id) => ResDef(id + offset)
       ```

       with

       ```
                           let shifted = match r {
                               ResDef(id) => ResDef(id)
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`,
         `a_package_is_found_by_its_manifests_path_on_both_sides`,
         `a_root_spelled_any_way_names_its_siblings_alike`,
         `a_root_whose_name_needs_escapes_is_printed_alike`,
         `a_root_with_no_directory_part_or_in_a_package_loads_alike`,
         `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`,
         `both_sides_take_test_in_every_position_alike`,
         `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
         `the_resolution_dump_has_a_header_per_root_and_std_its_own_part`,
         `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
         `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  104. **No fallback when std alone gives a diagnostic.**
       `selfhost/src/resolver.nova`:
       `shareable: alone.diagnostics.len() == parsed &&
       alone.tests.is_empty(),` becomes `shareable: true,`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`.
  105. **The shared `missing` names dropped.** `selfhost/src/resolver.nova`:
       `missing: shared.missing,` becomes `missing: Map::new(),`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: passes, accepted: as S0's twin: no program's part
         prints `missing` (decision 21).
  106. **Std's `@test` functions not checked before sharing.**
       `crates/nova-resolver/src/lib.rs`:
       `let shareable = alone.diagnostics.len() == std.diagnostics.len()
       && alone.tests.is_empty();` becomes `let shareable =
       alone.diagnostics.len() == std.diagnostics.len();`.
       - Run `cargo build --locked --workspace`, `cargo test --locked
         --workspace --lib -- dump::tests`, `cargo test --locked
         --workspace --lib -- resolution_dumps` and `SH`, each to its
         end.
       - Expected: fails:
         `dump::tests::std_with_a_diagnostic_or_a_test_is_resolved_for_each_program`.
  107. **Std's `@test` functions not checked before sharing.**
       `selfhost/src/resolver.nova`:
       `shareable: alone.diagnostics.len() == parsed &&
       alone.tests.is_empty(),` becomes `shareable:
       alone.diagnostics.len() == parsed,`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`.
  108. **A version's numbers not bounded by u64.** `selfhost/src/manifest.nova`:
       replace

       ```
           for p in parts.iter() { if !is_u64(p) { return false } }
           true
       }

       // Whether `text` is a number
       ```

       with

       ```
           for p in parts.iter() { if !is_number(p) { return false } }
           true
       }

       // Whether `text` is a number
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`s
         `the_graph_refuses_what_nova_pm_gives_an_m_code`,
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  109. **A 20-digit number past u64::MAX taken.** `selfhost/src/manifest.nova`:
       `if n < 20 { return true }` becomes `if n < 21 { return true }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`s
         `the_graph_refuses_what_nova_pm_gives_an_m_code`,
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  110. **The 20-digit comparison reversed.** `selfhost/src/manifest.nova`:
       `if a != b { return a < b }` becomes `if a != b { return a > b }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`s
         `the_graph_refuses_what_nova_pm_gives_an_m_code`,
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  111. **A requirement's operator not read.** `selfhost/src/manifest.nova`:
       `if rest.starts_with("^") || rest.starts_with("~") ||
       rest.starts_with("=") {` becomes `if rest.starts_with("^") {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
         `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`,
         N's `@test`
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  112. **A requirement of four numbers taken.** `selfhost/src/manifest.nova`:
       `if parts.len() > 3 { return false }` becomes `if parts.len() > 4
       { return false }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  113. **A requirement's numbers not bounded by u64.**
       `selfhost/src/manifest.nova`:
       replace

       ```
           if parts.len() > 3 { return false }
           for p in parts.iter() { if !is_u64(p) { return false } }
       ```

       with

       ```
           if parts.len() > 3 { return false }
           for p in parts.iter() { if !is_number(p) { return false } }
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  114. **A version entry taken in [dependencies].**
       `selfhost/src/manifest.nova`:
       replace

       ```
                               table == "dev-dependencies",
                               Span { start: i, end: k_end },
       ```

       with

       ```
                               true,
                               Span { start: i, end: k_end },
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`s
         `the_manifest_reader_reads_what_the_loader_needs_and_refuses_the_rest`,
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  115. **A dev-dependency's version entry refused.**
       `selfhost/src/manifest.nova`:
       `if field == "version" && versions && is_requirement(text) {`
       becomes `if false {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_name_fuzzer_agrees_and_gives_every_resolver_code`,
         `the_selfhost_tests_pass`; inside `the_selfhost_tests_pass`,
         N's `@test`s `the_graph_refuses_what_nova_pm_gives_an_m_code`,
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  116. **A version entry among the root's dev-dependencies read.**
       `selfhost/src/loader.nova`:
       `if d.version != "" {` becomes `if false {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`
         `the_graph_refuses_what_nova_pm_gives_an_m_code`.
  117. **M0011's relative-and-absolute clause never given.**
       `selfhost/src/loader.nova`:
       `let mixed = other.canonical.starts_with("/") !=
       new.canonical.starts_with("/")` becomes `let mixed = false`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`.
  118. **`--target=VALUE` taken as a root.** `selfhost/src/main.nova`:
       `if a.starts_with("--target=") {` becomes `if
       a.starts_with("--target=!") {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `both_sides_take_test_in_every_position_alike`.
  119. **An empty `--target=` taken as native.** `selfhost/src/main.nova`:
       replace

       ```
                   if target_name == "" { bad = true }
                   i += 1
       ```

       with

       ```
                   i += 1
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `both_sides_take_test_in_every_position_alike`.
  120. **DEL taken in a manifest.** `selfhost/src/manifest.nova`:
       `|| b == 127` becomes `|| b == 1000`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  121. **A 65-character name taken.** `selfhost/src/manifest.nova`:
       `n == 0 || n > 64` becomes `n == 0 || n > 65`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  122. **A version of four numbers taken.** `selfhost/src/manifest.nova`:
       `if parts.len() != 3 { return false }` becomes `if parts.len() <
       3 { return false }`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `the_selfhost_tests_pass`; inside
         `the_selfhost_tests_pass`, N's `@test`
         `the_manifest_readers_accepted_and_refused_forms_are_pinned`.
  123. **`[package]`'s span to the end of its last line.**
       `selfhost/src/manifest.nova`:
       `package_end = value_end` becomes `package_end = end`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_resolution_dumps_agree_on_std_the_corpus_and_the_cases`,
         `the_resolution_dumps_agree_on_the_cases_with_windows_line_endings`.
  124. **No package inputs under test mode or for the WASM target.**
       `crates/nova-cli/tests/selfhost.rs`:
       `for ((seed, inputs), (dir, roots)) in package_inputs[..2]`
       becomes `for ((seed, inputs), (dir, roots)) in
       package_inputs[..0]`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_name_fuzzer_agrees_and_gives_every_resolver_code`.
  125. **The package inputs' manifests in one form.**
       `crates/nova-cli/tests/selfhost.rs`:
       replace

       ```
                       "app/nova.toml" => app_manifest.as_str(),
                       "dep/nova.toml" => dep_manifest.as_str(),
       ```

       with

       ```
                       "app/nova.toml" => text,
                       "dep/nova.toml" => text,
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_name_fuzzer_agrees_and_gives_every_resolver_code`.
  126. **Run settles no resolution dump.** `crates/nova-cli/tests/selfhost.rs`:
       replace

       ```
               settle_paths(&stdout, windows)
           } else {
       ```

       with

       ```
               stdout
           } else {
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`.
  127. **Run settles S0's resolution dumps only.**
       `crates/nova-cli/tests/selfhost.rs`:
       `if args.starts_with(RESOLUTION_S0) || args.first() ==
       Some(&"resolve") {` becomes `if args.starts_with(RESOLUTION_S0)
       {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`.
  128. **Run settles N's resolution dumps only.**
       `crates/nova-cli/tests/selfhost.rs`:
       `if args.starts_with(RESOLUTION_S0) || args.first() ==
       Some(&"resolve") {` becomes `if args.first() == Some(&"resolve")
       {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`.
  129. **Run settles every dump, tokens and ASTs too.**
       `crates/nova-cli/tests/selfhost.rs`:
       `if args.starts_with(RESOLUTION_S0) || args.first() ==
       Some(&"resolve") {` becomes `if !args.is_empty() {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `every_run_of_a_resolution_dump_settles_e0004s_paths_on_windows`.
  130. **N's M0011 clause only for an absolute and a relative path.**
       `selfhost/src/loader.nova`:
       `let or_one = if mixed || climbs {` becomes `let or_one = if mixed {`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`.
  131. **N's climbing check reads the first package's path only.**
       `selfhost/src/loader.nova`:
       `climbs_above(other.canonical) || climbs_above(new.canonical)`
       becomes `climbs_above(other.canonical)`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`.
  132. **N's climbing check reads the second package's path only.**
       `selfhost/src/loader.nova`:
       `climbs_above(other.canonical) || climbs_above(new.canonical)`
       becomes `climbs_above(new.canonical)`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`.
  133. **N's climbing check takes `..` alone.** `selfhost/src/loader.nova`:
       `path == ".." || path.starts_with("../")` becomes `path == ".."`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`.
  134. **N's mixed check one-sided (the first package's path
       relative).** `selfhost/src/loader.nova`:
       `let mixed = other.canonical.starts_with("/") !=
       new.canonical.starts_with("/")` becomes `let mixed =
       !other.canonical.starts_with("/")`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_directory_spelled_two_ways_is_one_package_under_an_absolute_root`.
  135. **N takes `--target ""`.** `selfhost/src/main.nova`:
       replace

       ```
                       target_name = value
                       if target_name == "" { bad = true }
       ```

       with

       ```
                       target_name = value
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `both_sides_take_test_in_every_position_alike`.
  136. **N's `--target VALUE` marks no option given.** `selfhost/src/main.nova`:
       replace

       ```
                       if target_given { bad = true }
                       target_given = true
       ```

       with

       ```
                       if target_given { bad = true }
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `both_sides_take_test_in_every_position_alike`.
  137. **N's `--target=VALUE` takes a second `--target`.**
       `selfhost/src/main.nova`:
       replace

       ```
                   if target_given { bad = true }
                   target_given = true
                   target_name = a.byte_slice(9, a.byte_len())
       ```

       with

       ```
                   target_given = true
                   target_name = a.byte_slice(9, a.byte_len())
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `both_sides_take_test_in_every_position_alike`.
  138. **N takes `--test=VALUE` as a root.** `selfhost/src/main.nova`:
       replace

       ```
                   // A flag takes no value, as clap refuses one for S0.
                   bad = true
       ```

       with

       ```
                   // A flag takes no value, as clap refuses one for S0.
                   files.push(a)
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails: `both_sides_take_test_in_every_position_alike`.
  139. **N's Exports::rebased drops the trait offset.**
       `selfhost/src/resolver.nova`:
       `out.export_trait(n, self.trait_of(n) + offset)` becomes
       `out.export_trait(n, self.trait_of(n))`.
       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `a_trait_a_shared_std_module_exports_keeps_its_id_in_each_program`.
  140. **The fuzzer's form 3 manifests without \r\n.**
       `crates/nova-cli/tests/selfhost/fuzz.rs`:
       replace

       ```
                   format!("{bom}{}", app.replace('\n', "\r\n")),
                   dep.replace('\n', "\r\n"),
       ```

       with

       ```
                   format!("{bom}{app}"),
                   dep.clone(),
       ```

       - Run `cargo build --locked --workspace` and `SH`, each to its end.
       - Expected: fails:
         `the_name_fuzzer_agrees_and_gives_every_resolver_code`.
- [ ] **Step 9: The review's inputs.** The full run's figures, the new
  tests by binary, the golden suite's line and time, the parse, check,
  help and usage comparisons, the differential's times and the codes, and
  the mutants' outcome, with each gap, go in the ledger.
- [ ] **Step 10: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0054 and this plan, and is told to hunt for:
    - a Windows-only difference: a path S0 forms with `Path::join` or
      `display` that reaches a dump or a message N compares, beyond
      E0004's and E0093's; `settle_paths` reading more than those two
      messages; a case-insensitive disk (`Nova.toml`, `SRC/`); a line
      ending in a case's `nova.toml`; a test skipped on Windows that
      hides a failure (Review Focus 1; decision 14);
    - N's messages against S0's, word for word: every manifest refusal,
      E0004, E0001's two notes, the refusal line and the usage line
      (Review Focus 2);
    - a manifest S0 accepts that N's reader refuses, or one N reads
      otherwise (Review Focus 3; decision 16);
    - a way for the harness to pass when the two sides differ: a mode's
      comparison skipped or run in the wrong mode; a root dropped or
      dealt back out of order by the process split; a part not counted; a
      floor dropped; a code asserted absent where it should be; a case
      directory misread (Review Focus 4; decision 17);
    - option B's equivalence: anything `resolve_program_with_shared_std`
      or `resolve_program_shared` does that the full path does not, a
      shared state written by a program, the wrong mode's std, a rebasing
      offset; and anything it changes for `check`, `build`, `test` or the
      language server (Review Focus 4 and 5; decision 18);
    - test mode against `nova test`: the tests kept, stripped and named
      (Review Focus 6);
    - a comparison of S0's and N's output that does not take it from
      `run`, so a path-bearing message goes unsettled on Windows, as CI
      found once (Review Focus 1; decision 25);
    - argument handling: the hidden flags shown anywhere, `--test` in any
      position, an empty or repeated option, a value given to a flag,
      any command's `--help` changed (Review Focus 7);
    - package identity: an input on which N's folded path and S0's
      `real_path` differ beyond ADR 0054 §5's list, or where N's M0011
      lacks the "may be one directory" clause where folding cannot tell
      (Review Focus 8; decisions 23 and 25);
    - a commit between Task 1 and Task 4 that builds wrong code or fails
      the suite, and the two versions the head does not have (decision
      20);
    - the records: ADR 0054, the CHANGELOG, the dated notes, the branch
      name in each, every count (318, 251, 67, 39, 9, 145, 30, 126, 229)
      and figure against the tree, ADR 0054's Status (`Accepted`) and the
      CI figures (decision 24);
    - each mutant of Step 8 that passes, the accepted ones included, and
      whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock`, a test that
      touches the internet, a temporary name that is not fixed or missing
      from the Global Constraints' list.
  - Each finding is fixed in one commit, `5.2b: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Steps 1 and 2 run again.
- [ ] **Step 11: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-52b`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 5.2b adds: S0's dump of packages, the WASM target and test
      mode (hidden `--test`), std resolved once for the dump; N's manifest
      reader, package loader, WASM target and test mode; the cases, the
      fuzzer's WASM and package inputs, the differential on both sides;
      ADR 0054 and the live heap;
    - that nothing a user runs changes: every `.nova` file the base
      tracks parses and checks as before, every package tests and builds
      as before, every `--help` is unchanged, and every `nova check`
      command line but the hidden form's gives what it gave;
    - the departures, marked **(differs)** here;
    - the figures: the suite's `2869 1 10`, the golden suite's
      `147 passed, 9 excluded`, `selfhost.rs`'s time here and CI's
      before and after option B (runs 41813500363 and 42006844810), the
      fuzzer's codes, the live heap, and that `Cargo.lock` is unchanged;
    - the user's decisions, and this plan's decisions, numbered as here,
      with decisions 12, 14, 16, 18, 20, 23, 24 and 25 marked as
      differing from the spec;
    - the mutants, those that pass by design and any gap they found, and
      the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems; on the
    `test` job's `ubuntu-24.04` leg, `cargo test`'s log holds
    `test release_golden_suite ... ok`, and on each system `selfhost.rs`'s
    tests pass; record each system's time for it, and the run's address,
    in the ledger, and compare each with run 42006844810's (Linux 37.90 s,
    macOS 35.77 s, Windows 60.48 s; decision 24) in the PR's body.
  - **If the differential fails on CI and not here,** record its report,
    stop and ask the user: a Windows path or line ending is the likeliest
    (decision 14). **If `selfhost.rs` takes more than 70 s on Windows,**
    or more than a minute on Linux or macOS, ask the user (spec §9 risk
    4; decisions 10, 18 and 24); about 60 s on Windows is accepted. Never
    lower a count.
- [ ] **Step 12: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 11's.
