# Phase 5.6a, "the bootstrap gate: S1 == S2 == S3 in CI", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** CI holds the bootstrap fixed. A new step of the Selfhost job's
Linux leg, after 5.5b's golden step, runs `the_bootstrap_holds_s1_s2_s3`,
an ignored test of `selfhost.rs`: S0's `nova build --release` builds N
into S1, then S1's own `build` builds N into S2, then S2's builds it into
S3, strictly in order, with one `-o` basename, `selfhost`, and one
runtime archive, the release one that S0 carries; beside each build its
builder makes N's `.ll` in a front-end-only run, `NOVA_CLANG` (and S0's
`NOVA_LLC`) at a file that is not there, so it exits 1 and leaves the
`.ll`. N's `.ll` from S1 and from S2 must equal S0's, and S1 == S2 == S3,
byte for byte. A mismatch is reported by the `.ll`'s line, `define` and
block (`ll_difference`), or by both binaries' sizes and first differing
offset (`same_binaries`, now given the stages' names), and the files are
kept; a stage that fails stops the chain with its command, status and
stderr. The step prints `cc`'s and `ld`'s versions. Where S0 carries
its runtime, as on CI, the gate also fails unless S2's and S3's runtime
is the one S0 unpacked under its own `home/` (decision 22). The main
step gains `the_bootstrap_reports_name_the_stage` (the two reports, and
the four comparisons on files laid out as the gate lays them out; no
`clang`, all three systems), and `the_ci_step_names_every_clang_test`
holds both clang steps, each to `NOVA_REQUIRE_CLANG: "1"` and `if:
runner.os == 'Linux'`, and the bootstrap step to `--nocapture`, `cc
--version` and `ld --version`. Nothing a user of S0 sees changes: the
diff touches no S0 or N source, std, manifest or lockfile,
and the golden suite stays 147. ADR 0060 records the gate's tolerances,
none on the bytes and four conditions, what varies the binary and what
does not, the search of N for `Map` iteration, and the figures, CI's
among them; `selfhost/README.md` says what N is and how its chain and its
tests run.

**Architecture:**
- **The comparisons and their reports** (Task 1):
  `crates/nova-cli/tests/selfhost.rs`'s `same_binaries`, given (name,
  path) pairs, and its two callers; `ll_difference`, `stage_binary`,
  `stage_ll` and `bootstrap_failures`; the report test.
- **The gate and its CI step** (Task 2): `runtime_in(home)` under
  `golden_runtime()`; `the_bootstrap_holds_s1_s2_s3`;
  `the_ci_step_names_every_clang_test` reading two steps and a `run: |`
  block, with their `if:`, and the bootstrap step's `--nocapture`, `cc`
  and `ld`; the module doc's 5.6a paragraph; `.github/workflows/ci.yml`'s
  step "The bootstrap, S1 == S2 == S3 (Linux)".
- **Records** (Task 3): ADR 0060 (new); `selfhost/README.md` (new);
  `CHANGELOG.md`; dated notes in `docs/phase-5-plan.md` and
  `nova-spec/00-MASTER-SPEC.md`.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` and every `Cargo.toml` stay as they are**
  (spec §7, §10). Binaries are compared byte for byte; no hash crate.
- The gate runs S0, the N it builds, `clang` (CI's `clang-18`, 18.1.3)
  and `cc`. S0's and N's sources, std, the emitter, the runtime and
  `compare.rs` are unchanged, so ADR 0058 §16's rule is not engaged.

**Spec:**
`docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`,
approved by the user on 2026-11-22 (`45a06d1d`) with one change, the
literal chain (decision 9), and its dated amendment of 2026-11-22, which
this plan's commit appends (decision 20).
- Read its header, §1 to §14, and the amendment, before Task 1.
- The user's decisions are decisions 1 to 8 here, the approval's change
  decision 9, and the answer that allowed the prototype's push to CI
  decision 10.

**Base:** `45a06d1d` ("docs: the Phase 5.6a spec is approved"), `main`
`83ba8210` (5.5b's merge) and the spec's commit, whose full suite gives
`2837 1 10` here (the IPv6 test the only failure) and `selfhost.rs` 70
passed and 2 ignored. The branch is `claude/loving-archimedes-frtf5q-56`,
from `45a06d1d`, then this plan's commit (decision 20).

**Checked on 2026-11-22 and 2026-11-23, before execution** (the mutant
run and `selfhost.rs` with `RUST_BACKTRACE=1` on 2026-11-23):
- **A prototype built 5.6a whole** on a local branch from `45a06d1d`:
  `676c6515` (the bootstrap test, its reports and the CI step),
  `b90469a9` (the comparisons as a function, tested on laid-out files),
  `ce45a585` (a failed stage's status in plain words) and `a453c61e`
  (ADR 0060, `selfhost/README.md`, the CHANGELOG and the notes). Then
  three commits for this plan: `0669e9d0` (ADR 0060 §10, CI's figures,
  in place of "pending"; docs only), `b829f8dc` (the three cases the
  plan's mutants asked for, decision 17) and `1daf27c7` (ADR 0060, the
  README and the CHANGELOG naming them). Then three after the plan's
  first fact-check: `e84a41be` (the CI step test holds each clang step's
  `if:`, and the bootstrap step's `--nocapture`, `cc` and `ld`; decision
  17), `5c3d0584` (the gate's check of the runtime it hands on; decision
  22) and `6024caef` (ADR 0060, the README and the CHANGELOG after
  them). Then two after the second fact-check, docs only: `fa766425`
  (ADR 0060 §10: run 45003631229 ran `a453c61e` only, and CI has run
  none of the cases and checks added since) and `f24c008c` (§10 names
  those changes by what they are, with no hash of a commit off
  `origin`). The three commits the records name (`45a06d1d`,
  `83ba8210`, `a453c61e`) are each on `origin`: `83ba8210` on `main`,
  the other two on the pushed `claude/loving-archimedes-frtf5q-56-ci`.
- **CI, measured from the prototype** (decision 10), on
  `claude/loving-archimedes-frtf5q-56-ci` with draft Sakeerin/nova#148,
  closed unmerged, run 45003631229 on `a453c61e`: 19 of 19 jobs green.
  - **The bootstrap step:** 27.58 s wall, the test `27.44 s in all`: S1
    by S0 8.69 s, S2 by S1 9.45 s, S3 by S2 9.23 s, the front-end runs
    beside them 0.43, 1.29 and 1.25 s, the comparisons 0.07 s. It
    printed `cc` 13.3.0 and GNU ld 2.42, and S1 == S2 == S3 at 9,539,736
    bytes each, N's `.ll` at 12,760,111 bytes, on CI's embedded runtime
    `0.4.0-5549317a`, a different file from this machine's (decision 15).
  - **`selfhost.rs`'s main step** (`finished in`): 58.19 s on Linux
    (`71 0 3`), 30.03 s on macOS (`69 0 3`) and 91.31 s on Windows (`65 0
    3`), margins 21.81, 69.97 and 18.69 s against 80, 100 and 110 s.
  - **The golden step** 30.30 s; MSRV (1.78) passed; the Test job's main
    step `2838 0 10`, `2838 0 9` and `2815 0 9`; the advisory step that
    runs the ignored GC tests exited 101 on Linux and macOS, as on
    `main`, and is not counted. The Selfhost jobs took 4:01, 1:35 and
    5:32; the longest job, the Test job on Windows, 7:44.
  - **`main` at `83ba8210`,** run 44962982900: green; `selfhost.rs`
    58.13 s on Linux, 83.52 s on macOS and 78.89 s on Windows (`70 0 2`,
    `68 0 2`, `64 0 2`); the golden step 31 s; the Selfhost jobs 3:19,
    3:37 and 4:36.
- **The split** (decision 12 **(differs)**): two tasks test first, then
  the records. `split.py` makes the local branch `split/5-6a` from
  `45a06d1d`, one commit a patch: Task 1 `ce436685`,
  `cd50afd1`; Task 2 `e3dd432e`, `9440da89`; Task 3
  `11a0d088`. Its last tree is the prototype's `f24c008c` (`git diff
  f24c008c split/5-6a` prints nothing), and every patch applies with
  plain `git apply`, in order, on `45a06d1d`, with no output, each
  applied tree its commit's; no added line holds trailing whitespace, a
  CR or a backslash-u escape. **These hashes are references, not commits
  the executor makes:** the executor applies each task's patches and
  makes one commit per task (Conventions), with hashes of its own; each
  task's commit has the tree of that task's last split commit here.
- **The replay** (`replay.sh`, under the process limit, without
  `NOVA_EMBED_RUNTIME`, as the Test job runs), each step as written here,
  on the split's commits: each tests commit failed as its Step 2
  records, and each code commit passed the whole suite in three runs
  (`S`: `2837 1 10`, the IPv6 test the one failure) and `selfhost.rs`
  apart (`SH`: 71 passed and 2 ignored at Task 1, in 217 s; 71 and 3 at
  Tasks 2 and 3, in 218 and 209 s), `cargo fmt`, both clippy commands
  on the local stable (1.97) and on `+1.100.0` (CI's stable),
  `selfhost.rs`'s again with its `#[cfg(unix)]` items turned off
  (Windows mimicked), and `nova fmt --check` with CI's arguments and the
  earlier plans' superset; the gate at Task 2's tests and code commits,
  on the debug runtime, and G1's red by hand at Task 2's tests commit
  (Task 2, Step 2). The split was made from `1daf27c7`, then again
  from `6024caef` after the first fact-check's commits: Task 1's two
  commits came out the same, hashes included (its trees do not hold the
  changed code), and were not replayed again; Task 2's and Task 3's
  were, each item as above, Task 3's at `6edde36e`. It was made twice
  more after the second fact-check, from `fa766425`, then from
  `f24c008c`: each time only Task 3's commit changed, from `6edde36e` to
  `9758355c`, then to `11a0d088`, and `git diff --stat 6edde36e
  11a0d088` lists ADR 0060 alone, 11 lines added and 4 removed. No
  cargo replay was run for those changes, for three reasons: they are
  docs only; no test reads the records; and `selfhost.rs` is
  unchanged. The Windows mimic was also run as the
  Conventions write it on uncommitted trees, each task's patches applied
  on the previous code commit, at the first split (Task 2's and Task 3's
  `selfhost.rs` have changed since, only inside two tests): the file came
  back byte for byte, and `git status --short` listed the task's files.
- **At the second split's last commit,** `6edde36e` (Task 3's then; its
  tree differs from `11a0d088`'s only in ADR 0060's text, which no
  build or test reads). Every figure this plan gives "at the last
  commit" was measured there:
  - the whole suite in three runs: `2837 1 10` (as Task 4, Step 1 runs
    it); the test list 2,848 as at the base, the same names;
    `selfhost.rs` 74 listed, the base's 72 and two added (one ignored);
  - no S0 source changed: `git diff 45a06d1d HEAD` over the whole tree
    but `crates/nova-cli/tests`, `selfhost/`, `docs/`, `nova-spec/`, the
    CHANGELOG and `.github` prints nothing (decision 13); `Cargo.lock`
    unchanged;
  - `selfhost.rs` without the release runtime: 71 passed, 3 ignored, in
    209.13 s, and with `RUST_BACKTRACE=1` in 202.87 s; as the Selfhost
    job runs it (`NOVA_EMBED_RUNTIME=1`, `CARGO_PROFILE_DEV_OPT_LEVEL=1`),
    three times: 64.65, 66.73 and 66.65 s; the three ignored tests
    together, as `C` names them, `3 passed` in 53.19 s; the bootstrap
    step's whole `run: |` block twice, `finished in` 44.14 and 44.16 s,
    `S1 == S2 == S3, 9354680 bytes each`; the job's last step `1 passed`
    in 2.18 s.
- **At the first split's last commit,** `7c3cb7f0` (its tree the
  prototype's `1daf27c7`), and not run again:
  CI's Linux leg, `--all-features`, the golden suite required,
  `RUST_BACKTRACE=1`, in three parts: `1300 1 8`, `2 0 0` and `1535 0 2`
  in 154, 1 and 238 s, the golden suite inside the third; the golden
  suite, `release golden: 147 passed, 9 excluded, Ubuntu clang version
  18.1.3 (1ubuntu1)`, in 25 s; Rust 1.78, `cargo check` clean,
  `Finished` in 53.99 s. That tree differs from this one only in
  `selfhost.rs` and the records, which none of the three builds or
  runs.
- **The mutants** (decision 17): 74 breakages of what 5.6a adds, all run
  one at a time, in one run, at the second split's last commit,
  `6edde36e`, with `NOVA_EMBED_RUNTIME=1` (Task 4, Step 8), and not run
  again at `11a0d088`: the two trees differ only in ADR 0060's
  text, which no mutant edits and no test reads, and Task 4, Step 8 runs
  every mutant again at the executing branch's tip. They are the
  prototype's 23, and 51 for every check the diff adds that they miss
  (the two reports' words,
  the CI step test and the step, the gate's own report and checks, the
  runtime's handoff between stages, `runtime_in`, and the no-clang skip
  under `NOVA_REQUIRE_CLANG`). 53 are caught; 21 are accepted, each with
  its reason: 13 change output only, 4 are redundant with another check,
  3 are equivalent, and 1 is reached only when a stage fails. The runs
  add up to 2,174 s, 36 minutes, in ten batches of 187 to 243 s.
- **What the checks left standing,** each recorded or handed on:
  - macOS and Windows are reasoned from the code here, and CI ran only
    the prototype's `a453c61e`. This split's last tree differs from it by
    ADR 0060 §10's figures (`0669e9d0`), the three cases (`b829f8dc`: two
    inside the report test, one in the CI step test, all in the main
    step), the CI step test's new checks (`e84a41be`, in the main step),
    the gate's runtime check (`5c3d0584`, in the bootstrap step) and the
    records (`1daf27c7`, `6024caef`, `fa766425`, `f24c008c`). Those
    cases and checks ran on this machine only, on Linux: CI has run none
    of them, on any system (the CI step test reads both files as text,
    CRLF line endings allowed).
    The PR's runs are their first on CI;
  - the gate's own words, its failure lines and its success lines, are
    read by no test (decision 17);
  - the binaries' size and digest are the tree's and the runtime's: this
    machine's 9,354,680 bytes, CI's 9,539,736 on run 45003631229,
    recorded, not expected (decisions 15 and 21).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` under `RUSTUP_TOOLCHAIN=1.78.0`
  with `RUSTFLAGS=-D warnings`. No `std::sync::LazyLock` (1.80), no
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79); none is
  used. 5.6a's Rust is all test code, in `selfhost.rs`. A scan of the
  diff's added Rust lines (`git diff 45a06d1d <tip> -- '*.rs'`, its `+`
  lines: 657 of them, 541 without the comment lines), its string
  literals blanked, lists every method call (60 names), path (15) and
  macro (8) in them, and searches them for the language forms named
  here. The newest of those names and forms are: `Option::is_some_and`
  and `Result::is_err_and` (1.70); `let … else` (1.65);
  `std::thread::scope` and its `spawn` (1.63); format strings' captured
  identifiers (`{stage}`, `{want:?}`, 1.58); arrays iterated by value
  (`for (a, b) in [("S0", "S1"), ("S0", "S2")]`, and `Vec::extend` from
  an array of `String`s, 1.53); `Result::as_deref`
  (`std::env::var("NOVA_REQUIRE_CLANG").as_deref()`, 1.47);
  `str::strip_prefix` (1.45); `matches!` (1.42); and `Option::as_deref`
  (`runtime.as_deref()`, 1.40). Older ones it finds include
  `Duration::as_secs_f64` (1.38), `Iterator::copied` (1.36),
  `std::fs::read`, `std::fs::write` and closures' default binding modes
  (`.find(|(test, _)| test == name)`, 1.26), `Ord::min` on integers
  (1.21), `eprintln!` (1.19), `'static` elided in a `const` (`const
  STEPS: [&str; 2]`, 1.17) and `?` on a `Result` (1.13, inside
  `same_binaries`' and `ll_difference`'s closures). Each is at most
  1.78. The claim covers the names the scan lists and the forms searched
  for, no more. The MSRV job checks no test target, and Cargo 1.78
  cannot check one here (the dev-dependency `assert_cmd` 2.2.1 needs the
  2024 edition), so this scan is the check of the tests' Rust;
  `selfhost.rs` was not compiled on 1.78 for this plan. The workspace's
  libraries and binaries were: `cargo check` under 1.78 is clean (Task
  4, Step 6; run at the first split's last commit, and 5.6a changes no
  library or binary).
- **No new crate; `Cargo.lock` and every `Cargo.toml` stay as they are**
  (spec §7, §10). Binaries are compared byte for byte, no hash crate. If a
  step seems to need a crate or a manifest change, stop that task there
  and ask the user; do not add the crate or change a manifest first.
- **No `unwrap()` outside `#[cfg(test)]` and test files.** 5.6a's Rust is
  test code alone.
- **No `unsafe`.** 5.6a adds none.
- **Tests never touch the internet** and bind no port (spec §7). The gate
  runs `nova`, the N it builds, `clang` from `NOVA_CLANG` or `PATH` and
  `cc` from `PATH`; `clang` is installed by CI, never by a test. Without
  `clang` the gate prints `bootstrap: not run (no clang)` and passes, and
  fails instead under `NOVA_REQUIRE_CLANG=1`. The report test and the CI
  step test need no `clang`, no N and no network.
- **Temporary names are fixed** (spec §3.1, §7): under
  `std::env::temp_dir()`, through `fresh_dir`, which removes and makes
  each afresh. 5.6a adds `nova-selfhost-bootstrap` (the gate's: `s1/`,
  `s2/`, `s3/`, each stage's build; `ll-s0/`, `ll-s1/`, `ll-s2/`, each
  front-end run's; `home/`, S1's build's `NOVA_HOME`; `home-ll/`, the
  front-end runs'; `no-such-tool`, never created) and
  `nova-selfhost-bootstrap-report` (the report test's, Task 1). These are
  every fixed temporary name the diff adds (`grep -n 'fresh_dir(\|temp_dir'`
  over its added lines); `golden_runtime` reads `nova-selfhost-home`,
  `build_selfhost`'s since 5.3a, as before. The gate removes its
  directory on success and keeps it on a failure.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and the golden suite's directory is one of them.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the suite here with `NOVA_REQUIRE_NODE=1`. No test of 5.6a needs Node.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and
  passes. 5.6a does not touch it: 147 passed, 9 excluded.
- **The runtime the gate links.** S1's build runs with `NOVA_HOME` at the
  gate's fresh `home/`, and S0 links, in its order (`link.rs`),
  `NOVA_RUNTIME_LIB`, else the runtime it carries, unpacked into
  `home/runtime/<version>-<crc32>/`, else the one beside it; the gate
  takes the same file by the same order (`runtime_in(home)`) and hands it
  to S2's and S3's builds. In a `nova` built with `NOVA_EMBED_RUNTIME=1`,
  as CI's Selfhost job builds it, that is the release runtime (9,354,680
  bytes a binary here), and the gate fails unless it lies under
  `home/runtime/` (decision 22); in a plain checkout it is the debug
  runtime beside `target/debug/nova`, and the chain holds there too, at
  14,288,496 bytes (decision 14). Every run step starts with `cargo build
  --locked --workspace` (Conventions), so that runtime is the tree's.
- **`selfhost.rs` is not in `cargo test --workspace`:** its target says
  `test = false` (5.3a). `S` runs the whole suite as the Test job does,
  `SH` runs `selfhost.rs` as the Selfhost job's main step does (the three
  ignored tests skipped), and `C` (Conventions) runs the ignored tests it
  names as the clang steps run them; every figure here counts them apart.
- **`NOVA_EMBED_RUNTIME` and `CARGO_PROFILE_DEV_OPT_LEVEL` are set for one
  command only, or for a whole step that says so** (Task 4, Steps 7 and
  8): a change of either makes cargo rebuild, and every other test that
  links a program then runs on that runtime or at that level. Each such
  step ends with `cargo build --locked --workspace` without them. Never
  build every test target at `opt-level = 1` (`cargo build --tests` or the
  whole suite): only `selfhost.rs`'s, as the Selfhost job does.
- **`RUST_BACKTRACE` is `0` in every shell here** (Conventions), and `1`
  for one command only where a step says so (Task 4, Steps 2 and 7):
  CI's jobs export `RUST_BACKTRACE=1` (5.5a's lesson, ADR 0058 §13).
- **The process limit.** The gate runs one build at a time, with `clang`
  and `cc`, beside one front-end-only run, which never reaches `clang`;
  the suite starts thousands of processes. Every test,
  mutant and suite runs under the limit of the Conventions, so that a
  mistake that forks without end fails to fork rather than filling the
  machine.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other. Never build in another worktree, and never run two
  suites, or a suite and `SH`, at once: the tests' fixed directories are
  shared.
- **Keep `df -h /` above 6G (6,144 MiB), and check it before each build, not
  after.** Read the thresholds in MiB: `df -h` rounds up, so its `7.0G`
  can be 7,071 MiB, under 7 GiB (7,168 MiB). Every `df` this plan asks
  for is logged, by this block, its `echo` naming the task, the step and
  what comes next; the steps call it "the disk check":

  ```sh
  mkdir -p "$P/exec56/logs"
  { echo "Task 4 Step 7, before the opt-level 1 build"; df -h /; df -m / | tail -1; } >> "$P/exec56/logs/df.log"
  tail -n 1 "$P/exec56/logs/df.log"    # its fourth field: the MiB free, read against the threshold
  ```

  Older duplicate test executables in `target/debug/deps` may be
  removed between commands, never while one runs, and so may
  `target/debug/incremental`; never `cargo clean` the main target.
  - **Before each change of opt-level or runtime** (Task 4, Steps 7 and
    8, and back): at least 7,168 MiB free, or first remove the other
    level's or runtime's `nova-*` and `selfhost-*` executables in
    `target/debug/deps` (each 100 to 230 MB; the next build at that
    level makes them again). A build at another `opt-level` keeps both
    sets; one at the same level with the other runtime replaces them in
    place. The replay, at the second split's last commit (`6edde36e`),
    read 6.5G (6,642 MiB) before Step 7's `opt-level = 1` build, removed
    the five `opt-level = 0` ones and read 7.1G (7,246 MiB); after that
    build 6.9G (7,027 MiB),
    and after `selfhost.rs`'s 6.6G (6,682 MiB). Before Step 8's embedded
    build at `opt-level = 0` it read 6.7G (6,846 MiB), removed the three
    `opt-level = 1` ones and read 7.1G (7,188 MiB); after the build 6.9G
    (6,981 MiB), after `selfhost.rs`'s 6.6G (6,686 MiB). After the
    mutants it read 6.5G (6,611 MiB), and before the plain build it
    removed the three embedded ones (and the mutants' `gate-tmp`),
    reading 6.8G (6,905 MiB). It went from Step 7's `opt-level = 1`
    build straight to Step 8's embedded build: Step 7's return to plain,
    followed by Step 8's embedded build, was not replayed.
  - **Before the Rust 1.100 clippy:** at least 6.5G (6,656 MiB), checked
    once before the first of its two commands, which run back to back;
    its target directory of its own, made by the first, takes about
    0.4G, which the second reuses. It is made for each code task's
    checks and removed right after them. The replay read 6.6G (6,730
    MiB) before Task 2's pair and 6.2G (6,325 MiB) after it. It also ran
    the pair at the second split's Task 3 commit, `6edde36e`, a run of
    its own, which Task 3 does not ask for (Task 3, Step 3): before it,
    6.5G by `df -h` but 6,642 MiB, 14 MiB under the threshold read in
    MiB (its script compared with 6,500), and after it 6.1G (6,237 MiB),
    the lowest reading of the replay.
  - **Before any other build:** at least 6.3G (6,451 MiB). A build at
    the same level and runtime replaces its executables in place, but
    cargo keeps two copies of a few executables (`run_tests`, `nova`,
    `registry`) built for different command lines.
  - Each run of the whole suite leaves directories under
    `/tmp/nova-todo-*`, which are never cleaned. The Rust 1.78 check
    (Task 4) builds under `/dev/shm`, removed after. The replay of Tasks
    2 and 3 began with 7.0G free (7,100 MiB).
  - **The replay's checks.** Its script, `replay.sh`, checks 6,451 MiB
    before each item, 6,656 MiB before the first Rust 1.100 command and
    6,144 MiB before the second (6,300, 6,500 and 6,100 until it was
    fixed, after Task 3's checks at the second split's `6edde36e`). The
    7,168 MiB checks before a change of level or runtime are in this
    plan's steps (Task 4, Steps 7 and 8), and the replay made them by
    hand, each time removing the other set first. Its readings above
    were recorded by hand as it went; this plan's runs log theirs, as
    above.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.6a's
  patches or scripts holds one. Apply the patches with `patch_block`, from
  the committed plan.
- **No patch adds a CR, a NUL or trailing whitespace.** The report test's
  `\x7fELF` bytes are escapes in literals.
- **Nothing writes into the repository but the patches.** The gate builds
  N from `selfhost/src/main.nova` in place, with every output under the
  system's temporary directory; `git status --short` is empty after every
  run (checked after the gate's local runs).
- **Push and open the pull request on the user's word only.** No push
  of this branch is allowed yet. The user's answer of 2026-11-22,
  "อนุญาต push -56-ci + draft PR (แนะนำ)", was for the prototype to
  `claude/loving-archimedes-frtf5q-56-ci` with draft Sakeerin/nova#148,
  measurement only, closed unmerged (decision 10); it does not cover this
  branch. Task 4, Step 11 stops before any push, and the main session
  asks the user; the branch is pushed and the PR opened only after their
  word.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.6a: the release is the
  next pull request (decision 1).
- **This machine is Linux.** macOS and Windows results come from CI; what
  differs there is reasoned from the code (Review Focus 5).

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Every shell exports it, `export P=…` with the
  directory's path, before a command that names `$P` (the Runs block
  below starts with it). Exported, not merely set: Task 4's runner reads
  it from the environment, as a process of its own.
- **The executor's directory:** `$P/exec56/` (`mkdir -p "$P/exec56"`
  once, in Task 0; the Runs block's `mkdir -p "$NOVA_HOME"` makes it
  too). Every script this plan writes (Task 4's `names.py`, `mutrun.py`
  and `mutants.json`) and every output goes there: the suite's logs, the
  disk checks' `logs/df.log`, the extracted patches, the commit
  messages, `$NOVA_HOME`, the Rust 1.100 target, the runtime's copy, the
  mutants' `gate-tmp` and Task 4's outputs. The one exception is
  `/dev/shm/msrv-56`, Task 4, Step 6's Rust 1.78 target, removed right
  after it. The one file kept outside it is the ledger (below), where
  every phase's ledger is kept.
- **The ledger:** `$P/ledger-56.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/exec56/msg-<task>.txt`:
  the subject the task names, a blank line, and these two trailer lines,
  exactly:

  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
  ```

  Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/exec56/msg-<task>.txt"`. A task with a
  tests patch and a code patch makes one commit, after both. Every commit
  of this plan ends with those two lines: the plan's own (Task 0), each
  task's and the review's fixes (Task 4, Step 10). The executor makes
  these commits; the split's commits named in "Checked on"
  (`ce436685` to `11a0d088`) are the replay's references,
  whose trees each task's commit should have. The pull request's body
  ends as Task 4, Step 11 gives it. No model ID (a `claude-…` model name)
  goes in any artifact: no commit, file, pull request, comment or ledger
  line names one.
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

  600 tasks was ample here: the replay's suites, builds, gates and
  mutants ran under it on 4 CPUs. If a test fails to start a thread, read
  the group's `pids.peak`, raise `pids.max`, and record it.
- **Runs.** The replay ran every command in the group and with these
  variables set. Each shell puts itself under the cap and sets them too,
  with four shorthands, and runs every command from the repository's
  root; a shell that is not under the cap stops at once. A tool that
  starts a fresh shell for each call, as the replay's did, runs this
  block, and every variable its step exports, in every call (5.5b's
  ruling R5.3):

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
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/exec56/home"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG NOVA_EMBED_RUNTIME CARGO_PROFILE_DEV_OPT_LEVEL
  mkdir -p "$NOVA_HOME"
  SH() { cargo test --locked --workspace --test selfhost -- "$@"; }
  S() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test run_tests -- release_golden_suite --nocapture "$@"; }
  C() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test selfhost -- --ignored --exact "$@"; }
  ```

  So `SH` runs `selfhost.rs` as the Selfhost job's main step does, `S`
  the whole suite without `selfhost.rs`, as the Test job runs it, `G`
  the golden suite through LLVM as CI's Linux leg runs it, and `C` the
  ignored tests it names, with `clang-18` required, as the clang steps
  run them. Without `NOVA_EMBED_RUNTIME` the gate links the debug runtime
  beside `nova` (Global Constraints), as the replay's task steps did.
  `NOVA_HOME` keeps a run by hand off `~/.nova`; every test that needs
  one sets its own.
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
    --workspace --no-run` as a command of its own (1 to 15 s), and
    `cargo test --locked --workspace --test selfhost --no-run` before
    `SH` (up to 10 s), so that neither run holds a build.
  - **`S` in three runs,** everywhere, as the replay ran it:

    ```sh
    S --lib --bins > "$P/exec56/s-a.txt" 2>&1                       # the libraries' and binaries' tests
    S --doc > "$P/exec56/s-b.txt" 2>&1                              # the doc tests
    S --test '[!s]*' --test 's[!e]*' > "$P/exec56/s-c.txt" 2>&1     # every integration test but selfhost.rs
    cat "$P/exec56/s-a.txt" "$P/exec56/s-b.txt" "$P/exec56/s-c.txt" | grep '^test result' | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
    ```

    At the base and at each code commit they gave `1300 1 8` in 142 to
    151 s (the IPv6 test the failure), `2 0 0` in 1 to 2 s and `1535 0 2`
    in 213 to 225 s, `2837 1 10` summed; `grep -c 'Running
    tests/selfhost.rs' "$P/exec56/s-c.txt"` prints `0`. The globs name
    every integration test but `selfhost`: a `--test` pattern takes a
    `test = false` target, which `--tests` leaves out, and `--tests`
    would run the libraries' tests again. A difference from the sum is
    recorded in the ledger. `S` neither builds nor runs `selfhost.rs`,
    and 5.6a changes nothing else that `S` builds, so `S`'s binaries are
    the base's at every commit.
  - Every single command here fits 600 s at twice the replay's time,
    the mutants' batches included, which are cut to at most 300 s each.
    The longest: CI's Linux leg's third part (238 s), `S`'s third (225
    s), `SH` without the release runtime (203 to 220 s), and the mutants'
    batches (187 to 243 s).
  - **A child killed at `CHILD_LIMIT`.** The harness kills a child
    process past 120 s (`CHILD_LIMIT`); the gate's slowest build here, on
    the debug runtime, took at most 25.30 s in the replays. If a test
    fails with a
    child killed there (`… did not finish within 120s: killed`), run that
    test alone once, `SH --exact <test>` (or `C` for an ignored one). If
    it is killed again, stop and report to the main session. Never raise
    `CHILD_LIMIT` without the main session.
  - If a command is killed by its timeout, or the shell restarts, restore
    what it left: after a killed mutant run, `git checkout -- <file>` of
    the mutant's file, and then `git status --short` must print nothing
    before anything goes on (then Task 4, Step 8 runs every batch again
    from mutant 1).
