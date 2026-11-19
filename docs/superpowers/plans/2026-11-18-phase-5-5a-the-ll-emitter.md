# Phase 5.5a, "the `.ll` emitter, ported to Nova, its output compared byte for byte", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** N's seventh stage, the `.ll` emitter, natively. S0 gains a
hidden option of its MIR dump, `nova check --dump-mir --ll FILE…` (ADR
0053's mechanism, as ADR 0057 §1, absent from every `--help`), which
prints after each root's MIR after form, for a root that lowered, `(Ll
ok)` and `nova_codegen_llvm::compile_ir`'s text of that module, byte for
byte the intermediate `build --release` writes, or one line `(Ll error
"…")` with the emitter's words; `--ll` with `--before-async` is refused.
The printing lives in `nova_driver::MirDumps`; S0's emitter, `nova-mir`
and every manifest stay as they are. N (`selfhost/`) ports the emitter
function for function (`emit_ll.nova`) and `RtFunc::symbol` and
`signature` (`RT_FUNC_SIGNATURES` in `mir.nova`), and `selfhost mir --std
DIR --ll FILE…` prints the same part. The differential
(`crates/nova-cli/tests/selfhost.rs`) compares the `.ll`, byte for byte
with exit codes, wherever 5.4 compares the after form: the native
corpus's roots that run (251 loose programs and 10 packages' `src/main.nova`,
210 of them printing a `.ll`, N's own among them), the 342 HIR cases that
reach MIR and the 103 roots of `tests/selfhost/cases/mir/` (both line
endings), 17 of them new (14 of `ir_tests.rs`'s sources, a float `%` case
and two hand cases), the MIR fuzzer's 120 inputs and the 25 depth
programs, which N emits on a 1 MB main thread. A coverage test finds
every statement, terminator, constant, (operator, class) pair and runtime
function the emitter handles in the compared `.ll`, or a reason in
`LL_UNREACHABLE`; the one reachable emitter error, float `%`, agrees on
both sides. A mismatch in a `.ll` is reported by its root, its line in
the `.ll`, its `define` and its block (spec §6.4). Nothing a user sees
changes: `nova check`, `nova parse` and `--dump-mir` without `--ll` give
what they gave, byte for byte, on every `.nova` file the base tracks, and
`nova test` and `nova build` on its 11 packages outside `crates/` (22
runs). 5.5a is 5.5's first half (the user's decision 1): no `clang`, no
linking, no `build` of N's own. ADR 0058 records the option, the form,
N's port, the cases, the coverage, the report, the figures and CI's.

**Architecture:**
- **S0's `--ll`** (Task 1): `crates/nova-cli/src/cmd/run.rs`
  (`DumpMirCmd`'s `--ll`, its refusal with `--before-async`,
  `MirDumps::new(before_async, ll)`); `crates/nova-driver/src/lib.rs`
  (`Wanted::ll`, `ll_part`, `MirDumps::new` and `unshared` taking `ll`,
  the `.ll` appended to a lowered root's after form, the unit tests in
  `mod ll_dumps`); `selfhost.rs`'s S0 side: S0's after form with `--ll`
  and N's without (`N_LL = false`, `as_n_prints`, decision 12),
  `split_ll`, `check_ll`, the flag, header and refusal tests extended,
  the depth test reading S0's `.ll`, and §3.3's
  `the_ll_dump_is_what_build_release_writes`; the float `%` case.
- **The cases and the coverage** (Task 2): the 16 other new cases under
  `cases/mir/`; `selfhost.rs`'s
  `every_form_the_emitter_handles_occurs_in_the_compared_ll` with its
  lists (`LL_STATEMENTS`, `LL_TERMINATORS`, `LL_BINS`, `LL_ERRORS`,
  `LL_UNREACHABLE`), `package_mir`, the cases' `.ll` kept
  (`MirCases::ll`), and the `\r\n` test's raw line break.
- **N's emitter** (Task 3): `selfhost/src/emit_ll.nova` (new),
  `RT_FUNC_SIGNATURES`, `rt_func_symbol`, `rt_func_signature` and
  `read_signature` (`mir.nova`), `selfhost mir --ll` and `ll_part`
  (`main.nova`), `emit_ll_tests.nova` (new); the switch gone and the
  `.ll` compared on every input; the `.ll` mismatch report in
  `tests/selfhost/compare.rs` and its test.
- **Records** (Task 4): ADR 0058 (new); `CHANGELOG.md`; dated notes in
  `docs/phase-5-plan.md`, `nova-spec/00-MASTER-SPEC.md` and
  `nova-spec/14-CODEGEN.md`.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` and every `Cargo.toml` stay as they are**
  (spec §12).
- N is built by `nova build` through Cranelift, as before. `clang-18`
  (CI's pin) for the golden suite, and Node 22 for the WASM tests, as
  before; nothing 5.5a adds needs either.

**Spec:**
`docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`,
approved by the user on 2026-11-18 (`db640e97`), with its dated amendment
of 2026-11-18, which this plan's commit appends (decision 18).
- Read its header, §1 to §16, and the amendment, before Task 1.
- The user's decisions are decisions 1 to 8 here, and the answer that
  allowed the prototype's push to CI decision 9.

**Base:** `db640e97` ("docs: the Phase 5.5a spec is approved"), `main`
`1570ddbd` (5.4's merge) and the spec's commit, whose full suite gives
`2835 1 10` here (the IPv6 test the only failure) and `selfhost.rs` 64
apart. The branch is `claude/loving-archimedes-frtf5q-55`, from
`db640e97`, then this plan's commit (decision 18).

**Checked on 2026-11-18, before execution:**
- **A prototype built 5.5a whole** on a local branch from `db640e97`:
  part 1, S0's side, `ae764bb4` (the hidden `--ll`), `df497c69` (the
  cases), `4ac8924a` (the harness's S0 side and the coverage test),
  `ba4e1926` (the `.ll`'s size by set, and `05-geometry` for §3.3's
  package), `f3b32068` (the driver's test of an extern block); part 2,
  `799c8ef6` (N's emitter, `selfhost mir --ll`, N's tests), `8cb1b42b`
  (the `.ll` compared on every input, and the report); part 3,
  `3c6f1a07` (ADR 0058, N's least stack with `--ll`, the notes); then
  `36f74a9a` (the `build --release` test turns backtraces off, decision
  9), `4d61b6d3` (ADR 0058's CI figures), `568e5439` (the doc comment
  part 1 put in the wrong place in the driver, put back), then two
  commits that change ADR 0058 alone: `9238286d` (§11's `.ll` sizes
  measured again at the head, with N on the debug runtime; §13 saying
  what was pushed to CI; §10's E0002) and `a903ad98` (§11 naming the
  merged tree, not a local commit, and the replay's MIR comparison).
  Nothing else: no test was added after the prototype, since every
  mutant is caught or passes by design (decision 16).
- **CI, measured from the prototype** (decision 9), on
  `claude/loving-archimedes-frtf5q-55-ci` with draft Sakeerin/nova#144,
  closed unmerged. The Selfhost job's `selfhost.rs`, S0 at `opt-level =
  1` and N on the release runtime, `finished in`:
  - run 44259975902 (head `8cb1b42b`, the push the user answered,
    decision 9): the three Selfhost jobs (`ubuntu-24.04`, `macos-latest`,
    `windows-latest`) failed at their `cargo test --test selfhost` step,
    every other job green (GitHub's job list), at
    `the_ll_dump_is_what_build_release_writes`, whose float `%` build
    printed `anyhow`'s backtrace after the error's words under CI's
    `RUST_BACKTRACE=1`;
  - the main session then pushed the fix, `36f74a9a`, with the records'
    commit `3c6f1a07` before it, to the same branch under the same draft
    PR, as its reading of that answer, not a second answer from the user
    (decision 9);
  - run 44260509656 (head `36f74a9a`): 19 of 19 jobs green; Linux 51.39 s
    (67 tests), macOS 29.94 s (65), Windows 93.86 s (61), against the
    lines of 80, 100 and 110 s (ADR 0057 §19); the Test job's main step
    summed `2838 passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9` on
    macOS and `2815, 0, 9` on Windows;
  - `main`'s run at 5.4's merge, `1570ddbd`, run 43853141585: green;
    `selfhost.rs` 77.03 s (64 tests) on Linux, 31.72 s (62) on macOS and
    79.94 s (58) on Windows.
- **The split** (decision 11 **(differs)**): three tasks test first and
  the records. `split.py` makes the local branch `split/5-5a` from
  `db640e97`, one commit a patch: Task 1 `1e0d79d0`, `c93b3e6a`;
  Task 2 `38a037bf`, `a9cef128`; Task 3 `98d60dc8`,
  `e2afcb2b`; Task 4 `7ebb1d97`. Its last tree is the prototype's
  `a903ad98` (`git diff a903ad98 split/5-5a` prints nothing), and every
  patch applies with plain `git apply`, in order, on `db640e97`, with no
  output, each applied tree its commit's; no added line holds trailing
  whitespace, a CR or a backslash-u escape.
- **The replay** (`replay.sh`, under the process limit, without
  `NOVA_EMBED_RUNTIME`, as the Test job runs), each step as written
  here: each `tN-tests` failed as its Step 2 records, and each `tN-code`
  passed the whole suite in three runs (`S`: `2837 1 10` from Task 1 on,
  the IPv6 test the one failure) and `selfhost.rs` apart (`SH`: 65, 66
  and 67 passed), `cargo fmt`, both clippy commands on the local stable
  (1.97) and on `+1.100.0` (CI's stable), and `nova fmt --check` with
  CI's arguments and with the earlier plans' superset; N's `nova test`
  (108 passed at Task 3); Task 4's examples found in S0's dump.
- **At the last commit,** `7ebb1d97` (Task 5), whose code is
  `e2afcb2b`'s (these figures and the mutants' were taken at the
  first split's Task 4 commit, whose tree differs from
  `7ebb1d97`'s in ADR 0058 alone, `9238286d`'s and `a903ad98`'s
  changes; CI's leg under `RUST_BACKTRACE=1`, in three parts, at
  `7ebb1d97` itself):
  - the whole suite as the Test job runs it, in three runs (Task 5,
    Step 1): `2837 1 10` (`1300 1 8`, `2 0 0` and `1535 0 2`, in 99, 1
    and 166 s); the test list 2,848, the base's 2,846 and the driver's
    two; `selfhost.rs` apart, 67 tests: the base's 64 and three added;
  - CI's Linux leg, `--all-features` with the golden suite required and
    `RUST_BACKTRACE=1`, as CI exports it, in the conventions' three
    parts: `1300 1 8`, `2 0 0` and `1535 0 2` in 142, 2 and 248 s,
    `2837 1 10` summed, the golden suite inside the third (as one
    command, 308 s; 284 s without `RUST_BACKTRACE=1`);
  - the golden suite: `release golden: 147 passed, 9 excluded, Ubuntu
    clang version 18.1.3 (1ubuntu1)`, in 19 s;
  - `nova parse` over every base-tracked `.nova` file: `parse: 3876 runs
    over 1938 files, 0 differ; --check exits nonzero on 123 at the base,
    123 now`, in 165 s; `nova check`: `check: 2045 checks, 0 outside std
    differ, 0 std files differ`, in two parts of 1029 and 1016, 199 and
    190 s; `nova test` and `nova build` on the 11 packages outside
    `crates/`: `packages: 22 runs, 0 differ`, in 41 s; `nova check
    --dump-mir` without `--ll`, both forms, on 1,955 roots in 40 batches:
    `0 differ`, in 76 s;
  - `--help`: `help: 22 of 22 identical; hidden flags shown: 0`;
    `usagecmp.sh`: `usage: 558 of 558 runs identical; in a terminal, 10
    of 10; --dump-ast as --dump-tokens: 25 of 25; --dump-resolution
    refused: 114 of 114; --dump-hir and --dump-mir refused: 522 of 522`,
    in 34 s;
  - Rust 1.78: clean, `Finished` in 45 s; `Cargo.lock` and every
    `Cargo.toml` unchanged;
  - `selfhost.rs` without the release runtime, as `SH` runs it: 67
    passed in 142.62 s (user CPU 525 s), and again with
    `RUST_BACKTRACE=1` exported, as CI's jobs run, 67 passed in 147.51 s;
    as the Selfhost job runs it (`NOVA_EMBED_RUNTIME=1`,
    `CARGO_PROFILE_DEV_OPT_LEVEL=1`), three times: 45.49, 41.64 and 41.18
    s (user CPU 146 to 159 s), after the workspace's build at `opt-level
    = 1` (53 s) and the test's (38 s); N's `nova test`: `108 passed` on
    both runtimes; the job's last step 1.35 s;
  - N's memory and stack against ADR 0058 §11 and §12, measured again
    (decision 17): on the release runtime N's root 211 MB without `--ll`
    and 303 MB with it, 23 and 24 collections, the largest live heap 101
    and 102 MB, as §11 records; the fuzzer's and the depth cases' rows
    within 1 MB and 1 collection of §11's, or as §11; on the debug
    runtime, N's root 212 and 320 MB, the largest live heap 102 and 148
    MB, §11's debug table (measured in this replay); all 25 depth
    programs on 1 MB on both runtimes, with and without `--ll`, least
    stacks 48 to 216 KB, each within two bisection steps (8 KB) of §12's
    two rounds.
- **The mutants** (decision 16): 79 breakages of what 5.5a adds, each
  one or more texts replaced in one file, run one at a time under the
  process limit, with `NOVA_EMBED_RUNTIME=1`, at `7ebb1d97` (Task 5,
  Step 8): S0's option, refusal and `(Ll` part (13), the harness's
  `split_ll`, `check_ll` and switchless after form (5), the coverage test
  (9), the escape case's raw line break (1), the backtrace fix (1), the
  `.ll` mismatch report (8), N's port (42: the prototype's twelve, each
  error's words, each signature class, the runtime table, the C
  wrapper, `ll_part` and `--ll`'s refusals). 76 are caught and 3 pass
  by design, each equivalent: the before form asking for the `.ll`, and
  an error's words quoted without escaping on either side (no word holds
  a byte the quoting escapes). The runs add up to 46 minutes.
- **What the checks left standing,** each recorded or handed on:
  - macOS and Windows are reasoned from the code here and were run on CI
    from the prototype's `36f74a9a`; this split's last tree differs from
    it by ADR 0058 (§13's CI figures, and §10, §11 and §13 as `9238286d`
    and `a903ad98` corrected them) and two doc comments moved in
    `crates/nova-driver/src/lib.rs`, no code; the tests are identical.
    The PR's runs are the check;
  - `ret_of` with the function winning is held by N's `@test` alone (no
    input can reach it, decision 14), the switch on `I8` and the ordered
    float compare by one case each;
  - the mutants that pass by design (decision 16).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` under `RUSTUP_TOOLCHAIN=1.78.0`
  with `RUSTFLAGS=-D warnings`. No `std::sync::LazyLock` (1.80), no
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79). 5.5a's Rust
  adds `str::replacen` (1.16), `Result::expect_err` (1.17),
  `btree_map::Entry::or_default` (1.28), `str::trim_end_matches` (1.30),
  `str::strip_prefix` and `str::strip_suffix` (1.45), `<[u8]>::strip_suffix`
  and `str::split_inclusive` (1.51), `str::split_once` (1.52), format
  strings' captured identifiers (`{ir}`, 1.58), `std::thread::scope` (1.63)
  and `std::sync::OnceLock` (1.70), and also `Iterator::sum` (1.11),
  `std::fs::read_to_string` and `std::fs::write` (1.26), inclusive range
  patterns (`0..=0x1f`, 1.26), `str::trim_end` (1.30) and a `[char; N]`
  pattern (`split([' ', ')'])`, 1.51): each at most 1.78. That is every
  std API and form the diff's added Rust lines use that is newer than
  1.10 (a `grep` over them; the rest, `join`, `as_str`, `Path::exists`
  and older, predate it).
- **No new crate; `Cargo.lock` and every `Cargo.toml` stay as they are**
  (spec §12). The driver already depends on `nova-codegen-llvm` and
  `nova-resolver`, which the `.ll` part calls. If a step seems to need a
  crate or a manifest change, stop that task there and ask the user; do
  not add the crate or change a manifest first.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`. 5.5a's Rust outside tests adds none; N's
  `unwrap()`s are on values its own code has checked (`read_signature`
  reads `RT_FUNC_SIGNATURES`' own strings).
- **No `unsafe`.** 5.5a adds none.
- **Tests never touch the internet,** bind no port, and need no `clang`
  (spec §8). The differential runs `nova` and the N it builds, nothing
  else; the cases that name an `extern` are checked, lowered and emitted,
  never run; §3.3's test runs `nova build --release` with `NOVA_CLANG`
  and `NOVA_LLC` naming a file that is not there, so no LLVM tool runs.
- **Temporary names are fixed** (spec §8): under `std::env::temp_dir()`.
  5.5a adds `nova-selfhost-ll-build-release` (`fresh_dir("ll-build-release")`:
  §3.3's test's `$NOVA_HOME` and the builds' outputs) and the driver's
  `nova-driver-ll-dump` (`the_ll_dump_is_the_module_build_release_emits`'s
  programs), both in Task 1. These are every fixed temporary name the
  diff adds (`grep -n 'fresh_dir(\|std_copy(\|temp_dir'` over its added
  lines); the coverage test, the report's test and N's new `@test`s write
  nothing. Each is removed and made afresh by the test that uses it. A
  test that needs `$NOVA_HOME` sets a fresh one.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and the golden suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the suite here with `NOVA_REQUIRE_NODE=1`. No test of 5.5a needs Node.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and
  passes. 5.5a adds no runtime program and changes neither lowering nor
  S0's emitter, so it stays at 147.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary, which only a build
  of the workspace writes. `nova build` of `selfhost/` links against
  that runtime, or, in a `nova` built with `NOVA_EMBED_RUNTIME=1`, against
  the release runtime embedded in it, unless `NOVA_RUNTIME_LIB` names one.
  Every run step starts with `cargo build --locked --workspace`
  (Conventions).
- **`selfhost.rs` is not in `cargo test --workspace`:** its target says
  `test = false` (5.3a). `S` runs the whole suite as the Test job does,
  `SH` runs `selfhost.rs` as the Selfhost job does; every figure here
  counts the two apart.
- **`NOVA_EMBED_RUNTIME` and `CARGO_PROFILE_DEV_OPT_LEVEL` are set for one
  command only, or for a whole step that says so** (Task 5, Steps 7 and
  8): a change of either makes cargo rebuild, and every other test that
  links a program then runs on that runtime or at that level. Each such
  step ends with `cargo build --locked --workspace` without them. Never
  build every test target at `opt-level = 1` (`cargo build --tests` or the
  whole suite): only `selfhost.rs`'s, as the Selfhost job does.
- **`RUST_BACKTRACE` is `0` in every shell here** (Conventions), and `1`
  for one command only where a step says so (Task 5, Steps 2, 7 and 8): CI's
  jobs export `RUST_BACKTRACE=1`, which broke §3.3's test in the
  prototype's first CI run (decision 9).
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
  runs, and so may `target/debug/incremental`; never `cargo clean` the
  main target. Each run of the whole suite leaves about 25 to 35 MB under
  `/tmp/nova-todo-*` (measured: CI's Linux leg, Task 5, Step 2, added 33
  MB in 9 directories), which is never cleaned. Every `cargo` command here
  selects the whole workspace (Conventions), so cargo keeps one copy of
  each test executable. The Rust 1.100 clippy (Conventions) uses a target
  directory of its own, removed after Task 4 (about 0.4 GB), and the Rust
  1.78 check (Task 5) one under `/dev/shm`, removed after; the
  `opt-level = 1` build of Task 5, Step 7 adds about 1 GB to `target/`
  the first time. The replay began with 8.4G free and ended with 7.7G.
  Check `df -h /` before any build at a new opt-level or toolchain.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.5a's
  patches or scripts holds one. Apply the patches with `patch_block`, from
  the committed plan.
- **No patch adds a CR, a NUL or trailing whitespace.** The inputs that
  need them are written by the tests at run time (the CRLF cases; the
  escape case holds `\r\n` and `\0` as escapes in its literals), not kept
  as files.
- **Nothing writes into the repository but the patches.** `nova build -o`
  writes N under the system's temporary directory and `nova test` writes
  nothing into the package; `git status --short` is empty after every run.
- **Push and open the pull request on the user's word only.** No push
  of this branch is allowed yet. The user's answer of 2026-11-18, "Push +
  draft PR", was for the prototype's `8cb1b42b` to
  `claude/loving-archimedes-frtf5q-55-ci` with draft Sakeerin/nova#144,
  measurement only, closed unmerged; the fix `36f74a9a` went to the same
  branch afterwards by the main session's reading of that answer, not a
  second answer (decision 9). Neither covers this branch. Task 5, Step
  11 stops before any push, and the main session asks the user; the
  branch is pushed and the PR opened only after their word.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.5a.
- **This machine is Linux.** macOS and Windows results come from CI; what
  differs there is reasoned from the code (Review Focus 1).

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Every shell exports it, `export P=…` with the
  directory's path, before a command that names `$P` (the Runs block
  below starts with it). Exported, not merely set: the scripts this plan
  writes read it from the environment, `adrcheck.py` (Task 4) and
  `parsecmp.sh`, `checkcmp.sh`, `pkgcmp.sh`, `mircmp.sh`, `helpcmp.sh`,
  `usagecmp.sh` and `mem.py` (Task 5), each run as a process of its own;
  without it `mem.py` stops with a `KeyError` and the shell scripts stop
  at their first line.
- **The executor's directory:** `$P/exec55/` (`mkdir -p "$P/exec55"`
  once, in Task 0). Every script this plan writes (Task 4's
  `adrcheck.py`, Task 5's) and every output of Task 5 (the suite's and
  CI's logs, the test lists, `differ/`, `pkgcmp/`, `heap/`, `loose.txt`,
  `mem.txt`, the `stack-*.tsv`) go there, nowhere else. `$P/plan55/`
  holds the replay's own scripts and evidence, which this plan cites:
  nothing here writes into it. The scripts are the replay's with that
  one directory written `exec55` for `plan55`; their own scratch files
  (`$P/pc-src-55`, `$P/help-55`, `$P/usage-*.txt`) they make and remove.
- **The ledger:** `$P/ledger-55.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-55-<task>.txt`:
  the subject the task names, a blank line, and these two trailer lines,
  exactly:

  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
  ```

  Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-55-<task>.txt"`. A task with a tests
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
- **Runs.** The replay ran every command in the group and with these
  variables set. Each shell puts itself under the cap and sets them too,
  with three shorthands, and runs every command from the repository's
  root; a shell that is not under the cap stops at once:

  ```sh
  # The scratch directory (above), exported: the plan's scripts read it.
  export P=/path/to/the/scratch/directory
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
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-55"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG NOVA_EMBED_RUNTIME CARGO_PROFILE_DEV_OPT_LEVEL
  mkdir -p "$NOVA_HOME"
  SH() { cargo test --locked --workspace --test selfhost -- "$@"; }
  S() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test run_tests -- release_golden_suite --nocapture "$@"; }
  ```

  So `SH` runs `selfhost.rs`, the differential and the flags' tests, `S`
  the whole suite without `selfhost.rs`, as the Test job runs it, and `G`
  the golden suite through LLVM as CI's Linux leg runs it. `NOVA_HOME`
  keeps a run by hand off `~/.nova`; every test that needs one sets its
  own.
- **Every `cargo` command selects the whole workspace,** `--workspace`
  with `--test NAME` or `--lib` to narrow it, never `-p`. Cargo unifies
  features over the packages a command selects, so each `-p` selection
  builds and links its own copy of every test executable it needs, which
  the disk does not have room for; a `--workspace` command reuses one
  copy. The exceptions are the clippy commands and `cargo fmt --all`,
  written as CI writes them, without `--workspace`: run from the root,
  whose `Cargo.toml` is a virtual manifest (`[workspace]` alone), cargo
  selects every member all the same.
- **Timeouts, and nothing in the background.**
  - Give every command a tool timeout of at least its stated time plus
    half again, never under 120 s and at most 600 s, the most a
    foreground command may take. A command whose stated time, with half
    again, passes 600 s is split into runs that each fit (below).
  - Run every command in the foreground: never with `&`, `nohup` or a
    tool's background mode, and never let a tool send one to the
    background. A block of several commands runs one command at a time,
    each with its own timeout.
  - Each code step builds the tests first, `cargo test --locked
    --workspace --no-run` as a command of its own (11 to 13 s), and
    `cargo test --locked --workspace --test selfhost --no-run` before
    `SH`, so that neither run holds a build.
  - **`S` in three runs,** everywhere, as the replay ran it. The whole
    suite, run as one command, would take about 270 to 290 s here, near
    enough to the tool's limit on a slower day:

    ```sh
    S --lib --bins > "$P/s-a.txt" 2>&1                       # the libraries' and binaries' tests
    S --doc > "$P/s-b.txt" 2>&1                              # the doc tests
    S --test '[!s]*' --test 's[!e]*' > "$P/s-c.txt" 2>&1     # every integration test but selfhost.rs
    cat "$P/s-a.txt" "$P/s-b.txt" "$P/s-c.txt" | grep '^test result' | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
    ```

    At each code commit they gave `1300 1 8` in 98 to 106 s (the IPv6
    test the failure), `2 0 0` in 1 s and `1535 0 2` in 160 to 166 s,
    which add up to the whole run's `2837 1 10` (the base's `1298 1 8`,
    `2 0 0` and `1535 0 2`, in 102, 1 and 181 s); `grep -c 'Running
    tests/selfhost.rs' "$P/s-c.txt"` prints `0`. The globs name every
    integration test but `selfhost`: a `--test` pattern takes a `test =
    false` target, which `--tests` leaves out, and `--tests` would run the
    libraries' tests again. A difference from the whole run's sum is
    recorded in the ledger.
  - Every single command here fits 600 s at twice the replay's time.
    The other commands that come closest: CI's Linux leg, in three parts
    (142, 2 and 248 s),
    `checkcmp.sh`, run in two halves (199 and 190 s), `parsecmp.sh` (165
    s), `SH` without the release runtime (141 to 170 s), `mem.py` on the
    debug runtime (276 s), and the mutants' batches, each one command,
    sized so that each fits under 600 s at twice the replay's time (Task
    5, Step 8).
  - **A child killed at `CHILD_LIMIT`.** The harness kills a child
    process past 120 s (`CHILD_LIMIT`); none came near it here (`SH`
    without the release runtime took 141 to 170 s in all, its N's emit
    of its own root about 8 s). If a test fails with a child killed there
    (`… did not finish within 120s: killed`), run that test alone once,
    `SH --exact <test>`. If it is killed again, stop and report to the
    main session. Never raise `CHILD_LIMIT` or deal the inputs into more
    parts without the main session.
  - If a command is killed by its timeout, or the shell restarts, restore
    what it left: after a killed mutant run, `git checkout -- <file>` of
    the mutant's file, and then `git status --short` must print nothing
    before anything goes on.
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
  PLAN=docs/superpowers/plans/2026-11-18-phase-5-5a-the-ll-emitter.md   # this file, committed
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
  nothing: no 5.5a patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. Task 3's code patch
  adds 989 lines (a block of 1,199) and Task
  4's 980 (a block of 1,037): each is one
  block all the same. The non-ASCII characters the patches add are `§`
  (Tasks 1, 2's tests, 3 and 4), `…` (the same), `é` and `─` (Task 2's
  escape case and Task 3's `@test`s of it), and `—`, `→` and `−` (Task 4).
  A block that does not apply means the tree differs from the plan's:
  stop, and record why in the ledger.
- **Each task** but Task 4 is test first: its tests' patch, their run and
  the expected failure, then its code's patch, its run and the expected
  pass. Task 4's patch is the records. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy,
    `cargo clippy --locked --all-targets --all-features -- -D warnings`, as
    CI runs it, which ends with `Finished` and no warning;
  - `selfhost.rs`'s clippy,
    `cargo clippy --locked --all-features --test selfhost -- -D warnings`,
    CI's second command (`--all-targets` skips a `test = false` target),
    which ends with `Finished` and no warning;
  - both clippy commands again on CI's stable, Rust 1.100, in a target
    directory of their own (`rustup toolchain install 1.100.0 --profile
    minimal -c clippy` once, if `rustup toolchain list` lacks it):
    `CARGO_TARGET_DIR="$P/target-1100" cargo +1.100.0 clippy …` with each
    command's arguments, each ending with `Finished` and no warning; the
    local stable (1.97 here) and CI's lint differently;
  - `target/debug/nova fmt --check std examples selfhost`, CI's arguments,
    and `target/debug/nova fmt --check std examples tests/wasm tests/dom
    tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost`, the
    earlier plans' superset, with the `nova` the step's build made, each
    printing nothing.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost
  if wrong. After Task 4, `rm -rf "$P/target-1100"`.


## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
check in the task that owns the code:

1. **Windows** (the `.ll` per host, `\r\n`, `HIR_ARGS_LIMIT`): the `.ll`
   is compared on each system with nothing normalized; a string literal's
   raw line break reaches it as `\0A`, or `\0D\0A` from a `\r\n` copy,
   which the cases' `\r\n` test asserts on the escape case; root lists go
   in parts under 16,000 bytes as in 5.4; `settled_stdout` rewrites no
   `.ll` line (none starts `  (Error`) →
   `the_mir_dumps_agree_on_the_corpus_and_the_cases`,
   `the_mir_dumps_agree_on_the_cases_with_windows_line_endings` (Tasks 1
   to 3) (mutant 28). The PR's Windows and macOS legs are the runs; the
   prototype's passed on both (decision 9).
2. **N's port, word for word** (`emit_ll.nova` against
   `nova-codegen-llvm/src/lib.rs`, `RT_FUNC_SIGNATURES` against
   `RtFunc::symbol` and `signature`): escapes by bytes in uppercase
   `\XX`, a float's 16 uppercase digits, lengths in bytes, the empty
   string's `\00`, `ret_of`'s order and its missing callee, `%rN` per
   function, strings by first use, each layout's sizes, the switch's
   class, each predicate, each error's words, the C wrapper → the
   differential on every input, N's six `@test`s (Task 3) (mutants 38 to
   72). The reviewer reads each N function beside its S0 function, same
   names.
3. **The `(Ll` part** (`ll_part` and the after form's hook in the
   driver, `mir_of` and `ll_part` in `main.nova`): present exactly when
   the root lowered, the text byte for byte with its last line break,
   the error's words quoted as the dumps quote → the driver's
   `the_ll_part_is_the_emitters_text_or_its_words_quoted`, `check_ll`,
   the header test (Tasks 1 and 3) (mutants 6 to 13, 73 to 76).
4. **The hidden option stays hidden** (`DumpMirCmd`'s `--ll`, its
   refusal with `--before-async`, N's parser): `--ll` once, after the
   flag; twice, with a value, or with `--before-async` refused alike on
   both sides; every `--help` and every base command line the base's →
   `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`,
   `both_sides_refuse_the_mir_dumps_shapes_alike`, `helpcmp.sh`,
   `usagecmp.sh` (Task 5, Step 5) (mutants 1 to 5, 77 to 79).
5. **The `.ll` is what `build --release` writes** (the after form's
   module, `keep_the_entry_main`, the full check against the splice):
   → `the_ll_dump_is_what_build_release_writes`, the driver's
   `the_ll_dump_is_the_module_build_release_emits`; under CI's
   `RUST_BACKTRACE=1` too (Task 1; Task 5, Step 7) (mutants 9, 17, 29).
6. **The harness keeps every input, and finds every form** (`split_ll`,
   `check_ll`, N's after form with `--ll`, the coverage test's lists and
   `LL_UNREACHABLE`): no `.ll` dropped or left uncompared, no form missing
   unnoticed → Tasks 1 to 3's tests (mutants 14 to 27).
7. **The report says where a `.ll` differs** (`compare.rs`'s walk of a
   root's `.ll`: its line, `define` and block, `outside any define`, the
   `(Ll` line, every other dump's report unchanged) →
   `the_ll_mismatch_report_names_the_define_and_block` and the earlier
   report tests (Task 3) (mutants 30 to 37).
8. **N's memory, stack and time** (spec §9, §14): N's root at about 303
   MB with `--ll`, every depth program on 1 MB on both runtimes with and
   without `--ll`; `selfhost.rs` on CI under the lines → ADR 0058 §11 to
   §13, measured again (Task 5, Step 7).

## Decisions

The user's, from the spec's header (2026-11-17):

1. **5.5 splits into 5.5a and 5.5b** (the user's decision 1, as given:
   5.5a the emitter and the `.ll` compared byte for byte on all three
   systems, with no `clang`; 5.5b the driver, linking, N's `build` and
   `check`, and the golden suite with S1, its questions asked when 5.5b
   starts). This plan is 5.5a's; spec §1's six questions stay open, as
   ADR 0058 §14 lists them.
2. **The `.ll` printed beside the MIR** (the user's decision 2, as given:
   a hidden option of `--dump-mir` prints the `.ll` after the MIR after
   form in the same process, N's `selfhost mir` the same, compared in the
   existing MIR tests, about +2 to 4 s of CI, every `--help` unchanged).
   As built: `nova check --dump-mir --ll` (Task 1) and `selfhost mir --std
   DIR --ll` (Task 3); about +1.4 s locally as the Selfhost job runs
   `selfhost.rs`, and CI's run under each line (decision 9).
3. **Every input that lowers** (the user's decision 3, as given: the
   corpus's 210 roots that lower clean, N's among them, 407 cases,
   `ir_tests.rs`'s sources as about 18 cases, the MIR fuzzer's 120
   inputs, the 25 depth cases and a new float `%` case; all three
   systems; no `clang`). As built: the 210 corpus roots, 423 cases that
   print a `.ll` (341 HIR cases and 82 of `cases/mir/`'s 103 roots; 406 at
   the base by the harness's lists, decision 14), 14 of `ir_tests.rs`'s
   sources (decision 14), the float `%` case and two hand cases, `Le` on
   `Float` and every escape class; spec §6.3's gaps filled so: `Switch`
   on `I8` by `ir_tests__bool_match_switches_on_i8_not_i64`, `Le` on
   `Float` by the hand case (Task 2).
4. **The emitter's errors ported as written, with a case** (the user's
   decision 4, as given: N ports every `bail!` as written and prints the
   error in the dump as S0 does; a float `%` case; the others recorded
   unreachable natively, as 5.4's E0011; S0's bug not fixed). As built:
   no `main`, `Await`, float `%` and float bitwise ported, each pinned by
   N's `each_error_of_the_emitter_is_in_s0s_words` and S0's
   `the_ll_part_is_the_emitters_text_or_its_words_quoted`; `Mem` has
   nothing to port, N's MIR having no `Mem` statement (ADR 0058 §5).
5. **The `.ll` mismatch report names the `define` and the block** (the
   user's decision 5, as given: as 5.4's MIR report, the root, the
   function and the `bbN` above the first differing line, both lines,
   the reproduce commands, with a test and mutants). Task 3; its test
   `the_ll_mismatch_report_names_the_define_and_block` and mutants 30 to
   37.
6. **No new fuzzer; every form found** (the user's decision 6, as given:
   5.4's inputs, and a test that every statement, terminator and constant
   form the emitter knows, and each of the 111 `RtFunc`s, occurs in the
   compared `.ll`; hand cases fill the gaps). As built:
   `every_form_the_emitter_handles_occurs_in_the_compared_ll` (Task 2),
   190 forms, 183 found, the other 7 in `LL_UNREACHABLE` with their
   reasons (ADR 0058 §8).
7. **CI measured from the prototype first** (the user's decision 7, as
   given: pushed to `claude/loving-archimedes-frtf5q-55-ci` with a draft
   pull request, measurement only, closed unmerged, as #142; the push
   itself asked again before it happens; past a line, the user is asked).
   Decision 9.
8. **The rule** (the user's decision 8, as given: from 5.5, a change to
   the emitter, `RtFunc`'s symbols or signatures, or the runtime's ABI
   changes S0 and N in one pull request; recorded in the ADR and the
   CHANGELOG). ADR 0058 §16 and the CHANGELOG's entry (Task 4).

The user's answer of 2026-11-18:

9. **CI measured from the prototype, under every line.**
   - **The user's answer, as given:** asked again before the push
     (decision 7) whether to push the prototype's `8cb1b42b` to
     `claude/loving-archimedes-frtf5q-55-ci` with a draft PR, measurement
     only, closed unmerged, and to ask again past a line, the user
     answered "Push + draft PR".
   - **What happened, the facts:** `8cb1b42b` went to that branch with
     draft Sakeerin/nova#144. Its run, 44259975902, failed the three
     Selfhost jobs (`ubuntu-24.04`, `macos-latest`, `windows-latest`) at
     their `cargo test --test selfhost` step, every other job green
     (GitHub's job list confirms it), at
     `the_ll_dump_is_what_build_release_writes`: CI's jobs export
     `RUST_BACKTRACE=1`, under which `nova build --release` on the float
     `%` case prints `anyhow`'s backtrace after the error's words. The
     prototype's `36f74a9a` runs that build with `RUST_BACKTRACE=0` and
     `RUST_LIB_BACKTRACE=0`. The main session then pushed `36f74a9a`,
     with the records' commit `3c6f1a07` before it, to the same branch
     under the same draft PR, without asking again: its reading of the
     answer above, not a separate answer from the user, who was told
     afterwards. That push gave run 44260509656, which passed all 19
     jobs, `selfhost.rs` in 51.39 s on Linux, 29.94 s on macOS and 93.86
     s on Windows, against 80, 100 and 110 s (the header's figures, ADR
     0058 §13). #144 was closed unmerged.
   - **In this plan:** the fix is in Task 1's test (decision 11); `SH`
     runs once with `RUST_BACKTRACE=1` exported at the last commit (Task
     5, Step 7), and so does CI's Linux leg (Task 5, Step 2).

How this plan is cut and checked:

10. **The spec's recommendations, not asked, as taken** (spec header):
    the option `--ll`, refused with `--before-async`; the `.ll` after
    the root's `(Diagnostics)` under `(Ll ok)`, an error one line `(Ll
    error "…")`, a root that does not lower none; the new cases in
    `cases/mir/`; the coverage test reading the MIR after form of the
    roots whose `.ll` was compared, and the `.ll` for what only it shows;
    the driver's guard not extended; N building a root's `.ll` whole; the
    `Mem` error with nothing to port; one pull request, split into tasks,
    test first.
11. **Three tasks test first, then the records** **(differs)**: spec
    §10's cut, with these changes:
    - **the float `%` case in Task 1,** not with the cases: §3.3's test
      and `check_ll`, both Task 1's, read it, and §10 puts §3.3's test in
      its first item;
    - **Task 2's code is its cases:** the coverage test and the `\r\n`
      test's raw line break are its tests, which fail for what the base's
      inputs lack (`Le` on `Float`, `Switch` on `I8`, a literal's raw line
      break; Task 2, Step 2), and the 16 cases are what make them pass;
    - **§10's fourth item, the measurements, is no task:** CI was
      measured from the prototype before this plan (decision 9); N's
      stack and memory are measured again, and the mutants run, at the
      last commit (Task 5, Steps 7 and 8);
    - **the prototype's three fixes are folded in where they belong:**
      `36f74a9a`'s backtrace fix into Task 1's §3.3 test, `568e5439`'s
      doc comments into Task 1's driver, `f3b32068`'s extern block into
      Task 1's unit test.
    The intermediate trees: Task 1's, `run.rs` and the driver as the head
    has them, the float `%` case, and part 1's harness (`f3b32068`, with
    `36f74a9a`'s fix) without the coverage test, `package_mir`,
    `MirCases::ll` and the raw line break's check, the corpus test as the
    base has it; Task 2's, part 1's harness with the fix and every case;
    Task 3's, the head's but the records; Task 4's, the head's. Each tests
    commit is the previous code commit plus the next tree's test files:
    Task 1's `selfhost.rs`, the float `%` case and the driver's `mod
    ll_dumps` with its doc comment; Task 2's `selfhost.rs` alone; Task 3's
    `selfhost.rs` and N's `emit_ll_tests.nova`, imported by `main.nova`.
    `split.py` makes them and checks every commit's tree. Each task's
    commit, made after its code patch, builds and passes `S` and `SH`.
12. **S0's `.ll` checked alone before Task 3** (spec §10's first item;
    transitional, the prototype's part 1):
    - Tasks 1 and 2 run S0's after form with `--ll` (`MIR_LL_S0`) and
      N's without (`N_LL = false`); `as_n_prints` compares S0's output
      without its `(Ll` parts (`split_ll_bytes`) with N's; and S0's `.ll`
      is checked on S0 alone: `check_ll` on every after-form run (an
      `(Ll` part exactly when `lowered=true`, the float `%` case's error
      with its words, each `.ll` ending in a line break), Task 2's
      coverage test, and §3.3's test; so no `.ll` goes unchecked and no
      input is skipped unnoticed;
    - Task 3 removes all of it (`N_LL`, `as_n_prints`, `split_ll_bytes`,
      `split_ll` reading text alone): N's after form is `selfhost mir
      --ll` on every input, compared raw. A reviewer checks at Task 3's
      commit: `git grep -n 'N_LL\|as_n_prints\|split_ll_bytes' crates
      selfhost` prints nothing.
    No frozen list is needed: each task's N gives every input in the
    form it is asked for.
13. **The spec's open points as the prototype settled them** (ADR 0058
    §15), each cheap to change:
    - the switch turned on when S0's arguments held `--ll` and N's did
      not, rather than reading `N_LL`; `check_ll` ran whatever it said;
    - 5.4's refusal shapes keep plain `MIR_S0` on S0's side;
    - `nova_rt_str_new`, `ConstStr`'s, is not among the coverage's calls;
      a root whose `.ll` is an error adds only its error's form; each
      error is named through `LL_ERRORS`, an unknown one failing the
      test; the coverage test is a `#[test]` of its own, sharing its run
      with the corpus test through `OnceLock`s;
    - N writes the header first into the one builder; reads its runtime
      table (`RtTable`) once per module, since reading a `const` builds it
      anew; the escape function keeps S0's name, `escape_bytes`, as spec
      §5.1's list of S0's names has it, where §5.2 calls it "a new
      `escape_ll`" (the spec's own contradiction, resolved toward §5.1);
      `escape_bytes` writes onto the builder and the empty string's
      `\00` is a case at the global; the signature table holds strings
      (`"(Ptr, I64) -> Ptr"`); `ll_part` lives in `main.nova` beside
      `mir_of`;
    - the `.ll` is built whole on both sides.
14. **The counts and facts that proved wrong** **(differs; the
    amendment records each)** (ADR 0058 §15, which lists them as
    departures from the spec; the spec's amendment, decision 18):
    `ir_tests.rs` gives 14 cases, not about 18
    (six of its 20 sources were already compared); 406 cases lower at
    the base by the harness's lists, 423 with the 17 new, not 407 (the
    research's list of `cases/mir/`); clap's usage line for `--ll=x`
    names the flags used, `Usage: nova check --dump-mir --ll
    [FILES]...`, where spec §3.1 gives `[OPTIONS] [FILES]...` (`--ll`
    twice prints that), and where §3.3 ("`--ll` in no help or usage
    line") and §16's gate say no usage line shows `--ll`: clap names the
    flags given, as the base's `--before-async=1` already does; §3.3's package is `examples/05-geometry`, since
    `examples/01-hello-world` has no `nova.toml`; N's root takes 92 MB
    more RSS with `--ll` (211 to 303 MB), not 40 to 60; the `ret_of`
    mutant (41) is killed by N's `@test` alone, no input reaching it (a
    function's symbol and an extern's coincide only for `main`, which
    E0900, or E0002 in the same module, reserves, and every other
    function's holds a `.`), where spec
    §6.4 has every mutant fail the differential; and CI exports
    `RUST_BACKTRACE=1` (decision 9).
15. **The records name this branch and date 5.5a 2026-11-18,** as the
    prototype wrote them: ADR 0058's Status `Accepted (2026-11-18)` and
    `claude/loving-archimedes-frtf5q-55`; each dated note, in
    `docs/phase-5-plan.md`, `00-MASTER-SPEC.md` and `14-CODEGEN.md`, in
    the earlier notes' shape, `**Amended 2026-11-18 (branch
    `claude/loving-archimedes-frtf5q-55`, ADR 0058)`**, the phase plan's
    with `5.5a, ` before the branch. No record names a
    local branch or a scratch path. The ADR names five commits, none that
    exists only locally: `db640e97`, the spec's, this branch's first
    commit and the PR's; and four of the prototype's, each on the pushed
    `claude/loving-archimedes-frtf5q-55-ci` (draft #144, closed
    unmerged): `f3b32068` and `8cb1b42b`, where its figures were measured
    (as ADR 0057 names its prototype's), and `3c6f1a07` and `36f74a9a`,
    what was pushed to CI (§13). The figures measured again in the replay
    it gives as on "the merged tree", whose code is `36f74a9a`'s.
16. **The mutants** (Task 5, Step 8): 79 (S0 13, the harness 14, the
    escape case 1, the backtrace fix 1, the report 8, N 42), 76 caught
    and 3 passing by design, each equivalent (6, 13 and 76, with their
    reasons in Step 8). The prototype's twelve (ADR 0058 §10) are 38 to
    49, with the same outcomes: `ret_of` with the function winning (41)
    by N's `@test` alone (decision 14), the switch on `I8` (48) and the
    unordered float compare (49) each by one case. No check was added
    after the prototype: no mutant is a gap. A harness check is not
    weakened as a mutant of its own; each is met by a mutant of the
    behaviour it guards, which it catches: `check_ll`'s `(Ll` part only
    where the root lowered (8), its float `%` case and words (16, 17),
    the `(Ll ok)` line and the last line break (10, 11); the coverage
    test's lists, its unlisted and missing forms and `LL_UNREACHABLE`'s
    both ways (19 to 27); the `\r\n` test's raw line break (28). The runs
    add up to 46 minutes with N on the release runtime: a mutant of S0's
    48 s on average, of the harness's 26 to 42 s, of the report's 10 s,
    of N's 37 s.
17. **The figures, this machine's** (Task 5): 4 CPUs, Rust 1.97 (stable;
    1.100.0 for CI's clippy, 1.78.0 for the MSRV check), Node v22.23.4,
    `clang-18` 18.1.3. Its speed varies between sessions, by a factor of
    two in the prototype's (`SH` without the release runtime took 88 to
    216 s there, 141 to 170 s in the replay, the base's run included). N's memory and stack were
    measured again at the last commit (its N, build and inputs the head's,
    as they were at `8cb1b42b`, where the prototype measured them:
    `selfhost/` is the same, and the harness differs in §3.3's test's
    environment alone): on the release runtime, as ADR 0058 §11 has
    them, N's root 211 and 303 MB, 23 and 24 collections, the largest
    live heap 101 and 102 MB; the fuzzer's and the depth cases' rows
    within 1 MB and one collection; all 25 depth programs on 1 MB with and
    without `--ll` on both runtimes, each least stack within two bisection
    steps (8 KB) of §12's two rounds. On the debug runtime, which the
    prototype had not measured with `--ll`: N's root 212 and 320 MB, 23
    and 24 collections, the largest live heap 102 and 148 MB (Task 5,
    Step 7), a +46 MB jump in the live heap the release runtime does not
    show; ADR 0058 §11 records them (on the merged tree, whose code is
    `36f74a9a`'s), as
    it records the `.ll`'s size by set measured again at the head: N's
    root 390,750 lines and 12,252,987 bytes, all 778 roots 1,987,415 and
    61,964,175 (part 1's 379,439 and 11,842,977, measured before N held
    its own emitter, kept there as part 1's). These are local figures;
    CI's are decision 9's.
18. **The branch starts from `db640e97`:**
    `claude/loving-archimedes-frtf5q-55` from `db640e97` (`main` `1570ddbd`
    and the approved spec), then this plan's commit, which adds this file
    and appends the spec's dated amendment (2026-11-18): the counts and
    facts that proved wrong (decision 14) and CI's figures (decision 9).
19. **CI's lines and counts for the PR** (decisions 7 and 9): the
    Selfhost job's `selfhost.rs` is read on all three systems against the
    lines, 80 s on Linux, 100 s on macOS and 110 s on Windows, and against
    run 44260509656's 51.39, 29.94 and 93.86 s; past a line, the user is
    asked. The counts expected from the head: CI ran `36f74a9a`, from
    which the head differs by ADR 0058 (§13's figures; §10, §11 and §13
    as `9238286d` and `a903ad98` corrected them) and two doc comments
    moved in
    `crates/nova-driver/src/lib.rs`, no code, so its tests are identical:
    `selfhost.rs` 67 tests on Linux, 65 on macOS and
    61 on Windows (`main`'s run 43853141585: 64, 62 and 58; three added,
    none under a `cfg`); the Test job's main step, `cargo test --locked
    --workspace --all-features --no-fail-fast`, summing `2838 passed, 0
    failed, 10 ignored` on Linux, `2838 passed, 0 failed, 9 ignored` on
    macOS and `2815 passed, 0 failed, 9 ignored` on Windows, as run
    44260509656's did: the driver's two new unit tests over 5.4's. The
    advisory step after it, `-- --ignored` with `continue-on-error`, is
    not counted (it failed on Linux and macOS in that run, as it may).
    Here the Linux suite gives `2837 1 10`: the same 2,848 tests, the IPv6
    test failing for want of IPv6, which CI's runners have.

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-55`: from `db640e97`,
then this plan's commit, which adds this file and appends the spec's
dated amendment, so the patches apply to it as to `db640e97` (decision
18).

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-55`.
  - `git log --format='%h %s' -2` prints the plan's commit, then
    `db640e97 docs: the Phase 5.5a spec is approved`.
  - `git diff --stat db640e97 HEAD` lists only
    `docs/superpowers/plans/2026-11-18-phase-5-5a-the-ll-emitter.md` and
    `docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 5's review reads from it.
  - `df -h /` shows at least 6G free; if not, remove older duplicate test
    executables in `target/debug/deps` first. The replay began with 8.4G
    free.
- [ ] **Step 2: The tools and the process limit.**
  - `clang-18 --version | head -n 1` prints
    `Ubuntu clang version 18.1.3 (1ubuntu1)`: CI's pin, for the golden
    suite.
  - `node --version` prints `v22.23.4` here, for the WASM tests (any
    Node 22).
  - `RUSTUP_TOOLCHAIN=1.78.0 cargo --version` prints `cargo 1.78.0`, for
    Task 5; `rustup toolchain list` holds `1.100.0` (CI's stable, for the
    checks' second clippy; install it as the conventions say if not).
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
  cargo test --locked --workspace --no-run
  S --lib --bins > "$P/base-55-a.txt" 2>&1
  S --doc > "$P/base-55-b.txt" 2>&1
  S --test '[!s]*' --test 's[!e]*' > "$P/base-55-c.txt" 2>&1
  cat "$P"/base-55-[abc].txt > "$P/base-55.txt"
  grep '^test result' "$P/base-55.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-55.txt"
  mkdir -p "$P/exec55"
  [ -e "$P/nova-base-55" ] && mv "$P/nova-base-55" "$P/exec55/nova-base-55.replay"
  mkdir -p "$P/nova-base-55" && cp target/debug/nova target/debug/libnova_runtime.a "$P/nova-base-55/"
  sha256sum "$P/nova-base-55/nova" "$P/nova-base-55/libnova_runtime.a" "$P"/exec55/nova-base-55.replay/* 2>/dev/null
  cargo test --locked --workspace -- --list > "$P/base-list-55.txt" 2>&1
  grep -c ': test$' "$P/base-list-55.txt"
  cargo test --locked --workspace --test selfhost -- --list > "$P/base-list-sh-55.txt" 2>&1
  grep -c ': test$' "$P/base-list-sh-55.txt"
  cargo test --locked --workspace --test selfhost --no-run
  time SH
  ```

  Expected, from the replay on `db640e97`: `1298 1 8`, `2 0 0` and `1535
  0 2`, in 102, 1 and 181 s, `2835 1 10` summed; one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`, which needs IPv6, which this machine lacks, as in every
  earlier phase; the list holds `2846` names, the run's 2835 + 1 + 10;
  `selfhost.rs`'s list `64`, and `SH` `64 passed; 0 failed` in 170 s.
  The copy of `nova` and its runtime is the base's, for Task 5's
  comparisons (the replay's copy, built from `1570ddbd`, whose code
  `db640e97`'s is, had the sha256 `2fadc67a8b24734f…`, and is kept,
  moved aside to `$P/exec55/nova-base-55.replay/`, never overwritten);
  the lists are the base's names. Record both copies' sha256 values in
  the ledger. If the two `nova`s differ, as 5.4's ruling T0-1 found for
  its own, check `git diff --stat db640e97 HEAD -- . ':!docs'`: if it
  prints nothing, the source is the same and the difference is the
  build's (`nova` embeds no commit); record a ruling, keep the fresh copy
  (Task 5 compares the base's outputs, never its bytes), and compare the
  runtime libraries too. If it prints anything, stop and ask. Task 5 expects `2837 1 10` from the whole suite, 2 tests
  added, and 67 from `selfhost.rs` apart.
- [ ] **Step 4: The ledger.** Start `$P/ledger-55.md`, headed with the
  base commit, the baseline's figures and the process limit's form; and
  `$P/exec55`, the executor's directory (Conventions), made in Step 3.


### Task 1: S0's hidden `--ll`, the `.ll` after each lowered root's MIR

Spec §3, §4, §6.2; decisions 2, 4, 9, 10, 11 and 12.

**Files:**
- Create: `crates/nova-cli/tests/selfhost/cases/mir/ll_a_float_remainder_errs.nova`
- Modify: `crates/nova-cli/src/cmd/run.rs`, `crates/nova-driver/src/lib.rs`,
  `crates/nova-cli/tests/selfhost.rs`

- [ ] **Step 1: The tests.** `patch_block t1-tests`:
  - the driver's `mod ll_dumps`, two unit tests (temporary directory
    `nova-driver-ll-dump`):
    `the_ll_part_is_the_emitters_text_or_its_words_quoted` (a small
    module's `(Ll ok)` and text; `(Ll error "…")` with each error's words
    on modules built by hand: an `Await`, a float `Rem`, a float `BitAnd`,
    no `main`) and `the_ll_dump_is_the_module_build_release_emits` (on six
    programs, a hello, an async one, one with an extern block and a
    closure, a float `%`, no `main` and a type error, the dump with
    `--ll` is the dump without it, then `(Ll ok)` and `compile_ir` of
    `lower_to_mir`'s module, `build_program_release`'s; the before form
    takes no `.ll`);
  - `selfhost.rs`, S0's side of the `.ll` (decision 12): `MIR_LL_S0`, the
    after form's S0 arguments; `N_LL = false` and `as_n_prints`;
    `split_ll` and `split_ll_bytes`; `check_ll` on every after-form run;
    `FLOAT_REMAINDER_WORDS` and `FLOAT_REMAINDER_CASE`;
    `agree_on_mir_forms` returning the `.ll` parts too;
  - the flag test extended (`--ll` in no help; `nova check --ll`,
    `--dump-ll`, `--ll` before `--dump-mir`, after a plain `check`'s file
    and inside `--dump-mir=…` the base's errors; eleven `--ll` shapes one
    error each, the refusal with `--before-async` its own line; `--ll`
    before `--`); the header test (the after form without its `(Ll` parts
    is plain `--dump-mir`'s, and the three roots that lower have an `(Ll
    ok)` part); `both_sides_refuse_the_mir_dumps_shapes_alike` with
    `--ll` twice, `--ll=x` and `--ll` with `--before-async` both ways; the
    depth test reading S0's `.ll`; the fuzzer's test printing its size;
  - `the_ll_dump_is_what_build_release_writes` (spec §3.3; temporary
    directory `nova-selfhost-ll-build-release`), with the build's
    backtraces off (decision 9): `nova build --release FILE -o OUT` with
    no LLVM toolchain leaves `OUT.nova.ll`, byte for byte the dump's
    `.ll`, on `tests/runtime/generics.nova`,
    `examples/05-geometry/src/main.nova`, `tests/runtime/async_tasks.nova`,
    `tests/runtime/extern_ffi.nova` and `mir_every_native_form.nova`, and
    none on the float `%` case, whose stderr ends with the words the
    dump's `(Ll error` line quotes;
  - the float `%` case, `cases/mir/ll_a_float_remainder_errs.nova`.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished` and no warning.
  - `cargo test --locked --workspace --lib -- ll_dumps`. Expected: no
    test runs; `nova-driver`'s library tests do not compile, 6 errors:
    4 `E0061` (`this function takes 1 argument but 2 arguments were
    supplied`, `MirDumps::new`) and 2 `E0425` (`cannot find function
    `ll_part` in this scope`).
  - `cargo test --locked --workspace --test selfhost --no-run`, then `SH`
    (144 s). Expected: `58 passed; 7 failed`:
    `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`,
    `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`,
    `the_mir_dumps_agree_on_the_corpus_and_the_cases`,
    `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`,
    `the_mir_fuzzers_inputs_agree_and_hold_each_kind`,
    `n_lowers_the_depth_cases_on_a_1_mb_main_thread` and
    `the_ll_dump_is_what_build_release_writes`: the base's S0 refuses
    `--ll` (`error: unexpected argument '--ll' found`, exit 2), so its
    after form prints nothing (`the dumps differ in a.nova, at its
    header: one side's output ended before it`). The refusal test
    passes: the base's S0 and N refuse each new shape, exit 2.
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **the command line** (`run.rs`): `DumpMirCmd` takes `--ll` once;
    `dump_mir_code` refuses `--ll` with `--before-async`, `error:
    --dump-mir --ll takes no --before-async`, exit 2, before a missing
    file's error; `MirDumps::new(args.before_async, args.ll)`;
  - **the driver:** `Wanted::ll`, asked for with the after form alone;
    `ll_part`, `(Ll ok)` and `nova_codegen_llvm::compile_ir`'s text, or
    `(Ll error "…")` quoted by `nova_resolver::dump::quote`; the after
    form's module followed by its part in `HirDumps::dumps_with` when it
    lowered; `MirDumps::new` and `unshared` taking `ll`; `Wanted::ALL`,
    the guard's, without the `.ll` (spec §3.3). (`mod ll_dumps` came with
    its own doc comment in Step 1, above it, and `mod doc_test_builds`
    keeps its own: `568e5439`'s placement, decision 11.)
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --lib -- ll_dumps` (1 s). Expected:
    `2 passed`.
  - `cargo test --locked --workspace --no-run` (11 s). Expected:
    `Finished`.
  - `S` in three runs (106, 1 and 165 s). Expected: `1300 1 8`, `2 0 0`
    and `1535 0 2`, `2837 1 10`, the IPv6 test the one failure.
  - `cargo test --locked --workspace --test selfhost --no-run`, then `SH`
    (157 s). Expected: `65 passed; 0 failed`, the base's 64 and
    `the_ll_dump_is_what_build_release_writes`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `nova-driver: the hidden --ll prints each lowered
      root's .ll after its MIR`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 827f4df3..b569c95b 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -91,6 +91,17 @@
 //! in bodies that lowering and the specializer walk and in an instance's
 //! type argument, which N lowers on a 1 MB main thread.
 //!
+//! 5.5a (spec
+//! `docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`
+//! §3, §6): S0's after form is `nova check --dump-mir --ll FILE…`, each
+//! root that lowered followed by its `.ll`, `(Ll ok)` and what `build
+//! --release` writes, or `(Ll error "…")`, wherever the after form runs;
+//! [`split_ll`] cuts the `.ll` off after the comparison, so 5.4's checks
+//! read 5.4's text. Until N prints the `.ll` ([`N_LL`]), N's side runs
+//! without `--ll` and the `.ll` is checked on S0 alone: present exactly
+//! when the root lowered, and an error on the float `%` case alone. The
+//! `.ll` equals `build --release`'s intermediate on a few programs.
+//!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
 //!
@@ -614,7 +625,7 @@ fn compare_batch(
         (s0.join().expect("S0's thread"), n)
     });
     let (s0, n) = (s0?, n?);
-    let mismatch = first_difference(&s0.stdout, &n.stdout);
+    let mismatch = first_difference(&as_n_prints(dump, &s0.stdout), &n.stdout);
     if mismatch.is_none() && s0.code == Some(code) && n.code == Some(code) {
         return Ok(String::from_utf8_lossy(&s0.stdout).into_owned());
     }
@@ -638,6 +649,17 @@ fn compare_batch(
     Err(text)
 }
 
+/// S0's output of `dump` as N's must be: itself, or, when S0 was asked for
+/// the `.ll` and N was not ([`N_LL`]), without each root's `(Ll` part
+/// ([`split_ll`]), the `.ll` being checked on S0 alone (5.5a, spec §6.2).
+fn as_n_prints<'a>(dump: &Dump, s0: &'a [u8]) -> std::borrow::Cow<'a, [u8]> {
+    if dump.s0.contains(&"--ll") && !dump.n.contains(&"--ll") {
+        std::borrow::Cow::Owned(split_ll_bytes(s0).0)
+    } else {
+        std::borrow::Cow::Borrowed(s0)
+    }
+}
+
 /// Run `files`, relative to `dir`, through S0's and N's `dump`, `batch`
 /// files to a process, and fail with the report of the first difference.
 /// Both sides must exit with `code`. Returns S0's output.
@@ -4942,6 +4964,24 @@ fn the_body_fuzzers_inputs_agree_and_hold_each_aimed_code() {
 /// the roots; after the async transform.
 const MIR_S0: &[&str] = &["check", "--dump-mir"];
 
+/// S0's MIR dump after the async transform with each lowered root's `.ll`
+/// after its part (5.5a, spec §3.1, §6.1): `check --dump-mir --ll`, the
+/// after form wherever the harness runs it.
+const MIR_LL_S0: &[&str] = &["check", "--dump-mir", "--ll"];
+
+/// Whether N prints the `.ll`, `selfhost mir --ll` (5.5a, spec §5.1): not
+/// yet, so N's after form runs without `--ll`, S0's is compared without its
+/// `(Ll` parts ([`as_n_prints`]), and the `.ll` is checked on S0 alone
+/// ([`check_ll`]).
+const N_LL: bool = false;
+
+/// The words of the one emitter error a program reaches, float `%`, which
+/// type-checks and which the emitter refuses (5.5a, spec §2 item 5).
+const FLOAT_REMAINDER_WORDS: &str = "float remainder (%) is not supported by the LLVM backend";
+
+/// The case that reaches it, the only root whose `.ll` part is an error.
+const FLOAT_REMAINDER_CASE: &str = "ll_a_float_remainder_errs.nova";
+
 /// S0's MIR dump before the async transform: `check --dump-mir
 /// --before-async`.
 const MIR_BEFORE_S0: &[&str] = &["check", "--dump-mir", "--before-async"];
@@ -4951,7 +4991,8 @@ const MIR_BEFORE_S0: &[&str] = &["check", "--dump-mir", "--before-async"];
 const MIR_N: &[&str] = &["mir"];
 
 /// The MIR dump's two forms (spec 5.4 §3.1, §7.2): after the async
-/// transform, what the emitter reads, and before it, as lowering left it.
+/// transform, what the emitter reads, S0's with each lowered root's `.ll`
+/// (5.5a), and before it, as lowering left it.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 enum MirForm {
     After,
@@ -4962,7 +5003,7 @@ impl MirForm {
     /// S0's arguments for the form.
     fn s0(self) -> &'static [&'static str] {
         match self {
-            MirForm::After => MIR_S0,
+            MirForm::After => MIR_LL_S0,
             MirForm::Before => MIR_BEFORE_S0,
         }
     }
@@ -4970,7 +5011,8 @@ impl MirForm {
 
 /// The MIR dump through both sides in `form`: S0's `check --dump-mir`,
 /// and N's `mir --std DIR`, each with `--before-async` for the before form,
-/// followed by the roots.
+/// S0's with `--ll` for the after form, and N's too once it prints the
+/// `.ll` ([`N_LL`]), followed by the roots.
 fn mir_in(form: MirForm) -> &'static Dump {
     static DUMPS: [OnceLock<Dump>; 2] = [OnceLock::new(), OnceLock::new()];
     DUMPS[form as usize].get_or_init(|| {
@@ -4979,6 +5021,9 @@ fn mir_in(form: MirForm) -> &'static Dump {
         if form == MirForm::Before {
             n.push("--before-async");
         }
+        if form == MirForm::After && N_LL {
+            n.push("--ll");
+        }
         Dump {
             s0: form.s0(),
             n: leak_args(&n),
@@ -4991,7 +5036,8 @@ fn mir_in(form: MirForm) -> &'static Dump {
 /// through [`compare_batch`] when `with_n`, S0's alone through [`run`]
 /// otherwise; the roots' names under [`HIR_ARGS_LIMIT`], and the dump well
 /// formed ([`assert_well_formed`]) with a header and a `(Mir` line per
-/// root. Returns S0's output.
+/// root; in the after form, each root's `.ll` as [`check_ll`] says, cut
+/// off first ([`split_ll`]). Returns S0's output, the `.ll` included.
 fn mir_dump_of(form: MirForm, with_n: bool, dir: &Path, files: &[String], code: i32) -> String {
     let bytes: usize = files.iter().map(|f| f.len() + 1).sum();
     assert!(
@@ -5012,8 +5058,14 @@ fn mir_dump_of(form: MirForm, with_n: bool, dir: &Path, files: &[String], code:
         );
         String::from_utf8(s0.stdout).expect("the MIR dump is UTF-8")
     };
-    assert_well_formed(&text);
-    let parts = parts_by_root(&text);
+    let (mir, ll) = split_ll(&text);
+    if form == MirForm::After {
+        check_ll(&mir, &ll);
+    } else {
+        assert!(ll.lines().all(|l| l.starts_with("== ")), "{form:?}: {ll}");
+    }
+    assert_well_formed(&mir);
+    let parts = parts_by_root(&mir);
     assert_eq!(
         parts.iter().map(|(r, _)| r.as_str()).collect::<Vec<_>>(),
         files.iter().map(String::as_str).collect::<Vec<_>>(),
@@ -5045,6 +5097,74 @@ fn mir_dump_in(form: MirForm, with_n: bool, dir: &Path, files: &[String], code:
     })
 }
 
+/// S0's MIR dump split (5.5a, spec §6.2): 5.4's text, each root's part
+/// without its `(Ll` part, and the `.ll` parts, each root's header then its
+/// `(Ll` part, nothing for a root that printed none. A `(Ll` part runs from
+/// its `(Ll` line, which follows a root's `(Diagnostics`, to the next
+/// header or the end: no line of 5.4's grammar starts `(Ll`, and no `.ll`
+/// line starts `(` or `== ` (spec §4).
+fn split_ll(dump: &str) -> (String, String) {
+    let (mir, ll) = split_ll_bytes(dump.as_bytes());
+    (
+        String::from_utf8(mir).expect("the MIR dump is UTF-8"),
+        String::from_utf8(ll).expect("the .ll is UTF-8"),
+    )
+}
+
+/// [`split_ll`] on bytes, as the sides' outputs are compared.
+fn split_ll_bytes(dump: &[u8]) -> (Vec<u8>, Vec<u8>) {
+    let (mut mir, mut ll) = (Vec::with_capacity(dump.len()), Vec::new());
+    let mut in_ll = false;
+    for line in dump.split_inclusive(|&b| b == b'\n') {
+        if line.starts_with(b"== ") {
+            in_ll = false;
+            mir.extend_from_slice(line);
+            ll.extend_from_slice(line);
+            continue;
+        }
+        in_ll |= line.starts_with(b"(Ll ");
+        if in_ll {
+            ll.extend_from_slice(line);
+        } else {
+            mir.extend_from_slice(line);
+        }
+    }
+    (mir, ll)
+}
+
+/// The `.ll` parts of S0's after form on S0 alone (5.5a, spec §4, §6.2):
+/// `mir` and `ll` as [`split_ll`] gives them, the same roots in the same
+/// order. A root's `(Ll` part is there exactly when its `(Mir` line says
+/// `lowered=true`; it is `(Ll ok)` and the `.ll`, ending in a line break,
+/// or one line `(Ll error "…")`, which only [`FLOAT_REMAINDER_CASE`] gives,
+/// with [`FLOAT_REMAINDER_WORDS`].
+fn check_ll(mir: &str, ll: &str) {
+    let (mir, ll) = (parts_by_root(mir), parts_by_root(ll));
+    assert_eq!(mir.len(), ll.len(), "a .ll part with no root");
+    for ((root, part), (ll_root, ll)) in mir.iter().zip(&ll) {
+        assert_eq!(root, ll_root);
+        let lowered = part.starts_with("(Mir runs=true checked=true lowered=true ");
+        let float_remainder = Path::new(root).ends_with(FLOAT_REMAINDER_CASE);
+        if !lowered {
+            assert_eq!(ll, "", "{root}: a .ll for a root that did not lower");
+        } else if float_remainder {
+            assert_eq!(
+                ll,
+                &format!("(Ll error \"{FLOAT_REMAINDER_WORDS}\")\n"),
+                "{root}"
+            );
+        } else {
+            let text = ll
+                .strip_prefix("(Ll ok)\n")
+                .unwrap_or_else(|| panic!("{root}: {}", ll.lines().next().unwrap_or("")));
+            assert!(
+                text.starts_with("; LLVM IR emitted by the Nova compiler") && text.ends_with('\n'),
+                "{root}: a .ll that does not end in a line break"
+            );
+        }
+    }
+}
+
 /// Whether a root's part of the MIR dump holds a poll function, which the
 /// async transform made of an async function (spec 5.4 §2 item 9).
 fn holds_a_poll_function(part: &str) -> bool {
@@ -5060,9 +5180,10 @@ fn holds_a_poll_function(part: &str) -> bool {
 /// which [`the_mir_dumps_agree_on_the_corpus_and_the_cases`] checks. A root
 /// that cannot be read holds no poll function, so the roots compared in
 /// the before form exit 0. Returns S0's two outputs, after then before,
-/// each with every root's part in the roots' order.
-fn agree_on_mir_forms(dir: &Path, files: &[String], code: i32) -> (String, String) {
-    let after = mir_dump_in(MirForm::After, true, dir, files, code);
+/// each with every root's part in the roots' order, the after form without
+/// its `.ll` (5.5a), then the `.ll` parts ([`split_ll`]).
+fn agree_on_mir_forms(dir: &Path, files: &[String], code: i32) -> (String, String, String) {
+    let (after, ll) = split_ll(&mir_dump_in(MirForm::After, true, dir, files, code));
     let polls: Vec<bool> = parts_by_root(&after)
         .iter()
         .map(|(_, part)| holds_a_poll_function(part))
@@ -5097,7 +5218,7 @@ fn agree_on_mir_forms(dir: &Path, files: &[String], code: i32) -> (String, Strin
         compared.is_empty() && alone.is_empty(),
         "a part no root was dealt"
     );
-    (after, before)
+    (after, before, ll)
 }
 
 /// `nova check`'s help names no `--dump-mir`, every `nova check` command
@@ -5106,7 +5227,12 @@ fn agree_on_mir_forms(dir: &Path, files: &[String], code: i32) -> (String, Strin
 /// parser takes `--before-async` and files and refuses everything else
 /// with one error under its own usage line, `--std`, `--target`, `--test`,
 /// `--help`, `-h` and the flag again among them; with no file, one line on
-/// stderr; each exiting 2 with nothing on stdout (5.4, spec §3.1).
+/// stderr; each exiting 2 with nothing on stdout (5.4, spec §3.1). Since
+/// 5.5a it takes `--ll` once, anywhere after the flag, which no help names
+/// and the base's parser refuses as before, `nova check --ll` and
+/// `--dump-ll` among them; `--ll` twice or with a value is one clap error,
+/// and with `--before-async`, in either order, one line of its own (spec
+/// 5.5a §3.1).
 #[test]
 fn the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says() {
     let help = Command::new(nova())
@@ -5117,6 +5243,7 @@ fn the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says() {
     assert!(help.contains("--target"), "{help}");
     assert!(!help.contains("dump-mir"), "{help}");
     assert!(!help.contains("before-async"), "{help}");
+    assert!(!help.contains("--ll"), "{help}");
     let out = Command::new(nova())
         .args(["check", "--help", "--dump-mir"])
         .output()
@@ -5146,6 +5273,11 @@ fn the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says() {
             target,
         ),
         (&["check", "--test", "--dump-mir", "a.nova"], "--test", opts),
+        (&["check", "--ll", "a.nova"], "--ll", opts),
+        (&["check", "--dump-ll", "a.nova"], "--dump-ll", opts),
+        (&["check", "--ll", "--dump-mir", "a.nova"], "--ll", opts),
+        (&["check", "a.nova", "--ll"], "--ll", file),
+        (&["check", "--dump-mir=--ll", "a.nova"], "--dump-mir", opts),
     ] {
         let out = Command::new(nova())
             .args(args)
@@ -5180,6 +5312,17 @@ fn the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says() {
         &["--before-asyn", "a.nova"],
         &["--before-async"],
         &[],
+        &["--ll", "--ll", "a.nova"],
+        &["--ll", "a.nova", "--ll"],
+        &["--ll=x", "a.nova"],
+        &["--ll=", "a.nova"],
+        &["--ll", "--std", "a.nova"],
+        &["--ll", "--help"],
+        &["--ll", "--before-async", "a.nova"],
+        &["--before-async", "--ll", "a.nova"],
+        &["--ll", "a.nova", "--before-async"],
+        &["--ll", "--before-async"],
+        &["--ll"],
     ] {
         let out = Command::new(nova())
             .args(MIR_S0)
@@ -5195,9 +5338,15 @@ fn the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says() {
             stderr.matches("Usage: nova check --dump-mir ").count(),
             "{args:?}: {stderr}"
         );
-        // The form's own refusal, which is not clap's, names its flag.
+        // The form's own refusals, which are not clap's, name its flag:
+        // `--ll` with `--before-async` before a missing file.
         if !stderr.contains("Usage: ") {
-            assert_eq!(stderr, "error: --dump-mir needs a FILE\n", "{args:?}");
+            let refusal = if args.contains(&"--ll") && args.contains(&"--before-async") {
+                "error: --dump-mir --ll takes no --before-async\n"
+            } else {
+                "error: --dump-mir needs a FILE\n"
+            };
+            assert_eq!(stderr, refusal, "{args:?}");
         }
         assert!(out.stdout.is_empty(), "{args:?}");
     }
@@ -5214,6 +5363,32 @@ fn the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says() {
     assert!(String::from_utf8_lossy(&out.stdout).starts_with(
         "== -a.nova\n(Mir runs=true checked=true lowered=true async_lowered=false functions=1 "
     ));
+    // `--ll` anywhere after the flag, and before `--`.
+    for args in [&["--ll", "--", "-a.nova"][..], &["--", "-a.nova"]] {
+        let out = Command::new(nova())
+            .current_dir(&dir)
+            .args(MIR_S0)
+            .args(args)
+            .output()
+            .expect("run nova check --dump-mir -- -a.nova");
+        assert_eq!(out.status.code(), Some(0), "{args:?}");
+        let text = String::from_utf8_lossy(&out.stdout);
+        assert_eq!(
+            text.contains("\n(Diagnostics)\n(Ll ok)\n; LLVM IR"),
+            args[0] == "--ll"
+        );
+    }
+    let out = Command::new(nova())
+        .current_dir(&dir)
+        .args(MIR_S0)
+        .args(["--", "--ll"])
+        .output()
+        .expect("run nova check --dump-mir -- --ll");
+    assert_eq!(out.status.code(), Some(2));
+    assert_eq!(
+        String::from_utf8_lossy(&out.stdout),
+        "== --ll\nIOERR NotFound\n"
+    );
 }
 
 /// Each root's part follows a header with its path as given, in both
@@ -5224,7 +5399,9 @@ fn the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says() {
 /// checker, `checked=false` with no diagnostic; E0601 `lowered=false` with
 /// MIR's diagnostics, the first with no place; a clean program its
 /// functions, whose async ones the before form shows with their awaits
-/// (5.4, spec §4.1).
+/// (5.4, spec §4.1). The after form's roots that lowered, and they alone,
+/// are followed by their `.ll`, without which the after form is `--dump-mir`'s
+/// with no `--ll` (5.5a, spec §3.3, §4).
 #[test]
 fn the_mir_dump_has_a_header_per_root_and_says_how_far_each_got() {
     let dir = fresh_dir("mir-headers");
@@ -5264,7 +5441,19 @@ fn the_mir_dump_has_a_header_per_root_and_says_how_far_each_got() {
     .iter()
     .map(|f| f.to_string())
     .collect();
-    let (after, before) = agree_on_mir_forms(&dir, &files, 2);
+    let (after, before, ll) = agree_on_mir_forms(&dir, &files, 2);
+    let plain = run(&nova(), MIR_S0, &dir, &files).expect("run S0's MIR dump");
+    assert_eq!(plain.code, Some(2), "{}", plain.stderr);
+    assert_eq!(String::from_utf8_lossy(&plain.stdout), after);
+    let with_ll: Vec<String> = parts_by_root(&ll)
+        .into_iter()
+        .filter(|(_, part)| !part.is_empty())
+        .map(|(root, part)| {
+            assert!(part.starts_with("(Ll ok)\n"), "{root}: {part}");
+            root
+        })
+        .collect();
+    assert_eq!(with_ll, ["a.nova", "g.nova", "pkg/src/main.nova"]);
     for (form, out) in [(MirForm::After, &after), (MirForm::Before, &before)] {
         let parts: std::collections::BTreeMap<String, String> =
             parts_by_root(out).into_iter().collect();
@@ -5348,13 +5537,36 @@ fn every_run_of_a_mir_dump_settles_e0004s_paths_on_windows() {
 /// Both sides refuse the MIR dump's shapes alike, in both forms (5.4,
 /// spec §3.1, §6.3): no root, `--target` either way, `--test`,
 /// `--before-async` twice and std's part (S0's `--std`, N's `--std-part`)
-/// exit 2 with nothing on stdout.
+/// exit 2 with nothing on stdout; since 5.5a, `--ll` twice, with a value,
+/// and with `--before-async` in either order too (spec 5.5a §6.2), which N
+/// refuses today as any option it does not take.
 #[test]
 fn both_sides_refuse_the_mir_dumps_shapes_alike() {
     let file = "tests/runtime/array_helpers.nova";
     assert!(root().join(file).is_file());
+    let n = mir_in(MirForm::Before).n;
+    let n = &n[..n.len() - 1];
+    for extra in [
+        &["--ll", "--ll", file][..],
+        &["--ll=x", file],
+        &["--ll", "--before-async", file],
+        &["--before-async", "--ll", file],
+    ] {
+        let dump = Dump {
+            s0: leak_args(&[MIR_S0, extra].concat()),
+            n: leak_args(&[n, extra].concat()),
+        };
+        let out = compare_batch(&dump, &root(), &[], 2, &nova(), selfhost());
+        assert_eq!(out, Ok(String::new()), "{extra:?}");
+    }
     for form in [MirForm::After, MirForm::Before] {
         let n = mir_in(form).n;
+        let n = n.strip_suffix(&["--ll"]).unwrap_or(n);
+        let s0 = if form == MirForm::After {
+            MIR_S0
+        } else {
+            form.s0()
+        };
         // The before form's flag, a second time.
         let twice: &[&str] = match form {
             MirForm::After => &["--before-async", "--before-async", file],
@@ -5370,7 +5582,7 @@ fn both_sides_refuse_the_mir_dumps_shapes_alike() {
             (&["--std", file], &["--std", "std", file]),
         ] {
             let dump = Dump {
-                s0: leak_args(&[form.s0(), s0_extra].concat()),
+                s0: leak_args(&[s0, s0_extra].concat()),
                 n: leak_args(&[n, n_extra].concat()),
             };
             let out = compare_batch(&dump, &root(), &[], 2, &nova(), selfhost());
@@ -5467,15 +5679,15 @@ fn agree_on_mir_cases(check: &Path, mir: &Path) -> MirCases {
     let (loose, apart) = mir_hir_cases();
     let roots = case_roots(mir);
     assert!(roots.apart.is_empty());
-    let (mut after, mut before) = agree_on_mir_forms(check, loose, 0);
+    let (mut after, mut before, _) = agree_on_mir_forms(check, loose, 0);
     for dir in apart {
         let name = dir.file_name().expect("a case's directory");
-        let (a, b) = agree_on_mir_forms(&check.join(name), &["main.nova".to_string()], 0);
+        let (a, b, _) = agree_on_mir_forms(&check.join(name), &["main.nova".to_string()], 0);
         after.push_str(&a);
         before.push_str(&b);
     }
     let own: Vec<String> = roots.loose.into_iter().chain(roots.packages).collect();
-    let (a, b) = agree_on_mir_forms(mir, &own, 0);
+    let (a, b, _) = agree_on_mir_forms(mir, &own, 0);
     after.push_str(&a);
     before.push_str(&b);
     let hir = loose.len() + apart.len();
@@ -5707,11 +5919,11 @@ fn agree_on_mir_cases_with_crlf(compared: &mut bool) {
     *compared = true;
 }
 
-/// The loose corpus's MIR dump in both forms ([`agree_on_mir_forms`]),
-/// compared, once a run: the corpus test's, and the MIR fuzzer's bases are
-/// read from it.
-fn loose_mir() -> &'static (String, String) {
-    static DUMP: OnceLock<(String, String)> = OnceLock::new();
+/// The loose corpus's MIR dump in both forms and its `.ll` parts
+/// ([`agree_on_mir_forms`]), compared, once a run: the corpus test's, and
+/// the MIR fuzzer's bases are read from it.
+fn loose_mir() -> &'static (String, String, String) {
+    static DUMP: OnceLock<(String, String, String)> = OnceLock::new();
     DUMP.get_or_init(|| agree_on_mir_forms(&root(), &loose_corpus(), 0))
 }
 
@@ -5815,7 +6027,7 @@ fn the_mir_fuzzers_inputs_agree_and_hold_each_kind() {
             roots.push(format!("{seed}/{file}"));
         }
     }
-    let (after, _) = agree_on_mir_forms(&dir, &roots, 0);
+    let (after, _, ll) = agree_on_mir_forms(&dir, &roots, 0);
     let parts = parts_by_root(&after);
     assert_eq!(parts.len(), roots.len());
     let unclean: Vec<&str> = parts
@@ -5865,11 +6077,14 @@ fn the_mir_fuzzers_inputs_agree_and_hold_each_kind() {
     }
     eprintln!(
         "the MIR fuzzer over seeds {MIR_SEEDS:?}: {} inputs from {} bases, each kind {total:?}; \
-         {polls} compared in the before form; the after dump {} lines, {} bytes",
+         {polls} compared in the before form; the after dump {} lines, {} bytes; its .ll {} \
+         lines, {} bytes",
         roots.len(),
         bases.len(),
         after.lines().count(),
-        after.len()
+        after.len(),
+        ll.lines().count(),
+        ll.len()
     );
 }
 
@@ -5900,9 +6115,15 @@ fn n_lowers_the_depth_cases_on_a_1_mb_main_thread() {
             .expect("write a deep input");
         files.push(file);
     }
-    let s0 = run(&nova(), MIR_S0, &dir, &files).expect("run S0");
+    let s0 = run(&nova(), MirForm::After.s0(), &dir, &files).expect("run S0");
     assert_eq!(s0.code, Some(0), "{}", s0.stderr);
-    let text = String::from_utf8_lossy(&s0.stdout);
+    let (text, ll) = split_ll(&String::from_utf8_lossy(&s0.stdout));
+    check_ll(&text, &ll);
+    eprintln!(
+        "the depth cases' .ll: {} lines, {} bytes",
+        ll.lines().count(),
+        ll.len()
+    );
     let parts = parts_by_root(&text);
     assert_eq!(parts.len(), files.len());
     for (root, part) in &parts {
@@ -5932,7 +6153,8 @@ fn n_lowers_the_depth_cases_on_a_1_mb_main_thread() {
     let stack = if cfg!(unix) { Some(1024) } else { None };
     let (code, n) = n_args_on_stack(&dir, stack, &args);
     assert_eq!(code, Some(0), "N's `selfhost mir` on a {stack:?} KB stack");
-    if let Some(m) = first_difference(&s0.stdout, &n) {
+    let s0 = as_n_prints(mir_in(MirForm::After), &s0.stdout);
+    if let Some(m) = first_difference(&s0, &n) {
         panic!(
             "{}",
             report(&m, mir_in(MirForm::After), &dir, &nova(), selfhost())
@@ -5947,3 +6169,81 @@ fn n_lowers_the_depth_cases_on_a_1_mb_main_thread() {
         assert_ne!(code, Some(0), "N's `selfhost mir` passed on a 96 KB stack");
     }
 }
+
+/// The `.ll` the MIR dump prints is what `build --release` writes (5.5a,
+/// spec §3.3), on S0 alone: on a loose program, a package's
+/// `src/main.nova`, an async program, one with an extern and
+/// `mir_every_native_form.nova`, `nova build --release FILE -o OUT` with no
+/// LLVM toolchain (`NOVA_CLANG` and `NOVA_LLC` a file that is not there)
+/// and a fresh `$NOVA_HOME` exits 1 and leaves `OUT.nova.ll`, byte for byte
+/// the root's `.ll` in one `check --dump-mir --ll` of them all; on the
+/// float `%` case it leaves none, and its stderr ends with the words the
+/// dump's `(Ll error` line quotes.
+#[test]
+fn the_ll_dump_is_what_build_release_writes() {
+    let dir = fresh_dir("ll-build-release");
+    let home = dir.join("home");
+    std::fs::create_dir_all(&home).expect("create $NOVA_HOME");
+    let missing = dir.join("no-such-tool");
+    let mir = "crates/nova-cli/tests/selfhost/cases/mir";
+    let files: Vec<String> = [
+        "tests/runtime/generics.nova".to_string(),
+        "examples/05-geometry/src/main.nova".to_string(),
+        "tests/runtime/async_tasks.nova".to_string(),
+        "tests/runtime/extern_ffi.nova".to_string(),
+        format!("{mir}/mir_every_native_form.nova"),
+        format!("{mir}/{FLOAT_REMAINDER_CASE}"),
+    ]
+    .to_vec();
+    let s0 = run(&nova(), MIR_LL_S0, &root(), &files).expect("run S0's MIR dump");
+    assert_eq!(s0.code, Some(0), "{}", s0.stderr);
+    let (_, ll) = split_ll(&String::from_utf8_lossy(&s0.stdout));
+    let parts = parts_by_root(&ll);
+    assert_eq!(parts.len(), files.len());
+    for (i, (file, part)) in parts.iter().enumerate() {
+        let out = dir.join(format!("out{i}"));
+        let built = output_within(
+            Command::new(nova())
+                .current_dir(root())
+                .args(["build", "--release", file.as_str(), "-o"])
+                .arg(&out)
+                .env("NOVA_CLANG", &missing)
+                .env("NOVA_LLC", &missing)
+                .env("NOVA_HOME", &home)
+                // CI sets `RUST_BACKTRACE=1`, under which the error's
+                // backtrace follows its words on stderr.
+                .env("RUST_BACKTRACE", "0")
+                .env("RUST_LIB_BACKTRACE", "0")
+                .stdin(Stdio::null()),
+            CHILD_LIMIT,
+            &format!("nova build --release {file}"),
+        )
+        .unwrap_or_else(|e| panic!("{e}"));
+        let stderr = String::from_utf8_lossy(&built.stderr);
+        assert_eq!(built.status.code(), Some(1), "{file}: {stderr}");
+        let written = dir.join(format!("out{i}.nova.ll"));
+        if file.ends_with(FLOAT_REMAINDER_CASE) {
+            assert!(!written.exists(), "{file}");
+            assert_eq!(
+                part,
+                &format!("(Ll error \"{FLOAT_REMAINDER_WORDS}\")\n"),
+                "{file}"
+            );
+            assert!(
+                stderr.trim_end().ends_with(FLOAT_REMAINDER_WORDS),
+                "{file}: {stderr}"
+            );
+            continue;
+        }
+        assert!(
+            stderr.contains("release build failed; generated LLVM IR left at "),
+            "{file}: {stderr}"
+        );
+        let written = std::fs::read_to_string(&written).expect("read the intermediate .ll");
+        assert_eq!(
+            part.strip_prefix("(Ll ok)\n"),
+            Some(written.as_str()),
+            "{file}"
+        );
+    }
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ll_a_float_remainder_errs.nova b/crates/nova-cli/tests/selfhost/cases/mir/ll_a_float_remainder_errs.nova
new file mode 100644
index 00000000..d6e59648
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ll_a_float_remainder_errs.nova
@@ -0,0 +1,5 @@
+fn main() {
+    let a = 5.5
+    let b = 2.0
+    println("${a % b}")
+}
diff --git a/crates/nova-driver/src/lib.rs b/crates/nova-driver/src/lib.rs
index 0ba2a902..2477d9e6 100644
--- a/crates/nova-driver/src/lib.rs
+++ b/crates/nova-driver/src/lib.rs
@@ -2216,6 +2216,185 @@ mod resolution_dumps {
     }
 }
 
+/// The `.ll` part of the MIR dump (5.5a, spec
+/// `docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`
+/// §3, §4): each emitter error's words, and the `.ll` printed after a
+/// lowered root's MIR is `compile_ir` of what `build --release` lowers.
+#[cfg(test)]
+mod ll_dumps {
+    use super::*;
+    use nova_mir::{Block, Function, MirTy, Module, OperandClass, Stmt, Temp, Terminator};
+
+    /// A native module of one function, `name`, with `temps` and one block
+    /// of `stmts` that returns nothing.
+    fn module_of(name: &str, temps: Vec<MirTy>, stmts: Vec<Stmt>) -> Module {
+        Module {
+            functions: vec![Function {
+                name: name.to_string(),
+                params: 0,
+                takes_env: false,
+                capture_count: 0,
+                temps,
+                ret: MirTy::Unit,
+                is_async: false,
+                blocks: vec![Block {
+                    stmts,
+                    term: Terminator::Return(None),
+                }],
+            }],
+            externs: Vec::new(),
+            runtime_alloc: None,
+            runtime_exports: Vec::new(),
+        }
+    }
+
+    /// `main` applying `op` to two floats.
+    fn float_bin(op: hir::BinOp) -> Module {
+        module_of(
+            "main",
+            vec![MirTy::F64; 3],
+            vec![
+                Stmt::ConstFloat(Temp(0), 1.5),
+                Stmt::ConstFloat(Temp(1), 2.0),
+                Stmt::Bin {
+                    dst: Temp(2),
+                    op,
+                    class: OperandClass::Float,
+                    lhs: Temp(0),
+                    rhs: Temp(1),
+                },
+            ],
+        )
+    }
+
+    /// A root's `.ll` part (5.5a, spec §4): `(Ll ok)` and the emitter's
+    /// text, byte for byte, ending in its line break; or one line `(Ll
+    /// error "…")` with the emitter's own words, quoted, for each module
+    /// it refuses: an `Await` the transform left, a float `%`, a float
+    /// bitwise operator and no `main`.
+    #[test]
+    fn the_ll_part_is_the_emitters_text_or_its_words_quoted() {
+        let small = module_of("main", vec![MirTy::Unit], Vec::new());
+        let text = nova_codegen_llvm::compile_ir(&small).expect("a module the emitter takes");
+        assert_eq!(ll_part(&small), format!("(Ll ok)\n{text}"));
+        assert!(text.starts_with("; LLVM IR emitted by the Nova compiler (release backend).\n"));
+        assert!(
+            text.contains("\ndefine void @\"nova_main\"() {\nentry:\n"),
+            "{text}"
+        );
+        assert!(
+            text.ends_with("\n  call void @nova_main()\n  ret i32 0\n}\n"),
+            "{text}"
+        );
+        let awaits = module_of(
+            "main",
+            vec![MirTy::Ptr],
+            vec![Stmt::Await {
+                dst: None,
+                future: Temp(0),
+            }],
+        );
+        for (module, words) in [
+            (
+                awaits,
+                "an `.await` marker reached codegen: `nova-mir`'s async state-machine \
+                 transform must split every `async fn` body at its await points before a \
+                 module is emitted",
+            ),
+            (
+                float_bin(hir::BinOp::Rem),
+                "float remainder (%) is not supported by the LLVM backend",
+            ),
+            (
+                float_bin(hir::BinOp::BitAnd),
+                "bitwise operators are not defined for Float",
+            ),
+            (
+                module_of("helper", vec![MirTy::Unit], Vec::new()),
+                "no `main` function in MIR module",
+            ),
+        ] {
+            let error = nova_codegen_llvm::compile_ir(&module).expect_err(words);
+            assert_eq!(error.to_string(), words);
+            assert_eq!(ll_part(&module), format!("(Ll error \"{words}\")\n"));
+        }
+    }
+
+    /// The `.ll` the MIR dump prints after a root's after form is what
+    /// `build --release` emits (5.5a, spec §2 item 4, §3.3): the dump with
+    /// `--ll` is the dump without it, then `(Ll ok)` and
+    /// `nova_codegen_llvm::compile_ir` of the module [`lower_to_mir`] makes
+    /// of the root, `build_program_release`'s, from the full check, though
+    /// the dump splices std's shared functions in; for a float `%`, the
+    /// emitter's error both ways; a root that did not lower, or the before
+    /// form, has no `.ll`.
+    #[test]
+    fn the_ll_dump_is_the_module_build_release_emits() {
+        let dir = std::env::temp_dir().join("nova-driver-ll-dump");
+        let _ = std::fs::remove_dir_all(&dir);
+        std::fs::create_dir_all(&dir).unwrap();
+        let programs = [
+            (
+                "hello.nova",
+                "fn main() {\n    println(\"hi \\\"there\\\"\")\n}\n",
+            ),
+            (
+                "async.nova",
+                "async fn h() -> Int { 1 }\nasync fn k() -> Int { h().await + 1 }\n\
+                 fn main() { println(\"${block_on(k())}\") }\n",
+            ),
+            (
+                "extern.nova",
+                "extern \"C\" {\n    fn cbrt(x: Float) -> Float\n}\nfn main() {\n    \
+                 let k = 2.0\n    let f = |x: Float| cbrt(x) * k\n    println(\"${f(8.0)}\")\n}\n",
+            ),
+            (
+                "frem.nova",
+                "fn main() {\n    let a = 5.5\n    println(\"${a % 2.0}\")\n}\n",
+            ),
+            ("nomain.nova", "fn helper() {}\n"),
+            ("bad.nova", "fn main() { let x: Int = \"s\" }\n"),
+        ];
+        let (with, without, before) = (
+            MirDumps::new(false, true),
+            MirDumps::new(false, false),
+            MirDumps::new(true, true),
+        );
+        for (name, text) in programs {
+            let path = dir.join(name);
+            std::fs::write(&path, text).unwrap();
+            let mir = without.dump(&path).unwrap();
+            let dumped = with.dump(&path).unwrap();
+            let part = dumped
+                .strip_prefix(mir.as_str())
+                .unwrap_or_else(|| panic!("{name}: {dumped}"));
+            assert_eq!(
+                before.dump(&path).unwrap(),
+                MirDumps::new(true, false).dump(&path).unwrap(),
+                "{name}"
+            );
+            if !mir.starts_with("(Mir runs=true checked=true lowered=true ") {
+                assert!(["nomain.nova", "bad.nova"].contains(&name), "{name}: {mir}");
+                assert_eq!(part, "", "{name}");
+                continue;
+            }
+            let module = match lower_to_mir(Program::for_file(&path)).unwrap() {
+                Outcome::Ok(module) => module,
+                Outcome::Failed { errors } => panic!("{name}: {errors} errors"),
+            };
+            let expected = match nova_codegen_llvm::compile_ir(&module) {
+                Ok(ir) => format!("(Ll ok)\n{ir}"),
+                Err(e) => {
+                    assert_eq!(name, "frem.nova", "{e}");
+                    format!("(Ll error \"{e}\")\n")
+                }
+            };
+            assert_eq!(part, expected, "{name}");
+        }
+        let _ = std::fs::remove_dir_all(&dir);
+    }
+}
+
 /// What doc tests need of a build (spec
 /// `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md`
 /// §7, §11.2), where only this crate can see it.
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index e5a1dccc..9cb0a623 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -546,11 +546,13 @@ impl DumpKind {
 /// differential test. Hidden, with no stability promise, taken as
 /// [`DumpResolutionCmd`] is, only when `--dump-mir` is the first argument
 /// after `check`, by this parser of its own: after the flag it takes
-/// `--before-async` and files, `--` before a file that starts with `-`,
-/// the option once; it refuses everything else, `--std`, `--target`,
-/// `--test`, `--help` and `-h` included, in clap's words under the usage
-/// `nova check --dump-mir`. MIR has no std part, and the WASM target's and
-/// test mode's MIR are not compared.
+/// `--before-async`, `--ll` (5.5a, spec
+/// `docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`
+/// §3.1) and files, `--` before a file that starts with `-`, each option
+/// once; it refuses everything else, `--std`, `--target`, `--test`,
+/// `--help` and `-h` included, in clap's words under the usage `nova check
+/// --dump-mir`. MIR has no std part, and the WASM target's and test mode's
+/// MIR are not compared.
 #[derive(clap::Parser)]
 #[command(
     name = "nova check --dump-mir",
@@ -564,6 +566,11 @@ pub struct DumpMirCmd {
     #[arg(long)]
     before_async: bool,
 
+    /// After the MIR of each root that lowered, its LLVM IR, as `build
+    /// --release` writes it. Not with `--before-async`.
+    #[arg(long)]
+    ll: bool,
+
     /// Each a program's root. Taken as they are, so that an empty one is a
     /// file that is not there.
     #[arg(value_parser = OsStringValueParser::new().map(PathBuf::from))]
@@ -630,8 +637,9 @@ impl DumpCmd {
 /// `--test` and `--target wasm`, with neither a file nor `--std`, or with
 /// both, it prints one line to stderr, nothing to stdout, and exits 2, as
 /// N does. The MIR dump (5.4) takes files alone, `--before-async` the
-/// module before the async transform; with no file it prints one line to
-/// stderr and exits 2.
+/// module before the async transform, `--ll` each lowered root's `.ll`
+/// after its MIR (5.5a); with no file, or with `--ll` and
+/// `--before-async`, it prints one line to stderr and exits 2.
 pub fn dump(cmd: DumpCmd) -> Result<()> {
     let code = match &cmd {
         DumpCmd::Front { kind, args } => dump_code(*kind, args)?,
@@ -647,12 +655,17 @@ pub fn dump(cmd: DumpCmd) -> Result<()> {
 type DumpOne = dyn Fn(&Path) -> std::io::Result<String>;
 
 fn dump_mir_code(args: &DumpMirCmd) -> Result<i32> {
+    // The emitter reads the after form alone (5.5a, spec §3.1).
+    if args.ll && args.before_async {
+        eprintln!("error: --dump-mir --ll takes no --before-async");
+        return Ok(2);
+    }
     if args.files.is_empty() {
         eprintln!("error: --dump-mir needs a FILE");
         return Ok(2);
     }
     // std is parsed, resolved and checked once for every program.
-    let dumps = nova_driver::MirDumps::new(args.before_async);
+    let dumps = nova_driver::MirDumps::new(args.before_async, args.ll);
     dump_files(&args.files, &move |file| dumps.dump(file))
 }
 
diff --git a/crates/nova-driver/src/lib.rs b/crates/nova-driver/src/lib.rs
index 2477d9e6..5ab2c76b 100644
--- a/crates/nova-driver/src/lib.rs
+++ b/crates/nova-driver/src/lib.rs
@@ -520,18 +520,25 @@ impl HirDumps {
                     async_lowered: !before_async,
                 }
             });
+            // The after form's module, which `build --release` emits
+            // (5.5a, spec §3.3), is the one the `.ll` is made of.
+            if let (true, Some(Ok(module)), Some(mir)) = (wanted.ll, &after, &mut dumps.mir) {
+                mir.push_str(&ll_part(module));
+            }
             Some(dumps)
         })
     }
 }
 
-/// Which dumps of a root one check gives: the HIR dump's, and the MIR
-/// dump's after the async transform and before it (5.4).
+/// Which dumps of a root one check gives: the HIR dump's, the MIR dump's
+/// after the async transform and before it (5.4), and, after the after
+/// form of a root that lowered, its `.ll` (5.5a).
 #[derive(Clone, Copy)]
 struct Wanted {
     hir: bool,
     mir: bool,
     before: bool,
+    ll: bool,
 }
 
 impl Wanted {
@@ -539,17 +546,40 @@ impl Wanted {
         hir: true,
         mir: false,
         before: false,
+        ll: false,
     };
 
-    /// Every dump: the driver's guard's, natively.
+    /// Every dump but the `.ll`: the driver's guard's, natively. The `.ll`
+    /// is a function of the after form's module alone, so equal MIR gives
+    /// an equal `.ll` (5.5a, spec §3.3).
     #[cfg(test)]
     const ALL: Wanted = Wanted {
         hir: true,
         mir: true,
         before: true,
+        ll: false,
     };
 }
 
+/// A root's `.ll` part, after its MIR after form (5.5a, spec
+/// `docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`
+/// §4): `(Ll ok)` and `nova_codegen_llvm::compile_ir`'s text of `module`,
+/// byte for byte, ending in its own line break; or, when the emitter
+/// refuses the module, one line `(Ll error "…")`, the emitter's own words
+/// quoted as the dumps quote a string, without `build --release`'s context
+/// line.
+fn ll_part(module: &nova_mir::Module) -> String {
+    match nova_codegen_llvm::compile_ir(module) {
+        Ok(ir) => format!("(Ll ok)\n{ir}"),
+        Err(e) => {
+            let mut part = String::from("(Ll error ");
+            nova_resolver::dump::quote(&mut part, &e.to_string());
+            part.push_str(")\n");
+            part
+        }
+    }
+}
+
 /// One root's dumps, as [`Wanted`] asked for them.
 #[derive(Debug, Default, PartialEq, Eq)]
 struct RootDumps {
@@ -586,31 +616,38 @@ impl RootDumps {
 /// functions are kept once and spliced into each program's shared check
 /// with their ids moved ([`nova_typeck::shared_module`], spec §5); a
 /// program the shared path cannot check, and every program when std alone
-/// gives a diagnostic or holds a test, is checked in full. A user sees
-/// nothing of it: `check`, `build`, `run`, `test` and the language server
-/// keep their own paths.
+/// gives a diagnostic or holds a test, is checked in full. With `ll`, the
+/// after form of a root that lowered is followed by its `.ll`
+/// ([`ll_part`], 5.5a): `nova_codegen_llvm::compile_ir` of the module the
+/// after form prints, which is the module `build --release` emits. A user
+/// sees nothing of it: `check`, `build`, `run`, `test` and the language
+/// server keep their own paths.
 pub struct MirDumps {
     hir: HirDumps,
     before_async: bool,
+    ll: bool,
 }
 
 impl MirDumps {
     /// std's modules, natively, parsed, resolved, their items collected
     /// and their bodies checked once, each program's dump after the async
-    /// transform or, with `before_async`, before it.
-    pub fn new(before_async: bool) -> MirDumps {
+    /// transform or, with `before_async`, before it; with `ll`, the after
+    /// form followed by the `.ll` (the before form takes none).
+    pub fn new(before_async: bool, ll: bool) -> MirDumps {
         MirDumps {
             hir: HirDumps::new(Target::Native, false),
             before_async,
+            ll,
         }
     }
 
     /// [`MirDumps::new`], each program resolving and checking std again,
     /// as `check` does: the dumps are the same, which a test checks.
-    pub fn unshared(before_async: bool) -> MirDumps {
+    pub fn unshared(before_async: bool, ll: bool) -> MirDumps {
         MirDumps {
             hir: HirDumps::unshared(Target::Native, false),
             before_async,
+            ll,
         }
     }
 
@@ -627,6 +664,7 @@ impl MirDumps {
             hir: false,
             mir: !self.before_async,
             before: self.before_async,
+            ll: self.ll && !self.before_async,
         };
         let dumps = self.hir.dumps(path, wanted)?;
         Ok(if self.before_async {
````

### Task 2: The `.ll` cases, and every form the emitter handles found in the compared `.ll`

Spec §6.1, §6.3; decisions 3, 6 and 11.

**Files:**
- Create: 16 cases under `crates/nova-cli/tests/selfhost/cases/mir/`:
  `ir_tests__a_nested_match_emits_chains_and_a_flat_one_still_switches_1.nova`
  and `_2.nova`,
  `ir_tests__an_async_fn_containing_await_emits_a_resume_dispatch_and_an_indirect_poll.nova`,
  `ir_tests__arrays_emit_bounds_check_and_element_addressing.nova`,
  `ir_tests__bool_match_switches_on_i8_not_i64.nova`,
  `ir_tests__closures_emit_fat_pointer_and_indirect_call.nova`,
  `ir_tests__every_rt_func_is_declared_with_its_real_signature.nova`,
  `ir_tests__floats_use_hex_constants_and_fp_ops.nova`,
  `ir_tests__match_lowers_to_switch_with_trap_default.nova`,
  `ir_tests__panic_emits_declaration_and_call.nova`,
  `ir_tests__records_emit_field_addressing.nova`,
  `ir_tests__recursion_and_branch_lower.nova`,
  `ir_tests__string_plus_string_lowers_to_one_concat_call.nova`,
  `ir_tests__sum_match_still_switches_on_i64.nova`,
  `ll_a_float_le_is_an_ordered_compare.nova` and
  `ll_a_string_of_every_escape_class.nova`
- Modify: `crates/nova-cli/tests/selfhost.rs`

- [ ] **Step 1: The tests.** `patch_block t2-tests`:
  - `every_form_the_emitter_handles_occurs_in_the_compared_ll` (spec
    §6.3, decision 6), one run with the corpus test: in the roots that
    printed a `.ll`, the MIR after form's 25 statements (`LL_STATEMENTS`)
    and 5 terminators (`LL_TERMINATORS`), the 28 (operator, class) pairs
    (`LL_BINS`), `(ConstStr "")`, `Neg` on both classes, `Switch` on `I8`
    and `I64`, `Return` with and without a value, a `Unit` temp, a
    function taking an env, a call to an extern; in the `.ll`, each
    runtime function's `call`, read from each `.ll`'s header and the same
    in every one, a string global with each escape class, `c"\00"`, an
    extern's `declare`; each of `LL_ERRORS`; every form found or in
    `LL_UNREACHABLE` with its reason, none of those found, nothing
    unlisted; the `.ll`'s size by set;
  - `package_mir`, the packages' mains' run shared with the corpus test;
    `MirCases::ll`, the cases' `.ll` kept;
  - the cases' `\r\n` test: a string literal's raw line break reaches
    the `.ll` as `\0A`, and `\0D\0A` from the `\r\n` copy.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. Then `cargo test --locked --workspace --test selfhost
  --no-run`, and `SH` (161 s). Expected: `64 passed; 2 failed`:
  `every_form_the_emitter_handles_occurs_in_the_compared_ll` (`no
  compared .ll holds ["(Bin Le Float)", "(Switch I8)"]`) and
  `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`
  (`assertion failed: crlf.ll.contains(&raw(true)) &&
  !crlf.ll.contains(&raw(false))`): no input of the base's has a float
  `<=`, a `Bool` match or a literal's raw line break.
- [ ] **Step 3: The code.** `patch_block t2-code`: the 16 cases (ADR 0058
  §6): 14 of `ir_tests.rs`'s sources, `ir_tests__<test>[_n].nova`, the
  other six already compared (decision 14); by hand
  `ll_a_float_le_is_an_ordered_compare` (`<=` on two `Float`s) and
  `ll_a_string_of_every_escape_class` (`"`, `\`, a tab, a NUL, CR LF,
  UTF-8 of two and three bytes, an empty string and a raw line break).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --no-run`, then `S` in three runs
    (101, 1 and 160 s). Expected: `2837 1 10`.
  - `cargo test --locked --workspace --test selfhost --no-run`, then `SH`
    (151 s). Expected: `66 passed; 0 failed`.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `selfhost: the .ll cases, and every form the
      emitter handles found in the compared .ll`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index b569c95b..7b51eca5 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -99,8 +99,10 @@
 //! [`split_ll`] cuts the `.ll` off after the comparison, so 5.4's checks
 //! read 5.4's text. Until N prints the `.ll` ([`N_LL`]), N's side runs
 //! without `--ll` and the `.ll` is checked on S0 alone: present exactly
-//! when the root lowered, and an error on the float `%` case alone. The
-//! `.ll` equals `build --release`'s intermediate on a few programs.
+//! when the root lowered, an error on the float `%` case alone, and every
+//! form the emitter handles occurring in it, or listed unreachable
+//! ([`LL_UNREACHABLE`]). The `.ll` equals `build --release`'s intermediate
+//! on a few programs.
 //!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
@@ -4972,7 +4974,7 @@ const MIR_LL_S0: &[&str] = &["check", "--dump-mir", "--ll"];
 /// Whether N prints the `.ll`, `selfhost mir --ll` (5.5a, spec §5.1): not
 /// yet, so N's after form runs without `--ll`, S0's is compared without its
 /// `(Ll` parts ([`as_n_prints`]), and the `.ll` is checked on S0 alone
-/// ([`check_ll`]).
+/// ([`check_ll`], [`every_form_the_emitter_handles_occurs_in_the_compared_ll`]).
 const N_LL: bool = false;
 
 /// The words of the one emitter error a program reaches, float `%`, which
@@ -5666,6 +5668,8 @@ struct MirCases {
     after: String,
     /// S0's before form.
     before: String,
+    /// S0's `.ll` parts, a header per program (5.5a, [`split_ll`]).
+    ll: String,
     /// How many of the programs, the first, are HIR cases.
     hir: usize,
     /// How many programs in all.
@@ -5679,21 +5683,24 @@ fn agree_on_mir_cases(check: &Path, mir: &Path) -> MirCases {
     let (loose, apart) = mir_hir_cases();
     let roots = case_roots(mir);
     assert!(roots.apart.is_empty());
-    let (mut after, mut before, _) = agree_on_mir_forms(check, loose, 0);
+    let (mut after, mut before, mut ll) = agree_on_mir_forms(check, loose, 0);
     for dir in apart {
         let name = dir.file_name().expect("a case's directory");
-        let (a, b, _) = agree_on_mir_forms(&check.join(name), &["main.nova".to_string()], 0);
+        let (a, b, l) = agree_on_mir_forms(&check.join(name), &["main.nova".to_string()], 0);
         after.push_str(&a);
         before.push_str(&b);
+        ll.push_str(&l);
     }
     let own: Vec<String> = roots.loose.into_iter().chain(roots.packages).collect();
-    let (a, b, _) = agree_on_mir_forms(mir, &own, 0);
+    let (a, b, l) = agree_on_mir_forms(mir, &own, 0);
     after.push_str(&a);
     before.push_str(&b);
+    ll.push_str(&l);
     let hir = loose.len() + apart.len();
     MirCases {
         after,
         before,
+        ll,
         hir,
         count: hir + own.len(),
     }
@@ -5717,7 +5724,6 @@ fn mir_cases() -> &'static MirCases {
 /// ([`MIR_CODES`]).
 #[test]
 fn the_mir_dumps_agree_on_the_corpus_and_the_cases() {
-    let root = root();
     let (loose, packages) = mir_corpus();
     assert!(loose.len() >= 240, "only {} loose programs", loose.len());
     assert!(
@@ -5725,14 +5731,12 @@ fn the_mir_dumps_agree_on_the_corpus_and_the_cases() {
         "only {} packages' src/main.nova",
         packages.len()
     );
-    // The loose programs, the packages' mains and the cases side by side;
-    // the loose programs' once a run, which the MIR fuzzer's bases read.
+    // The loose programs, the packages' mains and the cases side by side,
+    // each once a run: the MIR fuzzer's bases read the loose programs', and
+    // the coverage test of the `.ll` all three (5.5a).
     let (sets, cases) = std::thread::scope(|scope| {
         let cases = scope.spawn(mir_cases);
-        let runs = vec![
-            scope.spawn(|| loose_mir().clone()),
-            scope.spawn(|| agree_on_mir_forms(&root, &packages, 0)),
-        ];
+        let runs = vec![scope.spawn(loose_mir), scope.spawn(package_mir)];
         (joined(runs), joined(vec![cases]).remove(0))
     });
     let corpus = [
@@ -5854,6 +5858,394 @@ fn the_mir_dumps_agree_on_the_corpus_and_the_cases() {
     eprintln!("the MIR cases' codes: {codes:?}");
 }
 
+/// The statements S0's emitter takes from a native build's MIR after the
+/// async transform, by the MIR dump's kinds (5.5a, spec §6.3): every
+/// native statement but `Await`, which the transform consumes and whose
+/// arm is an error ([`LL_ERRORS`]).
+const LL_STATEMENTS: [&str; 25] = [
+    "ConstInt",
+    "ConstFloat",
+    "ConstBool",
+    "ConstStr",
+    "ConstUnit",
+    "Copy",
+    "Bin",
+    "Neg",
+    "Not",
+    "BitNot",
+    "Call",
+    "CallIndirect",
+    "CallRuntime",
+    "MakeClosure",
+    "MakeSum",
+    "SumTag",
+    "SumField",
+    "MakeRecord",
+    "RecordField",
+    "SetField",
+    "MakeArray",
+    "ArrayAlloc",
+    "ArrayLen",
+    "ArrayGet",
+    "ArraySet",
+];
+
+/// The terminators, by the MIR dump's kinds.
+const LL_TERMINATORS: [&str; 5] = ["Goto", "Branch", "Switch", "Return", "Trap"];
+
+/// The (operator, class) pairs the type checker admits in a `Bin` (spec
+/// 5.5a §6.3): 16 on `Int`, 10 on `Float`, `Eq` and `Ne` on `Bool`. A float
+/// `%` is an error ([`LL_ERRORS`]).
+const LL_BINS: [(&str, &[&str]); 3] = [
+    (
+        "Int",
+        &[
+            "Add", "Sub", "Mul", "Div", "Rem", "BitAnd", "BitOr", "BitXor", "Shl", "Shr", "Eq",
+            "Ne", "Lt", "Le", "Gt", "Ge",
+        ],
+    ),
+    (
+        "Float",
+        &[
+            "Add", "Sub", "Mul", "Div", "Eq", "Ne", "Lt", "Le", "Gt", "Ge",
+        ],
+    ),
+    ("Bool", &["Eq", "Ne"]),
+];
+
+/// The emitter's errors (`bail!`), each by a name and the start of its
+/// words: a root's `(Ll error "…")` line is the form `(Ll error NAME)`.
+const LL_ERRORS: [(&str, &str); 5] = [
+    ("no main", "no `main` function in MIR module"),
+    ("Await", "an `.await` marker reached codegen: "),
+    ("Mem", "a WASM memory access ("),
+    ("float %", FLOAT_REMAINDER_WORDS),
+    (
+        "float bitwise",
+        "bitwise operators are not defined for Float",
+    ),
+];
+
+/// The forms of [`ll_forms`] the compared `.ll` may lack, each with its
+/// reason (5.5a, spec §2 items 5 and 7, §5.3, §6.3).
+const LL_UNREACHABLE: [(&str, &str); 7] = [
+    (
+        "call @nova_rt_test_selector",
+        "TestSelector: only test mode's synthesized `main` calls it, and test mode's code \
+         generation is not compared",
+    ),
+    (
+        "call @nova_rt_task_is_done",
+        "TaskIsDone: no native std function calls it, and a program's call is E0001",
+    ),
+    (
+        "call @nova_rt_task_take_output",
+        "TaskTakeOutput: no MIR statement emits it",
+    ),
+    (
+        "(Ll error no main)",
+        "lowering gives E0601 first, so no module without `main` reaches the emitter",
+    ),
+    (
+        "(Ll error Await)",
+        "an `Await` is in the before form alone, which takes no `.ll`",
+    ),
+    (
+        "(Ll error Mem)",
+        "a memory access is in a WASM build's MIR alone",
+    ),
+    (
+        "(Ll error float bitwise)",
+        "`&`, `|`, `^`, `<<` and `&=` on `Float` are E0013 and `~` E0010 before MIR, and lowering \
+         makes no other float `Bin`",
+    ),
+];
+
+/// Every form the emitter handles (5.5a, spec §6.3), as
+/// [`ll_forms_of`] names what it finds: in the MIR after form, each
+/// statement and terminator by its kind, `(ConstStr "")`, each `Bin` pair
+/// of [`LL_BINS`] as `(Bin OP CLASS)`, `(Neg Int)` and `(Neg Float)`,
+/// `(Switch I8)` and `(Switch I64)` by the discriminant's class, `(Return
+/// a value)` and `(Return _)`, `(Temp Unit)`, `(Function takes_env=true)`
+/// and `(Call an extern)`; in the `.ll`, `call @SYMBOL` of each of
+/// `runtime`, a string global with each escape (`\22`, `\5C`, `a control
+/// byte`, `a byte past ASCII`), `c"\00"` and `declare an extern`; and each
+/// of [`LL_ERRORS`].
+fn ll_forms(runtime: &[String]) -> Vec<String> {
+    let mut forms: Vec<String> = LL_STATEMENTS
+        .iter()
+        .chain(&LL_TERMINATORS)
+        .map(|k| format!("({k}"))
+        .collect();
+    for (class, ops) in LL_BINS {
+        forms.extend(ops.iter().map(|op| format!("(Bin {op} {class})")));
+    }
+    forms.extend(
+        [
+            "(ConstStr \"\")",
+            "(Neg Int)",
+            "(Neg Float)",
+            "(Switch I8)",
+            "(Switch I64)",
+            "(Return a value)",
+            "(Return _)",
+            "(Temp Unit)",
+            "(Function takes_env=true)",
+            "(Call an extern)",
+            "\\22",
+            "\\5C",
+            "a control byte",
+            "a byte past ASCII",
+            "c\"\\00\"",
+            "declare an extern",
+        ]
+        .map(String::from),
+    );
+    forms.extend(runtime.iter().map(|s| format!("call @{s}")));
+    forms.extend(
+        LL_ERRORS
+            .iter()
+            .map(|(name, _)| format!("(Ll error {name})")),
+    );
+    forms
+}
+
+/// The runtime functions an `.ll` declares, in order: the lines between
+/// `llvm.trap`'s declaration and `nova_rt_str_new`'s, each `declare TYPE
+/// @SYMBOL(…)` (spec 5.5a §6.3: every `.ll` declares them all).
+fn ll_runtime(ll: &str) -> Vec<String> {
+    ll.lines()
+        .skip_while(|l| *l != "declare void @llvm.trap()")
+        .skip(1)
+        .take_while(|l| *l != "declare ptr @nova_rt_str_new(ptr, i64)")
+        .map(|l| {
+            let symbol = l.split('@').nth(1).and_then(|r| r.split('(').next());
+            symbol
+                .unwrap_or_else(|| panic!("a declaration: {l}"))
+                .to_string()
+        })
+        .collect()
+}
+
+/// The forms of [`ll_forms`] in one root's MIR after form, `mir`, and its
+/// `.ll` part, `ll` ([`split_ll`]), added to `found` with a count each: an
+/// error's form alone for a root whose `.ll` is an error, whose MIR reaches
+/// no `.ll`.
+fn ll_forms_of(mir: &str, ll: &str, found: &mut std::collections::BTreeMap<String, usize>) {
+    let mut add = |form: String| *found.entry(form).or_default() += 1;
+    if let Some(error) = ll.strip_prefix("(Ll error \"") {
+        let (name, _) = LL_ERRORS
+            .iter()
+            .find(|(_, words)| error.starts_with(words))
+            .unwrap_or_else(|| panic!("an error of no list: {ll}"));
+        add(format!("(Ll error {name})"));
+        return;
+    }
+    let mut externs: Vec<&str> = Vec::new();
+    let mut temps: Vec<&str> = Vec::new();
+    for line in mir.lines() {
+        if let Some(rest) = line.strip_prefix("  (Extern \"") {
+            externs.push(rest.split('"').next().unwrap_or(""));
+        } else if line.starts_with("  (Function ") {
+            temps.clear();
+            if line.contains(" takes_env=true ") {
+                add("(Function takes_env=true)".to_string());
+            }
+        } else if let Some(rest) = line.strip_prefix("    (Temp t") {
+            let class = rest.split([' ', ')']).nth(1).unwrap_or("");
+            temps.push(class);
+            if class == "Unit" {
+                add("(Temp Unit)".to_string());
+            }
+        } else if let Some(rest) = line.strip_prefix("      (") {
+            let fields: Vec<&str> = rest.split(' ').collect();
+            let kind = fields[0].trim_end_matches(')');
+            add(format!("({kind}"));
+            match kind {
+                "ConstStr" if line.ends_with(" \"\")") => add("(ConstStr \"\")".to_string()),
+                "Bin" => add(format!("(Bin {} {})", fields[2], fields[3])),
+                "Neg" => add(format!("(Neg {})", fields[2])),
+                "Switch" => {
+                    let disc: usize = fields[1][1..].parse().expect("a temp");
+                    add(format!("(Switch {})", temps[disc]));
+                }
+                // A block's last line closes the nodes above it too.
+                "Return" if fields[1].trim_end_matches(')') == "_" => add("(Return _)".to_string()),
+                "Return" => add("(Return a value)".to_string()),
+                "Call" if externs.contains(&rest.split('"').nth(1).unwrap_or("")) => {
+                    add("(Call an extern)".to_string())
+                }
+                _ => {}
+            }
+        }
+    }
+    for line in ll.lines() {
+        if let Some((_, call)) = line.split_once("call ") {
+            if let Some(symbol) = call.split_once(" @nova_rt_").map(|(_, s)| s) {
+                // `nova_rt_str_new` is `ConstStr`'s, which no `RtFunc` names.
+                let symbol = symbol.split('(').next().unwrap_or("");
+                if symbol != "str_new" {
+                    add(format!("call @nova_rt_{symbol}"));
+                }
+            }
+        } else if line.starts_with("declare ") && line.contains(" @\"") {
+            add("declare an extern".to_string());
+        } else if let Some((_, text)) = line
+            .strip_prefix("@str")
+            .and_then(|l| l.split_once(" x i8] c\""))
+        {
+            let text = text
+                .strip_suffix("\", align 1")
+                .unwrap_or_else(|| panic!("a string global: {line}"));
+            if text == "\\00" && line.contains(" constant [1 x i8] ") {
+                add("c\"\\00\"".to_string());
+            }
+            for escape in text.split('\\').skip(1) {
+                let byte = u8::from_str_radix(&escape[..2], 16).expect("two hex digits");
+                add(match byte {
+                    b'"' => "\\22".to_string(),
+                    b'\\' => "\\5C".to_string(),
+                    0..=0x1f | 0x7f => "a control byte".to_string(),
+                    0x80..=0xff => "a byte past ASCII".to_string(),
+                    _ => panic!("{line}: \\{escape} escapes a printable byte"),
+                });
+            }
+        }
+    }
+}
+
+/// Every form S0's emitter handles occurs in the `.ll` compared on the
+/// native corpus's roots that run and the cases (5.5a, spec §6.3, one run
+/// with [`the_mir_dumps_agree_on_the_corpus_and_the_cases`]), read from S0's
+/// output of the roots that printed one: their MIR after form, which
+/// determines their `.ll`, and the `.ll` itself ([`ll_forms_of`]); or it is
+/// in [`LL_UNREACHABLE`] with its reason, and absent. Every `.ll` declares
+/// the same runtime functions, so every symbol and signature is compared on
+/// every input, whatever the calls.
+#[test]
+fn every_form_the_emitter_handles_occurs_in_the_compared_ll() {
+    let (loose, packages, cases) = std::thread::scope(|scope| {
+        let cases = scope.spawn(mir_cases);
+        let runs = vec![scope.spawn(loose_mir), scope.spawn(package_mir)];
+        let mut runs = joined(runs);
+        let packages = runs.pop().expect("the packages'");
+        (
+            runs.pop().expect("the loose programs'"),
+            packages,
+            joined(vec![cases]).remove(0),
+        )
+    });
+    let mut found = std::collections::BTreeMap::new();
+    let mut runtime: Option<Vec<String>> = None;
+    let (mut roots, mut errors) = (0, 0);
+    for (mir, ll) in [
+        (&loose.0, &loose.2),
+        (&packages.0, &packages.2),
+        (&cases.after, &cases.ll),
+    ] {
+        let (mir, ll) = (parts_by_root(mir), parts_by_root(ll));
+        assert_eq!(mir.len(), ll.len());
+        for ((root, mir), (_, ll)) in mir.iter().zip(&ll) {
+            if ll.is_empty() {
+                continue;
+            }
+            roots += 1;
+            if ll.starts_with("(Ll ok)\n") {
+                let declared = ll_runtime(ll);
+                match &runtime {
+                    Some(first) => assert_eq!(&declared, first, "{root}"),
+                    None => runtime = Some(declared),
+                }
+            } else {
+                errors += 1;
+            }
+            ll_forms_of(mir, ll, &mut found);
+        }
+    }
+    let runtime = runtime.expect("a .ll");
+    assert!(runtime.len() >= 100, "{} runtime functions", runtime.len());
+    // The `.ll`'s size by set, N's root apart from the packages'.
+    let size = |parts: &[(String, String)]| {
+        let lines: usize = parts.iter().map(|(_, ll)| ll.lines().count()).sum();
+        let bytes: usize = parts.iter().map(|(_, ll)| ll.len()).sum();
+        format!("{} roots, {lines} lines, {bytes} bytes", parts.len())
+    };
+    let (n_root, packages): (Vec<_>, Vec<_>) = parts_by_root(&packages.2)
+        .into_iter()
+        .filter(|(_, ll)| !ll.is_empty())
+        .partition(|(root, _)| root == "selfhost/src/main.nova");
+    let printed = |ll: &str| -> Vec<(String, String)> {
+        parts_by_root(ll)
+            .into_iter()
+            .filter(|(_, ll)| !ll.is_empty())
+            .collect()
+    };
+    let in_cases = parts_by_root(&cases.ll);
+    eprintln!(
+        "the .ll by set: loose programs {}; packages' mains but N's {}; N's root {}; HIR cases \
+         {}; MIR cases {}",
+        size(&printed(&loose.2)),
+        size(&packages),
+        size(&n_root),
+        size(
+            &in_cases[..cases.hir]
+                .iter()
+                .filter(|(_, l)| !l.is_empty())
+                .cloned()
+                .collect::<Vec<_>>()
+        ),
+        size(
+            &in_cases[cases.hir..]
+                .iter()
+                .filter(|(_, l)| !l.is_empty())
+                .cloned()
+                .collect::<Vec<_>>()
+        ),
+    );
+    assert!(
+        roots >= 600 && errors == 1,
+        "{roots} roots, {errors} errors"
+    );
+    let forms = ll_forms(&runtime);
+    let unlisted: Vec<&String> = found.keys().filter(|f| !forms.contains(f)).collect();
+    assert!(unlisted.is_empty(), "{unlisted:?} are in no list");
+    let unreachable: Vec<&str> = LL_UNREACHABLE.iter().map(|(f, _)| *f).collect();
+    let stray: Vec<&&str> = unreachable
+        .iter()
+        .filter(|f| !forms.iter().any(|g| g == *f))
+        .collect();
+    assert!(
+        stray.is_empty(),
+        "LL_UNREACHABLE names {stray:?}, which is no form"
+    );
+    let missing: Vec<&String> = forms
+        .iter()
+        .filter(|f| !found.contains_key(*f) && !unreachable.contains(&f.as_str()))
+        .collect();
+    assert!(missing.is_empty(), "no compared .ll holds {missing:?}");
+    let reached: Vec<&&str> = unreachable
+        .iter()
+        .filter(|f| found.contains_key(**f))
+        .collect();
+    assert!(
+        reached.is_empty(),
+        "{reached:?} occur, yet LL_UNREACHABLE lists them"
+    );
+    let rare: Vec<String> = found
+        .iter()
+        .filter(|(_, n)| **n < 3)
+        .map(|(f, n)| format!("{f} {n}"))
+        .collect();
+    eprintln!(
+        "the .ll of {roots} roots, {errors} an error: {} of {} forms, {} runtime functions \
+         declared; {} unreachable; seen under three times: {rare:?}",
+        found.len(),
+        forms.len(),
+        runtime.len(),
+        unreachable.len()
+    );
+}
+
 /// MIR's cases and the HIR cases that reach MIR as a Windows checkout has
 /// them ([`crlf`]), in both forms as [`agree_on_mir_forms`] compares them
 /// (spec 5.4 §7.2): each part is there and the codes are those of the
@@ -5916,6 +6308,10 @@ fn agree_on_mir_cases_with_crlf(compared: &mut bool) {
         assert_eq!(codes_in(dumps), codes_in(lf), "{form:?}");
         assert_ne!(dumps, lf, "{form:?}: no place moved");
     }
+    // A string literal's raw line break reaches the `.ll` (5.5a, spec §6.1).
+    let raw = |crlf: bool| format!("c\"first{}\\0Asecond\"", if crlf { "\\0D" } else { "" });
+    assert!(crlf.ll.contains(&raw(true)) && !crlf.ll.contains(&raw(false)));
+    assert!(lf.ll.contains(&raw(false)) && !lf.ll.contains(&raw(true)));
     *compared = true;
 }
 
@@ -5927,6 +6323,14 @@ fn loose_mir() -> &'static (String, String, String) {
     DUMP.get_or_init(|| agree_on_mir_forms(&root(), &loose_corpus(), 0))
 }
 
+/// [`loose_mir`] of the corpus packages' `src/main.nova` ([`mir_corpus`]),
+/// N's own among them: the corpus test's, and the coverage test of the
+/// `.ll` reads it too (5.5a).
+fn package_mir() -> &'static (String, String, String) {
+    static DUMP: OnceLock<(String, String, String)> = OnceLock::new();
+    DUMP.get_or_init(|| agree_on_mir_forms(&root(), &mir_corpus().1, 0))
+}
+
 /// The MIR fuzzer's seeds (5.4, spec §7.4): its own, past the body
 /// fuzzer's.
 const MIR_SEEDS: [u64; 3] = [601, 602, 603];
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__a_nested_match_emits_chains_and_a_flat_one_still_switches_1.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__a_nested_match_emits_chains_and_a_flat_one_still_switches_1.nova
new file mode 100644
index 00000000..091f1a3d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__a_nested_match_emits_chains_and_a_flat_one_still_switches_1.nova
@@ -0,0 +1,3 @@
+type E = | A(Int) | B
+fn f(o: (E, Int)) -> Int { match o { (A(1..=3), _) => 1, (A(_), n) if n > 0 => 2, _ => 3 } }
+fn main() { println("${f((B, 1))}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__a_nested_match_emits_chains_and_a_flat_one_still_switches_2.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__a_nested_match_emits_chains_and_a_flat_one_still_switches_2.nova
new file mode 100644
index 00000000..10e5477b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__a_nested_match_emits_chains_and_a_flat_one_still_switches_2.nova
@@ -0,0 +1,3 @@
+type E = | A(Int) | B
+fn f(e: E) -> Int { match e { A(n) => n, B => 0 } }
+fn main() { println("${f(B)}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__an_async_fn_containing_await_emits_a_resume_dispatch_and_an_indirect_poll.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__an_async_fn_containing_await_emits_a_resume_dispatch_and_an_indirect_poll.nova
new file mode 100644
index 00000000..1f77428e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__an_async_fn_containing_await_emits_a_resume_dispatch_and_an_indirect_poll.nova
@@ -0,0 +1,3 @@
+async fn g() -> Float { 1.5 }
+async fn f() -> Float { g().await + 1.0 }
+fn main() { let x = f() }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__arrays_emit_bounds_check_and_element_addressing.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__arrays_emit_bounds_check_and_element_addressing.nova
new file mode 100644
index 00000000..97998f1d
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__arrays_emit_bounds_check_and_element_addressing.nova
@@ -0,0 +1,5 @@
+fn main() {
+let mut xs = [1, 2, 3]
+ xs[0] = xs[1]
+println("${xs.len()} ${xs[2]}")
+}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__bool_match_switches_on_i8_not_i64.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__bool_match_switches_on_i8_not_i64.nova
new file mode 100644
index 00000000..831b7a8f
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__bool_match_switches_on_i8_not_i64.nova
@@ -0,0 +1,2 @@
+fn classify(b: Bool) -> Int { match b { true => 1, false => 0 } }
+fn main() { println("${classify(true)}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__closures_emit_fat_pointer_and_indirect_call.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__closures_emit_fat_pointer_and_indirect_call.nova
new file mode 100644
index 00000000..30491f45
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__closures_emit_fat_pointer_and_indirect_call.nova
@@ -0,0 +1,5 @@
+fn main() {
+let base = 10
+ let f = |n| n + base
+println("${f(5)}")
+}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__every_rt_func_is_declared_with_its_real_signature.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__every_rt_func_is_declared_with_its_real_signature.nova
new file mode 100644
index 00000000..e71fdf55
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__every_rt_func_is_declared_with_its_real_signature.nova
@@ -0,0 +1 @@
+fn main() {}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__floats_use_hex_constants_and_fp_ops.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__floats_use_hex_constants_and_fp_ops.nova
new file mode 100644
index 00000000..f660f389
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__floats_use_hex_constants_and_fp_ops.nova
@@ -0,0 +1,6 @@
+fn main() {
+let a = 3.5
+ let b = 2.0
+ let c = a * b - 1.0
+println("${c > 5.0}")
+}
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__match_lowers_to_switch_with_trap_default.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__match_lowers_to_switch_with_trap_default.nova
new file mode 100644
index 00000000..0f8c28fe
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__match_lowers_to_switch_with_trap_default.nova
@@ -0,0 +1,3 @@
+type Shape = | Circle(Int) | Rect(Int, Int) | Empty
+fn area(s: Shape) -> Int { match s { Circle(r) => r, Rect(w, h) => w, Empty => 0 } }
+fn main() { println("${area(Empty)}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__panic_emits_declaration_and_call.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__panic_emits_declaration_and_call.nova
new file mode 100644
index 00000000..f9c12b93
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__panic_emits_declaration_and_call.nova
@@ -0,0 +1 @@
+fn main() { panic("boom") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__records_emit_field_addressing.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__records_emit_field_addressing.nova
new file mode 100644
index 00000000..4eb13bcc
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__records_emit_field_addressing.nova
@@ -0,0 +1,3 @@
+record Point { x: Int, y: Int }
+fn main() { let p = Point { x: 3, y: 4 }
+ println("${p.x}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__recursion_and_branch_lower.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__recursion_and_branch_lower.nova
new file mode 100644
index 00000000..71728813
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__recursion_and_branch_lower.nova
@@ -0,0 +1,2 @@
+fn fib(n: Int) -> Int { if n <= 1 { n } else { fib(n - 1) + fib(n - 2) } }
+fn main() { println("${fib(5)}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__string_plus_string_lowers_to_one_concat_call.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__string_plus_string_lowers_to_one_concat_call.nova
new file mode 100644
index 00000000..8d466f6a
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__string_plus_string_lowers_to_one_concat_call.nova
@@ -0,0 +1,2 @@
+fn join(a: String, b: String) -> String { a + b }
+fn main() { println(join("x", "y")) }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__sum_match_still_switches_on_i64.nova b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__sum_match_still_switches_on_i64.nova
new file mode 100644
index 00000000..7a023943
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ir_tests__sum_match_still_switches_on_i64.nova
@@ -0,0 +1,3 @@
+type E = | A | B | C
+fn f(e: E) -> Int { match e { A => 0, B => 1, C => 2 } }
+fn main() { println("${f(B)}") }
\ No newline at end of file
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ll_a_float_le_is_an_ordered_compare.nova b/crates/nova-cli/tests/selfhost/cases/mir/ll_a_float_le_is_an_ordered_compare.nova
new file mode 100644
index 00000000..c344c81b
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ll_a_float_le_is_an_ordered_compare.nova
@@ -0,0 +1,5 @@
+fn main() {
+    let a = 1.5
+    let b = 2.5
+    println("${a <= b} ${b <= a}")
+}
diff --git a/crates/nova-cli/tests/selfhost/cases/mir/ll_a_string_of_every_escape_class.nova b/crates/nova-cli/tests/selfhost/cases/mir/ll_a_string_of_every_escape_class.nova
new file mode 100644
index 00000000..df5e532e
--- /dev/null
+++ b/crates/nova-cli/tests/selfhost/cases/mir/ll_a_string_of_every_escape_class.nova
@@ -0,0 +1,10 @@
+fn main() {
+    let quoted = "say \"hi\""
+    let slashed = "a\\b"
+    let control = "tab\there\0nul\r\n"
+    let past = "café ─"
+    let empty = ""
+    let raw = "first
+second"
+    println("${quoted} ${slashed} ${control} ${past} ${empty} ${raw}")
+}
````

### Task 3: N's `.ll` emitter, compared on every input, and the `.ll` mismatch report

Spec §5, §6, §6.4; decisions 4, 5, 8, 12 and 13.

**Files:**
- Create: `selfhost/src/emit_ll.nova`, `selfhost/src/emit_ll_tests.nova`
- Modify: `selfhost/src/mir.nova`, `selfhost/src/main.nova`,
  `crates/nova-cli/tests/selfhost.rs`,
  `crates/nova-cli/tests/selfhost/compare.rs`

- [ ] **Step 1: The tests.** `patch_block t3-tests`:
  - `emit_ll_tests.nova`, N's six `@test`s, imported by `main.nova`:
    `escape_bytes` on each byte class; `float_hex` of 0, `-0.0`, 1.5, a
    NaN, 1e300, 2.5e-308 and `-inf`; a symbol both a function and an
    extern takes the extern's class, a missing callee `Unit`; every
    runtime function's symbol distinct and six declarations exact; a
    small module's whole text; each error's words on a module built by
    hand;
  - `selfhost.rs`: the switch gone (`N_LL`, `as_n_prints`,
    `split_ll_bytes`; decision 12), N's after form `mir --std DIR --ll`
    on every input, compared raw; the refusal test asserting N's usage
    line for each `--ll` shape; the depth test running N with `--ll` on 1
    MB and on 96 KB;
  - `the_ll_mismatch_report_names_the_define_and_block` (decision 5):
    two roots written by hand, the first with two `define`s and the C
    wrapper; one whole report, and the place of seventeen differences:
    the header, a declaration, a blank line, a `define`, `entry:`, a
    slot, a block, `}`, the second function's `define` and label, a
    string global, the C wrapper, the `(Ll` line, the MIR before it, the
    next root's, and N leaving the `.ll` out.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. Then `cargo test --locked --workspace --test selfhost
  --no-run`, and `SH` (4 s). Expected: `23 passed; 44 failed`: N does not
  build, `error[E0001]: cannot find module `emit_ll`` (imported by
  `emit_ll_tests.nova`), so every test that builds N fails, 43; and
  `the_ll_mismatch_report_names_the_define_and_block` fails at its first
  assertion, since `compare.rs` is still the base's, whose walk does not
  enter a `.ll` (`left: "the dumps differ in a.nova, at line 40 of its
  dump, outside any function\n…`, where the right names `line 27 of its
  .ll, define \"g.1\", block bb1`). The 23 that pass run S0 alone or no
  N.
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`emit_ll.nova`** (new, 690 lines), `nova-codegen-llvm/src/lib.rs`
    function for function, S0's names: `compile_ir` (the header first
    into one builder, decision 13), `rt_table` and `RtTable`,
    `rt_func_decl`, `llty`, `escape_bytes`, `float_hex`, `FnEmit` and
    its helpers, `stmt`, `arg_list`, `emit_call`, `binop` (`cmp_pred`)
    and `terminator`; `Result<String, String>`, each error in S0's words;
  - **`mir.nova`:** `RT_FUNC_SIGNATURES`, the 111 symbols and signatures
    in `RtFunc::ALL`'s order; `rt_func_symbol`, `rt_func_signature`,
    `read_signature`;
  - **`main.nova`:** `selfhost mir --std DIR [--before-async | --ll]
    FILE…`, `--ll` once, refused twice, as `--ll=…` or with
    `--before-async`, N's usage line, exit 2; `ll_part` after a root that
    lowered, from the module `mir_of` lowered;
  - **`compare.rs`:** `MirWalk` enters a root's `.ll` at a depth-0 `(Ll
    ok)` and counts its lines; `ll_place` names the `define` (`define_name`)
    and the block, or `outside any define`; the `(Ll` line says so.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --no-run`, then `S` in three runs
    (98, 1 and 162 s). Expected: `2837 1 10`.
  - `cargo test --locked --workspace --test selfhost --no-run`, then `SH`
    (141 s). Expected: `67 passed; 0 failed`.
  - `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 1` (7 s).
    Expected: `test result: ok. 108 passed; 0 failed; 0 trapped; 108
    total`, the base's 102 and the six of Step 1.
  - `git grep -n 'N_LL\|as_n_prints\|split_ll_bytes' crates selfhost`.
    Expected: nothing (decision 12).
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `selfhost: N's .ll emitter, compared on every
      input, and the .ll mismatch report`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 7b51eca5..6557d17c 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -97,12 +97,13 @@
 //! root that lowered followed by its `.ll`, `(Ll ok)` and what `build
 //! --release` writes, or `(Ll error "…")`, wherever the after form runs;
 //! [`split_ll`] cuts the `.ll` off after the comparison, so 5.4's checks
-//! read 5.4's text. Until N prints the `.ll` ([`N_LL`]), N's side runs
-//! without `--ll` and the `.ll` is checked on S0 alone: present exactly
-//! when the root lowered, an error on the float `%` case alone, and every
-//! form the emitter handles occurring in it, or listed unreachable
-//! ([`LL_UNREACHABLE`]). The `.ll` equals `build --release`'s intermediate
-//! on a few programs.
+//! read 5.4's text. N's after form is `selfhost mir --std DIR --ll FILE…`,
+//! and the `.ll` is compared byte for byte with the rest of the dump; on
+//! S0's side it is checked too: present exactly when the root lowered, an
+//! error on the float `%` case alone, and every form the emitter handles
+//! occurring in it, or listed unreachable ([`LL_UNREACHABLE`]). The `.ll`
+//! equals `build --release`'s intermediate on a few programs. A
+//! difference in it is reported by its line, `define` and block.
 //!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
@@ -627,7 +628,7 @@ fn compare_batch(
         (s0.join().expect("S0's thread"), n)
     });
     let (s0, n) = (s0?, n?);
-    let mismatch = first_difference(&as_n_prints(dump, &s0.stdout), &n.stdout);
+    let mismatch = first_difference(&s0.stdout, &n.stdout);
     if mismatch.is_none() && s0.code == Some(code) && n.code == Some(code) {
         return Ok(String::from_utf8_lossy(&s0.stdout).into_owned());
     }
@@ -651,17 +652,6 @@ fn compare_batch(
     Err(text)
 }
 
-/// S0's output of `dump` as N's must be: itself, or, when S0 was asked for
-/// the `.ll` and N was not ([`N_LL`]), without each root's `(Ll` part
-/// ([`split_ll`]), the `.ll` being checked on S0 alone (5.5a, spec §6.2).
-fn as_n_prints<'a>(dump: &Dump, s0: &'a [u8]) -> std::borrow::Cow<'a, [u8]> {
-    if dump.s0.contains(&"--ll") && !dump.n.contains(&"--ll") {
-        std::borrow::Cow::Owned(split_ll_bytes(s0).0)
-    } else {
-        std::borrow::Cow::Borrowed(s0)
-    }
-}
-
 /// Run `files`, relative to `dir`, through S0's and N's `dump`, `batch`
 /// files to a process, and fail with the report of the first difference.
 /// Both sides must exit with `code`. Returns S0's output.
@@ -1879,6 +1869,216 @@ fn the_mir_mismatch_report_names_the_function_block_and_statement() {
     assert!(!text.contains("function \""), "{text}");
 }
 
+/// In a root's `.ll` (5.5a, spec §6.4) the report names the line of the
+/// `.ll`, counted from the line after `(Ll ok)`, and the `define` and the
+/// block it is in: a `define` line its own function, a label its own
+/// block, a line outside any function (the header, a declaration, a
+/// string global, the blank lines between) "outside any define", the C
+/// entry wrapper as `main`; the `(Ll` line itself says so. The MIR part
+/// before the `.ll` is reported as it was, and the next root's part starts
+/// afresh.
+#[test]
+fn the_ll_mismatch_report_names_the_define_and_block() {
+    const MIR_LL: Dump = Dump {
+        s0: &["check", "--dump-mir", "--ll"],
+        n: &["mir", "--std", "std", "--ll"],
+    };
+    let s0 = [
+        "== a.nova",
+        "(Mir runs=true checked=true lowered=true async_lowered=true functions=2 externs=0",
+        "  (Function \"main\" params=0 takes_env=false captures=0 ret=Unit async=false",
+        "    (Temp t0 I64)",
+        "    (Block bb0",
+        "      (Call t0 \"g.1\" ())",
+        "      (Return _)))",
+        "  (Function \"g.1\" params=0 takes_env=false captures=0 ret=I64 async=false",
+        "    (Temp t0 I64)",
+        "    (Block bb0",
+        "      (ConstInt t0 7)",
+        "      (Return t0))))",
+        "(Diagnostics)",
+        "(Ll ok)",
+        "; LLVM IR emitted by the Nova compiler (release backend).",
+        "; Requires LLVM >= 15 (opaque pointers).",
+        "",
+        "declare void @llvm.trap()",
+        "declare void @nova_rt_println(ptr)",
+        "declare ptr @nova_rt_str_new(ptr, i64)",
+        "",
+        "define void @\"nova_main\"() {",
+        "entry:",
+        "  %t0.slot = alloca i64",
+        "  br label %bb0",
+        "bb0:",
+        "  %r0 = call i64 @\"g.1\"()",
+        "  store i64 %r0, ptr %t0.slot",
+        "  ret void",
+        "}",
+        "",
+        "define i64 @\"g.1\"() {",
+        "entry:",
+        "  %t0.slot = alloca i64",
+        "  br label %bb0",
+        "bb0:",
+        "  store i64 7, ptr %t0.slot",
+        "  br label %bb1",
+        "bb1:",
+        "  %r0 = load i64, ptr %t0.slot",
+        "  ret i64 %r0",
+        "}",
+        "",
+        "@str0 = private unnamed_addr constant [2 x i8] c\"a\\0A\", align 1",
+        "",
+        "define i32 @main(i32 %argc, ptr %argv) {",
+        "entry:",
+        "  call void @nova_main()",
+        "  ret i32 0",
+        "}",
+        "== b.nova",
+        "(Mir runs=true checked=true lowered=true async_lowered=true functions=1 externs=0",
+        "  (Function \"main\" params=0 takes_env=false captures=0 ret=Unit async=false",
+        "    (Block bb0",
+        "      (Return _)))",
+        "(Diagnostics)",
+        "(Ll error \"float remainder (%) is not supported by the LLVM backend\")",
+        "",
+    ]
+    .join("\n");
+    let reported = |from: &str, to: &str| {
+        let n = s0.replacen(from, to, 1);
+        assert_ne!(s0, n, "{from} is not in:\n{s0}");
+        let m = first_difference(s0.as_bytes(), n.as_bytes()).expect("a difference");
+        report(
+            &m,
+            &MIR_LL,
+            Path::new("."),
+            Path::new("nova"),
+            Path::new("selfhost"),
+        )
+    };
+    assert_eq!(
+        reported("  ret i64 %r0", "  ret i64 %r1"),
+        "the dumps differ in a.nova, at line 40 of its dump, line 27 of its .ll, \
+         define \"g.1\", block bb1\n\
+         \x20 S0 (nova check --dump-mir --ll):    ret i64 %r0\n\
+         \x20 N  (selfhost mir --std std --ll):   ret i64 %r1\n\
+         reproduce, in .:\n\
+         \x20 nova check --dump-mir --ll a.nova > s0.txt\n\
+         \x20 selfhost mir --std std --ll a.nova > n.txt\n\
+         \x20 diff s0.txt n.txt\n\
+         and add the file to crates/nova-cli/tests/selfhost/cases/ once it is fixed"
+    );
+    for (from, to, at) in [
+        // The header, a declaration and the blank line after them.
+        (
+            "; Requires LLVM >= 15",
+            "; requires LLVM >= 15",
+            "line 2 of its .ll, outside any define",
+        ),
+        (
+            "declare void @nova_rt_println(ptr)",
+            "declare void @nova_rt_println(i64)",
+            "line 5 of its .ll, outside any define",
+        ),
+        (
+            "i64)\n\ndefine void",
+            "i64)\n \ndefine void",
+            "line 7 of its .ll, outside any define",
+        ),
+        // A `define` line, its entry block, a slot and its first MIR block.
+        (
+            "define void @\"nova_main\"() {",
+            "define void @\"nova_main\"(ptr %p0) {",
+            "line 8 of its .ll, define \"nova_main\"",
+        ),
+        (
+            "{\nentry:\n  %t0.slot = alloca i64\n  br label %bb0\nbb0:\n  %r0",
+            "{\nentry0:\n  %t0.slot = alloca i64\n  br label %bb0\nbb0:\n  %r0",
+            "line 9 of its .ll, define \"nova_main\", block entry",
+        ),
+        (
+            "  %t0.slot = alloca i64",
+            "  %t0.slot = alloca ptr",
+            "line 10 of its .ll, define \"nova_main\", block entry",
+        ),
+        (
+            "  store i64 %r0, ptr %t0.slot",
+            "  store i64 %r1, ptr %t0.slot",
+            "line 14 of its .ll, define \"nova_main\", block bb0",
+        ),
+        (
+            "  ret void\n}",
+            "  ret void\n  unreachable\n}",
+            "line 16 of its .ll, define \"nova_main\", block bb0",
+        ),
+        // The second function: its `define`, a block's label, its blocks.
+        (
+            "define i64 @\"g.1\"() {",
+            "define i64 @\"g.2\"() {",
+            "line 18 of its .ll, define \"g.1\"",
+        ),
+        (
+            "  store i64 7, ptr %t0.slot",
+            "  store i64 8, ptr %t0.slot",
+            "line 23 of its .ll, define \"g.1\", block bb0",
+        ),
+        (
+            "bb1:\n",
+            "bb2:\n",
+            "line 25 of its .ll, define \"g.1\", block bb1",
+        ),
+        // A string global, and the C entry wrapper.
+        (
+            "c\"a\\0A\"",
+            "c\"a\\0a\"",
+            "line 30 of its .ll, outside any define",
+        ),
+        (
+            "  call void @nova_main()",
+            "  call void @\"nova_main\"()",
+            "line 34 of its .ll, define main, block entry",
+        ),
+        // The `(Ll` line, and the MIR before it as it was.
+        (
+            "(Ll ok)",
+            "(Ll error \"no `main` function in MIR module\")",
+            "at line 13 of its dump, at its (Ll line",
+        ),
+        (
+            "(ConstInt t0 7)",
+            "(ConstInt t0 8)",
+            "at line 10 of its dump, function \"g.1\", block bb0, statement 0",
+        ),
+        // The next root's part, with no `.ll` walked.
+        (
+            "(Ll error \"float remainder",
+            "(Ll error \"Float remainder",
+            "b.nova, at line 6 of its dump, at its (Ll line",
+        ),
+        (
+            "async=false\n    (Block bb0\n      (Return _)))",
+            "async=false\n    (Block bb0\n      (Return t0)))",
+            "b.nova, at line 4 of its dump, function \"main\", block bb0, statement 0",
+        ),
+    ] {
+        let text = reported(from, to);
+        let first = text.lines().next().unwrap_or("");
+        assert!(
+            first.starts_with("the dumps differ in ") && first.ends_with(at),
+            "{at}: {text}"
+        );
+    }
+    // N leaves the `.ll` out: the report is at S0's `(Ll` line.
+    let text = reported(
+        "(Diagnostics)\n(Ll error",
+        "(Diagnostics)\n== c.nova\n(Ll error",
+    );
+    assert!(
+        text.starts_with("the dumps differ in b.nova, at line 6 of its dump, at its (Ll line\n"),
+        "{text}"
+    );
+}
+
 // --- 5.1b: the AST dump, the depth limit and S0's thread (ADR 0052) ---
 
 #[test]
@@ -4971,12 +5171,6 @@ const MIR_S0: &[&str] = &["check", "--dump-mir"];
 /// after form wherever the harness runs it.
 const MIR_LL_S0: &[&str] = &["check", "--dump-mir", "--ll"];
 
-/// Whether N prints the `.ll`, `selfhost mir --ll` (5.5a, spec §5.1): not
-/// yet, so N's after form runs without `--ll`, S0's is compared without its
-/// `(Ll` parts ([`as_n_prints`]), and the `.ll` is checked on S0 alone
-/// ([`check_ll`], [`every_form_the_emitter_handles_occurs_in_the_compared_ll`]).
-const N_LL: bool = false;
-
 /// The words of the one emitter error a program reaches, float `%`, which
 /// type-checks and which the emitter refuses (5.5a, spec §2 item 5).
 const FLOAT_REMAINDER_WORDS: &str = "float remainder (%) is not supported by the LLVM backend";
@@ -4988,8 +5182,8 @@ const FLOAT_REMAINDER_CASE: &str = "ll_a_float_remainder_errs.nova";
 /// --before-async`.
 const MIR_BEFORE_S0: &[&str] = &["check", "--dump-mir", "--before-async"];
 
-/// N's MIR dump, `selfhost mir --std DIR [--before-async]` (spec 5.4
-/// §6.3).
+/// N's MIR dump, `selfhost mir --std DIR [--before-async | --ll]` (spec
+/// 5.4 §6.3, 5.5a §5.1).
 const MIR_N: &[&str] = &["mir"];
 
 /// The MIR dump's two forms (spec 5.4 §3.1, §7.2): after the async
@@ -5012,9 +5206,8 @@ impl MirForm {
 }
 
 /// The MIR dump through both sides in `form`: S0's `check --dump-mir`,
-/// and N's `mir --std DIR`, each with `--before-async` for the before form,
-/// S0's with `--ll` for the after form, and N's too once it prints the
-/// `.ll` ([`N_LL`]), followed by the roots.
+/// and N's `mir --std DIR`, each with `--before-async` for the before form
+/// and with `--ll` for the after form, followed by the roots.
 fn mir_in(form: MirForm) -> &'static Dump {
     static DUMPS: [OnceLock<Dump>; 2] = [OnceLock::new(), OnceLock::new()];
     DUMPS[form as usize].get_or_init(|| {
@@ -5023,7 +5216,7 @@ fn mir_in(form: MirForm) -> &'static Dump {
         if form == MirForm::Before {
             n.push("--before-async");
         }
-        if form == MirForm::After && N_LL {
+        if form == MirForm::After {
             n.push("--ll");
         }
         Dump {
@@ -5106,37 +5299,28 @@ fn mir_dump_in(form: MirForm, with_n: bool, dir: &Path, files: &[String], code:
 /// header or the end: no line of 5.4's grammar starts `(Ll`, and no `.ll`
 /// line starts `(` or `== ` (spec §4).
 fn split_ll(dump: &str) -> (String, String) {
-    let (mir, ll) = split_ll_bytes(dump.as_bytes());
-    (
-        String::from_utf8(mir).expect("the MIR dump is UTF-8"),
-        String::from_utf8(ll).expect("the .ll is UTF-8"),
-    )
-}
-
-/// [`split_ll`] on bytes, as the sides' outputs are compared.
-fn split_ll_bytes(dump: &[u8]) -> (Vec<u8>, Vec<u8>) {
-    let (mut mir, mut ll) = (Vec::with_capacity(dump.len()), Vec::new());
+    let (mut mir, mut ll) = (String::with_capacity(dump.len()), String::new());
     let mut in_ll = false;
-    for line in dump.split_inclusive(|&b| b == b'\n') {
-        if line.starts_with(b"== ") {
+    for line in dump.split_inclusive('\n') {
+        if line.starts_with("== ") {
             in_ll = false;
-            mir.extend_from_slice(line);
-            ll.extend_from_slice(line);
+            mir.push_str(line);
+            ll.push_str(line);
             continue;
         }
-        in_ll |= line.starts_with(b"(Ll ");
+        in_ll |= line.starts_with("(Ll ");
         if in_ll {
-            ll.extend_from_slice(line);
+            ll.push_str(line);
         } else {
-            mir.extend_from_slice(line);
+            mir.push_str(line);
         }
     }
     (mir, ll)
 }
 
-/// The `.ll` parts of S0's after form on S0 alone (5.5a, spec §4, §6.2):
-/// `mir` and `ll` as [`split_ll`] gives them, the same roots in the same
-/// order. A root's `(Ll` part is there exactly when its `(Mir` line says
+/// The `.ll` parts of S0's after form (5.5a, spec §4, §6.2), which N's
+/// equal: `mir` and `ll` as [`split_ll`] gives them, the same roots in the
+/// same order. A root's `(Ll` part is there exactly when its `(Mir` line says
 /// `lowered=true`; it is `(Ll ok)` and the `.ll`, ending in a line break,
 /// or one line `(Ll error "…")`, which only [`FLOAT_REMAINDER_CASE`] gives,
 /// with [`FLOAT_REMAINDER_WORDS`].
@@ -5540,8 +5724,8 @@ fn every_run_of_a_mir_dump_settles_e0004s_paths_on_windows() {
 /// spec §3.1, §6.3): no root, `--target` either way, `--test`,
 /// `--before-async` twice and std's part (S0's `--std`, N's `--std-part`)
 /// exit 2 with nothing on stdout; since 5.5a, `--ll` twice, with a value,
-/// and with `--before-async` in either order too (spec 5.5a §6.2), which N
-/// refuses today as any option it does not take.
+/// and with `--before-async` in either order too (spec 5.5a §6.2), N
+/// printing its usage line for each.
 #[test]
 fn both_sides_refuse_the_mir_dumps_shapes_alike() {
     let file = "tests/runtime/array_helpers.nova";
@@ -5560,6 +5744,12 @@ fn both_sides_refuse_the_mir_dumps_shapes_alike() {
         };
         let out = compare_batch(&dump, &root(), &[], 2, &nova(), selfhost());
         assert_eq!(out, Ok(String::new()), "{extra:?}");
+        let refused = run(selfhost(), dump.n, &root(), &[]).expect("run N");
+        assert_eq!(
+            refused.stderr.replace("\r\n", "\n"),
+            "usage: selfhost mir --std DIR [--before-async | --ll] FILE...\n",
+            "{extra:?}"
+        );
     }
     for form in [MirForm::After, MirForm::Before] {
         let n = mir_in(form).n;
@@ -6492,8 +6682,8 @@ fn the_mir_fuzzers_inputs_agree_and_hold_each_kind() {
     );
 }
 
-/// N's MIR on its own stack (5.4, spec §7.5, ADR 0057): N's `selfhost mir`
-/// on a main thread of 1 MB, as
+/// N's MIR on its own stack (5.4, spec §7.5, ADR 0057): N's `selfhost mir`,
+/// since 5.5a with `--ll` (spec 5.5a §6.1), on a main thread of 1 MB, as
 /// [`n_checks_128_levels_of_every_kind_on_a_1_mb_main_thread`] runs N's
 /// type checker, its dump S0's, which S0 makes on its 64 MB thread. The
 /// type checker's deep bodies, every kind of [`fuzz::BODY_DEEP_KINDS`] at
@@ -6503,7 +6693,7 @@ fn the_mir_fuzzers_inputs_agree_and_hold_each_kind() {
 /// [`fuzz::MIR_DEEP_TYPES`], 128 levels deep, as an instance's type
 /// argument ([`fuzz::mir_deep_type`]), so that `mangle_ty` and the
 /// substitution walk it. Each lowers clean; none makes a poll function, so
-/// the after form is the only one to compare.
+/// the after form, its `.ll` included, is the only one to compare.
 #[test]
 fn n_lowers_the_depth_cases_on_a_1_mb_main_thread() {
     let dir = fresh_dir("deep-n-mir");
@@ -6550,15 +6740,16 @@ fn n_lowers_the_depth_cases_on_a_1_mb_main_thread() {
         };
         assert!(walked, "{root}: {part}");
     }
-    let std_dir = std_dir();
-    let mut args = MIR_N.to_vec();
-    args.extend(["--std", std_dir.as_str()]);
+    let mut args = mir_in(MirForm::After).n.to_vec();
     args.extend(files.iter().map(String::as_str));
     let stack = if cfg!(unix) { Some(1024) } else { None };
     let (code, n) = n_args_on_stack(&dir, stack, &args);
-    assert_eq!(code, Some(0), "N's `selfhost mir` on a {stack:?} KB stack");
-    let s0 = as_n_prints(mir_in(MirForm::After), &s0.stdout);
-    if let Some(m) = first_difference(&s0, &n) {
+    assert_eq!(
+        code,
+        Some(0),
+        "N's `selfhost mir --ll` on a {stack:?} KB stack"
+    );
+    if let Some(m) = first_difference(&s0.stdout, &n) {
         panic!(
             "{}",
             report(&m, mir_in(MirForm::After), &dir, &nova(), selfhost())
@@ -6567,10 +6758,14 @@ fn n_lowers_the_depth_cases_on_a_1_mb_main_thread() {
     // The stack is the one set: `bodies-for-ranges-127.nova`, whose least
     // stack is about 212 KB (ADR 0057 §17), fails on 96 KB.
     if cfg!(unix) {
-        let mut args = MIR_N.to_vec();
-        args.extend(["--std", std_dir.as_str(), "bodies-for-ranges-127.nova"]);
+        let mut args = mir_in(MirForm::After).n.to_vec();
+        args.push("bodies-for-ranges-127.nova");
         let (code, _) = n_args_on_stack(&dir, Some(96), &args);
-        assert_ne!(code, Some(0), "N's `selfhost mir` passed on a 96 KB stack");
+        assert_ne!(
+            code,
+            Some(0),
+            "N's `selfhost mir --ll` passed on a 96 KB stack"
+        );
     }
 }
 
diff --git a/selfhost/src/emit_ll_tests.nova b/selfhost/src/emit_ll_tests.nova
new file mode 100644
index 00000000..bf09e4f2
--- /dev/null
+++ b/selfhost/src/emit_ll_tests.nova
@@ -0,0 +1,200 @@
+// N's own tests of the `.ll` emitter (5.5a, spec
+// docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md
+// §5.1): the escape of each byte class, `float_hex` of `-0.0`, a NaN and
+// numbers with exponents, `ret_of`'s duplicate key and a missing callee,
+// the runtime functions' symbols distinct (nova-mir's `rt_func_tests`)
+// and each declared as its signature says, and each of the emitter's
+// errors in S0's words on a module built by hand. The `.ll` differential
+// compares everything else with S0, and these pin what it may not reach:
+// no input has a symbol both a function and an extern, and only float `%`
+// of the errors is reachable from source.
+//
+// They live in `src/`, imported by `main.nova`, for the reason
+// lexer_tests.nova gives.
+
+import emit_ll
+import hir
+import mir
+
+// A module of one function, `name`, returning nothing, with `temps` and
+// one block of `stmts`; and `externs`.
+fn module_of(name: String, temps: [Int], stmts: [MirStmt], externs: [ExternDecl]) -> MirModule {
+    let f = MirFunction {
+        name: name,
+        params: 0,
+        takes_env: false,
+        capture_count: 0,
+        temps: Vec::from_array(temps),
+        ret: MIR_UNIT,
+        is_async: false,
+        blocks: Vec::from_array([MirBlock { stmts: Vec::from_array(stmts), term: TermReturn(-1) }]),
+    }
+    MirModule { functions: Vec::from_array([f]), externs: Vec::from_array(externs) }
+}
+
+// `main` applying `op` to two floats.
+fn float_bin(op: HirBinOp) -> MirModule {
+    module_of(
+        "main",
+        [MIR_F64, MIR_F64, MIR_F64],
+        [SConstFloat(0, 1.5), SConstFloat(1, 2.0), SBin(2, op, OC_FLOAT, 0, 1)],
+        [],
+    )
+}
+
+fn escaped(text: String) -> String {
+    let mut out = StringBuilder::new()
+    escape_bytes(out, text)
+    out.to_string()
+}
+
+fn ir_of(program: MirModule) -> String {
+    match compile_ir(program) {
+        Ok(text) => text
+        Err(e) => panic("the emitter refused a module it takes: ${e}")
+    }
+}
+
+@test
+fn escape_bytes_writes_each_byte_class_as_s0_does() {
+    // Printable ASCII is kept, the space and `~` at its two ends.
+    assert_eq(escaped(" az AZ 09 ~"), " az AZ 09 ~")
+    // `"` and `\`, two uppercase digits and no `x`.
+    assert_eq(escaped("say \"hi\" a\\b"), "say \\22hi\\22 a\\5Cb")
+    // Control bytes, NUL and DEL.
+    assert_eq(escaped("\t\n\r"), "\\09\\0A\\0D")
+    assert_eq(
+        escaped(string_from_chars([char_from_int(0).unwrap(), char_from_int(31).unwrap()])),
+        "\\00\\1F",
+    )
+    assert_eq(escaped(string_from_chars([char_from_int(127).unwrap()])), "\\7F")
+    // Each byte past ASCII on its own, uppercase: `é` is C3 A9, `─` E2 94 80.
+    assert_eq(escaped("café ─"), "caf\\C3\\A9 \\E2\\94\\80")
+    assert_eq(escaped(""), "")
+}
+
+@test
+fn float_hex_is_the_bits_in_16_uppercase_digits() {
+    assert_eq(float_hex(0.0), "0x0000000000000000")
+    assert_eq(float_hex(-0.0), "0x8000000000000000")
+    assert_eq(float_hex(1.5), "0x3FF8000000000000")
+    assert_eq(float_hex("NaN".parse_float().unwrap()), "0x7FF8000000000000")
+    assert_eq(float_hex("1e300".parse_float().unwrap()), "0x7E37E43C8800759C")
+    assert_eq(float_hex("2.5e-308".parse_float().unwrap()), "0x0011FA182C40C60D")
+    assert_eq(float_hex("-inf".parse_float().unwrap()), "0xFFF0000000000000")
+}
+
+@test
+fn a_symbol_both_a_function_and_an_extern_returns_the_externs_class() {
+    // `ret_of` takes the functions, then the externs: an extern's class
+    // replaces a function's of the same symbol, as S0's `collect` keeps
+    // the last. `both` is an `I64` function and an `F64` extern.
+    let both = MirFunction {
+        name: "both",
+        params: 0,
+        takes_env: false,
+        capture_count: 0,
+        temps: Vec::from_array([MIR_I64]),
+        ret: MIR_I64,
+        is_async: false,
+        blocks: Vec::from_array(
+            [MirBlock { stmts: Vec::from_array([SConstInt(0, 7)]), term: TermReturn(0) }],
+        ),
+    }
+    let main = module_of(
+        "main",
+        [MIR_F64, MIR_UNIT],
+        [SCall(0, "both", Vec::new()), SCall(-1, "nowhere", Vec::new())],
+        [ExternDecl { symbol: "both", abi: ExternAbiC, params: Vec::new(), ret: MIR_F64 }],
+    )
+    let program = MirModule {
+        functions: Vec::from_array([main.functions.at(0), both]),
+        externs: main.externs,
+    }
+    let text = ir_of(program)
+    assert(
+        text.contains("\n  %r0 = call double @\"both\"()\n  store double %r0, ptr %t0.slot\n"),
+        text,
+    )
+    // A callee `ret_of` lacks is `Unit`: a call with no result.
+    assert(text.contains("\n  call void @\"nowhere\"()\n  ret void\n}\n"), text)
+    assert(text.contains("\ndeclare double @\"both\"()\n\n"), text)
+    assert(text.contains("\ndefine i64 @\"both\"() {\nentry:\n  %t0.slot = alloca i64\n"), text)
+}
+
+@test
+fn every_runtime_function_has_a_distinct_symbol_and_its_signatures_declaration() {
+    let count = RT_FUNC_VARIANTS.len()
+    assert(
+        RT_FUNC_SIGNATURES.len() == count,
+        "${RT_FUNC_SIGNATURES.len()} signatures for ${count} variants",
+    )
+    let mut seen: Map<String, Int> = Map::new()
+    for f in 0..count {
+        let symbol = rt_func_symbol(f)
+        match seen.get(symbol) {
+            Some(g) => panic(
+                "`${RT_FUNC_VARIANTS[g]}` and `${RT_FUNC_VARIANTS[f]}` are both `${symbol}`",
+            )
+            None => ()
+        }
+        let _ = seen.insert(symbol, f)
+        assert(symbol.starts_with("nova_rt_"), symbol)
+        // Every signature reads, and its declaration spells it.
+        let _ = rt_func_signature(f)
+    }
+    assert_eq(rt_func_decl(rt_func("Println")), "declare void @nova_rt_println(ptr)")
+    assert_eq(rt_func_decl(rt_func("TestSelector")), "declare i64 @nova_rt_test_selector()")
+    assert_eq(rt_func_decl(rt_func("FloatFixed")), "declare ptr @nova_rt_float_fixed(double, i64)")
+    assert_eq(
+        rt_func_decl(rt_func("FileOpen")),
+        "declare i64 @nova_rt_file_open(ptr, i8, i8, i8, i8, i8, i8)",
+    )
+    assert_eq(rt_func_decl(rt_func("NetConnect")), "declare ptr @nova_rt_net_connect_future(ptr)")
+    assert_eq(
+        rt_func_decl(rt_func("RouterStateStore")),
+        "declare void @nova_rt_router_state_store(ptr)",
+    )
+}
+
+@test
+fn a_small_module_is_the_header_its_function_and_the_c_main() {
+    let text = ir_of(module_of("main", [MIR_UNIT], [], []))
+    assert(
+        text.starts_with(
+            "; LLVM IR emitted by the Nova compiler (release backend).\n; Requires LLVM >= 15 (opaque pointers).\n\ndeclare void @llvm.trap()\ndeclare void @nova_rt_println(ptr)\n",
+        ),
+        text,
+    )
+    assert(
+        text.ends_with(
+            "declare ptr @nova_rt_str_new(ptr, i64)\n\ndefine void @\"nova_main\"() {\nentry:\n  br label %bb0\nbb0:\n  ret void\n}\n\n\ndefine i32 @main(i32 %argc, ptr %argv) {\nentry:\n  call void @nova_main()\n  ret i32 0\n}\n",
+        ),
+        text,
+    )
+}
+
+@test
+fn each_error_of_the_emitter_is_in_s0s_words() {
+    let awaits = module_of("main", [MIR_PTR], [SAwait(-1, 0)], [])
+    let cases = [
+        (
+            awaits,
+            "an `.await` marker reached codegen: `nova-mir`'s async state-machine transform must split every `async fn` body at its await points before a module is emitted",
+        ),
+        (float_bin(OpRem), "float remainder (%) is not supported by the LLVM backend"),
+        (float_bin(OpBitAnd), "bitwise operators are not defined for Float"),
+        (float_bin(OpShl), "bitwise operators are not defined for Float"),
+        (module_of("helper", [MIR_UNIT], [], []), "no `main` function in MIR module"),
+    ]
+    for (program, words) in cases.iter() {
+        match compile_ir(program) {
+            Ok(_) => panic("the emitter took a module it refuses: ${words}")
+            Err(e) => assert_eq(e, words)
+        }
+    }
+    // The float operators it takes.
+    let text = ir_of(float_bin(OpAdd))
+    assert(text.contains("\n  store double 0x3FF8000000000000, ptr %t0.slot\n"), text)
+    assert(text.contains("\n  %r2 = fadd double %r0, %r1\n"), text)
+}
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index 9ea5b7c2..e53aa814 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -55,6 +55,7 @@ import async_lower
 import builtins
 import diagnostic
 import dump
+import emit_ll_tests
 import fixes
 import hir
 import hir_dump
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/crates/nova-cli/tests/selfhost/compare.rs b/crates/nova-cli/tests/selfhost/compare.rs
index d07d14c3..871b3d9d 100644
--- a/crates/nova-cli/tests/selfhost/compare.rs
+++ b/crates/nova-cli/tests/selfhost/compare.rs
@@ -1,7 +1,8 @@
 //! The comparison of S0's and N's dumps, and the report of the first place
 //! they differ: the file, the line of its dump and of its source (in a MIR
-//! dump, which has no spans, its function, block and statement), both
-//! sides' lines, and how to reproduce it.
+//! dump, which has no spans, its function, block and statement; in a
+//! root's `.ll`, its line there, `define` and block), both sides' lines,
+//! and how to reproduce it.
 
 use std::path::Path;
 
@@ -122,7 +123,12 @@ pub fn first_difference(s0: &[u8], n: &[u8]) -> Option<Mismatch> {
 /// depth 0 the root's `(Mir` line and `(Diagnostics`; at depth 1 a
 /// `(Function` line, or an `(Extern` or a diagnostic's `(Error`; at depth 2
 /// a function's `(Temp` and `(Block` lines; at depth 3 a block's
-/// statements, then its terminator.
+/// statements, then its terminator. Since 5.5a (spec
+/// `docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`
+/// §6.4), a depth-0 line `(Ll ok)` enters the root's `.ll`, which runs to
+/// the next header, its lines counted from 1: a `define` line enters a
+/// function, a label (`entry:`, `bb<n>:`) a block, and `}` leaves the
+/// function.
 #[derive(Clone, Default)]
 struct MirWalk {
     /// Whether the part is a MIR root's: its first line starts `(Mir `.
@@ -134,17 +140,45 @@ struct MirWalk {
     block: Option<String>,
     /// The block's statement lines walked, its terminator counted as one.
     statements: usize,
+    /// In the root's `.ll`, the lines of it walked; `None` before it.
+    ll_lines: Option<usize>,
+    /// The `.ll` function the last line walked is in, as its `define`
+    /// names it ([`define_name`]); `None` outside any.
+    define: Option<String>,
+    /// The `.ll` block the last line walked is in, its label without the
+    /// colon; `None` outside any.
+    label: Option<String>,
 }
 
 impl MirWalk {
     /// Walk `line`, line `dump_line` of the root's part.
     fn step(&mut self, dump_line: usize, line: &[u8]) {
         if dump_line == 1 {
-            self.mir = line.starts_with(b"(Mir ");
+            *self = MirWalk {
+                mir: line.starts_with(b"(Mir "),
+                ..MirWalk::default()
+            };
         }
         if !self.mir {
             return;
         }
+        if let Some(lines) = self.ll_lines {
+            self.ll_lines = Some(lines + 1);
+            let (define, label) = self.ll_place(line);
+            self.define = define;
+            self.label = label;
+            if line.strip_suffix(b"\n").unwrap_or(line) == b"}" {
+                self.define = None;
+                self.label = None;
+            }
+            return;
+        }
+        if line.strip_suffix(b"\n").unwrap_or(line) == b"(Ll ok)" {
+            self.ll_lines = Some(0);
+            self.function = None;
+            self.block = None;
+            return;
+        }
         let line = text(line);
         let line = line.trim_end_matches('\n');
         let node = line.trim_start_matches(' ');
@@ -182,7 +216,11 @@ impl MirWalk {
     /// - a block's header: the function and that block;
     /// - a statement or a terminator: the function, the block, and its
     ///   index in the block from 0, as `Block::stmts` counts, the
-    ///   terminator after the last statement.
+    ///   terminator after the last statement;
+    /// - the `(Ll` line, `(Ll ok)` or `(Ll error`: "at its (Ll line";
+    /// - a line of the root's `.ll` (5.5a): its line there, and the
+    ///   function and block it is in ([`MirWalk::ll_place`]), or "outside
+    ///   any define".
     ///
     /// `None` for a part that is not a MIR root's, and at a header: the next
     /// root's `== FILE` line, where one side printed a line more than the
@@ -199,6 +237,17 @@ impl MirWalk {
         if !self.mir {
             return None;
         }
+        if let Some(lines) = self.ll_lines {
+            let at = lines + 1;
+            return Some(match self.ll_place(line) {
+                (None, _) => format!("line {at} of its .ll, outside any define"),
+                (Some(f), None) => format!("line {at} of its .ll, define {f}"),
+                (Some(f), Some(b)) => format!("line {at} of its .ll, define {f}, block {b}"),
+            });
+        }
+        if line.starts_with(b"(Ll ") {
+            return Some("at its (Ll line".to_string());
+        }
         let mut at = self.clone();
         at.step(dump_line, line);
         let depth = line.iter().take_while(|&&b| b == b' ').count() / 2;
@@ -212,6 +261,35 @@ impl MirWalk {
             }
         })
     }
+
+    /// The `.ll` function and block `line` of the root's `.ll` is in, after
+    /// the lines walked: a `define` line's own function and no block; a
+    /// label's function and its own block; any other line's the walk's.
+    fn ll_place(&self, line: &[u8]) -> (Option<String>, Option<String>) {
+        let line = line.strip_suffix(b"\n").unwrap_or(line);
+        if line.starts_with(b"define ") {
+            return (Some(define_name(&text(line))), None);
+        }
+        if self.define.is_some() && !line.starts_with(b" ") && line.ends_with(b":") {
+            let label = text(&line[..line.len() - 1]);
+            return (self.define.clone(), Some(label));
+        }
+        (self.define.clone(), self.label.clone())
+    }
+}
+
+/// The function a `.ll` `define` line names: its symbol as it is quoted,
+/// `"g.1"` for `@"g.1"`, or as it is, `main` for the C entry wrapper's
+/// `@main`.
+fn define_name(line: &str) -> String {
+    let symbol = line.split_once('@').map_or("", |(_, rest)| rest);
+    match symbol
+        .strip_prefix('"')
+        .and_then(|rest| rest.split_once('"'))
+    {
+        Some((quoted, _)) => format!("\"{quoted}\""),
+        None => symbol.split('(').next().unwrap_or(symbol).to_string(),
+    }
 }
 
 /// `bytes` as text, each run that is not UTF-8 as a replacement character.
diff --git a/selfhost/src/emit_ll.nova b/selfhost/src/emit_ll.nova
new file mode 100644
index 00000000..4493a23d
--- /dev/null
+++ b/selfhost/src/emit_ll.nova
@@ -0,0 +1,690 @@
+// The `.ll` emitter: a port of crates/nova-codegen-llvm/src/lib.rs, the
+// release backend's textual LLVM IR from a monomorphized native MIR
+// module, function by function and with the same names (5.5a, spec
+// docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md
+// §5): `compile_ir`, `rt_func_decl`, `llty`, `escape_bytes`, `float_hex`,
+// `FnEmit` and its helpers, `stmt`, `arg_list`, `emit_call`, `binop` and
+// `terminator`. S0's module comment gives the layout: 8-byte slots, the
+// heap layouts of sums, records, arrays and closures, the env-first
+// indirect call, every temp an entry-block `alloca`.
+//
+// The Rust's idioms, beyond mir.nova's (ADR 0057):
+//
+// - The text is a `StringBuilder`'s parts, joined once. S0 builds the
+//   functions' text first and the header after; the header reads nothing
+//   the functions make, so here it comes first, into the same builder.
+// - `\XX` escapes are written by bytes (`byte_at`), two uppercase hex
+//   digits and no `x`, for every byte outside 0x20 to 0x7E and for `"` and
+//   `\`; dump.nova's `push_escaped` is another rule. A float is `0x` and
+//   the 16 uppercase hex digits of its bits; a string's length is its
+//   byte length.
+// - `ret_of` is a `Map` read by lookup alone, the functions inserted
+//   before the externs, so an extern's class replaces a function's of the
+//   same symbol, as S0's `collect` keeps the last; a callee it lacks is
+//   `Unit`. `string_index` is a `Map` read by lookup alone, the strings
+//   in a `Vec` by first use. No map's order reaches the text.
+// - A class is its `MirTy` code and a runtime function its index
+//   (mir.nova): `llty` maps a code; each runtime function's declaration,
+//   symbol and return class are read once per module (`RtTable`), since
+//   reading a `const` builds it anew.
+// - `compile_ir` is `Result<String, String>`, each error in S0's words:
+//   no `main`, an `Await`, a float `%` and a float bitwise operator. S0's
+//   `Mem` arm has nothing to match: N's MIR has no `Mem` statement.
+
+import hir
+import mir
+
+/// The symbol the MIR `main` is emitted under (`NOVA_ENTRY_SYMBOL`),
+/// leaving `main` for the exported C entry wrapper.
+pub const NOVA_ENTRY_SYMBOL: String = "nova_main"
+
+// The LLVM trap intrinsic `TermTrap` calls (`TRAP_DECL`).
+const TRAP_DECL: String = "declare void @llvm.trap()"
+
+// The string constructor `ConstStr` calls by its raw symbol
+// (`STR_NEW_DECL`), which no runtime function names.
+const STR_NEW_DECL: String = "declare ptr @nova_rt_str_new(ptr, i64)"
+
+// The 16 hex digits, uppercase, by value.
+const HEX_UPPER: String = "0123456789ABCDEF"
+
+// Hex digit `d`, uppercase.
+fn hex_upper(d: Int) -> String { HEX_UPPER.byte_slice(d, d + 1) }
+
+/// Each runtime function's declaration, symbol and return class, by its
+/// index, read once from `RT_FUNC_SIGNATURES`.
+pub record RtTable { pub decls: Vec<String>, pub symbols: Vec<String>, pub rets: Vec<Int> }
+
+/// The table of every runtime function, in `RtFunc::ALL`'s order, read
+/// from `RT_FUNC_SIGNATURES` once.
+pub fn rt_table() -> RtTable {
+    let mut decls: Vec<String> = Vec::new()
+    let mut symbols: Vec<String> = Vec::new()
+    let mut rets: Vec<Int> = Vec::new()
+    let table = RT_FUNC_SIGNATURES
+    for f in 0..table.len() {
+        let (symbol, signature) = table[f]
+        let (params, ret) = read_signature(signature)
+        decls.push(decl_of(symbol, params, ret))
+        symbols.push(symbol)
+        rets.push(ret)
+    }
+    RtTable { decls: decls, symbols: symbols, rets: rets }
+}
+
+/// The LLVM declaration of runtime function `rt` (`rt_func_decl`), from
+/// its signature and symbol, as its calls spell them.
+pub fn rt_func_decl(rt: Int) -> String {
+    let (params, ret) = rt_func_signature(rt)
+    decl_of(rt_func_symbol(rt), params, ret)
+}
+
+// `declare <ret> @<symbol>(<params>)`.
+fn decl_of(symbol: String, params: Vec<Int>, ret: Int) -> String {
+    let mut names: Vec<String> = Vec::new()
+    for i in 0..params.len() { names.push(llty(params.at(i))) }
+    "declare ${llty(ret)} @${symbol}(${", ".join(names.to_array())})"
+}
+
+/// A monomorphized MIR module as a textual LLVM IR module (`compile_ir`).
+pub fn compile_ir(program: MirModule) -> Result<String, String> {
+    let functions = program.functions
+    let externs = program.externs
+    let mut has_main = false
+    for i in 0..functions.len() {
+        if functions.at(i).name == "main" {
+            has_main = true
+            break
+        }
+    }
+    if !has_main { return Err("no `main` function in MIR module") }
+    // Return class of every function, so a call site can spell the call's
+    // result type even when the result is discarded.
+    let mut ret_of: Map<String, Int> = Map::new()
+    for i in 0..functions.len() {
+        let f = functions.at(i)
+        let _ = ret_of.insert(f.name, f.ret)
+    }
+    for i in 0..externs.len() {
+        let e = externs.at(i)
+        let _ = ret_of.insert(e.symbol, e.ret)
+    }
+    let rt = rt_table()
+    let strings: Vec<String> = Vec::new()
+    let string_index: Map<String, Int> = Map::new()
+    let mut out = StringBuilder::new()
+    out.push("; LLVM IR emitted by the Nova compiler (release backend).\n")
+    out.push("; Requires LLVM >= 15 (opaque pointers).\n\n")
+    out.push(TRAP_DECL)
+    out.push("\n")
+    for f in 0..rt.decls.len() {
+        out.push(rt.decls.at(f))
+        out.push("\n")
+    }
+    out.push(STR_NEW_DECL)
+    out.push("\n")
+    // Each extern (FFI) symbol, which the call sites spell `@"symbol"`.
+    for i in 0..externs.len() {
+        let ext = externs.at(i)
+        let mut params: Vec<String> = Vec::new()
+        for k in 0..ext.params.len() { params.push(llty(ext.params.at(k))) }
+        out.push("declare ${llty(ext.ret)} @\"${ext.symbol}\"(${", ".join(params.to_array())})\n")
+    }
+    out.push("\n")
+    for i in 0..functions.len() {
+        let mut fe = FnEmit {
+            out: out,
+            f: functions.at(i),
+            ret_of: ret_of,
+            rt: rt,
+            strings: strings,
+            string_index: string_index,
+            ssa: 0,
+        }
+        fe.emit()?
+        out.push("\n")
+    }
+    // String literal globals.
+    for i in 0..strings.len() {
+        let s = strings.at(i)
+        let n = if s.byte_len() == 0 { 1 } else { s.byte_len() }
+        out.push("@str${i} = private unnamed_addr constant [${n} x i8] c\"")
+        if s.byte_len() == 0 { out.push("\\00") } else { escape_bytes(out, s) }
+        out.push("\", align 1\n")
+    }
+    out.push("\n")
+    // Exported C entry point: `int main(int, char**) { nova_main(); return 0; }`.
+    out.push("define i32 @main(i32 %argc, ptr %argv) {\nentry:\n")
+    out.push("  call void @nova_main()\n  ret i32 0\n}\n")
+    Ok(out.to_string())
+}
+
+/// The LLVM type spelling of a class (`llty`), `void` for unit.
+pub fn llty(ty: Int) -> String {
+    if ty == MIR_I64 {
+        "i64"
+    } else if ty == MIR_F64 {
+        "double"
+    } else if ty == MIR_I8 {
+        "i8"
+    } else if ty == MIR_PTR {
+        "ptr"
+    } else {
+        "void"
+    }
+}
+
+/// The bytes of `text` escaped for an LLVM `c"..."` constant, onto `out`
+/// (`escape_bytes`): printable ASCII but `"` and `\` as it is, every other
+/// byte `\XX`, two uppercase hex digits. The bytes kept are ASCII, so each
+/// run of them is whole characters.
+pub fn escape_bytes(mut out: StringBuilder, text: String) {
+    let n = text.byte_len()
+    let mut run = 0
+    let mut i = 0
+    while i < n {
+        let b = text.byte_at(i)
+        if b < 0x20 || b > 0x7E || b == 34 || b == 92 {
+            if i > run { out.push(text.byte_slice(run, i)) }
+            out.push("\\${hex_upper(b >> 4)}${hex_upper(b & 15)}")
+            run = i + 1
+        }
+        i += 1
+    }
+    if n > run { out.push(text.byte_slice(run, n)) }
+}
+
+/// A `double` constant as its exact IEEE-754 bits, LLVM's hex float form
+/// (`float_hex`): `0x` and 16 uppercase hex digits.
+pub fn float_hex(v: Float) -> String {
+    let bits = v.to_bits()
+    let mut out = StringBuilder::new()
+    out.push("0x")
+    let mut shift = 60
+    while shift >= 0 {
+        out.push(hex_upper((bits >> shift) & 15))
+        shift -= 4
+    }
+    out.to_string()
+}
+
+/// Per-function IR emission state (`FnEmit`): the text, shared by every
+/// function; the function; the module's return classes and runtime
+/// table; the string globals, shared; and the next `%rN`.
+pub record FnEmit {
+    out: StringBuilder,
+    f: MirFunction,
+    ret_of: Map<String, Int>,
+    rt: RtTable,
+    strings: Vec<String>,
+    string_index: Map<String, Int>,
+    ssa: Int,
+}
+
+impl FnEmit {
+    fn line(mut self, s: String) {
+        self.out.push(s)
+        self.out.push("\n")
+    }
+
+    // A fresh, named SSA temporary (`fresh`).
+    fn fresh(mut self) -> String {
+        let n = self.ssa
+        self.ssa += 1
+        "%r${n}"
+    }
+
+    fn temp_ty(self, t: Int) -> Int { self.f.temps.at(t) }
+
+    fn slot(self, t: Int) -> String { "%t${t}.slot" }
+
+    // Load a temp's current value into a fresh SSA name (`load`).
+    fn load(mut self, t: Int) -> String {
+        let ty = llty(self.temp_ty(t))
+        let slot = self.slot(t)
+        let r = self.fresh()
+        self.line("  ${r} = load ${ty}, ptr ${slot}")
+        r
+    }
+
+    // Store a value into a temp's slot, nothing for a unit temp (`store`).
+    fn store(mut self, t: Int, val: String) {
+        if self.temp_ty(t) == MIR_UNIT { return }
+        let ty = llty(self.temp_ty(t))
+        let slot = self.slot(t)
+        self.line("  store ${ty} ${val}, ptr ${slot}")
+    }
+
+    // The number of leading ABI parameter temps, the real parameters and
+    // the environment pointer if any (`abi_count`).
+    fn abi_count(self) -> Int { self.f.params + if self.f.takes_env { 1 } else { 0 } }
+
+    fn intern_string(mut self, s: String) -> Int {
+        match self.string_index.get(s) {
+            Some(i) => return i
+            None => ()
+        }
+        let i = self.strings.len()
+        self.strings.push(s)
+        let _ = self.string_index.insert(s, i)
+        i
+    }
+
+    /// The function's text (`emit`): its `define`, the entry block's slots
+    /// and parameters, then a labelled block per MIR block.
+    pub fn emit(mut self) -> Result<(), String> {
+        let symbol = if self.f.name == "main" { NOVA_ENTRY_SYMBOL } else { self.f.name }
+        let ret = llty(self.f.ret)
+        // Signature: non-unit ABI param temps become named LLVM parameters.
+        let abi = self.abi_count()
+        let mut params = StringBuilder::new()
+        for i in 0..abi {
+            if self.temp_ty(i) == MIR_UNIT { continue }
+            if !params.is_empty() { params.push(", ") }
+            params.push("${llty(self.temp_ty(i))} %p${i}")
+        }
+        self.line("define ${ret} @\"${symbol}\"(${params}) {")
+        // Entry block: a slot per non-unit temp, the parameters stored,
+        // then a jump to the first MIR block, which may be a loop's target.
+        self.line("entry:")
+        let temp_count = self.f.temps.len()
+        for i in 0..temp_count {
+            if self.temp_ty(i) == MIR_UNIT { continue }
+            let ty = llty(self.temp_ty(i))
+            let slot = self.slot(i)
+            self.line("  ${slot} = alloca ${ty}")
+        }
+        for i in 0..abi {
+            if self.temp_ty(i) == MIR_UNIT { continue }
+            let ty = llty(self.temp_ty(i))
+            let slot = self.slot(i)
+            self.line("  store ${ty} %p${i}, ptr ${slot}")
+        }
+        self.line("  br label %bb0")
+        let blocks = self.f.blocks
+        for i in 0..blocks.len() {
+            let block = blocks.at(i)
+            self.line("bb${i}:")
+            for k in 0..block.stmts.len() { self.stmt(block.stmts.at(k))? }
+            self.terminator(block.term)
+        }
+        self.line("}")
+        Ok(())
+    }
+
+    // A byte-offset GEP into a heap object, a fresh address (`gep_byte`).
+    fn gep_byte(mut self, base: String, byte_off: Int) -> String {
+        let r = self.fresh()
+        self.line("  ${r} = getelementptr inbounds i8, ptr ${base}, i64 ${byte_off}")
+        r
+    }
+
+    /// A statement's text (`stmt`).
+    pub fn stmt(mut self, stmt: MirStmt) -> Result<(), String> {
+        match stmt {
+            SConstInt(t, v) => self.store(t, "${v}")
+            SConstFloat(t, v) => {
+                let hex = float_hex(v)
+                self.store(t, hex)
+            }
+            SConstBool(t, v) => self.store(t, if v { "1" } else { "0" })
+            SConstUnit(_) => ()
+            SConstStr(t, s) => {
+                let idx = self.intern_string(s)
+                let n = if s.byte_len() == 0 { 1 } else { s.byte_len() }
+                let p = self.fresh()
+                self.line(
+                    "  ${p} = getelementptr inbounds [${n} x i8], ptr @str${idx}, i64 0, i64 0",
+                )
+                let r = self.fresh()
+                self.line("  ${r} = call ptr @nova_rt_str_new(ptr ${p}, i64 ${s.byte_len()})")
+                self.store(t, r)
+            }
+            SCopy(dst, src) => {
+                if self.temp_ty(src) != MIR_UNIT {
+                    let v = self.load(src)
+                    self.store(dst, v)
+                }
+            }
+            SBin(dst, op, class, lhs, rhs) => {
+                let l = self.load(lhs)
+                let r = self.load(rhs)
+                let opnd = llty(self.temp_ty(lhs))
+                let v = self.binop(op, class, opnd, l, r)?
+                self.store(dst, v)
+            }
+            SNeg(dst, class, src) => {
+                let v = self.load(src)
+                let ty = llty(self.temp_ty(src))
+                let r = self.fresh()
+                if class == OC_FLOAT {
+                    self.line("  ${r} = fneg ${ty} ${v}")
+                } else {
+                    self.line("  ${r} = sub ${ty} 0, ${v}")
+                }
+                self.store(dst, r)
+            }
+            SNot(dst, src) => {
+                let v = self.load(src)
+                let r = self.fresh()
+                self.line("  ${r} = xor i8 ${v}, 1")
+                self.store(dst, r)
+            }
+            SBitNot(dst, src) => {
+                let v = self.load(src)
+                let r = self.fresh()
+                self.line("  ${r} = xor i64 ${v}, -1")
+                self.store(dst, r)
+            }
+            SCall(dst, callee, args) => {
+                let ret = match self.ret_of.get(callee) {
+                    Some(c) => c
+                    None => MIR_UNIT
+                }
+                let arglist = self.arg_list(args)
+                self.emit_call(dst, ret, "@\"${callee}\"", arglist)
+            }
+            // A marker the async transform consumes; no IR expresses it.
+            SAwait(_, _) => return Err(
+                "an `.await` marker reached codegen: `nova-mir`'s async state-machine transform must split every `async fn` body at its await points before a module is emitted",
+            )
+            SCallRuntime(dst, func, args) => {
+                let ret = self.rt.rets.at(func)
+                let arglist = self.arg_list(args)
+                self.emit_call(dst, ret, "@${self.rt.symbols.at(func)}", arglist)
+            }
+            SCallIndirect(dst, callee, _, ret, args) => {
+                // `callee` is a fat pointer `{ code_ptr, env_ptr }`: call
+                // `code_ptr(env_ptr, args...)`, env first.
+                let fat = self.load(callee)
+                let code = self.fresh()
+                self.line("  ${code} = load ptr, ptr ${fat}")
+                let env_addr = self.gep_byte(fat, 8)
+                let env = self.fresh()
+                self.line("  ${env} = load ptr, ptr ${env_addr}")
+                let mut arglist = StringBuilder::new()
+                arglist.push("ptr ${env}")
+                for i in 0..args.len() {
+                    let a = args.at(i)
+                    if self.temp_ty(a) == MIR_UNIT { continue }
+                    let ty = llty(self.temp_ty(a))
+                    let v = self.load(a)
+                    arglist.push(", ${ty} ${v}")
+                }
+                self.emit_call(dst, ret, code, arglist.to_string())
+            }
+            SMakeClosure(dst, code, captures) => {
+                let env = if captures.is_empty() {
+                    "null"
+                } else {
+                    let size = 8 * captures.len()
+                    let env = self.fresh()
+                    self.line("  ${env} = call ptr @nova_rt_alloc(i64 ${size})")
+                    for i in 0..captures.len() {
+                        let (field, ty) = captures.at(i)
+                        if ty == MIR_UNIT { continue }
+                        let v = self.load(field)
+                        let addr = self.gep_byte(env, 8 * i)
+                        self.line("  store ${llty(ty)} ${v}, ptr ${addr}")
+                    }
+                    env
+                }
+                let fat = self.fresh()
+                self.line("  ${fat} = call ptr @nova_rt_alloc(i64 16)")
+                self.line("  store ptr @\"${code}\", ptr ${fat}")
+                let env_addr = self.gep_byte(fat, 8)
+                self.line("  store ptr ${env}, ptr ${env_addr}")
+                self.store(dst, fat)
+            }
+            SMakeSum(dst, tag, fields) => {
+                let size = 8 + 8 * fields.len()
+                let ptr = self.fresh()
+                self.line("  ${ptr} = call ptr @nova_rt_alloc(i64 ${size})")
+                self.line("  store i64 ${tag}, ptr ${ptr}")
+                for i in 0..fields.len() {
+                    let (field, ty) = fields.at(i)
+                    if ty == MIR_UNIT { continue }
+                    let v = self.load(field)
+                    let addr = self.gep_byte(ptr, 8 + 8 * i)
+                    self.line("  store ${llty(ty)} ${v}, ptr ${addr}")
+                }
+                self.store(dst, ptr)
+            }
+            SSumTag(dst, sum) => {
+                let p = self.load(sum)
+                let r = self.fresh()
+                self.line("  ${r} = load i64, ptr ${p}")
+                self.store(dst, r)
+            }
+            SSumField(dst, sum, index, ty) => {
+                if ty == MIR_UNIT { return Ok(()) }
+                let p = self.load(sum)
+                let addr = self.gep_byte(p, 8 + 8 * index)
+                let r = self.fresh()
+                self.line("  ${r} = load ${llty(ty)}, ptr ${addr}")
+                self.store(dst, r)
+            }
+            SMakeRecord(dst, fields) => {
+                let size = 8 * if fields.len() < 1 { 1 } else { fields.len() }
+                let ptr = self.fresh()
+                self.line("  ${ptr} = call ptr @nova_rt_alloc(i64 ${size})")
+                for i in 0..fields.len() {
+                    let (field, ty) = fields.at(i)
+                    if ty == MIR_UNIT { continue }
+                    let v = self.load(field)
+                    let addr = self.gep_byte(ptr, 8 * i)
+                    self.line("  store ${llty(ty)} ${v}, ptr ${addr}")
+                }
+                self.store(dst, ptr)
+            }
+            SRecordField(dst, rec, index, ty) => {
+                if ty == MIR_UNIT { return Ok(()) }
+                let p = self.load(rec)
+                let addr = self.gep_byte(p, 8 * index)
+                let r = self.fresh()
+                self.line("  ${r} = load ${llty(ty)}, ptr ${addr}")
+                self.store(dst, r)
+            }
+            SSetField(rec, index, value, ty) => {
+                // A unit-typed field has no machine representation to store.
+                if ty == MIR_UNIT { return Ok(()) }
+                let v = self.load(value)
+                let p = self.load(rec)
+                // The same `8 * index` offset `RecordField` loads from.
+                let addr = self.gep_byte(p, 8 * index)
+                self.line("  store ${llty(ty)} ${v}, ptr ${addr}")
+            }
+            SMakeArray(dst, elems) => {
+                let size = 8 + 8 * elems.len()
+                let ptr = self.fresh()
+                self.line("  ${ptr} = call ptr @nova_rt_alloc(i64 ${size})")
+                self.line("  store i64 ${elems.len()}, ptr ${ptr}")
+                for i in 0..elems.len() {
+                    let (field, ty) = elems.at(i)
+                    if ty == MIR_UNIT { continue }
+                    let v = self.load(field)
+                    let addr = self.gep_byte(ptr, 8 + 8 * i)
+                    self.line("  store ${llty(ty)} ${v}, ptr ${addr}")
+                }
+                self.store(dst, ptr)
+            }
+            SArrayAlloc(dst, len) => {
+                // `MakeArray`'s layout with the count known at run time:
+                // `8 + 8*len` bytes, `len` at offset 0.
+                let n = self.load(len)
+                let bytes = self.fresh()
+                self.line("  ${bytes} = mul i64 ${n}, 8")
+                let size = self.fresh()
+                self.line("  ${size} = add i64 ${bytes}, 8")
+                let ptr = self.fresh()
+                self.line("  ${ptr} = call ptr @nova_rt_alloc(i64 ${size})")
+                self.line("  store i64 ${n}, ptr ${ptr}")
+                self.store(dst, ptr)
+            }
+            SArrayLen(dst, arr) => {
+                let p = self.load(arr)
+                let r = self.fresh()
+                self.line("  ${r} = load i64, ptr ${p}")
+                self.store(dst, r)
+            }
+            SArrayGet(dst, arr, index, ty) => {
+                if ty == MIR_UNIT { return Ok(()) }
+                let addr = self.array_elem_addr(arr, index)
+                let r = self.fresh()
+                self.line("  ${r} = load ${llty(ty)}, ptr ${addr}")
+                self.store(dst, r)
+            }
+            SArraySet(arr, index, value, ty) => {
+                if ty == MIR_UNIT { return Ok(()) }
+                let v = self.load(value)
+                let addr = self.array_elem_addr(arr, index)
+                self.line("  store ${llty(ty)} ${v}, ptr ${addr}")
+            }
+        }
+        Ok(())
+    }
+
+    // The address of `arr[index]`, `arr + 8 + index*8` (`array_elem_addr`).
+    fn array_elem_addr(mut self, arr: Int, index: Int) -> String {
+        let base = self.load(arr)
+        let idx = self.load(index)
+        let scaled = self.fresh()
+        self.line("  ${scaled} = mul i64 ${idx}, 8")
+        let off = self.fresh()
+        self.line("  ${off} = add i64 ${scaled}, 8")
+        let addr = self.fresh()
+        self.line("  ${addr} = getelementptr inbounds i8, ptr ${base}, i64 ${off}")
+        addr
+    }
+
+    /// A comma-separated `<ty> <val>` argument list, unit temps skipped
+    /// (`arg_list`).
+    pub fn arg_list(mut self, args: Vec<Int>) -> String {
+        let mut out = StringBuilder::new()
+        for i in 0..args.len() {
+            let a = args.at(i)
+            if self.temp_ty(a) == MIR_UNIT { continue }
+            let ty = llty(self.temp_ty(a))
+            let v = self.load(a)
+            if !out.is_empty() { out.push(", ") }
+            out.push("${ty} ${v}")
+        }
+        out.to_string()
+    }
+
+    /// A call, its result stored into `dst` when the callee returns a
+    /// value; a void callee's is a statement (`emit_call`). `dst` is -1 for
+    /// none.
+    pub fn emit_call(mut self, dst: Int, ret: Int, callee: String, arglist: String) {
+        if ret == MIR_UNIT {
+            self.line("  call void ${callee}(${arglist})")
+        } else {
+            let r = self.fresh()
+            self.line("  ${r} = call ${llty(ret)} ${callee}(${arglist})")
+            if dst >= 0 { self.store(dst, r) }
+        }
+    }
+
+    /// A binary operation's text, its result's SSA name (`binop`): a
+    /// comparison an `i1` widened to the `i8` bool; a float `%` and a float
+    /// bitwise operator S0's errors.
+    pub fn binop(
+        mut self,
+        op: HirBinOp,
+        class: Int,
+        opnd: String,
+        l: String,
+        r: String,
+    ) -> Result<String, String> {
+        match cmp_pred(class, op) {
+            Some(pred) => {
+                let c = self.fresh()
+                self.line("  ${c} = ${pred} ${opnd} ${l}, ${r}")
+                let z = self.fresh()
+                self.line("  ${z} = zext i1 ${c} to i8")
+                return Ok(z)
+            }
+            None => ()
+        }
+        let instr = if class == OC_FLOAT {
+            match op {
+                OpAdd => "fadd"
+                OpSub => "fsub"
+                OpMul => "fmul"
+                OpDiv => "fdiv"
+                OpRem => return Err("float remainder (%) is not supported by the LLVM backend")
+                _ => return Err("bitwise operators are not defined for Float")
+            }
+        } else {
+            match op {
+                OpAdd => "add"
+                OpSub => "sub"
+                OpMul => "mul"
+                OpDiv => "sdiv"
+                OpRem => "srem"
+                OpBitAnd => "and"
+                OpBitOr => "or"
+                OpBitXor => "xor"
+                OpShl => "shl"
+                OpShr => "ashr"
+                _ => panic("handled as comparison above")
+            }
+        }
+        let d = self.fresh()
+        self.line("  ${d} = ${instr} ${opnd} ${l}, ${r}")
+        Ok(d)
+    }
+
+    /// A terminator's text (`terminator`).
+    pub fn terminator(mut self, term: MirTerm) {
+        match term {
+            TermGoto(b) => self.line("  br label %bb${b}")
+            TermBranch(cond, then_, else_) => {
+                let c = self.load(cond)
+                let c1 = self.fresh()
+                self.line("  ${c1} = icmp ne i8 ${c}, 0")
+                self.line("  br i1 ${c1}, label %bb${then_}, label %bb${else_}")
+            }
+            TermSwitch(disc, arms, default) => {
+                // The discriminant's own class: a sum tag's, an `Int`'s or a
+                // `Char`'s is `i64`, a `Bool` match's `i8`; every case
+                // constant shares it.
+                let ty = llty(self.temp_ty(disc))
+                let d = self.load(disc)
+                let mut table = StringBuilder::new()
+                for i in 0..arms.len() {
+                    let (value, block) = arms.at(i)
+                    table.push(" ${ty} ${value}, label %bb${block}")
+                }
+                self.line("  switch ${ty} ${d}, label %bb${default} [${table} ]")
+            }
+            TermReturn(value) => {
+                if value >= 0 && self.f.ret != MIR_UNIT {
+                    let val = self.load(value)
+                    self.line("  ret ${llty(self.f.ret)} ${val}")
+                } else {
+                    self.line("  ret void")
+                }
+            }
+            TermTrap => {
+                self.line("  call void @llvm.trap()")
+                self.line("  unreachable")
+            }
+        }
+    }
+}
+
+// A comparison's predicate (`binop`'s `cmp_pred`): `fcmp` on floats,
+// signed `icmp` on the rest; `None` for an operator that is not one.
+fn cmp_pred(class: Int, op: HirBinOp) -> Option<String> {
+    let float = class == OC_FLOAT
+    match op {
+        OpEq => Some(if float { "fcmp oeq" } else { "icmp eq" })
+        OpNe => Some(if float { "fcmp une" } else { "icmp ne" })
+        OpLt => Some(if float { "fcmp olt" } else { "icmp slt" })
+        OpLe => Some(if float { "fcmp ole" } else { "icmp sle" })
+        OpGt => Some(if float { "fcmp ogt" } else { "icmp sgt" })
+        OpGe => Some(if float { "fcmp oge" } else { "icmp sge" })
+        _ => None
+    }
+}
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index e53aa814..7ab60a42 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -33,17 +33,20 @@
 //                             ADR 0055), then every function of the
 //                             program, its body checked, std's bodies
 //                             checked once (§4.3, §4.5, ADR 0056)
-//   selfhost mir --std DIR [--before-async] FILE...
+//   selfhost mir --std DIR [--before-async | --ll] FILE...
 //                             prints the MIR dump of each program whose
 //                             root is FILE, after the same header, as
-//                             `nova check --dump-mir [--before-async]`
-//                             does: natively, monomorphized from `main`,
-//                             lowered and through the async transform, or
-//                             before it with `--before-async`, std's
-//                             checked functions spliced into each program
-//                             (spec
+//                             `nova check --dump-mir [--before-async |
+//                             --ll]` does: natively, monomorphized from
+//                             `main`, lowered and through the async
+//                             transform, or before it with
+//                             `--before-async`, std's checked functions
+//                             spliced into each program (spec
 //                             docs/superpowers/specs/2026-11-14-phase-5-4-mir-design.md
-//                             §5, §6, ADR 0057)
+//                             §5, §6, ADR 0057); with `--ll`, each
+//                             lowered root's `.ll` after its part (spec
+//                             docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md
+//                             §4, §5)
 //
 // A file with lex or parse errors is dumped with them, and the exit code
 // stays 0. A file that cannot be read prints `IOERR` and its kind after its
@@ -55,6 +58,7 @@ import async_lower
 import builtins
 import diagnostic
 import dump
+import emit_ll
 import emit_ll_tests
 import fixes
 import hir
@@ -272,21 +276,24 @@ async fn dump_command(argv: Vec<String>, command: String) {
     if failed { exit(2) }
 }
 
-// `selfhost mir --std DIR [--before-async] FILE...`: std's modules are
-// read from DIR, parsed, resolved, their items collected and their bodies
-// checked once, then each program's MIR dump after its header, natively,
-// as S0's `check --dump-mir` prints it (spec 5.4 §6.3). Each option is
-// taken once, and `--` before a root that starts with `-`. With no `--std
-// DIR`, with a second `--std` or `--before-async`, `--std-part`,
-// `--target`, `--test` or another option, or with no file, nothing is
-// printed and the exit code is 2, as S0's form refuses its shapes. The
-// dump is after the async transform, or before it with `--before-async`.
+// `selfhost mir --std DIR [--before-async | --ll] FILE...`: std's modules
+// are read from DIR, parsed, resolved, their items collected and their
+// bodies checked once, then each program's MIR dump after its header,
+// natively, as S0's `check --dump-mir` prints it (spec 5.4 §6.3). Each
+// option is taken once, and `--` before a root that starts with `-`. With
+// no `--std DIR`, with a second `--std`, `--before-async` or `--ll`, with
+// both `--before-async` and `--ll`, `--ll=…`, `--std-part`, `--target`,
+// `--test` or another option, or with no file, nothing is printed and the
+// exit code is 2, as S0's form refuses its shapes. The dump is after the
+// async transform, or before it with `--before-async`; with `--ll`, each
+// root that lowered has its `.ll` after its part (spec 5.5a §4).
 // A root whose package's manifest N's reader refuses stops the run with
 // the exit code 1, as `selfhost hir`.
 async fn mir_command(argv: Vec<String>) {
-    let usage = "usage: selfhost mir --std DIR [--before-async] FILE..."
+    let usage = "usage: selfhost mir --std DIR [--before-async | --ll] FILE..."
     let mut std_dir = ""
     let mut before = false
+    let mut ll = false
     let mut bad = false
     let mut only_files = false
     let mut files: Vec<String> = Vec::new()
@@ -305,16 +312,19 @@ async fn mir_command(argv: Vec<String>) {
         } else if a == "--before-async" {
             if before { bad = true }
             before = true
+        } else if a == "--ll" {
+            if ll { bad = true }
+            ll = true
         } else if a.starts_with("-") {
             // `--std` with no value, `--std-part`, `--target`, `--test`,
-            // `--before-async=…` and every other option.
+            // `--before-async=…`, `--ll=…` and every other option.
             bad = true
         } else {
             files.push(a)
         }
         i += 1
     }
-    if bad || std_dir == "" || files.is_empty() {
+    if bad || std_dir == "" || files.is_empty() || (before && ll) {
         eprintln(usage)
         exit(2)
         return
@@ -338,7 +348,7 @@ async fn mir_command(argv: Vec<String>) {
     if shared.shareable && s.shareable { items = Some(s) }
     let mut failed = false
     for path in files.iter() {
-        match dump_mir(path, shared, items, before).await {
+        match dump_mir(path, shared, items, before, ll).await {
             DumpedText(out) => print("== ${path}\n${out}")
             DumpedIoError(e) => {
                 print("== ${path}\nIOERR ${io_kind_name(e.kind)}\n")
@@ -362,12 +372,13 @@ async fn mir_command(argv: Vec<String>) {
 // be and in full otherwise, and, when neither gave an error, the full
 // check's module, std's functions spliced in, its entry `main` kept and
 // the module lowered natively, through the async transform or, when
-// `before`, before it.
+// `before`, before it; when `ll`, the lowered module's `.ll` after it.
 async fn dump_mir(
     path: String,
     shared: SharedStd,
     items: Option<SharedItems>,
     before: Bool,
+    ll: Bool,
 ) -> Dumped {
     let mut graph: Option<Graph> = None
     let mut kind = RootLoose
@@ -410,12 +421,12 @@ async fn dump_mir(
         )
     }
     if items.is_some() {
-        match mir_of(sources, shared, items, before) {
+        match mir_of(sources, shared, items, before, ll) {
             Some(d) => return d
             None => ()
         }
     }
-    mir_of(sources, shared, None, before).unwrap()
+    mir_of(sources, shared, None, before, ll).unwrap()
 }
 
 // `dump_mir`'s part from the resolver on, as `hir_of`'s: with std's
@@ -424,12 +435,14 @@ async fn dump_mir(
 // (`shared_module`); without, resolved and checked in full. An error
 // before MIR is `checked=false`. `None` when the shared path cannot check
 // the program, because it implements anything on a type it does not own,
-// which the full path then checks (ADR 0056 §4).
+// which the full path then checks (ADR 0056 §4). When `ll`, a module that
+// lowered has its `.ll` part after its own (`ll_part`, spec 5.5a §4).
 fn mir_of(
     sources: Vec<ModuleSource>,
     shared: SharedStd,
     items: Option<SharedItems>,
     before: Bool,
+    ll: Bool,
 ) -> Option<Dumped> {
     let std_set = shared.std_set
     let resolved = match items {
@@ -455,7 +468,30 @@ fn mir_of(
     }
     keep_the_entry_main(program, resolved.definitions)
     let result = lower_module_with(program, before)
-    Some(DumpedText(dump_mir_program(ReachedLowered(result, !before))))
+    let text = dump_mir_program(ReachedLowered(result, !before))
+    if ll {
+        match result {
+            Ok(lowered) => return Some(DumpedText(text + ll_part(lowered)))
+            Err(_) => ()
+        }
+    }
+    Some(DumpedText(text))
+}
+
+// A lowered root's `.ll` part (nova-driver's `ll_part`, spec 5.5a §4):
+// `(Ll ok)` and the emitter's text, or one line `(Ll error "…")` with the
+// emitter's words, quoted as the dumps quote.
+fn ll_part(program: MirModule) -> String {
+    match compile_ir(program) {
+        Ok(ir) => "(Ll ok)\n" + ir
+        Err(e) => {
+            let mut part = StringBuilder::new()
+            part.push("(Ll error \"")
+            push_escaped(part, e, true)
+            part.push("\")\n")
+            part.to_string()
+        }
+    }
 }
 
 // What `dump_resolution` gives for one root: its part; the error that
diff --git a/selfhost/src/mir.nova b/selfhost/src/mir.nova
index 9ded91c0..64e859dd 100644
--- a/selfhost/src/mir.nova
+++ b/selfhost/src/mir.nova
@@ -4,9 +4,9 @@
 // `MirTy`, `Temp`, `BlockId`, `Module`, `ExternDecl`, `Function`, `Block`,
 // `OperandClass`, `RtFunc`'s variant names in `RtFunc::ALL`'s order, `Stmt`
 // but its WASM-only `Mem`, `Terminator`, `mir_ty`, `mangle` and
-// `mangle_ty`. `RtFunc::symbol` and `signature` come with the emitter in
-// 5.5 (decision 7); the WASM code, `on_wasm` and `MemOp`, is S0's alone
-// (decision 4).
+// `mangle_ty`; since 5.5a, `RtFunc::symbol` and `signature`, which the
+// emitter reads (5.4's decision 7). The WASM code, `on_wasm` and `MemOp`,
+// is S0's alone (decision 4).
 //
 // The Rust's idioms, beyond hir.nova's (ADR 0057):
 //
@@ -185,6 +185,157 @@ pub const RT_FUNC_VARIANTS: [String] = [
     "RouterStateStore",
 ]
 
+/// Each runtime function's symbol and signature (`RtFunc::symbol`,
+/// `RtFunc::signature`), by its index, in `RtFunc::ALL`'s order as
+/// `RT_FUNC_VARIANTS`: the symbol, then the parameters' classes and the
+/// return class by their variants' names (5.5a, spec
+/// docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md
+/// §5.1). The emitter declares each and calls it by them (emit_ll.nova).
+pub const RT_FUNC_SIGNATURES: [(String, String)] = [
+    ("nova_rt_println", "(Ptr) -> Unit"),
+    ("nova_rt_print", "(Ptr) -> Unit"),
+    ("nova_rt_eprint", "(Ptr) -> Unit"),
+    ("nova_rt_eprintln", "(Ptr) -> Unit"),
+    ("nova_rt_str_concat", "(Ptr, Ptr) -> Ptr"),
+    ("nova_rt_str_concat_n", "(Ptr) -> Ptr"),
+    ("nova_rt_int_to_str", "(I64) -> Ptr"),
+    ("nova_rt_float_to_str", "(F64) -> Ptr"),
+    ("nova_rt_float_fixed", "(F64, I64) -> Ptr"),
+    ("nova_rt_str_to_float", "(Ptr) -> F64"),
+    ("nova_rt_str_is_float", "(Ptr) -> I8"),
+    ("nova_rt_float_to_bits", "(F64) -> I64"),
+    ("nova_rt_bool_to_str", "(I8) -> Ptr"),
+    ("nova_rt_char_to_str", "(I64) -> Ptr"),
+    ("nova_rt_str_eq", "(Ptr, Ptr) -> I8"),
+    ("nova_rt_str_cmp", "(Ptr, Ptr) -> I64"),
+    ("nova_rt_str_hash", "(Ptr) -> I64"),
+    ("nova_rt_str_len_chars", "(Ptr) -> I64"),
+    ("nova_rt_str_chars", "(Ptr) -> Ptr"),
+    ("nova_rt_str_from_chars", "(Ptr) -> Ptr"),
+    ("nova_rt_str_join", "(Ptr, Ptr) -> Ptr"),
+    ("nova_rt_str_index_of", "(Ptr, Ptr) -> I64"),
+    ("nova_rt_json_quote", "(Ptr) -> Ptr"),
+    ("nova_rt_str_to_upper", "(Ptr) -> Ptr"),
+    ("nova_rt_str_to_lower", "(Ptr) -> Ptr"),
+    ("nova_rt_alloc", "(I64) -> Ptr"),
+    ("nova_rt_check_bounds", "(I64, I64) -> Unit"),
+    ("nova_rt_panic_str", "(Ptr) -> Unit"),
+    ("nova_rt_test_selector", "() -> I64"),
+    ("nova_rt_task_spawn", "(Ptr) -> I64"),
+    ("nova_rt_task_block_on", "(Ptr) -> I64"),
+    ("nova_rt_task_is_done", "(Ptr) -> I8"),
+    ("nova_rt_task_take_output", "(I64) -> I64"),
+    ("nova_rt_task_release", "(Ptr) -> Unit"),
+    ("nova_rt_task_yield_future", "() -> Ptr"),
+    ("nova_rt_task_sleep_future_nanos", "(I64) -> Ptr"),
+    ("nova_rt_task_join_future", "(Ptr) -> Ptr"),
+    ("nova_rt_task_timeout_future", "(I64, Ptr) -> Ptr"),
+    ("nova_rt_fs_read_to_string", "(Ptr) -> I64"),
+    ("nova_rt_fs_write_string", "(Ptr, Ptr) -> I64"),
+    ("nova_rt_fs_take_string", "() -> Ptr"),
+    ("nova_rt_fs_last_error_message", "() -> Ptr"),
+    ("nova_rt_fs_temp_dir", "() -> Ptr"),
+    ("nova_rt_fs_exists", "(Ptr) -> I8"),
+    ("nova_rt_fs_create_dir", "(Ptr) -> I64"),
+    ("nova_rt_fs_create_dir_all", "(Ptr) -> I64"),
+    ("nova_rt_fs_remove_file", "(Ptr) -> I64"),
+    ("nova_rt_fs_remove_dir_all", "(Ptr) -> I64"),
+    ("nova_rt_fs_rename", "(Ptr, Ptr) -> I64"),
+    ("nova_rt_fs_read_dir", "(Ptr) -> I64"),
+    ("nova_rt_fs_take_string_array", "() -> Ptr"),
+    ("nova_rt_fs_kind", "(Ptr) -> I64"),
+    ("nova_rt_fs_read", "(Ptr) -> I64"),
+    ("nova_rt_fs_take_bytes", "() -> Ptr"),
+    ("nova_rt_fs_write", "(Ptr, Ptr) -> I64"),
+    ("nova_rt_file_open", "(Ptr, I8, I8, I8, I8, I8, I8) -> I64"),
+    ("nova_rt_file_close", "(I64) -> I64"),
+    ("nova_rt_file_read", "(I64, I64) -> I64"),
+    ("nova_rt_file_write", "(I64, Ptr) -> I64"),
+    ("nova_rt_file_flush", "(I64) -> I64"),
+    ("nova_rt_io_stdin_read", "(I64) -> I64"),
+    ("nova_rt_io_stdout_write", "(Ptr) -> I64"),
+    ("nova_rt_io_stderr_write", "(Ptr) -> I64"),
+    ("nova_rt_io_stdout_flush", "() -> I64"),
+    ("nova_rt_io_stderr_flush", "() -> I64"),
+    ("nova_rt_net_connect_future", "(Ptr) -> Ptr"),
+    ("nova_rt_net_close", "(I64) -> I64"),
+    ("nova_rt_net_read_future", "(I64, I64) -> Ptr"),
+    ("nova_rt_net_write_future", "(I64, Ptr) -> Ptr"),
+    ("nova_rt_net_read_timeout_future", "(I64, I64, I64) -> Ptr"),
+    ("nova_rt_net_listen", "(Ptr) -> I64"),
+    ("nova_rt_net_local_port", "(I64) -> I64"),
+    ("nova_rt_net_accept_future", "(I64) -> Ptr"),
+    ("nova_rt_bytes_len", "(Ptr) -> I64"),
+    ("nova_rt_bytes_from_string", "(Ptr) -> Ptr"),
+    ("nova_rt_bytes_is_utf8", "(Ptr) -> I8"),
+    ("nova_rt_bytes_to_string_unchecked", "(Ptr) -> Ptr"),
+    ("nova_rt_bytes_at", "(Ptr, I64) -> I64"),
+    ("nova_rt_bytes_slice", "(Ptr, I64, I64) -> Ptr"),
+    ("nova_rt_bytes_concat", "(Ptr, Ptr) -> Ptr"),
+    ("nova_rt_bytes_to_ints", "(Ptr) -> Ptr"),
+    ("nova_rt_bytes_from_ints", "(Ptr) -> Ptr"),
+    ("nova_rt_bytes_eq", "(Ptr, Ptr) -> I8"),
+    ("nova_rt_int_hash_seed", "() -> I64"),
+    ("nova_rt_http_parse_request", "(Ptr) -> Ptr"),
+    ("nova_rt_crypto_hash", "(I64, Ptr, Ptr, Ptr) -> I64"),
+    ("nova_rt_crypto_random_bytes", "(I64) -> I64"),
+    ("nova_rt_crypto_random_int", "(I64, I64) -> I64"),
+    ("nova_rt_crypto_pbkdf2_sha256", "(Ptr, Ptr, I64, I64) -> I64"),
+    ("nova_rt_crypto_pbkdf2_sha256_verify", "(Ptr, Ptr, I64, Ptr) -> I64"),
+    ("nova_rt_time_now_nanos", "() -> I64"),
+    ("nova_rt_time_now_epoch_nanos", "() -> I64"),
+    ("nova_rt_log_config_level", "() -> I64"),
+    ("nova_rt_log_config_to_stderr", "() -> I64"),
+    ("nova_rt_log_set_config", "(I64, I64) -> Unit"),
+    ("nova_rt_shutdown_requested", "() -> I8"),
+    ("nova_rt_process_arg_count", "() -> I64"),
+    ("nova_rt_process_arg", "(I64) -> Ptr"),
+    ("nova_rt_process_exit", "(I64) -> Unit"),
+    ("nova_rt_process_run", "(Ptr, Ptr, I8) -> I64"),
+    ("nova_rt_process_take_status", "() -> I64"),
+    ("nova_rt_process_take_stdout", "() -> Ptr"),
+    ("nova_rt_process_take_stderr", "() -> Ptr"),
+    ("nova_rt_process_env", "(Ptr) -> I8"),
+    ("nova_rt_process_current_exe", "() -> I64"),
+    ("nova_rt_ui_state_present", "() -> I8"),
+    ("nova_rt_ui_state_load", "() -> Ptr"),
+    ("nova_rt_ui_state_store", "(Ptr) -> Unit"),
+    ("nova_rt_router_state_present", "() -> I8"),
+    ("nova_rt_router_state_load", "() -> Ptr"),
+    ("nova_rt_router_state_store", "(Ptr) -> Unit"),
+]
+
+/// The symbol runtime function `f` is linked by (`RtFunc::symbol`).
+pub fn rt_func_symbol(f: Int) -> String {
+    let (symbol, _) = RT_FUNC_SIGNATURES[f]
+    symbol
+}
+
+/// Runtime function `f`'s parameter classes and return class
+/// (`RtFunc::signature`), read from `RT_FUNC_SIGNATURES`.
+pub fn rt_func_signature(f: Int) -> (Vec<Int>, Int) {
+    let (_, signature) = RT_FUNC_SIGNATURES[f]
+    read_signature(signature)
+}
+
+/// The parameter classes and return class a signature of
+/// `RT_FUNC_SIGNATURES` names, `"(Ptr, I64) -> Ptr"`.
+pub fn read_signature(signature: String) -> (Vec<Int>, Int) {
+    let arrow = signature.index_of(") -> ").unwrap()
+    let mut params: Vec<Int> = Vec::new()
+    let inside = signature.byte_slice(1, arrow)
+    if inside != "" { for name in inside.split(", ").iter() { params.push(mir_ty_of_name(name)) } }
+    let ret = mir_ty_of_name(signature.byte_slice(arrow + 5, signature.byte_len()))
+    (params, ret)
+}
+
+// The class whose variant's name is `name`, by `MIR_TY_NAMES`.
+fn mir_ty_of_name(name: String) -> Int {
+    let names = MIR_TY_NAMES
+    for i in 0..names.len() { if names[i] == name { return i } }
+    panic("no class is named `${name}`")
+}
+
 /// `RtFunc::StrConcat`, by its index (the lowering's own runtime
 /// functions, beside the builtins' table; `mir_tests.nova` holds each to
 /// its name).
````

### Task 4: ADR 0058 and the records

Spec §13; decisions 1 to 19.

**Files:**
- Create: `docs/adr/0058-the-self-hosted-ll-emitter-and-the-ll-dump.md`
- Modify: `CHANGELOG.md`, `docs/phase-5-plan.md`,
  `nova-spec/00-MASTER-SPEC.md`, `nova-spec/14-CODEGEN.md`

- [ ] **Step 1: The records.** `patch_block t4-code`:
  - **ADR 0058,** the self-hosted `.ll` emitter and the `.ll` dump: its
    Status `Accepted (2026-11-18)` and this branch; §1 the option and its
    refusals; §2 the form, with one example of every line kind; §3 what
    `compile_ir` is called with, and `build --release`; §4 N's port; §5
    the errors; §6 the inputs and the 17 cases; §7 the harness; §8 the
    coverage test; §9 the report; §10 the prototype's mutants; §11 the
    figures of parts 1 and 2; §12 N's least stack with `--ll`; §13 CI's
    figures; §14 5.5b's questions; §15 the departures and the decisions
    the spec does not settle; §16 the rule; Consequences.
  - **`CHANGELOG.md`,** Unreleased: Added, N's `.ll` emitter with the
    hidden `--ll`, and the rule; nothing Changed.
  - **Dated notes:** `docs/phase-5-plan.md`'s 5.5 amended (the split,
    `--ll` for `--emit-ll`, 5.5a's gate, 5.5b's questions deferred);
    `00-MASTER-SPEC.md`'s Phase 5; `14-CODEGEN.md`'s §2.2 (the emitter
    twice, the hidden dump). All dated 2026-11-18 and naming this branch
    (decision 15).
- [ ] **Step 2: Read them against the code.** Each record's claim against
  the tree: the ADR's decisions against Tasks 1 to 3 and this plan's
  decisions; its counts against the tests' own lines, as `SH --exact
  every_form_the_emitter_handles_occurs_in_the_compared_ll
  the_mir_fuzzers_inputs_agree_and_hold_each_kind
  n_lowers_the_depth_cases_on_a_1_mb_main_thread --nocapture` prints them
  (`the .ll of 633 roots, 1 an error: 183 of 190 forms, 111 runtime
  functions declared; 7 unreachable`; `the .ll by set: loose programs
  202 roots, 748170 lines, 23225549 bytes; packages' mains but N's 7
  roots, 128620 lines, 3787437 bytes; N's root 1 roots, 390750 lines,
  12252987 bytes; HIR cases 341 roots, 71478 lines, 2469445 bytes; MIR
  cases 82 roots, 65547 lines, 2056287 bytes`; the fuzzer's `its .ll
  462319 lines, 14555389 bytes`; `the depth cases' .ll: 120531 lines,
  3617081 bytes`, in 55 s), which are §11's but N's root, whose `.ll`
  §11 measured in part 1, before N held its emitter (decision 17); the
  CI figures against decision 9; its examples against `nova check
  --dump-mir --ll`, by a script that runs S0 on the roots §2 names and
  finds every line of §2's block in S0's output, in order, every snippet
  of the kinds listed after it in its root's output, and §1's quoted
  refusals in S0's stderr:

```sh
mkdir -p "$(dirname "$P/exec55/adrcheck.py")"
cat > "$P/exec55/adrcheck.py" <<'EOF'
#!/usr/bin/env python3
"""adrcheck.py: run from the repository's root, after the build, with $P exported.
ADR 0058's examples against S0's `nova check --dump-mir --ll`, run from the
root. The block after "One example of every line kind" is S0's dump of the
root its header names, in part (`…` lines left out): every other line must
be a line of S0's output, in order. Each snippet in backticks of the list
after it ("The kinds this root does not show") must be in S0's output of the
file its bullet names (a snippet the ADR wraps over two lines is joined with
one space). §1's refusals: each line it quotes is a line of S0's stderr for
its shape, exit 2, nothing on stdout. Prints the verdict and the counts."""
import os
import re
import subprocess
import sys

ADR = "docs/adr/0058-the-self-hosted-ll-emitter-and-the-ll-dump.md"
NOVA = os.path.abspath("target/debug/nova")
CASES = "crates/nova-cli/tests/selfhost/cases/mir/"
text = open(ADR, encoding="utf-8").read()
lines = text.split("\n")
start = next(i for i, l in enumerate(lines) if l.startswith("**One example of every line kind,**"))
a = next(k for k in range(start, len(lines)) if lines[k] == "```")
b = next(k for k in range(a + 1, len(lines)) if lines[k] == "```")
block = lines[a + 1:b]


def dump(*files):
    r = subprocess.run([NOVA, "check", "--dump-mir", "--ll", *files], capture_output=True, text=True)
    assert r.returncode == 0, r.stderr
    return r.stdout


bad, found = [], 0
root = block[0][3:]
out = dump(root).split("\n")
at = 0
for line in block:
    if "…" in line:
        continue
    while at < len(out) and out[at] != line:
        at += 1
    if at == len(out):
        bad.append(f"block: MISSING {line!r}")
        break
    at += 1
    found += 1
# The kinds the root does not show: `- what, `FILE`: `snippet`, ... ;`
rest = "\n".join(lines[b + 1:])
rest = rest[:rest.index("**Why this form**")]
items = re.split(r"\n- ", rest)[1:]
snippets = 0
for item in items:
    flat = re.sub(r"\s*\n\s*", " ", item)
    files = re.findall(r"`((?:cases/mir/|tests/runtime/)[\w./-]+\.nova)`", flat)
    assert len(files) == 1, flat
    path = files[0].replace("cases/mir/", CASES)
    body = flat.split(f"`{files[0]}`", 1)[1]
    out = dump(path)
    for snip in re.findall(r"`([^`]+)`", body):
        if snip.startswith("(Ll error"):
            ok = snip in out
        else:
            ok = snip in out or snip in re.sub(r"\s+", " ", out)
        snippets += 1
        if not ok:
            bad.append(f"{files[0]}: MISSING {snip!r}")
# §1's refusals.
REFUSALS = [
    (["--ll", "--ll", "a.nova"], ["error: the argument '--ll' cannot be used multiple times",
                                 "Usage: nova check --dump-mir [OPTIONS] [FILES]..."]),
    (["--ll=x", "a.nova"], ["error: unexpected value 'x' for '--ll' found; no more were expected",
                           "Usage: nova check --dump-mir --ll [FILES]..."]),
    (["--ll", "--before-async", "a.nova"], ["error: --dump-mir --ll takes no --before-async"]),
    (["--before-async", "--ll"], ["error: --dump-mir --ll takes no --before-async"]),
]
for args, want in REFUSALS:
    r = subprocess.run([NOVA, "check", "--dump-mir", *args], capture_output=True, text=True)
    got = r.stderr.split("\n")
    for w in want:
        if w not in got or w not in re.sub(r"\s*\n\s*", " ", text.replace("`", "")) and w not in text:
            bad.append(f"{args}: {w!r} not in stderr or not in the ADR")
    if r.returncode != 2 or r.stdout:
        bad.append(f"{args}: exit {r.returncode}, stdout {r.stdout[:40]!r}")
r = subprocess.run([NOVA, "check", "--ll", "a.nova"], capture_output=True, text=True)
for w in ["error: unexpected argument '--ll' found", "Usage: nova check [OPTIONS] [FILE]"]:
    if w not in r.stderr.split("\n"):
        bad.append(f"check --ll: {w!r} not in stderr")
print(f"examples: {len(block)} block lines, {found} found in order; {len(items)} kinds, "
      f"{snippets} snippets; {len(REFUSALS) + 1} refusals; {'ok' if not bad else 'BAD ' + '; '.join(bad)}")
sys.exit(1 if bad else 0)
EOF
```

  After the build, `python3 "$P/exec55/adrcheck.py"`. Expected, as the
  replay printed: `examples: 49 block lines, 43 found in order; 5 kinds,
  14 snippets; 5 refusals; ok`.
- [ ] **Step 3: The checks,** as the conventions list them; then
  `rm -rf "$P/target-1100"`.
- [ ] **Step 4: Commit**
      `docs: ADR 0058, the self-hosted .ll emitter and the .ll dump, and
      the notes for 5.5a`.

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index fa721cc6..7d74c279 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -26,6 +26,9 @@ dump (spec
 full HIR dump (§4; ADR 0056). Phase 5.4, the self-hosted MIR and the MIR
 dump, natively (spec
 `docs/superpowers/specs/2026-11-14-phase-5-4-mir-design.md`; ADR 0057).
+Phase 5.5a, the self-hosted `.ll` emitter and the `.ll` dump (spec
+`docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`;
+ADR 0058).
 
 ### Added
 
@@ -313,6 +316,33 @@ dump, natively (spec
   byte for byte, on every `.nova` file the repository tracks, and `nova
   test` and `nova build` on its 11 packages outside `crates/`; the
   packages under `crates/`, test fixtures, were not compared.
+- **`nova check --dump-mir --ll FILE…`, hidden:** after the MIR of each
+  program that lowered, after the async transform, its LLVM IR: `(Ll
+  ok)` and the text `nova build --release` writes for it, byte for byte,
+  or one line `(Ll error "…")` with the emitter's own words, as for a
+  float `%`, which type-checks and which the emitter refuses. A program
+  that did not lower prints none. It is refused with `--before-async`,
+  is absent from `--help`, carries no stability promise and is taken
+  only after `--dump-mir` (5.5a, ADR 0058).
+- **The self-hosted `.ll` emitter, in `selfhost/`:** N emits a
+  program's textual LLVM IR from its native MIR as `nova-codegen-llvm`
+  does, function for function: the runtime functions' declarations with
+  their symbols and signatures, every function's slots, blocks and
+  instructions, the escapes, floats as their bits in hex, the string
+  globals and the C `main`, and the emitter's errors word for word.
+  `selfhost mir --std DIR --ll FILE…` prints the same dump as `nova
+  check --dump-mir --ll`; the differential compares them byte for byte
+  on the corpus roots that run, the HIR cases that reach MIR, MIR's
+  cases with both line endings, among them `ir_tests.rs`'s sources and a
+  float `%` case, a MIR fuzzer's programs and programs nested 128 levels
+  deep, which N emits on a 1 MB stack. N is still built by Cranelift and
+  needs no `clang`. A change to the emitter, to the runtime functions'
+  symbols, signatures or order, or to the runtime's ABI as the emitter
+  spells it now changes both compilers in one pull request (ADR 0058).
+  Nothing a user sees changes with 5.5a: `nova check` gives what it
+  gave, byte for byte, on every `.nova` file the repository tracks, and
+  `nova test` and `nova build` on its 11 packages outside `crates/`; the
+  packages under `crates/`, test fixtures, were not compared.
 
 ### Changed
 
diff --git a/docs/adr/0058-the-self-hosted-ll-emitter-and-the-ll-dump.md b/docs/adr/0058-the-self-hosted-ll-emitter-and-the-ll-dump.md
new file mode 100644
index 00000000..83369e25
--- /dev/null
+++ b/docs/adr/0058-the-self-hosted-ll-emitter-and-the-ll-dump.md
@@ -0,0 +1,881 @@
+# ADR 0058 — The self-hosted `.ll` emitter, and the `.ll` dump
+
+## Status
+
+Accepted (2026-11-18). Branch `claude/loving-archimedes-frtf5q-55`
+(`docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`),
+written as 5.5a's prototype was built. CI's figures (§13) are from the
+prototype's run on all three systems, each under its line (spec §9).
+
+Part 1 (S0's side) built what §1 to §3, §6 and §8 record:
+- §1, the hidden `--ll` and its refusals;
+- §2, the `.ll` in the dump, with one example of every line kind;
+- §3, what `compile_ir` is called with, and the equality with `build
+  --release`;
+- §6, the inputs and the 17 new cases;
+- §8, the coverage test and `LL_UNREACHABLE`.
+
+Part 2 (N's side) built what §4, §5, §7, §9 and §10 record:
+- §4, N's port and its idioms;
+- §5, the emitter's errors;
+- §7, the harness, which compares the `.ll` on every input;
+- §9, the `.ll` mismatch report and its test;
+- §10, the mutants.
+
+Part 3 measured N's least stack on the depth cases with `--ll` (§12)
+and wrote the records: §11 to §16 and the Consequences, with the
+figures of parts 1 and 2 (§11).
+
+## Decision
+
+### 1. The option, `nova check --dump-mir --ll FILE…`
+
+ADR 0053's mechanism, as ADR 0057 §1 (spec §3.1, the user's decision
+2). `DumpMirCmd` (`nova-cli/src/cmd/run.rs`) gains `--ll`, taken once,
+anywhere after `--dump-mir`, before or after the files, and before
+`--`. Each root's after part is printed as in 5.4, followed, for a root
+that lowered, by its `.ll` (§2).
+
+The refusals, as built (each nothing on stdout, exit 2):
+- **`--ll` twice** is clap's error under the form's usage line:
+  `error: the argument '--ll' cannot be used multiple times`, then
+  `Usage: nova check --dump-mir [OPTIONS] [FILES]...`.
+- **`--ll=x`** is clap's error under a usage line that names the flag
+  used, as the base's `--before-async=1` already does: `error:
+  unexpected value 'x' for '--ll' found; no more were expected`, then
+  `Usage: nova check --dump-mir --ll [FILES]...`. The spec wrote
+  `[OPTIONS] [FILES]...` for every shape (§15).
+- **`--ll` with `--before-async`,** in either order, with or without a
+  file: one line, `error: --dump-mir --ll takes no --before-async`,
+  checked before the missing file's `error: --dump-mir needs a FILE`.
+  The emitter reads the after form alone; the before form holds an
+  `Await` in every async program (§5).
+- **`nova check --ll`, `nova check --dump-ll`,** `--ll` before
+  `--dump-mir`, after a plain `check`'s file, or inside `--dump-mir=…`:
+  not the form, so the base's refusal stands, byte for byte (for `nova
+  check --ll FILE`: `error: unexpected argument '--ll' found`, clap's
+  tip, `Usage: nova check [OPTIONS] [FILE]`).
+
+After `--`, `--ll` is a root's name (`-- --ll` prints `== --ll` and
+`IOERR NotFound`). Without `--ll` the dump is 5.4's, byte for byte
+(§11's `mircmp`).
+`--ll` is in no `--help` and no usage line of the base's shapes.
+
+N's side is `selfhost mir --std DIR [--before-async | --ll] FILE…`:
+`--ll` once; twice, as `--ll=…`, or with `--before-async`, N prints its
+usage line, `usage: selfhost mir --std DIR [--before-async | --ll]
+FILE...`, on stderr, nothing on stdout, and exits 2.
+
+**Checked by:**
+- `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`
+  (`selfhost.rs`), extended: `check --help` does not name `--ll`; the
+  five base shapes above give the base's error; eleven `--ll` shapes of
+  the form's own parser are one error each, exit 2, nothing on stdout:
+  `--ll` twice, twice around a file, `--ll=x`, `--ll=`, with `--std`,
+  with `--help`, with `--before-async` in three orders and with it but
+  no file (these four the line above), and alone with no file; `--ll`
+  before `--` takes a root that starts with `-`.
+- `both_sides_refuse_the_mir_dumps_shapes_alike` gains `--ll` twice,
+  `--ll=x` and `--ll` with `--before-async` in both orders: exit 2 and
+  nothing on stdout on both sides, and N's exact usage line.
+- `usagecmp.sh` (§11): 558 base shapes under six `argv[0]`s, 10 in a
+  terminal, and the hidden forms' refusals, the `--ll` shapes added.
+
+### 2. The `.ll` in the dump
+
+The form (spec §4). A root that lowered (`lowered=true`) is followed,
+after its `(Diagnostics)`, by one of:
+- **`(Ll ok)`, then `nova_codegen_llvm::compile_ir`'s text, byte for
+  byte,** ending in its own line break. There is no closing line: the
+  `.ll` runs to the next `== ` header or the end. No line of the `.ll`
+  starts with `(` or `== `, so 5.4's readers stop where they stopped.
+- **One line `(Ll error "…")`:** the emitter's own words, quoted as the
+  dumps quote a string (`nova_resolver::dump::quote`, 5.1's), without
+  `build --release`'s context line. The dump's exit code stays 0, as for
+  a part with MIR's diagnostics.
+
+A root that did not lower (`runs=false`, `checked=false`,
+`lowered=false`, `IOERR`) prints no `(Ll` line, and `compile_ir` is
+never called on it: `(Ll` occurs exactly when `lowered=true`. The
+before form takes no `.ll`.
+
+**One example of every line kind,** S0's `nova check --dump-mir --ll`
+at `8cb1b42b`, shown in part; every line below is S0's, byte for byte,
+and `…` stands for lines left out. The root is
+`cases/mir/ir_tests__recursion_and_branch_lower.nova`, `ir_tests.rs`'s
+`fn fib(n: Int) -> Int { if n <= 1 { n } else { fib(n - 1) + fib(n -
+2) } }` and `fn main() { println("${fib(5)}") }`:
+
+```
+== crates/nova-cli/tests/selfhost/cases/mir/ir_tests__recursion_and_branch_lower.nova
+(Mir runs=true checked=true lowered=true async_lowered=true functions=2 externs=0
+…
+(Diagnostics)
+(Ll ok)
+; LLVM IR emitted by the Nova compiler (release backend).
+; Requires LLVM >= 15 (opaque pointers).
+
+declare void @llvm.trap()
+declare void @nova_rt_println(ptr)
+…
+declare void @nova_rt_router_state_store(ptr)
+declare ptr @nova_rt_str_new(ptr, i64)
+
+define void @"nova_main"() {
+entry:
+  %t0.slot = alloca i64
+…
+define i64 @"fib.0"(i64 %p0) {
+entry:
+  %t0.slot = alloca i64
+…
+  store i64 %p0, ptr %t0.slot
+  br label %bb0
+bb0:
+  store i64 1, ptr %t2.slot
+  %r0 = load i64, ptr %t0.slot
+  %r1 = load i64, ptr %t2.slot
+  %r2 = icmp sle i64 %r0, %r1
+  %r3 = zext i1 %r2 to i8
+  store i8 %r3, ptr %t3.slot
+  %r4 = load i8, ptr %t3.slot
+  %r5 = icmp ne i8 %r4, 0
+  br i1 %r5, label %bb1, label %bb3
+bb1:
+…
+bb2:
+  %r7 = load i64, ptr %t1.slot
+  ret i64 %r7
+…
+  br label %bb2
+}
+
+
+define i32 @main(i32 %argc, ptr %argv) {
+entry:
+  call void @nova_main()
+  ret i32 0
+}
+```
+
+In order: the two comment lines and a blank line; `llvm.trap`'s
+declaration; the 111 runtime functions' declarations in `RtFunc::ALL`'s
+order; `nova_rt_str_new`'s, which `ConstStr` calls and no `RtFunc`
+names; each extern's; a blank line; each function, `main` emitted as
+`nova_main`, each followed by a blank line, its temps' `alloca`s and its
+parameters' stores under `entry:`, then its MIR blocks as `bbN:`, every
+instruction indented two spaces, `%rN` restarting at 0 in each function;
+the string globals, then a blank line (two blank lines in a row where
+there is no string, as here); the C entry wrapper. The kinds this root
+does not show, from other inputs at the same commit:
+- an extern's declaration, `tests/runtime/extern_ffi.nova`:
+  `declare i64 @"llabs"(i64)`, `declare double @"pow"(double, double)`;
+- string globals, `cases/mir/ll_a_string_of_every_escape_class.nova`,
+  every escape class: `@str0 = private unnamed_addr constant [8 x i8]
+  c"say \22hi\22", align 1`, `c"a\5Cb"`, `c"tab\09here\00nul\0D\0A"`,
+  `c"caf\C3\A9 \E2\94\80"`, the empty string `[1 x i8] c"\00"` and a
+  raw line break in a literal, `c"first\0Asecond"`;
+- a float, `cases/mir/ll_a_float_le_is_an_ordered_compare.nova`: `store
+  double 0x3FF8000000000000, ptr %t2.slot`, `%r4 = fcmp ole double %r2,
+  %r3`;
+- a switch on an `I8` and the trap,
+  `cases/mir/ir_tests__bool_match_switches_on_i8_not_i64.nova`: `switch
+  i8 %r0, label %bb4 [ i8 1, label %bb2 i8 0, label %bb3 ]`, then `call
+  void @llvm.trap()` and `unreachable`;
+- the error line, `cases/mir/ll_a_float_remainder_errs.nova`: `(Ll
+  error "float remainder (%) is not supported by the LLVM backend")`.
+
+**Why this form** (not asked; the spec's recommendation): the part
+still opens with `(Mir` and 5.4's grammar still ends at
+`(Diagnostics)`; the harness cuts each `(Ll` part off before 5.4's
+checks (§7) and the report's walk enters the `.ll` at `(Ll ok)` (§9).
+Quoting each line into 5.4's grammar would change the bytes, so the
+`.ll` would no longer be the intermediate `build --release` writes, and
+would cost N quoting about 62 MB.
+
+### 3. What `compile_ir` is called with, and `build --release`
+
+`MirDumps::new(before_async, ll)` and `MirDumps::unshared(before_async,
+ll)` (`nova-driver/src/lib.rs`) take whether to print the `.ll`;
+`Wanted` gains `ll`, asked for only with the after form. In
+`HirDumps::dumps`, when the after form lowered (`Some(Ok(module))`), the
+driver appends `ll_part(module)`: `(Ll ok)` and `compile_ir(module)`'s
+text, or `(Ll error ` and the error's `to_string()` quoted. The printing
+lives in the driver: `nova-mir` cannot depend on the emitter, and the
+emitter's crate does not depend on `nova-resolver`, so another place
+would change a manifest and `Cargo.lock`. S0's emitter and MIR do not
+change.
+
+**The module is the one `build --release` emits** (spec §2 item 4): the
+after form's, std's shared functions spliced in, `keep_the_entry_main`
+and `mir_options(None)` natively; `build_program_release` takes
+`lower_to_mir`'s, from the full check. 5.4's unit test and guard hold
+the two modules equal (ADR 0057 §5), and `compile_ir` is a function of
+the module alone. **The driver's guard is not extended** (not asked):
+`Wanted::ALL` keeps `ll: false`, so the Test job's slowest binary (ADR
+0057 §19) does no more work.
+
+**Checked by:**
+- `the_ll_dump_is_what_build_release_writes` (`selfhost.rs`, S0 alone,
+  temporary directory `nova-selfhost-ll-build-release`): one `check
+  --dump-mir --ll` of `tests/runtime/generics.nova`,
+  `examples/05-geometry/src/main.nova` (a package with a path
+  dependency, given as FILE), `tests/runtime/async_tasks.nova`,
+  `tests/runtime/extern_ffi.nova`, `cases/mir/mir_every_native_form.nova`
+  and the float `%` case; then, for each, `nova build --release FILE -o
+  outN` with `NOVA_CLANG` and `NOVA_LLC` a file that is not there and a
+  fresh `$NOVA_HOME`. Each exits 1. The five that emit leave
+  `outN.nova.ll`, equal byte for byte to the root's `.ll` in the dump,
+  and say `release build failed; generated LLVM IR left at `; the float
+  `%` case leaves none, and its stderr ends with the words its `(Ll
+  error` line quotes. `examples/01-hello-world`, whose `src/main.nova`
+  spec §4's example shows, has no `nova.toml`, so it is not a package
+  (§15).
+- Two driver unit tests, `mod ll_dumps` (temporary directory
+  `nova-driver-ll-dump`):
+  - `the_ll_part_is_the_emitters_text_or_its_words_quoted`: `(Ll ok)`
+    and the text for a module of one empty `main`; and `(Ll error "…")`
+    with each error's words for modules built by hand: an `Await`, a
+    float `Rem`, a float `BitAnd`, and a module whose one function is
+    not `main`.
+  - `the_ll_dump_is_the_module_build_release_emits`: on six programs (a
+    hello with an escaped quote, an async program, an extern block with
+    a closure, a float `%`, no `main`, a type error), the dump with
+    `--ll` is the dump without it, then `(Ll ok)` and `compile_ir` of
+    `lower_to_mir`'s module, the function `build_program_release`
+    calls; the float `%` gives the error both ways; the two that do not
+    lower get no `.ll`; the before form is the same with `ll` or without.
+- The header test: the after form with `--ll`, its `(Ll` parts cut off,
+  equals plain `--dump-mir` on its roots.
+
+### 4. N's port
+
+N's emitter is a port of `nova-codegen-llvm/src/lib.rs` (761 lines,
+633 of code) and of `RtFunc::symbol` and `signature`
+(`nova-mir/src/lib.rs`), function by function and with S0's names (spec
+§5.1):
+
+| N | lines (code) | ports |
+|---|---:|---|
+| `emit_ll.nova` (new) | 690 (562) | `compile_ir`, `rt_func_decl`, `llty`, `escape_bytes`, `float_hex`, `FnEmit` and its helpers, `stmt`, `arg_list`, `emit_call`, `binop`, `terminator` |
+| `mir.nova` | +154, −3 | `RT_FUNC_SIGNATURES` (111 entries), `rt_func_symbol`, `rt_func_signature`, `read_signature` |
+| `main.nova` | +63, −26 | `selfhost mir --ll`, `ll_part` beside `mir_of` |
+| `emit_ll_tests.nova` (new) | 200 (163) | N's six `@test`s |
+
+Code lines exclude blank lines and lines that start with `//` after
+their indentation. N's `selfhost/src` is then 55 files and 31,399
+lines, and `nova test` passes 108 (102 before).
+
+**Idioms,** beyond ADR 0057 §10's:
+- **Text:** one `StringBuilder`, parts pushed and joined once (S0 builds
+  the functions' text, then the header). The header reads nothing the
+  functions make, so N writes it first into the same builder; the bytes
+  are the same. A root's `.ll` is built whole, as S0 builds it (§5.2 of
+  the spec, not asked), and printed with its part.
+- **`escape_bytes` onto the builder:** printable ASCII but `"` and `\`
+  pushed in runs, every other byte `\XX` with two uppercase hex digits
+  and no `x`, by bytes (`byte_at`), never by characters. dump.nova's
+  `push_escaped`, lowercase `\xHH` that keeps bytes past ASCII, is not
+  reused.
+- **The empty string's `\00`** is a case at the global, `[1 x i8]
+  c"\00"`; every other length is the byte length.
+- **`float_hex`:** `0x` and the 16 uppercase hex digits of
+  `Float.to_bits`, by an uppercase helper (`hex_upper`), S0's
+  `0x{:016X}`.
+- **`RtTable`, read once:** each runtime function's declaration,
+  symbol and return class, read from `RT_FUNC_SIGNATURES` once per
+  module, since each read of a Nova `const` builds the array anew.
+- **The signature table's strings:** each entry is the symbol and a
+  string `"(Ptr, I64) -> Ptr"`, the classes by their variants' names,
+  read by `read_signature`; generated from S0's `.ll` header, and every
+  `.ll` compared checks it (§7).
+- **`ret_of`'s order:** a `Map` read by lookup alone, the functions
+  inserted, then the externs, so an extern's class replaces a
+  function's of the same symbol, as S0's `collect` over the two keeps
+  the last; a callee it lacks is `Unit`. `string_index` is a `Map` read
+  by lookup alone, the strings in a `Vec` by first use. No map's order
+  reaches the text.
+- **Codes:** a class is its `MirTy` code, `llty` a table of them; a
+  runtime function its index. Hard-coded as in S0: 8-byte slots, `i64`
+  sizes, the heap layouts of sums, records, arrays and closures, the
+  env-first indirect call, `main` as `nova_main` and the C wrapper.
+
+**N's `@test`s** (`emit_ll_tests.nova`):
+`escape_bytes_writes_each_byte_class_as_s0_does`;
+`float_hex_is_the_bits_in_16_uppercase_digits` (0, −0.0, 1.5, a NaN from
+`"NaN".parse_float()`, 1e300, 2.5e-308, −inf);
+`a_symbol_both_a_function_and_an_extern_returns_the_externs_class` (and
+a missing callee's `Unit`);
+`every_runtime_function_has_a_distinct_symbol_and_its_signatures_declaration`
+(S0's `rt_func_tests`, and six declarations exactly);
+`a_small_module_is_the_header_its_function_and_the_c_main`;
+`each_error_of_the_emitter_is_in_s0s_words` (§5). They also run on the
+debug runtime in the Selfhost job's last step; locally they pass on
+it.
+
+**The command:** `selfhost mir` takes `--ll` (§1); `mir_of` appends
+`ll_part` to a `lowered=true` part, from the module it lowered: `(Ll
+ok)` and the text, or `(Ll error "…")` quoted with `push_escaped`, as
+S0's `quote` quotes it.
+
+### 5. The emitter's errors (the user's decision 4)
+
+N's `compile_ir` returns `Result<String, String>`, each `bail!` ported
+with S0's words:
+- **float `%`** (`Bin` `Rem` on `Float`): reachable, since it
+  type-checks. `build --release` prints "Error: internal LLVM IR
+  generation error (this is a compiler bug)", "Caused by:" and the
+  words, exit 1; the dump prints `(Ll error "float remainder (%) is not
+  supported by the LLVM backend")` on both sides, exit 0. Its case is
+  `cases/mir/ll_a_float_remainder_errs.nova`, the only input that gives
+  an error (§8). S0's bug is not fixed (§14).
+- **float bitwise** ("bitwise operators are not defined for Float"),
+  **no `main`** ("no `main` function in MIR module") and **`Await`**
+  ("an `.await` marker reached codegen: `nova-mir`'s async
+  state-machine transform must split every `async fn` body at its await
+  points before a module is emitted"): ported as written, **unreachable
+  natively**, as ADR 0057 §7 records E0011. `&`, `|`, `^`, `<<` and `&=`
+  on `Float`, and `a & b` in a generic function, are E0013 before MIR,
+  and `~` on a `Float` is E0010; lowering makes no other float `Bin`. No
+  `main` is E0601 first. An `Await` is in the before form alone, which
+  takes no `.ll`. Each is pinned by S0's unit test (§3) and by N's
+  `each_error_of_the_emitter_is_in_s0s_words`, on modules built by hand,
+  with the same words (float `&` and `<<` for the bitwise one).
+- **`Mem`** ("a WASM memory access (…"): **not ported.** N's MIR has no
+  `Mem` statement (5.4 left WASM's MIR out, ADR 0057 §6), so the arm has
+  nothing to match.
+
+### 6. The inputs and the cases
+
+`MirForm::After` is S0's `check --dump-mir --ll` (`MIR_LL_S0`) and N's
+`mir --std DIR --ll`, so the `.ll` is compared wherever 5.4 compares the
+after form, through `compare_batch`, byte for byte with exit codes, per
+host, nothing normalized (spec §6.1, the user's decision 3):
+1. the native corpus's roots that run: 251 loose and 10 packages'
+   `src/main.nova`; 210 lower and print a `.ll`, N's
+   `selfhost/src/main.nova` among them;
+2. the HIR cases that reach MIR (342; 341 print a `.ll`) and
+   `cases/mir/` (103 roots; 82 print an `(Ll` part, 81 a `.ll` and the
+   float `%` case its error);
+3. the MIR fuzzer's 120 inputs and the 25 depth cases (ADR 0057 §16,
+   §17), unchanged; N runs the depth cases with `--ll` on a 1 MB main
+   thread (§12);
+4. the cases again with `\r\n`, as 5.4, the `.ll` included.
+
+The before form stays 5.4's, without `--ll`. Root lists pass
+`HIR_ARGS_LIMIT` in parts, as 5.4.
+
+**The 17 new cases,** in `cases/mir/` (not asked: `cases/mir/` is
+already an after-form input with both line endings, so no new list or
+test; their MIR is compared too):
+- **14 `ir_tests__<test>[_n].nova`,** the sources
+  `nova-codegen-llvm/tests/ir_tests.rs` feeds through `ir_for`,
+  extracted as 5.4 extracted `lower_tests.rs`'s: a temporary
+  instrumentation, a script outside the repository and never committed,
+  recorded every source; every `format!` is expanded; `_n` numbers a
+  test's sources when it has more than one. 20 sources were recorded;
+  **6 were dropped as duplicates:** both hello worlds (each the text of
+  `cases/check/check__hello_world_checks.nova`), `float_bits`
+  (`tests/runtime/float_bits.nova`, already corpus), and the await-free
+  async function, the field assignment and the repeat array (each the
+  text of a `lower_tests__` case). The spec expected about 18 (§15).
+- **3 by hand:**
+  - `ll_a_float_remainder_errs` (§5);
+  - `ll_a_float_le_is_an_ordered_compare`, `<=` on two `Float`s both
+    ways: `Le` on `Float` was the one (operator, class) pair no input
+    reached (spec §2 item 7);
+  - `ll_a_string_of_every_escape_class`: `"`, `\`, a tab, a NUL, CR LF,
+    UTF-8 of two and three bytes, an empty string and a raw line break
+    in a literal (§2's globals). The `\r\n` test then compares
+    `c"first\0D\0Asecond"` against the checkout's `c"first\0Asecond"`.
+
+`Switch` on an `I8` comes from
+`ir_tests__bool_match_switches_on_i8_not_i64`, so no hand case was
+needed for it. All 17 lower; only the float `%` case's `.ll` is an
+error.
+
+### 7. The harness
+
+Part 1 ran S0's after form with `--ll` while N's did not: a switch,
+`N_LL = false`, had the comparison read S0's output without its `(Ll`
+parts. Part 2 removed the switch: `mir_in(MirForm::After)` always
+passes `--ll`, and `compare_batch` compares raw bytes and exit codes.
+
+- **`split_ll`** cuts each root's `(Ll` part off S0's output after the
+  comparison, so every 5.4 check reads 5.4's text: well formed, a header
+  and a `(Mir` line per root, `KINDS`, the two forms equal where no
+  `$poll` occurs, `MIR_CODES`.
+- **`check_ll`,** in every after-form run: `(Ll ok)` or `(Ll error`
+  exactly when `lowered=true`; `(Ll error` on
+  `FLOAT_REMAINDER_CASE` alone, with its words; each `.ll` starts with
+  the header's first comment and ends in a line break.
+- `agree_on_mir_forms` returns the after form, the before form and the
+  `.ll` parts; the corpus test and the coverage test share one run of
+  the package mains (`package_mir`, a `OnceLock`).
+- **The depth test,** `n_lowers_the_depth_cases_on_a_1_mb_main_thread`,
+  runs N with `--ll` under `ulimit -s 1024` (on Windows, the main thread
+  as it is, 1 MB), and its check that `bodies-for-ranges-127.nova` fails
+  on 96 KB also passes `--ll`.
+- `settled_stdout` touches no `.ll` line: none starts `  (Error`.
+
+### 8. The coverage test (the user's decision 6)
+
+`every_form_the_emitter_handles_occurs_in_the_compared_ll`, on the
+native corpus's roots that run and the cases, sharing their run with
+`the_mir_dumps_agree_on_the_corpus_and_the_cases`, reads S0's output of
+each root that printed an `(Ll` part: its MIR after form, which
+determines its `.ll`, and the `.ll` itself. **190 forms:**
+- 25 statements (`LL_STATEMENTS`: every native statement but `Await`)
+  and 5 terminators, by the MIR dump's kinds;
+- the 28 (operator, class) pairs the type checker admits in a `Bin`
+  (`LL_BINS`): 16 on `Int`, 10 on `Float`, `Eq` and `Ne` on `Bool`;
+- 16 others: `(ConstStr "")`, `Neg` on `Int` and on `Float`, `Switch`
+  on `I8` and on `I64` by the discriminant's class, `Return` with a
+  value and without, a `Unit` temp, a function that takes an env, a
+  `Call` to an extern; in the `.ll`, a string global with `\22`, with
+  `\5C`, with a control byte and with a byte past ASCII, `c"\00"`, and
+  an extern's `declare`;
+- 111 `call @SYMBOL`, one per runtime function, read from each `.ll`'s
+  header, which the test asserts the same in every `.ll`
+  (`nova_rt_str_new` is `ConstStr`'s, which no `RtFunc` names, and is
+  not among them);
+- the 5 errors (`LL_ERRORS`), each `(Ll error NAME)`; an error of no
+  list fails the test.
+
+**Result** (2026-11-18): 633 roots, 1 of them an error; **183 of 190
+forms found**, 111 runtime functions declared. Seen in fewer than three
+roots: `Eq` and `Le` on `Float` (2 each), `BitNot` (1), the float `%`
+error (1), `Switch` on `I8` (1), and 13 runtime functions' calls (1 or
+2 each): four of crypto, four of files, `fs_create_dir`, `fs_rename`,
+stderr's write and flush, and stdout's flush.
+
+**`LL_UNREACHABLE`,** the 7 forms that may be absent, each with its
+reason; the test fails if one occurs, if it names no form, or if any
+form found is in no list:
+- `call @nova_rt_test_selector`: only test mode's synthesized `main`
+  calls it, and test mode's code generation is not compared;
+- `call @nova_rt_task_is_done`: no native std function calls it, and a
+  program's call is E0001;
+- `call @nova_rt_task_take_output`: no MIR statement emits it;
+- `(Ll error no main)`: lowering gives E0601 first;
+- `(Ll error Await)`: an `Await` is in the before form alone, which
+  takes no `.ll`;
+- `(Ll error Mem)`: a memory access is in a WASM build's MIR alone;
+- `(Ll error float bitwise)`: E0013 and E0010 before MIR, and lowering
+  makes no other float `Bin` (§5).
+
+All 111 declarations are in every `.ll`'s header, so every symbol and
+signature is compared on every input whatever the calls: a slip in the
+signature of `TaskTakeOutput`, which nothing calls, failed every one of
+the 777 `.ll`s (§10).
+
+### 9. The `.ll` mismatch report (the user's decision 5)
+
+`compare.rs`'s `MirWalk`, which names a MIR dump's function, block and
+statement (ADR 0057 §11), enters a root's `.ll` at a depth-0 line `(Ll
+ok)` and counts its lines from 1: a `define` line enters a function,
+named by its symbol as quoted (`"g.1"`), or `main` for the C wrapper; a
+label (`entry:`, `bbN:`) inside a function enters a block; `}` leaves
+the function. After the root and the line of its dump, the report names
+`line 27 of its .ll, define "g.1", block bb1`, a `define` line's own
+function with no block, `line 7 of its .ll, outside any define` for the
+header, a declaration, a string global or a blank line between them, or
+`at its (Ll line` at `(Ll ok)` or `(Ll error`. Both lines and the
+reproduce commands (`nova check --dump-mir --ll FILE`, `selfhost mir
+--std DIR --ll FILE`) follow, as for every dump. The walk starts afresh
+at each root; every other report is as it was.
+
+`the_ll_mismatch_report_names_the_define_and_block` pins it on dumps
+written by hand: two roots, the first with two functions (`main` and
+`g.1`), their `.ll` with a string global and the C wrapper, the second
+an error. One full report (line 27 of the `.ll`, `define "g.1", block
+bb1`) is compared whole, then 17 places by the first line they name: the
+header, a declaration and the blank line after them; the first `define`
+line, `entry:`, a slot, a line of `bb0` and the line before `}`; the
+second `define`, a line of its `bb0` and its label `bb1`; a string
+global; the C wrapper (`define main, block entry`); the `(Ll` line and
+the MIR line before it; the second root's `(Ll` line and its MIR; and,
+apart, N leaving the `.ll` out, reported at S0's `(Ll` line.
+
+### 10. The mutants
+
+Each was applied to `selfhost/src` in the working tree, N built from a
+copy, and run through N's `@test`s, a script comparing every input of
+§6 but `\r\n` (851 roots, 777 of them with a `.ll`) and `SH`'s
+`the_mir_dumps_agree_on_the_corpus_and_the_cases`; then restored, never
+committed. **All 12 were killed:**
+
+| Mutant | Killed by | Roots that differ | The report's first place |
+|---|---|---:|---|
+| `float_hex` lowercase | differential, `float_hex_is_the_bits_in_16_uppercase_digits`, `each_error_of_the_emitter_is_in_s0s_words` | 124 | `05-geometry`, `define "PI.3", block bb0` |
+| an escape's digits lowercase | differential, `escape_bytes_writes_each_byte_class_as_s0_does` | 79 | `07-fullstack-blog`, line 36949 of its `.ll`, outside any define (`@str43`, `c"\0c"`) |
+| an `x` in each escape | differential, the same `@test` | 96 | `06-counter-spa`, outside any define (`\x22`) |
+| `ret_of` with the function winning | **the `@test` alone** (`a_symbol_both_a_function_and_an_extern_returns_the_externs_class`) | 0 | none |
+| `%rN` not restarted per function | differential | 534 | `05-geometry`, `define "total_area.13", block bb0` |
+| strings numbered by last use | differential | 238 | `06-counter-spa`, `define "build.536", block bb5` (the `getelementptr`) |
+| `TaskTakeOutput`'s return a `Ptr` | differential | 777 | `05-geometry`, line 37 of its `.ll`, outside any define |
+| a global's length in characters | differential | 31 | N's root, line 389957 of its `.ll`, outside any define |
+| the empty string without its NUL | differential | 145 | `06-counter-spa`, outside any define (`[0 x i8] c""`) |
+| a record's size without `max(1)` | differential | 27 | `03-http-server`, `define "timeout.311$S3LiR93E$poll", block bb8` |
+| a switch always on `i64` | differential, one case | 1 | `ir_tests__bool_match_switches_on_i8_not_i64`, `define "classify.0", block bb0` |
+| float `Le` unordered (`fcmp ule`) | differential, one case | 1 | `ll_a_float_le_is_an_ordered_compare`, `define "nova_main", block bb0` |
+
+**`ret_of` with the function winning cannot be killed by any input,
+natively:** a function's symbol and an extern's coincide only for
+`main`, which E0900 reserves against an extern of that name, or E0002
+(a duplicate definition) in the same module as `fn main`, and every
+other function's symbol holds a `.` (`fib.0`), which no extern's can.
+N's `@test` pins it, as spec §14 risk 5 expected; spec §6.4's statement
+that every mutant fails the differential does not hold for it (§15). The
+`I8` switch and the ordered float compare are each held by one case,
+the case that §6 added for them.
+
+### 11. The figures of parts 1 and 2
+
+Measured locally on 4 cores, 2026-11-18: part 1's in its session, on
+its tree (S0, the harness and the cases those of its head `f3b32068`,
+whose last commit changed a driver unit test alone), part 2's at
+`8cb1b42b`; the `.ll`'s size by set and N on the debug runtime again
+on the tree this ADR is merged with (the merged tree), whose code is
+that of `36f74a9a` (it differs from it only in this ADR and two doc
+comments moved in `crates/nova-driver/src/lib.rs`), and whose
+`selfhost/` is as at `8cb1b42b`. Scripts and logs are outside the
+repository, not committed.
+
+**Agreement,** S0's `check --dump-mir --ll` against N's `mir --ll`,
+byte for byte with exit codes, per root (part 2):
+
+| Set | Roots | Print an `(Ll` part | Agree | Differ |
+|---|---:|---:|---:|---:|
+| native corpus's roots that run (N's own included) | 261 | 210 | 261 | 0 |
+| HIR cases that reach MIR | 342 | 341 | 342 | 0 |
+| `cases/mir/` | 103 | 82 | 103 | 0 |
+| the MIR fuzzer's inputs | 120 | 120 | 120 | 0 |
+| the depth cases | 25 | 25 | 25 | 0 |
+
+The cases again with `\r\n` agree too (`SH`'s test). No frozen list.
+On a 1 MB main thread (release N, `ulimit -s 1024`), all 851 agree, N's
+root included.
+
+**The `.ll`'s size,** S0, per set, on the merged tree, from the
+`--nocapture` lines of the coverage test, the fuzzer's test and the
+depth test. The
+corpus's and the cases' rows are the coverage test's count, each root's
+`(Ll ok)` line with its `.ll`; the fuzzer's and the depth cases' are
+their tests' count, each root's `== FILE` header too:
+
+| Set | Roots | Lines | Bytes |
+|---|---:|---:|---:|
+| loose corpus | 202 | 748,170 | 23,225,549 |
+| packages' mains but N's | 7 | 128,620 | 3,787,437 |
+| N's `selfhost/src/main.nova` | 1 | 390,750 | 12,252,987 |
+| HIR cases | 341 | 71,478 | 2,469,445 |
+| `cases/mir/` | 82 | 65,547 | 2,056,287 |
+| the MIR fuzzer | 120 | 462,319 | 14,555,389 |
+| the depth cases | 25 | 120,531 | 3,617,081 |
+| all | 778 | 1,987,415 | 61,964,175 |
+
+Part 1 measured the same rows at `f3b32068`, before N held its own
+emitter: N's root then gave 379,439 lines and 11,842,977 bytes, and all
+1,976,104 and 61,554,165; every other row was as above. The spec
+projected about 62 MB (the fuzzer's about 14, the depth cases' about
+4.5).
+
+**S0's after form without and with `--ll`,** one process per set,
+median wall time in seconds (part 1; the sets as the script took them:
+`cases/mir/`'s 98 files, its package cases left out; 9 package mains):
+
+| Set | opt-level 0 | opt-level 1 |
+|---|---|---|
+| loose corpus (251) | 4.64 → 4.92 | 1.77 → 1.88 |
+| packages' mains but N's (9) | 0.31 → 0.35 | 0.14 → 0.17 |
+| N's root | 1.13 → 1.25 | 0.53 → 0.59 |
+| HIR cases (341) | 5.55 → 5.71 | 2.41 → 2.54 |
+| `cases/mir/` (98) | 1.64 → 1.70 | 0.75 → 0.80 |
+| the fuzzer (120) | 2.10 → 2.30 | 0.90 → 1.02 |
+| the depth cases (25) | 2.30 → 2.35 | 0.97 → 0.96 |
+| all | 17.67 → 18.60 (+0.92) | 7.47 → 7.95 (+0.48) |
+
+**N on the release runtime, without and with `--ll`** (part 2), one
+process per set: median wall of three, peak RSS the largest `VmHWM` of
+the three, polled; collections and the largest live heap from one
+`NOVA_GC_DEBUG` run. The loose and package sets are split by file name
+here (245 and 15), not as the harness splits them (251 and 10):
+
+| Set | Wall | Peak RSS | Collections | Largest live heap | stdout |
+|---|---|---|---|---|---|
+| N's root | 1.93 → 2.35 s | **211 → 303 MB** | 23 → 24 | 101 → 102 MB | 5.49 → 17.74 MB |
+| loose corpus (245) | 3.98 → 4.60 s | 41 → 39 MB | 102 → 120 | 11 → 12 MB | 8.74 → 29.77 MB |
+| package mains (15) | 0.52 → 0.70 s | 44 → 48 MB | 21 → 25 | 13 → 15 MB | 2.71 → 8.69 MB |
+| HIR cases (341) | 4.89 → 4.99 s | 39 → 38 MB | 115 → 120 | 11 → 11 MB | 0.48 → 2.94 MB |
+| `cases/mir/` (103) | 1.45 → 1.50 s | 48 → 49 MB | 44 → 45 | 14 → 15 MB | 0.76 → 2.82 MB |
+| the fuzzer (120) | 2.10 → 2.56 s | 43 → 41 MB | 60 → 76 | 13 → 12 MB | 5.86 → 20.41 MB |
+| the depth cases (25) | 1.23 → 1.34 s | 40 → 44 MB | 42 → 45 | 12 → 13 MB | 1.89 → 5.50 MB |
+
+**N's root rises by 92 MB of RSS** (211 to 303 MB), against the spec's
+projected +40 to 60 MB on about 207 MB: the 12.25 MB `.ll` of N's root
+with its own emitter (12,252,987 bytes; stdout 5.49 to 17.74 MB) is
+built whole, as parts and then one joined string, and printed beside
+both modules. No threshold applies; if CI's memory asks for it,
+printing the `.ll` as it is made (spec §5.2), the same bytes, is the
+fallback.
+
+**N on the debug runtime, without and with `--ll`,** on the merged
+tree, by the same method (the loose corpus as the harness lists it, 251 files):
+
+| Set | Wall | Peak RSS | Collections | Largest live heap | stdout |
+|---|---|---|---|---|---|
+| N's root | 5.82 → 7.66 s | **212 → 320 MB** | 23 → 24 | 102 → 148 MB | 5.49 → 17.74 MB |
+| loose corpus (251) | 13.66 → 16.45 s | 42 → 44 MB | 103 → 122 | 12 → 14 MB | 9.71 → 32.94 MB |
+| the fuzzer (120) | 7.20 → 8.82 s | 44 → 40 MB | 60 → 73 | 12 → 12 MB | 5.86 → 20.41 MB |
+| the depth cases (25) | 4.41 → 4.80 s | 41 → 48 MB | 42 → 45 | 12 → 15 MB | 1.89 → 5.50 MB |
+
+On the debug runtime N's root rises by 108 MB of RSS, and its largest
+live heap by 46 MB; the release runtime does not show that jump in the
+live heap (101 to 102 MB, above). The same run on the release runtime on
+the merged tree gave N's root's row as above: 211 → 303 MB, 23 → 24
+collections, 101 → 102 MB.
+
+**`selfhost.rs`,** each comparison interleaved in one session (sessions
+on this machine differ by up to a factor of two, so only figures of one
+session compare):
+
+| Step | Style | Before | After |
+|---|---|---|---|
+| part 1 (S0 prints the `.ll`, N not yet), against the base's harness with the new cases set aside | plain | 88.05 s | 91.95, 92.69 s |
+| part 1 | as CI runs it | 54.04, 54.23, 55.22 s | 53.75, 53.26, 53.84 s |
+| part 2 (N prints and the `.ll` is compared), against part 1's head `f3b32068` | plain | 215.82, 212.35, 216.69 s | 212.14, 210.80, 202.34 s |
+| part 2 | as CI runs it | 50.74, 51.19, 53.50 s | 52.43, 52.57, 54.39 s |
+| part 2, the eight MIR tests alone | as CI runs it | 19.36, 18.46 s | 20.16, 19.49 s |
+
+"As CI runs it" is `NOVA_EMBED_RUNTIME=1
+CARGO_PROFILE_DEV_OPT_LEVEL=1`. Part 1 adds nothing measurable as CI
+runs it, and about 4 s plain against the one interleaved run before;
+part 2 adds about +1.4 s to the median as CI runs it, about +0.9 s of it
+in the MIR tests, and nothing measurable plain, where the HIR dump's
+package parts are the longest processes. The tests: 64 at the base, 66
+after part 1 (the coverage test and the `build --release` test), 67
+after part 2 (the report's test). The spec projected +1 to 2 s of local
+wall.
+
+**Verification:**
+- the full suite (`cargo test --workspace`, Node required): 2,837
+  passed, 1 failed (the IPv6 test, as on the base), 10 ignored, against
+  the base's 2,835, 1 and 10; the two new tests are the driver's;
+- `SH` passes 67; N's `nova test` 108 passed, and N's tests on the debug
+  runtime pass;
+- `cargo fmt --all -- --check`, both clippy commands as CI runs them,
+  `nova fmt --check std examples selfhost` and Rust 1.78's `cargo check
+  --locked --workspace` with `-D warnings` are clean; `Cargo.lock` and
+  the manifests are unchanged;
+- **byte for byte with the base's `nova`** (`db640e97`), by part 1's
+  scripts after part 2: `nova check` 2,045 checks; `nova parse --check`
+  and `--ast` 3,876 runs over 1,938 files; `nova test` and `nova build`
+  in each of the 11 packages outside `crates/` (whose packages are test
+  fixtures, not compared), 22 runs; every `--help`, 22 of 22, no hidden
+  flag shown; usage errors 558 runs under six `argv[0]`s and 10 in a
+  terminal; `--dump-ast` as `--dump-tokens` 25 of 25; every refusal of
+  the hidden forms (114 of `--dump-resolution`'s, 522 of `--dump-hir`'s
+  and `--dump-mir`'s) one error, exit 2, nothing on stdout; and
+  `--dump-mir` without `--ll`, in both forms, on 1,292 roots in 26
+  batches (part 1's script, run after part 2). On the merged tree, by
+  the plan's script, `--dump-mir` without `--ll` in both forms on 1,955
+  roots (the 1,938 `.nova` files the base tracks and the 17 new cases)
+  in 40 batches: 0 differ. None differs.
+
+### 12. N's least stack on the depth cases, with `--ll` (part 3)
+
+The least main-thread stack on which N's `selfhost mir --std DIR [--ll]
+FILE` exits 0 and prints S0's part (`check --dump-mir [--ll] FILE`), on
+each of the 25 depth cases (ADR 0057 §17), by ADR 0057 §17's method:
+Linux, `ulimit -s`, 4 KB bisection, each run's output compared with
+S0's, 2026-11-18, at `8cb1b42b`. N was built from that tree by S0's
+`nova build` twice: against the debug runtime beside S0, and against
+the release runtime CI's S0 embeds (`NOVA_RUNTIME_LIB` set to its
+extracted copy); both binaries are byte for byte part 2's. Two rounds,
+each form of each runtime in turn; a cell gives the two rounds, in KB.
+The last column is ADR 0057 §17's, without `--ll`, 2026-11-14:
+
+| Input | Levels | release, without | release, `--ll` | debug, without | debug, `--ll` | ADR 0057 §17, release / debug |
+|---|---:|---:|---:|---:|---:|---:|
+| body `annotations` | 128 | 64, 60 | 56, 64 | 64, 60 | 60, 60 | 56 / 60 |
+| body `arrays` | 128 | 200, 196 | 200, 200 | 200, 200 | 204, 204 | 196 / 196 |
+| body `closure-captures` | 64 | 200, 192 | 196, 196 | 196, 204 | 200, 204 | 196 / 196 |
+| body `closure-params` | 64 | 200, 196 | 200, 196 | 208, 200 | 208, 204 | 200 / 204 |
+| body `closures` | 64 | 196, 200 | 196, 200 | 200, 200 | 200, 200 | 200 / 200 |
+| body `for-arrays` | 127 | 208, 208 | 208, 208 | 208, 208 | 212, 216 | 212 / 212 |
+| body `for-patterns` | 127 | 212, 212 | 212, 212 | 212, 212 | 216, 212 | 204 / 216 |
+| body `for-ranges` | 127 | 208, 208 | 208, 212 | 212, 216 | 212, 208 | 212 / 212 |
+| body `indexes` | 128 | 200, 196 | 196, 200 | 204, 204 | 204, 204 | 200 / 200 |
+| body `interpolations` | 128 | 200, 196 | 196, 200 | 200, 196 | 196, 200 | 200 / 196 |
+| body `let-patterns` | 128 | 192, 196 | 192, 200 | 196, 200 | 196, 200 | 196 / 200 |
+| body `match-arms` | 128 | 208, 208 | 204, 208 | 216, 208 | 208, 212 | 204 / 208 |
+| body `match-scrutinees` | 128 | 196, 200 | 196, 200 | 200, 204 | 200, 200 | 200 / 204 |
+| body `method-chains` | 128 | 48, 52 | 48, 48 | 56, 56 | 56, 56 | 48 / 56 |
+| body `or-patterns` | 128 | 52, 52 | 56, 56 | 56, 56 | 60, 56 | 56 / 56 |
+| body `questions` | 128 | 196, 200 | 200, 196 | 200, 200 | 200, 200 | 200 / 200 |
+| body `tuple-patterns` | 127 | 196, 200 | 200, 192 | 200, 200 | 204, 200 | 196 / 200 |
+| body `tuples` | 128 | 196, 196 | 200, 196 | 200, 200 | 200, 200 | 200 / 196 |
+| body `variant-patterns` | 127 | 196, 200 | 196, 196 | 200, 196 | 200, 196 | 196 / 196 |
+| type argument, `arrays` | 128 | 44, 52 | 52, 48 | 52, 48 | 52, 56 | 48 / 52 |
+| type argument, `fn-parameters` | 128 | 52, 52 | 52, 48 | 56, 56 | 56, 56 | 52 / 52 |
+| type argument, `fn-returns` | 128 | 52, 52 | 52, 52 | 48, 48 | 56, 48 | 48 / 56 |
+| type argument, `generics` | 128 | 60, 60 | 64, 56 | 64, 60 | 64, 60 | 64 / 60 |
+| type argument, `options` | 128 | 56, 60 | 60, 60 | 64, 60 | 56, 60 | 60 / 64 |
+| type argument, `tuples` | 128 | 48, 52 | 52, 52 | 52, 52 | 52, 56 | 52 / 48 |
+
+**The emitter adds nothing measurable to N's stack.** With `--ll` the
+least stack is 48 to 212 KB on the release runtime and 48 to 216 KB on
+the debug one; without it, 44 to 212 KB and 48 to 216 KB. On every
+input, with `--ll` and without differ by at most 8 KB (two bisection
+steps), as two rounds of one form do, and today's figures without
+`--ll` are within 8 KB of ADR 0057 §17's. The emitter does not recurse
+on nesting, and the deep types' rows, whose symbols are 262 to 520
+characters, need no more with it. **No case needs more than the 1 MB
+the harness gives:** the most is 216 KB, so N has about 4.7 times the
+room it needs, and 5.3b's rule (record the depth N survives as
+(differs)) is not needed. The test's 96 KB check (§7) still fails
+`bodies-for-ranges-127.nova`, so the stack is the one set.
+
+### 13. CI's figures
+
+The prototype at `8cb1b42b` went to
+`claude/loving-archimedes-frtf5q-55-ci` with the user's word, as a
+draft pull request (#144), measurement only, closed unmerged, as #142.
+Its run, 44259975902 (the emitter and harness alone), failed the three
+Selfhost jobs (`ubuntu-24.04`, `macos-latest`, `windows-latest`) at
+their `cargo test --test selfhost` step, every other job green, on one
+test: `the_ll_dump_is_what_build_release_writes` read the float `%`
+case's stderr, which under CI's `RUST_BACKTRACE=1` ends in `anyhow`'s
+backtrace, not in the error's words. The test now runs `nova build
+--release` with `RUST_BACKTRACE=0` and `RUST_LIB_BACKTRACE=0`;
+reproduced and fixed locally with `RUST_BACKTRACE=1` set. That fix,
+`36f74a9a`, with the records' commit `3c6f1a07` before it, was then
+pushed to the same branch, under the same draft pull request; its run,
+44260509656, passed all 19 jobs.
+
+`selfhost.rs`'s main step, `finished in`, against the lines (ADR 0057
+§19) and two runs of `main` at 5.4's merge:
+
+| System | Lines | 5.5a (run 44260509656) | `main`, run 43853141585 | 5.4's pull request, run 43850851663 |
+|---|---:|---:|---:|---:|
+| Linux | 80 s | 51.39 s (67 tests) | 77.03 s (64) | 71.98 s |
+| macOS | 100 s | 29.94 s (65) | 31.72 s (62) | 89.34 s |
+| Windows | 110 s | 93.86 s (61) | 79.94 s (58) | 96.77 s |
+
+All three are under their lines. The runners' spread between runs of
+the same tree is larger than 5.5a's cost: `main`'s own tree measured
+71.98 and 77.03 s on Linux and 31.72 and 89.34 s on macOS, so one run
+cannot show 5.5a's increment on CI. Locally, as CI runs it, the
+increment is about +1.4 s (section 11). The Test job's main step summed
+2838 passed, 0 failed, 10 ignored on Linux, 2838/0/9 on macOS and
+2815/0/9 on Windows, two more than 5.4's (the driver's new unit tests).
+The advisory `--ignored` step failed on Linux (`extern_ffi_run`) and on
+macOS (three GC scan tests), as that step is allowed to (it runs with
+`continue-on-error`); no `.ll` test is among them. N's memory on its
+root on CI's runners is not reported by the job.
+
+### 14. 5.5b's questions, deferred
+
+5.5 splits (the user's decision 1): 5.5b is the driver, `clang` and
+`cc`, finding the runtime, N's `build` and `check`, and the golden
+suite built by S1. Its questions are asked when 5.5b starts, not
+decided here (spec §1, research §11 items 3 to 8,
+`docs/phase-5-plan.md` §5.5's "Questions for its spec"):
+1. where the S1 golden suite runs (the Selfhost job's Ubuntu leg with
+   `clang-18`, a job of its own, or the Test job), which runtime its
+   programs link, and whether S1 is built by Cranelift or `--release`;
+2. the intermediate file's name, the tolerance: one `-o` basename for
+   every stage, a constant name, or a `source_filename` line in both
+   emitters;
+3. N's command line and exit codes, how N finds std and the runtime,
+   project mode without `current_dir`, and whether stderr is compared;
+4. `check`'s output: codespan's layout ported, or codes and spans in
+   the dumps' form (ADR 0048 decision 1);
+5. N's `build` without `--release` (N has no Cranelift): `-O2` or
+   `-O0`;
+6. platforms: Windows `link.exe` discovery is not portable; N's `build`
+   on Linux and macOS only, or an OS query in std, in both compilers.
+
+Also left out of 5.5a (spec §15): WASM's and test mode's code
+generation, and Cranelift; optimizing the `.ll` or changing its text
+(no `source_filename`, triple or datalayout); fixing float `%` in S0
+(the user's decision 4); a runtime change for N's speed, memory or
+stack, unless the user rules.
+
+### 15. Departures from the spec, and decisions it does not settle
+
+**From the spec,** each recorded where it falls:
+- **`ir_tests.rs` gives 14 cases, not about 18** (§6): 20 sources, of
+  which both hellos, `float_bits` and three `lower_tests__` duplicates
+  were dropped.
+- **clap's usage line for `--ll=x` names the flags used**,
+  `Usage: nova check --dump-mir --ll [FILES]...`, as the base's
+  `--before-async=1` does; spec §3.1 gives `[OPTIONS] [FILES]...`,
+  which `--ll` twice prints (§1).
+- **§3.3's package is `examples/05-geometry`**, a package with a path
+  dependency: `examples/01-hello-world` has no `nova.toml` (§3).
+- **N's root rises by 92 MB of RSS** (211 to 303 MB), not the
+  projected +40 to 60 MB (§11). Measured, recorded; the fallback stays
+  unbuilt (spec §5.2).
+- **The `ret_of` mutant is killed by N's `@test` alone** (§10): spec
+  §6.4 expects each mutant to fail the differential, which no native
+  input can for this one; spec §14 risk 5 foresaw it.
+- **The spec's 407 cases that lower** counted `cases/mir/` by the
+  research's list (66 of 92 roots). By the harness's lists, the base's
+  86 roots of `cases/mir/` give 65, so 406 with the HIR cases' 341; with
+  the 17 new, 423 (§6).
+
+**Decisions the spec does not settle,** each with its cost if wrong
+(all cheap):
+- Part 1's switch turned on when S0's arguments held `--ll` and N's did
+  not, rather than reading `N_LL`; part 2 removed both.
+- `check_ll` runs whatever the switch said, so S0's `.ll` was checked
+  in part 1 already.
+- 5.4's refusal shapes keep plain `MIR_S0` on S0's side.
+- `nova_rt_str_new` is not among the coverage's calls (§8).
+- A root whose `.ll` is an error adds only its error's form to the
+  coverage; each error is named through `LL_ERRORS`, an unknown one
+  failing the test.
+- The coverage test is a `#[test]` of its own, sharing its run with the
+  corpus test through `OnceLock`s.
+- N writes the header first into the one builder (§4); `RtTable` is
+  read once per module; `escape_bytes` writes onto the builder and the
+  empty string's `\00` is a case at the global; the signature table
+  holds strings; `ll_part` lives in `main.nova` beside `mir_of`.
+- The `.ll` is built whole on both sides; nothing yet asks for printing
+  it as it is made.
+
+### 16. The rule
+
+From 5.5 the `.ll` emitter exists twice, in `nova-codegen-llvm` and in
+`selfhost/`. A change to either changes both in one pull request: the
+emitter's text (the header, the declarations, the instructions, the
+names, the escapes, the float spelling, the globals, the C `main`), its
+errors' words, `RtFunc`'s symbols, signatures or order in `RtFunc::ALL`,
+and the runtime's ABI as the emitter spells it (symbols, classes, slot
+sizes, the layouts of sums, records, arrays and closures, the env-first
+indirect call). The `.ll` differential (§6, §8, §12) holds them equal
+(phase plan decision 6; ADR 0057 §21 extended, which already names a
+runtime function's name or place in `RtFunc::ALL`). The coverage
+test's lists (`LL_STATEMENTS`, `LL_BINS`, `LL_ERRORS`,
+`LL_UNREACHABLE`) change with any new statement, operator or error, and
+with a runtime function no input calls. WASM's and test mode's code
+generation stay S0's alone.
+
+## Consequences
+
+- N's front end now reaches textual LLVM IR, the same bytes as S0's on
+  every input the differential holds, with no `clang`, on Linux, macOS
+  and Windows (§13). 5.5b links it.
+- The MIR dump's after form prints about 62 MB more over every input;
+  locally, as CI runs it, `selfhost.rs` grew about 1.4 s (§11); on CI
+  it stayed under each line, where one run cannot show the increment
+  within the runners' spread (§13).
+- N's root peaks at about 303 MB of RSS with its `.ll`, 92 MB more than
+  without (§11).
+- N lowers and emits the depth cases on 1 MB with about 4.7 times the
+  room it needs (§12). ADR 0055 §12's normalized type is still the one
+  input recorded as (differs) for N's stack.
+- Float `%` still type-checks and fails in `build --release`; its case
+  shows both sides agree on the error (§5).
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index 4aaa2e5e..4cc354f8 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -588,6 +588,41 @@ mir`, std's checked bodies spliced into each program on both sides.
 so the driver's item narrows to what 5.2 leaves: the command line and
 its exit codes, finding std next to the executable or in `$NOVA_HOME`,
 `clang` and `cc`, finding `libnova_runtime.a`, and linking.
+
+**Amended 2026-11-18 (5.5a, branch `claude/loving-archimedes-frtf5q-55`,
+ADR 0058; spec
+`docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`
+§1):** 5.5 is split in two, as the user decided on 2026-11-17.
+- **5.5a, the `.ll` emitter:** `RtFunc::symbol` and `signature` and
+  `nova-codegen-llvm`'s `compile_ir` ported (`selfhost/src/emit_ll.nova`),
+  N still built by S0's `nova build` (Cranelift), nothing needing
+  `clang`. **The public `--emit-ll` below becomes the hidden `--ll` of
+  `nova check --dump-mir`,** which prints each lowered root's `.ll` after
+  its MIR after form, in the same process, refused with
+  `--before-async`; N's side is `selfhost mir --ll`. Every `--help`
+  stays the base's.
+- **5.5a's gate,** in place of the first item of the gate below: per
+  host on Linux, macOS and Windows, N's `.ll` equals S0's byte for byte,
+  with exit codes, on the corpus roots that lower (N's own among them),
+  the cases that lower, `ir_tests.rs`'s sources as 14 cases, the float
+  `%` and hand cases, MIR's cases again with `\r\n`, the MIR fuzzer's
+  120 inputs and the 25 depth cases, which N runs on a 1 MB stack; the
+  emitter's one reachable error, float `%`, agrees in the dump, and its
+  unreachable errors are pinned by tests on both sides; every form the
+  emitter handles occurs in the compared `.ll` or is listed unreachable
+  with its reason; the dump's `.ll` equals `build --release`'s
+  intermediate; the `.ll` mismatch report names the `define` and the
+  block; CI's `selfhost.rs` is within the Selfhost job's lines, or the
+  user has ruled.
+- **5.5b, the driver and linking:** `clang` and `cc`, finding the
+  runtime, N's `build` and `check`, and the golden suite built by S1,
+  the gate's second item below. Its questions, the three below among
+  them, are deferred to its spec, not decided in 5.5a (ADR 0058 §14):
+  where the S1 golden suite runs and on which runtime, the intermediate
+  file's name, N's command line and exit codes and how it finds std and
+  the runtime, `check`'s output, N's `build` without `--release`, and
+  Windows linking.
+
 - **The emitter,** a port of `nova-codegen-llvm`, and `--emit-ll`.
 - **The driver:** the module loader, `clang` and `cc` through 5.0b's
   spawn, finding `libnova_runtime.a` as `link.rs` does, and the commands
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index a5a82d06..da8c207c 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -855,6 +855,25 @@ corpus programs) and programs nested 128 levels deep, which N lowers on
 a 1 MB stack. WASM's and test mode's MIR stay S0's. From here a change
 to MIR changes both compilers in one pull request.
 
+**Amended 2026-11-18 (branch `claude/loving-archimedes-frtf5q-55`, ADR
+0058):** step 5's backend is the textual LLVM IR of ADR 0048, not
+bindings, and it is ported in two parts
+(`docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`):
+5.5a, the `.ll` emitter, and 5.5b, the driver, linking and the golden
+suite built by stage1. 5.5a: `selfhost/` emits a program's LLVM IR from
+its native MIR as `nova-codegen-llvm` does, with `RtFunc`'s symbols and
+signatures and the emitter's errors in S0's words, and `selfhost mir
+--ll` prints each lowered root's `.ll` after its MIR, as S0's hidden
+`nova check --dump-mir --ll` does. The differential compares them byte
+for byte wherever it compares the MIR after the async transform: the
+corpus roots that run, the HIR cases that reach MIR, MIR's cases with
+both line endings (`ir_tests.rs`'s sources and a float `%` case among
+them), the MIR fuzzer's programs and programs nested 128 levels deep,
+which N emits on a 1 MB stack. No `clang` is needed: N is still built by
+S0's Cranelift backend. From here a change to the emitter, to
+`RtFunc`'s symbols or signatures, or to the runtime's ABI as the emitter
+spells it changes both compilers in one pull request.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/nova-spec/14-CODEGEN.md b/nova-spec/14-CODEGEN.md
index ac513499..f696d3e5 100644
--- a/nova-spec/14-CODEGEN.md
+++ b/nova-spec/14-CODEGEN.md
@@ -72,6 +72,21 @@ LLVM backend's parity is now tested: every golden program of
 §3.2; ADR 0048, which also makes textual `.ll` through `clang` the backend
 of Phase 5's compiler). The hello-world test stays.
 
+**Amended 2026-11-18 (branch `claude/loving-archimedes-frtf5q-55`, ADR
+0058):** the `.ll` emitter exists twice, in `nova-codegen-llvm` and in
+the self-hosted compiler (`selfhost/src/emit_ll.nova`), with
+`RtFunc`'s symbols and signatures. The hidden `nova check --dump-mir
+--ll FILE…` prints, after each lowered root's MIR after the async
+transform, `(Ll ok)` and `compile_ir`'s text of that module, byte for
+byte the intermediate `<out>.nova.ll` that `nova build --release`
+writes, or one line `(Ll error "…")` with the emitter's words; it is in
+no `--help`. The self-hosted compiler's `selfhost mir --ll` prints the
+same, byte for byte, on every input the differential holds. The text
+names no triple, datalayout or path. A float `%` still
+type-checks and is still refused by the emitter, an internal error in
+`build --release`. WASM's and test mode's code generation stay the Rust
+compiler's alone.
+
 ### 2.3 WASM (browser target)
 - Crates: `wasm-encoder` (write modules), `walrus` (manipulate)
 - Target: `nova build --target wasm` and `nova bundle`
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
  target/debug/nova fmt --check std examples selfhost
  target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost
  cargo test --locked --workspace --no-run
  S --lib --bins > "$P/exec55/full-55-a.txt" 2>&1
  S --doc > "$P/exec55/full-55-b.txt" 2>&1
  S --test '[!s]*' --test 's[!e]*' > "$P/exec55/full-55-c.txt" 2>&1
  cat "$P"/exec55/full-55-[abc].txt > "$P/exec55/full-55.txt"
  grep '^test result' "$P/exec55/full-55.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec55/full-55.txt"
  grep -c 'Running tests/selfhost.rs' "$P/exec55/full-55.txt"
  cargo test --locked --workspace -- --list > "$P/exec55/final-list-55.txt" 2>&1
  cargo test --locked --workspace --test selfhost -- --list > "$P/exec55/final-list-sh-55.txt" 2>&1
  python3 "$P/exec55/names.py" "$P/base-list-55.txt" "$P/exec55/final-list-55.txt"
  comm -3 <(grep ': test$' "$P/base-list-sh-55.txt" | sort) <(grep ': test$' "$P/exec55/final-list-sh-55.txt" | sort)
  ```

  `names.py` compares Task 0's list with this one binary by binary, by
  name: a binary is its `Running` or `Doc-tests` line, its hash stripped,
  an integration test's prefixed with the crate whose unit tests cargo
  listed just before it; a test is a line ending `: test`. Write it
  before the run, as the replay ran it:

```sh
mkdir -p "$(dirname "$P/exec55/names.py")"
cat > "$P/exec55/names.py" <<'EOF'
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
  - the formatters print nothing, and both clippy commands end with
    `Finished`, with no warning or error;
  - the suite, in three runs as the conventions give them (99, 1 and 166
    s): `1300 1 8`, `2 0 0` and `1535 0 2`, `2837 1 10` summed; the one
    failure is Task 0's IPv6 test; `0` binaries named
    `tests/selfhost.rs`;
  - `names.py` prints `binaries: base 111, now 111`, `tests: base 2846,
    now 2848`, and `net added: 2`: `nova_driver (unittests src/lib.rs):
    42 -> 44, +2 -0`,
    `ll_dumps::the_ll_dump_is_the_module_build_release_emits` and
    `ll_dumps::the_ll_part_is_the_emitters_text_or_its_words_quoted`;
  - `comm` prints `selfhost.rs`'s three added tests, 64 to 67:
    `every_form_the_emitter_handles_occurs_in_the_compared_ll`,
    `the_ll_dump_is_what_build_release_writes` and
    `the_ll_mismatch_report_names_the_define_and_block`; none gone.
- [ ] **Step 2: CI's Linux leg, as the Test job runs it.** The `test`
  job's `ubuntu-24.04` leg runs the whole suite with the golden suite
  required, with CI's own flags, and with `RUST_BACKTRACE=1` set for each
  suite command alone, as `.github/workflows/ci.yml` exports it for
  every job (decision 9's lesson). The suite runs in the conventions'
  three parts, so that each fits 600 s at twice the replay's time; `CI`
  is a function of this step's shell:

  ```sh
  CI() { RUST_BACKTRACE=1 NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast "$@"; }
  cargo test --locked --workspace --all-features --no-run
  time CI --lib --bins > "$P/exec55/ci-55-a.txt" 2>&1
  time CI --doc > "$P/exec55/ci-55-b.txt" 2>&1
  time CI --test '[!s]*' --test 's[!e]*' > "$P/exec55/ci-55-c.txt" 2>&1
  cat "$P"/exec55/ci-55-[abc].txt > "$P/exec55/ci-55.txt"
  grep '^test result' "$P/exec55/ci-55.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec55/ci-55.txt"
  grep -c 'release_golden_suite ... ok' "$P/exec55/ci-55.txt"
  grep -c 'Running tests/selfhost.rs' "$P/exec55/ci-55.txt"
  ```

  Expected, as the replay ran it at the last commit: `1300 1 8`, `2 0 0`
  and `1535 0 2` in 142, 2 and 248 s; `2837 1 10` summed, the one failure
  the IPv6 test, `1` (the golden suite ran and passed inside the third
  part) and `0`. (As one command it took 308 s, and 284 s without
  `RUST_BACKTRACE=1`.) If a part is killed at its timeout, or the shell
  restarts, run that part again alone, then the parts after it; never
  sum a part that did not end with its `test result` lines.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in 19 s of wall
  time: `release golden: 147 passed, 9 excluded, Ubuntu clang version
  18.1.3 (1ubuntu1)` and `1 passed`: unchanged, since 5.5a adds no
  runtime program and changes neither lowering nor S0's emitter.
- [ ] **Step 4: Parsing, checking and dumping every file the base
  tracks.** Four scripts, as the replay ran them, compare the base's
  `nova` (Task 0's copy, `$P/nova-base-55/nova`) with the tree's on every
  `.nova` file `git ls-tree -r --name-only db640e97` lists, each run's
  output, both streams, and exit status compared byte for byte. 5.5a
  changes neither the lexer, the parser, the resolver, the type checker
  nor the MIR dump without `--ll`, and no path a user takes reaches the
  `.ll` dump: nothing may differ. All run from the repository's root
  after the build, and read `$P`.
  - `parsecmp.sh`:

```sh
mkdir -p "$(dirname "$P/exec55/parsecmp.sh")"
cat > "$P/exec55/parsecmp.sh" <<'EOF'
#!/bin/bash
# parsecmp.sh: run from the repository's root, after the build, with $P exported.
# For every `.nova` file `git ls-tree -r --name-only db640e97` lists (the
# base's tracked files, so none 5.5a adds), `nova parse --check` and
# `nova parse --ast`, by the base's `nova` ($P/nova-base-55/nova, Task 0) and
# by the tree's `target/debug/nova`, both on the file as db640e97 has it,
# from a `git archive db640e97` unpacked in $P/pc-src-55: 5.5a adds two
# files to N's sources and changes two, which the base's text keeps out of
# the comparison. Each run passes the path relative to that root and
# keeps its output, both streams, and its exit status; every pair must be
# byte-identical, since 5.5a changes neither the lexer nor the parser.
# Prints each file that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
src="$P/pc-src-55"
rm -rf "$src"; mkdir -p "$src"
git archive db640e97 | tar -x -C "$src"
n=0; differ=0; nz_base=0; nz_new=0
while read -r f; do
  for mode in --check --ast; do
    a=$( (cd "$src" && "$P/nova-base-55/nova" parse $mode "$f") 2>&1; echo "exit=$?")
    b=$( (cd "$src" && "$nova" parse $mode "$f") 2>&1; echo "exit=$?")
    n=$((n + 1))
    if [ "$mode" = --check ]; then
      [ "$(tail -1 <<< "$a")" != exit=0 ] && nz_base=$((nz_base + 1))
      [ "$(tail -1 <<< "$b")" != exit=0 ] && nz_new=$((nz_new + 1))
    fi
    if [ "$a" != "$b" ]; then differ=$((differ + 1)); echo "differs: $mode $f"; fi
  done
done < <(git ls-tree -r --name-only db640e97 | grep '\.nova$')
echo "parse: $n runs over $((n / 2)) files, $differ differ; --check exits nonzero on $nz_base at the base, $nz_new now"
rm -rf "$src"
[ "$differ" = 0 ]
EOF
```

  - `checkcmp.sh`, which keeps each differing pair under
    `$P/exec55/differ/`, and runs in two halves, each under 600 s with
    half again:

```sh
mkdir -p "$(dirname "$P/exec55/checkcmp.sh")"
cat > "$P/exec55/checkcmp.sh" <<'EOF'
#!/bin/bash
# checkcmp.sh (5.5a, from 5.4's): run from the repository's root, after the build, with $P exported.
# `nova check` by the base's `nova` ($P/nova-base-55/nova, Task 0) and by the
# tree's `target/debug/nova`, both on the tree's files, for every `.nova`
# file `git ls-tree -r --name-only db640e97` lists. A file's package root
# is the nearest directory above it, in the tree, that holds a
# `nova.toml`, stopping below the repository's root: a file under one is
# checked as its package, once per package, by `nova check` run in the
# root with no argument; any other file alone, by `nova check FILE` from
# the repository's root. Each check keeps both streams and the exit
# status, and the pair must be byte-identical. Prints each that differs,
# with both sides' diagnostic codes, then the counts. `checkcmp.sh K N` (since 5.4)
# takes the K-th of N parts of the files, every N-th from the K-th, each
# package's own check in the part of its first file, so that the parts'
# counts add up to the whole run's; with no argument, all of them.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
K=${1:-1}; N=${2:-1}
OUT="$P/exec55/differ"; [ "$K" = 1 ] && rm -rf "$OUT"; mkdir -p "$OUT"
save() { k=$(echo "$1" | tr "/ " "__"); printf "%s" "$2" > "$OUT/$k.a"; printf "%s" "$3" > "$OUT/$k.b"; }
n=0; differ=0; stddiff=0; i=0
declare -A seen
codes() { grep -oE '^(error|warning)\[[A-Z0-9]+\]: .*' <<< "$1" | sort | uniq -c; }
while read -r f; do
  i=$((i + 1)); mine=$(( (i - 1) % N == K - 1 ))
  dir=$(dirname "$f"); pkg=""
  while [ "$dir" != "." ] && [ "$dir" != "/" ]; do
    if [ -f "$dir/nova.toml" ]; then pkg=$dir; break; fi
    dir=$(dirname "$dir")
  done
  if [ -n "$pkg" ] && [ "$mine" = 0 ]; then
    seen[$pkg]=1
    continue
  fi
  [ "$mine" = 1 ] || continue
  if [ -n "$pkg" ]; then
    # Since 5.2b: each file of a package by `nova check FILE` too, from the
    # repository's root, as the dump takes a package's root.
    a=$("$P/nova-base-55/nova" check "$f" 2>&1; echo "exit=$?")
    b=$("$nova" check "$f" 2>&1; echo "exit=$?")
    n=$((n + 1))
    if [ "$a" != "$b" ]; then
      differ=$((differ + 1)); echo "differs: $f (nova check FILE)"; save "$f" "$a" "$b"
    fi
    [ -n "${seen[$pkg]:-}" ] && continue
    seen[$pkg]=1
    a=$( (cd "$pkg" && "$P/nova-base-55/nova" check) 2>&1; echo "exit=$?")
    b=$( (cd "$pkg" && "$nova" check) 2>&1; echo "exit=$?")
    what="$pkg/ (package)"
  else
    a=$("$P/nova-base-55/nova" check "$f" 2>&1; echo "exit=$?")
    b=$("$nova" check "$f" 2>&1; echo "exit=$?")
    what=$f
  fi
  n=$((n + 1))
  if [ "$a" != "$b" ]; then
    case $what in std/*) stddiff=$((stddiff + 1));; *) differ=$((differ + 1));; esac
    save "$what" "$a" "$b"
    echo "differs: $what"
    echo "  base: $(codes "$a" | tr -s ' ' | tr '\n' ';') $(tail -1 <<< "$a")"
    echo "  now:  $(codes "$b" | tr -s ' ' | tr '\n' ';') $(tail -1 <<< "$b")"
  fi
done < <(git ls-tree -r --name-only db640e97 | grep '\.nova$')
echo "check: $n checks, $differ outside std differ, $stddiff std files differ (part $K of $N)"
[ "$differ" = 0 ] && [ "$stddiff" = 0 ]
EOF
```

  - `pkgcmp.sh`:

```sh
mkdir -p "$(dirname "$P/exec55/pkgcmp.sh")"
cat > "$P/exec55/pkgcmp.sh" <<'EOF'
#!/bin/bash
# pkgcmp.sh: run from the repository's root, after the build, with $P exported. For each package of the
# base's tree outside crates/ (a directory holding a nova.toml, of `git ls-tree -r --name-only db640e97`:
# 11 packages, 22 runs; the 96 under crates/ are test fixtures, skipped by `grep -v '^crates/'`), a copy in a
# fresh directory for each side, then `nova test` and `nova build -o OUT` in it by the base's `nova`
# ($P/nova-base-55/nova) and by the tree's: both streams and the exit status must be byte-identical
# (the built program's bytes are not compared), both with the tree's runtime library, which 5.5a does not change. Prints each
# that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
nova="$PWD/target/debug/nova"; repo=$PWD
work=$P/exec55/pkgcmp; rm -rf "$work"; mkdir -p "$work"
n=0; differ=0
for m in $(git ls-tree -r --name-only db640e97 | grep '/nova.toml$' | grep -v '^crates/'); do
  pkg=$(dirname "$m")
  for side in base tree; do
    rm -rf "$work/$side"; mkdir -p "$work/$side"; cp -r "$pkg" "$work/$side/p"; rm -rf "$work/$side/p/target"
  done
  for cmd in test "build -o $work/OUT"; do
    a=$(cd "$work/base/p" && NOVA_RUNTIME_LIB=$repo/target/debug/libnova_runtime.a NOVA_HOME=$work/home-base "$P/nova-base-55/nova" $cmd 2>&1; echo "exit=$?")
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

  - `mircmp.sh`, `--dump-mir` without `--ll` in both forms:

```sh
mkdir -p "$(dirname "$P/exec55/mircmp.sh")"
cat > "$P/exec55/mircmp.sh" <<'EOF'
#!/bin/bash
# mircmp.sh (5.5a): run from the repository's root, after the build, with $P exported.
# `nova check --dump-mir` without `--ll`, in both forms (after the async
# transform, and with `--before-async`), by the base's `nova`
# ($P/nova-base-55/nova, Task 0) and by the tree's `target/debug/nova`, on
# every `.nova` file `git ls-tree -r --name-only db640e97` lists and every
# file the tree adds under `cases/mir/`, each a root, from the repository's
# root, in batches of 100 roots: both streams and the exit status of each
# batch must be byte-identical, since without `--ll` the dump is 5.4's.
# Prints each batch that differs, then the counts.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
nova="$PWD/target/debug/nova"
list="$P/exec55/mircmp-roots.txt"
{ git ls-tree -r --name-only db640e97 | grep '\.nova$'
  find crates/nova-cli/tests/selfhost/cases/mir -name '*.nova'; } | LC_ALL=C sort -u > "$list"
roots=$(wc -l < "$list")
n=0; differ=0; bytes=0
for form in "" --before-async; do
  i=0
  while :; do
    mapfile -t batch < <(tail -n +$((i * 100 + 1)) "$list" | head -100)
    [ "${#batch[@]}" = 0 ] && break
    a=$("$P/nova-base-55/nova" check --dump-mir $form "${batch[@]}" 2>&1; echo "exit=$?")
    b=$("$nova" check --dump-mir $form "${batch[@]}" 2>&1; echo "exit=$?")
    n=$((n + 1)); bytes=$((bytes + ${#a}))
    if [ "$a" != "$b" ]; then differ=$((differ + 1)); echo "differs: --dump-mir $form batch $i (${batch[0]} ...)"; diff <(echo "$a") <(echo "$b") | head -8; fi
    i=$((i + 1))
  done
done
rm -f "$list"
echo "--dump-mir: $roots roots in both forms, $n batches, $bytes characters at the base, $differ differ"
[ "$differ" = 0 ]
EOF
```

  Then `bash "$P/exec55/parsecmp.sh"`, `bash "$P/exec55/checkcmp.sh" 1
  2`, `bash "$P/exec55/checkcmp.sh" 2 2`, `bash "$P/exec55/pkgcmp.sh"`
  and `bash "$P/exec55/mircmp.sh"`. Expected, as the replay printed, in
  165, 199, 190, 41 and 76 s:
  - `parse: 3876 runs over 1938 files, 0 differ; --check exits nonzero on
    123 at the base, 123 now`;
  - `check: 1029 checks, 0 outside std differ, 0 std files differ (part 1
    of 2)` and `check: 1016 checks, 0 outside std differ, 0 std files
    differ (part 2 of 2)`: 2,045 in all;
  - `packages: 22 runs, 0 differ`: `nova test` and `nova build` on each
    of the 11 packages outside `crates/`; the packages under `crates/`
    are test fixtures, which `pkgcmp.sh` skips;
  - `--dump-mir: 1955 roots in both forms, 40 batches, 36507339
    characters at the base, 0 differ`: the 1,938 `.nova` files the base
    tracks and the 17 cases 5.5a adds, without `--ll` 5.4's dump on
    both sides.
  Any difference is a change spec §3 does not allow: stop and record it.
- [ ] **Step 5: The hidden option stays hidden.**
  - `helpcmp.sh`: `--help` of `nova` and of every command, both sides,
    `--ll` among the flags no help may show:

```sh
mkdir -p "$(dirname "$P/exec55/helpcmp.sh")"
cat > "$P/exec55/helpcmp.sh" <<'EOF'
#!/bin/bash
# helpcmp.sh: run from the repository's root, after the build, with $P exported.
# `--help` of `nova` and of every command, by the base's `nova`
# ($P/nova-base-55/nova, Task 0) and the tree's `target/debug/nova`, both run
# through a link named `nova` so the usage lines agree: both streams and the
# exit status must be byte-identical, the hidden flags and `more` absent
# from `parse --help` and `check --help`.
set -u
: "${P:?set P to the scratch directory}"
[ -f Cargo.toml ] && [ -x target/debug/nova ] || { echo "run from the repository's root, after the build"; exit 1; }
d="$P/help-55"
rm -rf "$d"; mkdir -p "$d/base" "$d/tree"
ln -s "$P/nova-base-55/nova" "$d/base/nova"
ln -s "$PWD/target/debug/nova" "$d/tree/nova"
n=0; same=0
for cmd in "" parse run build bundle dev check test fmt doc lsp new init add remove fetch update login package publish version help; do
  a=$("$d/base/nova" $cmd --help 2>&1; echo "exit=$?")
  b=$("$d/tree/nova" $cmd --help 2>&1; echo "exit=$?")
  n=$((n + 1))
  if [ "$a" == "$b" ]; then same=$((same + 1)); else echo "differs: nova $cmd --help"; fi
done
hidden=$( ("$d/tree/nova" parse --help; "$d/tree/nova" check --help) | grep -c 'dump-\|MORE\|--std\|--test\|before-async\|--ll')
echo "help: $same of $n identical; hidden flags shown: $hidden"
rm -rf "$d"
[ "$same" = "$n" ] && [ "$hidden" = 0 ]
EOF
```

  - `usagecmp.sh`: every refusal `parse` and `check` give, both sides,
    through `argv0.py`, which runs a program by execv with a first
    argument of its choosing; `--ll`'s shapes outside the form among the
    base's, compared byte for byte:

```sh
mkdir -p "$(dirname "$P/exec55/argv0.py")"
cat > "$P/exec55/argv0.py" <<'EOF'
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
mkdir -p "$(dirname "$P/exec55/usagecmp.sh")"
cat > "$P/exec55/usagecmp.sh" <<'EOF'
#!/bin/bash
# usagecmp.sh (5.5a: `--dump-mir --ll`'s shapes added; from 5.4's): run from the repository's root, after the build, with $P exported
# and $P/exec55/argv0.py written. The errors `parse` and `check` give for
# arguments they refuse, and what they print for the shapes beside them,
# from the base's `nova` ($P/nova-base-55/nova, Task 0) and the tree's
# `target/debug/nova`, each run by execv with the same first argument:
# `nova` (a link's name), `othernova` (a renamed binary), and four with no
# file name in UTF-8, "", "/", ".." and the bytes ff fe. Both streams and
# the exit status must be byte-identical, tips, usage lines and `--help`
# included: every `check` command line whose first argument after `check`
# is none of `--dump-resolution`, `--dump-hir` and `--dump-mir` is parsed
# as the base parses it. Then the
# same in a terminal, colours included, through `script`'s pseudo-terminal
# with NO_COLOR unset. Then 5.1's `--dump-ast` against the tree's own
# `--dump-tokens`. Then the shapes the hidden `check --dump-resolution`
# and `check --dump-hir` refuse, since 5.4 `check --dump-mir`'s, and since
# 5.5a its `--ll`'s, which
# the base lacks: each must exit 2 with nothing on stdout and one error on
# stderr, no panic, and a refusal of `--dump-hir` or `--dump-mir` names its
# flag (in its usage line, or first in its own error) and never another
# hidden flag but as the argument refused.
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
  "check --help --dump-hir" "check --test --dump-hir a" "check a --test --dump-hir"
  "check --dump-hir-item a.nova" "check --dump-hir-itemss a.nova" "check --dump-hir-items=1 a.nova"
  "check a.nova --dump-hir-items" "check -- --dump-hir-items" "check --help --dump-hir-items"
  "check --dump-hir-items a.nova" "check --dump-hir-items" "check --dump-hir-items --std" "check --dump-hir-items --test a"
  "check --dump-hir-items --target wasm a" "check --dump-hir-items --help" "check --dump-hir-items -h"
  "check --dump-mi a.nova" "check --dump-mirr a.nova" "check --dump-mir=1 a.nova" "check --dump-mir= a.nova"
  "check --dump-mir=before-async a.nova" "check a.nova --dump-mir" "check --target native --dump-mir a.nova"
  "check -- --dump-mir" "check --help --dump-mir" "check --test --dump-mir a" "check a --test --dump-mir"
  "check --before-async a.nova" "check --before-async" "check a --before-async" "check --dump-mir-before a.nova"
  "check --ll a.nova" "check --ll" "check a --ll" "check -- --ll" "check --help --ll" "check --ll --dump-mir a.nova"
  "check --dump-ll a.nova" "check --dump-ll" "check --dump-mir-ll a.nova" "check --dump-mir=ll a.nova"
  "check --dump-mir=--ll a.nova" "check --target native --ll a.nova" "check --test --ll a" "check --ll=1 a.nova")
runs=0; same=0
for arg0 in nova othernova "" / .. '\xff\xfe'; do
  for args in "${shapes[@]}"; do
    eval "python3 -I '$P/exec55/argv0.py' '$arg0' '$P/nova-base-55/nova' $args" > "$P/usage-base.txt" 2>&1
    echo "exit $?" >> "$P/usage-base.txt"
    eval "python3 -I '$P/exec55/argv0.py' '$arg0' '$nova' $args" > "$P/usage-tree.txt" 2>&1
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
  env -u NO_COLOR script -qc "python3 -I '$P/exec55/argv0.py' othernova '$P/nova-base-55/nova' $args" /dev/null > "$P/usage-base.txt"
  env -u NO_COLOR script -qc "python3 -I '$P/exec55/argv0.py' othernova '$nova' $args" /dev/null > "$P/usage-tree.txt"
  if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then tty=$((tty + 1)); else echo "differs in a terminal: $args"; fi
done
grep -qa $'\e\\[' "$P/usage-tree.txt" || echo "no colour in the terminal's output"
asts=0; astsame=0
for arg0 in othernova "" / .. '\xff\xfe'; do
  for args in "parse --dump-ast --ast a b" "parse --dump-ast --check a" "parse --dump-ast" "parse --dump-ast ''" "parse --dump-ast=1 a"; do
    tok=${args//dump-ast/dump-tokens}
    eval "python3 -I '$P/exec55/argv0.py' '$arg0' '$nova' $args" > "$P/usage-base.txt" 2>&1
    echo "exit $?" >> "$P/usage-base.txt"
    eval "python3 -I '$P/exec55/argv0.py' '$arg0' '$nova' $tok" 2>&1 | sed 's/dump-tokens/dump-ast/g' > "$P/usage-tree.txt"
    echo "exit ${PIPESTATUS[0]}" >> "$P/usage-tree.txt"
    asts=$((asts + 1))
    if cmp -s "$P/usage-base.txt" "$P/usage-tree.txt"; then astsame=$((astsame + 1)); else
      echo "--dump-ast differs: [$arg0] $args"; diff "$P/usage-base.txt" "$P/usage-tree.txt"; fi
  done
done
res=0; resok=0; hir=0; hirok=0
for arg0 in nova othernova "" / .. '\xff\xfe'; do
  for flag in --dump-hir --dump-mir; do
  for rest in "" "--std a" "--dump-hir a" "--dump-resolution a" "--target bogus a" "--std --std" "--stdd" "--help" "-h" \
              "--test --test a" "--test=1 a" "--tests a" "--test --target wasm a" "--target=wasm --test --std" \
              "--test --std a" "--test" "--test -h" "--test --std --target native a" "--target wasm --test" \
              "--test --std --test" "--target= a" "--before-async --before-async a" "--before-async=1 a" \
              "--before-asyn a" "--before-async" "--dump-mir a" "--target native a" "--test a" "--std" \
              "--ll --ll a" "--ll=x a" "--ll= a" "--ll --before-async a" "--before-async --ll a" \
              "--ll a --before-async" "--ll --before-async" "--ll" "--l a" "--lll a" "--ll --std a" \
              "--ll --target native a" "--ll --test a" "--ll --help" "--ll -h" "--ll --dump-mir a"; do
    # The HIR dump takes these (5.4's MIR-only refusals).
    case "$flag:$rest" in "--dump-hir:--target native a"|"--dump-hir:--test a"|"--dump-hir:--std") continue;; esac
    eval "python3 -I '$P/exec55/argv0.py' '$arg0' '$nova' check $flag $rest" > "$P/usage-tree.txt" 2> "$P/usage-err.txt"
    code=$?
    hir=$((hir + 1))
    named=$( (grep -q '^Usage: ' "$P/usage-err.txt" && grep -q "^Usage: nova check $flag " "$P/usage-err.txt") \
      || grep -q "^error: $flag " "$P/usage-err.txt" \
      || (grep -q "possible values" "$P/usage-err.txt" && ! grep -q '^Usage: ' "$P/usage-err.txt"); echo $?)
    stray=0
    for other in --dump-resolution --dump-hir --dump-mir; do
      [ "$other" = "$flag" ] && continue
      stray=$((stray + $(grep -v "'$other'" "$P/usage-err.txt" | grep -c -- "$other ")))
    done
    if [ "$code" = 2 ] && [ ! -s "$P/usage-tree.txt" ] && [ "$(grep -c '^error' "$P/usage-err.txt")" = 1 ] \
       && ! grep -q panicked "$P/usage-err.txt" && [ "$named" = 0 ] && [ "$stray" = 0 ]; then hirok=$((hirok + 1)); else
      echo "$flag: [$arg0] $rest: exit $code"; cat "$P/usage-tree.txt" "$P/usage-err.txt"; fi
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
    eval "python3 -I '$P/exec55/argv0.py' '$arg0' '$nova' $args" > "$P/usage-tree.txt" 2> "$P/usage-err.txt"
    code=$?
    res=$((res + 1))
    if [ "$code" = 2 ] && [ ! -s "$P/usage-tree.txt" ] && [ "$(grep -c '^error' "$P/usage-err.txt")" = 1 ] \
       && ! grep -q panicked "$P/usage-err.txt"; then resok=$((resok + 1)); else
      echo "--dump-resolution: [$arg0] $args: exit $code"; cat "$P/usage-tree.txt" "$P/usage-err.txt"; fi
  done
done
echo "usage: $same of $runs runs identical; in a terminal, $tty of ${#ttys[@]}; --dump-ast as --dump-tokens: $astsame of $asts; --dump-resolution refused: $resok of $res; --dump-hir and --dump-mir refused: $hirok of $hir"
rm -f "$P/usage-base.txt" "$P/usage-tree.txt" "$P/usage-err.txt"; cd / && rmdir "$work"
[ "$same" = "$runs" ] && [ "$tty" = "${#ttys[@]}" ] && [ "$astsame" = "$asts" ] && [ "$resok" = "$res" ] && [ "$hirok" = "$hir" ]
EOF
```

  Then `bash "$P/exec55/helpcmp.sh"` and `bash "$P/exec55/usagecmp.sh"`.
  Expected, as the replay printed: `help: 22 of 22 identical; hidden
  flags shown: 0`; and, in 34 s, six lines of the usage each first
  argument gives (`[nova] Usage: nova check [OPTIONS] [FILE], exit 2`,
  and so on), then `usage: 558 of 558 runs identical; in a terminal, 10
  of 10; --dump-ast as --dump-tokens: 25 of 25; --dump-resolution
  refused: 114 of 114; --dump-hir and --dump-mir refused: 522 of 522`.
  The form's own `--ll` shapes are Task 1's flag test's and refusal
  test's (`SH`).
- [ ] **Step 6: The lockfile and Rust 1.78.**
  - `git diff --numstat db640e97 -- Cargo.lock '*Cargo.toml'`. Expected:
    nothing.
  - Then:

    ```sh
    RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-55 cargo check --locked --workspace
    rm -rf /dev/shm/msrv-55
    ```

    Expected: clean, `Finished` in 45 s. It checks the libraries and
    binaries, as CI's MSRV job does, not the tests, whose Rust meets 1.78
    by review (Global Constraints).
- [ ] **Step 7: The differential's time, N's tests, and N's memory and
  stack.**
  - Without the release runtime, as the build stands: `cargo test
    --locked --workspace --test selfhost --no-run`, then `time SH`, then
    `time RUST_BACKTRACE=1 SH`, `RUST_BACKTRACE=1` for that command alone,
    as CI's jobs export it (decision 9). Expected: `67 passed; 0 failed`
    in 142.62 s (user CPU 525 s), and `67 passed; 0 failed` again in
    147.51 s; no child past the 120 s `CHILD_LIMIT` (a child killed
    there: the conventions' rule).
  - Then N's memory and stack on the debug runtime (ADR 0058 §11 and
    §12's method), with these scripts, as the replay ran them:
    `lists.py` the loose corpus as the harness lists it, `mem.py` N's
    time, peaks, collections and largest live heap per set without and
    with `--ll`, `stack.py` the least stacks of the depth programs `SH`
    wrote, in either form:

```sh
mkdir -p "$(dirname "$P/exec55/lists.py")"
cat > "$P/exec55/lists.py" <<'EOF'
#!/usr/bin/env python3
"""lists.py: run from the repository's root. The loose corpus as the
harness lists it (`loose_corpus`: a sorted walk, `selfhost/` and the cases
left out, and no file of a package's `src/` or `tests/`), one per line."""
import os

ROOT = os.getcwd()
CASES = os.path.join(ROOT, "crates/nova-cli/tests/selfhost/cases")


def nova_files(d, skip, out):
    for name in sorted(os.listdir(d)):
        p = os.path.join(d, name)
        if os.path.islink(p):
            continue
        if os.path.isdir(p):
            if name in ("target", "node_modules") or name.startswith(".") or p in skip:
                continue
            nova_files(p, skip, out)
        elif os.path.isfile(p) and name.endswith(".nova"):
            out.append(os.path.relpath(p, ROOT))


def in_package(f):
    parent = os.path.dirname(os.path.join(ROOT, f))
    return (os.path.basename(parent) in ("src", "tests")
            and os.path.isfile(os.path.join(os.path.dirname(parent), "nova.toml")))


files = []
nova_files(ROOT, [os.path.join(ROOT, "selfhost"), CASES], files)
for f in files:
    if not in_package(f):
        print(f)
EOF
```

```sh
mkdir -p "$(dirname "$P/exec55/mem.py")"
cat > "$P/exec55/mem.py" <<'EOF'
#!/usr/bin/env python3
"""mem.py COLUMN NBIN: run from the repository's root, after `SH` has written
the MIR fuzzer's and the depth test's inputs, with $P exported and
$P/exec55/loose.txt written (lists.py). N's `mir --std std` (NBIN) on ADR
0058 §11's inputs that the harness lists alike, N's root, the loose corpus,
the fuzzer and the depth cases, each set in one process, without `--ll` and
with it: the median wall time of three runs, the peak RSS
(/proc/PID/status's VmHWM, polled every 2 ms) the largest of the three, the
size of stdout, and from one run with NOVA_GC_DEBUG=1 the collections and
the largest live heap they report. Appends one line a set and form to
$P/exec55/mem.txt, headed by COLUMN."""
import os
import re
import statistics
import subprocess
import sys
import tempfile
import threading
import time

P = os.environ["P"]
ROOT = os.getcwd()
col, binary = sys.argv[1], sys.argv[2]
TMP = tempfile.gettempdir()
FZ, DP = os.path.join(TMP, "nova-selfhost-mir-fuzz"), os.path.join(TMP, "nova-selfhost-deep-n-mir")
fz = [f"{s}/{f}" for s in (601, 602, 603) for f in sorted(os.listdir(f"{FZ}/{s}"))]
dp = sorted(f for f in os.listdir(DP) if f.endswith(".nova"))
loose = open(f"{P}/exec55/loose.txt").read().split()
SETS = [("N's root", ROOT, ["selfhost/src/main.nova"]),
        ("the loose corpus", ROOT, loose),
        ("the fuzzer", FZ, fz),
        ("the depth cases", DP, dp)]


def run(cmd, cwd, env=None):
    start = time.time()
    pr = subprocess.Popen(cmd, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, env=env)
    out = [b"", b""]

    def read(i, stream):
        out[i] = stream.read()

    readers = [threading.Thread(target=read, args=(0, pr.stdout)),
               threading.Thread(target=read, args=(1, pr.stderr))]
    for t in readers:
        t.start()
    peak = 0
    while pr.poll() is None:
        try:
            for line in open(f"/proc/{pr.pid}/status"):
                if line.startswith("VmHWM:"):
                    peak = max(peak, int(line.split()[1]))
        except OSError:
            pass
        time.sleep(0.002)
    for t in readers:
        t.join()
    return time.time() - start, peak / 1024, pr.returncode, len(out[0]), out[1].decode("utf-8", "replace")


with open(f"{P}/exec55/mem.txt", "a") as log:
    for name, cwd, files in SETS:
        for ll in ([], ["--ll"]):
            cmd = [binary, "mir", "--std", f"{ROOT}/std", *ll, *files]
            runs = [run(cmd, cwd) for _ in range(3)]
            assert all(r[2] == 0 for r in runs), (name, [r[2] for r in runs])
            err = run(cmd, cwd, dict(os.environ, NOVA_GC_DEBUG="1"))[4]
            lives = [int(m.group(1)) for m in
                     re.finditer(r"collection \d+ freed \d+ bytes, \d+ objects live \((\d+) bytes\)", err)]
            line = (f"{col} | {name} ({len(files)}) {'--ll' if ll else 'plain'} | "
                    f"{statistics.median(r[0] for r in runs):.2f} s | {max(r[1] for r in runs):.0f} MB | "
                    f"{len(lives)} collections, largest live {max(lives, default=0) / 1e6:.0f} MB | "
                    f"stdout {runs[0][3] / 1e6:.2f} MB")
            print(line, flush=True)
            log.write(line + "\n")
EOF
```

```sh
mkdir -p "$(dirname "$P/exec55/stack.py")"
cat > "$P/exec55/stack.py" <<'EOF'
#!/usr/bin/env python3
"""stack.py NBIN FORM OUT [FILE...]: run from the repository's root, after
the build and after `SH` wrote the depth test's inputs (5.4's script, with
FORM). The least main-thread stack (KB, 4 KB bisection under `ulimit -s`) on
which N's `selfhost mir --std std [--ll] FILE` exits 0 and prints S0's
`check --dump-mir [--ll] FILE` part, FORM `ll` with `--ll` and `plain`
without, for each depth input in the temporary directory's
`nova-selfhost-deep-n-mir` (every one when no FILE is named), appended to
OUT a line each: the file and the KB, or `FAILS at 1024`."""
import os
import subprocess
import sys
import tempfile

ROOT = os.getcwd()
D = os.path.join(tempfile.gettempdir(), "nova-selfhost-deep-n-mir")
STD = os.path.join(ROOT, "std")
NB, FORM, OUT = sys.argv[1], sys.argv[2], sys.argv[3]
assert FORM in ("ll", "plain"), FORM
LL = ["--ll"] if FORM == "ll" else []
files = sys.argv[4:] or sorted(f for f in os.listdir(D) if f.endswith(".nova"))


def s0(f):
    r = subprocess.run([os.path.join(ROOT, "target/debug/nova"), "check", "--dump-mir", *LL, f], cwd=D,
                       capture_output=True)
    assert r.returncode == 0 and (b"\n(Ll ok)\n" in r.stdout) == (FORM == "ll"), f
    return r.stdout


def ok(f, kb, want):
    r = subprocess.run(["sh", "-c", 'ulimit -s %d && exec "$0" "$@"' % kb, NB, "mir", "--std", STD, *LL, f],
                       cwd=D, capture_output=True)
    return r.returncode == 0 and r.stdout == want


with open(OUT, "a") as out:
    for f in files:
        want = s0(f)
        lo, hi = 4, 1024  # lo fails (or untested), hi passes
        if not ok(f, hi, want):
            out.write("%s\tFAILS at 1024\n" % f)
            out.flush()
            continue
        while hi - lo > 4:
            mid = (lo + hi) // 2 // 4 * 4
            if mid <= lo:
                mid = lo + 4
            if ok(f, mid, want):
                hi = mid
            else:
                lo = mid
        out.write("%s\t%d\n" % (f, hi))
        out.flush()
EOF
```

    ```sh
    mkdir -p "$P/exec55/heap"
    (cd selfhost && NOVA_RUNTIME_LIB="$PWD/../target/debug/libnova_runtime.a" ../target/debug/nova build -o "$P/exec55/heap/n")
    python3 "$P/exec55/lists.py" > "$P/exec55/loose.txt"
    : > "$P/exec55/mem.txt"
    python3 "$P/exec55/mem.py" "N debug" "$P/exec55/heap/n"
    : > "$P/exec55/stack-debug-ll.tsv"
    python3 "$P/exec55/stack.py" "$P/exec55/heap/n" ll "$P/exec55/stack-debug-ll.tsv"
    : > "$P/exec55/stack-debug-plain.tsv"
    python3 "$P/exec55/stack.py" "$P/exec55/heap/n" plain "$P/exec55/stack-debug-plain.tsv"
    ```

    Expected, as the replay measured (`mem.py` 276 s, each `stack.py`
    122 to 129 s): `loose.txt` 251 lines; `mem.py`'s lines, N on the
    debug runtime, without and with `--ll`: N's root 5.82 and 7.66 s,
    212 and 320 MB, 23 and 24 collections, the largest live heap 102 and
    148 MB, stdout 5.49 and 17.74 MB; the loose corpus 42 and 44 MB, 103
    and 122 collections; the fuzzer 44 and 40 MB, 60 and 73; the depth
    cases 41 and 48 MB, 42 and 45; each `stack-debug-*.tsv` 25 lines,
    none `FAILS at 1024`, 52 to 216 KB with `--ll` and 56 to 212 KB
    without. These rows are ADR 0058 §11's debug table (the replay's,
    decision 17); each within 1 MB and one collection of it, and every
    figure goes in the ledger.
  - Then as the Selfhost job runs, in one shell: `export
    NOVA_EMBED_RUNTIME=1 CARGO_PROFILE_DEV_OPT_LEVEL=1`, two of the job's
    variables; the third, the workflow's `RUST_BACKTRACE=1`, is not set
    here (the shell keeps the Runs block's `0`; `SH` under
    `RUST_BACKTRACE=1` is the step's first part), nor is `--all-features`
    passed, which changes nothing `selfhost.rs` builds. `df -h /` (at
    least 6G), `time cargo build --locked --workspace` (53 s), `time cargo
    test --locked --workspace --test selfhost --no-run` (38 s), then three
    times `time SH`; then N's tests on each runtime: `(cd selfhost &&
    NOVA_RUNTIME_LIB=$PWD/../target/debug/libnova_runtime.a
    ../target/debug/nova test) 2>&1 | tail -n 1` and `(cd selfhost &&
    ../target/debug/nova test) 2>&1 | tail -n 1`; then the job's last
    step, `time NOVA_RUNTIME_LIB=$PWD/target/debug/libnova_runtime.a cargo
    test --locked --workspace --all-features --no-fail-fast --test selfhost
    -- --exact the_selfhost_tests_pass`. Expected: `67 passed; 0 failed`
    three times, 45.49, 41.64 and 41.18 s (user CPU 146 to 159 s); `108
    passed` on each runtime; `1 passed` in 1.35 s.
  - Then, in the same shell, N on the release runtime:

    ```sh
    (cd selfhost && ../target/debug/nova build -o "$P/exec55/heap/n-rel")
    python3 "$P/exec55/mem.py" "N release" "$P/exec55/heap/n-rel"
    : > "$P/exec55/stack-release-ll.tsv"
    python3 "$P/exec55/stack.py" "$P/exec55/heap/n-rel" ll "$P/exec55/stack-release-ll.tsv"
    : > "$P/exec55/stack-release-plain.tsv"
    python3 "$P/exec55/stack.py" "$P/exec55/heap/n-rel" plain "$P/exec55/stack-release-plain.tsv"
    ```

    Expected, as measured (`mem.py` 67 s, each `stack.py` 32 to 33 s),
    without and with `--ll`: N's root 1.55 and 1.94 s, 211 and 303 MB, 23
    and 24 collections, the largest live heap 101 and 102 MB, stdout 5.49
    and 17.74 MB; the loose corpus 41 and 39 MB, 102 and 125
    collections; the fuzzer 43 and 41 MB, 60 and 75, live 13 and 12 MB;
    the depth cases 40 and 44 MB, 42 and 45, live 12 and 13 MB; each
    `stack-release-*.tsv` 25 lines, none failing, 48 to 212 KB. Against
    ADR 0058 §11 and §12: N's root's row as §11's; the fuzzer's and the
    depth cases' within 1 MB and one collection (§11 counts the loose
    corpus by file name, 245 files, so that row is not compared); every
    least stack, on both runtimes and in both forms, within two bisection
    steps (8 KB) of §12's two rounds. A change past 16 KB, an overflow
    at 1 MB, or a peak that moves past 10 MB is recorded in the ledger;
    it is no threshold (spec §9).
  - Then `unset NOVA_EMBED_RUNTIME CARGO_PROFILE_DEV_OPT_LEVEL` and
    `cargo build --locked --workspace`, which ends with `Finished`: the
    debug-runtime `nova` at `opt-level = 0` again (`target/debug/nova
    version` prints `runtime: not embedded`).
- [ ] **Step 8: The mutants.** One at a time, each edit is made, its
  commands run, and the file restored with `git checkout -- <file>`, all
  on the last commit, in a shell under the process limit, with
  `NOVA_EMBED_RUNTIME=1` exported for the whole step (decision 16), N
  on the release runtime as the Selfhost job runs `selfhost.rs`. Of the
  job's other variables, `CARGO_PROFILE_DEV_OPT_LEVEL=1` is not set (S0
  at `opt-level = 0`, as the replay ran the mutants) and
  `RUST_BACKTRACE` stays the Runs block's `0` but for mutant 29's
  command, which sets `1` (CI's). These two lists are set:

  ```sh
  LL="the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says the_mir_dump_has_a_header_per_root_and_says_how_far_each_got every_run_of_a_mir_dump_settles_e0004s_paths_on_windows both_sides_refuse_the_mir_dumps_shapes_alike the_mir_dumps_agree_on_the_corpus_and_the_cases every_form_the_emitter_handles_occurs_in_the_compared_ll the_mir_dumps_agree_on_the_cases_with_windows_line_endings the_ll_dump_is_what_build_release_writes the_mir_fuzzers_inputs_agree_and_hold_each_kind n_lowers_the_depth_cases_on_a_1_mb_main_thread a_manifest_n_cannot_read_stops_n_with_a_message n_checks_std_for_each_program_when_std_gives_a_body_diagnostic the_selfhost_tests_pass"
  GUARD=the_resolution_dump_has_a_header_per_root_and_std_its_own_part
  ```

  - Each mutant runs only the tests it should trip and `$GUARD`, a cheap
    test that builds N and compares its resolution dump with S0's, which
    must pass every time: a mutant that broke everything would fail it.
    The whole of `SH` and `S` would take hours over 79 mutants; the
    replay's runs add up to 46 minutes this way (13 tests and `$GUARD`
    in 36 s unmutated).
  - Run them in batches sized so that each, at twice the replay's time
    (this machine's speed varies by a factor of two between sessions,
    decision 17), fits under 600 s: no batch took more than 260 s in the
    replay. Each batch is one command in the foreground (Conventions):
    - S0's, 1 to 13, in three: 1 to 5 (220 s), 6 to 10 (237 s) and 11
      to 13 (163 s);
    - the harness's, 14 to 37, in three: 14 to 18 (210 s), 19 to 28, the
      coverage test's and the escape case's (260 s), and 29 to 37, the
      backtrace fix's and the report's (89 s);
    - N's, 38 to 79, five at a time: 38 to 42 (200 s), 43 to 47 (200 s),
      48 to 52 (213 s), 53 to 57 (172 s), 58 to 62 (167 s), 63 to 67
      (174 s), 68 to 72 (182 s), 73 to 77 (178 s) and 78 to 79 (74 s).
    If a batch is killed, or the shell restarts, `git checkout -- <file>`
    the file of the mutant it was on, see `git status --short` print
    nothing, and run that mutant again before the next batch.
  - Each edit replaces one exact text, which occurs once in its file
    unless the mutant names which occurrence, counted on the text as
    shown ("the first of 2"); "the line … goes" removes that whole line.
    A mutant of several edits makes them in order, each on the text the
    one before left.
  - Each command list starts with `cargo build --locked --workspace`. A
    mutant of N needs no Rust rebuilt: `selfhost.rs` builds N from
    `selfhost/` each run.
  - Each caught mutant must fail as written: the tests named, and no
    other among those run; a name after `the_selfhost_tests_pass` in
    parentheses is one of N's `@test`s, which that test runs; a name
    `ll_dumps::…` is the driver's unit test, which the `--lib` command
    runs. Three pass by design, each with its reason below: mutants 6, 13
    and 76. Any other mutant that passes is a gap: record it in the
    ledger, name it in the PR, and give it to the review (Step 10), which
    says whether a check is added.
  - At the end, `git status --short` prints nothing, then `unset
    NOVA_EMBED_RUNTIME` and `cargo build --locked --workspace`.

  1. **S0's form takes `--ll` with `--before-async`.** `crates/nova-cli/src/cmd/run.rs`: `if args.ll && args.before_async {` becomes `if false && args.ll && args.before_async {`.
     - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
     - Expected: fails:
       `both_sides_refuse_the_mir_dumps_shapes_alike`, `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`.
  2. **S0's refusal of `--ll` with `--before-async` after the missing FILE's.** `crates/nova-cli/src/cmd/run.rs`: replace

     ```
         // The emitter reads the after form alone (5.5a, spec §3.1).
         if args.ll && args.before_async {
             eprintln!("error: --dump-mir --ll takes no --before-async");
             return Ok(2);
         }
         if args.files.is_empty() {
             eprintln!("error: --dump-mir needs a FILE");
             return Ok(2);
         }
     ```

     with

     ```
         if args.files.is_empty() {
             eprintln!("error: --dump-mir needs a FILE");
             return Ok(2);
         }
         // The emitter reads the after form alone (5.5a, spec §3.1).
         if args.ll && args.before_async {
             eprintln!("error: --dump-mir --ll takes no --before-async");
             return Ok(2);
         }
     ```

     - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
     - Expected: fails:
       `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`.
  3. **S0's refusal in other words.** `crates/nova-cli/src/cmd/run.rs`: `eprintln!("error: --dump-mir --ll takes no --before-async");` becomes `eprintln!("error: --dump-mir --ll takes no --before-async.");`.
     - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
     - Expected: fails:
       `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`.
  4. **S0's parser takes `--ll` twice.** `crates/nova-cli/src/cmd/run.rs`: replace

     ```
         #[arg(long)]
         ll: bool,
     ```

     with

     ```
         #[arg(long, overrides_with = "ll")]
         ll: bool,
     ```

     - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
     - Expected: fails:
       `both_sides_refuse_the_mir_dumps_shapes_alike`, `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`.
  5. **S0's `--ll` asks for nothing.** `crates/nova-cli/src/cmd/run.rs`: `nova_driver::MirDumps::new(args.before_async, args.ll)` becomes `nova_driver::MirDumps::new(args.before_async, false)`.
     - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
     - Expected: fails:
       `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_ll_dump_is_what_build_release_writes`, `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  6. **The before form asks for the `.ll` too.** `crates/nova-driver/src/lib.rs`: `ll: self.ll && !self.before_async,` becomes `ll: self.ll,`.
     - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
     - Expected: passes, by design: equivalent. The `.ll` is made of `after` alone, the after form's module, which `HirDumps::dumps_with` lowers only when `Wanted::mir` asks for it; `MirDumps::dump` asks for `mir` exactly when `before_async` is false, so with `before_async` the `.ll` part's condition finds no module whatever `ll` says, and the before form takes no `.ll` either way (`the_ll_dump_is_the_module_build_release_emits` asserts it, and passes under the mutant). The command line refuses `--ll` with `--before-async` before a `MirDumps` is made (mutants 1 and 2).
  7. **The `.ll` printed without `--ll`.** `crates/nova-driver/src/lib.rs`: `= (wanted.ll, &after, &mut dumps.mir) {` becomes `= (true, &after, &mut dumps.mir) {`.
     - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
     - Expected: fails:
       `ll_dumps::the_ll_dump_is_the_module_build_release_emits`, `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`.
  8. **An `(Ll` part for a root that did not lower.** `crates/nova-driver/src/lib.rs`: replace

     ```
                     mir.push_str(&ll_part(module));
                 }
     ```

     with

     ```
                     mir.push_str(&ll_part(module));
                 }
                 if let (true, Some(Err(_)), Some(mir)) = (wanted.ll, &after, &mut dumps.mir) {
                     mir.push_str("(Ll error \"no `main` function in MIR module\")\n");
                 }
     ```

     - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
     - Expected: fails:
       `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `ll_dumps::the_ll_dump_is_the_module_build_release_emits`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  9. **The `.ll` of the before form's module.** `crates/nova-driver/src/lib.rs`: replace

     ```
                 if let (true, Some(Ok(module)), Some(mir)) = (wanted.ll, &after, &mut dumps.mir) {
                     mir.push_str(&ll_part(module));
     ```

     with

     ```
                 if let (true, Some(Ok(_)), Some(mir)) = (wanted.ll, &after, &mut dumps.mir) {
                     mir.push_str(&ll_part(lowered(true).as_ref().expect("lowers")));
     ```

     - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
     - Expected: fails:
       `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `ll_dumps::the_ll_dump_is_the_module_build_release_emits`, `the_ll_dump_is_what_build_release_writes`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  10. **The `.ll` without its last line break.** `crates/nova-driver/src/lib.rs`: `Ok(ir) => format!("(Ll ok)\n{ir}"),` (the first of 2) becomes `Ok(ir) => format!("(Ll ok)\n{}", ir.trim_end()),`.
      - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `ll_dumps::the_ll_dump_is_the_module_build_release_emits`, `ll_dumps::the_ll_part_is_the_emitters_text_or_its_words_quoted`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_ll_dump_is_what_build_release_writes`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  11. **`(Ll ok)` in other words.** `crates/nova-driver/src/lib.rs`: `Ok(ir) => format!("(Ll ok)\n{ir}"),` (the first of 2) becomes `Ok(ir) => format!("(Ll  ok)\n{ir}"),`.
      - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `ll_dumps::the_ll_dump_is_the_module_build_release_emits`, `ll_dumps::the_ll_part_is_the_emitters_text_or_its_words_quoted`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_ll_dump_is_what_build_release_writes`, `the_mir_dump_flag_is_hidden_and_refused_as_the_spec_says`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  12. **An error's words not quoted.** `crates/nova-driver/src/lib.rs`: `nova_resolver::dump::quote(&mut part, &e.to_string());` becomes `part.push_str(&e.to_string());`.
      - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `ll_dumps::the_ll_dump_is_the_module_build_release_emits`, `ll_dumps::the_ll_part_is_the_emitters_text_or_its_words_quoted`, `the_ll_dump_is_what_build_release_writes`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`.
  13. **An error's words in quotes, not escaped.** `crates/nova-driver/src/lib.rs`: `nova_resolver::dump::quote(&mut part, &e.to_string());` becomes `part.push_str(&format!("\"{e}\""));`.
      - Run `cargo test --locked --workspace --lib -- ll_dumps` and `SH --exact $LL $GUARD`.
      - Expected: passes, by design: equivalent. `quote` escapes a byte below 0x20, 0x7F, `"` and `\` and keeps every other character; the emitter's errors hold none of those: the four that reach `ll_part` (no `main`, an `Await`, a float `%`, a float bitwise operator) are constant words, and the fifth, a WASM memory access (`{op:?}`), cannot reach the native dump (only a WASM build's std seeds the memory builtins), and `MemOp`'s thirteen names (`Load8`, `Store8`, `Load64`, `Store64`, `Copy`, `Fill`, `Pages`, `Grow`, `HeapBase`, `Poll`, `Abort`, `ClosureWord`, `ClosureCall`) hold no escaped byte either; so the quoted and the unescaped text are the same bytes for every module, built by hand or lowered. Dropping the quotes themselves is mutant 12, caught.
  14. **`split_ll` leaves the `.ll` at its `(Ll` line.** `crates/nova-cli/tests/selfhost.rs`: `in_ll |= line.starts_with("(Ll ");` becomes `in_ll = line.starts_with("(Ll ");`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_ll_dump_is_what_build_release_writes`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  15. **`split_ll` keeps the `.ll` past the next header.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
              if line.starts_with("== ") {
                  in_ll = false;
      ```

      with

      ```
              if line.starts_with("== ") {
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_ll_dump_is_what_build_release_writes`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  16. **`check_ll` looks for another float `%` case.** `crates/nova-cli/tests/selfhost.rs`: `const FLOAT_REMAINDER_CASE: &str = "ll_a_float_remainder_errs.nova";` becomes `const FLOAT_REMAINDER_CASE: &str = "ll_a_float_le_is_an_ordered_compare.nova";`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_ll_dump_is_what_build_release_writes`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`.
  17. **`check_ll` expects other words for float `%`.** `crates/nova-cli/tests/selfhost.rs`: `const FLOAT_REMAINDER_WORDS: &str = "float remainder (%) is not supported by the LLVM backend";` becomes `const FLOAT_REMAINDER_WORDS: &str = "float remainder (%) is not supported";`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_ll_dump_is_what_build_release_writes`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`.
  18. **N's after form without `--ll`.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
              if form == MirForm::After {
                  n.push("--ll");
              }
      ```

      with nothing.

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  19. **A terminator gone from `LL_TERMINATORS`.** `crates/nova-cli/tests/selfhost.rs`: `const LL_TERMINATORS: [&str; 5] = ["Goto", "Branch", "Switch", "Return", "Trap"];` becomes `const LL_TERMINATORS: [&str; 4] = ["Goto", "Branch", "Switch", "Return"];`.
      - Run `SH --exact every_form_the_emitter_handles_occurs_in_the_compared_ll $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`.
  20. **`Le` on `Float` gone from `LL_BINS`.** `crates/nova-cli/tests/selfhost.rs`: `"Add", "Sub", "Mul", "Div", "Eq", "Ne", "Lt", "Le", "Gt", "Ge",` becomes `"Add", "Sub", "Mul", "Div", "Eq", "Ne", "Lt", "Gt", "Ge",`.
      - Run `SH --exact every_form_the_emitter_handles_occurs_in_the_compared_ll $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`.
  21. **`(Switch I8)` gone from the forms.** `crates/nova-cli/tests/selfhost.rs`: the line `"(Switch I8)",` goes.
      - Run `SH --exact every_form_the_emitter_handles_occurs_in_the_compared_ll $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`.
  22. **`TestSelector` gone from `LL_UNREACHABLE`.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
      const LL_UNREACHABLE: [(&str, &str); 7] = [
          (
              "call @nova_rt_test_selector",
              "TestSelector: only test mode's synthesized `main` calls it, and test mode's code \
               generation is not compared",
          ),
      ```

      with

      ```
      const LL_UNREACHABLE: [(&str, &str); 6] = [
      ```

      - Run `SH --exact every_form_the_emitter_handles_occurs_in_the_compared_ll $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`.
  23. **A reachable form listed unreachable.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
      const LL_UNREACHABLE: [(&str, &str); 7] = [
      ```

      with

      ```
      const LL_UNREACHABLE: [(&str, &str); 8] = [
          ("(Switch I8)", "a mutant"),
      ```

      - Run `SH --exact every_form_the_emitter_handles_occurs_in_the_compared_ll $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`.
  24. **`ll_runtime` misses the first runtime function.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
              .skip_while(|l| *l != "declare void @llvm.trap()")
              .skip(1)
      ```

      with

      ```
              .skip_while(|l| *l != "declare void @llvm.trap()")
              .skip(2)
      ```

      - Run `SH --exact every_form_the_emitter_handles_occurs_in_the_compared_ll $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`.
  25. **A byte past ASCII taken for a control byte.** `crates/nova-cli/tests/selfhost.rs`: `0x80..=0xff => "a byte past ASCII".to_string(),` becomes `0x80..=0xff => "a control byte".to_string(),`.
      - Run `SH --exact every_form_the_emitter_handles_occurs_in_the_compared_ll $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`.
  26. **Every switch taken for an `I64` one.** `crates/nova-cli/tests/selfhost.rs`: `add(format!("(Switch {})", temps[disc]));` becomes `add("(Switch I64)".to_string());`.
      - Run `SH --exact every_form_the_emitter_handles_occurs_in_the_compared_ll $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`.
  27. **No call to an extern seen.** `crates/nova-cli/tests/selfhost.rs`: `"Call" if externs.contains(&rest.split('"').nth(1).unwrap_or("")) => {` becomes `"Call" if externs.contains(&"") => {`.
      - Run `SH --exact every_form_the_emitter_handles_occurs_in_the_compared_ll $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`.
  28. **The escape case without its raw line break.** `crates/nova-cli/tests/selfhost/cases/mir/ll_a_string_of_every_escape_class.nova`: replace

      ```
          let raw = "first
      second"
      ```

      with

      ```
          let raw = "first second"
      ```

      - Run `SH --exact the_mir_dumps_agree_on_the_cases_with_windows_line_endings $GUARD`.
      - Expected: fails:
        `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`.
  29. **The build's backtrace left on (under `RUST_BACKTRACE=1`).** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
                      // CI sets `RUST_BACKTRACE=1`, under which the error's
                      // backtrace follows its words on stderr.
                      .env("RUST_BACKTRACE", "0")
                      .env("RUST_LIB_BACKTRACE", "0")
      ```

      with nothing.

      - Run `RUST_BACKTRACE=1 SH --exact the_ll_dump_is_what_build_release_writes $GUARD`, `RUST_BACKTRACE=1` set for that command alone, as CI's jobs export it.
      - Expected: fails:
        `the_ll_dump_is_what_build_release_writes`.
  30. **The walk never enters the `.ll`.** `crates/nova-cli/tests/selfhost/compare.rs`: `== b"(Ll ok)" {` becomes `== b"(Ll ok) " {`.
      - Run `SH --exact the_ll_mismatch_report_names_the_define_and_block the_mir_mismatch_report_names_the_function_block_and_statement the_mismatch_report_names_the_file_and_line a_missing_final_line_break_is_marked $GUARD`.
      - Expected: fails:
        `the_ll_mismatch_report_names_the_define_and_block`.
  31. **The `.ll`'s lines counted from 0.** `crates/nova-cli/tests/selfhost/compare.rs`: `let at = lines + 1;` becomes `let at = lines;`.
      - Run `SH --exact the_ll_mismatch_report_names_the_define_and_block the_mir_mismatch_report_names_the_function_block_and_statement the_mismatch_report_names_the_file_and_line a_missing_final_line_break_is_marked $GUARD`.
      - Expected: fails:
        `the_ll_mismatch_report_names_the_define_and_block`.
  32. **`}` leaves no function.** `crates/nova-cli/tests/selfhost/compare.rs`: replace

      ```
                  if line.strip_suffix(b"\n").unwrap_or(line) == b"}" {
                      self.define = None;
                      self.label = None;
                  }
      ```

      with nothing.

      - Run `SH --exact the_ll_mismatch_report_names_the_define_and_block the_mir_mismatch_report_names_the_function_block_and_statement the_mismatch_report_names_the_file_and_line a_missing_final_line_break_is_marked $GUARD`.
      - Expected: fails:
        `the_ll_mismatch_report_names_the_define_and_block`.
  33. **A `define` named without its quotes.** `crates/nova-cli/tests/selfhost/compare.rs`: `Some((quoted, _)) => format!("\"{quoted}\""),` becomes `Some((quoted, _)) => quoted.to_string(),`.
      - Run `SH --exact the_ll_mismatch_report_names_the_define_and_block the_mir_mismatch_report_names_the_function_block_and_statement the_mismatch_report_names_the_file_and_line a_missing_final_line_break_is_marked $GUARD`.
      - Expected: fails:
        `the_ll_mismatch_report_names_the_define_and_block`.
  34. **The C wrapper named with its parameters.** `crates/nova-cli/tests/selfhost/compare.rs`: `None => symbol.split('(').next().unwrap_or(symbol).to_string(),` becomes `None => symbol.to_string(),`.
      - Run `SH --exact the_ll_mismatch_report_names_the_define_and_block the_mir_mismatch_report_names_the_function_block_and_statement the_mismatch_report_names_the_file_and_line a_missing_final_line_break_is_marked $GUARD`.
      - Expected: fails:
        `the_ll_mismatch_report_names_the_define_and_block`.
  35. **A label taken only when it ends `0:`.** `crates/nova-cli/tests/selfhost/compare.rs`: `!line.starts_with(b" ") && line.ends_with(b":")` becomes `!line.starts_with(b" ") && line.ends_with(b"0:")`.
      - Run `SH --exact the_ll_mismatch_report_names_the_define_and_block the_mir_mismatch_report_names_the_function_block_and_statement the_mismatch_report_names_the_file_and_line a_missing_final_line_break_is_marked $GUARD`.
      - Expected: fails:
        `the_ll_mismatch_report_names_the_define_and_block`.
  36. **The `(Ll` line walked as MIR.** `crates/nova-cli/tests/selfhost/compare.rs`: replace

      ```
              if line.starts_with(b"(Ll ") {
                  return Some("at its (Ll line".to_string());
              }
      ```

      with nothing.

      - Run `SH --exact the_ll_mismatch_report_names_the_define_and_block the_mir_mismatch_report_names_the_function_block_and_statement the_mismatch_report_names_the_file_and_line a_missing_final_line_break_is_marked $GUARD`.
      - Expected: fails:
        `the_ll_mismatch_report_names_the_define_and_block`.
  37. **The walk's `.ll` kept into the next root.** `crates/nova-cli/tests/selfhost/compare.rs`: replace

      ```
                  *self = MirWalk {
                      mir: line.starts_with(b"(Mir "),
                      ..MirWalk::default()
                  };
      ```

      with

      ```
                  self.mir = line.starts_with(b"(Mir ");
      ```

      - Run `SH --exact the_ll_mismatch_report_names_the_define_and_block the_mir_mismatch_report_names_the_function_block_and_statement the_mismatch_report_names_the_file_and_line a_missing_final_line_break_is_marked $GUARD`.
      - Expected: fails:
        `the_ll_mismatch_report_names_the_define_and_block`.
  38. **`float_hex` in lowercase.** `selfhost/src/emit_ll.nova`: `out.push(hex_upper((bits >> shift) & 15))` becomes `out.push("0123456789abcdef".byte_slice((bits >> shift) & 15, ((bits >> shift) & 15) + 1))`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`, `the_selfhost_tests_pass` (N's `@test` `each_error_of_the_emitter_is_in_s0s_words`, `float_hex_is_the_bits_in_16_uppercase_digits`).
  39. **An escape's low digit in lowercase.** `selfhost/src/emit_ll.nova`: `out.push("\\${hex_upper(b >> 4)}${hex_upper(b & 15)}")` becomes `out.push("\\${hex_upper(b >> 4)}${"0123456789abcdef".byte_slice(b & 15, (b & 15) + 1)}")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`, `the_selfhost_tests_pass` (N's `@test` `escape_bytes_writes_each_byte_class_as_s0_does`).
  40. **An escape with an `x`.** `selfhost/src/emit_ll.nova`: `out.push("\\${hex_upper(b >> 4)}${hex_upper(b & 15)}")` becomes `out.push("\\x${hex_upper(b >> 4)}${hex_upper(b & 15)}")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`, `the_selfhost_tests_pass` (N's `@test` `escape_bytes_writes_each_byte_class_as_s0_does`).
  41. **`ret_of` with the function winning.** `selfhost/src/emit_ll.nova`: replace

      ```
          for i in 0..functions.len() {
              let f = functions.at(i)
              let _ = ret_of.insert(f.name, f.ret)
          }
          for i in 0..externs.len() {
              let e = externs.at(i)
              let _ = ret_of.insert(e.symbol, e.ret)
          }
      ```

      with

      ```
          for i in 0..externs.len() {
              let e = externs.at(i)
              let _ = ret_of.insert(e.symbol, e.ret)
          }
          for i in 0..functions.len() {
              let f = functions.at(i)
              let _ = ret_of.insert(f.name, f.ret)
          }
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `a_symbol_both_a_function_and_an_extern_returns_the_externs_class`).
  42. **`%rN` not restarted per function.** `selfhost/src/emit_ll.nova`, 3 edits, in order:

      1. replace

         ```
             let rt = rt_table()
         ```

         with

         ```
             let rt = rt_table()
             let mut ssa = 0
         ```


      2. replace

         ```
                     ssa: 0,
         ```

         with

         ```
                     ssa: ssa,
         ```


      3. replace

         ```
                 fe.emit()?
         ```

         with

         ```
                 fe.emit()?
                 ssa = fe.ssa
         ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  43. **Strings numbered by last use.** `selfhost/src/emit_ll.nova`: replace

      ```
              match self.string_index.get(s) {
                  Some(i) => return i
                  None => ()
              }
      ```

      with nothing.

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  44. **`TaskTakeOutput`'s signature, never called, slips.** `selfhost/src/mir.nova`: `("nova_rt_task_take_output", "(I64) -> I64")` becomes `("nova_rt_task_take_output", "(I64) -> Ptr")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  45. **A string's length in characters.** `selfhost/src/emit_ll.nova`: replace

      ```
              let n = if s.byte_len() == 0 { 1 } else { s.byte_len() }
              out.push("@str
      ```

      with

      ```
              let n = if s.byte_len() == 0 { 1 } else { s.len() }
              out.push("@str
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  46. **The empty string without its NUL.** `selfhost/src/emit_ll.nova`, 2 edits, in order:

      1. `if s.byte_len() == 0 { out.push("\\00") } else { escape_bytes(out, s) }` becomes `escape_bytes(out, s)`.

      2. replace

         ```
                 let n = if s.byte_len() == 0 { 1 } else { s.byte_len() }
                 out.push("@str
         ```

         with

         ```
                 let n = s.byte_len()
                 out.push("@str
         ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  47. **A record's size without its floor of one field.** `selfhost/src/emit_ll.nova`: `let size = 8 * if fields.len() < 1 { 1 } else { fields.len() }` becomes `let size = 8 * fields.len()`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  48. **Every switch on `i64`.** `selfhost/src/emit_ll.nova`: replace

      ```
                      let ty = llty(self.temp_ty(disc))
      ```

      with

      ```
                      let ty = llty(MIR_I64)
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`.
  49. **Float `<=` unordered.** `selfhost/src/emit_ll.nova`: `OpLe => Some(if float { "fcmp ole" }` becomes `OpLe => Some(if float { "fcmp ule" }`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`.
  50. **No `main`'s words.** `selfhost/src/emit_ll.nova`: replace

      ```
      return Err("no `main` function in MIR module")
      ```

      with

      ```
      return Err("no `main` function in the MIR module")
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `each_error_of_the_emitter_is_in_s0s_words`).
  51. **An `Await`'s words.** `selfhost/src/emit_ll.nova`: `before a module is emitted",` becomes `before the module is emitted",`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `each_error_of_the_emitter_is_in_s0s_words`).
  52. **Float `%`'s words.** `selfhost/src/emit_ll.nova`: `OpRem => return Err("float remainder (%) is not supported by the LLVM backend")` becomes `OpRem => return Err("float remainder (%) is not supported by the backend")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_selfhost_tests_pass` (N's `@test` `each_error_of_the_emitter_is_in_s0s_words`).
  53. **A float bitwise operator's words.** `selfhost/src/emit_ll.nova`: `_ => return Err("bitwise operators are not defined for Float")` becomes `_ => return Err("bitwise operators are not defined on Float")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `each_error_of_the_emitter_is_in_s0s_words`).
  54. **No `main` not refused.** `selfhost/src/emit_ll.nova`: replace

      ```
          if !has_main { return Err("no `main` function in MIR module") }
      ```

      with nothing.

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `each_error_of_the_emitter_is_in_s0s_words`).
  55. **A `Ptr` parameter as `I64`.** `selfhost/src/mir.nova`: `("nova_rt_println", "(Ptr) -> Unit")` becomes `("nova_rt_println", "(I64) -> Unit")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`, `the_selfhost_tests_pass` (N's `@test` `a_small_module_is_the_header_its_function_and_the_c_main`, `every_runtime_function_has_a_distinct_symbol_and_its_signatures_declaration`).
  56. **An `I64` parameter as `Ptr`.** `selfhost/src/mir.nova`: `("nova_rt_alloc", "(I64) -> Ptr")` becomes `("nova_rt_alloc", "(Ptr) -> Ptr")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  57. **An `F64` parameter as `I64`.** `selfhost/src/mir.nova`: `("nova_rt_float_to_str", "(F64) -> Ptr")` becomes `("nova_rt_float_to_str", "(I64) -> Ptr")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  58. **An `I8` parameter as `I64`.** `selfhost/src/mir.nova`: `("nova_rt_bool_to_str", "(I8) -> Ptr")` becomes `("nova_rt_bool_to_str", "(I64) -> Ptr")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  59. **No parameter as one `I64`.** `selfhost/src/mir.nova`: `("nova_rt_int_hash_seed", "() -> I64")` becomes `("nova_rt_int_hash_seed", "(I64) -> I64")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  60. **A `Ptr` return as `I64`.** `selfhost/src/mir.nova`: `("nova_rt_int_to_str", "(I64) -> Ptr")` becomes `("nova_rt_int_to_str", "(I64) -> I64")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  61. **An `I64` return as `Ptr`.** `selfhost/src/mir.nova`: `("nova_rt_str_cmp", "(Ptr, Ptr) -> I64")` becomes `("nova_rt_str_cmp", "(Ptr, Ptr) -> Ptr")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  62. **An `F64` return as `I64`.** `selfhost/src/mir.nova`: `("nova_rt_str_to_float", "(Ptr) -> F64")` becomes `("nova_rt_str_to_float", "(Ptr) -> I64")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  63. **An `I8` return as `I64`.** `selfhost/src/mir.nova`: `("nova_rt_str_eq", "(Ptr, Ptr) -> I8")` becomes `("nova_rt_str_eq", "(Ptr, Ptr) -> I64")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  64. **A `Unit` return as `I64`.** `selfhost/src/mir.nova`: `("nova_rt_check_bounds", "(I64, I64) -> Unit")` becomes `("nova_rt_check_bounds", "(I64, I64) -> I64")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  65. **A runtime function's symbol.** `selfhost/src/mir.nova`: `("nova_rt_print", "(Ptr) -> Unit")` becomes `("nova_rt_prints", "(Ptr) -> Unit")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  66. **`F64` spelled `float`.** `selfhost/src/emit_ll.nova`: replace

      ```
              "double"
      ```

      with

      ```
              "float"
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`, `the_selfhost_tests_pass` (N's `@test` `a_symbol_both_a_function_and_an_extern_returns_the_externs_class`, `each_error_of_the_emitter_is_in_s0s_words`, `every_runtime_function_has_a_distinct_symbol_and_its_signatures_declaration`).
  67. **`I8` spelled `i1`.** `selfhost/src/emit_ll.nova`: replace

      ```
              "i8"
      ```

      with

      ```
              "i1"
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`, `the_selfhost_tests_pass` (N's `@test` `every_runtime_function_has_a_distinct_symbol_and_its_signatures_declaration`).
  68. **The `RtTable`'s return class read one off.** `selfhost/src/emit_ll.nova`: replace

      ```
                      let ret = self.rt.rets.at(func)
      ```

      with

      ```
                      let ret = self.rt.rets.at(if func + 1 < self.rt.rets.len() { func + 1 } else { func })
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  69. **The `RtTable`'s symbols one off.** `selfhost/src/emit_ll.nova`: replace

      ```
              symbols.push(symbol)
      ```

      with

      ```
              symbols.push(if f + 1 < table.len() { rt_func_symbol(f + 1) } else { symbol })
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  70. **The C wrapper calls `nova_main` quoted.** `selfhost/src/emit_ll.nova`: `out.push("  call void @nova_main()\n  ret i32 0\n}\n")` becomes `out.push("  call void @\"nova_main\"()\n  ret i32 0\n}\n")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`, `the_selfhost_tests_pass` (N's `@test` `a_small_module_is_the_header_its_function_and_the_c_main`).
  71. **The C wrapper without its `entry:` block.** `selfhost/src/emit_ll.nova`: `out.push("define i32 @main(i32 %argc, ptr %argv) {\nentry:\n")` becomes `out.push("define i32 @main(i32 %argc, ptr %argv) {\n")`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`, `the_selfhost_tests_pass` (N's `@test` `a_small_module_is_the_header_its_function_and_the_c_main`).
  72. **A callee `ret_of` lacks returns `I64`.** `selfhost/src/emit_ll.nova`: replace

      ```
                      let ret = match self.ret_of.get(callee) {
                          Some(c) => c
                          None => MIR_UNIT
                      }
      ```

      with

      ```
                      let ret = match self.ret_of.get(callee) {
                          Some(c) => c
                          None => MIR_I64
                      }
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `a_symbol_both_a_function_and_an_extern_returns_the_externs_class`).
  73. **N's `(Ll` part for a root that did not lower.** `selfhost/src/main.nova`: replace

      ```
                  Err(_) => ()
              }
          }
          Some(DumpedText(text))
      ```

      with

      ```
                  Err(_) => return Some(DumpedText(text + "(Ll ok)\n"))
              }
          }
          Some(DumpedText(text))
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  74. **N's `.ll` with a line more.** `selfhost/src/main.nova`: replace

      ```
              Ok(ir) => "(Ll ok)\n" + ir
      ```

      with

      ```
              Ok(ir) => "(Ll ok)\n" + ir + "\n"
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `n_lowers_the_depth_cases_on_a_1_mb_main_thread`, `the_mir_dump_has_a_header_per_root_and_says_how_far_each_got`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`, `the_mir_fuzzers_inputs_agree_and_hold_each_kind`.
  75. **N's error's words not quoted.** `selfhost/src/main.nova`, 2 edits, in order:

      1. replace

         ```
                     part.push("(Ll error \"")
         ```

         with

         ```
                     part.push("(Ll error ")
         ```


      2. replace

         ```
                     part.push("\")\n")
         ```

         with

         ```
                     part.push(")\n")
         ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `every_form_the_emitter_handles_occurs_in_the_compared_ll`, `the_mir_dumps_agree_on_the_cases_with_windows_line_endings`, `the_mir_dumps_agree_on_the_corpus_and_the_cases`.
  76. **N's error's words in quotes, not escaped.** `selfhost/src/main.nova`: replace

      ```
                  push_escaped(part, e, true)
      ```

      with

      ```
                  part.push(e)
      ```

      - Run `SH --exact $LL $GUARD`.
      - Expected: passes, by design: equivalent, as mutant 13 on S0's side: `push_escaped(part, e, true)` escapes the same bytes as S0's `quote`, and none of N's four error words, S0's words, holds one (N's MIR has no `Mem` statement, so S0's fifth has nothing to port), so `part.push(e)` writes the same bytes. Dropping the quotes is mutant 75, caught.
  77. **N takes `--ll` twice.** `selfhost/src/main.nova`: the line `if ll { bad = true }` goes.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `both_sides_refuse_the_mir_dumps_shapes_alike`.
  78. **N takes `--ll` with `--before-async`.** `selfhost/src/main.nova`: `if bad || std_dir == "" || files.is_empty() || (before && ll) {` becomes `if bad || std_dir == "" || files.is_empty() {`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `both_sides_refuse_the_mir_dumps_shapes_alike`.
  79. **N takes `--ll=…`.** `selfhost/src/main.nova`: `} else if a == "--ll" {` becomes `} else if a.starts_with("--ll") {`.
      - Run `SH --exact $LL $GUARD`.
      - Expected: fails:
        `both_sides_refuse_the_mir_dumps_shapes_alike`.
- [ ] **Step 9: The review's inputs.** The full run's figures, the new
  tests by binary and `selfhost.rs`'s apart, the golden suite's line and
  time, the parse, check, package, MIR dump, help and usage comparisons,
  the differential's times, N's memory and stack, and the mutants'
  outcome, with each gap, go in the ledger.
- [ ] **Step 10: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec
    and its amendment, ADR 0058 and this plan, and is told to hunt for:
    - a Windows-only difference: a `.ll` line `settled_stdout` rewrites, a
      line ending that reaches the `.ll` otherwise than through a
      literal, a root list past `HIR_ARGS_LIMIT` (Review Focus 1);
    - N's port against S0's, word for word: each escape, the float
      spelling, a length in characters, `ret_of`'s order and its missing
      callee, `%rN` per function, the strings' numbering, each layout's
      size, the switch's class, each predicate and instruction, each
      error's words, each runtime function's symbol and signature in
      `RtFunc::ALL`'s order, the C wrapper (Review Focus 2);
    - the `(Ll` part on both sides: printed for a root that did not
      lower or for the before form, the text altered, the last line break,
      an error's words quoted otherwise than the dumps quote (Review
      Focus 3);
    - argument handling: `--ll` shown in a help or usage line (but for
      clap's usage line for `--ll=x`, `Usage: nova check --dump-mir --ll
      [FILES]...`, recorded in decision 14 and the amendment), a base
      command line answered differently, a shape refused on one side and
      taken on the other, the refusal's order (Review Focus 4);
    - a `.ll` that is not what `build --release` writes: another module,
      another option, the splice against the full check; the test under
      `RUST_BACKTRACE=1` (Review Focus 5);
    - a way for the harness to pass when the two sides differ: a `.ll`
      cut off before the comparison, a root or set compared without
      `--ll`, a form missing from the coverage's lists or wrongly in
      `LL_UNREACHABLE`, a count or floor dropped (Review Focus 6);
    - the `.ll` mismatch report: a line counted wrong, a `define` or a
      block named from the wrong line, a line outside any function named
      inside one, another dump's report changed (Review Focus 7);
    - N's stack, memory and time against ADR 0058 §11 to §13 (Review
      Focus 8; decision 17);
    - a commit between Task 1 and Task 4 that builds wrong code or fails
      the suite, and the transitional mechanism the head does not have
      (decisions 11 and 12);
    - the records: ADR 0058, the CHANGELOG, the dated notes, the spec's
      amendment, every count (251 loose programs and 10 packages' mains,
      210 printing a `.ll`; 342 HIR cases and 103 roots of `cases/mir/`,
      423 printing a `.ll`; 17 new cases, 14 of `ir_tests.rs`; 190 forms,
      183 found, 7 unreachable; 111 runtime functions; 120 fuzzer inputs,
      25 depth programs) and figure against the tree, CI's runs (decision
      9);
    - each mutant of Step 8 that passes, the accepted ones included, and
      whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock` or to a manifest,
      a test that touches the internet, a temporary name that is not
      fixed or missing from the Global Constraints' list.
  - Each finding is fixed in one commit, `5.5a: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, the Conventions' checks run again in full: `cargo
    fmt`, both clippy commands on the local stable and again on
    `+1.100.0` (its target directory, `$P/target-1100`, made afresh if
    Task 4 removed it, after `df -h /` shows at least 6G; removed again
    after), and both `nova fmt --check` lines. Then Steps 1 and 2, `SH`,
    and every mutant of Step 8 whose file a fix changed, each with its
    commands and expected outcome as Step 8 gives them, in Step 8's
    batches.
- [ ] **Step 11: The PR.**
  - **Stop here, and have the main session ask the user** whether to push
    the branch and open the PR (Global Constraints). No push of this
    branch has been allowed: the user's "Push + draft PR" of 2026-11-18
    covered only the prototype's `…-55-ci` and draft Sakeerin/nova#144.
    Nothing is pushed, and no PR opened, before their word; record their
    answer in the ledger.
  - Before asking, check that every commit the PR would carry ends with
    the two trailer lines of the Conventions: `git log --format=%B
    origin/main..HEAD`, its commits `db640e97` (the spec's), the plan's,
    each task's and the review's fixes.
  - On the user's word: push, `git push -u origin
    claude/loving-archimedes-frtf5q-55`, then open the PR to `main` with
    the repository's template headings. The body names no local path, no
    person and no model ID. It ends with this line, a blank line, and the
    session's address, exactly:

    ```
    🤖 Generated with [Claude Code](https://claude.com/claude-code)

    https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
    ```

  - It holds:
    - its commits: first `db640e97`, the spec's; then the plan's, with
      the spec's amendment (decision 18); then each task's, and the
      review's fixes;
    - what 5.5a adds: S0's hidden `--ll` of the MIR dump; N's `.ll`
      emitter, `RtFunc`'s symbols and signatures, and `selfhost mir
      --ll`; the 17 cases, the coverage test, the `.ll` compared on every
      input and the `.ll` mismatch report; ADR 0058 and the notes, with
      the rule (decision 8);
    - what a user sees change: nothing; every `.nova` file the base
      tracks parses, checks and dumps its MIR as before, each of the 11
      packages outside `crates/` tests and builds as before (`pkgcmp.sh`,
      22 runs; the test fixtures under `crates/` were not compared),
      every `--help` is unchanged, and the golden suite is 147 as before;
    - the departures, marked **(differs)** here: decision 11 (the cut)
      and, with it, ADR 0058 §15's departures from the spec, decision 14
      (14 `ir_tests.rs` cases; 406 and 423 cases lowering, not 407;
      clap's usage line for `--ll=x`, against spec §3.1, §3.3 and §16's
      gate; `05-geometry` for §3.3; N's root +92 MB; the `ret_of` mutant
      not failing the differential, against spec §6.4), each recorded in
      the spec's amendment;
    - the figures: the Test job's suite `2837 1 10` and `selfhost.rs`'s 67
      apart, the golden suite's `147 passed, 9 excluded`, `selfhost.rs`'s
      times here and CI's from runs 44259975902 and 44260509656 and
      `main`'s 43853141585, N's memory and stack, `Cargo.lock`
      unchanged;
    - the user's decisions and this plan's, numbered as here, with
      decisions 11 and 14 marked as differing from the spec;
    - the mutants, those that pass by design, and the review's findings
      and what came of each.
  - Read CI once it ends. Expected: green on all three systems; on the
    Test job's `ubuntu-24.04` leg, `cargo test`'s log holds `test
    release_golden_suite ... ok` and no `tests/selfhost.rs`; on each
    system the Selfhost job passes `selfhost.rs` and then
    `the_selfhost_tests_pass` on the debug runtime; the counts are
    decision 19's: `selfhost.rs` 67 on Linux, 65 on macOS, 61 on Windows;
    the Test job's main step,
    `cargo test --locked --workspace --all-features --no-fail-fast`, its
    `test result:` lines summed, `2838 passed, 0 failed, 10 ignored` on
    Linux, `2838 passed, 0 failed, 9 ignored` on macOS and
    `2815 passed, 0 failed, 9 ignored` on Windows; the advisory
    `-- --ignored` step after it is not counted, and what it shows is
    recorded as it is. Record each system's time for `selfhost.rs`, the
    jobs' totals and the run's address in the ledger, and compare each
    with run 44260509656's in the PR's body.
  - **If the differential fails on CI and not here,** record its report,
    stop and ask the user: a Windows path, a line ending or a command
    line is the likeliest (Review Focus 1). **If `selfhost.rs` takes more
    than 110 s on Windows, 100 s on macOS or 80 s on Linux,** ask the user
    (decisions 7 and 19). Never lower a count.
- [ ] **Step 12: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 11's.