- **The build.** Each "Run" step below starts with

  ```sh
  cargo build --locked --workspace
  ```

  which ends with `Finished`, so that the `nova` the steps run by hand and
  the runtime library beside it are the tree's (Global Constraints).
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the
  repository's root:

  ```sh
  PLAN=docs/superpowers/plans/2026-11-22-phase-5-6a-the-bootstrap-gate.md   # this file, committed
  block() {
    awk -v name="$1" '
      $0 == "<!-- patch " name " -->" { grab = 1; next }
      grab && /^````diff$/ { inside = 1; next }
      inside && /^````$/ { exit }
      inside { print }
    ' "$PLAN" > "$P/exec56/$1.patch"
  }
  patch_block() {
    block "$1" && git apply --check "$P/exec56/$1.patch" && git apply "$P/exec56/$1.patch"
  }
  ```

  `patch_block t1-tests` applies the block named `t1-tests`, and prints
  nothing: no 5.6a patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. Task 3's patch adds
  903 lines (a block of 955): it is one
  block all the same. The non-ASCII characters the patches add are `§`
  (every patch) and `×`, `—` and `…` (Task 3). A block that does not
  apply means the tree differs from the plan's: stop, and record why in
  the ledger.
- **Each task** but Task 3 is test first: its tests' patch, their run and
  the expected failure, then its code's patch, its run and the expected
  pass. Task 3's patch is the records. Each task then runs its checks:
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
    minimal -c clippy` once, if `rustup toolchain list` lacks it), after
    the disk check (named for its task and step: `Task 1 Step 5, before
    the Rust 1.100 clippy`) shows at least 6,656 MiB (6.5G), once before
    the first command (the second, run right after it, reuses its target
    directory):
    `CARGO_TARGET_DIR="$P/exec56/target-1100" cargo +1.100.0 clippy …` with
    each command's arguments, each ending with `Finished` and no warning
    (37 to 39 s, then 2 s); then `rm -rf "${P:?}/exec56/target-1100"`. The
    local stable (1.97 here) and CI's lint differently;
  - **Windows, mimicked:** local clippy runs only Linux's items, and CI's
    Windows clippy once failed on a constant used only under
    `#[cfg(unix)]` (5.5b's lesson). So `selfhost.rs`'s clippy runs again
    with its `#[cfg(unix)]` items from `golden_runtime` on turned off,
    then the file is restored:

    ```sh
    F=crates/nova-cli/tests/selfhost.rs
    cp "$F" "$P/exec56/selfhost.rs.keep"                    # the task's own file, uncommitted, set aside
    S5=$(grep -n '^fn golden_runtime' "$F" | cut -d: -f1)    # 5.5b's and 5.6a's part of the file starts here
    sed -i "$S5,\$ s/#\[cfg(unix)\]/#[cfg(any())]/" "$F"
    diff "$P/exec56/selfhost.rs.keep" "$F" | grep -c '^>.*cfg(any())'   # the items turned off
    cargo clippy --locked --all-features --test selfhost -- -D warnings
    cp "$P/exec56/selfhost.rs.keep" "$F"
    cmp "$P/exec56/selfhost.rs.keep" "$F" && echo restored
    git status --short
    ```

    which prints the count, `3` at every task (5.5b's `CC_ARGS`, its use
    in `probe_dir`, and the clang test's making executable the binary
    its harness self-check runs; 5.6a adds no `#[cfg(unix)]`), then
    `Finished` with no warning, then `restored`, then the task's files,
    uncommitted as the checks run before the commit: Task 1's `M` for
    `crates/nova-cli/tests/selfhost.rs`; Task 2's `M` for
    `.github/workflows/ci.yml` and `selfhost.rs`; Task 3's `M` for
    `CHANGELOG.md`, `docs/phase-5-plan.md` and
    `nova-spec/00-MASTER-SPEC.md`, and `??` for
    `docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md` and
    `selfhost/README.md`. Never `git checkout` the file here: it would
    drop the task's uncommitted changes;
  - `target/debug/nova fmt --check std examples selfhost`, CI's arguments,
    and `target/debug/nova fmt --check std examples tests/wasm tests/dom
    tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost`, the
    earlier plans' superset, with the `nova` the step's build made, each
    printing nothing.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost
  if wrong.


## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
check in the task that owns the code:

1. **The chain is the literal one, in order, under the conditions** (S1
   by S0 `--release`, S2 by S1's own `build`, S3 by S2's, each build
   needing the one before it; one `-o` basename, `selfhost`; the runtime
   S1's build linked handed to S2's and S3's, and, where S0 carries one,
   the one it unpacked under the gate's `home/`; `clang` probed once): →
   the gate (Task 2) (mutants G1, G2, G4, H1, H3, H4, R1 to R3).
2. **Every comparison runs, and none is loosened** (N's `.ll` from S1 and
   from S2 against S0's, then S2 against S1 and S3 against S2, byte for
   byte, the failures collected in that order): → `bootstrap_failures` and
   the report test's laid-out files (Task 1) (mutants M1 to M9).
3. **A mismatch is placed, and its files kept** (`ll_difference`'s line,
   `define` and block, both lines and the `diff` line; its fallback place;
   `same_binaries`' names, sizes and first offset; the golden test's
   words for S0 and S1 unchanged; a file neither can read): → the report
   test (Task 1) (mutants M10, M11, W1 to W9, B1 to B5).
4. **Each front-end run leaves the `.ll` its build would remove**
   (`NOVA_CLANG`, and S0's `NOVA_LLC`, at a file that is not there; exit 1
   and the `.ll` left): → the gate (Task 2) (mutants G3, GB2, GB3, GB5).
5. **CI runs the gate, and cannot skip it unseen** (the step's name
   after `--exact`, `--test selfhost -- --ignored`, the Selfhost job,
   `NOVA_REQUIRE_CLANG: "1"`, `if: runner.os == 'Linux'` on both clang
   steps, the `run: |` block's `cargo test` line with `--nocapture`, and
   its `cc --version` and `ld --version`; the test's own skip without
   `clang`): → the CI step test (Task 2), the gate run without `clang`
   (Task 4, Step 8) (mutants M12 to M19, C1, C3 to C9, N1, N2).
6. **The binaries are the tree's and the runtime's** (CI's 9,539,736
   bytes against this machine's 9,354,680; the debug runtime's 14,288,496
   in a checkout): → decision 15, decision 22's check, Task 4, Step 7,
   ADR 0060 §9 and §10.
7. **CI's time** (spec §8, §14): the step's time recorded, the main
   step's `selfhost.rs` under the 80, 100 and 110 s lines with the report
   test in it → decision 10, Task 4, Steps 7 and 11.

## Decisions

The user's, from the spec's header (2026-11-22):

1. **5.6 is three pull requests** (the user's decision 1, as given: 5.6a
   the gate in CI, the tolerances ADR and a `selfhost/` README; then the
   release `v0.5.0`, the version bump by `docs/releasing.md`; then the
   records, the boundary ADR closing Phase 5, after the release). This
   plan is 5.6a's; nothing here bumps a version or tags.
2. **The chain runs in a step of the Selfhost job's Linux leg** (the
   user's decision 2, as given: on every pull request, after 5.5b's
   golden step, with the `clang-18` installed there). As built: `ci.yml`'s
   "The bootstrap, S1 == S2 == S3 (Linux)" (Task 2), with
   `NOVA_REQUIRE_CLANG: "1"` and `NOVA_CLANG: clang-18`; 27.58 s on CI
   (decision 10).
3. **S1 is S0 `--release`'s** (the user's decision 3, as given: so S1 ==
   S2 == S3 byte for byte; the ADR records why the Cranelift S1 differs).
   The gate never calls `selfhost()`; ADR 0060 §4.
4. **The gate is an ignored test in `selfhost.rs`** (the user's decision
   4, as given: named by `--exact` on its step's line, the existing CI
   test holding the names; N's `.ll` per stage first, then the binaries,
   reported by `define` and block or first differing offset). As built:
   `the_bootstrap_holds_s1_s2_s3`, `#[ignore = "needs clang; the Selfhost
   job's Linux step runs it"]`; `bootstrap_failures`, `ll_difference` and
   `same_binaries` (Task 1); `the_ci_step_names_every_clang_test` holding
   both steps (Task 2).
5. **One chain, the conditions recorded** (the user's decision 5, as
   given: S2 and S3 built in different directories; the ADR records one
   `-o` basename, clang 18.1.3, one runtime archive, one `cc`/`ld`, and
   the search of N for `Map` iteration; no extra rebuild, no lint). ADR
   0060 §2, §3 and §5.
6. **Linux only** (the user's decision 6, as given; macOS and Windows go
   to the boundary ADR's backlog). The step's `if: runner.os == 'Linux'`.
7. **`cc` and `ld` printed, not pinned** (the user's decision 7, as
   given). The step's `cc --version` and `ld --version`, which the CI
   step test requires (decision 17); on CI, `cc` 13.3.0 and GNU ld 2.42,
   as here.
8. **The release does not ship `selfhost/`** (the user's decision 8, as
   given: the archive unchanged; the release notes and the README say N
   lives in `selfhost/` and passes the gate). Settled for the release's
   pull request; this plan writes `selfhost/README.md` alone.

The user's answers of 2026-11-22:

9. **The literal chain.**
   - **The user's answer, as given:** on the spec draft, "อนุมัติ + chain
     ตามตัวอักษร (แนะนำ)": approved, with the chain built literally, in
     order: S0 `--release` → S1 → S2 by S1 → S3 by S2, no Cranelift S1 in
     the step, about 4.5 s slower. It narrows decision 2's "S1 and S2
     built side by side": only the front-end-only `.ll` runs go beside
     the builds.
   - **In this plan:** the gate's loop builds each stage with the binary
     the stage before it built (`builder = out`), the front-end run in a
     second thread (`std::thread::scope`), both awaited before the next
     stage (Task 2); mutant G2 (S2 built by S0) is caught.
10. **CI measured from the prototype.**
    - **The user's answer, as given:** asked after the prototype whether
      to push it to `claude/loving-archimedes-frtf5q-56-ci` with a draft
      PR to measure CI, closed unmerged, the user answered "อนุญาต push
      -56-ci + draft PR (แนะนำ)": CI's MSRV job to stand in for the local
      Rust 1.78 check, which the prototype's session had been refused.
    - **What happened, the facts:** `a453c61e` went to that branch with
      draft Sakeerin/nova#148. Its run, 45003631229, passed all 19 jobs:
      the bootstrap step 27.58 s (27.44 s in the test; S1 by S0 8.69 s,
      S2 by S1 9.45 s, S3 by S2 9.23 s), printing `cc` 13.3.0 and GNU ld
      2.42, S1 == S2 == S3 at 9,539,736 bytes and the `.ll` at 12,760,111
      bytes; `selfhost.rs` 58.19 s on Linux, 30.03 s on macOS and 91.31 s
      on Windows (`71 0 3`, `69 0 3`, `65 0 3`), margins 21.81, 69.97 and
      18.69 s against 80, 100 and 110 s; the golden step 30.30 s; MSRV
      passed; the Test job `2838 0 10`, `2838 0 9` and `2815 0 9`. #148
      was closed unmerged. `main`'s run at `83ba8210`, 44962982900, is
      the base's: 58.13, 83.52 and 78.89 s.
    - **In this plan:** ADR 0060 §10 holds these (`0669e9d0`), and says
      that the run measured `a453c61e` only: CI has run none of the cases
      and checks added since, on any system (`fa766425` and `f24c008c`;
      decision 21).
      The local Rust 1.78 check ran here (Task 4, Step 6), and the PR's
      MSRV job is the check of record.

How this plan is cut and checked:

11. **The spec's recommendations, not asked, as taken** (spec header): a
    new step, not a name on 5.5b's step; the runtime taken from S0's own
    unpacking; each `.ll` from a front-end-only run; the CI step test
    extended to two steps and a `run: |` block; `--nocapture` on the
    step; `same_binaries` given the stages' names, and the directory kept
    on a failure; a main-step test of both reports; the prototype
    measured on CI first; one pull request in tasks.
12. **Two tasks test first, then the records** **(differs)**: spec §9's
    cut, with these changes:
    - **§9's item 1 is two tasks.** Task 1 holds the comparisons, which
      the report test exercises without `clang`: its tests commit adds
      `the_bootstrap_reports_name_the_stage` and does not compile
      (`ll_difference`, `stage_ll`, `stage_binary` and
      `bootstrap_failures` missing, `same_binaries` taking two paths), its
      code adds them and names `same_binaries`' sides. Task 2 holds the
      gate and its step: its tests commit adds `runtime_in`, the gate and
      the CI step test holding two steps, which fails for its stated
      reason, `ci.yml has no step `- name: The bootstrap, S1 == S2 == S3
      (Linux)``; its code is the step. The gate calls Task 1's functions,
      so Task 1 comes first.
    - **The gate passes from its tests commit.** What it checks, S0 and
      N, predates 5.6a: S1 == S2 == S3 holds on the base (spec §2 item
      1), so no 5.6a code turns it green. Its red is shown as §9 asks
      ("red by hand on a wrong basename"): by hand at Task 2's tests
      commit, with G1's edit (S3 built as `selfhosx`; Task 2, Step 2),
      and by mutants G1, G2, G3 and G4 in Task 4, Step 8.
    - **§9's item 2, the measurements, is no task:** CI was measured from
      the prototype before this plan (decision 10), and the local figures
      are measured again at the last commit (Task 4, Step 7); ADR 0060 §9
      keeps the prototype's.
    - **The prototype's later commits are folded in where they belong:**
      `b90469a9`'s `bootstrap_failures` and laid-out case and `b829f8dc`'s
      two report cases into Task 1; `ce45a585`'s wording, `b829f8dc`'s
      `NOVA_REQUIRE_CLANG` check, `e84a41be`'s checks of the CI step test
      and `5c3d0584`'s runtime check into Task 2; `0669e9d0`'s,
      `1daf27c7`'s, `6024caef`'s, `fa766425`'s and `f24c008c`'s records
      into Task 3.
    The intermediate trees: Task 1's tests commit, the base's
    `selfhost.rs` with the head's report test after its last test; Task
    1's code, the head's `selfhost.rs` without the module doc's 5.6a
    paragraph, `runtime_in` (the base's `golden_runtime`), the gate and
    the head's CI step test (the base's); Task 2's tests, the head's
    `selfhost.rs`; Task 2's code, the head's `ci.yml`; Task 3's, the
    head's. `split.py` makes them and checks every commit's tree. Each
    task's commit, made after its code patch, builds and passes `S` and
    `SH`.
13. **No user-visible change, shown without a base binary,** as 5.5b's
    plan showed it (its decision 12): 5.6a touches no S0 source, so Task
    4, Step 4 shows that `git diff 45a06d1d HEAD` over the whole tree but
    six excluded paths prints nothing. The pathspec takes every file but
    those, so it covers every S0 input wherever it lies: the manifests
    and the lockfile, every crate's `src/` and `build.rs`, `std/`, and
    the inputs outside `src/` that a narrower pathspec would miss,
    `crates/nova-cli/build_support.rs` (`mod build_support;` in its
    `build.rs`) and `crates/nova-doc/assets/` (`include_str!` in
    `nova-doc/src/lib.rs`); Step 4 lists them. No excluded path can reach
    S0's binary:
    - `crates/nova-cli/tests`: integration tests, built as test targets
      only, never into `nova`;
    - `selfhost`: N's Nova sources and README, not a workspace member,
      read only by the tests that build N;
    - `docs` and `nova-spec`: prose;
    - `CHANGELOG.md`: prose;
    - `.github`: CI's configuration, which runs the build but is no
      input to it.
    And no `include_str!` or `include_bytes!` outside a crate's `tests/`
    and `benches/` names a file under an excluded path (Step 4's third
    command). With the golden suite at 147 and 9 (Step 3), that is the
    proof. N is unchanged too: the diff's only file under `selfhost/` is
    the README.
14. **The spec's open points as the prototype settled them** (ADR 0060
    §12), each cheap to change: the comparisons are a function,
    `bootstrap_failures`, tested in the main step on laid-out files (§7
    lists only the two reports); a stage is its build and its front-end
    run, and either failing stops the chain; S0's build is held to
    `built OUT` too; every build runs with its cwd at the gate's
    directory and N by its path under the repository; `NOVA_LLC` and
    `NOVA_HOME` are set on N's runs too, which N does not read;
    `NOVA_CLANG` is set on every build to the `clang` probed; `bootstrap:
    not run (no clang)` goes to stderr; the report gives each stage's
    build and front-end times, then the comparisons' and the total;
    `same_binaries` takes (name, path) pairs; `runtime_in`'s panic no
    longer names the golden walk; `ll_difference` has a fallback place for
    a line that starts `== `. And §3.6's "unmeasured": on the debug
    runtime S1 == S2 == S3 holds, at 14,288,496 bytes, in about 66 s.
15. **The facts that proved wrong** **(differs; the amendment records
    each)**:
    - the research's 14 `.keys()` sites are 17 lines (spec §2 item 6 had
      already counted 17; ADR 0060 §5 lists them);
    - the debug runtime (§3.6, "not measured"): the chain holds, but takes
      about 66 s (64.24 s at Task 2's tests commit here, 62.85 s at its
      code commit), against 45 s on the release runtime;
    - §8's 43 to 45 s locally is 45.0 to 45.8 s (the prototype), and
      44.14 and 44.16 s here, at the second split's last commit;
    - S0's front end (§2 item 3: 1.47 s) is 1.42 to 1.71 s at
      `opt-level = 0` (the prototype's two runs, ADR 0060 §9), but 0.57
      to 0.72 s at `opt-level = 1`, as CI's Selfhost job builds S0;
    - §7's list needed a case it does not name: `bootstrap_failures` on
      laid-out files, one file made to differ at a time, since a skipped
      or loosened comparison changes nothing the gate can see on a
      correct tree; and, for this plan's mutants, three more (decision
      17): `ll_difference`'s fallback place, a file that neither report
      can read, and `NOVA_REQUIRE_CLANG: "1"` on each clang step; then,
      after the plan's first fact-check, the CI step test's `if:`,
      `--nocapture`, `cc` and `ld` checks (decision 17) and the gate's
      runtime check (decision 22);
    - §8's projection for CI, 52 to 61 s: the step took 27.58 s;
    - the binaries' size: CI's are 9,539,736 bytes, this machine's
      9,354,680. S0 on CI carries a runtime built there,
      `0.4.0-5549317a`, another file than this machine's
      `0.4.0-be7e8b48`, and the runtime's bytes vary the binary (§2 item
      4); the `.ll` is one file on both. So the gate's size and digest are
      as of the tree and of the runtime, and differ between this machine
      and CI.
16. **The records name this branch and date 5.6a 2026-11-22,** as the
    prototype wrote them: ADR 0060's Status `Accepted (2026-11-22)` and
    `claude/loving-archimedes-frtf5q-56`; the specification's note,
    `**Amended 2026-11-22 (branch `claude/loving-archimedes-frtf5q-56`,
    ADR 0060):**`, in 5.5b's shape, and `docs/phase-5-plan.md`'s in that
    file's own, after its 5.5b note: `**Amended 2026-11-22 (5.6a, branch
    `claude/loving-archimedes-frtf5q-56`, ADR 0060; spec …):**`. No
    record names a local branch or a scratch path. The ADR names three
    commits: `83ba8210` (`main`, the research's base), `45a06d1d` (the
    spec's, this branch's first) and `a453c61e` (the commit CI measured,
    on the pushed `claude/loving-archimedes-frtf5q-56-ci`). Its §7 lists the CI step test's checks, `NOVA_REQUIRE_CLANG` and the
    `if:` among them; its §8 the prototype's 23 mutants, whose edits it
    cites from Task 4, Step 8, and this plan's 74; its §10 CI's figures,
    and that CI ran `a453c61e` only: the records are complete before
    Task 3, not left to the review.
17. **The mutants** (Task 4, Step 8): 74. The prototype's 23 as 1 to 23
    (M1 to M19, G1 to G4); then 51 for every check the diff adds that
    those miss: the two reports' words (W1 to W9, B1 to B5), the CI step
    test and the steps (C1 to C9), the gate's own report (GW1 to GW7, GS1
    to GS5) and checks (GB1 to GB5), the runtime's handoff between stages
    (H1 to H5), `runtime_in` (R1 to R3) and the no-clang skip (N1 to N3).
    53 are caught; 21 are accepted, each with its reason in Step 8:
    - output only (13): GW1 to GW7, GS1 to GS5 and N3, where the gate's
      verdict does not change and no test reads the words; G1 to G4 show
      the failure lines in full, and Task 4, Step 7 reads the success
      lines;
    - redundant (4): C2 (a name on both steps fails the final equality
      too), GB1 (`built OUT` held elsewhere), GB2 and GB3 (a front-end
      run's status and its `.ll`, which only a run stopped at `clang`
      leaves, and whose absence fails the comparison);
    - equivalent (3): GB5 (S0 without `NOVA_LLC`: with an `llc` on
      `PATH`, `cc` refuses its non-PIE object; without one, S0 stops with
      "no LLVM toolchain found"; either way it exits 1 and leaves the same
      `.ll`, and the gate, which sets `NOVA_LLC`, depends on neither), H2
      and H5;
    - reached only when a stage fails (1): GB4.
    Three survivors became cases on the prototype before the split
    (`b829f8dc`), so none is accepted: W6 and W7 (the fallback place, its
    words and its line), which the report test reaches with one `.ll`
    going on with a line `== x` where the other ends; W8 and B5 (a file
    neither report can read); and C3 (the step without
    `NOVA_REQUIRE_CLANG`), which the CI step test now refuses. Five more
    were accepted at the first split and are caught now, by checks the
    plan's first fact-check asked for: C4, C5 and C6 (the step without
    `--nocapture`, without its `if:`, without `cc --version` and `ld
    --version`), with C7 to C9 added beside them (the golden step without
    its `if:`, the step without one of the two versions), by the CI step
    test (`e84a41be`); H4 (the runtime taken before S1's build, so that
    all three link the debug one) by the gate's runtime check (decision
    22); and R2 (`runtime_in` ignoring `home`) by that check, and by the
    gate run with `TMPDIR` at a fresh, empty `$P/exec56/gate-tmp`, where
    no `nova-selfhost-home` lies, so S2 takes the debug runtime and
    differs from S1. The no-clang skip (N1, N2) is caught by the gate
    itself, run with `NOVA_CLANG` at a missing file: under
    `NOVA_REQUIRE_CLANG=1` it must fail, without it pass; R3 by the gate
    run with `NOVA_RUNTIME_LIB` set. All 74 results come from one run at
    `6edde36e`, the second split's last commit, with S0 on the release
    runtime (`NOVA_EMBED_RUNTIME=1`, `opt-level = 0`); the runs add up
    to 2,174 s. That tree differs from `11a0d088`'s only in ADR
    0060's text, which no mutant edits and no test reads, so they were
    not run again; Task 4, Step 8 runs every mutant again at the
    executing branch's tip.
18. **The figures, this machine's** (Task 4): 4 CPUs, Rust 1.97 (stable;
    1.100.0 for CI's clippy, 1.78.0 for the MSRV check), Node v22.23.4,
    `clang-18` 18.1.3, `cc` 13.3.0, GNU ld 2.42, cgroup v1. `SH` without
    the release runtime took 203 to 220 s here; the gate as the step runs
    it 44.14 and 44.16 s, and 46.78 to 49.24 s at `opt-level = 0` with
    the release runtime (Step 8's baselines). ADR 0060 §9 keeps the
    prototype's figures, labelled as measured on 2026-11-22; this plan's
    replay does not replace them.
19. **The prototype's Rust 1.78 check, refused there, ran here** (Task 4,
    Step 6): `cargo check --locked --workspace` under 1.78 is clean. It
    checks the libraries and binaries, as CI's MSRV job does, not the
    tests (Global Constraints).
20. **The branch starts from `45a06d1d`:**
    `claude/loving-archimedes-frtf5q-56` from `45a06d1d` (`main`
    `83ba8210` and the approved spec), then this plan's commit, which
    adds this file and appends the spec's dated amendment (2026-11-22):
    the facts that proved wrong (decision 15), CI's figures (decision 10),
    the gate's size and digest as of the tree and the runtime, the cases
    and checks added for the mutants (decision 17), the open decisions as
    settled (decisions 14 and 22), and §9's cut as this plan makes it
    (decision 12).
21. **CI's lines and counts for the PR** (decision 10): the Selfhost job's
    `selfhost.rs` is read on all three systems against the lines, 80 s on
    Linux, 100 s on macOS and 110 s on Windows, and against run
    45003631229's 58.19, 30.03 and 91.31 s; past a line, the user is
    asked. The counts expected: `selfhost.rs` 71 passed and 3 ignored on
    Linux, 69 and 3 on macOS, 65 and 3 on Windows, as on run 45003631229
    (`main`'s 70, 68 and 64 passed and 2 ignored, the report test added
    to the main step and the gate to the ignored tests; this plan's three
    cases and the fact-check's checks are inside existing tests); the
    golden step `2 passed`, `72 filtered out`; the bootstrap step `1
    passed`, `73 filtered out`, its report `S1 == S2 == S3`. Run
    45003631229's 9,539,736 bytes and runtime `0.4.0-5549317a` are
    recorded, not expected: CI builds its runtime with the current
    stable, so both can change, and no test pins a size or a digest. Run
    45003631229 ran the prototype's `a453c61e`: none of `b829f8dc`'s
    three cases, `e84a41be`'s checks or `5c3d0584`'s check, on any
    system; the PR's run is their first on CI. The Test job's tests are
    the base's (2,848 listed, the same names), so its sums should be
    `2838 passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9` on macOS
    and `2815, 0, 9` on Windows, as on run 45003631229. Here the Linux suite gives `2837 1
    10`, the IPv6 test failing for want of IPv6.
22. **The gate checks the runtime it hands on** (fact-check 1; the
    check chosen over reading the report's runtime line in Task 4,
    Steps 7 and 11). Where S0 carries a runtime (`nova version` prints
    `runtime: embedded`, as on CI) and `NOVA_RUNTIME_LIB` is not set, the
    gate, after its comparisons, fails unless the runtime it handed to S2
    and S3 lies under its own `home/runtime/`: `S2 and S3 linked PATH,
    not the runtime S0 carries, unpacked under …/home/runtime`. A plain
    checkout's `nova` carries none, so there the gate links the debug
    runtime beside S0, unchecked, as before; with `NOVA_RUNTIME_LIB` set
    it links that file. Its cost is one `nova version` run. It catches H4,
    which only the report's runtime line showed before, and R2 (Step 8);
    GS3, the runtime line's word, stays output only (decision 17), but
    no step now relies on that line to find H4. In the prototype as
    `5c3d0584`, and ADR 0060 §1 and §12.

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-56`: from `45a06d1d`,
then this plan's commit, which adds this file and appends the spec's
dated amendment, so the patches apply to it as to `45a06d1d` (decision
20).

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-56`.
  - `git log --format='%h %s' -2` prints the plan's commit, then
    `45a06d1d docs: the Phase 5.6a spec is approved`.
  - `git diff --stat 45a06d1d HEAD` lists only
    `docs/superpowers/plans/2026-11-22-phase-5-6a-the-bootstrap-gate.md`
    and
    `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 4's review reads from it.
  - The disk check (Global Constraints), `Task 0 Step 1, the start`,
    shows at least 6,451 MiB (6.3G) free; if not, remove older duplicate
    test executables in `target/debug/deps` first. The first split's
    replay began with 7.2G, the second's, of Tasks 2 and 3, with 7.0G
    (Global Constraints).
- [ ] **Step 2: The tools and the process limit.**
  - `clang-18 --version | head -n 1` prints
    `Ubuntu clang version 18.1.3 (1ubuntu1)`: CI's pin, for the golden
    suite and the clang tests. `cc --version | head -n 1` and `ld
    --version | head -n 1` print `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1)
    13.3.0` and `GNU ld (GNU Binutils for Ubuntu) 2.42` here, as on CI;
    record them (the user's decision 7: printed, not pinned).
  - `node --version` prints `v22.23.4` here, for the WASM tests (any
    Node 22).
  - `RUSTUP_TOOLCHAIN=1.78.0 cargo --version` prints `cargo 1.78.0`, for
    Task 4; `rustup toolchain list` holds `1.100.0` (CI's stable, for the
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
  mkdir -p "$P/exec56"
  cargo build --locked --workspace
  cargo test --locked --workspace --no-run
  S --lib --bins > "$P/exec56/base-a.txt" 2>&1
  S --doc > "$P/exec56/base-b.txt" 2>&1
  S --test '[!s]*' --test 's[!e]*' > "$P/exec56/base-c.txt" 2>&1
  cat "$P"/exec56/base-[abc].txt > "$P/exec56/base.txt"
  grep '^test result' "$P/exec56/base.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec56/base.txt"
  cargo test --locked --workspace -- --list > "$P/exec56/base-list.txt" 2>&1
  grep -c ': test$' "$P/exec56/base-list.txt"
  cargo test --locked --workspace --test selfhost -- --list > "$P/exec56/base-list-sh.txt" 2>&1
  grep -c ': test$' "$P/exec56/base-list-sh.txt"
  cargo test --locked --workspace --test selfhost --no-run
  time SH
  sha256sum Cargo.lock
  ```

  Expected, from the replay on `45a06d1d`: `1300 1 8`, `2 0 0` and `1535
  0 2`, in 144, 2 and 218 s, `2837 1 10` summed; one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`, which needs IPv6, which this machine lacks, as in every
  earlier phase; the list holds `2848` names, the run's 2837 + 1 + 10;
  `selfhost.rs`'s list `72`, and `SH` `70 passed; 0 failed; 2 ignored` in
  217 s; `Cargo.lock`'s sha256 begins `a3009b6850e56f7b`. Task 4 expects
  `2837 1 10` and 2,848 names again, and 74 names in `selfhost.rs`'s
  list.
- [ ] **Step 4: The ledger.** Start `$P/ledger-56.md`, headed with the
  base commit, the baseline's figures, `Cargo.lock`'s sha256, the tools'
  versions and the process limit's form; and `$P/exec56`, the executor's
  directory (Conventions), made in Step 3.


### Task 1: The bootstrap's comparisons and their reports

Spec §3.4, §4, §7; decisions 4, 12 and 14.

**Files:**
- Modify: `crates/nova-cli/tests/selfhost.rs`

- [ ] **Step 1: The tests.** `patch_block t1-tests`:
  `the_bootstrap_reports_name_the_stage`, after the file's last test, in
  the main step on every system (temporary directory
  `nova-selfhost-bootstrap-report`; no `clang`, no N):
  - **`ll_difference`** on a short `.ll` text: equal files `Ok(())`; then
    S2's made to differ in a block (`line 10 of its .ll, define "g.1",
    block bb0`), on a `define` line (`line 5 of its .ll, define "g.1"`),
    outside any `define` (`line 3 of its .ll, outside any define`), where
    S2's ends first (`line 11 of its .ll, define "g.1", block bb0`, S2's
    line `(nothing: the .ll ended)`) and where S2's goes on with a line
    `== x` after S0's ends (the fallback place, `line 13 of its .ll`),
    each report held whole: `N's .ll from S2 differs from S0's at
    PLACE`, both lines, and `both kept: diff A B`;
  - **`same_binaries`** on two small files one byte apart: `S3's binary
    differs from S2's: 8 bytes against S2's 8, first at offset 7`; a
    shorter one, `S1's binary differs from S0's: 5 bytes against S0's 8,
    first at offset 5` (the golden test's words for S0 and S1); equal
    files `Ok(())`;
  - **a file neither can read:** each report begins `cannot read PATH: `;
  - **`bootstrap_failures`** on files laid out as the gate lays them out
    (`stage_ll`'s `ll-sN/selfhost.nova.ll`, `stage_binary`'s
    `sN/selfhost`): all equal, no failure; then one file made to differ
    at a time, S1's or S2's `.ll`, or S1's, S2's or S3's binary: its
    comparisons fail, in order, and no other; S0's `.ll` fails both `.ll`
    comparisons, S1's first.
- [ ] **Step 2: Run, and see it fail.** First the build, which ends with
  `Finished`. Then `SH --exact the_bootstrap_reports_name_the_stage` (2
  s). Expected: `selfhost.rs` does not compile, `error: could not compile
  `nova-cli` (test "selfhost") due to 20 previous errors`: `error[E0425]:
  cannot find function` for `stage_ll` (6), `stage_binary` (4),
  `ll_difference` (3) and `bootstrap_failures` (3), and `error[E0308]:
  arguments to this function are incorrect` for `same_binaries` (4),
  which takes two paths: the comparisons come with the code.
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`same_binaries`** takes (name, path) pairs, `same_binaries(("S1",
    a), ("S2", b))`, and words its error `S2's binary differs from S1's:
    B bytes against S1's A, first at offset N`; both files read by one
    closure, `cannot read PATH: WHY`. Its two callers, the golden walk and
    the clang test's harness self-check, pass `"S0"` and `"S1"`, so their
    words are as before;
  - **`ll_difference(a, b)`:** each `.ll` wrapped as one root's MIR part,
    `== selfhost/src/main.nova`, `(Mir bootstrap)`, `(Ll ok)`, then the
    `.ll`, walked by `compare::first_difference`, whose place names the
    `.ll`'s line, `define` and block; where it gives none (a line that
    starts `== `), the line of the first differing byte; then both lines,
    `(nothing: the .ll ended)` for a side that ended, and `both kept: diff
    A B`;
  - **`stage_binary(dir, stage)`** (`sN/selfhost`, one basename for
    every stage) and **`stage_ll(dir, stage)`** (`ll-sN/selfhost.nova.ll`);
  - **`bootstrap_failures(dir)`:** N's `.ll` from S1, then S2, against
    S0's; then S2's binary against S1's, and S3's against S2's; every
    one run, each failure's report collected in that order.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH --exact the_bootstrap_reports_name_the_stage` (4 s with its
    build). Expected: `1 passed`, `finished in 0.01s`.
  - `cargo test --locked --workspace --no-run` (1 s), then `S` in three
    runs (151, 1 and 225 s). Expected: `1300 1 8`, `2 0 0` and `1535 0
    2`, `2837 1 10`, the IPv6 test the one failure.
  - `cargo test --locked --workspace --test selfhost --no-run`, then `SH`
    (217 s). Expected: `71 passed; 0 failed; 2 ignored`: the base's 70
    and the report test.
- [ ] **Step 5: The checks,** as the conventions list them (the
  mimicked Windows clippy turns off `3` items, none of them 5.6a's).
- [ ] **Step 6: Commit** `selfhost: the bootstrap's comparisons and their
      reports`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 6cfcbc27..7de62d33 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -8317,3 +8317,187 @@ fn n_build_answers_as_s0_does_with_clang() {
         failures.join("\n\n")
     );
 }
+
+/// The bootstrap's two reports (spec
+/// `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
+/// §4, §7), on short texts, with no `clang`: [`ll_difference`] names the
+/// stage, the line of the `.ll`, its `define` and block, both lines and
+/// the `diff` of the two files kept, on a difference in a block, on a
+/// `define` line, outside any `define`, where one `.ll` ends first, and
+/// where the other goes on with a line that starts `== `, which only the
+/// fallback place names; [`same_binaries`] names both stages, both sizes
+/// and the first differing offset, and keeps the golden test's words for
+/// S0 and S1; each names a file it cannot read. And
+/// [`bootstrap_failures`] runs all four comparisons, in order, on files
+/// laid out as the gate lays them out: with one file made to differ, its
+/// comparisons fail, and no other, so none is skipped, loosened or made
+/// against the wrong stage.
+#[test]
+fn the_bootstrap_reports_name_the_stage() {
+    let dir = fresh_dir("bootstrap-report");
+    let s0 = [
+        "; LLVM IR emitted by the Nova compiler (release backend).",
+        "",
+        "declare void @llvm.trap()",
+        "",
+        "define i64 @\"g.1\"() {",
+        "entry:",
+        "  %t0.slot = alloca i64",
+        "  br label %bb0",
+        "bb0:",
+        "  store i64 7, ptr %t0.slot",
+        "  ret i64 7",
+        "}",
+        "",
+    ]
+    .join("\n");
+    let a = dir.join("ll-s0.ll");
+    std::fs::write(&a, &s0).expect("write S0's .ll");
+    let b = dir.join("ll-s2.ll");
+    std::fs::write(&b, &s0).expect("write S2's .ll");
+    assert_eq!(
+        ll_difference(("S0", a.as_path()), ("S2", b.as_path())),
+        Ok(())
+    );
+    let kept = format!("both kept: diff {} {}", a.display(), b.display());
+    for (from, to, at, s0_line, s2_line) in [
+        (
+            "  store i64 7",
+            "  store i64 8",
+            "line 10 of its .ll, define \"g.1\", block bb0",
+            "  store i64 7, ptr %t0.slot",
+            "  store i64 8, ptr %t0.slot",
+        ),
+        (
+            "@\"g.1\"() {",
+            "@\"g.2\"() {",
+            "line 5 of its .ll, define \"g.1\"",
+            "define i64 @\"g.1\"() {",
+            "define i64 @\"g.2\"() {",
+        ),
+        (
+            "@llvm.trap()",
+            "@llvm.debugtrap()",
+            "line 3 of its .ll, outside any define",
+            "declare void @llvm.trap()",
+            "declare void @llvm.debugtrap()",
+        ),
+        (
+            "  ret i64 7\n}\n",
+            "",
+            "line 11 of its .ll, define \"g.1\", block bb0",
+            "  ret i64 7",
+            "(nothing: the .ll ended)",
+        ),
+        (
+            "}\n",
+            "}\n== x\n",
+            "line 13 of its .ll",
+            "(nothing: the .ll ended)",
+            "== x",
+        ),
+    ] {
+        std::fs::write(&b, s0.replace(from, to)).expect("write S2's .ll");
+        assert_eq!(
+            ll_difference(("S0", a.as_path()), ("S2", b.as_path())),
+            Err(format!(
+                "N's .ll from S2 differs from S0's at {at}\n  S0: {s0_line}\n  S2: {s2_line}\n{kept}"
+            )),
+            "{from:?}"
+        );
+    }
+
+    let one = dir.join("one");
+    let other = dir.join("other");
+    std::fs::write(&one, b"\x7fELF\x02\x01\x01\x00").expect("write a binary");
+    std::fs::write(&other, b"\x7fELF\x02\x01\x01\x01").expect("write a binary");
+    assert_eq!(
+        same_binaries(("S1", one.as_path()), ("S2", one.as_path())),
+        Ok(())
+    );
+    assert_eq!(
+        same_binaries(("S2", one.as_path()), ("S3", other.as_path())),
+        Err("S3's binary differs from S2's: 8 bytes against S2's 8, first at offset 7".to_string())
+    );
+    std::fs::write(&other, b"\x7fELF\x02").expect("write a shorter binary");
+    assert_eq!(
+        same_binaries(("S0", one.as_path()), ("S1", other.as_path())),
+        Err("S1's binary differs from S0's: 5 bytes against S0's 8, first at offset 5".to_string())
+    );
+    let missing = dir.join("no-such-file");
+    let unread = format!("cannot read {}: ", missing.display());
+    assert!(
+        ll_difference(("S0", a.as_path()), ("S1", missing.as_path()))
+            .is_err_and(|why| why.starts_with(&unread)),
+        "{unread}"
+    );
+    assert!(
+        same_binaries(("S1", one.as_path()), ("S2", missing.as_path()))
+            .is_err_and(|why| why.starts_with(&unread)),
+        "{unread}"
+    );
+
+    // The four comparisons, each run, in order, on the stages' files: one
+    // file made to differ at a time, its comparisons fail and no other.
+    let stages = dir.join("stages");
+    let lay_out = || {
+        for stage in ["S0", "S1", "S2"] {
+            let ll = stage_ll(&stages, stage);
+            std::fs::create_dir_all(ll.parent().expect("a directory")).expect("make ll-sN");
+            std::fs::write(&ll, &s0).expect("write a stage's .ll");
+        }
+        for stage in ["S1", "S2", "S3"] {
+            let exe = stage_binary(&stages, stage);
+            std::fs::create_dir_all(exe.parent().expect("a directory")).expect("make sN");
+            std::fs::write(&exe, b"\x7fELF\x02\x01\x01\x00").expect("write a stage's binary");
+        }
+    };
+    lay_out();
+    assert_eq!(bootstrap_failures(&stages), Vec::<String>::new());
+    let ll_from = |stage: &str| {
+        format!(
+            "N's .ll from {stage} differs from S0's at line 10 of its .ll, define \"g.1\", block \
+             bb0\n  S0:   store i64 7, ptr %t0.slot\n  {stage}:   store i64 8, ptr %t0.slot\n\
+             both kept: diff {} {}",
+            stage_ll(&stages, "S0").display(),
+            stage_ll(&stages, stage).display()
+        )
+    };
+    let binary_of = |a: &str, b: &str| {
+        format!("{b}'s binary differs from {a}'s: 8 bytes against {a}'s 8, first at offset 7")
+    };
+    for (changed, want) in [
+        (stage_ll(&stages, "S1"), vec![ll_from("S1")]),
+        (stage_ll(&stages, "S2"), vec![ll_from("S2")]),
+        (stage_binary(&stages, "S1"), vec![binary_of("S1", "S2")]),
+        (stage_binary(&stages, "S3"), vec![binary_of("S2", "S3")]),
+        (
+            stage_binary(&stages, "S2"),
+            vec![binary_of("S1", "S2"), binary_of("S2", "S3")],
+        ),
+    ] {
+        lay_out();
+        if changed.ends_with("selfhost.nova.ll") {
+            std::fs::write(&changed, s0.replace("  store i64 7", "  store i64 8"))
+                .expect("write a different .ll");
+        } else {
+            std::fs::write(&changed, b"\x7fELF\x02\x01\x01\x01").expect("write a different binary");
+        }
+        assert_eq!(bootstrap_failures(&stages), want, "{}", changed.display());
+    }
+    // S0's `.ll` differing fails both `.ll` comparisons, S1's first.
+    lay_out();
+    std::fs::write(
+        stage_ll(&stages, "S0"),
+        s0.replace("  store i64 7", "  store i64 8"),
+    )
+    .expect("write a different .ll");
+    let failures = bootstrap_failures(&stages);
+    assert_eq!(failures.len(), 2, "{failures:?}");
+    assert!(
+        failures[0].starts_with("N's .ll from S1 differs from S0's at line 10")
+            && failures[1].starts_with("N's .ll from S2 differs from S0's at line 10"),
+        "{failures:?}"
+    );
+    let _ = std::fs::remove_dir_all(&dir);
+}
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 7de62d33..5f7b1005 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -6912,23 +6912,30 @@ fn golden_runtime() -> PathBuf {
     }
 }
 
-/// Golden program `name`'s two binaries, `s0` and `s1`, the same byte for
-/// byte, or where they first differ.
-fn same_binaries(s0: &Path, s1: &Path) -> Result<(), String> {
-    let a = std::fs::read(s0).map_err(|e| format!("cannot read {}: {e}", s0.display()))?;
-    let b = std::fs::read(s1).map_err(|e| format!("cannot read {}: {e}", s1.display()))?;
-    if a == b {
+/// Two binaries the same byte for byte, or where they first differ, each
+/// given with the name of the side or stage that built it (spec
+/// `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
+/// §4): `b`'s size against `a`'s, and the first offset where they differ,
+/// or the shorter's length. The golden test passes `"S0"` and `"S1"`.
+fn same_binaries(a: (&str, &Path), b: (&str, &Path)) -> Result<(), String> {
+    let ((a_name, a_path), (b_name, b_path)) = (a, b);
+    let read = |path: &Path| {
+        std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
+    };
+    let (a_bytes, b_bytes) = (read(a_path)?, read(b_path)?);
+    if a_bytes == b_bytes {
         return Ok(());
     }
-    let at = a
+    let at = a_bytes
         .iter()
-        .zip(&b)
+        .zip(&b_bytes)
         .position(|(x, y)| x != y)
-        .unwrap_or(a.len().min(b.len()));
+        .unwrap_or(a_bytes.len().min(b_bytes.len()));
     Err(format!(
-        "S1's binary differs from S0's: {} bytes against S0's {}, first at offset {at}",
-        b.len(),
-        a.len()
+        "{b_name}'s binary differs from {a_name}'s: {} bytes against {a_name}'s {}, first at \
+         offset {at}",
+        b_bytes.len(),
+        a_bytes.len()
     ))
 }
 
@@ -7045,7 +7052,7 @@ fn s1_builds_the_golden_suite() {
                     )
                 })
                 .and_then(|_| run_golden(name, &s1_exe, &tmp))
-                .and_then(|()| same_binaries(&s0_exe, &s1_exe));
+                .and_then(|()| same_binaries(("S0", s0_exe.as_path()), ("S1", s1_exe.as_path())));
                 for side in [&s0_exe, &s1_exe] {
                     let _ = std::fs::remove_file(side);
                     for ext in ["ll", "o"] {
@@ -8302,8 +8309,8 @@ fn n_build_answers_as_s0_does_with_clang() {
             }
         }
         ran += 1;
-        if same_binaries(&hi, &hi).is_err()
-            || !same_binaries(&hi, &other)
+        if same_binaries(("S0", hi.as_path()), ("S1", hi.as_path())).is_err()
+            || !same_binaries(("S0", hi.as_path()), ("S1", other.as_path()))
                 .is_err_and(|why| why.ends_with(&format!("first at offset {last}")))
         {
             failures.push("the golden harness took a different binary".to_string());
@@ -8318,6 +8325,88 @@ fn n_build_answers_as_s0_does_with_clang() {
     );
 }
 
+/// N's `.ll` from stage `b`, the same byte for byte as stage `a`'s, each
+/// given by the stage's name and the file's path; or where they first
+/// differ (spec
+/// `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
+/// §4): each `.ll` is read as one root's MIR part, `(Mir bootstrap)`, then
+/// `(Ll ok)`, then the `.ll`, which [`first_difference`] walks as it walks
+/// a root's `.ll` in the MIR dump, so the report names the line of the
+/// `.ll`, its `define` and its block, both lines, and the `diff` of the two
+/// files, which are kept.
+fn ll_difference(a: (&str, &Path), b: (&str, &Path)) -> Result<(), String> {
+    let ((a_name, a_path), (b_name, b_path)) = (a, b);
+    let read = |path: &Path| {
+        std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
+    };
+    let part = |ll: Vec<u8>| {
+        let mut part = b"== selfhost/src/main.nova\n(Mir bootstrap)\n(Ll ok)\n".to_vec();
+        part.extend(ll);
+        part
+    };
+    let (a_ll, b_ll) = (read(a_path)?, read(b_path)?);
+    // The line of the first differing byte, for a place the walk cannot
+    // give: a line of one `.ll` that starts `== `, where the other ends.
+    let line = 1 + a_ll
+        .iter()
+        .zip(&b_ll)
+        .take_while(|(x, y)| x == y)
+        .filter(|(x, _)| **x == b'\n')
+        .count();
+    let Some(m) = first_difference(&part(a_ll), &part(b_ll)) else {
+        return Ok(());
+    };
+    let at = m.place.unwrap_or_else(|| format!("line {line} of its .ll"));
+    let shown = |s: Option<String>| s.unwrap_or_else(|| "(nothing: the .ll ended)".to_string());
+    Err(format!(
+        "N's .ll from {b_name} differs from {a_name}'s at {at}\n  {a_name}: {}\n  {b_name}: {}\n\
+         both kept: diff {} {}",
+        shown(m.s0),
+        shown(m.n),
+        a_path.display(),
+        b_path.display()
+    ))
+}
+
+/// Stage `stage`'s binary under the bootstrap's directory `dir`:
+/// `s1/selfhost` for S1. One basename for every stage, since the `.ll`'s
+/// name is in the binary (its `STT_FILE`).
+fn stage_binary(dir: &Path, stage: &str) -> PathBuf {
+    dir.join(stage.to_lowercase()).join("selfhost")
+}
+
+/// N's `.ll` from stage `stage`'s front-end-only run under the bootstrap's
+/// directory `dir`: `ll-s0/selfhost.nova.ll` for S0.
+fn stage_ll(dir: &Path, stage: &str) -> PathBuf {
+    dir.join(format!("ll-{}", stage.to_lowercase()))
+        .join("selfhost.nova.ll")
+}
+
+/// The bootstrap's comparisons on the stages' files under `dir`, every one
+/// run, in order (spec
+/// `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
+/// §3.4): N's `.ll` from S1, then from S2, against S0's
+/// ([`ll_difference`]); then S2's binary against S1's, and S3's against
+/// S2's ([`same_binaries`]); and each one's report that fails, in that
+/// order. N's `.ll` comes first: equal `.ll`s and differing binaries point
+/// at the toolchain or the link, differing `.ll`s at the front end.
+fn bootstrap_failures(dir: &Path) -> Vec<String> {
+    let mut failures = Vec::new();
+    for (a, b) in [("S0", "S1"), ("S0", "S2")] {
+        let (a_ll, b_ll) = (stage_ll(dir, a), stage_ll(dir, b));
+        if let Err(why) = ll_difference((a, a_ll.as_path()), (b, b_ll.as_path())) {
+            failures.push(why);
+        }
+    }
+    for (a, b) in [("S1", "S2"), ("S2", "S3")] {
+        let (a_exe, b_exe) = (stage_binary(dir, a), stage_binary(dir, b));
+        if let Err(why) = same_binaries((a, a_exe.as_path()), (b, b_exe.as_path())) {
+            failures.push(why);
+        }
+    }
+    failures
+}
+
 /// The bootstrap's two reports (spec
 /// `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
 /// §4, §7), on short texts, with no `clang`: [`ll_difference`] names the
````


### Task 2: The bootstrap gate, and its step in CI

Spec §3, §4, §6, §7; decisions 2 to 7, 9, 12, 14, 17 and 22.

**Files:**
- Modify: `crates/nova-cli/tests/selfhost.rs`, `.github/workflows/ci.yml`

- [ ] **Step 1: The tests.** `patch_block t2-tests`:
  - **`runtime_in(home)`:** `golden_runtime()`'s body, given the unpack
    directory as a parameter: `NOVA_RUNTIME_LIB` when it is set; else the
    one runtime S0 unpacked into `home/runtime/<version>-<crc32>/`; else
    the one beside S0. `golden_runtime()` calls it with
    `nova-selfhost-home`, as before;
  - **`the_bootstrap_holds_s1_s2_s3`**, `#[ignore = "needs clang; the
    Selfhost job's Linux step runs it"]` (temporary directory
    `nova-selfhost-bootstrap`): `clang` (`NOVA_CLANG` or `clang`) probed
    by `--version`, `bootstrap: not run (no clang)` and a pass if it does
    not spawn, a failure under `NOVA_REQUIRE_CLANG=1`; then three stages,
    strictly in order: S1 by S0 (`nova build --release N -o
    s1/selfhost`), S2 by S1 (`S1 build N -o s2/selfhost --release --std
    STD`), S3 by S2, N being `selfhost/src/main.nova` in place. Beside
    each build, in a second thread, its builder's front-end run, the same
    command into `ll-sN/`, `NOVA_HOME` at `home-ll/`, `NOVA_CLANG` and
    `NOVA_LLC` at `no-such-tool`: it must exit 1 and leave
    `ll-sN/selfhost.nova.ll`. Each build must succeed and print exactly
    `built OUT`. S1's build has `NOVA_HOME` at the fresh `home/`; after
    it, the runtime S0 linked (`runtime_in(&home)`) goes to S2's and S3's
    builds as `NOVA_RUNTIME_LIB`. Every run has `NO_COLOR=1`,
    `RUST_BACKTRACE=0`, `RUST_LIB_BACKTRACE=0`, its cwd at the gate's
    directory, and `CHILD_LIMIT`. A stage that fails stops the chain:
    `the bootstrap stopped at SN; no later stage was built:` with each
    failure's command, status and stderr, and the directory kept. Then
    `bootstrap_failures`: `the bootstrap does not hold: K of 4
    comparisons failed:` with each report, and the directory kept. Then,
    where S0 carries a runtime (`nova version`: `runtime: embedded`) and
    `NOVA_RUNTIME_LIB` is not set, the runtime handed to S2 and S3 must
    lie under `home/runtime/` (`S2 and S3 linked PATH, not the runtime S0
    carries, unpacked under …`, the directory kept; decision 22). On
    success it prints the sizes, `clang`'s version, the runtime, each
    stage's times and the total, and removes the directory;
  - **`the_ci_step_names_every_clang_test`,** its name kept, reads two
    steps, "S1 builds the golden suite (Linux)" and "The bootstrap, S1
    == S2 == S3 (Linux)": each in the Selfhost job, each setting
    `NOVA_REQUIRE_CLANG: "1"` and carrying `if: runner.os == 'Linux'`;
    each step's command its `run:` line or, after `run: |`, the block's
    `cargo test` line, with `--test selfhost`, `--` and `--ignored`
    before `--exact`; the bootstrap step's command with `--nocapture`,
    and its block printing `cc --version` and `ld --version`; each name
    after `--exact` a test of the file marked `#[ignore`; no name on both
    steps; the two steps' names together exactly the tests ignored as
    "needs clang";
  - **the module doc** gains the gate's paragraph.
- [ ] **Step 2: Run, and see it fail.** First the build, which ends with
  `Finished`. Then `cargo test --locked --workspace --test selfhost
  --no-run`.
  - `SH --exact the_ci_step_names_every_clang_test` (4 s). Expected: `0
    passed; 1 failed`, the panic ``ci.yml has no step `- name: The
    bootstrap, S1 == S2 == S3 (Linux)` ``: the step comes with the code.
  - `SH` (220 s). Expected: `70 passed; 1 failed; 3 ignored`, the failure
    that test alone.
  - `C the_bootstrap_holds_s1_s2_s3 --nocapture` (64 s, the debug
    runtime). Expected: `1 passed`, `bootstrap: S1 == S2 == S3, SIZE
    bytes each; N's .ll the same from S0, S1 and S2, 12760111 bytes`,
    `runtime …/target/debug/libnova_runtime.a`: the gate holds before any
    5.6a code, since what it checks predates 5.6a (decision 12). SIZE
    was 14288496 here, this machine's figure, with Rust 1.97's debug
    runtime; it varies with the runtime and the toolchain, so it is
    recorded, not expected. A plain `nova` carries no runtime, so the
    runtime check of decision 22 does not apply here.

  **The gate's red, by hand** (spec §9's "red by hand on a wrong
  basename"; Task 4's mutant G1), by this block:

```sh
F=crates/nova-cli/tests/selfhost.rs
cp "$F" "$P/exec56/selfhost.rs.keep"
python3 - "$F" <<'EOF'
import sys
f = sys.argv[1]
t = open(f, encoding="utf-8").read()
old = '    dir.join(stage.to_lowercase()).join("selfhost")'
new = '    dir.join(stage.to_lowercase())\n        .join(if stage == "S3" { "selfhosx" } else { "selfhost" })'
assert t.count(old) == 1
open(f, "w", encoding="utf-8").write(t.replace(old, new))
EOF
C the_bootstrap_holds_s1_s2_s3 --nocapture
cp "$P/exec56/selfhost.rs.keep" "$F"
cmp "$P/exec56/selfhost.rs.keep" "$F" && echo restored
git status --short
```

  `stage_binary` then names S3's binary `s3/selfhosx`, so that binary's
  `STT_FILE` symbol, `selfhosx.nova.ll`, differs from S2's by one byte
  (ADR 0060 §3). Expected: `0 passed; 1 failed`, and in its panic:
  - `the bootstrap does not hold: 1 of 4 comparisons failed:`;
  - `S3's binary differs from S2's: A bytes against S2's B, first at
    offset N`: the two sizes, A and B, equal to each other, and an offset
    N given;
  - `kept:` with the directory, `/tmp/nova-selfhost-bootstrap`;

  then `restored`, and `git status --short` lists only `selfhost.rs`, as
  the task's tests left it uncommitted. Run once at this commit for the
  plan (61 s, 66 s with the test's rebuild), it printed `14288496 bytes
  against S2's 14288496, first at offset 13342310`: this machine's
  figures, with Rust 1.97's debug runtime, which vary with the runtime
  and the toolchain, so they are recorded, not expected. The replay ran
  it on the split's committed tree, where `git status --short` printed
  nothing; the block itself was run as the plan gives it, on a copy of
  Task 2's tests' `selfhost.rs`, without the `C` line: its edit is one
  line, S3's basename `selfhosx`, and the `cp` and `cmp` restore the
  file. The directory kept is the gate's own, under `/tmp/nova-*`: the
  gate's next run removes it, never by hand.
- [ ] **Step 3: The code.** `patch_block t2-code`, `ci.yml`: after "S1
  builds the golden suite (Linux)", before "N's own tests on the debug
  runtime", with a comment citing the spec and ADR 0060, the step "The
  bootstrap, S1 == S2 == S3 (Linux)", `if: runner.os == 'Linux'`, with
  `NOVA_REQUIRE_CLANG: "1"` and `NOVA_CLANG: clang-18`, whose `run: |`
  prints `cc --version` and `ld --version`, then runs `cargo test
  --locked --workspace --all-features --no-fail-fast --test selfhost --
  --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `SH --exact the_ci_step_names_every_clang_test` (under 1 s).
    Expected: `1 passed`.
  - `cargo test --locked --workspace --no-run` (9 s), then `S` in three
    runs (142, 2 and 220 s). Expected: `2837 1 10`.
  - `cargo test --locked --workspace --test selfhost --no-run`, then `SH`
    (218 s). Expected: `71 passed; 0 failed; 3 ignored`.
  - The step's own command, as `ci.yml` writes it, with the step's
    variables: `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test
    --locked --workspace --all-features --no-fail-fast --test selfhost --
    --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3` (63 s, the
    debug runtime). Expected: `1 passed; 0 failed; 0 ignored; 0 measured;
    73 filtered out`, the report's first line `bootstrap: S1 == S2 == S3,
    SIZE bytes each; …`, SIZE 14288496 here: this machine's figure, with
    Rust 1.97's debug runtime, recorded, not expected.
- [ ] **Step 5: The checks,** as the conventions list them (the
  mimicked Windows clippy turns off `3` items).
- [ ] **Step 6: Commit** `ci: the bootstrap gate, S1 == S2 == S3, in the
      Selfhost job's Linux leg`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 5f7b1005..708142ca 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -121,6 +121,14 @@
 //! `clang`, and [`n_build_answers_as_s0_does_with_clang`], ignored here and
 //! run beside the golden walk.
 //!
+//! 5.6a (spec
+//! `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
+//! §3, §4; ADR 0060): [`the_bootstrap_holds_s1_s2_s3`], ignored here and
+//! run by CI's Selfhost job on its Linux leg in a step of its own, builds
+//! the chain in order, S1 by S0 `--release`, S2 by S1 and S3 by S2, one
+//! `-o` basename, and holds N's `.ll` from S1 and S2 equal to S0's and
+//! S1 == S2 == S3, byte for byte.
+//!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
 //!
@@ -6877,13 +6885,23 @@ fn the_ll_dump_is_what_build_release_writes() {
 /// (`NOVA_EMBED_RUNTIME=1`), which must be the only one there; else the one
 /// beside S0, where cargo leaves it in a checkout.
 fn golden_runtime() -> PathBuf {
+    runtime_in(&std::env::temp_dir().join("nova-selfhost-home"))
+}
+
+/// The runtime S0 links in a build whose `NOVA_HOME` is `home`, found in
+/// S0's order (`link.rs`): `NOVA_RUNTIME_LIB` when it is set; else the one
+/// S0 unpacked into `home/runtime/<version>-<crc32>/` from the runtime it
+/// carries, which must be the only one there; else the one beside S0.
+/// [`golden_runtime`] asks with [`build_selfhost`]'s home, and
+/// [`the_bootstrap_holds_s1_s2_s3`] with the home of S1's build (spec
+/// `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
+/// §3.1).
+fn runtime_in(home: &Path) -> PathBuf {
     if let Some(lib) = std::env::var_os("NOVA_RUNTIME_LIB") {
         return PathBuf::from(lib);
     }
     let name = "libnova_runtime.a";
-    let unpacked = std::env::temp_dir()
-        .join("nova-selfhost-home")
-        .join("runtime");
+    let unpacked = home.join("runtime");
     let mut found: Vec<PathBuf> = std::fs::read_dir(&unpacked)
         .map(|entries| {
             entries
@@ -6897,8 +6915,8 @@ fn golden_runtime() -> PathBuf {
             let beside = nova().parent().expect("nova's directory").join(name);
             assert!(
                 beside.is_file(),
-                "no runtime for the golden walk: NOVA_RUNTIME_LIB is not set, \
-                 {} holds none, and none is beside S0 at {}",
+                "no runtime: NOVA_RUNTIME_LIB is not set, {} holds none, and none is \
+                 beside S0 at {}",
                 unpacked.display(),
                 beside.display()
             );
@@ -7084,63 +7102,123 @@ fn s1_builds_the_golden_suite() {
     );
 }
 
-/// CI's step "S1 builds the golden suite (Linux)" names, after `--exact`,
-/// exactly the tests this file marks `#[ignore = "needs clang; ..."]`
-/// (spec
+/// CI's clang steps, "S1 builds the golden suite (Linux)" (spec
 /// `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`
-/// §7): with `--exact`, a name that matches no test runs nothing and the
-/// step still passes, so a name misspelt or dropped there, or a test no
-/// longer ignored here, would go unseen; so would a step that lost
-/// `--ignored`, which then runs none of them, or `--test selfhost`. The
-/// step must also sit in the Selfhost job. Both files are read as text,
-/// with CRLF line endings too, as a Windows checkout may leave them.
+/// §7) and "The bootstrap, S1 == S2 == S3 (Linux)" (spec
+/// `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
+/// §6.2), name after `--exact`, between them, exactly the tests this file
+/// marks `#[ignore = "needs clang; ..."]`, each on one step alone: with
+/// `--exact`, a name that matches no test runs nothing and the step still
+/// passes, so a name misspelt or dropped there, or a test no longer ignored
+/// here, would go unseen; so would a step that lost `--ignored`, which then
+/// runs none of them, or `--test selfhost`. Each step must sit in the
+/// Selfhost job, set `NOVA_REQUIRE_CLANG: "1"`, so that a runner without
+/// `clang` fails it rather than skip its tests, and carry `if: runner.os ==
+/// 'Linux'`, since only that leg installs `clang-18` (and N refuses `build`
+/// on Windows). The bootstrap step's command must also pass `--nocapture`,
+/// and its block print `cc --version` and `ld --version`, so that its log
+/// shows the gate's report and the two tools it ran under (ADR 0060 §2,
+/// §7). A step's command is its `run:` line or, after `run: |`, the block's
+/// `cargo test` line. Both files are read as text, with CRLF line endings
+/// too, as a Windows checkout may leave them.
 #[test]
 fn the_ci_step_names_every_clang_test() {
-    const STEP: &str = "- name: S1 builds the golden suite (Linux)";
+    const STEPS: [&str; 2] = [
+        "- name: S1 builds the golden suite (Linux)",
+        "- name: The bootstrap, S1 == S2 == S3 (Linux)",
+    ];
     let path = root().join(".github").join("workflows").join("ci.yml");
     let ci =
         std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
     let lines: Vec<&str> = ci.lines().map(|l| l.trim_end_matches('\r')).collect();
     let indent = |l: &str| l.len() - l.trim_start().len();
-    let at = lines
-        .iter()
-        .position(|l| l.trim() == STEP)
-        .unwrap_or_else(|| panic!("ci.yml has no step `{STEP}`"));
-    let run = lines[at + 1..]
-        .iter()
-        .take_while(|l| l.trim().is_empty() || indent(l) > indent(lines[at]))
-        .find_map(|l| l.trim().strip_prefix("run:"))
-        .unwrap_or_else(|| panic!("ci.yml's step `{STEP}` has no `run:` line"));
-    let words: Vec<&str> = run.split_whitespace().collect();
-    let exact = words
-        .iter()
-        .position(|w| *w == "--exact")
-        .unwrap_or_else(|| panic!("the step's `run:` line has no `--exact`: {run}"));
-    let mut names: Vec<&str> = words[exact + 1..]
-        .iter()
-        .take_while(|w| !w.starts_with('-'))
-        .copied()
-        .collect();
-    assert!(!names.is_empty(), "the step names no test: {run}");
-    // `--test selfhost`, then `--`, then `--ignored`, before `--exact`.
-    let test = words.windows(2).position(|w| w == ["--test", "selfhost"]);
-    let dashes = words.iter().position(|w| *w == "--");
-    let ignored = words.iter().position(|w| *w == "--ignored");
-    assert!(
-        matches!((test, dashes, ignored), (Some(t), Some(d), Some(i)) if t < d && d < i && i < exact),
-        "the step's `run:` line does not run `--test selfhost -- --ignored` before `--exact`: {run}"
-    );
-    // The step's job: the nearest line above it at a job's indent.
-    let job = lines[..at]
-        .iter()
-        .rev()
-        .find(|l| indent(l) == 2 && !l.trim().is_empty() && !l.trim().starts_with('#'))
-        .map(|l| l.trim());
-    assert_eq!(
-        job,
-        Some("selfhost:"),
-        "the step `{STEP}` is not in the Selfhost job"
-    );
+    // Each step's names after `--exact`, in the order of `STEPS`.
+    let mut named: Vec<(&str, Vec<&str>)> = Vec::new();
+    for step in STEPS {
+        let at = lines
+            .iter()
+            .position(|l| l.trim() == step)
+            .unwrap_or_else(|| panic!("ci.yml has no step `{step}`"));
+        let body: Vec<&str> = lines[at + 1..]
+            .iter()
+            .take_while(|l| l.trim().is_empty() || indent(l) > indent(lines[at]))
+            .copied()
+            .collect();
+        let run_at = body
+            .iter()
+            .position(|l| l.trim().starts_with("run:"))
+            .unwrap_or_else(|| panic!("ci.yml's step `{step}` has no `run:` line"));
+        let run_line = body[run_at]
+            .trim()
+            .strip_prefix("run:")
+            .unwrap_or("")
+            .trim();
+        let run = if run_line == "|" {
+            body[run_at + 1..]
+                .iter()
+                .take_while(|l| l.trim().is_empty() || indent(l) > indent(body[run_at]))
+                .map(|l| l.trim())
+                .find(|l| l.starts_with("cargo test"))
+                .unwrap_or_else(|| {
+                    panic!("ci.yml's step `{step}` has no `cargo test` line in its `run: |` block")
+                })
+        } else {
+            run_line
+        };
+        let words: Vec<&str> = run.split_whitespace().collect();
+        let exact = words
+            .iter()
+            .position(|w| *w == "--exact")
+            .unwrap_or_else(|| panic!("the step `{step}`'s command has no `--exact`: {run}"));
+        let names: Vec<&str> = words[exact + 1..]
+            .iter()
+            .take_while(|w| !w.starts_with('-'))
+            .copied()
+            .collect();
+        assert!(!names.is_empty(), "the step `{step}` names no test: {run}");
+        // `--test selfhost`, then `--`, then `--ignored`, before `--exact`.
+        let test = words.windows(2).position(|w| w == ["--test", "selfhost"]);
+        let dashes = words.iter().position(|w| *w == "--");
+        let ignored = words.iter().position(|w| *w == "--ignored");
+        assert!(
+            matches!((test, dashes, ignored), (Some(t), Some(d), Some(i)) if t < d && d < i && i < exact),
+            "the step `{step}`'s command does not run `--test selfhost -- --ignored` before \
+             `--exact`: {run}"
+        );
+        // The step's job: the nearest line above it at a job's indent.
+        let job = lines[..at]
+            .iter()
+            .rev()
+            .find(|l| indent(l) == 2 && !l.trim().is_empty() && !l.trim().starts_with('#'))
+            .map(|l| l.trim());
+        assert_eq!(
+            job,
+            Some("selfhost:"),
+            "the step `{step}` is not in the Selfhost job"
+        );
+        assert!(
+            body.iter().any(|l| l.trim() == "NOVA_REQUIRE_CLANG: \"1\""),
+            "the step `{step}` does not set `NOVA_REQUIRE_CLANG: \"1\"`"
+        );
+        assert!(
+            body.iter().any(|l| l.trim() == "if: runner.os == 'Linux'"),
+            "the step `{step}` does not carry `if: runner.os == 'Linux'`"
+        );
+        if step == STEPS[1] {
+            assert!(
+                words.contains(&"--nocapture"),
+                "the step `{step}`'s command has no `--nocapture`, so its log would not \
+                 show the gate's report: {run}"
+            );
+            for tool in ["cc --version", "ld --version"] {
+                assert!(
+                    body.iter().any(|l| l.trim() == tool),
+                    "the step `{step}` does not print `{tool}`"
+                );
+            }
+        }
+        named.push((step, names));
+    }
 
     // This file's top-level tests: each name, and its attributes.
     let mut tests: Vec<(&str, Vec<&str>)> = Vec::new();
@@ -7163,14 +7241,22 @@ fn the_ci_step_names_every_clang_test() {
         }
         attrs.clear();
     }
-    for name in &names {
-        let Some((_, attrs)) = tests.iter().find(|(test, _)| test == name) else {
-            panic!("the step names `{name}`, which is no test in selfhost.rs");
-        };
-        assert!(
-            attrs.iter().any(|a| a.starts_with("#[ignore")),
-            "the step names `{name}`, which selfhost.rs does not mark #[ignore]"
-        );
+    let mut names: Vec<&str> = Vec::new();
+    for (step, on_step) in &named {
+        for name in on_step {
+            let Some((_, attrs)) = tests.iter().find(|(test, _)| test == name) else {
+                panic!("the step `{step}` names `{name}`, which is no test in selfhost.rs");
+            };
+            assert!(
+                attrs.iter().any(|a| a.starts_with("#[ignore")),
+                "the step `{step}` names `{name}`, which selfhost.rs does not mark #[ignore]"
+            );
+            assert!(
+                !names.contains(name),
+                "`{name}` is named more than once on the steps"
+            );
+            names.push(name);
+        }
     }
     let mut clang: Vec<&str> = tests
         .iter()
@@ -7185,7 +7271,7 @@ fn the_ci_step_names_every_clang_test() {
     clang.sort_unstable();
     assert_eq!(
         names, clang,
-        "the step's names are not the tests selfhost.rs ignores as needing clang"
+        "the steps' names are not the tests selfhost.rs ignores as needing clang"
     );
 }
 
@@ -8407,6 +8493,226 @@ fn bootstrap_failures(dir: &Path) -> Vec<String> {
     failures
 }
 
+/// The bootstrap gate (spec
+/// `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
+/// §3, §4; ADR 0060): S0's `nova build --release` builds N
+/// (`selfhost/src/main.nova`, in place) into S1, then S1's own `build`
+/// builds N into S2, then S2's builds it into S3, strictly in order, each
+/// into a directory of its own with one `-o` basename, `selfhost`, so one
+/// `selfhost.nova.ll`. Beside each build, and only there, its builder makes
+/// N's `.ll` in a front-end-only run: the same command into a directory of
+/// its own, with `NOVA_CLANG` and `NOVA_LLC`, which only S0 reads, a file
+/// that is not there, so it exits 1 and leaves the `.ll` that a build that
+/// succeeds removes. S1's build has a fresh `NOVA_HOME`, where S0 unpacks
+/// the runtime it carries, and S2's and S3's link the runtime S1 linked
+/// ([`runtime_in`]): one archive for all three. Then N's `.ll` from S1 and
+/// from S2 must equal S0's, and S2 must equal S1 and S3 equal S2, byte for
+/// byte ([`bootstrap_failures`]); the test fails with each comparison that
+/// fails. A stage that fails stops
+/// the chain. When S0 carries a runtime (`nova version` prints `runtime:
+/// embedded`, as CI's does) and `NOVA_RUNTIME_LIB` is not set, the runtime
+/// handed to S2 and S3 must also be the one S0 unpacked under `home/`, not
+/// one beside S0, which would let all three stages agree on another archive
+/// than CI's. On success it prints the sizes, `clang`'s version, the
+/// runtime and each stage's time, and removes its directory; on a failure
+/// the directory is kept.
+///
+/// It needs `clang`, as [`s1_builds_the_golden_suite`] does: without one it
+/// says so and passes, but under `NOVA_REQUIRE_CLANG=1` it fails. CI's
+/// Selfhost job runs it on its Linux leg, in a step of its own after the
+/// golden step.
+#[test]
+#[ignore = "needs clang; the Selfhost job's Linux step runs it"]
+fn the_bootstrap_holds_s1_s2_s3() {
+    let required = std::env::var("NOVA_REQUIRE_CLANG").as_deref() == Ok("1");
+    let clang = std::env::var("NOVA_CLANG").unwrap_or_else(|_| "clang".to_string());
+    let Ok(probe) = Command::new(&clang).arg("--version").output() else {
+        assert!(
+            !required,
+            "NOVA_REQUIRE_CLANG=1, but `{clang} --version` does not run"
+        );
+        eprintln!("bootstrap: not run (no clang)");
+        return;
+    };
+    let version = String::from_utf8_lossy(&probe.stdout)
+        .lines()
+        .next()
+        .unwrap_or("")
+        .trim()
+        .to_string();
+    let started = Instant::now();
+    let dir = fresh_dir("bootstrap");
+    for sub in [
+        "s1", "s2", "s3", "ll-s0", "ll-s1", "ll-s2", "home", "home-ll",
+    ] {
+        std::fs::create_dir_all(dir.join(sub)).expect("create a stage's directory");
+    }
+    let home = dir.join("home");
+    let home_ll = dir.join("home-ll");
+    let missing = dir.join("no-such-tool");
+    let source = root().join("selfhost").join("src").join("main.nova");
+    let std_dir = root().join("std");
+    // A build of N by `builder` into `out`, in S0's words for the first
+    // stage, in N's for the others, as a printable line and a command.
+    let command = |builder: &Path, n_side: bool, out: &Path| {
+        let mut args: Vec<String> = Vec::new();
+        if n_side {
+            args.extend(["build".to_string(), source.display().to_string()]);
+            args.extend(["-o".to_string(), out.display().to_string()]);
+            args.extend(["--release".to_string(), "--std".to_string()]);
+            args.push(std_dir.display().to_string());
+        } else {
+            args.extend(["build".to_string(), "--release".to_string()]);
+            args.push(source.display().to_string());
+            args.extend(["-o".to_string(), out.display().to_string()]);
+        }
+        let line = format!("{} {}", builder.display(), args.join(" "));
+        let mut command = Command::new(builder);
+        command
+            .args(&args)
+            .current_dir(&dir)
+            .env("NOVA_CLANG", &clang)
+            .env("NO_COLOR", "1")
+            .env("RUST_BACKTRACE", "0")
+            .env("RUST_LIB_BACKTRACE", "0")
+            .stdin(Stdio::null());
+        (line, command)
+    };
+    // What a run did, and how long it took.
+    let timed = |mut command: Command, what: &str| {
+        let start = Instant::now();
+        let out = output_within(&mut command, CHILD_LIMIT, what);
+        (out, start.elapsed())
+    };
+
+    let stages = [("S1", "S0"), ("S2", "S1"), ("S3", "S2")];
+    let mut builder = nova();
+    let mut runtime: Option<PathBuf> = None;
+    let mut times = Vec::new();
+    for (k, (stage, by)) in stages.into_iter().enumerate() {
+        let n_side = k > 0;
+        let out = stage_binary(&dir, stage);
+        let ll = stage_ll(&dir, by);
+        let (build_line, mut build) = command(&builder, n_side, &out);
+        let (ll_line, mut front) = command(&builder, n_side, &ll.with_file_name("selfhost"));
+        build.env("NOVA_HOME", &home);
+        front
+            .env("NOVA_HOME", &home_ll)
+            .env("NOVA_CLANG", &missing)
+            .env("NOVA_LLC", &missing);
+        if let Some(runtime) = &runtime {
+            build.env("NOVA_RUNTIME_LIB", runtime);
+            front.env("NOVA_RUNTIME_LIB", runtime);
+        }
+        let ((built, build_time), (fronted, front_time)) = std::thread::scope(|s| {
+            let fronted = s.spawn(|| timed(front, &format!("{by}'s front-end run")));
+            let built = timed(build, &format!("{stage}'s build by {by}"));
+            (built, fronted.join().expect("the front-end run's thread"))
+        });
+        let mut why = Vec::new();
+        match built {
+            Ok(o) if o.status.success() => {
+                let want = format!("built {}\n", out.display());
+                if o.stdout != want.as_bytes() {
+                    why.push(format!(
+                        "{stage}'s build by {by} printed {:?}, not {want:?}\n  {build_line}",
+                        String::from_utf8_lossy(&o.stdout)
+                    ));
+                }
+            }
+            Ok(o) => why.push(format!(
+                "{stage}'s build by {by} failed ({})\n  {build_line}{}",
+                o.status,
+                quoted_stderr(&String::from_utf8_lossy(&o.stderr))
+            )),
+            Err(e) => why.push(format!("{e}\n  {build_line}")),
+        }
+        match fronted {
+            Ok(o) if o.status.code() == Some(1) && ll.is_file() => {}
+            Ok(o) => why.push(format!(
+                "{by}'s front-end run, `NOVA_CLANG` a file that is not there, ended ({}) and \
+                 {} {}, where it must exit 1 and leave it\n  {ll_line}{}",
+                o.status,
+                if ll.is_file() {
+                    "left"
+                } else {
+                    "did not leave"
+                },
+                ll.display(),
+                quoted_stderr(&String::from_utf8_lossy(&o.stderr))
+            )),
+            Err(e) => why.push(format!("{e}\n  {ll_line}")),
+        }
+        assert!(
+            why.is_empty(),
+            "the bootstrap stopped at {stage}; no later stage was built:\n\n{}\n\nkept: {}",
+            why.join("\n\n"),
+            dir.display()
+        );
+        if runtime.is_none() {
+            runtime = Some(runtime_in(&home));
+        }
+        times.push((stage, by, build_time, front_time));
+        builder = out;
+    }
+
+    let compared = Instant::now();
+    let failures = bootstrap_failures(&dir);
+    let compared = compared.elapsed();
+    assert!(
+        failures.is_empty(),
+        "the bootstrap does not hold: {} of 4 comparisons failed:\n\n{}\n\nkept: {}",
+        failures.len(),
+        failures.join("\n\n"),
+        dir.display()
+    );
+    // S0 links the runtime it carries, unpacked into `home`, unless
+    // `NOVA_RUNTIME_LIB` names another: S2's and S3's must then be that one.
+    let about = output_within(
+        Command::new(nova()).arg("version"),
+        CHILD_LIMIT,
+        "nova version",
+    )
+    .unwrap_or_else(|e| panic!("{e}"));
+    let carried = String::from_utf8_lossy(&about.stdout)
+        .lines()
+        .any(|l| l == "runtime: embedded");
+    if carried && std::env::var_os("NOVA_RUNTIME_LIB").is_none() {
+        let unpacked = home.join("runtime");
+        assert!(
+            runtime.as_deref().is_some_and(|r| r.starts_with(&unpacked)),
+            "S2 and S3 linked {}, not the runtime S0 carries, unpacked under {}\n\nkept: {}",
+            runtime.as_deref().unwrap_or(Path::new("")).display(),
+            unpacked.display(),
+            dir.display()
+        );
+    }
+    let size = |path: PathBuf| std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
+    println!(
+        "bootstrap: S1 == S2 == S3, {} bytes each; N's .ll the same from S0, S1 and S2, {} bytes",
+        size(stage_binary(&dir, "S1")),
+        size(stage_ll(&dir, "S0"))
+    );
+    println!("bootstrap: {version}");
+    println!(
+        "bootstrap: runtime {}",
+        runtime.as_deref().unwrap_or(Path::new("")).display()
+    );
+    for (stage, by, build_time, front_time) in &times {
+        println!(
+            "bootstrap: {stage} by {by} {:.2} s, {by}'s .ll beside it {:.2} s",
+            build_time.as_secs_f64(),
+            front_time.as_secs_f64()
+        );
+    }
+    println!(
+        "bootstrap: compared in {:.2} s; {:.2} s in all",
+        compared.as_secs_f64(),
+        started.elapsed().as_secs_f64()
+    );
+    let _ = std::fs::remove_dir_all(&dir);
+}
+
 /// The bootstrap's two reports (spec
 /// `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`
 /// §4, §7), on short texts, with no `clang`: [`ll_difference`] names the
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 859bf886..a9bc5056 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -213,6 +213,24 @@ jobs:
           NOVA_REQUIRE_CLANG: "1"
           NOVA_CLANG: clang-18
         run: cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang
+      # The bootstrap gate (spec
+      # docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md
+      # §3, §6; ADR 0060): S0 `--release` builds N into S1, then S1 builds
+      # N into S2, then S2 builds it into S3, in order, one `-o` basename,
+      # on the release runtime S0 carries and `clang-18`; N's `.ll` from S1
+      # and S2 must equal S0's, and S1 == S2 == S3 byte for byte. On the
+      # Linux leg alone, in a step of its own so that CI times it by itself;
+      # `cc` and `ld` are printed, not pinned: one of each within a job is
+      # one of ADR 0060's conditions.
+      - name: The bootstrap, S1 == S2 == S3 (Linux)
+        if: runner.os == 'Linux'
+        env:
+          NOVA_REQUIRE_CLANG: "1"
+          NOVA_CLANG: clang-18
+        run: |
+          cc --version
+          ld --version
+          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
       - name: N's own tests on the debug runtime
         env:
           NOVA_RUNTIME_LIB: ${{ github.workspace }}/target/debug/${{ matrix.runtime_lib }}
````


### Task 3: ADR 0060, `selfhost/README.md` and the records

Spec §11; decisions 1 to 22.

**Files:**
- Create: `docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md`,
  `selfhost/README.md`
- Modify: `CHANGELOG.md`, `docs/phase-5-plan.md`,
  `nova-spec/00-MASTER-SPEC.md`

- [ ] **Step 1: The records.** `patch_block t3-code`:
  - **ADR 0060,** the bootstrap gate and its tolerances: its Status
    `Accepted (2026-11-22)` and this branch; Context; §1 the gate; §2 the
    four conditions and no tolerance on the bytes; §3 what varies the
    binary and what does not; §4 the Cranelift S1 not used; §5 the search
    of N for `Map` iteration (17 `.keys()` lines); §6 the `.ll`
    differential, recorded, not added; §7 the CI step and the test that
    holds it, `NOVA_REQUIRE_CLANG` and the `if:` among its checks; §8 the
    prototype's 23 mutants, this plan's 74 and the cases and checks added
    for them; §9 the figures, as of the tree; §10 CI's figures, run
    45003631229 with `main`'s 44962982900, the size as of the tree and
    the runtime; §11
    the departures; §12 the decisions the spec does not settle;
    Consequences.
  - **`selfhost/README.md`** (196 lines): what N is (Nova's compiler in
    Nova, not shipped); its commands and their usage lines; S1, S2 and S3
    by hand, then `cmp`; how its tests run, the clang steps and their
    test, clippy and `nova fmt`; the rule that S0 and N change together;
    the ADRs.
  - **`CHANGELOG.md`,** Unreleased: the 5.6a line; Added, the bootstrap
    gate in CI, ADR 0060 and `selfhost/README.md`; nothing Changed.
  - **Dated notes:** `docs/phase-5-plan.md` §5.6 (the user's split,
    5.6a's gate, the departures); `00-MASTER-SPEC.md`'s Phase 5 gate
    (stage2 == stage1 byte for byte, no tolerance, four conditions, ADR
    0060). Both dated 2026-11-22 and naming this branch (decision 16).
- [ ] **Step 2: Read them against the tree.**
  - `git grep -n --untracked 'proto/5-6a\|split/5-6a\|scratchpad\|/home/[a-z]'
    -- docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md
    selfhost/README.md CHANGELOG.md docs/phase-5-plan.md nova-spec`
    prints nothing (exit 1). `--untracked`, since the two new files are
    not yet tracked.
  - Each commit the ADR names exists on `origin`. ``grep -o
    '`[0-9a-f]\{8\}`' docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md
    | sort -u`` prints three, `45a06d1d`, `83ba8210` and `a453c61e`, each
    in its backticks; `git branch -r --contains 83ba8210 | grep -c
    'origin/main$'` prints `1`, and `for h in 45a06d1d a453c61e; do git
    branch -r --contains $h | grep -c
    'origin/claude/loving-archimedes-frtf5q-56-ci$'; done` prints `1`
    twice (`git fetch origin` first, if a branch is not known here).
  - §10's figures against decision 10 (run 45003631229: 27.58 s, 8.69,
    9.45 and 9.23 s, 9,539,736 and 12,760,111 bytes, 58.19, 30.03 and
    91.31 s with `71 0 3`, `69 0 3`, `65 0 3`, 30.30 s; `main`'s 58.13,
    83.52 and 78.89 s), §8's mutants against Task 4, Step 8 (74: 53
    caught, 21 accepted, each group's reason), §7's list against
    `the_ci_step_names_every_clang_test`, and `selfhost/README.md`'s
    usage lines against `selfhost/src/main.nova` and `driver.nova`.
- [ ] **Step 3: The checks,** as the conventions list them, after a
  build: Task 3 has no run step, so first `cargo build --locked
  --workspace` (a no-op), so that the `nova` the two `nova fmt` lines
  run is the tree's (5.5b's ruling R4.2). The mimic turns off `3` items;
  Task 3 changes no Rust, so its `selfhost.rs` is Task 2's; its `git
  status --short` lists the five records, uncommitted. No code changed,
  so the Rust 1.100 clippy is not run again here (the replay ran it here
  all the same, a run of its own; Global Constraints, the disk).
- [ ] **Step 4: Commit**
      `docs: ADR 0060, the bootstrap gate and its tolerances,
      selfhost/README.md and the notes for 5.6a`.

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 72f901fa..eaed8d28 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -32,6 +32,9 @@ ADR 0058).
 Phase 5.5b, the self-hosted driver and linking (spec
 `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`;
 ADR 0059).
+Phase 5.6a, the bootstrap gate (spec
+`docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`;
+ADR 0060).
 
 ### Added
 
@@ -382,6 +385,32 @@ ADR 0059).
   Windows, a signal's status text, `check`'s snippets) is listed in ADR
   0059. Nothing a user sees changes with 5.5b: S0's sources are
   unchanged, so `nova build` and `nova check` are too.
+- **The bootstrap gate, in CI:** a new step of the Selfhost job's Linux
+  leg, after the golden step, runs `the_bootstrap_holds_s1_s2_s3`. The
+  Rust compiler's `nova build --release` builds the self-hosted compiler
+  into stage 1. Stage 1's own `build` then builds it into stage 2, and
+  stage 2's into stage 3, in that order, with one `-o` basename and one
+  runtime archive. The self-hosted compiler's `.ll` of itself must be
+  the same from all three compilers, and stage 1, stage 2 and stage 3
+  must be the same binary, byte for byte. A mismatch is reported by the
+  `.ll`'s line, `define` and block, or by both binaries' sizes and first
+  differing offset, and the files are kept. The step prints `cc`'s and
+  `ld`'s versions. `the_ci_step_names_every_clang_test` now holds both
+  clang steps, each to `NOVA_REQUIRE_CLANG: "1"` and its Linux-only
+  `if:`, and the bootstrap step to `--nocapture` and both versions; a
+  main-step test holds the two reports and the four comparisons (5.6a,
+  ADR 0060).
+- **ADR 0060, the bootstrap gate and its tolerances:** there is no
+  tolerance on the bytes, under four conditions: one `-o` basename, one
+  `clang` (18.1.3), one runtime archive, and one `cc` and `ld` within a
+  job. It records what varies the binary and what does not, why the
+  Cranelift-built stage 1 is not the chain's start, and a search of the
+  self-hosted compiler for `Map` iteration that could reach its output.
+- **`selfhost/README.md`:** what the self-hosted compiler is (in the
+  repository, not shipped), its commands and their usage lines, how to
+  build stage 1, 2 and 3 by hand and compare them, how its tests run,
+  and the rule that the two compilers change together in one pull
+  request.
 
 ### Changed
 
diff --git a/docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md b/docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md
new file mode 100644
index 00000000..01cf0573
--- /dev/null
+++ b/docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md
@@ -0,0 +1,618 @@
+# ADR 0060 — The bootstrap gate and its tolerances
+
+## Status
+
+Accepted (2026-11-22). Branch `claude/loving-archimedes-frtf5q-56`
+(`docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`),
+written as 5.6a's prototype was built. The figures in §9 are local.
+CI's figures are §10's, from the prototype's draft pull request
+Sakeerin/nova#148 (measurement only, closed unmerged); no line was
+passed.
+
+This ADR records the gate's tolerances, which phase plan decision 3
+leaves to "the boundary ADR". That ADR closes Phase 5 after the release
+(the user's decision 1) and cites this one.
+
+## Context
+
+- **The master spec's Phase 5 gate** (`nova-spec/00-MASTER-SPEC.md`):
+  "CI builds stage2 == stage1 byte-for-byte (or close — may differ in
+  non-deterministic codegen, document tolerances)".
+- **Phase plan decision 3** (`docs/phase-5-plan.md`) asks for three
+  things, on Linux, in CI, with one pinned `clang`:
+  1. S0 and S1 emit identical `.ll`;
+  2. S1 == S2;
+  3. S2 == S3.
+
+  Its tolerances: one intermediate name, a pinned `clang`, no build-id,
+  and Linux only.
+- **Decision 7** asks for a CI job on every pull request.
+- **Decision 9** asks for determinism by construction: a seeded `Map`
+  is for lookups only, which 5.6's spec checks by a search of N.
+- **The names:**
+  - **S0:** the Rust compiler.
+  - **N:** the Nova compiler in `selfhost/`.
+  - **S1** = S0 `--release`(N), **S2** = S1(N), **S3** = S2(N).
+- **The research of 2026-11-22**, on `83ba8210`, found S1 == S2 == S3
+  already, over 12 builds, with one `-o` basename. It also probed what
+  varies the binary (§3).
+- **The user's decisions of 2026-11-22** are in the spec's header:
+  - S1 is S0 `--release`'s (decision 3);
+  - the gate is an ignored test in `selfhost.rs`, in a step of the
+    Selfhost job's Linux leg (decisions 2 and 4);
+  - one chain, its conditions recorded (decision 5);
+  - Linux only (decision 6);
+  - `cc` and `ld` printed, not pinned (decision 7);
+  - the literal chain, built in order (the approval).
+
+## Decision
+
+### 1. The gate
+
+On `ubuntu-24.04`, on every pull request, the Selfhost job's step "The
+bootstrap, S1 == S2 == S3 (Linux)" runs `the_bootstrap_holds_s1_s2_s3`
+(`crates/nova-cli/tests/selfhost.rs`). The test does four things.
+
+1. **It builds the chain, strictly in order.** Each stage goes into a
+   fixed directory of its own under `nova-selfhost-bootstrap/`:
+   - S0 `nova build --release N -o s1/selfhost`, with a fresh
+     `NOVA_HOME` (`home/`);
+   - S1 `build N -o s2/selfhost --release --std STD`;
+   - S2 `build N -o s3/selfhost --release --std STD`.
+
+   N is `selfhost/src/main.nova` in place, and STD is `std/`. S0 links
+   `NOVA_RUNTIME_LIB` if the environment sets it. Otherwise it links its
+   embedded runtime, which it unpacks into
+   `home/runtime/<version>-<crc32>/`, or else the runtime beside it.
+   After S1's build, `runtime_in(home)` finds that same file in the same
+   order, and S2's and S3's builds link it through `NOVA_RUNTIME_LIB`.
+   When S0 carries a runtime (`nova version` prints `runtime: embedded`,
+   as on CI) and `NOVA_RUNTIME_LIB` is not set, the test also checks that
+   this file lies under `home/runtime/`, not beside S0.
+2. **It writes each stage's `.ll` of N** in a front-end-only run, beside
+   that stage's build and only there. The run is the same command into
+   `ll-s0/`, `ll-s1/` or `ll-s2/`, with `NOVA_CLANG` (and S0's
+   `NOVA_LLC`) set to `no-such-tool`, which is never created. It must
+   exit 1 and leave `selfhost.nova.ll`. A build that succeeds removes its
+   own `.ll`, so this run is the only way to keep one. It needs no
+   flag, no wrapper and no change to either driver.
+3. **It compares, in order:**
+   - N's `.ll` from S1, then from S2, against S0's;
+   - then S2 against S1, and S3 against S2, byte for byte.
+
+   Every comparison runs. The test fails with every one that fails
+   (`bootstrap_failures`).
+4. **It reports.**
+   - On success it prints the sizes, `clang`'s version, the runtime's
+     path and each stage's time. Then it removes the directory.
+   - On a failure it keeps the directory, and says that it does.
+   - A stage that fails stops the chain, with the stage, the command,
+     the status and the stderr.
+   - A `.ll` mismatch names the line of the `.ll`, the `define` and the
+     block (`ll_difference`, through `compare::first_difference`), and
+     prints the `diff` command for the two kept files.
+   - A binary mismatch names both stages, both sizes and the first
+     differing offset (`same_binaries`).
+
+### 2. The conditions, and no tolerance on the bytes
+
+S1 == S2 == S3 holds byte for byte, with nothing tolerated, under four
+conditions:
+
+1. **One `-o` basename across the stages.** The test uses `selfhost`.
+   The `.ll`'s name, `selfhost.nova.ll`, reaches the binary as its
+   `STT_FILE` symbol (§3).
+2. **One `clang`, 18.1.3.** The job's install step asserts the version
+   (ADR 0048), and the test prints it.
+3. **One runtime archive, the same bytes,** for all three links. The
+   test passes S1's runtime on to S2 and S3.
+4. **One `cc` and one `ld` within a job.** They are not pinned: the
+   runner image fixes them for the job, and the step prints both
+   versions (the user's decision 7). The chain runs in one step, so it
+   runs under one of each.
+
+So phase plan decision 3's build-id clause ("no build IDs:
+`-Wl,--build-id=none`, or a comparison after `strip`") is not needed.
+GNU ld's build-id is a hash of the content, and identical inputs give
+identical ids (§3). The `strip` half would not hide a basename of
+another length in any case: the build-id still differs after `strip`.
+
+### 3. What varies the binary, and what does not
+
+These are the research's probes, made on 2026-11-22 on `83ba8210`
+(spec §2 item 4). In each probe S2 was rebuilt with one thing changed
+and compared with the reference S2.
+
+**What varies the binary:**
+
+| Changed | Result |
+|---|---|
+| the basename, same length (`selfhosx`) | 1 byte differs, the `STT_FILE` `selfhosx.nova.ll`; the build-id is the same |
+| the basename, shorter (`stage2`) | 367,460 bytes and the build-id differ; after `strip`, the 20 build-id bytes still differ; `strip -R .note.gnu.build-id` makes them equal |
+| the runtime archive (the debug one) | another binary, 14,288,496 bytes |
+| the link driver (`CC=clang-18`) | 5,586,894 bytes differ |
+
+**What does not:**
+- `SOURCE_DATE_EPOCH`, and builds made minutes apart;
+- the runtime's path (the same file copied elsewhere);
+- `CC=gcc`;
+- `NOVA_CLANG=clang` (the same 18.1.3; no other version is installed);
+- `env -i` with only `PATH`, `LANG`/`LC_ALL=C`, and a 3,000-byte variable
+  with `HOME=/nonexistent`;
+- N and std at longer paths, relative paths, another cwd and another
+  `TMPDIR`, for both S2 and S0 `--release`;
+- nine front-end runs, which gave one `.ll`.
+
+**Why.**
+- N's object has no debug section and names no path; its only file
+  name is `STT_FILE`. The binary's `.debug_*` sections and paths come
+  from the runtime archive (ring's C, `/rustc/<hash>`), so they are the
+  same whenever the archive is.
+- N reads only `NOVA_CLANG`, `CC`, `NOVA_RUNTIME_LIB` and
+  `current_exe` from outside (`selfhost/src/driver.nova`).
+- std's modules come from a fixed list (`STD_MODULES`,
+  `selfhost/src/builtins.nova`). `read_dir` only tests membership.
+
+The prototype repeated two of these probes in the test itself:
+- the same-length basename, as a mutant (§8, G1): "S3's binary differs
+  from S2's: 9354680 bytes against S2's 9354680, first at offset
+  8970099", where `readelf -s` shows `selfhosx.nova.ll`;
+- the debug runtime for S2 alone (§8, G4): 14,288,496 bytes against
+  9,354,680.
+
+### 4. The Cranelift S1 is not used in the chain
+
+The Selfhost job's other steps build N with S0's default backend,
+Cranelift (`build_selfhost`). That S1 is another binary (9,376,184
+bytes, SHA-256 `c2e8a846…`): its machine code is Cranelift's, not
+`clang`'s. Its `.ll` is S0's, and the S2 it builds is S1 `--release`,
+byte for byte. So "S1 == S2" holds literally only for the `--release`
+S1, and the chain starts there (the user's decision 3). The test never
+calls `selfhost()`. Of what `build_selfhost` gives, it needs only a
+runtime, and S0's own unpacking gives it that. So the step rebuilds
+nothing that the golden step built.
+
+### 5. The search of N for `Map` iteration
+
+Decision 9 holds by construction, and the search re-checks it on the
+tree as of `45a06d1d`.
+
+- **std's `Map` has one way to iterate:** `keys()`. Its order is
+  unspecified ("it can change from one run of a program to the next",
+  `std/collections/lib.nova`).
+- **N uses no `OrderedMap`,** and uses its two `Set`s only for
+  membership (`insert`'s result): `loader.nova:368` and
+  `typeck_traits.nova:1083`.
+- **`grep -rn '\.keys()' selfhost/`** finds **17 lines**. Each falls in
+  one of four classes, and none lets the map's order reach the output:
+
+| Class | Lines |
+|---|---|
+| sorted before use (`sort_strings`) | `resolution_dump.nova:54, 316, 378, 381, 384` (5) |
+| sorted and made unique (`sorted_unique`) | `typeck_body.nova:125` (1) |
+| candidates for `closest()` | `fixes.nova:521, 527, 528, 530, 538, 545` (`did_you_mean`, through `change_to`), `typeck_pattern.nova:722`, `typeck_expr.nova:281`, `typeck_ty.nova:184` (both through `name_fixes`) (9) |
+| re-inserted into a `Map` | `typeck.nova:879` (std's `impl_selves` into the program's), `typeck.nova:1027` (`copy_scope`) (2) |
+
+- **`closest()` gives the same answer in any order**
+  (`selfhost/src/diagnostic.nova`, `closest`), as checked for this ADR.
+  It skips the name itself. It keeps the best candidate by the triple
+  (far, distance, bytes): a name equal but for ASCII case comes first,
+  then the least edit distance, then the least in byte order (`less`).
+  That is a strict lexicographic minimum, so the result is the least
+  candidate whatever the order of the candidates. Two candidates equal
+  in all three are the same string.
+- **A re-inserted map's order** is again unspecified. Its later reads
+  are lookups, or the classes above: `copy_scope`'s scopes reach only
+  `visible_locals` (sorted and unique) and `did_you_mean`.
+
+The research counted "14 `.keys()` sites". Its own classes sum to 15
+(5 sorted, 7 for `closest()`, 1 sorted and unique, 2 re-inserted). The
+lines number 17, because `fixes.nova`'s `did_you_mean` holds six.
+Neither count is a gate. A later change that iterates a `Map` into
+output is guarded three ways:
+- the `.ll` differential (§6), on every pull request, on three systems;
+- this test's `.ll` comparison;
+- this search, which the boundary ADR repeats.
+
+No lint is added (the user's decision 5).
+
+### 6. The `.ll` differential: recorded, not added
+
+Decision 3's first item is "S0 and S1 emit identical `.ll` on
+`tests/runtime/`, `examples/` and N". It is part of 5.5a's corpus,
+which the Selfhost job's main step runs on Linux, macOS and Windows on
+every pull request. That step compares S0's `check --dump-mir --ll`
+against N's `mir --ll` on 778 `.ll`s, N's root among them (ADR 0058
+§11), with the Cranelift S1. 5.6a adds no check for it.
+
+The new test's `.ll` comparison is not a second corpus gate. It covers
+N's root alone, on Linux, through `build`, from S1 and S2, which `clang`
+compiled. Its job is to place a binary mismatch: equal `.ll`s and
+differing binaries point at the toolchain or the link, and differing
+`.ll`s point at the front end.
+
+### 7. The CI step, and the test that holds it
+
+The step comes after "S1 builds the golden suite (Linux)" and before
+"N's own tests on the debug runtime". It runs under the job's
+`NOVA_EMBED_RUNTIME=1` and `opt-level = 1`, so cargo reuses the build:
+
+```yaml
+      - name: The bootstrap, S1 == S2 == S3 (Linux)
+        if: runner.os == 'Linux'
+        env:
+          NOVA_REQUIRE_CLANG: "1"
+          NOVA_CLANG: clang-18
+        run: |
+          cc --version
+          ld --version
+          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
+```
+
+**Why a step of its own, not a name on 5.5b's line** (spec §6.1):
+- CI times it by itself;
+- it does not share the 4-vCPU runner with the golden walk's four
+  threads;
+- it needs no Cranelift S1, so it rebuilds nothing.
+
+`--nocapture` shows the success report in the log.
+
+**`the_ci_step_names_every_clang_test`** keeps its name and now holds
+both clang steps:
+- each step is in the Selfhost job;
+- each step sets `NOVA_REQUIRE_CLANG: "1"`, so that a runner without
+  `clang` fails it rather than skip its tests;
+- each step carries `if: runner.os == 'Linux'`, since only that leg
+  installs `clang-18`;
+- the bootstrap step's command has `--nocapture`, and its block prints
+  `cc --version` and `ld --version`, so that its log shows the report
+  and the two tools (§2's fourth condition);
+- each step's command is its `run:` line, or, after `run: |`, the
+  block's `cargo test` line;
+- the command has `--test selfhost`, `--` and `--ignored` before
+  `--exact`;
+- each name after `--exact` is a test marked `#[ignore`;
+- no name is on both steps, or twice;
+- the two steps' names together are exactly the tests `selfhost.rs`
+  ignores as "needs clang".
+
+### 8. The mutants
+
+Each mutant was run once on the prototype, in the working tree, and
+reverted; none was committed. The exact edits are in the plan's Task 4,
+Step 8 (`docs/superpowers/plans/2026-11-22-phase-5-6a-the-bootstrap-gate.md`),
+mutants 1 to 23. **All 23 were killed.**
+
+**Killed by `the_bootstrap_reports_name_the_stage`** (the main step, no
+`clang`):
+- M1 to M4: each of the four comparisons skipped;
+- M5: a stage compared with itself (S3 against S3);
+- M6: `same_binaries`' `==` loosened to the sizes;
+- M7: `ll_difference` passing on equal sizes;
+- M8: both `.ll` comparisons gone;
+- M9: `same_binaries`' offset one off;
+- M10: `ll_difference` without the `(Mir` line, so with no place;
+- M11: `ll_difference`'s place replaced by the line alone.
+
+**Killed by `the_ci_step_names_every_clang_test`:**
+- M12: the new name dropped;
+- M13: the new name misspelt;
+- M14: the new name on both steps;
+- M15: `--ignored` dropped;
+- M16: the step moved into the `vscode` job;
+- M17: the step's `cargo test` line deleted;
+- M18: the ignore reason changed;
+- M19: the `#[ignore]` removed.
+
+**Killed by `the_bootstrap_holds_s1_s2_s3`** (by hand, with `clang`):
+- G1: S3 built as `selfhosx` (§3);
+- G2: S2 built by S0 instead of S1. S0 refuses N's `--std` with exit 2,
+  and the chain stops at S2.
+- G3: S0's front-end run with the real `clang`. It exits 0 and leaves no
+  `.ll`, and the chain stops at S1.
+- G4: S2 linked with the debug runtime. Two binary comparisons fail.
+
+**Two mutants are equivalent, argued and not run:**
+- **S2 built by S0 with S0's own command line.** S0 `--release` gives S1
+  by the gate's own claim, and both print `built OUT`. The stage's
+  command form follows its index, not its builder, so G2 is the form
+  that can be killed.
+- **S3 built by S1.** S1 == S2.
+
+**A case added for these mutants.** On a correct tree, a comparison
+that is skipped or loosened changes nothing the gate test can see. So
+the comparisons are a function, `bootstrap_failures`, and the report
+test lays out the gate's files and makes one file differ at a time.
+That comparison, and no other, must fail. This addition is not in the
+spec's §7, which lists only the two reports.
+
+**The plan's mutants.** The plan runs the 23 again with 51 more, all in
+one run at its last commit, with S0 carrying the release runtime as on
+CI: 74 in all, **53 caught and 21 accepted**, each with its reason. The
+51 cover every check the diff adds that the 23 miss:
+- **the two reports' words** (W1 to W9, B1 to B5): each part of
+  `ll_difference`'s and `same_binaries`' reports, their fallback place
+  and its line, and a file neither can read. All 14 are caught by
+  `the_bootstrap_reports_name_the_stage`.
+- **the CI step test and the step** (C1 to C9): the `run: |` block's
+  first line taken as the command; the step without
+  `NOVA_REQUIRE_CLANG`, without `--nocapture`, or without its `if:`; the
+  golden step without its `if:`; and the step without `cc --version`,
+  `ld --version` or both. All eight are caught by
+  `the_ci_step_names_every_clang_test`. One is accepted: the check that
+  no name is on both steps removed (redundant: such a name is counted
+  twice, and the final equality fails).
+- **the gate's own report** (GW1 to GW7, GS1 to GS5): its failure lines
+  and its success lines. All 12 are accepted as output only: the
+  verdict does not depend on the words, and G1 to G4 show the failure
+  lines in full.
+- **the gate's checks** (GB1 to GB5), all accepted: `built OUT` not
+  checked (held elsewhere: S1's by the golden walk, N's against S0's by
+  the clang test, S0's by `project.rs`); a front-end run's status, or
+  its `.ll`, not checked (redundant: only a run stopped at `clang`
+  leaves a `.ll`, and a `.ll` not left fails its comparison); a failed
+  stage not stopping the chain (reached only on a failure); and S0's
+  front-end run without `NOVA_LLC`, which is equivalent with an `llc`
+  on `PATH` or without one. With one, S0 compiles the `.ll` with it,
+  and `cc` refuses to link its non-PIE object; without one, S0 stops
+  with "no LLVM toolchain found". Either way the run exits 1 and leaves
+  the same `.ll`. The gate sets `NOVA_LLC`, so it depends on neither.
+- **the runtime's handoff between stages** (H1 to H5): no runtime
+  handed to S2's and S3's builds (N finds none, and the chain stops at
+  S2) and the runtime taken from the front-end runs' home (the debug
+  one, so S2 differs from S1) are caught by the comparisons; the
+  runtime taken before S1's build, so that all three link the debug one
+  beside S0, is caught by the gate's check that S2's and S3's runtime
+  lies under `home/runtime/` when S0 carries one (§1). Two are
+  accepted, both equivalent: no runtime handed to the front-end runs,
+  and the runtime taken again after each stage.
+- **`runtime_in`** (R1 to R3): the wrong directory under `home` is
+  caught by the gate; `NOVA_RUNTIME_LIB` not read is caught by the gate
+  run with `NOVA_RUNTIME_LIB` set; `home` ignored for the golden walk's
+  fixed directory is caught twice: by the runtime check, and by the gate
+  run with `TMPDIR` at a fresh, empty directory, where the golden walk's
+  directory does not exist, so S2 takes the debug runtime and differs
+  from S1.
+- **the no-clang skip** (N1 to N3): the requirement dropped, or read the
+  other way, is caught by the gate run with `NOVA_CLANG` a missing file:
+  under `NOVA_REQUIRE_CLANG=1` it must fail, and without it pass. The
+  `not run (no clang)` line's words are accepted as output only.
+
+**Cases added for the plan's mutants.** Three checks would otherwise
+have let mutants pass, so the prototype gained them before the plan:
+the report test reaches `ll_difference`'s fallback place (one `.ll` goes
+on with a line that starts `== ` where the other ends: "line 13 of its
+.ll") and a file that `ll_difference` or `same_binaries` cannot read;
+and the CI step test holds each clang step to `NOVA_REQUIRE_CLANG: "1"`.
+The plan's fact-check added two more: the CI step test holds each clang
+step to its `if:`, and the bootstrap step to `--nocapture`, `cc
+--version` and `ld --version` (C4 to C9); and the gate checks the
+runtime it hands to S2 and S3 (H4, R2).
+
+### 9. The figures, as of the tree
+
+These were measured on 5.6a's prototype on 2026-11-22: 4 cores,
+`clang-18` 18.1.3, `cc` (Ubuntu 13.3.0-6ubuntu2~24.04.1), GNU ld 2.42.
+"CI-style" means as the Selfhost job runs: `NOVA_EMBED_RUNTIME=1
+CARGO_PROFILE_DEV_OPT_LEVEL=1`, with the runtime that S0 carries.
+
+**The bytes:**
+
+| Output | Bytes | SHA-256 |
+|---|---:|---|
+| S1, S2, S3 | 9,354,680 | `877a8ee8cd27ed4452316ec9387733303cfa7033d793d2aa9ccb6f3bc1c446e2` |
+| N's `.ll` from S0, S1, S2 (406,253 lines) | 12,760,111 | `ae8ce14baa34386e7c3c37323c2ddce0d875c83478485def8777eece2bb382b7` |
+| the runtime, `home/runtime/0.4.0-be7e8b48/libnova_runtime.a` | 28,790,180 | `7924727dc09fc8bba33ff56fb481e5cdfa6b5c6d65ce30394bf3ca68f4325065` |
+
+They match the research's figures on `83ba8210`. The digests, and the
+runtime's `0.4.0-…` directory, will change at the release's version
+bump, so they hold as of this tree only.
+
+**The test as CI runs it** (seconds; each stage's build, with its
+front-end run beside it):
+
+| Run | S1 by S0 | S0's `.ll` | S2 by S1 | S1's `.ll` | S3 by S2 | S2's `.ll` | compare | test | wall |
+|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
+| `C` 1 | 13.84 | 0.61 | 15.43 | 2.07 | 15.57 | 2.37 | 0.13 | 45.02 | 45.28 |
+| `C` 2 | 14.30 | 0.69 | 15.66 | 2.17 | 15.69 | 2.28 | 0.11 | 45.77 | 45.97 |
+| step 1 | 13.83 | 0.57 | 16.07 | 2.18 | 15.45 | 2.42 | 0.13 | 45.52 | 45.77 |
+| step 2 | 14.10 | 0.72 | 15.95 | 2.54 | 15.44 | 2.22 | 0.11 | 45.60 | 45.86 |
+
+The "step" rows ran the step's whole `run:` block, `cc --version` and
+`ld --version` included, on a built tree.
+
+**Other runs:**
+- **At opt-level 0, with `NOVA_RUNTIME_LIB` naming the release runtime:**
+  47.10 and 45.78 s in the test. S0's front end took 1.42 to 1.71 s
+  there, in these two runs, against 0.57 to 0.72 s at opt-level 1 (the
+  table above).
+- **At opt-level 0 with the debug runtime beside S0,** as a checkout
+  runs it without `NOVA_RUNTIME_LIB`: **S1 == S2 == S3 holds**, at
+  14,288,496 bytes, in 66.96 s. That settles the spec's §3.6, which had
+  not measured it. On the debug runtime N's own front end takes 10.4 to
+  11.1 s.
+- **Disk:** the test's directory peaks at about 95 MB on disk:
+  - three binaries;
+  - three `.ll`s;
+  - the 28.8 MB runtime that S0 unpacks.
+
+  `df` on `/` read 7,095 MiB free before the run and 7,025 MiB at its
+  peak.
+- **`selfhost.rs`'s main step as CI runs it,** before (`45a06d1d`) and
+  after, interleaved:
+  - before: 69.35 and 64.48 s, `70 passed; 2 ignored`;
+  - after: 64.98 and 66.34 s, `71 passed; 3 ignored`.
+
+  The two main-step tests cost nothing that can be measured; the spread
+  is the machine's.
+- **The golden step's line as CI runs it:** 39.18 s, 2 passed.
+
+**Against the spec's projection** (§8: 43 to 45 s locally):
+- The test took 45.0 to 45.8 s CI-style, at the top of the range.
+- S2's and S3's builds took 15.4 to 16.1 s, against the research's
+  14.5 to 15.1 s in order. The front-end run beside each build, on a
+  second core, explains the difference, as risk 4 expected.
+- The spec's projection for CI was 52 to 61 s (§8: its 43 to 45 s at
+  ×1.2 to 1.35), under the 75 s margin. CI took 27.58 s (§10).
+
+### 10. CI's figures
+
+From the prototype's draft pull request Sakeerin/nova#148 (measurement
+only, closed unmerged): run 45003631229 on `a453c61e`, the commit CI
+measured, 19 of 19 jobs green. `main`'s run at `83ba8210` (5.5b's
+merge) is 44962982900. Run 45003631229 ran `a453c61e` only: since then
+the prototype has added three cases for the plan's mutants (two in the
+report test, and the CI step test's check of `NOVA_REQUIRE_CLANG`); the
+CI step test's checks of each clang step's `if:`, and of the bootstrap
+step's `--nocapture`, `cc` and `ld`; and the gate's check that S2's and
+S3's runtime is the one S0 unpacked under its home (§8). CI has run
+none of them on any system.
+
+**The bootstrap step,** "The bootstrap, S1 == S2 == S3 (Linux)": 27.58
+s wall; the test `27.44 s in all`, `finished in 27.46s`, `1 passed`,
+`73 filtered out`. Each stage's build, with the front-end run beside it
+(seconds):
+
+| Stage | Build | The builder's `.ll` beside it |
+|---|---:|---:|
+| S1 by S0 | 8.69 | 0.43 |
+| S2 by S1 | 9.45 | 1.29 |
+| S3 by S2 | 9.23 | 1.25 |
+
+The comparisons took 0.07 s. The step printed:
+- `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0` and `GNU ld (GNU Binutils
+  for Ubuntu) 2.42`, the versions measured here (§9);
+- `Ubuntu clang version 18.1.3 (1ubuntu1)`;
+- S1 == S2 == S3, 9,539,736 bytes each, and N's `.ll` the same from S0,
+  S1 and S2, 12,760,111 bytes, as here;
+- the runtime, `home/runtime/0.4.0-5549317a/libnova_runtime.a` under
+  the test's directory.
+
+**The binaries' size differs from §9's 9,354,680 bytes** because CI's
+embedded runtime differs: S0 on CI carries a runtime built there
+(`0.4.0-5549317a`), a different file from this machine's
+(`0.4.0-be7e8b48`, §9). The `.ll` is the same file on both machines,
+and the runtime's bytes vary the binary (§3). So the gate holds "as of
+the tree", on each machine with the runtime that S0 carries there:
+S1 == S2 == S3 under one archive. The size and the digest are the
+tree's and the runtime's, and they differ between this machine and CI.
+
+27.58 s is below this machine's 45.0 to 45.8 s (§9), below the spec's
+projection of 52 to 61 s for CI (spec §8), and well under its 75 s
+margin.
+
+**`selfhost.rs`'s main step** (`finished in`; passed, failed, ignored),
+against the Selfhost job's lines (ADR 0057 §19). The margin is the line
+minus the time:
+
+| System | Line | Run 45003631229 | Counts | `main`, run 44962982900 |
+|---|---:|---:|---|---:|
+| Linux | 80 s | 58.19 s (margin 21.81) | `71 0 3` | 58.13 s, `70 0 2` |
+| macOS | 100 s | 30.03 s (margin 69.97) | `69 0 3` | 83.52 s, `68 0 2` |
+| Windows | 110 s | 91.31 s (margin 18.69) | `65 0 3` | 78.89 s, `64 0 2` |
+
+Each system adds the report test to the main step, and the gate to the
+ignored tests. `the_ci_step_names_every_clang_test` and
+`the_bootstrap_reports_name_the_stage`, as `a453c61e` has them, passed
+on all three. The times
+move with the runners: macOS took 53 s less than on `main`'s run, and
+Windows 12 s more. No line was passed.
+
+**The other steps on Linux:**
+- **S1 builds the golden suite:** 30.30 s wall (`finished in 30.18s`,
+  `2 passed`), against `main`'s 31 s.
+- **N's own tests on the debug runtime:** `1 passed` in 1.16 s.
+
+**The whole Selfhost job:** 4:01 on Linux (`main`: 3:19), 1:35 on macOS
+(3:37) and 5:32 on Windows (4:36). The longest job of the run was the
+Test job on Windows, 7:44, so a pull request's wall time does not
+change.
+
+**The other jobs:**
+- **MSRV (1.78):** passed, `cargo check --locked --workspace` with
+  `rustc 1.78.0`.
+- **The Test job's main step,** its `test result:` lines summed: `2838
+  passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9` on macOS and
+  `2815, 0, 9` on Windows, as on `main`.
+- **The advisory step that runs the ignored GC tests** exited 101 on
+  Linux and macOS, as it does on `main`, and is not counted.
+
+### 11. Departures
+
+- **Phase plan decision 7:** "a CI job on `ubuntu-latest`" is a step of
+  the Selfhost job on `ubuntu-24.04` (the user's decision 2). The job
+  already has the build, the embedded runtime and `clang-18`.
+- **Phase plan decision 3's tolerances** are recorded here, not in the
+  boundary ADR, which will cite this one:
+  - the intermediate name is one `-o` basename;
+  - `clang` is one 18.1.3;
+  - no build-id tolerance is needed (§2);
+  - Linux only.
+
+  This ADR adds two conditions: one runtime archive, and one `cc`/`ld`.
+- **The master spec's gate** reads "or close …, document tolerances".
+  There is no tolerance: stage2 == stage1 byte for byte, under §2's four
+  conditions.
+
+### 12. Decisions the spec does not settle
+
+Each decision comes with its cost if it is wrong.
+
+1. **The comparisons are a function, tested in the main step** (§8).
+   Cost: none; the report test takes about 0.01 s.
+2. **A stage is its build and its front-end run, and either failing
+   stops the chain.** Cost: if only the `.ll` run failed, the later
+   stages' binaries go unbuilt; the report names the failure.
+3. **`NOVA_LLC` is set on N's front-end runs too.** N does not read it.
+   Cost: none.
+4. **Every build runs with cwd at the test's directory, N by its
+   absolute path** (`crates/nova-cli/../../selfhost/src/main.nova`).
+   The probes show that neither cwd nor the path reaches the binary.
+   After a local run `git status` was clean, which settles the spec's
+   "unverified".
+5. **S0's build is held to `built OUT` too,** which S0 prints as N does.
+   Cost: none.
+6. **`NOVA_CLANG` is set on every build to the `clang` probed,** and
+   `NOVA_HOME` on N's builds too, though N does not read it. Cost: none.
+7. **`bootstrap: not run (no clang)` goes to stderr,** as the other clang
+   tests' lines do. Cost: none.
+8. **The report gives each stage's build time and its front-end run's,**
+   then the comparisons' time and the total.
+9. **`same_binaries` takes (name, path) pairs,** so callers pass
+   `.as_path()`: a tuple does not coerce `&PathBuf`.
+10. **`runtime_in`'s panic** no longer names "the golden walk", since
+    two tests call it.
+11. **`ll_difference` has a fallback place** for a line that starts
+    `== ` where the other `.ll` ends. LLVM IR never has such a line;
+    the report test reaches it with one (§8).
+12. **The runtime handed to S2 and S3 is checked** to lie under
+    `home/runtime/` when S0 carries a runtime and `NOVA_RUNTIME_LIB` is
+    not set (§1), S0 asked by `nova version`. A checkout's plain `nova`
+    carries none, so there the gate links the debug runtime beside S0
+    unchecked, as before. Cost: one `nova version` run.
+
+## Consequences
+
+- CI holds S1 == S2 == S3 byte for byte, and N's `.ll` equal from S0,
+  S1 and S2, on every pull request on Linux. This is phase plan
+  decision 3's second and third items; the first has run since 5.5a.
+- A mismatch is placed:
+  - in the front end, when the `.ll`s differ;
+  - in the toolchain or the link, when the `.ll`s are equal and the
+    binaries differ.
+
+  Either way the files are kept for a `diff` or a `cmp`.
+- The Selfhost job's Linux leg gains about 45 s locally and 27.58 s on
+  CI (§10). The job stays under the Test job, so a pull request's wall
+  time does not change.
+- The binaries' size and digest are recorded as of the tree and the
+  runtime: this machine's and CI's differ, since each S0 carries its
+  own runtime (§10).
+- A later change to N that varies its output from run to run is caught
+  by the `.ll` differential or by this gate, with §5's search as the
+  record.
+- macOS and Windows stay outside the gate (the user's decision 6). They
+  go to the boundary ADR's backlog with an unpinned `cc` and `ld`.
+- S0, N, std, the emitter, the runtime and `compare.rs` are unchanged,
+  so ADR 0058 §16's rule is not engaged. No crate was added, and
+  `Cargo.lock` is unchanged.
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index f29bb090..acbd376c 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -693,6 +693,44 @@ ADR 0059; spec
   pull request; how N's determinism is checked (decision 9); what the
   release archive says about `selfhost/`.
 
+**Amended 2026-11-22 (5.6a, branch `claude/loving-archimedes-frtf5q-56`,
+ADR 0060; spec
+`docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`):**
+5.6's questions, decided by the user on 2026-11-22:
+- **5.6 is three pull requests:**
+  - 5.6a: the gate in CI, the tolerances ADR (ADR 0060) and
+    `selfhost/README.md`;
+  - then the release `v0.5.0`, the version bump by `docs/releasing.md`;
+  - then the records, the boundary ADR closing Phase 5, after the
+    release.
+- **5.6a's gate:**
+  - **Where:** on `ubuntu-24.04`, on every pull request, in a step of
+    the Selfhost job's Linux leg after 5.5b's golden step.
+  - **What:** `the_bootstrap_holds_s1_s2_s3` builds S1 by S0
+    `--release`, then S2 by S1, then S3 by S2, strictly in order, with
+    one `-o` basename and one runtime archive. Only the front-end runs
+    that keep each stage's `.ll` go beside the builds.
+  - **What must hold:** N's `.ll` from S1 and from S2 equals S0's, and
+    S1 == S2 == S3, byte for byte.
+- **The departures:**
+  - decision 7's "CI job on `ubuntu-latest`" is that step;
+  - decision 3's first item, the `.ll` differential, has run since 5.5a
+    in the Selfhost job's main step on all three systems (ADR 0058), and
+    5.6a adds no check for it;
+  - decision 3's tolerances are recorded in ADR 0060, which the boundary
+    ADR cites, and there is no tolerance on the bytes:
+    - the intermediate name is one `-o` basename;
+    - `clang` is one 18.1.3;
+    - the build-id clause is not needed, since GNU ld's build-id is a
+      content hash;
+    - ADR 0060 adds two conditions: one runtime archive, and one `cc`
+      and `ld` within a job, printed, not pinned;
+  - decision 9's search of N for `Map` iteration is in ADR 0060 §5: 17
+    `.keys()` lines, each sorted, given to `closest()` (a total order)
+    or re-inserted into a `Map`;
+  - macOS and Windows go to the boundary ADR's backlog;
+  - `selfhost/` is not shipped (decision 5).
+
 ## 5. Cross-cutting
 
 - **Testing.**
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index b286ec40..710ddc18 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -891,6 +891,28 @@ std is found only where `--std` names it. Stage1 built N with its own
 `build`, and the result was, byte for byte, the stage1 that S0's `nova
 build --release` makes with the same `-o` basename.
 
+**Amended 2026-11-22 (branch `claude/loving-archimedes-frtf5q-56`, ADR
+0060):** 5.6a, the bootstrap gate
+(`docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`).
+CI builds stage2 == stage1 byte for byte, with no tolerance, and stage3
+== stage2 too. On Ubuntu, on every pull request, a step of the Selfhost
+job builds the chain in order:
+- stage1 by stage0's `nova build --release`;
+- stage2 by stage1's own `build`;
+- stage3 by stage2's.
+
+It holds the self-hosted compiler's `.ll` of itself equal from stage0,
+stage1 and stage2, and the three binaries equal. This holds under four
+conditions, which ADR 0060 records with what varies the binary and what
+does not:
+- one `-o` basename;
+- one `clang` (18.1.3);
+- one runtime archive;
+- one `cc` and `ld` within a job.
+
+The release `v0.5.0` and the boundary ADR closing Phase 5 follow in
+pull requests of their own.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/selfhost/README.md b/selfhost/README.md
new file mode 100644
index 00000000..a02d7d2c
--- /dev/null
+++ b/selfhost/README.md
@@ -0,0 +1,196 @@
+# `selfhost/`: Nova's compiler, written in Nova
+
+This package is **N**, the self-hosted Nova compiler. It reads a Nova
+program and writes its LLVM IR (`.ll`), the same text the Rust compiler
+writes. Then, like `nova build --release`, it runs `clang` and `cc` on
+that `.ll` to leave a native binary.
+
+N is not shipped. The release ships the Rust `nova`, and N stays in the
+repository, unpublished; no user is meant to run it as a tool (phase
+plan decision 5, `docs/phase-5-plan.md`). It is Phase 5's proof that
+Nova can compile itself, and the tests below hold it to the Rust
+compiler, byte for byte.
+
+## The names
+
+- **S0:** the Rust compiler, `nova`, built by cargo from `crates/`.
+- **N:** this package, `selfhost/src/main.nova` and the modules it
+  imports. Its own tests sit in the `*_tests.nova` files.
+- **S1:** N built by S0. The tests build it with `nova build` in this
+  directory (Cranelift). The bootstrap builds it with `nova build
+  --release` (`clang`).
+- **S2:** N built by S1's own `build`.
+- **S3:** N built by S2's own `build`.
+
+## Its commands
+
+Each command prints what one of S0's commands prints, byte for byte. The
+tests compare the two. These are N's usage lines, as it prints them on a
+command line it does not take, with exit code 2:
+
+```
+usage: selfhost tokens|ast FILE...
+usage: selfhost resolve --std DIR [--target native|wasm] [--test] (--std-part | FILE...)
+usage: selfhost hir --std DIR [--target native|wasm] [--test] (--std-part | FILE...)
+usage: selfhost mir --std DIR [--before-async | --ll] FILE...
+usage: selfhost build FILE [-o OUT] [--release] --std DIR
+usage: selfhost check FILE --std DIR
+```
+
+| N | S0 | What it prints |
+|---|---|---|
+| `tokens` | `nova parse --dump-tokens` | each file's tokens (ADR 0051) |
+| `ast` | `nova parse --dump-ast` | each file's AST (ADR 0052) |
+| `resolve` | `nova check --dump-resolution` | names resolved, per root (ADRs 0053, 0054) |
+| `hir` | `nova check --dump-hir` | items, then the checked bodies (ADRs 0055, 0056) |
+| `mir` | `nova check --dump-mir` | native MIR, after the async transform or `--before-async` before it; with `--ll`, each lowered root's `.ll` (ADRs 0057, 0058) |
+| `build` | `nova build --release FILE -o OUT` | `built OUT`, leaving a binary (ADR 0059) |
+| `check` | `nova check FILE` | each diagnostic's code and locus, then `ok: FILE` (ADR 0059) |
+
+- **std is read only from `--std DIR`,** never from beside the
+  executable or from `$NOVA_HOME`.
+- **`build` works as S0 does:**
+  - it writes `OUT.nova.ll` beside OUT;
+  - it runs `clang -O2 -c` on it (`$NOVA_CLANG`, or `clang`), then `cc`
+    with S0's line (`$CC`, or `cc`);
+  - it finds the runtime in `$NOVA_RUNTIME_LIB`, or else beside the
+    executable or up to two directories above it;
+  - it removes the `.ll` and the object once the build succeeds;
+  - without `--release` it builds the same way.
+
+  It refuses Windows. Its linking on macOS has not yet run.
+
+## S1, S2 and S3 by hand
+
+The chain needs:
+- `clang` 18.1.3 (`clang-18` on Ubuntu 24.04);
+- `cc`;
+- a release build of the runtime.
+
+A checkout's `nova` carries no runtime unless it is built with
+`NOVA_EMBED_RUNTIME=1`. Point `NOVA_RUNTIME_LIB` at a release
+`libnova_runtime.a`, and give all three stages **the same file**.
+
+Use **one `-o` basename for every stage**, in different directories. The
+`.ll`'s name, `selfhost.nova.ll` here, reaches the binary.
+
+From the repository's root:
+
+```sh
+export NOVA_CLANG=clang-18 NOVA_RUNTIME_LIB=/path/to/release/libnova_runtime.a
+mkdir -p /tmp/chain/s1 /tmp/chain/s2 /tmp/chain/s3
+nova build --release selfhost/src/main.nova -o /tmp/chain/s1/selfhost
+/tmp/chain/s1/selfhost build selfhost/src/main.nova -o /tmp/chain/s2/selfhost --release --std std
+/tmp/chain/s2/selfhost build selfhost/src/main.nova -o /tmp/chain/s3/selfhost --release --std std
+cmp /tmp/chain/s1/selfhost /tmp/chain/s2/selfhost && cmp /tmp/chain/s2/selfhost /tmp/chain/s3/selfhost
+```
+
+Each stage takes about 15 s, and `clang` is most of that. The three
+binaries are the same, byte for byte.
+
+**What breaks the equality:**
+- another basename;
+- another runtime archive;
+- another `clang`;
+- another link driver (`CC=clang-18`, for instance).
+
+**What does not:** paths, the working directory, `TMPDIR`, the locale,
+the time and the environment. ADR 0060 records the probes.
+
+To keep a stage's `.ll`, rerun its command with `NOVA_CLANG` (and, for
+S0, `NOVA_LLC`) naming a file that does not exist. The build then stops
+at the `clang` probe, exits 1 and leaves `OUT.nova.ll`.
+
+## How the tests run
+
+`cargo test --workspace` skips `crates/nova-cli/tests/selfhost.rs`,
+because its target says `test = false`. CI's Selfhost job runs it on
+Linux, macOS and Windows:
+
+```sh
+cargo test --workspace --test selfhost
+```
+
+- **The differential.** It compares each of N's dumps with S0's, byte
+  for byte, with the same exit codes, on:
+  - the corpus (every `.nova` file in the repository, N's own included);
+  - the fixed cases under `crates/nova-cli/tests/selfhost/cases/`;
+  - the fuzzers' inputs, on fixed seeds;
+  - programs nested 128 levels deep.
+
+  N is built once by S0's Cranelift backend into a fixed temporary
+  directory, so this needs no `clang`. A difference is reported by file
+  and line, with its place (a MIR function and block, or a `.ll` line,
+  `define` and block), and the commands that reproduce it.
+  `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer fuzz locally.
+- **N's own `@test`s** run through `the_selfhost_tests_pass`, which runs
+  `nova test` in this directory. Every `@test` must pass, so a run of no
+  test at all fails.
+- **The tests that need `clang`** are `#[ignore]`d with the reason
+  "needs clang; the Selfhost job's Linux step runs it". CI names each
+  one by `--exact` on a step of the Selfhost job's Linux leg:
+
+  ```sh
+  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --workspace --test selfhost -- \
+      --ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang
+  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --workspace --test selfhost -- \
+      --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
+  ```
+
+  - `s1_builds_the_golden_suite` has S1 build the 147 golden programs
+    and holds each binary equal to S0 `--release`'s.
+  - `n_build_answers_as_s0_does_with_clang` holds N's `build` to S0's
+    messages.
+  - `the_bootstrap_holds_s1_s2_s3` is the bootstrap gate:
+    - S1 by S0 `--release`, S2 by S1 and S3 by S2, in order;
+    - N's `.ll` from S1 and S2 equal to S0's;
+    - S1 == S2 == S3 (ADR 0060).
+
+  Without `clang` each of these prints that it did not run, and passes.
+  Under `NOVA_REQUIRE_CLANG=1` each fails instead.
+
+  `the_ci_step_names_every_clang_test` holds the steps' names to
+  exactly these tests, since a name that matches nothing would run
+  nothing. It holds each step to `NOVA_REQUIRE_CLANG: "1"` and `if:
+  runner.os == 'Linux'`, and the bootstrap step to `--nocapture`, `cc
+  --version` and `ld --version`.
+- **Clippy** lints the file by name, because `--all-targets` skips it:
+
+  ```sh
+  cargo clippy --all-features --test selfhost -- -D warnings
+  ```
+- **N's sources are formatted:**
+
+  ```sh
+  nova fmt --check std examples selfhost
+  ```
+
+## The rule: S0 and N change together
+
+Each stage of N is a port of S0's, and the differential holds the two
+equal. So a change to the language, to a dump, to the `.ll` emitter, to
+the runtime functions' symbols, signatures or order, or to the runtime's
+ABI as the emitter spells it, changes **both compilers in one pull
+request** (phase plan decision 6; ADR 0058 §16). A part that N does not
+port (`run`, `test`, `fmt`, the LSP, the WASM backend, and WASM and
+test-mode code generation) changes in Rust alone.
+
+## The records
+
+- ADR 0048: Phase 5's scope and backend.
+- ADR 0049: tuples and patterns.
+- ADR 0050: the std a compiler needs.
+- ADR 0051: the lexer and the differential.
+- ADR 0052: the parser, the AST dump and the depth limit.
+- ADR 0053: the loader, the resolver and the resolution dump.
+- ADR 0054: packages, the WASM target and test mode in the resolution
+  dump.
+- ADR 0055: item collection and the HIR dump.
+- ADR 0056: the type checker and the full HIR dump.
+- ADR 0057: MIR and the MIR dump.
+- ADR 0058: the `.ll` emitter and the `.ll` dump.
+- ADR 0059: the driver, `build` and `check`.
+- ADR 0060: the bootstrap gate and its tolerances.
+
+The ADRs are in `docs/adr/`. Phase 5's plan is `docs/phase-5-plan.md`,
+and each sub-phase's spec and plan are under `docs/superpowers/`.
````


### Task 4: Verification, the mutants, the review, the PR, and the merge

Every step runs in a shell set up as the conventions say, the process
limit included.

- [ ] **Step 1: The full checks.** First, before the build, the disk
  check (Global Constraints), `Task 4 Step 1, before the build`, must
  show at least 6,451 MiB (6.3G) free; if not, remove older duplicate
  test executables.

  ```sh
  cargo build --locked --workspace
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo clippy --locked --all-features --test selfhost -- -D warnings
  target/debug/nova fmt --check std examples selfhost
  target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost
  cargo test --locked --workspace --no-run
  S --lib --bins > "$P/exec56/full-a.txt" 2>&1
  S --doc > "$P/exec56/full-b.txt" 2>&1
  S --test '[!s]*' --test 's[!e]*' > "$P/exec56/full-c.txt" 2>&1
  cat "$P"/exec56/full-[abc].txt > "$P/exec56/full.txt"
  grep '^test result' "$P/exec56/full.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec56/full.txt"
  grep -c 'Running tests/selfhost.rs' "$P/exec56/full.txt"
  cargo test --locked --workspace -- --list > "$P/exec56/final-list.txt" 2>&1
  cargo test --locked --workspace --test selfhost -- --list > "$P/exec56/final-list-sh.txt" 2>&1
  python3 "$P/exec56/names.py" "$P/exec56/base-list.txt" "$P/exec56/final-list.txt"
  comm -3 <(grep ': test$' "$P/exec56/base-list-sh.txt" | sort) <(grep ': test$' "$P/exec56/final-list-sh.txt" | sort)
  ```

  `names.py` compares Task 0's list with this one binary by binary, by
  name: a binary is its `Running` or `Doc-tests` line, its hash stripped,
  an integration test's prefixed with the crate whose unit tests cargo
  listed just before it; a test is a line ending `: test`. Write it
  before the run, as the replay ran it:

```sh
mkdir -p "$P/exec56"
cat > "$P/exec56/names.py" <<'EOF'
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
  - the suite, in three runs as the conventions give them (146, 1 and
    213 s): `1300 1 8`, `2 0 0` and `1535 0 2`, `2837 1 10` summed; the
    one failure is Task 0's IPv6 test; `0` binaries named
    `tests/selfhost.rs`;
  - `names.py` prints `binaries: base 111, now 111`, `tests: base 2848,
    now 2848` and `net added: 0`: the suite's tests are the base's;
  - `comm` prints `selfhost.rs`'s two added tests, 72 to 74:
    `the_bootstrap_holds_s1_s2_s3` (ignored) and
    `the_bootstrap_reports_name_the_stage`; none gone.
- [ ] **Step 2: CI's Linux leg, as the Test job runs it.** The `test`
  job's `ubuntu-24.04` leg runs the whole suite with the golden suite
  required, with CI's own flags, and with `RUST_BACKTRACE=1` set for each
  suite command, as `.github/workflows/ci.yml` exports it for every job.
  The suite runs in the conventions' three parts; `CI` is a function of
  this step's shell:

  ```sh
  CI() { RUST_BACKTRACE=1 NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast "$@"; }
  cargo test --locked --workspace --all-features --no-run
  time CI --lib --bins > "$P/exec56/ci-a.txt" 2>&1
  time CI --doc > "$P/exec56/ci-b.txt" 2>&1
  time CI --test '[!s]*' --test 's[!e]*' > "$P/exec56/ci-c.txt" 2>&1
  cat "$P"/exec56/ci-[abc].txt > "$P/exec56/ci.txt"
  grep '^test result' "$P/exec56/ci.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec56/ci.txt"
  grep -c 'release_golden_suite ... ok' "$P/exec56/ci.txt"
  grep -c 'Running tests/selfhost.rs' "$P/exec56/ci.txt"
  ```

  Expected, as the replay ran it at the first split's last commit,
  `7c3cb7f0` (not run again at `11a0d088`, whose tree differs only
  in `selfhost.rs` and the records, which this leg neither builds nor
  runs): `1300 1 8`, `2 0 0` and `1535 0 2` in 154, 1 and 238 s; `2837 1
  10` summed, the one failure
  the IPv6 test, `1` (the golden suite ran and passed inside the third
  part) and `0`. If a part is killed at its timeout, or the shell
  restarts, run that part again alone, then the parts after it; never sum
  a part that did not end with its `test result` lines.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in 25 s of wall
  time (the replay, at the first split's `7c3cb7f0`, as Step 2):
  `release golden: 147
  passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1)` and `1
  passed`: unchanged.
- [ ] **Step 4: No S0 source changed** (decision 13).

  ```sh
  X="':(exclude)crates/nova-cli/tests' ':(exclude)selfhost' ':(exclude)docs' ':(exclude)nova-spec' ':(exclude)CHANGELOG.md' ':(exclude).github'"
  eval git diff --name-only 45a06d1d HEAD -- . "$X"
  eval git ls-files -- . "$X" | grep -e '^crates/nova-cli/build_support.rs$' -e '^crates/nova-doc/assets/'
  git grep -h -o 'include_\(str\|bytes\)!([^)]*)' HEAD -- crates std ':(exclude)crates/*/tests/*' ':(exclude)crates/*/benches/*' | grep -c -e selfhost -e 'tests/' -e docs -e nova-spec -e CHANGELOG -e github
  git diff --stat 45a06d1d HEAD -- . ':(exclude)docs/superpowers'
  ```

  Expected:
  - the first prints nothing: no file outside the six excluded paths
    changed. Its pathspec is the whole tree but those paths, so it holds
    every S0 input wherever it lies (decision 13 says why no excluded
    path reaches S0's binary);
  - the second prints `crates/nova-cli/build_support.rs`,
    `crates/nova-doc/assets/nova.css` and `crates/nova-doc/assets/nova.js`:
    the S0 inputs outside a crate's `src/` and `build.rs`, which that
    pathspec holds;
  - the third prints `0`: no `include_str!` or `include_bytes!` outside a
    crate's `tests/` and `benches/` names a file under an excluded path;
  - the fourth lists the 7 files of the prototype's `f24c008c` against
    `45a06d1d`, 1,578 lines added and 78 removed: `ci.yml`, the
    CHANGELOG, `selfhost.rs`, ADR 0060, the phase plan, the master spec
    and `selfhost/README.md`.

  With Step 3's 147, nothing a user of S0 sees has changed, and the only
  file under `selfhost/` is the README, so N has not either. No base
  binary is needed or built.
- [ ] **Step 5: The CI steps' names.** `SH --exact
  the_ci_step_names_every_clang_test` prints `1 passed`: the two clang
  steps name exactly the tests `selfhost.rs` ignores as needing `clang`,
  each step in the Selfhost job with `NOVA_REQUIRE_CLANG: "1"` and `if:
  runner.os == 'Linux'`, the bootstrap step with `--nocapture`, `cc
  --version` and `ld --version` (mutants M12 to M19, C1 and C3 to C9
  fail it). And `grep -o -- '--exact [a-z0-9_ ]*$'
  .github/workflows/ci.yml` prints three lines, the two clang steps'
  names and the last step's, for the ledger: `--exact
  s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang`,
  `--exact the_bootstrap_holds_s1_s2_s3` and `--exact
  the_selfhost_tests_pass`.
- [ ] **Step 6: The lockfile and Rust 1.78.**
  - `git diff --numstat 45a06d1d -- Cargo.lock '*Cargo.toml'`. Expected:
    nothing; `sha256sum Cargo.lock` as Task 0's.
  - Then, once:

    ```sh
    RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-56 cargo check --locked --workspace
    rm -rf /dev/shm/msrv-56
    ```

    Expected: clean, `Finished` in 54 s (the replay, at the first split's
    `7c3cb7f0`; 5.6a
    changes no library or binary since). It checks the libraries and
    binaries, as CI's MSRV job does, not the tests, whose Rust meets 1.78
    by review (Global Constraints). If the session refuses the command,
    do not retry it in another form: record the refusal in the ledger
    and cite CI's MSRV job, which passed on run 45003631229 and runs again
    on the PR (decisions 10 and 19).
- [ ] **Step 7: `selfhost.rs`'s time, and the gate as the step runs it.**
  - Without the release runtime, as the build stands: `cargo test
    --locked --workspace --test selfhost --no-run`, then `time SH`, then
    `time RUST_BACKTRACE=1 SH`, `RUST_BACKTRACE=1` for that command alone,
    as CI's jobs export it. Expected: `71 passed; 0 failed; 3 ignored` in
    209.13 s, and again in 202.87 s; no child past the 120 s
    `CHILD_LIMIT`.
  - Then as the Selfhost job runs, with the step's variables exported in
    every tool call (each call is a fresh shell: the Runs block, then
    the exports; 5.5b's ruling R5.3). **First the disk,** since this
    changes the opt-level and the runtime: the disk check, `Task 4 Step
    7, before the opt-level 1 build`, must show at least 7,168 MiB; if
    not, remove the `opt-level = 0` `nova-*` and
    `selfhost-*` executables in `target/debug/deps` first (the build
    below makes this level's; the replay read 6.5G, 6,642 MiB, removed
    five and read 7.1G, 7,246 MiB). Then `export NOVA_EMBED_RUNTIME=1
    CARGO_PROFILE_DEV_OPT_LEVEL=1`, two of the job's variables; the third,
    the workflow's `RUST_BACKTRACE=1`, is not set here (the shell keeps
    the Runs block's `0`). `time cargo build --locked --workspace` (8 s;
    `target/debug/nova version` prints `runtime: embedded`); before the
    test build, the disk check at least 6,451 MiB (6.3G; the replay read
    6.9G, 7,027 MiB); `time cargo test --locked --workspace --test selfhost
    --no-run` (15 s). Then:
    - three times `time SH`. Expected: `71 passed; 0 failed; 3 ignored`,
      64.65, 66.73 and 66.65 s;
    - the three ignored tests together, `time C s1_builds_the_golden_suite
      n_build_answers_as_s0_does_with_clang the_bootstrap_holds_s1_s2_s3
      --nocapture`. Expected: `3 passed` in 53.19 s, with `s1 golden: 147
      passed, 9 excluded, Ubuntu clang version 18.1.3 (1ubuntu1), runtime
      /tmp/nova-selfhost-home/runtime/0.4.0-be7e8b48/libnova_runtime.a`
      and the gate's report (below; the runtime's directory in both is
      this machine's), the golden walk and the gate sharing the 4 CPUs;
    - the bootstrap step's whole `run: |` block, twice, as `ci.yml` writes
      it, with its variables: `cc --version; ld --version;
      NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked
      --workspace --all-features --no-fail-fast --test selfhost --
      --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3`, under
      `time`. Expected, each time: `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1)
      13.3.0`, `GNU ld (GNU Binutils for Ubuntu) 2.42`, then the report,
      which the replay's first run printed as

      ```
      bootstrap: S1 == S2 == S3, 9354680 bytes each; N's .ll the same from S0, S1 and S2, 12760111 bytes
      bootstrap: Ubuntu clang version 18.1.3 (1ubuntu1)
      bootstrap: runtime /tmp/nova-selfhost-bootstrap/home/runtime/0.4.0-be7e8b48/libnova_runtime.a
      bootstrap: S1 by S0 13.90 s, S0's .ll beside it 0.65 s
      bootstrap: S2 by S1 15.26 s, S1's .ll beside it 2.32 s
      bootstrap: S3 by S2 14.80 s, S2's .ll beside it 2.18 s
      bootstrap: compared in 0.12 s; 44.09 s in all
      ```

      (its second: 13.91, 0.63, 15.00, 2.07, 15.07, 2.19, 0.13 and 44.12
      s), and `1 passed; 0 failed; 0 ignored; 0 measured; 73 filtered
      out`, `finished in` 44.14 and 44.16 s (CI's 27.46 s). What must
      hold: `S1 == S2 == S3` at one size, N's `.ll` the same from S0, S1
      and S2, and the runtime line under the gate's `home/runtime/`, as
      decision 22's check requires here. The size, 9354680, and the
      runtime's directory, `0.4.0-be7e8b48`, are this machine's figures,
      with Rust 1.97's release runtime: they vary with the runtime and
      the toolchain, so they are recorded, not expected, as are run
      45003631229's 9,539,736 bytes and `0.4.0-5549317a`, since CI builds
      its runtime with the current stable (decisions 15 and 21). A time
      may differ with the machine's speed, and is recorded in the ledger;
    - the job's last step, `time NOVA_RUNTIME_LIB=$PWD/target/debug/libnova_runtime.a
      cargo test --locked --workspace --all-features --no-fail-fast --test
      selfhost -- --exact the_selfhost_tests_pass`. Expected: `1 passed`
      in 2.18 s;
    - `git status --short` prints nothing.
  - Then `unset NOVA_EMBED_RUNTIME CARGO_PROFILE_DEV_OPT_LEVEL`. Before
    the build, which changes the opt-level and the runtime back: the
    disk check, `Task 4 Step 7, before the plain build`, at least 7,168
    MiB, or remove the `opt-level = 1` `nova-*` and
    `selfhost-*` executables this step built first. Then `cargo build
    --locked --workspace`, which ends with `Finished`: the debug-runtime
    `nova` at `opt-level = 0` again (`target/debug/nova version` prints
    `runtime: not embedded`). The replay did not run this return: it
    went from this step's build straight to Step 8's embedded build at
    `opt-level = 0` (Global Constraints, the disk).
- [ ] **Step 8: The mutants.** One at a time, each edit is made, its
  commands run, and the file restored with `git checkout -- <file>`, all
  on the last commit, committed, under the process limit, with
  `NOVA_EMBED_RUNTIME=1` exported for the whole step (decision 17), in
  every tool call after the Runs block (5.5b's ruling R5.3), S0
  carrying the release runtime, which the gate's S1 build unpacks, as on
  CI. Of the Selfhost job's other variables,
  `CARGO_PROFILE_DEV_OPT_LEVEL=1` is not set (S0 at `opt-level = 0`, as
  the replay ran the mutants) and `RUST_BACKTRACE` stays the Runs block's
  `0`; `NOVA_RUNTIME_LIB` stays unset. The replay's runner does it, and
  the executor uses it as given, writing no runner of its own:
  `mutrun.py` reads the mutants from `mutants.json`, applies each one's
  edits, runs its commands, restores the file, checks `git status
  --short` before and after, and writes each result to
  `$P/exec56/mut/<id>.json` and its log beside it. Write both before the
  step, from the repository's root:

```sh
mkdir -p "$P/exec56"
cat > "$P/exec56/mutrun.py" <<'EOF'
#!/usr/bin/env python3
"""mutrun.py [--force] [--check] IDS...: run the mutants of mutants.json one at
a time on the checked-out tree (the last commit), each file restored after
with `git checkout -- FILE` and `git status --short` checked empty before and
after. IDS are numbers or ranges (3-7). Each result goes to
$P/exec56/mut/<id>.json (and its log to <id>.log). A result already there is
kept unless --force. --check only checks each edit's text. Run from the
repository's root, under the process limit (the Conventions' Runs), with
NOVA_EMBED_RUNTIME=1 exported and NOVA_RUNTIME_LIB unset, as the plan's
mutant step runs: S0 then carries the release runtime, which the gate's S1
build unpacks.

Per mutant: `cargo build --locked --workspace`; then
  SH --exact <its selfhost.rs tests> GUARD                         (always)
  gate:        NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 SH --ignored --exact GATE --nocapture
  gate_rt:     the same, NOVA_RUNTIME_LIB=$P/exec56/rt/libnova_runtime.a
  gate_tmp:    the same, TMPDIR=$P/exec56/gate-tmp, removed and made empty first, so
               that no nova-selfhost-home lies in the temporary directory
  noclang_req: NOVA_REQUIRE_CLANG=1 NOVA_CLANG=$P/exec56/no-clang SH --ignored --exact GATE
               (it must fail; "noclang_req passes" when it does not)
  noclang:     NOVA_CLANG=$P/exec56/no-clang SH --ignored --exact GATE --nocapture
               (it must pass; "noclang fails" when it does not)
A mutant is caught when anything fails that passes unmutated; it fails as
expected when what failed is its "expect" list exactly. Each run's exit
status is kept in the result's "runs".
"""
import json
import os
import re
import shutil
import subprocess
import sys
import time

P = os.environ["P"]
D = f"{P}/exec56"
REPO = os.getcwd()
if not os.path.isdir(os.path.join(REPO, ".git")):
    raise SystemExit("run mutrun.py from the repository's root")
OUT = f"{D}/mut"
os.makedirs(OUT, exist_ok=True)
MUTANTS = json.load(open(f"{D}/mutants.json"))
GUARD = "the_resolution_dump_has_a_header_per_root_and_std_its_own_part"
GATE = "the_bootstrap_holds_s1_s2_s3"
SH = ["cargo", "test", "--locked", "--workspace", "--test", "selfhost", "--"]
CLANG = {"NOVA_REQUIRE_CLANG": "1", "NOVA_CLANG": "clang-18"}
NO_CLANG = f"{D}/no-clang"
RT = f"{D}/rt/libnova_runtime.a"
GATE_TMP = f"{D}/gate-tmp"
RUNS = {
    "gate": (SH + ["--ignored", "--exact", GATE, "--nocapture"], CLANG),
    "gate_rt": (SH + ["--ignored", "--exact", GATE, "--nocapture"], dict(CLANG, NOVA_RUNTIME_LIB=RT)),
    "gate_tmp": (SH + ["--ignored", "--exact", GATE, "--nocapture"], dict(CLANG, TMPDIR=GATE_TMP)),
    "noclang_req": (SH + ["--ignored", "--exact", GATE], {"NOVA_REQUIRE_CLANG": "1", "NOVA_CLANG": NO_CLANG}),
    "noclang": (SH + ["--ignored", "--exact", GATE, "--nocapture"], {"NOVA_CLANG": NO_CLANG}),
}


def sh(cmd, env=None, log=None, timeout=590):
    e = dict(os.environ, **(env or {}))
    t = time.time()
    r = subprocess.run(cmd, cwd=REPO, env=e, capture_output=True, text=True, errors="replace",
                       timeout=timeout)
    out = r.stdout + r.stderr
    if log is not None:
        shown = " ".join(f"{k}={v}" for k, v in (env or {}).items())
        log.write(f"$ {shown} {' '.join(cmd)}\n{out}\n[exit {r.returncode}, {time.time() - t:.1f} s]\n\n")
    return r.returncode, out


def status_clean():
    return subprocess.run(["git", "status", "--short"], cwd=REPO, capture_output=True,
                          text=True).stdout.strip() == ""


def ids(args):
    out = []
    for a in args:
        if "-" in a:
            lo, hi = a.split("-")
            out += range(int(lo), int(hi) + 1)
        else:
            out.append(int(a))
    return out


def failed_of(out):
    return re.findall(r"^test (\S+) \.\.\. FAILED", out, re.M)


def apply(m, text):
    for k, (old, new) in enumerate(m["edits"]):
        n = text.count(old)
        occ = m.get("occ", 1) if k == 0 else 1
        if ("occ" in m and k == 0 and n < occ) or (("occ" not in m or k > 0) and n != 1):
            raise SystemExit(f"mutant {m['id']} {m['key']}: edit {k + 1}'s text occurs {n} times")
        at = -1
        for _ in range(occ):
            at = text.index(old, at + 1)
        text = text[:at] + new + text[at + len(old):]
    return text


def run(m, force):
    res_path = f"{OUT}/{m['id']}.json"
    if os.path.exists(res_path) and not force:
        print(f"{m['id']} {m['key']} kept")
        return
    if not status_clean():
        raise SystemExit("the tree is not clean")
    path = os.path.join(REPO, m["file"])
    orig = open(path, encoding="utf-8").read()
    open(path, "w", encoding="utf-8").write(apply(m, orig))
    t0 = time.time()
    log = open(f"{OUT}/{m['id']}.log", "w")
    res = {"id": m["id"], "key": m["key"], "observed": [], "compile": False, "guard_failed": False}
    try:
        rc, out = sh(["cargo", "build", "--locked", "--workspace"], log=log)
        res["compile"] |= rc != 0
        tests = [r[2:] for r in m["runs"] if r.startswith("t:")]
        rc, out = sh(SH + ["--exact"] + tests + [GUARD], log=log)
        res["compile"] |= "error: could not compile `" in out
        f = failed_of(out)
        res["guard_failed"] = GUARD in f
        res["observed"] += [x for x in f if x != GUARD]
        for name in [r for r in m["runs"] if r in RUNS]:
            cmd, env = RUNS[name]
            if name == "gate_tmp":
                assert GATE_TMP.endswith("/gate-tmp") and P
                shutil.rmtree(GATE_TMP, ignore_errors=True)
                os.makedirs(GATE_TMP)
            rc, out = sh(cmd, env=env, log=log)
            res.setdefault("runs", {})[name] = rc
            res["compile"] |= "error: could not compile `" in out
            if name == "noclang_req":
                if rc == 0:
                    res["observed"].append("noclang_req passes")
                elif "NOVA_REQUIRE_CLANG=1, but" not in out:
                    res["observed"].append("noclang_req fails otherwise")
            elif name == "noclang":
                if rc != 0:
                    res["observed"].append("noclang fails")
            else:
                res["observed"] += [x for x in failed_of(out) if x not in res["observed"]]
                res.setdefault("reports", {})[name] = re.findall(r"^bootstrap: .*|^the bootstrap .*|^S2 and S3 linked .*", out, re.M)
    finally:
        subprocess.run(["git", "checkout", "--", m["file"]], cwd=REPO, check=True)
        log.close()
    if not status_clean():
        raise SystemExit(f"mutant {m['id']}: the tree is not clean after it")
    res["caught"] = bool(res["observed"]) or res["compile"] or res["guard_failed"]
    res["as_expected"] = sorted(res["observed"]) == sorted(m["expect"]) and not res["compile"] \
        and not res["guard_failed"]
    res["seconds"] = round(time.time() - t0, 1)
    json.dump(res, open(res_path, "w"), indent=1)
    print(f"{m['id']} {m['key']}: {'CAUGHT' if res['caught'] else 'SURVIVED'} "
          f"{'as expected' if res['as_expected'] else 'NOT AS EXPECTED'} {res['seconds']} s "
          f"failed={res['observed']} guard_failed={res['guard_failed']} compile={res['compile']}",
          flush=True)


def main():
    args = sys.argv[1:]
    force = "--force" in args
    check = "--check" in args
    args = [a for a in args if not a.startswith("--")]
    sel = [m for m in MUTANTS if m["id"] in ids(args)] if args else MUTANTS
    if check:
        for m in sel:
            apply(m, open(os.path.join(REPO, m["file"]), encoding="utf-8").read())
        print(f"{len(sel)} mutants: every edit's text found")
        return
    for m in sel:
        run(m, force)


main()
EOF
```

```sh
mkdir -p "$P/exec56"
cat > "$P/exec56/mutants.json" <<'EOF'
[
 {
  "id": 1,
  "key": "M1",
  "title": "comparison skipped: N's .ll S1 vs S0",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "for (a, b) in [(\"S0\", \"S1\"), (\"S0\", \"S2\")] {",
    "for (a, b) in [(\"S0\", \"S2\")] {"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 2,
  "key": "M2",
  "title": "comparison skipped: N's .ll S2 vs S0",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "for (a, b) in [(\"S0\", \"S1\"), (\"S0\", \"S2\")] {",
    "for (a, b) in [(\"S0\", \"S1\")] {"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 3,
  "key": "M3",
  "title": "comparison skipped: binaries S1 vs S2",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "for (a, b) in [(\"S1\", \"S2\"), (\"S2\", \"S3\")] {",
    "for (a, b) in [(\"S2\", \"S3\")] {"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 4,
  "key": "M4",
  "title": "comparison skipped: binaries S2 vs S3",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "for (a, b) in [(\"S1\", \"S2\"), (\"S2\", \"S3\")] {",
    "for (a, b) in [(\"S1\", \"S2\")] {"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 5,
  "key": "M5",
  "title": "a stage compared with itself (S3 vs S3)",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "for (a, b) in [(\"S1\", \"S2\"), (\"S2\", \"S3\")] {",
    "for (a, b) in [(\"S1\", \"S2\"), (\"S3\", \"S3\")] {"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 6,
  "key": "M6",
  "title": "== loosened: same_binaries compares sizes",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "if a_bytes == b_bytes {",
    "if a_bytes.len() == b_bytes.len() {"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 7,
  "key": "M7",
  "title": "== loosened: ll_difference compares sizes",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "let (a_ll, b_ll) = (read(a_path)?, read(b_path)?);",
    "let (a_ll, b_ll) = (read(a_path)?, read(b_path)?);\n    if a_ll.len() == b_ll.len() {\n        return Ok(());\n    }"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 8,
  "key": "M8",
  "title": "the .ll runs not compared (both .ll comparisons gone)",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "for (a, b) in [(\"S0\", \"S1\"), (\"S0\", \"S2\")] {",
    "for (a, b) in [(\"S0\", \"S1\"), (\"S0\", \"S2\")].into_iter().take(0) {"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 9,
  "key": "M9",
  "title": "`same_binaries`' offset one off",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "        .unwrap_or(a_bytes.len().min(b_bytes.len()));",
    "        .map(|at| at + 1)\n        .unwrap_or(a_bytes.len().min(b_bytes.len()));"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 10,
  "key": "M10",
  "title": "`ll_difference`'s define and block: no (Mir line, so no place",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "b\"== selfhost/src/main.nova\\n(Mir bootstrap)\\n(Ll ok)\\n\"",
    "b\"== selfhost/src/main.nova\\n(Ll ok)\\n\""
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 11,
  "key": "M11",
  "title": "`ll_difference`'s define and block: the place dropped for the line alone",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "let at = m.place.unwrap_or_else(|| format!(\"line {line} of its .ll\"));",
    "let at = format!(\"line {line} of its .ll\");"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 12,
  "key": "M12",
  "title": "`ci.yml`: the new name dropped from the step",
  "group": "proto",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3\n",
    "          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 13,
  "key": "M13",
  "title": "`ci.yml`: the new name misspelt",
  "group": "proto",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3\n",
    "          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s4\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 14,
  "key": "M14",
  "title": "`ci.yml`: the new name on both steps",
  "group": "proto",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "--exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang\n",
    "--exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang the_bootstrap_holds_s1_s2_s3\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 15,
  "key": "M15",
  "title": "`ci.yml`: --ignored dropped from the new step",
  "group": "proto",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3\n",
    "          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --nocapture --exact the_bootstrap_holds_s1_s2_s3\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 16,
  "key": "M16",
  "title": "`ci.yml`: the step moved out of the Selfhost job (to the end of the vscode job)",
  "group": "proto",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "      # The bootstrap gate (spec\n      # docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md\n      # §3, §6; ADR 0060): S0 `--release` builds N into S1, then S1 builds\n      # N into S2, then S2 builds it into S3, in order, one `-o` basename,\n      # on the release runtime S0 carries and `clang-18`; N's `.ll` from S1\n      # and S2 must equal S0's, and S1 == S2 == S3 byte for byte. On the\n      # Linux leg alone, in a step of its own so that CI times it by itself;\n      # `cc` and `ld` are printed, not pinned: one of each within a job is\n      # one of ADR 0060's conditions.\n      - name: The bootstrap, S1 == S2 == S3 (Linux)\n        if: runner.os == 'Linux'\n        env:\n          NOVA_REQUIRE_CLANG: \"1\"\n          NOVA_CLANG: clang-18\n        run: |\n          cc --version\n          ld --version\n          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3\n",
    ""
   ],
   [
    "          if-no-files-found: error\n",
    "          if-no-files-found: error\n      # The bootstrap gate (spec\n      # docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md\n      # §3, §6; ADR 0060): S0 `--release` builds N into S1, then S1 builds\n      # N into S2, then S2 builds it into S3, in order, one `-o` basename,\n      # on the release runtime S0 carries and `clang-18`; N's `.ll` from S1\n      # and S2 must equal S0's, and S1 == S2 == S3 byte for byte. On the\n      # Linux leg alone, in a step of its own so that CI times it by itself;\n      # `cc` and `ld` are printed, not pinned: one of each within a job is\n      # one of ADR 0060's conditions.\n      - name: The bootstrap, S1 == S2 == S3 (Linux)\n        if: runner.os == 'Linux'\n        env:\n          NOVA_REQUIRE_CLANG: \"1\"\n          NOVA_CLANG: clang-18\n        run: |\n          cc --version\n          ld --version\n          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 17,
  "key": "M17",
  "title": "`ci.yml`: the new step without its cargo test line",
  "group": "proto",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3\n",
    ""
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 18,
  "key": "M18",
  "title": "the ignore reason changed",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "#[ignore = \"needs clang; the Selfhost job's Linux step runs it\"]\nfn the_bootstrap_holds_s1_s2_s3",
    "#[ignore = \"slow; the Selfhost job's Linux step runs it\"]\nfn the_bootstrap_holds_s1_s2_s3"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 19,
  "key": "M19",
  "title": "the gate no longer ignored",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "#[ignore = \"needs clang; the Selfhost job's Linux step runs it\"]\nfn the_bootstrap_holds_s1_s2_s3",
    "fn the_bootstrap_holds_s1_s2_s3"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 20,
  "key": "G1",
  "title": "the basename differing per stage: S3 built as selfhosx",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "    dir.join(stage.to_lowercase()).join(\"selfhost\")",
    "    dir.join(stage.to_lowercase())\n        .join(if stage == \"S3\" { \"selfhosx\" } else { \"selfhost\" })"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ]
 },
 {
  "id": 21,
  "key": "G2",
  "title": "the order: S2 built by S0 instead of S1",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "        builder = out;",
    "        builder = if k == 0 { nova() } else { out };"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ]
 },
 {
  "id": 22,
  "key": "G3",
  "title": "a front-end run with a real clang",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    ".env(\"NOVA_HOME\", &home_ll)\n            .env(\"NOVA_CLANG\", &missing)",
    ".env(\"NOVA_HOME\", &home_ll)\n            .env(\"NOVA_CLANG\", &clang)"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ]
 },
 {
  "id": 23,
  "key": "G4",
  "title": "S2 linked with another runtime than S1's (the debug one beside S0)",
  "group": "proto",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "            build.env(\"NOVA_RUNTIME_LIB\", runtime);",
    "            if k == 1 {\n                build.env(\"NOVA_RUNTIME_LIB\", nova().with_file_name(\"libnova_runtime.a\"));\n            } else {\n                build.env(\"NOVA_RUNTIME_LIB\", runtime);\n            }"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ]
 },
 {
  "id": 24,
  "key": "W1",
  "title": "`ll_difference`'s head names the stages the other way",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "N's .ll from {b_name} differs from {a_name}'s at {at}",
    "N's .ll from {a_name} differs from {b_name}'s at {at}"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 25,
  "key": "W2",
  "title": "`ll_difference`'s two lines labelled the other way",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "at {at}\\n  {a_name}: {}\\n  {b_name}: {}",
    "at {at}\\n  {b_name}: {}\\n  {a_name}: {}"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 26,
  "key": "W3",
  "title": "`both kept: diff` worded otherwise",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "both kept: diff {} {}",
    "kept: diff {} {}"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ],
  "occ": 1
 },
 {
  "id": 27,
  "key": "W4",
  "title": "the `diff` line's two paths swapped",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "        a_path.display(),\n        b_path.display()\n",
    "        b_path.display(),\n        a_path.display()\n"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 28,
  "key": "W5",
  "title": "`(nothing: the .ll ended)` worded otherwise",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "\"(nothing: the .ll ended)\".to_string()",
    "\"(nothing)\".to_string()"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 29,
  "key": "W6",
  "title": "the fallback place worded otherwise",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "format!(\"line {line} of its .ll\")",
    "format!(\"line {line}\")"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 30,
  "key": "W7",
  "title": "the fallback place's line one off",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "let line = 1 + a_ll",
    "let line = a_ll"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 31,
  "key": "W8",
  "title": "`ll_difference`'s unreadable file worded otherwise",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "format!(\"cannot read {}: {e}\", path.display())",
    "format!(\"cannot open {}: {e}\", path.display())"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ],
  "occ": 2
 },
 {
  "id": 32,
  "key": "W9",
  "title": "`ll_difference`'s part without `(Ll ok)`",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "\\n(Mir bootstrap)\\n(Ll ok)\\n",
    "\\n(Mir bootstrap)\\n"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 33,
  "key": "B1",
  "title": "`same_binaries` names the stages the other way",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "{b_name}'s binary differs from {a_name}'s: {} bytes",
    "{a_name}'s binary differs from {b_name}'s: {} bytes"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 34,
  "key": "B2",
  "title": "`same_binaries`' two sizes swapped",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "        b_bytes.len(),\n        a_bytes.len()\n",
    "        a_bytes.len(),\n        b_bytes.len()\n"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 35,
  "key": "B3",
  "title": "`first at offset` worded otherwise",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "bytes against {a_name}'s {}, first at",
    "bytes against {a_name}'s {}, at"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 36,
  "key": "B4",
  "title": "the shorter file's length taken as the first's",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    ".unwrap_or(a_bytes.len().min(b_bytes.len()));",
    ".unwrap_or(a_bytes.len());"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ]
 },
 {
  "id": 37,
  "key": "B5",
  "title": "`same_binaries`' unreadable file worded otherwise",
  "group": "words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "format!(\"cannot read {}: {e}\", path.display())",
    "format!(\"cannot open {}: {e}\", path.display())"
   ]
  ],
  "runs": [
   "t:the_bootstrap_reports_name_the_stage"
  ],
  "expect": [
   "the_bootstrap_reports_name_the_stage"
  ],
  "occ": 1
 },
 {
  "id": 38,
  "key": "C1",
  "title": "the `run: |` block's first line taken as the command",
  "group": "ci",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    ".find(|l| l.starts_with(\"cargo test\"))",
    ".find(|l| !l.is_empty())"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 39,
  "key": "C2",
  "title": "the check that no name is on both steps removed",
  "group": "ci",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "            assert!(\n                !names.contains(name),\n                \"`{name}` is named more than once on the steps\"\n            );\n",
    ""
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [],
  "accept": "redundant: a name on both steps is counted twice, so the steps' names no longer equal the clang tests and the final assertion fails (M14's edit with this one still fails it); only the message is lost"
 },
 {
  "id": 40,
  "key": "C3",
  "title": "`ci.yml`: the new step without `NOVA_REQUIRE_CLANG`",
  "group": "ci",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "          NOVA_REQUIRE_CLANG: \"1\"\n          NOVA_CLANG: clang-18\n        run: |\n",
    "          NOVA_CLANG: clang-18\n        run: |\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 41,
  "key": "C4",
  "title": "`ci.yml`: the new step without `--nocapture`",
  "group": "ci",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3\n",
    "          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --exact the_bootstrap_holds_s1_s2_s3\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 42,
  "key": "C5",
  "title": "`ci.yml`: the new step on every system (its `if:` gone)",
  "group": "ci",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "        if: runner.os == 'Linux'\n        env:\n          NOVA_REQUIRE_CLANG: \"1\"\n          NOVA_CLANG: clang-18\n        run: |\n",
    "        env:\n          NOVA_REQUIRE_CLANG: \"1\"\n          NOVA_CLANG: clang-18\n        run: |\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 43,
  "key": "C6",
  "title": "`ci.yml`: the new step prints neither `cc`'s nor `ld`'s version",
  "group": "ci",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "          cc --version\n          ld --version\n",
    ""
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 44,
  "key": "C7",
  "title": "`ci.yml`: the golden step on every system (its `if:` gone)",
  "group": "ci",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "        if: runner.os == 'Linux'\n        env:\n          NOVA_REQUIRE_CLANG: \"1\"\n          NOVA_CLANG: clang-18\n        run: cargo test",
    "        env:\n          NOVA_REQUIRE_CLANG: \"1\"\n          NOVA_CLANG: clang-18\n        run: cargo test"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 45,
  "key": "C8",
  "title": "`ci.yml`: the new step prints `cc`'s version but not `ld`'s",
  "group": "ci",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "          cc --version\n          ld --version\n",
    "          cc --version\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 46,
  "key": "C9",
  "title": "`ci.yml`: the new step prints `ld`'s version but not `cc`'s",
  "group": "ci",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "          cc --version\n          ld --version\n",
    "          ld --version\n"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test"
  ],
  "expect": [
   "the_ci_step_names_every_clang_test"
  ]
 },
 {
  "id": 47,
  "key": "GW1",
  "title": "the stop's head worded otherwise",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "the bootstrap stopped at {stage}; no later stage was built",
    "the bootstrap stopped at {stage}"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full)"
 },
 {
  "id": 48,
  "key": "GW2",
  "title": "a failed build's line without its builder",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "\"{stage}'s build by {by} failed ({})",
    "\"{stage}'s build failed ({})"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full)"
 },
 {
  "id": 49,
  "key": "GW3",
  "title": "a wrong `built OUT`'s line without the line wanted",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "printed {:?}, not {want:?}",
    "printed {:?}"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full)"
 },
 {
  "id": 50,
  "key": "GW4",
  "title": "a front-end run's line without `NOVA_CLANG`'s part",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "{by}'s front-end run, `NOVA_CLANG` a file that is not there, ended",
    "{by}'s front-end run ended"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full)"
 },
 {
  "id": 51,
  "key": "GW5",
  "title": "`did not leave` worded otherwise",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "\"did not leave\"",
    "\"left no\""
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full)"
 },
 {
  "id": 52,
  "key": "GW6",
  "title": "the failures' head without the count's total",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "the bootstrap does not hold: {} of 4 comparisons failed",
    "the bootstrap does not hold: {} comparisons failed"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full)"
 },
 {
  "id": 53,
  "key": "GW7",
  "title": "the failures' report without `kept:`",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "comparisons failed:\\n\\n{}\\n\\nkept: {}",
    "comparisons failed:\\n\\n{}\\n\\n{}"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full)"
 },
 {
  "id": 54,
  "key": "GS1",
  "title": "the sizes' line without `each`",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "S1 == S2 == S3, {} bytes each;",
    "S1 == S2 == S3, {} bytes;"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7)"
 },
 {
  "id": 55,
  "key": "GS2",
  "title": "`clang`'s version not printed",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "println!(\"bootstrap: {version}\");",
    "println!(\"bootstrap: clang\");"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7)"
 },
 {
  "id": 56,
  "key": "GS3",
  "title": "the runtime's line without `runtime`",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "\"bootstrap: runtime {}\",",
    "\"bootstrap: {}\","
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7)"
 },
 {
  "id": 57,
  "key": "GS4",
  "title": "a stage's line without the builder's name",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "{by}'s .ll beside it {:.2} s",
    "the .ll beside it {:.2} s"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7)"
 },
 {
  "id": 58,
  "key": "GS5",
  "title": "the total's line without `in all`",
  "group": "gate-words",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "{:.2} s in all",
    "{:.2} s"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7)"
 },
 {
  "id": 59,
  "key": "GB1",
  "title": "a build's `built OUT` not checked",
  "group": "gate",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "if o.stdout != want.as_bytes() {",
    "if false && o.stdout != want.as_bytes() {"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "met elsewhere: S1's `built OUT` is held by the golden walk, N's against S0's by the clang test, S0's by `project.rs`; a wrong line beside a right binary changes no comparison"
 },
 {
  "id": 60,
  "key": "GB2",
  "title": "a front-end run's exit status not checked",
  "group": "gate",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "Ok(o) if o.status.code() == Some(1) && ll.is_file() => {}",
    "Ok(_) if ll.is_file() => {}"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "redundant: a run that succeeds removes its .ll (G3), so a .ll is left only by a run stopped at clang; any .ll left is compared"
 },
 {
  "id": 61,
  "key": "GB3",
  "title": "a front-end run's .ll not checked",
  "group": "gate",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "Ok(o) if o.status.code() == Some(1) && ll.is_file() => {}",
    "Ok(o) if o.status.code() == Some(1) => {}"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "redundant: a .ll not left fails its comparison (`cannot read`), later and with other words"
 },
 {
  "id": 62,
  "key": "GB4",
  "title": "a failed stage does not stop the chain",
  "group": "gate",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "            why.is_empty(),\n",
    "            why.is_empty() || true,\n"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "reached only when a stage fails (G2, G3 show the stop); then the later stages' builds fail too, or their comparisons do"
 },
 {
  "id": 63,
  "key": "GB5",
  "title": "S0's front-end run without `NOVA_LLC`",
  "group": "gate",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "            .env(\"NOVA_CLANG\", &missing)\n            .env(\"NOVA_LLC\", &missing);",
    "            .env(\"NOVA_CLANG\", &missing);"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "equivalent, with `llc` on PATH or without it: with it (18.1.3 here), S0 compiles the .ll with `llc`, whose non-PIE object `cc` refuses to link; without it, S0 stops with \"no LLVM toolchain found\"; either way the run exits 1 and leaves the same .ll (with `llc`, in about 10 s, not 1.4). The gate sets `NOVA_LLC`, so it depends on neither"
 },
 {
  "id": 64,
  "key": "H1",
  "title": "no runtime handed to S2's and S3's builds",
  "group": "handoff",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "            build.env(\"NOVA_RUNTIME_LIB\", runtime);\n            front.env(\"NOVA_RUNTIME_LIB\", runtime);\n",
    "            let _ = runtime;\n"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ]
 },
 {
  "id": 65,
  "key": "H2",
  "title": "no runtime handed to S1's and S2's front-end runs",
  "group": "handoff",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "            front.env(\"NOVA_RUNTIME_LIB\", runtime);\n",
    ""
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "equivalent: a front-end run stops at the clang probe, before either driver looks for a runtime (spec §3.3)"
 },
 {
  "id": 66,
  "key": "H3",
  "title": "the runtime taken from the front-end runs' home",
  "group": "handoff",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "runtime = Some(runtime_in(&home));",
    "runtime = Some(runtime_in(&home_ll));"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ]
 },
 {
  "id": 67,
  "key": "H4",
  "title": "the runtime taken before S1's build, so S1 links it too",
  "group": "handoff",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "let mut runtime: Option<PathBuf> = None;",
    "let mut runtime: Option<PathBuf> = Some(runtime_in(&home));"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ]
 },
 {
  "id": 68,
  "key": "H5",
  "title": "the runtime taken again after every stage",
  "group": "handoff",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "        if runtime.is_none() {\n            runtime = Some(runtime_in(&home));\n        }\n",
    "        runtime = Some(runtime_in(&home));\n"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [],
  "accept": "equivalent: only S1's build writes into home (N reads no NOVA_HOME), so each call finds the same file"
 },
 {
  "id": 69,
  "key": "R1",
  "title": "`runtime_in` looks in the wrong directory under home",
  "group": "runtime_in",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "let unpacked = home.join(\"runtime\");",
    "let unpacked = home.join(\"runtimes\");"
   ]
  ],
  "runs": [
   "gate"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ],
  "occ": 1
 },
 {
  "id": 70,
  "key": "R2",
  "title": "`runtime_in` ignores home (the golden walk's directory always)",
  "group": "runtime_in",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "let unpacked = home.join(\"runtime\");",
    "let unpacked = std::env::temp_dir().join(\"nova-selfhost-home\").join(\"runtime\");\n    let _ = home;"
   ]
  ],
  "runs": [
   "gate",
   "gate_tmp"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ],
  "occ": 1
 },
 {
  "id": 71,
  "key": "R3",
  "title": "`runtime_in` ignores NOVA_RUNTIME_LIB",
  "group": "runtime_in",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "    if let Some(lib) = std::env::var_os(\"NOVA_RUNTIME_LIB\") {\n        return PathBuf::from(lib);\n    }\n    let name = \"libnova_runtime.a\";\n",
    "    let name = \"libnova_runtime.a\";\n"
   ]
  ],
  "runs": [
   "gate",
   "gate_rt"
  ],
  "expect": [
   "the_bootstrap_holds_s1_s2_s3"
  ]
 },
 {
  "id": 72,
  "key": "N1",
  "title": "no clang passes under NOVA_REQUIRE_CLANG=1",
  "group": "noclang",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "            !required,\n            \"NOVA_REQUIRE_CLANG=1, but",
    "            true,\n            \"NOVA_REQUIRE_CLANG=1, but"
   ]
  ],
  "runs": [
   "noclang_req",
   "noclang"
  ],
  "expect": [
   "noclang_req passes"
  ],
  "occ": 3
 },
 {
  "id": 73,
  "key": "N2",
  "title": "the requirement read the other way",
  "group": "noclang",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "            !required,\n            \"NOVA_REQUIRE_CLANG=1, but",
    "            required,\n            \"NOVA_REQUIRE_CLANG=1, but"
   ]
  ],
  "runs": [
   "noclang_req",
   "noclang"
  ],
  "expect": [
   "noclang_req passes",
   "noclang fails"
  ],
  "occ": 3
 },
 {
  "id": 74,
  "key": "N3",
  "title": "`not run (no clang)` worded otherwise",
  "group": "noclang",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "eprintln!(\"bootstrap: not run (no clang)\");",
    "eprintln!(\"bootstrap: skipped\");"
   ]
  ],
  "runs": [
   "noclang"
  ],
  "expect": [],
  "accept": "output only, as the other clang tests' lines"
 }
]
EOF
```

  Then `python3 "$P/exec56/mutrun.py" --check`, which prints `74
  mutants: every edit's text found`. These names are set, for the
  baseline below, and `C` is the Runs block's:

  ```sh
  REPORT=the_bootstrap_reports_name_the_stage
  CITEST=the_ci_step_names_every_clang_test
  GATE=the_bootstrap_holds_s1_s2_s3
  GUARD=the_resolution_dump_has_a_header_per_root_and_std_its_own_part
  ```

  - First, unmutated. Before the build, which changes the runtime: the
    disk check, `Task 4 Step 8, before the embedded build`, at least
    7,168 MiB, or remove the `nova-*` and `selfhost-*`
    executables of the runtime and level the last build left first (the
    replay came here from Step 7's `opt-level = 1` build, read 6.7G,
    6,846 MiB, removed its three, and read 7.1G, 7,188 MiB). Then `cargo
    build --locked --workspace` (14 s; `nova version` prints `runtime:
    embedded`); before the test build, the disk check at least 6,451
    MiB (6.3G; the replay read 6.9G, 6,981 MiB); `cargo test --locked
    --workspace --test selfhost --no-run` (5 s; the replay then read
    6.6G, 6,686 MiB), then:
    - `SH --exact $REPORT $CITEST $GUARD`: `3 passed`, 9 s. `$GUARD`
      builds N through `selfhost()`, so S0 unpacks its runtime into
      `nova-selfhost-home`;
    - `mkdir -p "$P/exec56/rt" && cp /tmp/nova-selfhost-home/runtime/0.4.0-*/libnova_runtime.a
      "$P/exec56/rt/" && sha256sum "$P/exec56/rt/libnova_runtime.a"`:
      one file, the release runtime, for the runs that set
      `NOVA_RUNTIME_LIB` (its sha256 began `7924727dc09fc8bb` here, this
      machine's, with Rust 1.97: recorded, not expected);
    - `C $GATE --nocapture`: `1 passed`, 48.91 s, `S1 == S2 == S3` at
      one size (9354680 bytes here, this machine's figure), the runtime
      under the gate's `home/runtime/`;
      `NOVA_RUNTIME_LIB="$P/exec56/rt/libnova_runtime.a" C $GATE
      --nocapture`: `1 passed`, 49.24 s; and, with an empty temporary
      directory, `rm -rf "${P:?}/exec56/gate-tmp" && mkdir "$P/exec56/gate-tmp"
      && TMPDIR="$P/exec56/gate-tmp" C $GATE --nocapture`: `1 passed`,
      46.78 s, the runtime under `$P/exec56/gate-tmp/nova-selfhost-bootstrap/home/runtime/`;
    - **the no-clang check, required:** `NOVA_REQUIRE_CLANG=1
      NOVA_CLANG="$P/exec56/no-clang" SH --ignored --exact $GATE` (a file
      never made): it fails, `1 failed`, its panic `NOVA_REQUIRE_CLANG=1,
      but `…/no-clang --version` does not run`;
    - **the no-clang check, not required:** `NOVA_CLANG="$P/exec56/no-clang"
      SH --ignored --exact $GATE --nocapture`: `1 passed`, and `bootstrap:
      not run (no clang)` on stderr.

    So every mutant runs against a baseline that does what it must.
  - Each mutant runs only the tests it should trip and `$GUARD`, a cheap
    test that builds N and compares its resolution dump with S0's, which
    must pass every time: a mutant that broke everything would fail it.
    A mutant of the gate runs `C $GATE --nocapture`; the runner reads its
    report. R2 also runs it with `TMPDIR` at `$P/exec56/gate-tmp`, which
    the runner removes and makes empty first, so that no
    `nova-selfhost-home` lies there (`gate_tmp`).
  - Run them in ten batches, each one command in the foreground,
    `python3 "$P/exec56/mutrun.py" 1-19` and so on, and the disk check,
    `Task 4 Step 8, before batch 1-19` and so on, at least 6,451 MiB
    (6.3G) before each, since each builds. They are sized
    so that each fits under 600 s at twice the replay's time, at most 300
    s at the replay's: 1 to 19 (228 s), 20 to 28 (238 s), 29 to 46 (213
    s), 47 to 50 (216 s), 51 to 54 (211 s), 55 to 58 (220 s), 59 to 62
    (214 s), 63 to 66 (206 s), 67 to 69 (187 s) and 70 to 74 (243 s);
    2,174 s in all, one run, at `6edde36e`, the second split's last
    commit, whose tree differs from `11a0d088`'s only in ADR 0060's
    text (decision 17). The runner prints each mutant's line,
    `CAUGHT` or `SURVIVED`, `as expected` or `NOT AS EXPECTED` against
    the mutant's own list in `mutants.json`, and what failed: the tests,
    or `noclang_req passes` (the required no-clang check passed, where it
    must fail) and `noclang fails` (the other failed, where it must
    pass). If a batch is cut off, killed at its timeout or by a
    shell's restart, restore the tree first: `git checkout -- <the
    mutated file>`, the file of the mutant it was on, then `git status
    --short` prints nothing. Then move the results aside to a fixed
    name, `rm -rf "${P:?}/exec56/mut-cut" && mv "$P/exec56/mut"
    "$P/exec56/mut-cut"` (the runner skips a mutant whose result file
    exists), and run every batch again from mutant 1, at the same tip:
    all 74 results come from one run (decision 17).
  - Each edit replaces one exact text, which occurs once in its file
    unless the mutant names which occurrence, counted on the text as
    shown ("the third of 3"); "the line … goes" removes that whole line.
  - Each command list starts with `cargo build --locked --workspace`
    (the runner runs it). A mutant of `selfhost.rs` or `ci.yml` needs no
    Rust rebuilt but the test.
  - Each caught mutant must fail as written: the tests named, and no
    other among those run. 21 pass by design, each with its reason
    below. Any other mutant that passes, or one that fails otherwise, is
    a gap: record it in the ledger, name it in the PR, and give it to the
    review (Step 10), which says whether a check is added.
  - At the end, `git status --short` prints nothing, then `rm -rf
    "${P:?}/exec56/gate-tmp"` and `unset NOVA_EMBED_RUNTIME`; before the
    build, which changes the runtime back, the disk check, `Task 4 Step
    8, before the plain build`, at least 7,168 MiB, or remove the
    `nova-*` and `selfhost-*` executables in `target/debug/deps` first
    (the replay read 6.5G, 6,611 MiB, removed the three and `gate-tmp`,
    and read 6.8G, 6,905 MiB); then `cargo
    build --locked --workspace` (`runtime: not embedded`).

  1. **M1: Comparison skipped: N's .ll S1 vs S0.** `crates/nova-cli/tests/selfhost.rs`: `for (a, b) in [("S0", "S1"), ("S0", "S2")] {` becomes `for (a, b) in [("S0", "S2")] {`.
     - Run `SH --exact $REPORT $GUARD`.
     - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  2. **M2: Comparison skipped: N's .ll S2 vs S0.** `crates/nova-cli/tests/selfhost.rs`: `for (a, b) in [("S0", "S1"), ("S0", "S2")] {` becomes `for (a, b) in [("S0", "S1")] {`.
     - Run `SH --exact $REPORT $GUARD`.
     - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  3. **M3: Comparison skipped: binaries S1 vs S2.** `crates/nova-cli/tests/selfhost.rs`: `for (a, b) in [("S1", "S2"), ("S2", "S3")] {` becomes `for (a, b) in [("S2", "S3")] {`.
     - Run `SH --exact $REPORT $GUARD`.
     - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  4. **M4: Comparison skipped: binaries S2 vs S3.** `crates/nova-cli/tests/selfhost.rs`: `for (a, b) in [("S1", "S2"), ("S2", "S3")] {` becomes `for (a, b) in [("S1", "S2")] {`.
     - Run `SH --exact $REPORT $GUARD`.
     - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  5. **M5: A stage compared with itself (S3 vs S3).** `crates/nova-cli/tests/selfhost.rs`: `for (a, b) in [("S1", "S2"), ("S2", "S3")] {` becomes `for (a, b) in [("S1", "S2"), ("S3", "S3")] {`.
     - Run `SH --exact $REPORT $GUARD`.
     - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  6. **M6: == loosened: same_binaries compares sizes.** `crates/nova-cli/tests/selfhost.rs`: `if a_bytes == b_bytes {` becomes `if a_bytes.len() == b_bytes.len() {`.
     - Run `SH --exact $REPORT $GUARD`.
     - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  7. **M7: == loosened: ll_difference compares sizes.** `crates/nova-cli/tests/selfhost.rs`: replace

     ```
     let (a_ll, b_ll) = (read(a_path)?, read(b_path)?);
     ```

     with

     ```
     let (a_ll, b_ll) = (read(a_path)?, read(b_path)?);
         if a_ll.len() == b_ll.len() {
             return Ok(());
         }
     ```

     - Run `SH --exact $REPORT $GUARD`.
     - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  8. **M8: The .ll runs not compared (both .ll comparisons gone).** `crates/nova-cli/tests/selfhost.rs`: `for (a, b) in [("S0", "S1"), ("S0", "S2")] {` becomes `for (a, b) in [("S0", "S1"), ("S0", "S2")].into_iter().take(0) {`.
     - Run `SH --exact $REPORT $GUARD`.
     - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  9. **M9: `same_binaries`' offset one off.** `crates/nova-cli/tests/selfhost.rs`: replace

     ```
             .unwrap_or(a_bytes.len().min(b_bytes.len()));
     ```

     with

     ```
             .map(|at| at + 1)
             .unwrap_or(a_bytes.len().min(b_bytes.len()));
     ```

     - Run `SH --exact $REPORT $GUARD`.
     - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  10. **M10: `ll_difference`'s define and block: no (Mir line, so no place.** `crates/nova-cli/tests/selfhost.rs`: `b"== selfhost/src/main.nova\n(Mir bootstrap)\n(Ll ok)\n"` becomes `b"== selfhost/src/main.nova\n(Ll ok)\n"`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  11. **M11: `ll_difference`'s define and block: the place dropped for the line alone.** `crates/nova-cli/tests/selfhost.rs`: `let at = m.place.unwrap_or_else(|| format!("line {line} of its .ll"));` becomes `let at = format!("line {line} of its .ll");`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  12. **M12: `ci.yml`: the new name dropped from the step.** `.github/workflows/ci.yml`: replace

      ```
                cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
      ```

      with

      ```
                cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  13. **M13: `ci.yml`: the new name misspelt.** `.github/workflows/ci.yml`: replace

      ```
                cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
      ```

      with

      ```
                cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s4
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  14. **M14: `ci.yml`: the new name on both steps.** `.github/workflows/ci.yml`: replace

      ```
      --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang
      ```

      with

      ```
      --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang the_bootstrap_holds_s1_s2_s3
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  15. **M15: `ci.yml`: --ignored dropped from the new step.** `.github/workflows/ci.yml`: replace

      ```
                cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
      ```

      with

      ```
                cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --nocapture --exact the_bootstrap_holds_s1_s2_s3
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  16. **M16: `ci.yml`: the step moved out of the Selfhost job (to the end of the vscode job).** `.github/workflows/ci.yml`, 2 edits, in order:

      1. replace

         ```
               # The bootstrap gate (spec
               # docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md
               # §3, §6; ADR 0060): S0 `--release` builds N into S1, then S1 builds
               # N into S2, then S2 builds it into S3, in order, one `-o` basename,
               # on the release runtime S0 carries and `clang-18`; N's `.ll` from S1
               # and S2 must equal S0's, and S1 == S2 == S3 byte for byte. On the
               # Linux leg alone, in a step of its own so that CI times it by itself;
               # `cc` and `ld` are printed, not pinned: one of each within a job is
               # one of ADR 0060's conditions.
               - name: The bootstrap, S1 == S2 == S3 (Linux)
                 if: runner.os == 'Linux'
                 env:
                   NOVA_REQUIRE_CLANG: "1"
                   NOVA_CLANG: clang-18
                 run: |
                   cc --version
                   ld --version
                   cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
         ```

         with nothing.


      2. replace

         ```
                   if-no-files-found: error
         ```

         with

         ```
                   if-no-files-found: error
               # The bootstrap gate (spec
               # docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md
               # §3, §6; ADR 0060): S0 `--release` builds N into S1, then S1 builds
               # N into S2, then S2 builds it into S3, in order, one `-o` basename,
               # on the release runtime S0 carries and `clang-18`; N's `.ll` from S1
               # and S2 must equal S0's, and S1 == S2 == S3 byte for byte. On the
               # Linux leg alone, in a step of its own so that CI times it by itself;
               # `cc` and `ld` are printed, not pinned: one of each within a job is
               # one of ADR 0060's conditions.
               - name: The bootstrap, S1 == S2 == S3 (Linux)
                 if: runner.os == 'Linux'
                 env:
                   NOVA_REQUIRE_CLANG: "1"
                   NOVA_CLANG: clang-18
                 run: |
                   cc --version
                   ld --version
                   cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
         ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  17. **M17: `ci.yml`: the new step without its cargo test line.** `.github/workflows/ci.yml`: the line `cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3` goes.
      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  18. **M18: The ignore reason changed.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
      #[ignore = "needs clang; the Selfhost job's Linux step runs it"]
      fn the_bootstrap_holds_s1_s2_s3
      ```

      with

      ```
      #[ignore = "slow; the Selfhost job's Linux step runs it"]
      fn the_bootstrap_holds_s1_s2_s3
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  19. **M19: The gate no longer ignored.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
      #[ignore = "needs clang; the Selfhost job's Linux step runs it"]
      fn the_bootstrap_holds_s1_s2_s3
      ```

      with

      ```
      fn the_bootstrap_holds_s1_s2_s3
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  20. **G1: The basename differing per stage: S3 built as selfhosx.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
          dir.join(stage.to_lowercase()).join("selfhost")
      ```

      with

      ```
          dir.join(stage.to_lowercase())
              .join(if stage == "S3" { "selfhosx" } else { "selfhost" })
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its report: `the bootstrap does not hold: 1 of 4 comparisons failed`.
  21. **G2: The order: S2 built by S0 instead of S1.** `crates/nova-cli/tests/selfhost.rs`: `builder = out;` becomes `builder = if k == 0 { nova() } else { out };`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its report: `the bootstrap stopped at S2; no later stage was built`.
  22. **G3: A front-end run with a real clang.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
      .env("NOVA_HOME", &home_ll)
                  .env("NOVA_CLANG", &missing)
      ```

      with

      ```
      .env("NOVA_HOME", &home_ll)
                  .env("NOVA_CLANG", &clang)
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its report: `the bootstrap stopped at S1; no later stage was built`.
  23. **G4: S2 linked with another runtime than S1's (the debug one beside S0).** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
                  build.env("NOVA_RUNTIME_LIB", runtime);
      ```

      with

      ```
                  if k == 1 {
                      build.env("NOVA_RUNTIME_LIB", nova().with_file_name("libnova_runtime.a"));
                  } else {
                      build.env("NOVA_RUNTIME_LIB", runtime);
                  }
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its report: `the bootstrap does not hold: 2 of 4 comparisons failed`.
  24. **W1: `ll_difference`'s head names the stages the other way.** `crates/nova-cli/tests/selfhost.rs`: `N's .ll from {b_name} differs from {a_name}'s at {at}` becomes `N's .ll from {a_name} differs from {b_name}'s at {at}`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  25. **W2: `ll_difference`'s two lines labelled the other way.** `crates/nova-cli/tests/selfhost.rs`: `at {at}\n  {a_name}: {}\n  {b_name}: {}` becomes `at {at}\n  {b_name}: {}\n  {a_name}: {}`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  26. **W3: `both kept: diff` worded otherwise.** `crates/nova-cli/tests/selfhost.rs`: `both kept: diff {} {}` (the first of 3) becomes `kept: diff {} {}`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  27. **W4: The `diff` line's two paths swapped.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
              a_path.display(),
              b_path.display()
      ```

      with

      ```
              b_path.display(),
              a_path.display()
      ```

      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  28. **W5: `(nothing: the .ll ended)` worded otherwise.** `crates/nova-cli/tests/selfhost.rs`: `"(nothing: the .ll ended)".to_string()` becomes `"(nothing)".to_string()`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  29. **W6: The fallback place worded otherwise.** `crates/nova-cli/tests/selfhost.rs`: `format!("line {line} of its .ll")` becomes `format!("line {line}")`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  30. **W7: The fallback place's line one off.** `crates/nova-cli/tests/selfhost.rs`: `let line = 1 + a_ll` becomes `let line = a_ll`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  31. **W8: `ll_difference`'s unreadable file worded otherwise.** `crates/nova-cli/tests/selfhost.rs`: `format!("cannot read {}: {e}", path.display())` (the second of 2) becomes `format!("cannot open {}: {e}", path.display())`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  32. **W9: `ll_difference`'s part without `(Ll ok)`.** `crates/nova-cli/tests/selfhost.rs`: `\n(Mir bootstrap)\n(Ll ok)\n` becomes `\n(Mir bootstrap)\n`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  33. **B1: `same_binaries` names the stages the other way.** `crates/nova-cli/tests/selfhost.rs`: `{b_name}'s binary differs from {a_name}'s: {} bytes` becomes `{a_name}'s binary differs from {b_name}'s: {} bytes`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  34. **B2: `same_binaries`' two sizes swapped.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
              b_bytes.len(),
              a_bytes.len()
      ```

      with

      ```
              a_bytes.len(),
              b_bytes.len()
      ```

      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  35. **B3: `first at offset` worded otherwise.** `crates/nova-cli/tests/selfhost.rs`: `bytes against {a_name}'s {}, first at` becomes `bytes against {a_name}'s {}, at`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  36. **B4: The shorter file's length taken as the first's.** `crates/nova-cli/tests/selfhost.rs`: `.unwrap_or(a_bytes.len().min(b_bytes.len()));` becomes `.unwrap_or(a_bytes.len());`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  37. **B5: `same_binaries`' unreadable file worded otherwise.** `crates/nova-cli/tests/selfhost.rs`: `format!("cannot read {}: {e}", path.display())` (the first of 2) becomes `format!("cannot open {}: {e}", path.display())`.
      - Run `SH --exact $REPORT $GUARD`.
      - Expected: fails: `the_bootstrap_reports_name_the_stage`.
  38. **C1: The `run: |` block's first line taken as the command.** `crates/nova-cli/tests/selfhost.rs`: `.find(|l| l.starts_with("cargo test"))` becomes `.find(|l| !l.is_empty())`.
      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  39. **C2: The check that no name is on both steps removed.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
                  assert!(
                      !names.contains(name),
                      "`{name}` is named more than once on the steps"
                  );
      ```

      with nothing.

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: passes, by design: redundant: a name on both steps is counted twice, so the steps' names no longer equal the clang tests and the final assertion fails (M14's edit with this one still fails it); only the message is lost.
  40. **C3: `ci.yml`: the new step without `NOVA_REQUIRE_CLANG`.** `.github/workflows/ci.yml`: replace

      ```
                NOVA_REQUIRE_CLANG: "1"
                NOVA_CLANG: clang-18
              run: |
      ```

      with

      ```
                NOVA_CLANG: clang-18
              run: |
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  41. **C4: `ci.yml`: the new step without `--nocapture`.** `.github/workflows/ci.yml`: replace

      ```
                cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
      ```

      with

      ```
                cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --exact the_bootstrap_holds_s1_s2_s3
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  42. **C5: `ci.yml`: the new step on every system (its `if:` gone).** `.github/workflows/ci.yml`: replace

      ```
              if: runner.os == 'Linux'
              env:
                NOVA_REQUIRE_CLANG: "1"
                NOVA_CLANG: clang-18
              run: |
      ```

      with

      ```
              env:
                NOVA_REQUIRE_CLANG: "1"
                NOVA_CLANG: clang-18
              run: |
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  43. **C6: `ci.yml`: the new step prints neither `cc`'s nor `ld`'s version.** `.github/workflows/ci.yml`: replace

      ```
                cc --version
                ld --version
      ```

      with nothing.

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  44. **C7: `ci.yml`: the golden step on every system (its `if:` gone).** `.github/workflows/ci.yml`: replace

      ```
              if: runner.os == 'Linux'
              env:
                NOVA_REQUIRE_CLANG: "1"
                NOVA_CLANG: clang-18
              run: cargo test
      ```

      with

      ```
              env:
                NOVA_REQUIRE_CLANG: "1"
                NOVA_CLANG: clang-18
              run: cargo test
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  45. **C8: `ci.yml`: the new step prints `cc`'s version but not `ld`'s.** `.github/workflows/ci.yml`: replace

      ```
                cc --version
                ld --version
      ```

      with

      ```
                cc --version
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  46. **C9: `ci.yml`: the new step prints `ld`'s version but not `cc`'s.** `.github/workflows/ci.yml`: replace

      ```
                cc --version
                ld --version
      ```

      with

      ```
                ld --version
      ```

      - Run `SH --exact $CITEST $GUARD`.
      - Expected: fails: `the_ci_step_names_every_clang_test`.
  47. **GW1: The stop's head worded otherwise.** `crates/nova-cli/tests/selfhost.rs`: `the bootstrap stopped at {stage}; no later stage was built` becomes `the bootstrap stopped at {stage}`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full).
  48. **GW2: A failed build's line without its builder.** `crates/nova-cli/tests/selfhost.rs`: `"{stage}'s build by {by} failed ({})` becomes `"{stage}'s build failed ({})`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full).
  49. **GW3: A wrong `built OUT`'s line without the line wanted.** `crates/nova-cli/tests/selfhost.rs`: `printed {:?}, not {want:?}` becomes `printed {:?}`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full).
  50. **GW4: A front-end run's line without `NOVA_CLANG`'s part.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
      {by}'s front-end run, `NOVA_CLANG` a file that is not there, ended
      ```

      with

      ```
      {by}'s front-end run ended
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full).
  51. **GW5: `did not leave` worded otherwise.** `crates/nova-cli/tests/selfhost.rs`: `"did not leave"` becomes `"left no"`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full).
  52. **GW6: The failures' head without the count's total.** `crates/nova-cli/tests/selfhost.rs`: `the bootstrap does not hold: {} of 4 comparisons failed` becomes `the bootstrap does not hold: {} comparisons failed`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full).
  53. **GW7: The failures' report without `kept:`.** `crates/nova-cli/tests/selfhost.rs`: `comparisons failed:\n\n{}\n\nkept: {}` becomes `comparisons failed:\n\n{}\n\n{}`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the gate's verdict does not depend on the words, which a failing chain alone prints (G1 to G4's reports show them in full).
  54. **GS1: The sizes' line without `each`.** `crates/nova-cli/tests/selfhost.rs`: `S1 == S2 == S3, {} bytes each;` becomes `S1 == S2 == S3, {} bytes;`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7).
  55. **GS2: `clang`'s version not printed.** `crates/nova-cli/tests/selfhost.rs`: `println!("bootstrap: {version}");` becomes `println!("bootstrap: clang");`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7).
  56. **GS3: The runtime's line without `runtime`.** `crates/nova-cli/tests/selfhost.rs`: `"bootstrap: runtime {}",` becomes `"bootstrap: {}",`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7).
  57. **GS4: A stage's line without the builder's name.** `crates/nova-cli/tests/selfhost.rs`: `{by}'s .ll beside it {:.2} s` becomes `the .ll beside it {:.2} s`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7).
  58. **GS5: The total's line without `in all`.** `crates/nova-cli/tests/selfhost.rs`: `{:.2} s in all` becomes `{:.2} s`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: output only: the success report is printed, not checked; the plan's steps read it (Task 4, Step 7).
  59. **GB1: A build's `built OUT` not checked.** `crates/nova-cli/tests/selfhost.rs`: `if o.stdout != want.as_bytes() {` becomes `if false && o.stdout != want.as_bytes() {`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: met elsewhere: S1's `built OUT` is held by the golden walk, N's against S0's by the clang test, S0's by `project.rs`; a wrong line beside a right binary changes no comparison.
  60. **GB2: A front-end run's exit status not checked.** `crates/nova-cli/tests/selfhost.rs`: `Ok(o) if o.status.code() == Some(1) && ll.is_file() => {}` becomes `Ok(_) if ll.is_file() => {}`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: redundant: a run that succeeds removes its .ll (G3), so a .ll is left only by a run stopped at clang; any .ll left is compared.
  61. **GB3: A front-end run's .ll not checked.** `crates/nova-cli/tests/selfhost.rs`: `Ok(o) if o.status.code() == Some(1) && ll.is_file() => {}` becomes `Ok(o) if o.status.code() == Some(1) => {}`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: redundant: a .ll not left fails its comparison (`cannot read`), later and with other words.
  62. **GB4: A failed stage does not stop the chain.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
                  why.is_empty(),
      ```

      with

      ```
                  why.is_empty() || true,
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: reached only when a stage fails (G2, G3 show the stop); then the later stages' builds fail too, or their comparisons do.
  63. **GB5: S0's front-end run without `NOVA_LLC`.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
                  .env("NOVA_CLANG", &missing)
                  .env("NOVA_LLC", &missing);
      ```

      with

      ```
                  .env("NOVA_CLANG", &missing);
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: equivalent, with `llc` on PATH or without it: with it (18.1.3 here), S0 compiles the .ll with `llc`, whose non-PIE object `cc` refuses to link; without it, S0 stops with "no LLVM toolchain found"; either way the run exits 1 and leaves the same .ll (with `llc`, in about 10 s, not 1.4). The gate sets `NOVA_LLC`, so it depends on neither.
  64. **H1: No runtime handed to S2's and S3's builds.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
                  build.env("NOVA_RUNTIME_LIB", runtime);
                  front.env("NOVA_RUNTIME_LIB", runtime);
      ```

      with

      ```
                  let _ = runtime;
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its report: `the bootstrap stopped at S2; no later stage was built`.
  65. **H2: No runtime handed to S1's and S2's front-end runs.** `crates/nova-cli/tests/selfhost.rs`: the line `front.env("NOVA_RUNTIME_LIB", runtime);` goes.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: equivalent: a front-end run stops at the clang probe, before either driver looks for a runtime (spec §3.3).
  66. **H3: The runtime taken from the front-end runs' home.** `crates/nova-cli/tests/selfhost.rs`: `runtime = Some(runtime_in(&home));` becomes `runtime = Some(runtime_in(&home_ll));`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its report: `the bootstrap does not hold: 1 of 4 comparisons failed`.
  67. **H4: The runtime taken before S1's build, so S1 links it too.** `crates/nova-cli/tests/selfhost.rs`: `let mut runtime: Option<PathBuf> = None;` becomes `let mut runtime: Option<PathBuf> = Some(runtime_in(&home));`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its report: `S2 and S3 linked …/target/debug/libnova_runtime.a, not the runtime S0 carries, unpacked under /tmp/nova-selfhost-bootstrap/home/runtime`.
  68. **H5: The runtime taken again after every stage.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
              if runtime.is_none() {
                  runtime = Some(runtime_in(&home));
              }
      ```

      with

      ```
              runtime = Some(runtime_in(&home));
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: passes, by design: equivalent: only S1's build writes into home (N reads no NOVA_HOME), so each call finds the same file.
  69. **R1: `runtime_in` looks in the wrong directory under home.** `crates/nova-cli/tests/selfhost.rs`: `let unpacked = home.join("runtime");` (the first of 2) becomes `let unpacked = home.join("runtimes");`.
      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its report: `the bootstrap does not hold: 1 of 4 comparisons failed`.
  70. **R2: `runtime_in` ignores home (the golden walk's directory always).** `crates/nova-cli/tests/selfhost.rs`: replace (the first of 2)

      ```
      let unpacked = home.join("runtime");
      ```

      with

      ```
      let unpacked = std::env::temp_dir().join("nova-selfhost-home").join("runtime");
          let _ = home;
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`, then `TMPDIR="$P/exec56/gate-tmp" C $GATE --nocapture`, that directory removed and made empty first.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its reports: `C $GATE`, `S2 and S3 linked /tmp/nova-selfhost-home/runtime/0.4.0-be7e8b48/libnova_runtime.a, not the runtime S0 carries, unpacked under /tmp/nova-selfhost-bootstrap/home/runtime`; with `TMPDIR`, `the bootstrap does not hold: 1 of 4 comparisons failed`.
  71. **R3: `runtime_in` ignores NOVA_RUNTIME_LIB.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
          if let Some(lib) = std::env::var_os("NOVA_RUNTIME_LIB") {
              return PathBuf::from(lib);
          }
          let name = "libnova_runtime.a";
      ```

      with

      ```
          let name = "libnova_runtime.a";
      ```

      - Run `SH --exact $GUARD`, then `C $GATE --nocapture`, then `NOVA_RUNTIME_LIB="$P/exec56/rt/libnova_runtime.a" C $GATE --nocapture`.
      - Expected: fails: `the_bootstrap_holds_s1_s2_s3`.
        Its report: `the bootstrap does not hold: 1 of 4 comparisons failed`.
  72. **N1: No clang passes under NOVA_REQUIRE_CLANG=1.** `crates/nova-cli/tests/selfhost.rs`: replace (the third of 3)

      ```
                  !required,
                  "NOVA_REQUIRE_CLANG=1, but
      ```

      with

      ```
                  true,
                  "NOVA_REQUIRE_CLANG=1, but
      ```

      - Run `SH --exact $GUARD`, then the no-clang check, required, then the no-clang check, not required.
      - Expected: fails: the required no-clang check passes, where it must fail.
  73. **N2: The requirement read the other way.** `crates/nova-cli/tests/selfhost.rs`: replace (the third of 3)

      ```
                  !required,
                  "NOVA_REQUIRE_CLANG=1, but
      ```

      with

      ```
                  required,
                  "NOVA_REQUIRE_CLANG=1, but
      ```

      - Run `SH --exact $GUARD`, then the no-clang check, required, then the no-clang check, not required.
      - Expected: fails: the required no-clang check passes, where it must fail; the no-clang check that is not required fails, where it must pass.
  74. **N3: `not run (no clang)` worded otherwise.** `crates/nova-cli/tests/selfhost.rs`: `eprintln!("bootstrap: not run (no clang)");` becomes `eprintln!("bootstrap: skipped");`.
      - Run `SH --exact $GUARD`, then the no-clang check, not required.
      - Expected: passes, by design: output only, as the other clang tests' lines.
- [ ] **Step 9: The review's inputs.** The full run's figures, the new
  tests by binary and `selfhost.rs`'s apart, the golden suite's line and
  time, Step 4's diffs, the CI steps' names, the MSRV check, `selfhost.rs`'s
  times, the gate's reports and times as the step runs it, and the
  mutants' outcome, with each gap, go in the ledger.
- [ ] **Step 10: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec
    and its amendment, ADR 0060 and this plan, and is told to hunt for:
    - a chain that is not the literal one: a stage built by the wrong
      builder or out of order, another basename, another runtime on one
      stage, a `clang` other than the one probed (Review Focus 1);
    - a comparison skipped, loosened, made against the wrong stage, or
      not reported in order (Review Focus 2);
    - a report that misplaces a mismatch: the `.ll`'s line, `define` or
      block, the fallback place, a size or offset, a stage named wrong,
      the golden test's words changed (Review Focus 3);
    - a front-end run that can pass without leaving the `.ll`, or a
      build that can pass without `built OUT` (Review Focus 4);
    - a way for CI to skip the gate unseen: a name in either step that
      runs no test, a step outside the Selfhost job or without
      `--ignored`, `NOVA_REQUIRE_CLANG` or the `if:` missing, the step's
      report or its `cc` and `ld` lines not shown, the test's own skip
      without `clang`, and whether `the_ci_step_names_every_clang_test`
      reads the steps as CI does, a `run: |` block included (Review
      Focus 5);
    - the size and digest claimed as more than the tree's and the
      runtime's, and the runtime check of decision 22 (Review Focus 6;
      decision 15);
    - CI's time against the lines (Review Focus 7; decision 21);
    - a commit between Task 1 and Task 3 that builds wrong code or fails
      the suite, and code a task adds that its tests do not reach
      (decision 12);
    - the records: ADR 0060, `selfhost/README.md`, the CHANGELOG, the
      dated notes, the spec's amendment, every count (the 23 and 74
      mutants, 53 caught and 21 accepted; 17 `.keys()` lines; 74 tests in
      `selfhost.rs`, 3 ignored) and figure against the tree and CI's run
      (decision 10);
    - each mutant of Step 8 that passes, the accepted ones included, and
      whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock` or to a manifest,
      a test that touches the internet, a temporary name that is not
      fixed or missing from the Global Constraints' list.
  - Each finding is fixed in one commit, `5.6a: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, the Conventions' checks run again in full: `cargo
    fmt`, both clippy commands on the local stable and again on
    `+1.100.0` (`$P/exec56/target-1100` made afresh after the disk
    check, `Task 4 Step 10, before the Rust 1.100 clippy`, shows at
    least 6,656 MiB (6.5G), before the first of the two commands;
    removed after), the mimicked Windows clippy, and both `nova fmt
    --check` lines. Then Steps 1 and 2, `SH` and `C $GATE --nocapture`.
  - **Then every mutant again, all 74,** whichever files the fixes
    changed: all mutant results come from one tip. Before the re-run,
    move Step 8's results aside to a fixed name, `rm -rf
    "${P:?}/exec56/mut-before-fixes" && mv "$P/exec56/mut"
    "$P/exec56/mut-before-fixes"` (the runner skips a mutant whose
    result file exists); then run Step 8 again as it is written: its
    variables, `--check`, disk checks, baseline, ten batches and return
    to plain. Expected: 74, 53 caught and 21 accepted, each as Step 8
    gives it. A mutant whose result changes from Step 8's is a finding
    for the ledger, with the fix that changed it or the reason it
    stands.
- [ ] **Step 11: The PR.**
  - **Stop here, and have the main session ask the user** whether to push
    the branch and open the PR (Global Constraints). No push of this
    branch has been allowed: the user's answer of 2026-11-22 covered only
    the prototype's `…-56-ci` and draft Sakeerin/nova#148. Nothing is
    pushed, and no PR opened, before their word; record their answer in
    the ledger.
  - Before asking, check that every commit the PR would carry ends with
    the two trailer lines of the Conventions: `git log --format=%B
    origin/main..HEAD`, its commits `45a06d1d` (the spec's), the plan's,
    each task's and the review's fixes.
  - On the user's word: push, `git push -u origin
    claude/loving-archimedes-frtf5q-56`, then open the PR to `main` with
    the repository's template headings. The body names no local path, no
    person and no model ID. It ends with this line, a blank line, and the
    session's address, exactly:

    ```
    🤖 Generated with [Claude Code](https://claude.com/claude-code)

    https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
    ```

  - It holds:
    - its commits: first `45a06d1d`, the spec's; then the plan's, with
      the spec's amendment (decision 20); then each task's, and the
      review's fixes;
    - what 5.6a adds: the bootstrap gate, `the_bootstrap_holds_s1_s2_s3`,
      in a new step of the Selfhost job's Linux leg; the comparisons and
      their reports, and the main-step test that holds them; the CI step
      test holding both clang steps; ADR 0060, `selfhost/README.md` and
      the notes;
    - what a user sees change: nothing; no S0 or N source, std, manifest
      or lockfile changed (Step 4), and the golden suite is 147 as
      before;
    - the departures, marked **(differs)** here: decision 12 (the cut:
      §9's item 1 in two tasks, the gate green from its tests commit,
      the measurements no task) and decision 15 (the facts that proved
      wrong: the `.keys()` count, the debug runtime's time, §8's local
      and CI projections, S0's front end at `opt-level = 1`, §7's list
      and the cases and checks added for the mutants, the binaries' size
      on CI), each recorded in the spec's amendment (its items 1 to 7 and
      9);
    - the figures: the Test job's suite `2837 1 10` here and the counts
      expected on CI (decision 21), `selfhost.rs`'s 74 (3 ignored), the
      golden suite's `147 passed, 9 excluded`, `selfhost.rs`'s times here
      and CI's from run 45003631229 and `main`'s 44962982900, the gate's
      time here and on CI, its sizes here and on CI and why they differ
      (CI's recorded, not expected: decision 21), Rust 1.78 clean,
      `Cargo.lock` unchanged;
    - what CI ran before this PR: run 45003631229 ran the prototype's
      `a453c61e`, so none of the cases and checks added since
      (`b829f8dc`'s three cases, `e84a41be`'s checks of the CI step
      test, `5c3d0584`'s runtime check) has run on CI, on any system;
      they ran on this machine only, on Linux, and the PR's run is their
      first on CI;
    - the user's decisions and this plan's, numbered as here, with
      decisions 12 and 15 marked as differing;
    - the mutants: 74, 53 caught and 21 accepted with their reasons, the
      cases and checks added for them (decisions 17 and 22), and the
      review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems; on the
    Selfhost job, `selfhost.rs` `71 passed; 0 failed; 3 ignored` on
    Linux, `69`, `3` on macOS and `65`, `3` on Windows,
    `the_ci_step_names_every_clang_test` and
    `the_bootstrap_reports_name_the_stage` passing on each; then, on Linux
    alone, the clang install, the golden step `2 passed; 0 failed; 0
    ignored; 0 measured; 72 filtered out`, the bootstrap step printing
    `cc`'s and `ld`'s versions and `bootstrap: S1 == S2 == S3, …`, its
    runtime line under the gate's `home/runtime/` (decision 22), `1
    passed; 0 failed; 0 ignored; 0 measured; 73 filtered out` (a `0
    passed` there means the step's name runs no test, which the main
    step's test should have stopped: stop and report), then
    `the_selfhost_tests_pass` on the debug runtime; on macOS and Windows
    the clang steps skipped; on the Test job's `ubuntu-24.04` leg,
    `cargo test`'s log holds `test release_golden_suite ... ok` and no
    `tests/selfhost.rs`; the Test job's main step, `cargo test --locked
    --workspace --all-features --no-fail-fast`, its `test result:` lines
    summed, `2838 passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9` on
    macOS and `2815, 0, 9` on Windows (decision 21); the advisory
    `-- --ignored` step after it is not counted, and what it shows is
    recorded as it is. Record each system's time for `selfhost.rs`, the
    golden step's and the bootstrap step's, the gate's sizes and runtime,
    `cc`'s and `ld`'s versions, the jobs' totals and the run's address in
    the ledger, and compare each with run 45003631229's in the PR's body.
    That run's 9,539,736 bytes and `0.4.0-5549317a` are recorded, not
    expected: CI builds its runtime with the current stable, so another
    size or runtime directory is no failure, while S1 == S2 == S3 is the
    gate (decision 21).
  - **If the gate fails on CI and not here,** record its report, which
    names the stage or the comparison, stop and ask the user: a runner
    whose `cc`, `ld` or `clang` changed is the likeliest (ADR 0060 §2).
    **If the step takes more than 75 s** (spec §8's margin), **or
    `selfhost.rs` more than 110 s on Windows, 100 s on macOS or 80 s on
    Linux,** ask the user (decision 21). Never lower a count.
- [ ] **Step 12: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 11's.
