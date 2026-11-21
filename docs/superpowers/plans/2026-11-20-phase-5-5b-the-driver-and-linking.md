# Phase 5.5b, "the driver and linking: N's `build` and `check`, and the golden suite built by S1", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** N gains a driver. `selfhost build FILE [-o OUT] [--release]
--std DIR` loads, checks and lowers FILE natively, writes S0's `.ll` to
`<OUT>.nova.ll`, runs `clang -O2 -c` and `cc` on it with the runtime, and
prints `built OUT`; `selfhost check FILE --std DIR` prints each
diagnostic's header and loci, `error[CODE]: MESSAGE` and `  ┌─
PATH:LINE:COL`, then `ok: FILE` or `Error: found N error(s)`. Every
failure is S0's words in anyhow's layout, exit 1; a command line N does
not take prints N's usage line, exit 2; on Windows `build` refuses. The
golden tables move from `run_tests.rs` into `tests/golden/shared.rs`,
which `run_tests.rs` and `selfhost.rs` include. A new step of the
Selfhost job's Linux leg installs `clang-18` and runs two ignored tests:
`s1_builds_the_golden_suite`, where S1's own `build` builds the 147
golden programs, each run with the tables' stdin and variables and held
equal to S0 `--release`'s binary byte for byte; and
`n_build_answers_as_s0_does_with_clang`, which compares `build` with S0
where `clang` runs. The main step gains `n_build_and_check_answer_as_s0_does`
(39 probes, no `clang`, all three systems), an extraction test, and
`the_ci_step_names_every_clang_test`, which holds the new step's names
to the ignored tests; N gains eleven `@test`s. Nothing a user of S0 sees changes: the diff touches no
S0 source, std, manifest or lockfile, and the golden suite stays 147.
ADR 0059 records the commands, the steps, the harness, the figures and
CI's.

**Architecture:**
- **The tables, S1's golden walk, N's `build` on its success path, CI**
  (Task 1): `crates/nova-cli/tests/golden/shared.rs` (new: the
  `RELEASE_GOLDEN_*` tables and limits, `PROCESS_TEST_*`,
  `quoted_stderr`, `run_within_limit`, `run_release_binary`,
  `golden_dir`, `golden_programs`, `run_golden`); `run_tests.rs` including
  it; `selfhost.rs`'s `golden_runtime`, `same_binaries`,
  `s1_builds_the_golden_suite` and `the_ci_step_names_every_clang_test`;
  `selfhost/src/driver.nova` (new: the
  command line, `build_command`, `compile_root`, `intermediate`,
  `compile_and_link`, `run_tool`, `debug_quoted`, `tool_text`,
  `find_runtime`, `failure_text` and `fail`, and std's loading moved
  from `main.nova`), `loader.nova`'s `file_stem_or` and public
  `file_name`, `main.nova`'s `build` dispatch; `ci.yml`'s two steps.
- **`build`'s refusals and errors** (Task 2): the probe harness in
  `selfhost.rs` (`PROBE_INPUTS`, `Probe`, `extracted_stderr`, `PROBES`'
  `build` cases, `probe_dir`, `probe_side`, `probe_difference`,
  `run_probes`), `n_build_and_check_answer_as_s0_does`,
  `the_extraction_keeps_the_headers_the_loci_and_the_error`, the clang
  probe test with `CLANG_PROBES` and `CC_ARGS` (`#[cfg(unix)]`, as its
  use); `selfhost/src/driver_tests.nova` (new), imported by `main.nova`;
  the CI step naming the clang test; in `driver.nova`, the Windows
  refusal (`windows_shape`, `on_windows`), the diagnostics `build` prints
  (`SourceFile`, `diagnostic_lines`, `loci`, `line_and_column`,
  `with_std_files`, `print_diagnostics`) and `status_cause`.
- **`check`** (Task 3): `PROBES`' `check` cases, their inputs, the copy
  of `examples/05-geometry`, `N_CHECK_USAGE`; `check`'s command line in
  N's `@test`; `check_args`, `command_args`, `check_command` and
  `compile_root` lowering only a program that runs, and `main.nova`'s
  `check` dispatch.
- **Records** (Task 4): ADR 0059 (new); `CHANGELOG.md`; dated notes in
  `docs/phase-5-plan.md`, `nova-spec/00-MASTER-SPEC.md`,
  `nova-spec/14-CODEGEN.md` and `nova-spec/40-TOOLING.md`.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` and every `Cargo.toml` stay as they are**
  (spec §12).
- N is built by `nova build` through Cranelift, as before; N's `build`
  runs `clang` (CI's `clang-18`, 18.1.3) and `cc`. Node 22 for the WASM
  tests, as before; nothing 5.5b adds needs it.

**Spec:**
`docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`,
approved by the user on 2026-11-19 (`17ecca86`), with its dated amendment
of 2026-11-20, which this plan's commit appends (decision 18).
- Read its header, §1 to §16, and the amendment, before Task 1.
- The user's decisions are decisions 1 to 8 here, and the answer that
  allowed the prototype's push to CI decision 9.

**Base:** `17ecca86` ("docs: the Phase 5.5b spec is approved"), `main`
`e71446df` (5.5a's merge) and the spec's commit, whose full suite gives
`2837 1 10` here (the IPv6 test the only failure) and `selfhost.rs` 67
apart. The branch is `claude/loving-archimedes-frtf5q-55b`, from
`17ecca86`, then this plan's commit (decision 18).

**Checked on 2026-11-20, before execution:**
- **A prototype built 5.5b whole** on a local branch from `17ecca86`:
  part 1, `929191a4` (the tables shared), `832314bc`
  (`s1_builds_the_golden_suite`, red on N's usage line), `be9ce33a` (N's
  `build`, its success path), `09f4257b` (the CI steps); part 2,
  `c0af53bc` (both probe tests, red), `95f077a5` (N's refusals and
  errors, the Windows refusal, `check`, N's `@test`s), `973765a8` (two
  clang cases the mutants asked for), `77d247c7` (the harness's stdout
  check met by `fs_already_exists`); part 3, `ecfcd0f8` (`CC_ARGS` made
  `#[cfg(unix)]` after CI's Windows clippy failed), `fe0e5f10` (ADR 0059,
  S1 building N, the notes), `40e10c42` (ADR 0059 §14's CI figures).
  Then four commits for this plan (decision 16): `9304d751`, the clang
  test's case "a runtime two directories up", which mutant 37 asked for;
  and, after the plan's first fact-check, `e97c759a` (the clang test's
  "N of M" counted from the cases that ran), `bbc44b68`
  (`the_ci_step_names_every_clang_test`, which catches mutant 41) and
  `82d22393` (ADR 0059, the CHANGELOG and the notes naming both). The
  four commits the ADR names (`17ecca86`, `09f4257b`, `77d247c7`,
  `fe0e5f10`) are each on the pushed
  `claude/loving-archimedes-frtf5q-55b-ci`.
- **CI, measured from the prototype** (decision 9), on
  `claude/loving-archimedes-frtf5q-55b-ci` with draft Sakeerin/nova#146,
  closed unmerged; the Selfhost job's `selfhost.rs`, S0 at `opt-level =
  1` and N on the release runtime, `finished in`:
  - run 44672859855 (head `77d247c7`): Clippy (`windows-latest`) failed,
    `constant CC_ARGS is never used` at `selfhost.rs:7180` under `-D
    warnings`; every other job green; `selfhost.rs` 56.35 s Linux (69
    passed, 2 ignored), 71.15 s macOS (67, 2), 63.56 s Windows (63, 2);
  - run 44674948875 (head `fe0e5f10`, `ecfcd0f8`'s fix in it): 19 of 19
    jobs green; `selfhost.rs` 56.75 s on Linux, 86.12 s on macOS and 70.30
    s on Windows (69, 67 and 63 passed, 2 ignored each), against the lines
    of 80, 100 and 110 s (margins 23.25, 13.88 and 39.70 s); the clang
    install 10.7 s and the new step 30.3 s (`2 passed`, 30.21 s) on Linux;
    the Selfhost jobs 3m26s, 4m20s and 3m55s. The Test job's totals were
    not extracted (only each log's tail could be read);
  - `main`'s run at 5.5a's merge, `e71446df`, run 44479648345: green;
    `selfhost.rs` 54.92 s on Linux, 67.47 s on macOS and 66.85 s on
    Windows (margins 25.1, 32.5 and 43.2 s).
- **The split** (decision 11 **(differs)**): three tasks test first and
  the records. `split.py` makes the local branch `split/5-5b` from
  `17ecca86`, one commit a patch: Task 1 `bbacd0c5`,
  `f806ffa6`; Task 2 `552ad9a0`, `d2daf5aa`; Task 3
  `ba94ef06`, `475d63ce`; Task 4 `40111dbf`. Its last tree
  is the prototype's `82d22393` (`git diff 82d22393 split/5-5b` prints
  nothing), and every patch applies with plain `git apply`, in order, on
  `17ecca86`, with no output, each applied tree its commit's; no added
  line holds trailing whitespace, a CR or a backslash-u escape. **These
  hashes are references, not commits the executor makes:** the executor
  applies each task's patches and makes one commit per task
  (Conventions), with hashes of its own; each task's commit has the tree
  of that task's last split commit here.
- **The replay** (`replay.sh`, under the process limit, without
  `NOVA_EMBED_RUNTIME`, as the Test job runs), each step as written here,
  on the split's commits: each tests commit failed as its Step 2
  records, and each code commit passed the whole suite in three runs
  (`S`: `2837 1 10`, the IPv6 test the one failure) and `selfhost.rs`
  apart (`SH`: 68 passed and 1 ignored at Task 1, 70 and 2 at Tasks 2 and
  3), `cargo fmt`, both clippy commands on the local stable (1.97) and on
  `+1.100.0` (CI's stable), `selfhost.rs`'s again with the diff's
  `#[cfg(unix)]` items turned off (Windows mimicked), and `nova fmt
  --check` with CI's arguments and the earlier plans' superset; the
  ignored tests as CI's step names them; N's `nova test` (119 passed
  from Task 2 on). The split was made three times: from `40e10c42`, from
  `9304d751`, and from `82d22393` after the fact-check, which changes
  `selfhost.rs` from Task 1 on and the records. Every figure here is the
  third split's, all re-run on it, but S1 building N (decision 17),
  measured at the first split's last commit, whose `selfhost/` and S0
  sources are this one's. The Windows mimic was also run as the
  Conventions write it on uncommitted trees, Task 2's and Task 3's
  patches applied on the previous code commit: the file came back byte
  for byte, and `git status --short` listed the task's files.
- **At the last commit,** `40111dbf` (Task 4's), whose tree is
  `475d63ce`'s and the records:
  - the whole suite in three runs: `2837 1 10` (as Task 5, Step 1 runs
    it); the test list 2,848 as at the base; `selfhost.rs` 72 listed, the
    base's 67 and five added (two ignored);
  - CI's Linux leg, `--all-features`, the golden suite required,
    `RUST_BACKTRACE=1`, in three parts: `1300 1 8`, `2 0 0` and `1535 0
    2` in 119, 1 and 192 s, the golden suite inside the third;
  - the golden suite: `release golden: 147 passed, 9 excluded, Ubuntu
    clang version 18.1.3 (1ubuntu1)`, in 21 s;
  - no S0 source changed: `git diff 17ecca86 HEAD` over the whole tree
    but `crates/nova-cli/tests`, `selfhost/`, `docs/`, `nova-spec/`, the
    CHANGELOG and `.github` prints nothing (decision 12 **(differs)**);
  - Rust 1.78: clean, `Finished` in 28 s; `Cargo.lock` unchanged;
  - `selfhost.rs` without the release runtime: 70 passed, 2 ignored, in
    175.58 s, and with `RUST_BACKTRACE=1` in 186.67 s;
    as the Selfhost job runs it (`NOVA_EMBED_RUNTIME=1`,
    `CARGO_PROFILE_DEV_OPT_LEVEL=1`), three times: 52.87, 48.73 and 57.36
    s (user CPU 170 to 204 s); the new step's command twice, 33.51 and
    34.72 s (`s1 golden: 147 passed, 9 excluded`); N's `nova test` 119
    on both runtimes; the job's last step 1.71 s;
  - S1 building N (ADR 0059 §13's method, decision 17), at the first
    split's last commit: 10.02 and 10.21 s, peak 203.5 MB; S0's
    `--release` 10.01 and 9.73 s; S2 equal to the `--release` S1,
    9,354,680 bytes, and S3 to S2; the `.ll`s equal, 12,759,478 bytes.
- **The mutants** (decision 16): 64 breakages of what 5.5b adds, all run
  one at a time, in one run, at `40111dbf` with
  `NOVA_EMBED_RUNTIME=1` (Task 5, Step 8): part 2's 23; part 1's checks
  (the tables, the walk's checks, the usage line, `clang`'s and `cc`'s
  lines, the runtime's search, the stem, std's loading, and the CI step
  against the tests it names: a name misspelt in `ci.yml` (41), a name
  dropped from it (63), and the clang test's `#[ignore]` taken off in
  `selfhost.rs` (64); 20 in all); and §3.4's words part 2 did not mutate (21). 60
  are caught; 4 are accepted, each with its reason: 3 equivalent and 1
  unreachable on Linux. The runs add up to 1,534 s, 26 minutes.
- **What the checks left standing,** each recorded or handed on:
  - macOS and Windows are reasoned from the code here and were run on CI
    from the prototype's `fe0e5f10`. This split's last tree differs from
    it by ADR 0059 §14's figures (`40e10c42`), the clang test's case and
    count (`9304d751`, `e97c759a`, inside a test only Linux runs), the
    records (`82d22393`), and `the_ci_step_names_every_clang_test`
    (`bbc44b68`), which runs in the main step on all three systems and
    has not yet run on CI's macOS or Windows (it reads both files as
    text, CRLF line endings allowed). The PR's runs are the check;
  - N's Windows refusal is held on Linux by an `@test` of its shapes, its
    words only on CI's Windows leg (mutant 45);
  - the locus rule's "earliest primary against an earlier secondary" by
    N's `@test` alone (no source input reaches it: mutants 16, 17);
  - the accepted mutants (decision 16).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` under `RUSTUP_TOOLCHAIN=1.78.0`
  with `RUSTFLAGS=-D warnings`. No `std::sync::LazyLock` (1.80), no
  `Option::is_none_or` (1.82), no `std::path::absolute` (1.79). 5.5b's Rust
  is all test code (`selfhost.rs`, `run_tests.rs` and `golden/shared.rs`,
  part of it moved from `run_tests.rs`). These are every std API and
  language form its added lines use, new and moved, that is newer than
  1.10, found by listing every method call, path and macro in the
  diff's added Rust lines (`git diff 17ecca86 <tip> -- '*.rs'`, the `+`
  lines) and searching them for each language form: `?` on a `Result`
  (1.13) and on an `Option` (1.22); `Option::unwrap_or_default` and
  `Result::unwrap_or_default` (1.16); field-init shorthand (`Probe {
  name, … }`, 1.17); `'static` elided in a `const`'s type (`const
  PROBE_INPUTS: &[(&str, &str)]`, 1.17); `Child::try_wait` (1.18,
  moved); `Command::envs` (1.19, moved); `break` with a value from
  `loop` (`let status = loop { … break status … }`, 1.19, moved);
  `slice::sort_unstable` (1.20); `Ord::min` on integers (1.21); a
  `&[]` promoted to `'static` inside a `const fn` (1.21); default
  binding modes (`for (name, _) in RELEASE_GOLDEN_EXCLUDED`, `|(_, c)|
  *c`, 1.26); `impl Trait` in argument position (1.26); `std::fs::read`,
  `std::fs::read_to_string` and `std::fs::write` (1.26);
  `Path::ancestors` and `NonZeroUsize::get` (1.28); `str::trim_start`,
  `str::trim_end`, `str::trim_end_matches` and `Iterator::find_map`
  (1.30); `const fn` (1.31); tool lints
  (`#[allow(clippy::too_many_arguments)]`, 1.31); `Duration::from_secs` in a `const` (1.32, moved);
  `Iterator::copied` (1.36, moved and new); `str::strip_prefix` (1.45);
  `Result::as_deref` (1.47); `bool::then` (1.50); a `[char; N]` pattern
  (`replace([' ', ',', …], "-")`, 1.51); an array iterated by value
  (`for (side, run) in [("S0", s0), ("N", n)]`, 1.53); format strings'
  captured identifiers (`{name}`, 1.58);
  `std::thread::available_parallelism` (1.59); `bool::then_some`
  (1.62); `std::thread::scope` (1.63); `let … else` (1.65); and
  `Result::is_err_and` (1.70). Each is at most 1.78; the rest
  (`Iterator::ne`, `BTreeSet`, `AtomicUsize::fetch_add`, `Mutex::into_inner`,
  `PermissionsExt`, `Path::strip_prefix`, `Duration`, `Instant` and
  older) predate 1.10. The MSRV job checks no test target, and Cargo
  1.78 cannot check one here (the dev-dependency `assert_cmd` 2.2.1
  needs the 2024 edition), so this list is the check of the tests' Rust
  (Task 5, Step 6). It was also checked once by hand: `rustc` 1.78
  type-checks `selfhost.rs` with `golden/shared.rs` at the last commit
  (`--test --emit=metadata`, against Rust 1.78's own build of the
  workspace's libraries) with no error and no warning in 5.5b's lines;
  its only errors are in older lines (`selfhost/fuzz.rs`'s `pick` calls,
  and `selfhost.rs`'s `twice` slice of temporaries), which 1.78 rejects
  and later compilers accept. `run_tests.rs`'s own added lines (an
  import, three calls) were not compiled on 1.78, since it uses
  `assert_cmd`.
- **No new crate; `Cargo.lock` and every `Cargo.toml` stay as they are**
  (spec §12). Binaries are compared byte for byte, no hash crate. If a
  step seems to need a crate or a manifest change, stop that task there
  and ask the user; do not add the crate or change a manifest first.
- **No `unwrap()` outside `#[cfg(test)]` and test files.** 5.5b's Rust is
  test code alone. N's `unwrap()`s are on values its own code has checked
  (`tool_text` re-reads bytes it made valid UTF-8; `build_command` unwraps
  the module `compile_root` lowered for `build`).
- **No `unsafe`.** 5.5b adds none.
- **Tests never touch the internet** and bind no port (spec §9). The
  probe tests run `nova` and the N they build, and, in the two ignored
  tests, `clang` and `cc` from `PATH` or `NOVA_CLANG` and `CC`; `clang` is
  installed by CI, never by a test. Without `clang` the two ignored tests
  print `not run (no clang)` and pass, and fail instead under
  `NOVA_REQUIRE_CLANG=1`.
- **Temporary names are fixed** (spec §9): under `std::env::temp_dir()`,
  through `fresh_dir`, which removes and makes each afresh. 5.5b adds
  `nova-selfhost-golden` (`s1_builds_the_golden_suite`'s builds, Task 1),
  `nova-selfhost-probe` (the main probe test's cases),
  `nova-selfhost-probe-clang` (the clang probe test's cases) and
  `nova-selfhost-alone` (N copied alone, and two directories down, in the
  clang probe test), all three in Task 2. These are every fixed temporary
  name the diff adds (`grep -n 'fresh_dir(\|temp_dir'` over its added
  lines); `golden_runtime` reads `nova-selfhost-home`, `build_selfhost`'s
  since 5.3a, and writes nothing there. N's `@test`s write nothing. A test
  that needs `$NOVA_HOME` sets a fresh one: the probe tests set
  `<their directory>/home`.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and the golden suite's directory is one of them.
  The measurement of S1 building N (Task 5, Step 7) writes under
  `$P/exec55b/s1n/`, not under `/tmp`.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets. Run
  the suite here with `NOVA_REQUIRE_NODE=1`. No test of 5.5b needs Node.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** which
  CI's Linux leg sets with `NOVA_CLANG=clang-18`; elsewhere it prints
  `release golden: not run (set NOVA_REQUIRE_CLANG=1 to run it)` and
  passes. Its tables move (Task 1) and stay as they were, so it stays at
  147 passed, 9 excluded.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary, which only a build
  of the workspace writes. `nova build` of `selfhost/` links against that
  runtime, or, in a `nova` built with `NOVA_EMBED_RUNTIME=1`, against the
  release runtime embedded in it, which `build_selfhost` unpacks into
  `nova-selfhost-home/runtime/`, and which the ignored tests then find
  (`golden_runtime`); `NOVA_RUNTIME_LIB` overrides both. Every run step
  starts with `cargo build --locked --workspace` (Conventions).
- **`selfhost.rs` is not in `cargo test --workspace`:** its target says
  `test = false` (5.3a). `S` runs the whole suite as the Test job does,
  `SH` runs `selfhost.rs` as the Selfhost job's main step does (the two
  ignored tests skipped), and `C` (Conventions) runs the ignored tests as
  the new step does; every figure here counts them apart.
- **`NOVA_EMBED_RUNTIME` and `CARGO_PROFILE_DEV_OPT_LEVEL` are set for one
  command only, or for a whole step that says so** (Task 5, Steps 7 and
  8): a change of either makes cargo rebuild, and every other test that
  links a program then runs on that runtime or at that level. Each such
  step ends with `cargo build --locked --workspace` without them. Never
  build every test target at `opt-level = 1` (`cargo build --tests` or the
  whole suite): only `selfhost.rs`'s, as the Selfhost job does.
- **`RUST_BACKTRACE` is `0` in every shell here** (Conventions), and `1`
  for one command only where a step says so (Task 5, Steps 2 and 7): CI's
  jobs export `RUST_BACKTRACE=1` (5.5a's lesson, ADR 0058 §13).
- **The process limit.** The probes start two processes a case, and the
  suite thousands; every test, mutant and suite runs under the limit of
  the Conventions, so that a mistake that forks without end fails to fork
  rather than filling the machine.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other. Never build in another worktree, and never run two
  suites, or a suite and `SH`, at once: the tests' fixed directories are
  shared.
- **Keep `df -h /` above 6G, and check it before each build, not
  after.** Older duplicate test executables in `target/debug/deps` may be
  removed between commands, never while one runs, and so may
  `target/debug/incremental`; never `cargo clean` the main target.
  - **Before each change of opt-level or runtime** (Task 5, Steps 7 and
    8, and back): `df -h /` must show at least 7.0G, or first remove the
    other level's or runtime's `nova-*` and `selfhost-*` executables in
    `target/debug/deps` (each 100 to 230 MB; the next build at that
    level makes them again). A build at another `opt-level` keeps both
    sets: in the replay the `opt-level = 1` build took 0.6G and the
    embedded build at `opt-level = 0` 0.3G; the replay read 6.5G each
    time and removed the other set first, reaching 7.0G each time (7,159
    and 7,095 MiB by `df -m /`).
  - **Before the Rust 1.100 clippy:** at least 6.5G, checked once
    before the first of its two commands, which run back to back; its
    target directory of its own takes 405M, made by the first, which the
    second reuses (about 1 MB more), so the second starts near 6.1G,
    above the floor (the replay read 6.1G with it). It is made for each
    code task's checks and removed right after them.
  - **Before any other build:** at least 6.3G. A build at the same
    level and runtime replaces its executables in place, but cargo
    keeps two copies of a few executables (`run_tests`, `nova`,
    `registry`) built for different command lines: the replay's first
    test build took 0.7G.
  - Each run of the whole suite leaves about 33 MB under
    `/tmp/nova-todo-*` (CI's Linux leg added 33 MB in 9 directories),
    which is never cleaned. The Rust 1.78 check (Task 5) builds under
    `/dev/shm`, removed after. The replay began with 7.4G free.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code in
  braces: they decode the escape into the character. None of 5.5b's
  patches or scripts holds one. Apply the patches with `patch_block`, from
  the committed plan.
- **No patch adds a CR, a NUL or trailing whitespace.** The `\r\n` probe
  input and the NUL in `process_env`'s source are escapes in literals.
- **Nothing writes into the repository but the patches.** The probes and
  builds write under the system's temporary directory; `nova test` writes
  nothing into the package; `git status --short` is empty after every run.
- **Push and open the pull request on the user's word only.** No push
  of this branch is allowed yet. The user's answer of 2026-11-20, "อนุญาต
  push -55b-ci + draft PR (แนะนำ)", was for the prototype to
  `claude/loving-archimedes-frtf5q-55b-ci` with draft Sakeerin/nova#146,
  measurement only, closed unmerged (decision 9); it does not cover this
  branch. Task 5, Step 11 stops before any push, and the main session
  asks the user; the branch is pushed and the PR opened only after their
  word.
- **Merge by rebase, on the user's word only.** Never enable auto-merge.
  Nothing is published and no tag is made in 5.5b.
- **This machine is Linux.** macOS and Windows results come from CI; what
  differs there is reasoned from the code (Review Focus 1, 2).

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Every shell exports it, `export P=…` with the
  directory's path, before a command that names `$P` (the Runs block
  below starts with it). Exported, not merely set: Task 5's scripts read
  it from the environment, each run as a process of its own.
- **The executor's directory:** `$P/exec55b/` (`mkdir -p "$P/exec55b"`
  once, in Task 0; the Runs block's `mkdir -p "$NOVA_HOME"` makes it
  too). Every script this plan writes (Task 5's `names.py`, `tv.py`,
  `s1n.sh`, `mutrun.py` and `mutants.json`) and every output goes there,
  nowhere else: the suite's logs, the extracted patches, the commit
  messages, `$NOVA_HOME`, the Rust 1.100 target and Task 5's outputs. The
  one file outside it is the ledger (below), where every phase's ledger
  is kept. `$P/plan55b/` holds the replay's own scripts and evidence,
  which this plan cites: nothing here writes into it. The scripts are the
  replay's with that one directory written `exec55b` for `plan55b`.
- **The ledger:** `$P/ledger-55b.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/exec55b/msg-<task>.txt`:
  the subject the task names, a blank line, and these two trailer lines,
  exactly:

  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
  ```

  Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/exec55b/msg-<task>.txt"`. A task with a tests
  patch and a code patch makes one commit, after both. Every commit of
  this plan ends with those two lines: the plan's own (Task 0), each
  task's and the review's fixes (Task 5, Step 10). The executor makes
  these commits; the split's commits named in "Checked on" (`bbacd0c5`
  to `40111dbf`) are the replay's references, whose trees each
  task's commit should have. The pull request's
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

  600 tasks was ample here: the replay's suites, builds, golden walks and
  mutants ran under it on 4 CPUs. If a test fails to start a thread, read
  the group's `pids.peak`, raise `pids.max`, and record it.
- **Runs.** The replay ran every command in the group and with these
  variables set. Each shell puts itself under the cap and sets them too,
  with four shorthands, and runs every command from the repository's
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
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/exec55b/home"
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
  ignored tests it names, with `clang-18` required, as the new step runs
  them. Without `NOVA_EMBED_RUNTIME` the ignored tests link the debug
  runtime beside `nova` (`golden_runtime`), as the replay's task steps
  did; their binaries are still S0's byte for byte. `NOVA_HOME` keeps a
  run by hand off `~/.nova`; every test that needs one sets its own.
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
    `SH` (up to 3 s), so that neither run holds a build.
  - **`S` in three runs,** everywhere, as the replay ran it:

    ```sh
    S --lib --bins > "$P/exec55b/s-a.txt" 2>&1                       # the libraries' and binaries' tests
    S --doc > "$P/exec55b/s-b.txt" 2>&1                              # the doc tests
    S --test '[!s]*' --test 's[!e]*' > "$P/exec55b/s-c.txt" 2>&1     # every integration test but selfhost.rs
    cat "$P/exec55b/s-a.txt" "$P/exec55b/s-b.txt" "$P/exec55b/s-c.txt" | grep '^test result' | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
    ```

    At the base and at each code commit they gave `1300 1 8` in 98 to 117
    s (the IPv6 test the failure), `2 0 0` in 1 to 3 s and `1535 0 2` in
    157 to 185 s, `2837 1 10` summed; `grep -c 'Running
    tests/selfhost.rs' "$P/exec55b/s-c.txt"` prints `0`. The globs name every
    integration test but `selfhost`: a `--test` pattern takes a `test =
    false` target, which `--tests` leaves out, and `--tests` would run the
    libraries' tests again. A difference from the sum is recorded in the
    ledger. `S` neither builds nor runs `selfhost.rs`, so a change to that
    file alone leaves `S`'s binaries as they were.
  - Every single command here fits 600 s at twice the replay's time. The
    longest: CI's Linux leg's third part (192
    s), `S`'s third (185 s), `SH` without the release runtime (148 to 187
    s), and the mutants' batches (227 to 273 s).
  - **A child killed at `CHILD_LIMIT`.** The harness kills a child
    process past 120 s (`CHILD_LIMIT`); none came near it here. If a test
    fails with a child killed there (`… did not finish within 120s:
    killed`), run that test alone once, `SH --exact <test>`. If it is
    killed again, stop and report to the main session. Never raise
    `CHILD_LIMIT` without the main session.
  - If a command is killed by its timeout, or the shell restarts, restore
    what it left: after a killed mutant run, `git checkout -- <file>` of
    the mutant's file, and then `git status --short` must print nothing
    before anything goes on (the replay's third batch of mutants was
    killed at 590 s in mutant 34, and restored so).
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
  PLAN=docs/superpowers/plans/2026-11-20-phase-5-5b-the-driver-and-linking.md   # this file, committed
  block() {
    awk -v name="$1" '
      $0 == "<!-- patch " name " -->" { grab = 1; next }
      grab && /^````diff$/ { inside = 1; next }
      inside && /^````$/ { exit }
      inside { print }
    ' "$PLAN" > "$P/exec55b/$1.patch"
  }
  patch_block() {
    block "$1" && git apply --check "$P/exec55b/$1.patch" && git apply "$P/exec55b/$1.patch"
  }
  ```

  `patch_block t1-tests` applies the block named `t1-tests`, and prints
  nothing: no 5.5b patch adds a line with trailing whitespace, a CR or a
  backslash-u escape, so `git apply` warns for none. Task 2's tests patch
  adds 1,095 lines (a block of 1,140) and Task
  4's 912 (a block of 987): each is one
  block all the same. The non-ASCII characters the patches add are `§`
  (every task), `é` and `✓` (Task 1's tests: the moved
  `PROCESS_TEST_VALUE`), `…` (Task 1's code, Task 4), `─` and `┌` (Tasks
  2 to 4: the locus line), `│` (Task 2's tests), Thai letters and marks,
  `世`, `界` and `🎉` (Tasks 2 and 3's tests: the column cases), `¤`
  (Task 3's tests: L0001's input) and `—` (Task 4). A block that does not
  apply means the tree differs from the plan's: stop, and record why in
  the ledger.
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
    minimal -c clippy` once, if `rustup toolchain list` lacks it), after
    `df -h /` shows at least 6.5G, checked once before the first command
    (the second, run right after it, reuses its target directory):
    `CARGO_TARGET_DIR="$P/exec55b/target-1100" cargo +1.100.0 clippy …` with each
    command's arguments, each ending with `Finished` and no warning (22 to
    46 s, then 1 s); then `rm -rf "$P/exec55b/target-1100"`. The local stable
    (1.97 here) and CI's lint differently;
  - **Windows, mimicked:** CI's Windows clippy failed on the prototype
    where a constant was used only under `#[cfg(unix)]` (decision 9), and
    local clippy runs only Linux's items. So `selfhost.rs`'s clippy runs
    again with the diff's `#[cfg(unix)]` items turned off, then the file
    is restored:

    ```sh
    F=crates/nova-cli/tests/selfhost.rs
    cp "$F" "$P/exec55b/selfhost.rs.keep"                   # the task's own file, uncommitted, set aside
    S5=$(grep -n '^fn golden_runtime' "$F" | cut -d: -f1)    # 5.5b's part of the file starts here
    sed -i "$S5,\$ s/#\[cfg(unix)\]/#[cfg(any())]/" "$F"
    diff "$P/exec55b/selfhost.rs.keep" "$F" | grep -c '^>.*cfg(any())'   # the items turned off
    cargo clippy --locked --all-features --test selfhost -- -D warnings
    cp "$P/exec55b/selfhost.rs.keep" "$F"
    cmp "$P/exec55b/selfhost.rs.keep" "$F" && echo restored
    git status --short
    ```

    which prints the count (`0` at Task 1; `3` at Tasks 2, 3 and 4,
    whose `selfhost.rs` is Task 3's: `CC_ARGS`, its use in `probe_dir`,
    and the clang test's making executable the binary its harness
    self-check runs), then `Finished`
    with no warning, then `restored`, then the task's files, uncommitted
    as the checks run before the commit: Task 1's `M` for
    `.github/workflows/ci.yml`, `crates/nova-cli/tests/run_tests.rs`,
    `crates/nova-cli/tests/selfhost.rs`, `selfhost/src/loader.nova` and
    `selfhost/src/main.nova`, and `??` for
    `crates/nova-cli/tests/golden/` and `selfhost/src/driver.nova`;
    Task 2's `M` for `ci.yml`, `selfhost.rs`, `driver.nova` and
    `main.nova`, `??` for `selfhost/src/driver_tests.nova`; Task 3's `M`
    for `selfhost.rs`, `driver.nova`, `driver_tests.nova` and
    `main.nova`; Task 4's the six records of its Files list (`??` for the
    ADR). Never `git checkout` the file here: it would drop the task's
    uncommitted changes;
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

1. **Windows** (N's refusal, the probe test on CI's Windows leg, a
   `cfg(unix)` item): N knows Windows by `current_exe()`'s shape (a drive
   or `\\`), refuses `build` before reading std or FILE, and runs `check`;
   on Windows the main probe test expects N's refusal for each `build`
   case and reads a locus's `\` as `/`; every item used only on Unix is
   itself `#[cfg(unix)]` → N's `@test`
   `windows_is_known_by_a_drive_or_a_double_backslash`, the main probe
   test on CI's Windows leg, the mimicked clippy (Tasks 2, 3) (mutants
   11, 12, 45).
2. **The binary is S0's, byte for byte** (`clang -O2 -c`, the runtime
   looked for after `clang`, `cc -o OUT OBJ RUNTIME -lpthread -ldl -lm`,
   one runtime for both sides, the `.ll` removed on success): →
   `s1_builds_the_golden_suite` and the clang probe test's success,
   `cc-args`, default-name and no-`--release` cases (Tasks 1, 2) (mutants
   1 to 7, 13, 20, 33 to 37, 42).
3. **Every failure in S0's words and layout** (`fail`, `failure_text`,
   each head and cause of §3.4, `status_cause`, `debug_quoted`,
   `tool_text`): → the probes, compared with S0's extracted stderr, and
   N's `@test`s (Tasks 2, 3) (mutants 9, 10, 46 to 62).
4. **`check`'s lines** (the header, the loci by codespan's rule, line and
   column by `\n` and characters, std's `<std/NAME>` read from `--std`,
   the extraction of S0's stderr): → the main probe test's `check` cases,
   `the_extraction_keeps_the_headers_the_loci_and_the_error`, N's
   `@test`s (Tasks 2, 3) (mutants 14 to 19, 22, 23).
5. **The command lines** (each option once, any order, `--`, `-` as a
   FILE, `check` without `-o` or `--release`, N's usage lines, exit 2):
   → the usage and N-alone probes, N's command-line `@test` (Tasks 2, 3)
   (mutants 8, 32, 44).
6. **The harness keeps every program and every check** (the shared
   tables, the walk's floor, S1's `built OUT`, its stdout and status,
   the binaries compared; the probes' leftovers compared; the CI step's
   names) → `release_golden_suite`, the clang test's self-checks,
   `the_ci_step_names_every_clang_test` (Tasks 1, 2) (mutants 21, 24 to
   31, 41, 63, 64).
7. **N's std loading, moved** from `main.nova` to `driver.nova`: every
   earlier `selfhost.rs` test reads it → the resolution and HIR tests
   (Task 1) (mutants 39, 40).
8. **CI's time** (spec §10, §14): the main step's `selfhost.rs` under the
   80, 100 and 110 s lines with the probe test in it; the new step and
   the install recorded → decision 9, Task 5, Steps 7 and 11.

## Decisions

The user's, from the spec's header (2026-11-19):

1. **S1's golden suite runs in a new step of the Selfhost job's
   `ubuntu-24.04` leg** (the user's decision 1, as given: after
   `selfhost.rs`, `clang-18` installed there; S1 built by Cranelift, as
   `selfhost.rs` builds it; its programs on the release runtime; outside
   the 80 s line). As built: `ci.yml`'s "Install clang 18.1.3 (Linux)"
   and "S1 builds the golden suite (Linux)" (Task 1; the clang test's
   name added in Task 2); 30.3 s on CI (decision 9). A main-step test,
   `the_ci_step_names_every_clang_test`, holds the step's names to the
   ignored tests (decision 11).
2. **The intermediate is named as S0 names it** (the user's decision 2,
   as given: `<out>.nova.ll`; no emitter change; 5.6 uses one `-o`
   basename across stages). `intermediate` (Task 1), N's `@test`
   `the_intermediate_is_named_for_the_outputs_file_name_as_s0_names_it`
   (Task 2).
3. **`check` prints code and span** (the user's decision 3, as given:
   `error[CODE]: message` with `path:line:col`, about 30 to 60 lines;
   compared by code and span, ADR 0048 decision 1, not rendered text). As
   built: codespan's own header and locus lines, `  ┌─ PATH:LINE:COL`
   (Tasks 2 and 3), S0's extracted by `extracted_stderr`.
4. **Linux and macOS** (the user's decision 4, as given: N's `build`
   refuses Windows with a message, recorded (differs); the `.ll`
   comparison stays on all three systems). `windows_shape`, `on_windows`
   and `WINDOWS_REFUSAL` (Task 2); ADR 0059 §4.
5. **File mode and `--std`** (the user's decision 5, as given: `selfhost
   build FILE [-o OUT] [--release] --std DIR`, `selfhost check FILE --std
   DIR`; the runtime from `NOVA_RUNTIME_LIB` or beside the executable;
   exit codes 0/1/2 and S0's `built`/`Error:` lines; no project mode, no
   std builtin (differs)). Tasks 1 to 3; ADR 0059 §1 to §3.
6. **`build` without `--release` is `-O2`, as with it** (the user's
   decision 6, as given; recorded that S0 without `--release` uses
   Cranelift). One path in `build_command`; the clang test's "without
   --release" case.
7. **The harness runs S1's real `build` per golden program** (the user's
   decision 7, as given: the same stdin, environment and exclusion tables
   as `run_tests.rs`). The tables in `golden/shared.rs`, both walks
   through `golden_programs` and `run_golden` (Task 1).
8. **The gate besides golden 147/147** (the user's decision 8, as given:
   (a) S1's binaries hash-equal to S0 `--release`'s for the same `-o`
   name; (b) `build`'s and `check`'s messages and exit codes against S0's
   on a probe set, paths and Rust's signal text excepted; not chosen:
   `check`'s code and span on every input). (a) `same_binaries`, byte for
   byte, no hash crate (Task 1); (b) the two probe tests (Tasks 2, 3).

The user's answer of 2026-11-20:

9. **CI measured from the prototype.**
   - **The user's answer, as given:** asked after part 2 whether to push
     the prototype to `claude/loving-archimedes-frtf5q-55b-ci` with a
     draft PR to measure CI, closed unmerged (as #144 in 5.5a), the user
     answered "อนุญาต push -55b-ci + draft PR (แนะนำ)".
   - **What happened, the facts:** `77d247c7` went to that branch with
     draft Sakeerin/nova#146. Its run, 44672859855, failed one job,
     Clippy (`windows-latest`): `constant CC_ARGS is never used`
     (`selfhost.rs:7180`) under `-D warnings`, since its one use sat in a
     `#[cfg(unix)]` block; every other job green, `selfhost.rs` 56.35,
     71.15 and 63.56 s. Part 3 made the constant `#[cfg(unix)]`
     (`ecfcd0f8`) and wrote ADR 0059 (`fe0e5f10`); the main session
     pushed `fe0e5f10` to the same branch under the same draft PR, under
     the user's answer above, "อนุญาต push -55b-ci + draft PR (แนะนำ)",
     given on 2026-11-20, with no second answer asked. Its
     run, 44674948875, passed all 19 jobs: `selfhost.rs` 56.75 s on
     Linux, 86.12 s on macOS and 70.30 s on Windows, against 80, 100 and
     110 s (margins 23.25, 13.88 and 39.70 s); the install 10.7 s; the
     new step 30.3 s. ADR 0059 §14 holds these (`40e10c42`, local). #146
     was closed unmerged.
   - **In this plan:** the fix is in Task 2's tests; the mimicked Windows
     clippy runs in every task's checks (Conventions), and the PR's
     Windows legs are the check. `main`'s run at `e71446df`, 44479648345,
     is the base's: 54.92, 67.47 and 66.85 s.

How this plan is cut and checked:

10. **The spec's recommendations, not asked, as taken** (spec header):
    `--std` required, never searched for; N's own usage line, exit 2;
    the default output the relative `<stem>`; the runtime looked for
    after `clang`, no `llc`; Windows known from `current_exe`'s shape;
    codespan's header and locus lines; the harness as `#[ignore]`d tests
    in `selfhost.rs`, the tables in a module both test files include;
    binaries compared byte for byte; the probes without `clang` in the
    main step on all three systems, those with it in the new step; the
    prototype measured on CI first; one pull request, split into tasks.
11. **Three tasks test first, then the records** **(differs)**: spec
    §11's cut, with these changes:
    - **Task 1's code carries `fail` and the errors part 1 wrote,**
      where §11 puts them in item 2: `build`'s success path is a chain
      of `Result`s (`compile_and_link`, `run_tool`, `find_runtime`), and
      part 1 wrote each `Err` arm in S0's words through `fail`. Task 1
      tests the success path (S1's golden walk); Task 2's probes are the
      first to compare those words with S0's;
    - **`build` and `check` are separated** (part 2 built and turned them
      green together): Task 2 holds `build`'s refusals and errors, and
      with them §4.1's lines, since `build` prints its diagnostics in
      them (spec §3.2, step 3) and its compile-error probe compares
      them; the main probe test there holds `build`'s 15 cases (9
      `build`, 4 usage, 2 N alone) and the clang test all of its own;
      Task 3 adds `check`: `check_command`, its command line, its 24
      cases (19 `check`, 3 usage, 2 N alone), their inputs and the copy
      of `examples/05-geometry`. The test keeps the head's name,
      `n_build_and_check_answer_as_s0_does`, from Task 2 on;
    - **the CI step names `s1_builds_the_golden_suite` alone in Task 1,**
      the one ignored test there; Task 2's tests patch adds
      `n_build_answers_as_s0_does_with_clang` to the line with the test,
      so each name exists at the commit that names it;
    - **`the_ci_step_names_every_clang_test` comes in Task 1's tests,**
      where it first can hold: it asserts that the step's `--exact` names
      are exactly the tests `selfhost.rs` marks `#[ignore = "needs clang;
      ..."]`, a rule that holds with one ignored test in Task 1 and two
      from Task 2 on, with no edit. Task 1's tests commit has no step
      yet, so the test fails there for that stated reason (`ci.yml has
      no step `- name: S1 builds the golden suite (Linux)``), beside the
      golden walk's failure; Task 1's code adds the step, and it passes
      from then on;
    - **§11's fourth item, the measurements, is no task:** CI was measured
      from the prototype before this plan (decision 9); S1 building N is
      measured again at the last commit (Task 5, Step 7);
    - **the prototype's later fixes are folded in where they belong:**
      `973765a8`'s and `77d247c7`'s cases and `ecfcd0f8`'s `#[cfg(unix)]`
      into Task 2's tests, and so are `9304d751`'s case and `e97c759a`'s
      count; `bbc44b68`'s test into Task 1's tests; `82d22393`'s records
      into Task 4 (decision 16).
    The intermediate trees: Task 1's, `selfhost.rs` as part 1 left it
    (`832314bc`) with the head's `the_ci_step_names_every_clang_test`
    after its last test, and `selfhost/` as part 1 left it (`be9ce33a`),
    the CI step with one name; Task 2's, the head's harness without `check`'s
    cases, inputs, usage line and the 05-geometry copy, the head's
    `driver_tests.nova` without `check`'s command line, and N's driver
    without `check` (no `check_args`, `command_args`, `check_command`;
    `compile_root` lowering every root it checks); Task 3's, the head's
    but the records; Task 4's, the head's. Each tests commit is the
    previous code commit plus the next tree's test files (Task 2's with
    `driver_tests.nova` imported by `main.nova` and the CI line). `split.py`
    makes them and checks every commit's tree. Each task's commit, made
    after its code patch, builds and passes `S` and `SH`.
12. **No user-visible change, shown without a base binary** **(differs**
    from 5.5a's final task, not from the spec**)**: 5.5b touches no S0
    source, so where 5.5a compared parse, check, packages, `--dump-mir`,
    help and usage against a base build, Task 5, Step 4 shows that `git
    diff 17ecca86 HEAD` over the whole tree but six excluded paths
    prints nothing. The pathspec takes every file but those, so it
    covers every S0 input, wherever it lies: the manifests and the
    lockfile, every crate's `src/` and `build.rs`, `std/`, and the inputs
    outside `src/` that a narrower pathspec would miss,
    `crates/nova-cli/build_support.rs` (`mod build_support;` in its
    `build.rs`) and `crates/nova-doc/assets/` (`include_str!` in
    `nova-doc/src/lib.rs`); Step 4 lists them. No excluded path can reach
    S0's binary:
    - `crates/nova-cli/tests`: integration tests, built as test targets
      only, never into `nova`;
    - `selfhost`: N's Nova sources, not a workspace member, read only by
      the tests that build N;
    - `docs` and `nova-spec`: prose;
    - `CHANGELOG.md`: prose;
    - `.github`: CI's configuration, which runs the build but is no
      input to it.
    And no `include_str!` or `include_bytes!` outside a crate's `tests/`
    and `benches/` names a file under an excluded path (Step 4's third
    command). With the golden suite at 147 and 9 (Step 3), that is the
    proof. No base binary is built or kept (the disk has no room for
    one).
13. **The spec's open points as the prototype settled them** (ADR 0059
    §17), each cheap to change: `build` checks in full, as S0's does;
    std's loading (`load_std`, `read_std_modules`, `labels_and_texts`,
    `io_kind_name`) moved verbatim to `driver.nova`, public, since Nova
    forbids import cycles; std unreadable is N's `error: cannot read
    std's PATH: KIND`, exit 2; the command line's edges (an empty value
    or a separate one starting with `-` refused; `-` a FILE; `-oX` and
    `-o=X` refused); a refused manifest's head names FILE; the
    extraction's tail every line from the first `Error: `; std's loci
    read from `--std` on demand; each probe its own directory, the files
    each side leaves compared; `golden_dir()` and `run_golden()` in the
    shared file.
14. **The facts that proved wrong** **(differs; the amendment records
    each)** (ADR 0059 §17, which lists them as departures from the spec):
    §10 projected +1 to 3 s for the main step; it measured +9 to 15 s
    locally (the probe test about 3.5 s alone), and on CI it did not show
    above the runners' spread (Linux 56.75 s against `main`'s 54.92 s);
    §10 projected the new step at 46 to 57 s on CI; it took 30.3 s;
    §6.3's clang list gained three cases: "clang fails, and no runtime"
    and "cc's command line" in part 2, and "a runtime two directories up"
    here (decision 16); §9's locus rule ("the earliest primary against an
    earlier secondary") has no source input, and only N's `@test` reaches
    it; the research's `frem.nova` wrote `{a % b}` without `$`, so it
    never hit float `%`; local clippy runs only Linux's items, so
    `CC_ARGS` was dead code on Windows; and where part 2's report says two
    mutants survived until a case was added, they were four (M3b, M4a,
    M4b and H2; mutants 4, 5, 6 and 21 here), mutant 37 is a fifth, and
    mutant 41 (a name misspelt in the CI step, which no test read) a
    sixth, caught by a test added for it.
15. **The records name this branch and date 5.5b 2026-11-20,** as the
    prototype wrote them: ADR 0059's Status `Accepted (2026-11-20)` and
    `claude/loving-archimedes-frtf5q-55b`; its four dated notes in the
    specification (`00-MASTER-SPEC.md`, `14-CODEGEN.md`'s two and
    `40-TOOLING.md`) in one shape, `**Amended 2026-11-20 (branch
    `claude/loving-archimedes-frtf5q-55b`, ADR 0059):**` (`82d22393` gave
    `40-TOOLING.md`'s note its ADR, which earlier notes there lack), and
    `docs/phase-5-plan.md`'s in that file's own shape, after its 5.5a
    note: `**Amended 2026-11-20 (5.5b, branch
    `claude/loving-archimedes-frtf5q-55b`, ADR 0059; spec …):**`. No
    record names a local branch or a scratch path. The ADR names four commits: `17ecca86`
    (the spec's, this branch's first) and three of the prototype's on the
    pushed `claude/loving-archimedes-frtf5q-55b-ci`: `09f4257b`,
    `77d247c7` and `fe0e5f10`. Its §9, §11 and §17 list `9304d751`'s
    case, §8 the CI-step test, and §11 this plan's mutants (`82d22393`):
    the records are complete before Task 4, not left to the review.
16. **The mutants** (Task 5, Step 8): 64 (part 2's 23 as 1 to 23; part
    1's checks 24 to 41, and 63 and 64, the CI step's; §3.4's words 42 to
    62), 60 caught and 4 accepted with their reasons in Step 8: 28 (an
    environment entry `env` itself makes moot), 31 (S1's `built OUT`
    check, met by the clang test's comparison of the same line, mutant
    42), 33 (`clang`'s arguments in another order, the same command) and
    45 (the Windows refusal's words, unreachable here; CI's Windows leg).
    Two survivors became tests on the prototype, and the split was made
    again after each:
    - mutant 37 (the runtime looked for in N's own directory alone)
      survived every test, so `9304d751` adds the clang test's case "a
      runtime two directories up" (N copied into `up/bin`, the runtime
      beside `up`, `NOVA_RUNTIME_LIB` unset: `built up`), which catches
      it;
    - mutant 41 (a name misspelt in the CI step) passed: with `--exact`
      the step ran one test and passed. `bbc44b68` adds
      `the_ci_step_names_every_clang_test` (decision 11), in the main
      step on all three systems, which catches it, and mutants 63 (a
      name dropped from the step) and 64 (`#[ignore]` taken off the clang
      test) with it.
    A harness check is not weakened as a mutant of its own where a
    mutant of the behaviour it guards meets it, but the checks that hold
    the gate (decision 8) are each run as one: the binary comparison
    (20), the stdout check (21, 30), S1's `built OUT` (31), the extraction
    (22, 23). All 64 results come from one run on the last commit's tree,
    after the fact-check's re-split; the runs add up to 1,534 s, 26
    minutes, with N on the release runtime.
17. **The figures, this machine's** (Task 5): 4 CPUs, Rust 1.97 (stable;
    1.100.0 for CI's clippy, 1.78.0 for the MSRV check), Node v22.23.4,
    `clang-18` 18.1.3. Its speed varies between sessions (`SH` without
    the release runtime took 148 to 187 s here, 209 to 228 s in part 2's).
    S1 building N, measured again on 2026-11-20 by ADR 0059 §13's method
    (S1 built by Cranelift on the release runtime), at the first split's
    last commit, whose `selfhost/` and S0 sources this split's has: 10.02 and
    10.21 s against §13's 13.29 and 12.97 s, peak 203.5 MB as §13; S0's
    `--release` 10.01 and 9.73 s (11.74, 12.36); S2 building N 9.97 and
    10.21 s (12.84, 12.76); every size, digest and equality as §13
    (9,354,680 bytes, the `.ll` 12,759,478 bytes and 406,232 lines). The
    times run 0.75 to 0.85 of §13's, the machine's speed; the ADR keeps
    part 3's figures, labelled as measured at `ecfcd0f8` on 2026-11-20,
    and this plan's replay does not replace them.
18. **The branch starts from `17ecca86`:**
    `claude/loving-archimedes-frtf5q-55b` from `17ecca86` (`main`
    `e71446df` and the approved spec), then this plan's commit, which adds
    this file and appends the spec's dated amendment (2026-11-20): the
    facts that proved wrong (decision 14), part 2's open decisions as
    settled (decision 13), and CI's figures (decision 9).
19. **CI's lines and counts for the PR** (decision 9): the Selfhost job's
    `selfhost.rs` is read on all three systems against the lines, 80 s on
    Linux, 100 s on macOS and 110 s on Windows, and against run
    44674948875's 56.75, 86.12 and 70.30 s; past a line, the user is
    asked. The counts expected: `selfhost.rs` 70 passed and 2 ignored on
    Linux, 68 and 2 on macOS, 64 and 2 on Windows: run 44674948875's 69,
    67 and 63 (`main`'s 67, 65 and 61, and two tests added in the main
    step) and `the_ci_step_names_every_clang_test`, which runs on every
    system (`9304d751`'s case and `e97c759a`'s count are inside an
    ignored test); the new step `2 passed`, `70 filtered out`. The Test job's main step was not
    extracted from #146; its tests are the base's (2,848 listed, the same
    names), so its sums should be 5.5a's PR run's, `2838 passed, 0
    failed, 10 ignored` on Linux, `2838, 0, 9` on macOS and `2815, 0, 9`
    on Windows. Here the Linux suite gives `2837 1 10`, the IPv6 test
    failing for want of IPv6.

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-55b`: from `17ecca86`,
then this plan's commit, which adds this file and appends the spec's
dated amendment, so the patches apply to it as to `17ecca86` (decision
18).

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-55b`.
  - `git log --format='%h %s' -2` prints the plan's commit, then
    `17ecca86 docs: the Phase 5.5b spec is approved`.
  - `git diff --stat 17ecca86 HEAD` lists only
    `docs/superpowers/plans/2026-11-20-phase-5-5b-the-driver-and-linking.md`
    and
    `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 5's review reads from it.
  - `df -h /` shows at least 6.5G free; if not, remove older duplicate
    test executables in `target/debug/deps` first. The replay began with
    7.4G, and its first test build took 0.7G (Global Constraints).
- [ ] **Step 2: The tools and the process limit.**
  - `clang-18 --version | head -n 1` prints
    `Ubuntu clang version 18.1.3 (1ubuntu1)`: CI's pin, for the golden
    suite and the ignored tests.
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
  mkdir -p "$P/exec55b"
  cargo build --locked --workspace
  cargo test --locked --workspace --no-run
  S --lib --bins > "$P/exec55b/base-a.txt" 2>&1
  S --doc > "$P/exec55b/base-b.txt" 2>&1
  S --test '[!s]*' --test 's[!e]*' > "$P/exec55b/base-c.txt" 2>&1
  cat "$P"/exec55b/base-[abc].txt > "$P/exec55b/base.txt"
  grep '^test result' "$P/exec55b/base.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec55b/base.txt"
  cargo test --locked --workspace -- --list > "$P/exec55b/base-list.txt" 2>&1
  grep -c ': test$' "$P/exec55b/base-list.txt"
  cargo test --locked --workspace --test selfhost -- --list > "$P/exec55b/base-list-sh.txt" 2>&1
  grep -c ': test$' "$P/exec55b/base-list-sh.txt"
  cargo test --locked --workspace --test selfhost --no-run
  time SH
  sha256sum Cargo.lock
  ```

  Expected, from the replay on `17ecca86`: `1300 1 8`, `2 0 0` and `1535
  0 2`, in 98, 3 and 185 s, `2837 1 10` summed; one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`, which needs IPv6, which this machine lacks, as in every
  earlier phase; the list holds `2848` names, the run's 2837 + 1 + 10;
  `selfhost.rs`'s list `67`, and `SH` `67 passed; 0 failed` in 148 s;
  `Cargo.lock`'s sha256 begins `a3009b6850e56f7b`. Task 5 expects `2837 1
  10` and 2,848 names again, and 72 names in `selfhost.rs`'s list.
- [ ] **Step 4: The ledger.** Start `$P/ledger-55b.md`, headed with the
  base commit, the baseline's figures, `Cargo.lock`'s sha256 and the
  process limit's form; and `$P/exec55b`, the executor's directory
  (Conventions), made in Step 3.


### Task 1: The golden tables shared, N's `build` on its success path, and S1's golden walk in CI

Spec §3.1 to §3.3, §5, §6.1, §6.2, §7, §11 item 1; decisions 1, 2, 5, 6,
7, 8a and 11.

**Files:**
- Create: `crates/nova-cli/tests/golden/shared.rs`, `selfhost/src/driver.nova`
- Modify: `crates/nova-cli/tests/run_tests.rs`,
  `crates/nova-cli/tests/selfhost.rs`, `selfhost/src/loader.nova`,
  `selfhost/src/main.nova`, `.github/workflows/ci.yml`

- [ ] **Step 1: The tests.** `patch_block t1-tests`:
  - **`golden/shared.rs`** (new), included by `run_tests.rs` and
    `selfhost.rs` as `#[path = "golden/shared.rs"] mod golden;`: the
    `RELEASE_GOLDEN_*` tables and limits, `PROCESS_TEST_VALUE` and
    `PROCESS_TEST_EQ`, `quoted_stderr`, `run_within_limit`,
    `run_release_binary`, moved unchanged from `run_tests.rs`; and
    `golden_dir`, `golden_programs` (`release_golden_suite`'s walk, its
    table checks and its floor of 131) and `run_golden` (one program run
    with its stdin, variables and expected status, its stdout held equal
    to its `.stdout`), which both walks call; `release_golden_suite` is
    otherwise as it was;
  - **`selfhost.rs`:** `golden_runtime` (`NOVA_RUNTIME_LIB`, else the one
    runtime `build_selfhost` unpacked, else the one beside S0),
    `same_binaries` (both sizes and the first differing offset), and
    `s1_builds_the_golden_suite`, `#[ignore]`d (temporary directory
    `nova-selfhost-golden`): `clang` probed (`not run (no clang)` and a
    pass, or a failure under `NOVA_REQUIRE_CLANG=1`); on 4 threads, each
    program built by S1 (`selfhost build SRC -o DIR/s1/NAME --release
    --std std`, its stdout exactly `built OUT`) and by S0 (`nova build
    --release SRC -o DIR/s0/NAME`), one runtime passed to both; S1's
    binary run as `run_golden` runs it; the two binaries equal; the
    report `s1 golden: 147 passed, 9 excluded, <clang>, runtime <path>`;
  - **`selfhost.rs`, `the_ci_step_names_every_clang_test`,** in the main
    step (decision 11): it reads `.github/workflows/ci.yml` from the
    repository and `selfhost.rs`'s own source (`include_str!`), as text,
    CRLF line endings allowed; finds the step `S1 builds the golden
    suite (Linux)` and the names after `--exact` on its `run:` line; and
    asserts that each is a test of the file marked `#[ignore]`, and that
    they are exactly the tests marked `#[ignore = "needs clang; ..."]`.
    It needs no crate, no network and no N: `finished in 0.00s`, 3 to
    5 ms for the whole test process, five runs.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished` and no warning; then `cargo test --locked --workspace --test
  selfhost --no-run` (3 s).
  - `G` (19 s). Expected: `release golden: 147 passed, 9 excluded, Ubuntu
    clang version 18.1.3 (1ubuntu1)` and `1 passed`: the tables moved,
    and the walk unchanged.
  - `C s1_builds_the_golden_suite --nocapture` (9 s). Expected: `0
    passed; 1 failed`: `s1 golden: 0 passed, 9 excluded, Ubuntu clang
    version 18.1.3 (1ubuntu1), runtime …/target/debug/libnova_runtime.a`
    and `147 of 147 golden programs failed as S1 built them`, each
    ``NAME: `selfhost build` failed, exit status Some(2)`` with the
    stderr `usage: selfhost tokens|ast FILE...`: the base's N has no
    `build`.
  - `SH --exact the_ci_step_names_every_clang_test` (under 1 s).
    Expected: `0 passed; 1 failed`, the panic ``ci.yml has no step `-
    name: S1 builds the golden suite (Linux)` ``: the step comes with
    the code.
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`driver.nova`** (new, 555 lines): `BUILD_USAGE`, `NO_TOOLCHAIN`,
    `BuildArgs` and `build_args` (each option once, any order, `--`
    ending them; anything else `None`); `build_command`: std from
    `--std`, FILE through `compile_root` (loaded as `selfhost mir` loads
    it, resolved, checked and lowered in full, std's bodies with it), the
    emitter's text written to `intermediate(out, "ll")`, then
    `compile_and_link` (`$NOVA_CLANG` or `clang`, probed by `--version`,
    `-O2 -c`; then `find_runtime`, `$NOVA_RUNTIME_LIB` or
    `libnova_runtime.a` in N's directory, its parent or its grandparent;
    then `$CC` or `cc`, `-o OUT OBJ RUNTIME -lpthread -ldl -lm`), the
    object removed always, the `.ll` on success, `built OUT`; each
    failure through `fail`, anyhow's layout in S0's words, exit 1 (Task
    2's probes compare them, decision 11); `run_tool`, `debug_quoted`,
    `tool_text`; and std's loading (`load_std`, `read_std_modules`,
    `labels_and_texts`, `io_kind_name`) moved from `main.nova`, verbatim;
  - **`loader.nova`:** `file_stem_or(path, fallback)`, `module_name`'s
    stem with `main`, the default output's with `out`; `file_name`
    public;
  - **`main.nova`:** `selfhost build` dispatched to `build_command`; its
    usage comment; std's loading gone to `driver.nova`;
  - **`ci.yml`:** after `cargo test --test selfhost`, on the Linux leg,
    "Install clang 18.1.3 (Linux)" (the Test job's step, verbatim) and
    "S1 builds the golden suite (Linux)" with `NOVA_REQUIRE_CLANG: "1"`
    and `NOVA_CLANG: clang-18`, running `cargo test --locked --workspace
    --all-features --no-fail-fast --test selfhost -- --ignored --exact
    s1_builds_the_golden_suite` (decision 11).
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `C s1_builds_the_golden_suite --nocapture` (64 s, the debug runtime).
    Expected: `s1 golden: 147 passed, 9 excluded, Ubuntu clang version
    18.1.3 (1ubuntu1), runtime …/target/debug/libnova_runtime.a` and `1
    passed`.
  - `cargo test --locked --workspace --no-run` (1 s), then `S` in three
    runs (105, 1 and 168 s). Expected: `1300 1 8`, `2 0 0` and `1535 0
    2`, `2837 1 10`, the IPv6 test the one failure.
  - `cargo test --locked --workspace --test selfhost --no-run`, then `SH`
    (158 s). Expected: `68 passed; 0 failed; 1 ignored`: the base's 67
    and `the_ci_step_names_every_clang_test`, with
    `s1_builds_the_golden_suite` ignored.
- [ ] **Step 5: The checks,** as the conventions list them (the
  mimicked Windows clippy turns off `0` items: Task 1 adds no
  `#[cfg(unix)]`).
- [ ] **Step 6: Commit** `selfhost: N's build, its success path, and S1
      builds the golden suite`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/golden/shared.rs b/crates/nova-cli/tests/golden/shared.rs
new file mode 100644
index 00000000..338b9121
--- /dev/null
+++ b/crates/nova-cli/tests/golden/shared.rs
@@ -0,0 +1,250 @@
+//! The golden programs of `tests/runtime/` as a built binary runs them
+//! (5.0 spec §3.2; ADR 0048), shared by the two walks that build them:
+//! `run_tests.rs`'s `release_golden_suite`, S0's `nova build --release`,
+//! and `selfhost.rs`'s `s1_builds_the_golden_suite`, N's own `selfhost
+//! build` (spec
+//! `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`
+//! §6.1). Both include this file, so there is one copy of the walk, of its
+//! tables and of how a binary is run and judged. A file in a subdirectory
+//! of `tests/` is no test target of its own; what one includer leaves
+//! unused is allowed dead there.
+
+#![allow(dead_code)]
+
+use std::path::{Path, PathBuf};
+
+/// The programs of `tests/runtime/` that `release_golden_suite` leaves out,
+/// each with why: what their own tests set up, which a built binary run
+/// alone does not have (5.0 spec §3.2).
+pub const RELEASE_GOLDEN_EXCLUDED: &[(&str, &str)] = &[
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
+pub const RELEASE_GOLDEN_STDIN: &[(&str, &str)] = &[("io_streams", "hello-stdin")];
+
+/// The environment variables `release_golden_suite` sets on a program,
+/// beside those it inherits: `process_env` reads the two its tests set (5.0
+/// spec §5.7).
+pub const RELEASE_GOLDEN_ENV: &[(&str, &str, &str)] = &[
+    ("process_env", "NOVA_PROCESS_TEST_VALUE", PROCESS_TEST_VALUE),
+    ("process_env", "NOVA_PROCESS_TEST_EQ", PROCESS_TEST_EQ),
+];
+
+/// The exit status `release_golden_suite` expects of a program, where it is
+/// not 0: a program that panics after printing its `.stdout` aborts.
+pub const RELEASE_GOLDEN_STATUS: &[(&str, i32)] = &[];
+
+/// How long one `nova build --release`, or one release binary, may run.
+pub const RELEASE_GOLDEN_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);
+
+/// How much of a process's standard error a failure quotes: its last
+/// `RELEASE_GOLDEN_STDERR` characters.
+pub const RELEASE_GOLDEN_STDERR: usize = 2000;
+
+/// `stderr` for a failure's message: its end, where a panic's message is,
+/// cut to `RELEASE_GOLDEN_STDERR` characters, or a note that it is empty.
+pub fn quoted_stderr(stderr: &str) -> String {
+    let stderr = stderr.trim_end();
+    if stderr.is_empty() {
+        return "\n--- stderr: empty ---".to_string();
+    }
+    let count = stderr.chars().count();
+    if count <= RELEASE_GOLDEN_STDERR {
+        return format!("\n--- stderr ---\n{stderr}");
+    }
+    let tail: String = stderr.chars().skip(count - RELEASE_GOLDEN_STDERR).collect();
+    format!("\n--- stderr, its last {RELEASE_GOLDEN_STDERR} of {count} characters ---\n{tail}")
+}
+
+/// What `command` did within `RELEASE_GOLDEN_LIMIT`, given `stdin` or a
+/// closed standard input: its standard output, its standard error and its
+/// exit status, or why it has none, with its standard error quoted.
+pub fn run_within_limit(
+    mut command: std::process::Command,
+    stdin: Option<&str>,
+) -> Result<(String, String, Option<i32>), String> {
+    use std::io::{Read, Write};
+    use std::process::Stdio;
+    fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
+        std::thread::spawn(move || {
+            let mut out = Vec::new();
+            let _ = pipe.read_to_end(&mut out);
+            out
+        })
+    }
+    let mut child = command
+        .stdin(if stdin.is_some() {
+            Stdio::piped()
+        } else {
+            Stdio::null()
+        })
+        .stdout(Stdio::piped())
+        .stderr(Stdio::piped())
+        .spawn()
+        .map_err(|e| format!("cannot start: {e}"))?;
+    let out = drain(child.stdout.take().expect("piped stdout"));
+    let err = drain(child.stderr.take().expect("piped stderr"));
+    if let Some(text) = stdin {
+        let mut pipe = child.stdin.take().expect("piped stdin");
+        let _ = pipe.write_all(text.as_bytes());
+    }
+    let text = |h: std::thread::JoinHandle<Vec<u8>>| {
+        String::from_utf8_lossy(&h.join().expect("a reader thread")).replace('\r', "")
+    };
+    let start = std::time::Instant::now();
+    let status = loop {
+        match child.try_wait() {
+            Ok(Some(status)) => break status,
+            Ok(None) if start.elapsed() > RELEASE_GOLDEN_LIMIT => {
+                let _ = child.kill();
+                let _ = child.wait();
+                let _ = text(out);
+                let stderr = text(err);
+                return Err(format!(
+                    "timed out after {RELEASE_GOLDEN_LIMIT:?}{}",
+                    quoted_stderr(&stderr)
+                ));
+            }
+            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
+            Err(e) => return Err(format!("cannot wait: {e}")),
+        }
+    };
+    Ok((text(out), text(err), status.code()))
+}
+
+/// What one release binary did: its standard output, its standard error
+/// and its exit status, or why it has none.
+pub fn run_release_binary(
+    exe: &std::path::Path,
+    tmp: &std::path::Path,
+    stdin: Option<&str>,
+    vars: &[(&str, &str)],
+) -> Result<(String, String, Option<i32>), String> {
+    let mut command = std::process::Command::new(exe);
+    // Each program's temporary files go in a directory of its own, so none
+    // races the same fixture's `run` test.
+    command
+        .env("TMPDIR", tmp)
+        .env("TMP", tmp)
+        .env("TEMP", tmp)
+        .env_remove("NOVA_GC_STRESS")
+        .envs(vars.iter().copied());
+    run_within_limit(command, stdin)
+}
+
+/// The value `process_env`'s tests, and the `--release` walk, give
+/// `NOVA_PROCESS_TEST_VALUE`: `=` and non-ASCII included.
+pub const PROCESS_TEST_VALUE: &str = "a = b, héllo ✓";
+
+/// The value they give `NOVA_PROCESS_TEST_EQ`: with it set, glibc's
+/// `getenv("NOVA_PROCESS_TEST_EQ=x")` finds `y`, and macOS's the whole
+/// value, so the program's `None` for that name is `env`'s own refusal of a
+/// name holding `=`, seen on every system (5.0 spec §5.7).
+pub const PROCESS_TEST_EQ: &str = "x=y";
+
+/// `tests/runtime/`, where the golden programs are.
+pub fn golden_dir() -> PathBuf {
+    Path::new(env!("CARGO_MANIFEST_DIR"))
+        .join("..")
+        .join("..")
+        .join("tests")
+        .join("runtime")
+}
+
+/// The golden walk: every program of `tests/runtime/` that has a `.stdout`
+/// file, by name, sorted, less those `RELEASE_GOLDEN_EXCLUDED` names. The
+/// tables must name programs that exist, and the walk must hold at least
+/// 5.0a's 131.
+pub fn golden_programs() -> Vec<String> {
+    let runtime = golden_dir();
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
+    for (name, _, _) in RELEASE_GOLDEN_ENV {
+        assert!(
+            programs.iter().any(|p| p == name),
+            "the environment table names `{name}`, which has no golden program"
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
+    walk
+}
+
+/// Golden program `name`'s binary `exe` run, its temporary files in `tmp`,
+/// with the standard input and the variables the tables give it: `Ok` when
+/// its standard output is its `.stdout` file and its exit status the one
+/// `RELEASE_GOLDEN_STATUS` expects, else why not.
+pub fn run_golden(name: &str, exe: &Path, tmp: &Path) -> Result<(), String> {
+    let source = golden_dir().join(format!("{name}.nova"));
+    let stdin = RELEASE_GOLDEN_STDIN
+        .iter()
+        .find(|(n, _)| *n == name)
+        .map(|(_, t)| *t);
+    let want_status = RELEASE_GOLDEN_STATUS
+        .iter()
+        .find(|(n, _)| *n == name)
+        .map_or(0, |(_, c)| *c);
+    let expected = std::fs::read_to_string(source.with_extension("stdout"))
+        .expect("the .stdout file")
+        .replace('\r', "");
+    let vars: Vec<(&str, &str)> = RELEASE_GOLDEN_ENV
+        .iter()
+        .filter(|(n, _, _)| *n == name)
+        .map(|(_, k, v)| (*k, *v))
+        .collect();
+    let (out, stderr, status) = run_release_binary(exe, tmp, stdin, &vars)?;
+    if status != Some(want_status) {
+        return Err(format!(
+            "exit status {status:?}, expected {want_status}{}",
+            quoted_stderr(&stderr)
+        ));
+    }
+    if out != expected {
+        return Err(format!(
+            "standard output differs\n--- expected ---\n{expected}\
+             --- got ---\n{out}{}",
+            quoted_stderr(&stderr)
+        ));
+    }
+    Ok(())
+}
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index ea36b5ea..720c8a1a 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -1,7 +1,14 @@
 //! End-to-end `nova run` tests — the Phase 1 gate criteria
 //! (00-MASTER-SPEC.md §3, Phase 1).
 
+#[path = "golden/shared.rs"]
+mod golden;
+
 use assert_cmd::Command;
+use golden::{
+    golden_dir, golden_programs, quoted_stderr, run_golden, run_within_limit, PROCESS_TEST_EQ,
+    PROCESS_TEST_VALUE, RELEASE_GOLDEN_EXCLUDED,
+};
 use std::path::PathBuf;
 
 fn repo_root() -> PathBuf {
@@ -1779,141 +1786,6 @@ fn release_builds_and_runs_when_clang_available() {
     let _ = std::fs::remove_file(&exe);
 }
 
-/// The programs of `tests/runtime/` that `release_golden_suite` leaves out,
-/// each with why: what their own tests set up, which a built binary run
-/// alone does not have (5.0 spec §3.2).
-const RELEASE_GOLDEN_EXCLUDED: &[(&str, &str)] = &[
-    (
-        "process_args",
-        "prints `args()`, whose index 0 is the file as `nova run` was given it",
-    ),
-    ("file_open_dir", "its test makes the directory it opens"),
-    (
-        "fs_permission_denied",
-        "its test makes a file it cannot read, and skips as root",
-    ),
-    ("net_interleave", "its test serves the port it connects to"),
-    ("net_lifetime", "its test serves the port it connects to"),
-    ("net_refused", "its test reserves the port it connects to"),
-    ("net_roundtrip", "its test serves the port it connects to"),
-    ("net_timeout", "its test serves the port it connects to"),
-    ("nova_test", "a `@test` file, which `nova test` runs"),
-];
-
-/// The standard input `release_golden_suite` gives a program; every other
-/// program's is closed.
-const RELEASE_GOLDEN_STDIN: &[(&str, &str)] = &[("io_streams", "hello-stdin")];
-
-/// The environment variables `release_golden_suite` sets on a program,
-/// beside those it inherits: `process_env` reads the two its tests set (5.0
-/// spec §5.7).
-const RELEASE_GOLDEN_ENV: &[(&str, &str, &str)] = &[
-    ("process_env", "NOVA_PROCESS_TEST_VALUE", PROCESS_TEST_VALUE),
-    ("process_env", "NOVA_PROCESS_TEST_EQ", PROCESS_TEST_EQ),
-];
-
-/// The exit status `release_golden_suite` expects of a program, where it is
-/// not 0: a program that panics after printing its `.stdout` aborts.
-const RELEASE_GOLDEN_STATUS: &[(&str, i32)] = &[];
-
-/// How long one `nova build --release`, or one release binary, may run.
-const RELEASE_GOLDEN_LIMIT: std::time::Duration = std::time::Duration::from_secs(60);
-
-/// How much of a process's standard error a failure quotes: its last
-/// `RELEASE_GOLDEN_STDERR` characters.
-const RELEASE_GOLDEN_STDERR: usize = 2000;
-
-/// `stderr` for a failure's message: its end, where a panic's message is,
-/// cut to `RELEASE_GOLDEN_STDERR` characters, or a note that it is empty.
-fn quoted_stderr(stderr: &str) -> String {
-    let stderr = stderr.trim_end();
-    if stderr.is_empty() {
-        return "\n--- stderr: empty ---".to_string();
-    }
-    let count = stderr.chars().count();
-    if count <= RELEASE_GOLDEN_STDERR {
-        return format!("\n--- stderr ---\n{stderr}");
-    }
-    let tail: String = stderr.chars().skip(count - RELEASE_GOLDEN_STDERR).collect();
-    format!("\n--- stderr, its last {RELEASE_GOLDEN_STDERR} of {count} characters ---\n{tail}")
-}
-
-/// What `command` did within `RELEASE_GOLDEN_LIMIT`, given `stdin` or a
-/// closed standard input: its standard output, its standard error and its
-/// exit status, or why it has none, with its standard error quoted.
-fn run_within_limit(
-    mut command: std::process::Command,
-    stdin: Option<&str>,
-) -> Result<(String, String, Option<i32>), String> {
-    use std::io::{Read, Write};
-    use std::process::Stdio;
-    fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<Vec<u8>> {
-        std::thread::spawn(move || {
-            let mut out = Vec::new();
-            let _ = pipe.read_to_end(&mut out);
-            out
-        })
-    }
-    let mut child = command
-        .stdin(if stdin.is_some() {
-            Stdio::piped()
-        } else {
-            Stdio::null()
-        })
-        .stdout(Stdio::piped())
-        .stderr(Stdio::piped())
-        .spawn()
-        .map_err(|e| format!("cannot start: {e}"))?;
-    let out = drain(child.stdout.take().expect("piped stdout"));
-    let err = drain(child.stderr.take().expect("piped stderr"));
-    if let Some(text) = stdin {
-        let mut pipe = child.stdin.take().expect("piped stdin");
-        let _ = pipe.write_all(text.as_bytes());
-    }
-    let text = |h: std::thread::JoinHandle<Vec<u8>>| {
-        String::from_utf8_lossy(&h.join().expect("a reader thread")).replace('\r', "")
-    };
-    let start = std::time::Instant::now();
-    let status = loop {
-        match child.try_wait() {
-            Ok(Some(status)) => break status,
-            Ok(None) if start.elapsed() > RELEASE_GOLDEN_LIMIT => {
-                let _ = child.kill();
-                let _ = child.wait();
-                let _ = text(out);
-                let stderr = text(err);
-                return Err(format!(
-                    "timed out after {RELEASE_GOLDEN_LIMIT:?}{}",
-                    quoted_stderr(&stderr)
-                ));
-            }
-            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(20)),
-            Err(e) => return Err(format!("cannot wait: {e}")),
-        }
-    };
-    Ok((text(out), text(err), status.code()))
-}
-
-/// What one release binary did: its standard output, its standard error
-/// and its exit status, or why it has none.
-fn run_release_binary(
-    exe: &std::path::Path,
-    tmp: &std::path::Path,
-    stdin: Option<&str>,
-    vars: &[(&str, &str)],
-) -> Result<(String, String, Option<i32>), String> {
-    let mut command = std::process::Command::new(exe);
-    // Each program's temporary files go in a directory of its own, so none
-    // races the same fixture's `run` test.
-    command
-        .env("TMPDIR", tmp)
-        .env("TMP", tmp)
-        .env("TEMP", tmp)
-        .env_remove("NOVA_GC_STRESS")
-        .envs(vars.iter().copied());
-    run_within_limit(command, stdin)
-}
-
 /// Every golden program of `tests/runtime/` built with `--release`, through
 /// `clang`, and run (5.0 spec §3.2; ADR 0048): its standard output must be
 /// its `.stdout` file and its exit status the one the table expects.
@@ -1953,41 +1825,8 @@ fn release_golden_suite() {
         panic!("NOVA_REQUIRE_CLANG=1, but `{clang} --version` does not run");
     };
 
-    let runtime = repo_root().join("tests/runtime");
-    let mut programs: Vec<String> = std::fs::read_dir(&runtime)
-        .expect("tests/runtime")
-        .filter_map(|e| {
-            let path = e.expect("a directory entry").path();
-            let stem = path.file_stem()?.to_str()?.to_string();
-            let is_nova = path.extension().and_then(|x| x.to_str()) == Some("nova");
-            (is_nova && path.with_extension("stdout").exists()).then_some(stem)
-        })
-        .collect();
-    programs.sort();
-    for (name, _) in RELEASE_GOLDEN_EXCLUDED {
-        assert!(
-            programs.iter().any(|p| p == name),
-            "the exclusion table names `{name}`, which has no golden program"
-        );
-    }
-    for (name, _, _) in RELEASE_GOLDEN_ENV {
-        assert!(
-            programs.iter().any(|p| p == name),
-            "the environment table names `{name}`, which has no golden program"
-        );
-    }
-    let walk: Vec<String> = programs
-        .iter()
-        .filter(|p| !RELEASE_GOLDEN_EXCLUDED.iter().any(|(n, _)| n == p))
-        .cloned()
-        .collect();
-    // A floor, 5.0a's count: a walk that found fewer programs would pass
-    // having checked less than it should.
-    assert!(
-        walk.len() >= 131,
-        "the walk found {} programs, fewer than 5.0a's 131",
-        walk.len()
-    );
+    let runtime = golden_dir();
+    let walk = golden_programs();
 
     let dir = std::env::temp_dir().join("nova-release-golden");
     std::fs::create_dir_all(&dir).expect("the golden directory");
@@ -2036,40 +1875,10 @@ fn release_golden_suite() {
                         continue;
                     }
                 }
-                let stdin = RELEASE_GOLDEN_STDIN
-                    .iter()
-                    .find(|(n, _)| n == name)
-                    .map(|(_, t)| *t);
-                let want_status = RELEASE_GOLDEN_STATUS
-                    .iter()
-                    .find(|(n, _)| n == name)
-                    .map_or(0, |(_, c)| *c);
-                let expected = std::fs::read_to_string(source.with_extension("stdout"))
-                    .expect("the .stdout file")
-                    .replace('\r', "");
-                let vars: Vec<(&str, &str)> = RELEASE_GOLDEN_ENV
-                    .iter()
-                    .filter(|(n, _, _)| n == name)
-                    .map(|(_, k, v)| (*k, *v))
-                    .collect();
-                let ran = run_release_binary(&exe, &tmp, stdin, &vars);
+                let ran = run_golden(name, &exe, &tmp);
                 let _ = std::fs::remove_file(&exe);
-                match ran {
-                    Err(why) => failures.lock().unwrap().push(format!("{name}: {why}")),
-                    Ok((out, stderr, status)) => {
-                        if status != Some(want_status) {
-                            failures.lock().unwrap().push(format!(
-                                "{name}: exit status {status:?}, expected {want_status}{}",
-                                quoted_stderr(&stderr)
-                            ));
-                        } else if out != expected {
-                            failures.lock().unwrap().push(format!(
-                                "{name}: standard output differs\n--- expected ---\n{expected}\
-                                 --- got ---\n{out}{}",
-                                quoted_stderr(&stderr)
-                            ));
-                        }
-                    }
+                if let Err(why) = ran {
+                    failures.lock().unwrap().push(format!("{name}: {why}"));
                 }
             });
         }
@@ -13338,16 +13147,6 @@ fn a_child_killed_by_a_signal_is_128_plus_the_signal() {
     let _ = std::fs::remove_dir_all(&dir);
 }
 
-/// The value `process_env`'s tests, and the `--release` walk, give
-/// `NOVA_PROCESS_TEST_VALUE`: `=` and non-ASCII included.
-const PROCESS_TEST_VALUE: &str = "a = b, héllo ✓";
-
-/// The value they give `NOVA_PROCESS_TEST_EQ`: with it set, glibc's
-/// `getenv("NOVA_PROCESS_TEST_EQ=x")` finds `y`, and macOS's the whole
-/// value, so the program's `None` for that name is `env`'s own refusal of a
-/// name holding `=`, seen on every system (5.0 spec §5.7).
-const PROCESS_TEST_EQ: &str = "x=y";
-
 /// `env` and `current_exe` (5.0 spec §5.7): a variable the test sets on the
 /// program, one it removes, and names no variable can have, under `nova
 /// run`, where the program sees `nova`'s environment, and built, where it
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index dfcee018..424faa0a 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -105,6 +105,16 @@
 //! equals `build --release`'s intermediate on a few programs. A
 //! difference in it is reported by its line, `define` and block.
 //!
+//! 5.5b (spec
+//! `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`
+//! §6.2): N's `selfhost build FILE -o OUT --release --std DIR` writes the
+//! `.ll`, runs `clang` and `cc` on it and leaves a binary.
+//! [`s1_builds_the_golden_suite`], ignored here and run by CI's Selfhost
+//! job on its Linux leg, builds every golden program `release_golden_suite`
+//! walks with it, runs each with the tables both files include
+//! (`golden/shared.rs`), and holds each binary equal to S0 `build
+//! --release`'s, byte for byte.
+//!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
 //!
@@ -123,6 +133,8 @@
 mod compare;
 #[path = "selfhost/fuzz.rs"]
 mod fuzz;
+#[path = "golden/shared.rs"]
+mod golden;
 
 use std::env::consts::EXE_SUFFIX;
 use std::io::Read;
@@ -134,6 +146,10 @@ use std::time::{Duration, Instant};
 
 use compare::{first_difference, report, settle_paths, Dump, AST, TOKENS};
 use fuzz::SplitMix64;
+use golden::{
+    golden_dir, golden_programs, quoted_stderr, run_golden, run_within_limit,
+    RELEASE_GOLDEN_EXCLUDED,
+};
 
 /// How many files one process lexes.
 const BATCH: usize = 50;
@@ -6847,3 +6863,294 @@ fn the_ll_dump_is_what_build_release_writes() {
         );
     }
 }
+
+/// The runtime both sides of [`s1_builds_the_golden_suite`] link: the one
+/// S1 links, so that S0's binary can equal S1's. `NOVA_RUNTIME_LIB` when it
+/// is set; else the one [`build_selfhost`] unpacked into its fresh
+/// `NOVA_HOME`, from a `nova` that carries a runtime, as CI's does
+/// (`NOVA_EMBED_RUNTIME=1`), which must be the only one there; else the one
+/// beside S0, where cargo leaves it in a checkout.
+fn golden_runtime() -> PathBuf {
+    if let Some(lib) = std::env::var_os("NOVA_RUNTIME_LIB") {
+        return PathBuf::from(lib);
+    }
+    let name = "libnova_runtime.a";
+    let unpacked = std::env::temp_dir()
+        .join("nova-selfhost-home")
+        .join("runtime");
+    let mut found: Vec<PathBuf> = std::fs::read_dir(&unpacked)
+        .map(|entries| {
+            entries
+                .map(|e| e.expect("read a directory entry").path().join(name))
+                .filter(|lib| lib.is_file())
+                .collect()
+        })
+        .unwrap_or_default();
+    match found.len() {
+        0 => {
+            let beside = nova().parent().expect("nova's directory").join(name);
+            assert!(
+                beside.is_file(),
+                "no runtime for the golden walk: NOVA_RUNTIME_LIB is not set, \
+                 {} holds none, and none is beside S0 at {}",
+                unpacked.display(),
+                beside.display()
+            );
+            beside
+        }
+        1 => found.remove(0),
+        _ => panic!(
+            "{} holds more than one runtime: {found:?}",
+            unpacked.display()
+        ),
+    }
+}
+
+/// Golden program `name`'s two binaries, `s0` and `s1`, the same byte for
+/// byte, or where they first differ.
+fn same_binaries(s0: &Path, s1: &Path) -> Result<(), String> {
+    let a = std::fs::read(s0).map_err(|e| format!("cannot read {}: {e}", s0.display()))?;
+    let b = std::fs::read(s1).map_err(|e| format!("cannot read {}: {e}", s1.display()))?;
+    if a == b {
+        return Ok(());
+    }
+    let at = a
+        .iter()
+        .zip(&b)
+        .position(|(x, y)| x != y)
+        .unwrap_or(a.len().min(b.len()));
+    Err(format!(
+        "S1's binary differs from S0's: {} bytes against S0's {}, first at offset {at}",
+        b.len(),
+        a.len()
+    ))
+}
+
+/// The golden suite built by S1's own `build` (spec
+/// `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`
+/// §6.2; decisions 1, 7 and 8a): every program `release_golden_suite`
+/// walks, built by `selfhost build SRC -o DIR/s1/NAME --release --std
+/// STD` and by S0's `nova build --release SRC -o DIR/s0/NAME`, one
+/// basename, so one `NAME.nova.ll`, and one runtime ([`golden_runtime`])
+/// for both; S1's binary run with the tables' standard input and variables
+/// must print its `.stdout` and exit with its status, and must equal S0's
+/// byte for byte. Four at a time; each pair of binaries is removed once
+/// compared, and the directory at the end.
+///
+/// It needs `clang` (`NOVA_CLANG`, or `clang`): without one it says so and
+/// passes, but under `NOVA_REQUIRE_CLANG=1` it fails. CI's Selfhost job
+/// runs it on its Linux leg, in a step of its own after this file's, with
+/// `clang-18` installed there, so the main step's time is untouched.
+#[test]
+#[ignore = "needs clang; the Selfhost job's Linux step runs it"]
+fn s1_builds_the_golden_suite() {
+    let required = std::env::var("NOVA_REQUIRE_CLANG").as_deref() == Ok("1");
+    let clang = std::env::var("NOVA_CLANG").unwrap_or_else(|_| "clang".to_string());
+    let Ok(probe) = Command::new(&clang).arg("--version").output() else {
+        assert!(
+            !required,
+            "NOVA_REQUIRE_CLANG=1, but `{clang} --version` does not run"
+        );
+        eprintln!("s1 golden: not run (no clang)");
+        return;
+    };
+    let version = String::from_utf8_lossy(&probe.stdout)
+        .lines()
+        .next()
+        .unwrap_or("")
+        .trim()
+        .to_string();
+    let s1 = selfhost();
+    let runtime = golden_runtime();
+    let std_dir = root().join("std");
+    let source_dir = golden_dir();
+    let walk = golden_programs();
+    let dir = fresh_dir("golden");
+    for side in ["s0", "s1"] {
+        std::fs::create_dir_all(dir.join(side)).expect("create a side's directory");
+    }
+    let threads = std::thread::available_parallelism()
+        .map(|n| n.get())
+        .unwrap_or(1)
+        .min(4);
+    let next = std::sync::atomic::AtomicUsize::new(0);
+    let failures = std::sync::Mutex::new(Vec::<String>::new());
+    // One build, of either side, within the walk's limit: its stdout, or
+    // why it failed, its stderr quoted.
+    let build = |program: &Path, args: &[&std::ffi::OsStr], what: &str| {
+        let mut command = std::process::Command::new(program);
+        command
+            .args(args)
+            .env("NOVA_RUNTIME_LIB", &runtime)
+            .env("NO_COLOR", "1")
+            .env("RUST_BACKTRACE", "0")
+            .env("RUST_LIB_BACKTRACE", "0");
+        match run_within_limit(command, None) {
+            Ok((stdout, _, Some(0))) => Ok(stdout),
+            Ok((_, stderr, status)) => Err(format!(
+                "{what} failed, exit status {status:?}{}",
+                quoted_stderr(&stderr)
+            )),
+            Err(why) => Err(format!("{what} {why}")),
+        }
+    };
+    std::thread::scope(|s| {
+        for _ in 0..threads {
+            s.spawn(|| loop {
+                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
+                let Some(name) = walk.get(i) else { break };
+                let source = source_dir.join(format!("{name}.nova"));
+                let s0_exe = dir.join("s0").join(name);
+                let s1_exe = dir.join("s1").join(name);
+                let tmp = dir.join(format!("{name}.tmp"));
+                std::fs::create_dir_all(&tmp).expect("a temporary directory");
+                let built = build(
+                    s1,
+                    &[
+                        "build".as_ref(),
+                        source.as_os_str(),
+                        "-o".as_ref(),
+                        s1_exe.as_os_str(),
+                        "--release".as_ref(),
+                        "--std".as_ref(),
+                        std_dir.as_os_str(),
+                    ],
+                    "`selfhost build`",
+                )
+                .and_then(|stdout| {
+                    let want = format!("built {}\n", s1_exe.display());
+                    if stdout == want {
+                        Ok(())
+                    } else {
+                        Err(format!("`selfhost build` printed {stdout:?}, not {want:?}"))
+                    }
+                })
+                .and_then(|()| {
+                    build(
+                        &nova(),
+                        &[
+                            "build".as_ref(),
+                            "--release".as_ref(),
+                            source.as_os_str(),
+                            "-o".as_ref(),
+                            s0_exe.as_os_str(),
+                        ],
+                        "`nova build --release`",
+                    )
+                })
+                .and_then(|_| run_golden(name, &s1_exe, &tmp))
+                .and_then(|()| same_binaries(&s0_exe, &s1_exe));
+                for side in [&s0_exe, &s1_exe] {
+                    let _ = std::fs::remove_file(side);
+                    for ext in ["ll", "o"] {
+                        let _ =
+                            std::fs::remove_file(side.with_file_name(format!("{name}.nova.{ext}")));
+                    }
+                }
+                let _ = std::fs::remove_dir_all(&tmp);
+                if let Err(why) = built {
+                    failures.lock().unwrap().push(format!("{name}: {why}"));
+                }
+            });
+        }
+    });
+    let _ = std::fs::remove_dir_all(&dir);
+    let failures = failures.into_inner().unwrap();
+    println!(
+        "s1 golden: {} passed, {} excluded, {version}, runtime {}",
+        walk.len() - failures.len(),
+        RELEASE_GOLDEN_EXCLUDED.len(),
+        runtime.display()
+    );
+    assert!(
+        failures.is_empty(),
+        "{} of {} golden programs failed as S1 built them:\n\n{}",
+        failures.len(),
+        walk.len(),
+        failures.join("\n\n")
+    );
+}
+
+/// CI's step "S1 builds the golden suite (Linux)" names, after `--exact`,
+/// exactly the tests this file marks `#[ignore = "needs clang; ..."]`
+/// (spec
+/// `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`
+/// §7): with `--exact`, a name that matches no test runs nothing and the
+/// step still passes, so a name misspelt or dropped there, or a test no
+/// longer ignored here, would go unseen. Both files are read as text, with
+/// CRLF line endings too, as a Windows checkout may leave them.
+#[test]
+fn the_ci_step_names_every_clang_test() {
+    const STEP: &str = "- name: S1 builds the golden suite (Linux)";
+    let path = root().join(".github").join("workflows").join("ci.yml");
+    let ci =
+        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
+    let lines: Vec<&str> = ci.lines().map(|l| l.trim_end_matches('\r')).collect();
+    let indent = |l: &str| l.len() - l.trim_start().len();
+    let at = lines
+        .iter()
+        .position(|l| l.trim() == STEP)
+        .unwrap_or_else(|| panic!("ci.yml has no step `{STEP}`"));
+    let run = lines[at + 1..]
+        .iter()
+        .take_while(|l| l.trim().is_empty() || indent(l) > indent(lines[at]))
+        .find_map(|l| l.trim().strip_prefix("run:"))
+        .unwrap_or_else(|| panic!("ci.yml's step `{STEP}` has no `run:` line"));
+    let words: Vec<&str> = run.split_whitespace().collect();
+    let exact = words
+        .iter()
+        .position(|w| *w == "--exact")
+        .unwrap_or_else(|| panic!("the step's `run:` line has no `--exact`: {run}"));
+    let mut names: Vec<&str> = words[exact + 1..]
+        .iter()
+        .take_while(|w| !w.starts_with('-'))
+        .copied()
+        .collect();
+    assert!(!names.is_empty(), "the step names no test: {run}");
+
+    // This file's top-level tests: each name, and its attributes.
+    let mut tests: Vec<(&str, Vec<&str>)> = Vec::new();
+    let mut attrs: Vec<&str> = Vec::new();
+    for line in include_str!("selfhost.rs")
+        .lines()
+        .map(|l| l.trim_end_matches('\r'))
+    {
+        if line.starts_with("///") {
+            continue;
+        }
+        if line.starts_with("#[") {
+            attrs.push(line);
+            continue;
+        }
+        if let Some(rest) = line.strip_prefix("fn ") {
+            if attrs.contains(&"#[test]") {
+                tests.push((rest.split('(').next().unwrap_or(rest), attrs.clone()));
+            }
+        }
+        attrs.clear();
+    }
+    for name in &names {
+        let Some((_, attrs)) = tests.iter().find(|(test, _)| test == name) else {
+            panic!("the step names `{name}`, which is no test in selfhost.rs");
+        };
+        assert!(
+            attrs.iter().any(|a| a.starts_with("#[ignore")),
+            "the step names `{name}`, which selfhost.rs does not mark #[ignore]"
+        );
+    }
+    let mut clang: Vec<&str> = tests
+        .iter()
+        .filter(|(_, attrs)| {
+            attrs
+                .iter()
+                .any(|a| a.starts_with("#[ignore = \"needs clang"))
+        })
+        .map(|(name, _)| *name)
+        .collect();
+    names.sort_unstable();
+    clang.sort_unstable();
+    assert_eq!(
+        names, clang,
+        "the step's names are not the tests selfhost.rs ignores as needing clang"
+    );
+}
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 5e5fbdf3..82f5d8ae 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -188,6 +188,31 @@ jobs:
         run: cargo build --locked --workspace
       - name: cargo test --test selfhost (N on the release runtime)
         run: cargo test --locked --workspace --all-features --no-fail-fast --test selfhost
+      # S1 builds the golden suite (spec
+      # docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md
+      # §6.2, §7): N's own `selfhost build` builds every program
+      # `release_golden_suite` walks, through `clang-18` and `cc`, on the
+      # release runtime the step's N unpacks; each runs as the Test job's
+      # walk runs it, and each binary must equal S0 `--release`'s byte for
+      # byte. On the Linux leg alone, after the step above, so that step's
+      # time and its lines are untouched; the install sits right before it,
+      # the Test job's step, asserting the same version (ADR 0048).
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
+      - name: S1 builds the golden suite (Linux)
+        if: runner.os == 'Linux'
+        env:
+          NOVA_REQUIRE_CLANG: "1"
+          NOVA_CLANG: clang-18
+        run: cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --exact s1_builds_the_golden_suite
       - name: N's own tests on the debug runtime
         env:
           NOVA_RUNTIME_LIB: ${{ github.workspace }}/target/debug/${{ matrix.runtime_lib }}
diff --git a/selfhost/src/driver.nova b/selfhost/src/driver.nova
new file mode 100644
index 00000000..64a19908
--- /dev/null
+++ b/selfhost/src/driver.nova
@@ -0,0 +1,555 @@
+// N's driver (5.5b, spec
+// docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md
+// §3): `selfhost build`, a port of S0's file-mode `nova build --release`
+// (nova-cli's `build`, nova-driver's `build_program_release`, `link.rs`'s
+// `compile_ir_to_object`, `link_executable` and `link_cc`), and the
+// loading of std that every command of N's shares.
+//
+//   selfhost build FILE [-o OUT | --output OUT] [--release] --std DIR
+//
+// FILE is loaded as `selfhost mir` loads it, checked and lowered natively;
+// the emitter's text is written to `<OUT>.nova.ll`, `clang -O2 -c` makes
+// `<OUT>.nova.o` of it, and `cc` links that with the runtime into OUT,
+// FILE's stem when there is no `-o`. On success the object and the `.ll`
+// are gone, `built OUT` is on stdout and the exit code is 0. A failure
+// prints S0's words in anyhow's layout and exits 1; a command line it does
+// not take prints its usage line and exits 2. Without `--release` it does
+// the same, `-O2` too (decision 6), where S0 would use Cranelift.
+
+import builtins
+import diagnostic
+import emit_ll
+import loader
+import mir
+import mono
+import resolver
+import splice
+import typeck
+import typeck_body
+
+/// `selfhost build`'s usage line, which every command line it does not
+/// take prints (spec 5.5b §3.1).
+pub const BUILD_USAGE: String = "usage: selfhost build FILE [-o OUT] [--release] --std DIR"
+
+// S0's words when neither `clang` nor `llc` can be spawned
+// (`compile_ir_to_object`); N looks for `clang` alone, and says the same.
+const NO_TOOLCHAIN: String = "no LLVM toolchain found for `--release`: install LLVM (so `clang` or `llc` is on PATH), or set NOVA_CLANG / NOVA_LLC to one"
+
+/// What `selfhost build`'s command line names: the root, the output, the
+/// std directory, and whether `--release` was given.
+pub record BuildArgs {
+    pub file: String,
+    pub output: String,
+    pub std_dir: String,
+    pub release: Bool,
+}
+
+/// `selfhost build`'s command line, from `argv`'s third argument on
+/// (spec 5.5b §3.1): each option once, in any order, `-o OUT`, `--output
+/// OUT`, `--output=OUT`, `--release`, `--std DIR`, `--std=DIR`, and one
+/// FILE; `--` ends the options. `None` for anything else: a second FILE or
+/// option, an unknown option, `--release=…`, no FILE, no `--std`, or an
+/// option without its value, which an empty one is, and a separate one
+/// that begins with `-` but is not `-`, as clap takes that for an option.
+/// The output is FILE's stem, `out` when it has none, when no `-o` names
+/// one.
+pub fn build_args(argv: Vec<String>) -> Option<BuildArgs> {
+    let mut file: Option<String> = None
+    let mut output: Option<String> = None
+    let mut std_dir: Option<String> = None
+    let mut release = false
+    let mut only_files = false
+    let mut i = 2
+    while i < argv.len() {
+        let a = argv.at(i)
+        i += 1
+        if only_files || a == "-" || !a.starts_with("-") {
+            if file.is_some() { return None }
+            file = Some(a)
+            continue
+        }
+        if a == "--" {
+            only_files = true
+            continue
+        }
+        if a == "--release" {
+            if release { return None }
+            release = true
+            continue
+        }
+        let mut is_std = false
+        let mut value = ""
+        if a.starts_with("--output=") {
+            value = a.byte_slice(9, a.byte_len())
+        } else if a.starts_with("--std=") {
+            is_std = true
+            value = a.byte_slice(6, a.byte_len())
+        } else if a == "-o" || a == "--output" || a == "--std" {
+            is_std = a == "--std"
+            if i < argv.len() {
+                value = argv.at(i)
+                i += 1
+                if value.starts_with("-") && value != "-" { return None }
+            }
+        } else {
+            return None
+        }
+        if value == "" { return None }
+        if is_std {
+            if std_dir.is_some() { return None }
+            std_dir = Some(value)
+        } else {
+            if output.is_some() { return None }
+            output = Some(value)
+        }
+    }
+    let root = match file {
+        Some(f) => f
+        None => return None
+    }
+    let dir = match std_dir {
+        Some(d) => d
+        None => return None
+    }
+    let out = match output {
+        Some(o) => o
+        None => file_stem_or(root, "out")
+    }
+    Some(BuildArgs { file: root, output: out, std_dir: dir, release: release })
+}
+
+/// `selfhost build` (spec 5.5b §3.2), in S0's order: the command line;
+/// std read from its directory; FILE loaded, checked and lowered; the
+/// `.ll` emitted and written beside the output; `clang`, the runtime and
+/// `cc` (`compile_and_link`); the object removed whatever happened, and on
+/// success the `.ll` too, and `built OUT` printed.
+pub async fn build_command(argv: Vec<String>) {
+    let args = match build_args(argv) {
+        Some(a) => a
+        None => {
+            eprintln(BUILD_USAGE)
+            exit(2)
+            return
+        }
+    }
+    let std_set = match load_std(args.std_dir, CompileTargetNative, false).await {
+        Ok(s) => s
+        Err(message) => {
+            eprintln(message)
+            exit(2)
+            return
+        }
+    }
+    let path = args.file
+    let program = match compile_root(path, std_set).await {
+        CompiledModule(p, _) => p
+        CompiledErrors(diagnostics) => {
+            fail(could_not_compile(error_count(diagnostics)), [])
+            return
+        }
+        CompiledIoError(e) => {
+            fail("failed to open ${path}", [e.message])
+            return
+        }
+        CompiledRefused(why) => {
+            fail("selfhost build cannot read ${path}: ${why}", [])
+            return
+        }
+    }
+    let ir = match compile_ir(program) {
+        Ok(text) => text
+        Err(words) => {
+            fail("internal LLVM IR generation error (this is a compiler bug)", [words])
+            return
+        }
+    }
+    let out = args.output
+    let ll = intermediate(out, "ll")
+    match write_string(ll, ir).await {
+        Ok(_) => ()
+        Err(e) => {
+            fail("failed to write ${ll}", [e.message])
+            return
+        }
+    }
+    let obj = intermediate(out, "o")
+    let linked = compile_and_link(ll, obj, out).await
+    let _ = remove_file(obj).await
+    match linked {
+        Ok(_) => {
+            let _ = remove_file(ll).await
+            println("built ${out}")
+        }
+        Err(causes) => fail("release build failed; generated LLVM IR left at ${ll}", causes)
+    }
+}
+
+/// What `compile_root` made of a root: its lowered module and every
+/// diagnostic, warnings, on the way; the diagnostics of the stage that
+/// stopped it; the error that kept it from being read; or why N refuses
+/// its package's manifest.
+pub type Compiled =
+    | CompiledModule(MirModule, Vec<Diagnostic>)
+    | CompiledErrors(Vec<Diagnostic>)
+    | CompiledIoError(IoError)
+    | CompiledRefused(String)
+
+/// The root `path` loaded as `selfhost mir` loads it, a loose program or a
+/// module of the package whose `src/` or `tests/` it is directly in, its
+/// `@test` functions stripped, then resolved, checked and lowered natively
+/// in full, std's bodies with it, as S0's `build` does
+/// (`FrontendContext::check`, then `lower_module_with`): each stage's
+/// diagnostics after the last's, the first stage with an error the last.
+pub async fn compile_root(path: String, std_set: StdSet) -> Compiled {
+    let mut graph: Option<Graph> = None
+    let mut kind = RootLoose
+    match package_of(path).await {
+        Some((dir, k)) => {
+            match read_graph(dir).await {
+                Ok(g) => graph = Some(g)
+                Err(why) => return CompiledRefused(why)
+            }
+            kind = k
+        }
+        None => ()
+    }
+    let load = match load_program(graph, path, kind).await {
+        Ok(l) => l
+        Err(e) => return CompiledIoError(e)
+    }
+    let modules = load.modules
+    let mut diagnostics: Vec<Diagnostic> = Vec::new()
+    for d in load.diagnostics.iter() { diagnostics.push(d) }
+    if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics) }
+    strip_test_functions(modules, false)
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
+    let resolved = resolve_program_for(CompileTargetNative, sources, std_set)
+    for d in resolved.diagnostics.iter() { diagnostics.push(d) }
+    if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics) }
+    let (labels, texts) = labels_and_texts(sources, std_set)
+    let mut tables = check_items(resolved.items, resolved.definitions, labels, texts)
+    tables.check_bodies(resolved.newline_splits)
+    for d in tables.diagnostics.iter() { diagnostics.push(d) }
+    if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics) }
+    let program = full_module(tables)
+    keep_the_entry_main(program, resolved.definitions)
+    match lower_module_with(program, false) {
+        Ok(lowered) => CompiledModule(lowered, diagnostics)
+        Err(errors) => {
+            for d in errors.iter() { diagnostics.push(d) }
+            CompiledErrors(diagnostics)
+        }
+    }
+}
+
+// How many of `diagnostics` are errors.
+fn error_count(diagnostics: Vec<Diagnostic>) -> Int {
+    let mut n = 0
+    for d in diagnostics.iter() { if d.is_error() { n += 1 } }
+    n
+}
+
+// S0's last line for a build that did not compile (nova-cli's
+// `could_not_compile`).
+fn could_not_compile(errors: Int) -> String {
+    let s = if errors == 1 { "" } else { "s" }
+    "could not compile due to ${errors} previous error${s}"
+}
+
+/// An intermediate file's path beside `output` (nova-driver's
+/// `intermediate`): its file name, `out` when it has none, then `.nova.`
+/// and `ext`, so that it never names the output itself: `-o out.ll` gives
+/// `out.ll.nova.ll`.
+pub fn intermediate(output: String, ext: String) -> String {
+    let name = file_name(output)
+    if name == "" { return path_join(output, "out.nova.${ext}") }
+    let file = "${name}.nova.${ext}"
+    match path_parent(output) {
+        Some(dir) => path_join(dir, file)
+        None => file
+    }
+}
+
+// `clang`, the runtime and `cc` (spec 5.5b §3.3), as S0's
+// `compile_ir_to_object` and `link_executable` run them: `clang -O2 -c LL
+// -o OBJ`, `$NOVA_CLANG` or `clang`, when it can be spawned; then the
+// runtime, looked for only now, as S0 does; then `$CC` or `cc`, `-o OUT
+// OBJ RUNTIME -lpthread -ldl -lm`. The causes of the first failure, in
+// S0's words.
+async fn compile_and_link(ll: String, obj: String, out: String) -> Result<(), [String]> {
+    let clang = env_or("NOVA_CLANG", "clang")
+    if !tool_available(clang) { return Err([NO_TOOLCHAIN]) }
+    run_tool(clang, ["-O2", "-c", ll, "-o", obj], "clang", "clang")?
+    let runtime = find_runtime().await?
+    let cc = env_or("CC", "cc")
+    run_tool(cc, ["-o", out, obj, runtime, "-lpthread", "-ldl", "-lm"], "linker", "linking")
+}
+
+// The variable `name`'s value, or `fallback` when it is not set
+// (`std::env::var(name).unwrap_or_else`).
+fn env_or(name: String, fallback: String) -> String {
+    match env(name) {
+        Some(value) => value
+        None => fallback
+    }
+}
+
+// Whether `tool` can be spawned, whatever its status (`tool_available`:
+// `<tool> --version`).
+fn tool_available(tool: String) -> Bool {
+    match run(tool, ["--version"]) {
+        Ok(_) => true
+        Err(_) => false
+    }
+}
+
+// `program` run with `args` (`run_tool`, `run_linker`): what it printed is
+// dropped when it succeeds; it cannot be spawned, two causes, `failed to
+// spawn SPAWN: "PROGRAM"` and the system's words; it fails, one, `FAILED
+// failed (exit status: N):` and its stdout and stderr, each trimmed, on
+// lines of their own.
+fn run_tool(
+    program: String,
+    args: [String],
+    spawn: String,
+    failed: String,
+) -> Result<(), [String]> {
+    match run(program, args) {
+        Err(e) => Err(["failed to spawn ${spawn}: ${debug_quoted(program)}", e.message])
+        Ok(output) => {
+            if output.status == 0 { return Ok(()) }
+            let stdout = tool_text(output.stdout).trim()
+            let stderr = tool_text(output.stderr).trim()
+            Err(["${failed} failed (exit status: ${output.status}):\n${stdout}\n${stderr}"])
+        }
+    }
+}
+
+/// `text` quoted as Rust's `{:?}` quotes a program's name, for the names
+/// N meets: in `"`, with `"` and `\` escaped.
+pub fn debug_quoted(text: String) -> String {
+    let mut out = StringBuilder::new()
+    out.push("\"")
+    let n = text.byte_len()
+    let mut start = 0
+    let mut i = 0
+    while i < n {
+        let b = text.byte_at(i)
+        if b == 34 || b == 92 {
+            out.push(text.byte_slice(start, i))
+            out.push("\\")
+            start = i
+        }
+        i += 1
+    }
+    out.push(text.byte_slice(start, n))
+    out.push("\"")
+    out.to_string()
+}
+
+/// A tool's output as text (`String::from_utf8_lossy`): itself when it is
+/// UTF-8; else its ASCII bytes kept and U+FFFD for each other byte.
+pub fn tool_text(bytes: Bytes) -> String {
+    match bytes.to_string() {
+        Some(text) => text
+        None => {
+            let mut kept: Vec<Int> = Vec::new()
+            for b in bytes.to_ints().iter() {
+                if b < 128 {
+                    kept.push(b)
+                } else {
+                    kept.push(239)
+                    kept.push(191)
+                    kept.push(189)
+                }
+            }
+            bytes_from_ints(kept.to_array()).to_string().unwrap()
+        }
+    }
+}
+
+// The runtime library (spec 5.5b §3.3; `find_runtime_lib` without an
+// embedded runtime): `$NOVA_RUNTIME_LIB`, which must exist; else
+// `libnova_runtime.a` in the executable's directory or one of the two
+// above it (`beside_exe`, without its `build/` rule for cargo's test
+// binaries); else S0's words, with N's path.
+async fn find_runtime() -> Result<String, [String]> {
+    match env("NOVA_RUNTIME_LIB") {
+        Some(lib) => {
+            if exists(lib).await { return Ok(lib) }
+            return Err(["NOVA_RUNTIME_LIB points to ${lib}, which does not exist"])
+        }
+        None => ()
+    }
+    let exe = match current_exe() {
+        Ok(path) => path
+        Err(e) => return Err(["locating the nova executable", e.message])
+    }
+    let name = "libnova_runtime.a"
+    let mut dir = path_parent(exe)
+    let mut tried = 0
+    while tried < 3 {
+        let d = match dir {
+            Some(d) => d
+            None => break
+        }
+        let candidate = path_join(d, name)
+        if exists(candidate).await { return Ok(candidate) }
+        dir = path_parent(d)
+        tried += 1
+    }
+    Err(
+        [
+            "could not find the Nova runtime library (${name}) near ${exe}, and this nova does not carry one. Install nova with the release profile, which is `cargo install`'s default, or build it with NOVA_EMBED_RUNTIME=1; in a checkout, `cargo build -p nova-runtime` puts the library beside it. NOVA_RUNTIME_LIB can also name one.",
+        ],
+    )
+}
+
+/// An error as S0's `main` prints it, anyhow's layout (spec 5.5b §2 item
+/// 3): `Error: HEAD`; then, when there are causes, a blank line, `Caused
+/// by:` and each cause's lines indented four spaces, empty ones too, the
+/// causes numbered `0:`, `1:` when there are several, a numbered cause's
+/// later lines indented seven.
+pub fn failure_text(head: String, causes: [String]) -> String {
+    let mut out = StringBuilder::new()
+    out.push("Error: ${head}")
+    if causes.len() > 0 {
+        out.push("\n\nCaused by:")
+        let numbered = causes.len() > 1
+        let mut n = 0
+        for cause in causes.iter() {
+            out.push("\n")
+            let mut first = true
+            for line in cause.split("\n").iter() {
+                if first {
+                    if numbered {
+                        let number = "${n}"
+                        out.push(" ".repeat(5 - number.byte_len()))
+                        out.push("${number}: ")
+                    } else {
+                        out.push("    ")
+                    }
+                    first = false
+                } else {
+                    out.push(if numbered { "\n       " } else { "\n    " })
+                }
+                out.push(line)
+            }
+            n += 1
+        }
+    }
+    out.to_string()
+}
+
+// `failure_text` on stderr, then the exit code 1, as S0's `main` exits
+// with an error.
+fn fail(head: String, causes: [String]) {
+    eprintln(failure_text(head, causes))
+    exit(1)
+}
+
+// ---------------------------------------------------------------------------
+// std, as every command of N's reads it
+// ---------------------------------------------------------------------------
+
+// What every program's resolution reads for `target`, under test mode
+// when `test`: the target's std modules, each read from under `dir` and
+// parsed once, labelled as the driver registers them (`<std/core>`), with
+// `std/test` after them under test mode, as `nova test` registers it;
+// `STD_MODULES`, parsed, which the WASM target's `missing` reads (a file
+// both lists name is parsed once); and the builtin table.
+pub async fn load_std(dir: String, target: CompileTarget, test: Bool) -> Result<StdSet, String> {
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
+    let mut out: Vec<StdModule> = Vec::new()
+    for (name, rel) in list.iter() {
+        match parsed.get(rel) {
+            Some(m) => {
+                out.push(m)
+                continue
+            }
+            None => ()
+        }
+        let path = "${dir}/${rel}"
+        match read_to_string(path).await {
+            Ok(source) => {
+                let m = std_module(name, std_label(name), source)
+                let _ = parsed.insert(rel, m)
+                out.push(m)
+            }
+            Err(e) => return Err("error: cannot read std's ${path}: ${io_kind_name(e.kind)}")
+        }
+    }
+    Ok(out)
+}
+
+// Each module's file label, the program's `sources` then std's modules
+// that parsed, as the resolver numbers them, and each program module's
+// source text, for a fix's edit; std's have none.
+pub fn labels_and_texts(
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
+// The kind's name, as S0's dump prints it.
+pub fn io_kind_name(kind: IoErrorKind) -> String {
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
diff --git a/selfhost/src/loader.nova b/selfhost/src/loader.nova
index 7a58e892..94c4ee0e 100644
--- a/selfhost/src/loader.nova
+++ b/selfhost/src/loader.nova
@@ -770,9 +770,15 @@ async fn is_file(path: String) -> Bool {
 
 /// A module's name from its path, its file's stem, or `main` when it has
 /// none (`FrontendContext::module_name`).
-pub fn module_name(path: String) -> String {
+pub fn module_name(path: String) -> String { file_stem_or(path, "main") }
+
+/// The stem of `path`'s file name (`Path::file_stem`), or `fallback` when
+/// it has none: the name before its last `.`, unless that `.` begins it,
+/// as `.nova`'s stem is `.nova`. `module_name`'s fallback is `main`, and
+/// the driver's default output's `out` (spec 5.5b §3.2).
+pub fn file_stem_or(path: String, fallback: String) -> String {
     let name = file_name(path)
-    if name == "" || name == ".." { return "main" }
+    if name == "" || name == ".." { return fallback }
     let n = name.byte_len()
     let mut dot = -1
     let mut i = n - 1
@@ -786,10 +792,10 @@ pub fn module_name(path: String) -> String {
     if dot <= 0 { name } else { name.byte_slice(0, dot) }
 }
 
-// The last part of `path` (`Path::file_name`), "" for none: a path's
-// last component, past any `/` or `.` component that ends it, unless it is
-// `..`.
-fn file_name(path: String) -> String {
+/// The last part of `path` (`Path::file_name`), "" for none: a path's
+/// last component, past any `/` or `.` component that ends it, unless it
+/// is `..`.
+pub fn file_name(path: String) -> String {
     let lo = before_body(path)
     let end = trim_back(path, lo, path.byte_len())
     if end == lo { return "" }
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index 7ab60a42..47732fba 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -47,6 +47,13 @@
 //                             lowered root's `.ll` after its part (spec
 //                             docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md
 //                             §4, §5)
+//   selfhost build FILE [-o OUT] [--release] --std DIR
+//                             builds FILE as S0's `nova build --release
+//                             FILE -o OUT` does: its `.ll` written beside
+//                             OUT, `clang` and `cc` run on it, and `built
+//                             OUT` printed (spec
+//                             docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md
+//                             §3, driver.nova)
 //
 // A file with lex or parse errors is dumped with them, and the exit code
 // stays 0. A file that cannot be read prints `IOERR` and its kind after its
@@ -57,6 +64,7 @@ import ast_dump
 import async_lower
 import builtins
 import diagnostic
+import driver
 import dump
 import emit_ll
 import emit_ll_tests
@@ -111,6 +119,10 @@ async fn main() {
         mir_command(argv).await
         return
     }
+    if command == "build" {
+        build_command(argv).await
+        return
+    }
     // No file is refused, as S0's clap refuses it.
     let tokens = command == "tokens"
     if argv.len() < 3 || (!tokens && command != "ast") {
@@ -498,59 +510,6 @@ fn ll_part(program: MirModule) -> String {
 // kept its root from being read, `IOERR`; or why N refuses its package.
 type Dumped = | DumpedText(String) | DumpedIoError(IoError) | DumpedRefused(String)
 
-// What every program's resolution reads for `target`, under test mode
-// when `test`: the target's std modules, each read from under `dir` and
-// parsed once, labelled as the driver registers them (`<std/core>`), with
-// `std/test` after them under test mode, as `nova test` registers it;
-// `STD_MODULES`, parsed, which the WASM target's `missing` reads (a file
-// both lists name is parsed once); and the builtin table.
-async fn load_std(dir: String, target: CompileTarget, test: Bool) -> Result<StdSet, String> {
-    let mut parsed: Map<String, StdModule> = Map::new()
-    let natives = read_std_modules(dir, STD_MODULES, parsed).await?
-    let mut modules = match target {
-        CompileTargetNative => natives
-        CompileTargetWasm => read_std_modules(dir, WASM_STD_MODULES, parsed).await?
-    }
-    if test {
-        let mut with_test: Vec<StdModule> = Vec::new()
-        for m in modules.iter() { with_test.push(m) }
-        for m in read_std_modules(dir, [STD_TEST_MODULE], parsed).await?.iter() {
-            with_test.push(m)
-        }
-        modules = with_test
-    }
-    Ok(StdSet { modules: modules, natives: natives, builtins: builtin_table() })
-}
-
-// The std modules `list` names, in order, each read from under `dir` and
-// parsed, or taken from `parsed`, where each is kept by its file.
-async fn read_std_modules(
-    dir: String,
-    list: [(String, String)],
-    mut parsed: Map<String, StdModule>,
-) -> Result<Vec<StdModule>, String> {
-    let mut out: Vec<StdModule> = Vec::new()
-    for (name, rel) in list.iter() {
-        match parsed.get(rel) {
-            Some(m) => {
-                out.push(m)
-                continue
-            }
-            None => ()
-        }
-        let path = "${dir}/${rel}"
-        match read_to_string(path).await {
-            Ok(source) => {
-                let m = std_module(name, std_label(name), source)
-                let _ = parsed.insert(rel, m)
-                out.push(m)
-            }
-            Err(e) => return Err("error: cannot read std's ${path}: ${io_kind_name(e.kind)}")
-        }
-    }
-    Ok(out)
-}
-
 // The resolution dump of the program whose root is `path`
 // (nova-driver's `ResolutionDumps::dump`): `check`'s pipeline up to the
 // resolver, or under test mode `nova test`'s. A root directly in a
@@ -716,28 +675,6 @@ fn hir_of(
     Some(DumpedText(dump_hir_program(target, test, Some((resolved, tables)), diagnostics)))
 }
 
-// Each module's file label, the program's `sources` then std's modules
-// that parsed, as the resolver numbers them, and each program module's
-// source text, for a fix's edit; std's have none.
-fn labels_and_texts(
-    sources: Vec<ModuleSource>,
-    std_set: StdSet,
-) -> (Vec<String>, Vec<Option<String>>) {
-    let mut labels: Vec<String> = Vec::new()
-    let mut texts: Vec<Option<String>> = Vec::new()
-    for m in sources.iter() {
-        labels.push(m.label)
-        texts.push(m.text)
-    }
-    for s in std_set.modules.iter() {
-        if s.file.is_some() {
-            labels.push(s.label)
-            texts.push(None)
-        }
-    }
-    (labels, texts)
-}
-
 // Name each test of a module of a package's `tests/`
 // `<file stem>::<function>` (the driver's `name_tests`), as `nova test`'s
 // inventory names it.
@@ -757,17 +694,3 @@ fn name_tests(tests: Vec<TestFn>, definitions: Definitions, modules: Vec<Loaded>
         }
     }
 }
-
-// The kind's name, as S0's dump prints it.
-fn io_kind_name(kind: IoErrorKind) -> String {
-    match kind {
-        NotFound => "NotFound"
-        PermissionDenied => "PermissionDenied"
-        AlreadyExists => "AlreadyExists"
-        InvalidData => "InvalidData"
-        Interrupted => "Interrupted"
-        TimedOut => "TimedOut"
-        ConnectionRefused => "ConnectionRefused"
-        Other => "Other"
-    }
-}
````

### Task 2: `build`'s refusals and errors, compared with S0's

Spec §3.2 to §3.6, §4.1, §4.2, §6.3, §9, §11 item 2; decisions 3, 4, 8b
and 11.

**Files:**
- Create: `selfhost/src/driver_tests.nova`
- Modify: `crates/nova-cli/tests/selfhost.rs`, `selfhost/src/main.nova`,
  `.github/workflows/ci.yml`, `selfhost/src/driver.nova`

- [ ] **Step 1: The tests.** `patch_block t2-tests`:
  - **`selfhost.rs`, the probe harness** (temporary directories
    `nova-selfhost-probe`, `nova-selfhost-probe-clang`,
    `nova-selfhost-alone`): `PROBE_INPUTS` (`warn.nova`, `two.nova`,
    `clean.nova`, `frem.nova`, written into each case's own directory
    with the directory `dir.nova`), `CC_ARGS` (`#[cfg(unix)]`, as its
    use), `N_REFUSES_WINDOWS`, `N_BUILD_USAGE`, `Expect`, `Probe`,
    `ProbeVar`, `ProbeRun`; `extracted_stderr` (the header lines, the
    loci re-indented to two spaces, `\` read as `/` on Windows, and
    everything from the first `Error: ` line) and its test
    `the_extraction_keeps_the_headers_the_loci_and_the_error`;
    `PROBES`, `build`'s 15: compile errors, a missing FILE, a directory,
    `-o nodir/x`, no `clang` (with `-o`, the default name, a warning, and
    no runtime), float `%`; four usage errors; and N alone without `--std`
    or FILE; `probe_dir`, `files_under`, `probe_side`, `shown`,
    `probe_difference`, `probe_vars`, `run_probes`; and
    `n_build_and_check_answer_as_s0_does`, in the main step on every
    system (on Windows N's refusal is expected for each `build` case);
  - **the clang probe test,** `n_build_answers_as_s0_does_with_clang`,
    `#[ignore]`d. `CLANG_PROBES`' 10 cases, each against S0: built with
    `-o`, with the default name, and with a warning; `NOVA_CLANG=false`,
    and `NOVA_CLANG=false` with no runtime; `CC=false`; `CC` missing;
    `cc`'s command line through `CC_ARGS`; `NOVA_RUNTIME_LIB` missing,
    and a text file. Then three cases on N alone: no runtime (N copied
    alone, against `beside_exe`'s words); a runtime two directories up
    (`9304d751`, decision 16); and `build` without `--release` giving the
    binary `--release` gives. On that binary, the golden harness's three
    refusals: a wrong stdout for `arrays` and for `fs_already_exists`,
    and a binary one byte off. Each case counts once as it runs
    (`e97c759a`), so the report's "N of M" is out of the cases that ran,
    16 when all run;
  - **`driver_tests.nova`** (new, imported by `main.nova`): N's eleven
    `@test`s, `check`'s command line excepted: the intermediate's name;
    the stem; the Windows shapes; the command line; anyhow's layout; line
    and column; the loci; a diagnostic's lines; the `{:?}` quoting; the
    status line; the U+FFFD fallback;
  - **`ci.yml`:** the step's line names
    `n_build_answers_as_s0_does_with_clang` too.
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. Then `cargo test --locked --workspace --test selfhost
  --no-run`, and `SH` (4 s). Expected: `26 passed; 44 failed; 2
  ignored`. N does not build, so every test that builds N fails, 44:
  - 43 of them panic with `nova build of selfhost/ failed:` and N's own
    `nova build`'s two errors, `error[E0001]: cannot find function
    `line_and_column` in this scope` at `src/driver_tests.nova:26:26` and
    `error[E0001]: cannot find function `loci` in this scope` at
    `src/driver_tests.nova:33:26`, then `Error: could not compile due to
    2 previous errors`;
  - `the_selfhost_tests_pass` fails on N's `nova test`, which builds
    `driver_tests.nova`'s `@test`s too: `Error: could not compile due to
    19 previous errors`, `cannot find function` for `windows_shape` (9),
    `diagnostic_lines` (3), `status_cause` (3), `line_and_column` (1)
    and `loci` (1), and `cannot find record `SourceFile`` (2).
  The 26 that pass run S0 alone or no N,
  `the_ci_step_names_every_clang_test` among them.
- [ ] **Step 3: The code.** `patch_block t2-code`, `driver.nova`:
  - **the Windows refusal:** `WINDOWS_REFUSAL`, `windows_shape` (an ASCII
    letter and `:`, or `\\`, at the path's start), `on_windows` (Unix
    when `current_exe` fails); `build_command` refuses after the command
    line, before std or FILE, exit 1;
  - **the diagnostics `build` prints** (spec §4.1, §4.2): `SourceFile`,
    the files `Compiled` carries; `diagnostic_lines` (`SEVERITY[CODE]:
    MESSAGE`, one `  ┌─ PATH:LINE:COL` per file), `severity_name`,
    `loci` (one per file, in first-named order, its earliest primary
    label, else its earliest secondary), `line_and_column` (1 + the `\n`
    bytes before; 1 + the bytes that are not continuation bytes),
    `source_file`, `with_std_files` (std's text read from `--std`,
    `<std/NAME>`), `print_diagnostics`; `build_command` prints errors
    before `could not compile due to N previous error(s)`, and warnings;
  - **`status_cause`**, a failed tool's cause, out of `run_tool`.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --no-run`, then `S` in three runs
    (116, 1 and 167 s). Expected: `2837 1 10`.
  - `cargo test --locked --workspace --test selfhost --no-run`, then `SH`
    (158 s). Expected: `70 passed; 0 failed; 2 ignored`.
  - `C s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang`
    (69 s). Expected: `2 passed`.
  - `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 1` (7 s).
    Expected: `test result: ok. 119 passed; 0 failed; 0 trapped; 119
    total`, the base's 108 and the eleven of Step 1.
- [ ] **Step 5: The checks,** as the conventions list them (the
  mimicked Windows clippy turns off `3` items).
- [ ] **Step 6: Commit** `selfhost: build's refusals and errors, compared
      with S0's on the probes`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 82f5d8ae..859bf886 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -212,7 +212,7 @@ jobs:
         env:
           NOVA_REQUIRE_CLANG: "1"
           NOVA_CLANG: clang-18
-        run: cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --exact s1_builds_the_golden_suite
+        run: cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang
       - name: N's own tests on the debug runtime
         env:
           NOVA_RUNTIME_LIB: ${{ github.workspace }}/target/debug/${{ matrix.runtime_lib }}
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index 424faa0a..c507889c 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -113,7 +113,12 @@
 //! job on its Linux leg, builds every golden program `release_golden_suite`
 //! walks with it, runs each with the tables both files include
 //! (`golden/shared.rs`), and holds each binary equal to S0 `build
-//! --release`'s, byte for byte.
+//! --release`'s, byte for byte. On a probe set (§6.3) `build` answers as
+//! S0's does: the same stdout, exit code and files left behind, and N's
+//! stderr S0's as [`extracted_stderr`] extracts it;
+//! [`n_build_and_check_answer_as_s0_does`] here, on every system, with no
+//! `clang`, and [`n_build_answers_as_s0_does_with_clang`], ignored here and
+//! run beside the golden walk.
 //!
 //! `NOVA_SELFHOST_FUZZ=SEED:COUNT` runs a longer search locally, never in
 //! CI. Every input that finds a difference becomes a fixed case.
@@ -7154,3 +7159,881 @@ fn the_ci_step_names_every_clang_test() {
         "the step's names are not the tests selfhost.rs ignores as needing clang"
     );
 }
+
+/// The inputs of the probe set (spec
+/// `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`
+/// §6.3), each written into every case's directory: a path relative to it
+/// and its text. `dir.nova` is a directory, made beside them.
+const PROBE_INPUTS: &[(&str, &str)] = &[
+    // E0021 alone: a warning.
+    (
+        "warn.nova",
+        "fn main() {\n    let v = 1\n    match v {\n        _ => println(\"a\")\n        \
+         1 => println(\"b\")\n    }\n}\n",
+    ),
+    // Two errors, the second on line 10.
+    (
+        "two.nova",
+        "fn main() {\n    let a: Int = true\n    println(\"${a}\")\n\n\n\n\n\n\n    \
+         let b: String = 5\n    println(b)\n}\n",
+    ),
+    // Clean.
+    ("clean.nova", "fn main() {\n    println(\"clean\")\n}\n"),
+    // Float `%`: the emitter refuses it.
+    (
+        "frem.nova",
+        "fn main() {\n    let a = 5.5\n    let b = 2.0\n    println(\"${a % b}\")\n}\n",
+    ),
+];
+
+/// A `cc` that writes its arguments, one a line, to `cc-args.txt` in its
+/// working directory, then runs `cc` with them: a probe's `CC`, made
+/// executable in every case's directory on Unix, so that `cc`'s command
+/// line is left behind and compared.
+#[cfg(unix)]
+const CC_ARGS: &str = "#!/bin/sh\nprintf '%s\\n' \"$@\" > cc-args.txt\nexec cc \"$@\"\n";
+
+/// N's refusal to build on Windows (spec 5.5b §3.5).
+const N_REFUSES_WINDOWS: &str = "Error: selfhost build links on Linux and macOS only\n";
+
+/// N's usage line for `build` (spec 5.5b §3.1).
+const N_BUILD_USAGE: &str = "usage: selfhost build FILE [-o OUT] [--release] --std DIR\n";
+
+/// How a probe's two sides are compared.
+#[derive(Clone, Copy, PartialEq, Eq)]
+enum Expect {
+    /// Stdout, the exit code, N's stderr against S0's extracted
+    /// ([`extracted_stderr`]), and what each left behind, byte for byte.
+    Same,
+    /// A usage error: both exit 2, print nothing on stdout and leave
+    /// nothing; N prints this line. clap's text is not matched (§3.6).
+    Usage(&'static str),
+}
+
+/// One case of the probe set: its name; S0's arguments, `None` where N
+/// alone is run (no `--std` or no FILE, which S0 has no form of); N's,
+/// `--std DIR` added to them when `std` is set; the variables both sides
+/// get, a value naming a file of the case's directory by its name there;
+/// whether it builds, which N refuses on Windows; and how it is compared.
+struct Probe {
+    name: &'static str,
+    s0: Option<&'static [&'static str]>,
+    n: &'static [&'static str],
+    std: bool,
+    vars: &'static [(&'static str, ProbeVar)],
+    links: bool,
+    expect: Expect,
+}
+
+/// A variable's value for a probe: a file in the case's directory, by its
+/// name there, absolute; or a value as it is.
+#[derive(Clone, Copy)]
+enum ProbeVar {
+    InDir(&'static str),
+    Text(&'static str),
+}
+
+/// What one side did on one probe: its stdout, stderr and exit code, and
+/// the files it left in the case's directory, by their paths there with
+/// `/` between their parts, and their bytes.
+struct ProbeRun {
+    stdout: Vec<u8>,
+    stderr: String,
+    code: Option<i32>,
+    left: std::collections::BTreeMap<String, Vec<u8>>,
+}
+
+/// S0's stderr as N's is compared with it (spec 5.5b §4.3): the lines that
+/// begin `error[`, `warning[`, `note[` or `help[`; the lines whose first
+/// non-blank text is `┌─ `, re-indented to two spaces, a `\` of their path
+/// read as `/` on `windows`; and, from the first line that begins `Error: `
+/// on, every line as it is, anyhow's error and its causes. A diagnostic's
+/// snippet, notes and help, and the blank lines between, are left out. A
+/// message holding a line break would lose its second line here, and N's
+/// stderr, which keeps it, would then differ.
+fn extracted_stderr(stderr: &str, windows: bool) -> String {
+    let lines: Vec<&str> = stderr.lines().collect();
+    let tail = lines
+        .iter()
+        .position(|line| line.starts_with("Error: "))
+        .unwrap_or(lines.len());
+    let mut out = String::new();
+    for line in &lines[..tail] {
+        if ["error[", "warning[", "note[", "help["]
+            .iter()
+            .any(|head| line.starts_with(head))
+        {
+            out.push_str(line);
+            out.push('\n');
+        } else if let Some(locus) = line.trim_start().strip_prefix("┌─ ") {
+            out.push_str("  ┌─ ");
+            if windows {
+                out.push_str(&locus.replace('\\', "/"));
+            } else {
+                out.push_str(locus);
+            }
+            out.push('\n');
+        }
+    }
+    for line in &lines[tail..] {
+        out.push_str(line);
+        out.push('\n');
+    }
+    out
+}
+
+#[test]
+fn the_extraction_keeps_the_headers_the_loci_and_the_error() {
+    let s0 = "error[E0074]: method `len` is defined by multiple overlapping inherent impls\n    \
+              ┌─ overlap.nova:1:6\n    │\n  1 │ impl String {\n    │      ^^^^^^ here\n    \
+              │\n    ┌─ <std/strings>:193:6\n    │\n193 │ impl String {\n    │      ------ \
+              `len` is also defined by std's impl\n\nwarning[E0021]: unreachable match arm\n  \
+              ┌─ d\\w.nova:5:9\n  │\n  = help: remove the unreachable arm\n\n\
+              Error: release build failed; generated LLVM IR left at x.nova.ll\n\nCaused by:\n    \
+              no LLVM toolchain\n";
+    let tail = "Error: release build failed; generated LLVM IR left at x.nova.ll\n\n\
+                Caused by:\n    no LLVM toolchain\n";
+    let head = "error[E0074]: method `len` is defined by multiple overlapping inherent impls\n  \
+                ┌─ overlap.nova:1:6\n  ┌─ <std/strings>:193:6\nwarning[E0021]: unreachable \
+                match arm\n";
+    assert_eq!(
+        extracted_stderr(s0, false),
+        format!("{head}  ┌─ d\\w.nova:5:9\n{tail}")
+    );
+    assert_eq!(
+        extracted_stderr(s0, true),
+        format!("{head}  ┌─ d/w.nova:5:9\n{tail}")
+    );
+    assert_eq!(extracted_stderr("", false), "");
+}
+
+/// The probe cases N's `build` and `check` answer as S0 does with no
+/// `clang` (spec 5.5b §6.3): `check`'s, `build --release`'s that stop
+/// before `clang` runs or find none, and the usage errors.
+const PROBES: &[Probe] = {
+    use Expect::{Same, Usage};
+    use ProbeVar::InDir;
+    const NO_TOOL: &[(&str, ProbeVar)] = &[
+        ("NOVA_CLANG", InDir("no-such-tool")),
+        ("NOVA_LLC", InDir("no-such-tool")),
+    ];
+    const NO_TOOL_NO_RUNTIME: &[(&str, ProbeVar)] = &[
+        ("NOVA_CLANG", InDir("no-such-tool")),
+        ("NOVA_LLC", InDir("no-such-tool")),
+        ("NOVA_RUNTIME_LIB", InDir("no-such-runtime.a")),
+    ];
+    const fn build(
+        name: &'static str,
+        args: &'static [&'static str],
+        vars: &'static [(&'static str, ProbeVar)],
+    ) -> Probe {
+        Probe {
+            name,
+            s0: Some(args),
+            n: args,
+            std: true,
+            vars,
+            links: true,
+            expect: Same,
+        }
+    }
+    const fn usage(name: &'static str, args: &'static [&'static str], line: &'static str) -> Probe {
+        Probe {
+            name,
+            s0: Some(args),
+            n: args,
+            std: true,
+            vars: &[],
+            links: false,
+            expect: Usage(line),
+        }
+    }
+    const fn alone(name: &'static str, n: &'static [&'static str], line: &'static str) -> Probe {
+        Probe {
+            name,
+            s0: None,
+            n,
+            std: false,
+            vars: &[],
+            links: false,
+            expect: Usage(line),
+        }
+    }
+    &[
+        build(
+            "build compile errors",
+            &["build", "--release", "two.nova"],
+            NO_TOOL,
+        ),
+        build(
+            "build a missing FILE",
+            &["build", "--release", "nope.nova"],
+            NO_TOOL,
+        ),
+        build(
+            "build a directory as FILE",
+            &["build", "--release", "dir.nova"],
+            NO_TOOL,
+        ),
+        build(
+            "build into a missing directory",
+            &["build", "--release", "clean.nova", "-o", "nodir/x"],
+            NO_TOOL,
+        ),
+        build(
+            "build with no clang",
+            &["build", "--release", "clean.nova", "-o", "nc"],
+            NO_TOOL,
+        ),
+        build(
+            "build with no clang, the default name",
+            &["build", "--release", "clean.nova"],
+            NO_TOOL,
+        ),
+        build(
+            "build with no clang, a warning",
+            &["build", "--release", "warn.nova", "--output", "wn"],
+            NO_TOOL,
+        ),
+        build(
+            "build with no clang and no runtime",
+            &["build", "--release", "clean.nova", "--output=nr"],
+            NO_TOOL_NO_RUNTIME,
+        ),
+        build(
+            "build float %",
+            &["build", "--release", "frem.nova", "-o", "fr"],
+            NO_TOOL,
+        ),
+        usage(
+            "build an unknown option",
+            &["build", "--release", "--bogus", "clean.nova"],
+            N_BUILD_USAGE,
+        ),
+        usage(
+            "build -o twice",
+            &["build", "--release", "clean.nova", "-o", "a", "-o", "b"],
+            N_BUILD_USAGE,
+        ),
+        usage(
+            "build a second FILE",
+            &["build", "--release", "clean.nova", "two.nova"],
+            N_BUILD_USAGE,
+        ),
+        usage(
+            "build --release=yes",
+            &["build", "--release=yes", "clean.nova"],
+            N_BUILD_USAGE,
+        ),
+        alone(
+            "N builds without --std",
+            &["build", "clean.nova", "--release"],
+            N_BUILD_USAGE,
+        ),
+        alone(
+            "N builds without FILE",
+            &["build", "--release", "--std", "std"],
+            N_BUILD_USAGE,
+        ),
+    ]
+};
+
+/// A fresh directory for probe `name` under `under`, holding
+/// [`PROBE_INPUTS`] and the directory `dir.nova`.
+fn probe_dir(under: &Path, name: &str) -> PathBuf {
+    let dir = under.join(name.replace([' ', ',', '%', '.', '/', '\''], "-"));
+    let _ = std::fs::remove_dir_all(&dir);
+    for (path, text) in PROBE_INPUTS {
+        let file = dir.join(path);
+        std::fs::create_dir_all(file.parent().expect("a parent")).expect("a case's directory");
+        std::fs::write(&file, text).expect("write a probe input");
+    }
+    std::fs::create_dir_all(dir.join("dir.nova")).expect("make dir.nova");
+    #[cfg(unix)]
+    {
+        use std::os::unix::fs::PermissionsExt;
+        let script = dir.join("cc-args");
+        std::fs::write(&script, CC_ARGS).expect("write cc-args");
+        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
+            .expect("make cc-args executable");
+    }
+    dir
+}
+
+/// Every file under `dir`, by its path there with `/` between its parts.
+fn files_under(dir: &Path) -> std::collections::BTreeSet<String> {
+    fn walk(base: &Path, dir: &Path, out: &mut std::collections::BTreeSet<String>) {
+        let Ok(entries) = std::fs::read_dir(dir) else {
+            return;
+        };
+        for entry in entries {
+            let path = entry.expect("a directory entry").path();
+            if path.is_dir() {
+                walk(base, &path, out);
+            } else {
+                let rel = path.strip_prefix(base).expect("under the case's directory");
+                out.insert(rel.to_string_lossy().replace('\\', "/"));
+            }
+        }
+    }
+    let mut out = std::collections::BTreeSet::new();
+    walk(dir, dir, &mut out);
+    out
+}
+
+/// `program` with `args` in `dir`, `NOVA_HOME` at `home`, `NO_COLOR=1` and
+/// no backtrace on either side (CI sets `RUST_BACKTRACE=1`), with `vars`
+/// set and `removed` unset: what it printed and its exit code, and what it
+/// left in `dir` beside `before`, which is read and removed, so the other
+/// side finds the directory as this one did.
+#[allow(clippy::too_many_arguments)]
+fn probe_side(
+    program: &Path,
+    args: &[String],
+    dir: &Path,
+    home: &Path,
+    vars: &[(&str, std::ffi::OsString)],
+    removed: &[&str],
+    before: &std::collections::BTreeSet<String>,
+) -> Result<ProbeRun, String> {
+    let mut command = Command::new(program);
+    command
+        .current_dir(dir)
+        .args(args)
+        .env("NOVA_HOME", home)
+        .env("NO_COLOR", "1")
+        .env("RUST_BACKTRACE", "0")
+        .env("RUST_LIB_BACKTRACE", "0")
+        .stdin(Stdio::null());
+    for (name, value) in vars {
+        command.env(name, value);
+    }
+    for name in removed {
+        command.env_remove(name);
+    }
+    let what = format!("{} {}", program.display(), args.join(" "));
+    let out = output_within(&mut command, CHILD_LIMIT, &what)?;
+    let mut left = std::collections::BTreeMap::new();
+    for rel in files_under(dir).difference(before) {
+        let path = dir.join(rel);
+        left.insert(
+            rel.clone(),
+            std::fs::read(&path).expect("read a file left behind"),
+        );
+        std::fs::remove_file(&path).expect("remove a file left behind");
+    }
+    Ok(ProbeRun {
+        stdout: out.stdout,
+        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
+        code: out.status.code(),
+        left,
+    })
+}
+
+/// A side's run, for a report.
+fn shown(side: &str, run: &ProbeRun) -> String {
+    format!(
+        "--- {side}: exit {:?}, left {:?}\n--- stdout ---\n{}--- stderr ---\n{}",
+        run.code,
+        run.left.keys().collect::<Vec<_>>(),
+        String::from_utf8_lossy(&run.stdout),
+        run.stderr
+    )
+}
+
+/// Where `s0` and `n`, the two sides of one case, part, if they do, as
+/// `expect` compares them (spec 5.5b §6.3): `None` when they agree.
+fn probe_difference(expect: Expect, s0: &ProbeRun, n: &ProbeRun, windows: bool) -> Option<String> {
+    match expect {
+        Expect::Same => {
+            let extracted = extracted_stderr(&s0.stderr, windows);
+            if s0.code != n.code {
+                return Some(format!("exit {:?} against S0's {:?}", n.code, s0.code));
+            }
+            if s0.stdout != n.stdout {
+                return Some("stdout differs".to_string());
+            }
+            if extracted != n.stderr {
+                return Some(format!(
+                    "stderr differs from S0's extracted:\n--- S0, extracted ---\n{extracted}"
+                ));
+            }
+            if s0.left.keys().ne(n.left.keys()) {
+                return Some("the files left behind differ".to_string());
+            }
+            for (name, bytes) in &s0.left {
+                if n.left[name] != *bytes {
+                    return Some(format!("{name}, left behind, differs from S0's"));
+                }
+            }
+            None
+        }
+        Expect::Usage(line) => {
+            for (side, run) in [("S0", s0), ("N", n)] {
+                if run.code != Some(2) || !run.stdout.is_empty() || !run.left.is_empty() {
+                    return Some(format!(
+                        "{side} is not a usage error: exit 2, nothing on stdout, nothing left"
+                    ));
+                }
+            }
+            (n.stderr != line).then(|| format!("N's stderr is not {line:?}"))
+        }
+    }
+}
+
+/// The variables of a probe in `dir`.
+fn probe_vars(
+    vars: &[(&'static str, ProbeVar)],
+    dir: &Path,
+) -> Vec<(&'static str, std::ffi::OsString)> {
+    vars.iter()
+        .map(|(name, value)| {
+            let value = match value {
+                ProbeVar::InDir(file) => dir.join(file).into_os_string(),
+                ProbeVar::Text(text) => std::ffi::OsString::from(text),
+            };
+            (*name, value)
+        })
+        .collect()
+}
+
+/// Each of `probes`, four at a time, each in a fresh directory of its own
+/// under `under`, S0 then N; the failures, each with both sides' output.
+/// On Windows N refuses to build, so a case that builds is N's refusal
+/// there alone: exit 1, nothing on stdout and nothing left (spec 5.5b
+/// §3.5, §3.6), and S0 is not run.
+fn run_probes(probes: &[Probe], under: &Path, home: &Path, n: &Path) -> Vec<String> {
+    let std_dir = root().join("std");
+    let windows = cfg!(windows);
+    let next = std::sync::atomic::AtomicUsize::new(0);
+    let failures = std::sync::Mutex::new(Vec::<String>::new());
+    std::thread::scope(|s| {
+        for _ in 0..4 {
+            s.spawn(|| loop {
+                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
+                let Some(probe) = probes.get(i) else { break };
+                let dir = probe_dir(under, probe.name);
+                let before = files_under(&dir);
+                let vars = probe_vars(probe.vars, &dir);
+                let mut n_args: Vec<String> = probe.n.iter().map(|a| a.to_string()).collect();
+                if probe.std {
+                    n_args.push("--std".to_string());
+                    n_args.push(std_dir.to_string_lossy().into_owned());
+                }
+                let result = (|| -> Result<Option<String>, String> {
+                    let n_run = probe_side(n, &n_args, &dir, home, &vars, &[], &before)?;
+                    if windows && probe.links {
+                        let refused = n_run.code == Some(1)
+                            && n_run.stdout.is_empty()
+                            && n_run.stderr == N_REFUSES_WINDOWS
+                            && n_run.left.is_empty();
+                        return Ok((!refused)
+                            .then(|| format!("N did not refuse Windows\n{}", shown("N", &n_run))));
+                    }
+                    let s0_run = match probe.s0 {
+                        Some(args) => {
+                            let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
+                            probe_side(&nova(), &args, &dir, home, &vars, &[], &before)?
+                        }
+                        // N alone: what S0 would do is not asked; a usage
+                        // error is compared with itself.
+                        None => ProbeRun {
+                            stdout: Vec::new(),
+                            stderr: String::new(),
+                            code: Some(2),
+                            left: std::collections::BTreeMap::new(),
+                        },
+                    };
+                    Ok(
+                        probe_difference(probe.expect, &s0_run, &n_run, windows).map(|why| {
+                            format!("{why}\n{}{}", shown("S0", &s0_run), shown("N", &n_run))
+                        }),
+                    )
+                })();
+                let failure = match result {
+                    Ok(None) => None,
+                    Ok(Some(why)) => Some(why),
+                    Err(why) => Some(why),
+                };
+                if let Some(why) = failure {
+                    failures.lock().unwrap().push(format!(
+                        "{} (`{}`): {why}",
+                        probe.name,
+                        n_args.join(" ")
+                    ));
+                }
+                let _ = std::fs::remove_dir_all(&dir);
+            });
+        }
+    });
+    failures.into_inner().unwrap()
+}
+
+/// N's `build` and `check` answer as S0's do (spec
+/// `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`
+/// §6.3, decision 8b), on the cases that need no `clang` ([`PROBES`]): on
+/// each, S0 and N in one fixed directory, a fresh `NOVA_HOME`, `NO_COLOR=1`
+/// and no backtrace, must print the same stdout, exit with the same code,
+/// and leave the same files, byte for byte (the `.ll` with no `clang`),
+/// and N's stderr must be S0's as §4.3 extracts it: each diagnostic's
+/// header and loci, then anyhow's error. §3.6's exceptions, where they
+/// apply: clap's usage text, against which N's own line is pinned (exit 2
+/// and empty stdout compared); N without `--std` or FILE, run alone;
+/// Windows, where N refuses each `build` and S0's is not compared, and a
+/// locus's `\` is read as `/`.
+#[test]
+fn n_build_and_check_answer_as_s0_does() {
+    let n = selfhost();
+    let under = fresh_dir("probe");
+    let home = under.join("home");
+    std::fs::create_dir_all(&home).expect("create $NOVA_HOME");
+    let failures = run_probes(PROBES, &under, &home, n);
+    let _ = std::fs::remove_dir_all(&under);
+    assert!(
+        failures.is_empty(),
+        "{} of {} probes failed:\n\n{}",
+        failures.len(),
+        PROBES.len(),
+        failures.join("\n\n")
+    );
+}
+
+/// The probe cases that run `clang`, the runtime and `cc` (spec 5.5b
+/// §6.3), each with the runtime both sides link given as
+/// `NOVA_RUNTIME_LIB` where it does not name its own: success, with `-o`
+/// and with the default name from a relative FILE; a warning and success;
+/// `clang` and `cc` that fail, `clang` with no runtime too, which S0 looks
+/// for after `clang` has run; `cc` and the runtime missing; a text file as
+/// the runtime, where `ld`'s lines are the same on both sides; and `cc`'s
+/// command line, which [`CC_ARGS`] records, beside a binary that does not
+/// show every flag (`-lpthread`, `-ldl` and `-lm` are part of glibc 2.34's
+/// libc and of macOS's libSystem).
+const CLANG_PROBES: &[Probe] = {
+    use Expect::Same;
+    use ProbeVar::{InDir, Text};
+    const fn build(
+        name: &'static str,
+        args: &'static [&'static str],
+        vars: &'static [(&'static str, ProbeVar)],
+    ) -> Probe {
+        Probe {
+            name,
+            s0: Some(args),
+            n: args,
+            std: true,
+            vars,
+            links: true,
+            expect: Same,
+        }
+    }
+    &[
+        build(
+            "built with -o",
+            &["build", "--release", "clean.nova", "-o", "hi"],
+            &[],
+        ),
+        build(
+            "built with the default name",
+            &["build", "--release", "clean.nova"],
+            &[],
+        ),
+        build(
+            "built with a warning",
+            &["build", "--release", "warn.nova", "-o", "wn"],
+            &[],
+        ),
+        build(
+            "clang fails",
+            &["build", "--release", "clean.nova", "-o", "cf"],
+            &[("NOVA_CLANG", Text("false"))],
+        ),
+        // The runtime is looked for after `clang` has run, as S0 does:
+        // `clang`'s failure is the one reported.
+        build(
+            "clang fails, and no runtime",
+            &["build", "--release", "clean.nova", "-o", "cfr"],
+            &[
+                ("NOVA_CLANG", Text("false")),
+                ("NOVA_RUNTIME_LIB", InDir("no-such-runtime.a")),
+            ],
+        ),
+        build(
+            "cc fails",
+            &["build", "--release", "clean.nova", "-o", "ccf"],
+            &[("CC", Text("false"))],
+        ),
+        build(
+            "cc missing",
+            &["build", "--release", "clean.nova", "-o", "ccm"],
+            &[("CC", InDir("no-such-tool"))],
+        ),
+        build(
+            "cc's command line",
+            &["build", "--release", "warn.nova", "-o", "ca"],
+            &[("CC", InDir("cc-args"))],
+        ),
+        build(
+            "NOVA_RUNTIME_LIB missing",
+            &["build", "--release", "clean.nova", "-o", "rm"],
+            &[("NOVA_RUNTIME_LIB", InDir("no-such-runtime.a"))],
+        ),
+        build(
+            "NOVA_RUNTIME_LIB a text file",
+            &["build", "--release", "clean.nova", "-o", "rt"],
+            &[("NOVA_RUNTIME_LIB", InDir("clean.nova"))],
+        ),
+    ]
+};
+
+/// S0's words when no runtime is found beside it (`beside_exe`), as
+/// written there, for an executable at `exe`.
+fn no_runtime_words(exe: &Path) -> String {
+    format!(
+        "could not find the Nova runtime library (libnova_runtime.a) near {}, and this nova \
+         does not carry one. Install nova with the release profile, which is `cargo \
+         install`'s default, or build it with NOVA_EMBED_RUNTIME=1; in a checkout, `cargo \
+         build -p nova-runtime` puts the library beside it. NOVA_RUNTIME_LIB can also name \
+         one.",
+        exe.display()
+    )
+}
+
+/// N's `build` answers as S0's does where `clang` runs (spec 5.5b §6.3):
+/// [`CLANG_PROBES`] compared as [`n_build_and_check_answer_as_s0_does`]
+/// compares its cases, the binaries left behind byte for byte, and no
+/// `.ll` or `.o` beside a binary built; then, on N alone, no runtime (N
+/// copied alone into a fixed directory, `NOVA_RUNTIME_LIB` unset), against
+/// `beside_exe`'s words as written there, since CI's S0 carries a runtime,
+/// with N's own path (§3.6); and N's `build` without `--release`, which
+/// builds as with it (decision 6), the same binary for the same name. It
+/// needs `clang`, as [`s1_builds_the_golden_suite`] does, and runs in the
+/// same CI step. A binary built here also shows the golden harness
+/// refusing a wrong standard output and a different binary.
+#[test]
+#[ignore = "needs clang; the Selfhost job's Linux step runs it"]
+fn n_build_answers_as_s0_does_with_clang() {
+    let required = std::env::var("NOVA_REQUIRE_CLANG").as_deref() == Ok("1");
+    let clang = std::env::var("NOVA_CLANG").unwrap_or_else(|_| "clang".to_string());
+    if Command::new(&clang).arg("--version").output().is_err() {
+        assert!(
+            !required,
+            "NOVA_REQUIRE_CLANG=1, but `{clang} --version` does not run"
+        );
+        eprintln!("n build probes: not run (no clang)");
+        return;
+    }
+    let n = selfhost();
+    let runtime = golden_runtime();
+    let under = fresh_dir("probe-clang");
+    let home = under.join("home");
+    std::fs::create_dir_all(&home).expect("create $NOVA_HOME");
+    // The runtime, where a case does not name its own.
+    let runtime_var = [("NOVA_RUNTIME_LIB", runtime.clone().into_os_string())];
+    let std_dir = root().join("std").to_string_lossy().into_owned();
+    let mut failures = Vec::new();
+    // Every case counts once as it runs, so the report's "N of M" is out
+    // of the cases that ran.
+    let mut ran = 0;
+    for probe in CLANG_PROBES {
+        ran += 1;
+        let dir = probe_dir(&under, probe.name);
+        let before = files_under(&dir);
+        let mut vars = probe_vars(probe.vars, &dir);
+        if !vars.iter().any(|(name, _)| *name == "NOVA_RUNTIME_LIB") {
+            vars.extend(runtime_var.iter().cloned());
+        }
+        let mut n_args: Vec<String> = probe.n.iter().map(|a| a.to_string()).collect();
+        n_args.extend(["--std".to_string(), std_dir.clone()]);
+        let s0_args: Vec<String> = probe.s0.unwrap().iter().map(|a| a.to_string()).collect();
+        let s0_run = probe_side(&nova(), &s0_args, &dir, &home, &vars, &[], &before)
+            .unwrap_or_else(|e| panic!("{e}"));
+        let n_run = probe_side(n, &n_args, &dir, &home, &vars, &[], &before)
+            .unwrap_or_else(|e| panic!("{e}"));
+        let mut why = probe_difference(probe.expect, &s0_run, &n_run, false);
+        if why.is_none() && n_run.code == Some(0) {
+            let intermediates: Vec<&String> = n_run
+                .left
+                .keys()
+                .filter(|f| f.ends_with(".nova.ll") || f.ends_with(".nova.o"))
+                .collect();
+            if !intermediates.is_empty() {
+                why = Some("a build left its .ll or its object".to_string());
+            }
+        }
+        if let Some(why) = why {
+            failures.push(format!(
+                "{} (`{}`): {why}\n{}{}",
+                probe.name,
+                n_args.join(" "),
+                shown("S0", &s0_run),
+                shown("N", &n_run)
+            ));
+        }
+        let _ = std::fs::remove_dir_all(&dir);
+    }
+
+    // No runtime: N alone, copied into a fixed directory with none near it.
+    ran += 1;
+    let alone = fresh_dir("alone");
+    let exe = alone.join(format!("selfhost{EXE_SUFFIX}"));
+    std::fs::copy(n, &exe).expect("copy N alone");
+    for dir in exe.ancestors().skip(1).take(3) {
+        assert!(
+            !dir.join("libnova_runtime.a").exists(),
+            "{} holds a runtime",
+            dir.display()
+        );
+    }
+    let dir = probe_dir(&under, "no runtime");
+    let before = files_under(&dir);
+    let args: Vec<String> = [
+        "build",
+        "clean.nova",
+        "-o",
+        "nr",
+        "--release",
+        "--std",
+        &std_dir,
+    ]
+    .iter()
+    .map(|a| a.to_string())
+    .collect();
+    let run = probe_side(
+        &exe,
+        &args,
+        &dir,
+        &home,
+        &[],
+        &["NOVA_RUNTIME_LIB"],
+        &before,
+    )
+    .unwrap_or_else(|e| panic!("{e}"));
+    let want = format!(
+        "Error: release build failed; generated LLVM IR left at nr.nova.ll\n\nCaused by:\n    \
+         {}\n",
+        no_runtime_words(&exe)
+    );
+    if run.code != Some(1)
+        || !run.stdout.is_empty()
+        || run.stderr != want
+        || run.left.keys().ne(["nr.nova.ll"].iter())
+    {
+        failures.push(format!(
+            "no runtime: not S0's words with N's path\n--- want ---\n{want}{}",
+            shown("N", &run)
+        ));
+    }
+
+    // A runtime two directories up: N looks for it in its own directory,
+    // the parent and the grandparent, as S0's `beside_exe` does (spec
+    // §3.3), so a copy of N in `up/bin` finds the one beside `up`.
+    ran += 1;
+    let deep = alone.join("up").join("bin");
+    std::fs::create_dir_all(&deep).expect("make up/bin");
+    let deep_exe = deep.join(format!("selfhost{EXE_SUFFIX}"));
+    std::fs::copy(n, &deep_exe).expect("copy N into up/bin");
+    std::fs::copy(&runtime, alone.join("libnova_runtime.a")).expect("copy the runtime");
+    let dir = probe_dir(&under, "a runtime two directories up");
+    let before = files_under(&dir);
+    let args: Vec<String> = [
+        "build",
+        "clean.nova",
+        "-o",
+        "up",
+        "--release",
+        "--std",
+        &std_dir,
+    ]
+    .iter()
+    .map(|a| a.to_string())
+    .collect();
+    let run = probe_side(
+        &deep_exe,
+        &args,
+        &dir,
+        &home,
+        &[],
+        &["NOVA_RUNTIME_LIB"],
+        &before,
+    )
+    .unwrap_or_else(|e| panic!("{e}"));
+    if run.code != Some(0) || run.stdout != b"built up\n" || run.left.keys().ne(["up"].iter()) {
+        failures.push(format!(
+            "a runtime two directories up: not found\n{}",
+            shown("N", &run)
+        ));
+    }
+    let _ = std::fs::remove_dir_all(&dir);
+    let _ = std::fs::remove_dir_all(&alone);
+    let _ = std::fs::remove_dir_all(&dir);
+
+    // Without `--release`, as with it (decision 6); and the golden
+    // harness, on a binary built here, refuses a wrong standard output and
+    // a different binary.
+    ran += 1;
+    let dir = probe_dir(&under, "without release");
+    let before = files_under(&dir);
+    let mut runs = Vec::new();
+    for release in [true, false] {
+        let mut args: Vec<String> = ["build", "clean.nova", "-o", "hi", "--std", &std_dir]
+            .iter()
+            .map(|a| a.to_string())
+            .collect();
+        if release {
+            args.push("--release".to_string());
+        }
+        runs.push(
+            probe_side(n, &args, &dir, &home, &runtime_var, &[], &before)
+                .unwrap_or_else(|e| panic!("{e}")),
+        );
+    }
+    if runs[0].code != Some(0) || runs[0].stdout != runs[1].stdout || runs[0].left != runs[1].left {
+        failures.push(format!(
+            "without --release: not as with it\n{}{}",
+            shown("N --release", &runs[0]),
+            shown("N", &runs[1])
+        ));
+    } else {
+        let hi = dir.join(format!("hi{EXE_SUFFIX}"));
+        let other = dir.join("other");
+        std::fs::write(&hi, &runs[0].left["hi"]).expect("write the binary back");
+        let mut bytes = runs[0].left["hi"].clone();
+        let last = bytes.len() - 1;
+        bytes[last] ^= 1;
+        std::fs::write(&other, &bytes).expect("write a binary one byte off");
+        #[cfg(unix)]
+        {
+            use std::os::unix::fs::PermissionsExt;
+            std::fs::set_permissions(&hi, std::fs::Permissions::from_mode(0o755))
+                .expect("make the binary executable");
+        }
+        let tmp = dir.join("tmp");
+        std::fs::create_dir_all(&tmp).expect("a temporary directory");
+        // `clean.nova` prints `clean`: `arrays`'s `.stdout` is another
+        // text, and `fs_already_exists`'s holds `cleaned`, which a check
+        // looser than equality could take.
+        for name in ["arrays", "fs_already_exists"] {
+            ran += 1;
+            match run_golden(name, &hi, &tmp) {
+                Err(why) if why.starts_with("standard output differs") => {}
+                other => failures.push(format!(
+                    "the golden harness took a wrong standard output for {name}: {other:?}"
+                )),
+            }
+        }
+        ran += 1;
+        if same_binaries(&hi, &hi).is_err()
+            || !same_binaries(&hi, &other)
+                .is_err_and(|why| why.ends_with(&format!("first at offset {last}")))
+        {
+            failures.push("the golden harness took a different binary".to_string());
+        }
+    }
+    let _ = std::fs::remove_dir_all(&under);
+    assert!(
+        failures.is_empty(),
+        "{} of {ran} clang cases failed:\n\n{}",
+        failures.len(),
+        failures.join("\n\n")
+    );
+}
diff --git a/selfhost/src/driver_tests.nova b/selfhost/src/driver_tests.nova
new file mode 100644
index 00000000..9a1a2f56
--- /dev/null
+++ b/selfhost/src/driver_tests.nova
@@ -0,0 +1,209 @@
+// N's own tests of its driver (5.5b, spec
+// docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md
+// §9): the intermediate's name, the default output's stem, the Windows
+// shapes, anyhow's layout, a diagnostic's line and column and its loci, the
+// `{:?}` quoting of a tool's name, a failed tool's status line and the
+// U+FFFD fallback for its output. The probe tests in
+// crates/nova-cli/tests/selfhost.rs compare `build` with S0;
+// these pin the pieces where no probe reaches, or where one would say
+// less: Windows on Linux, a name-less output, two causes of several lines.
+//
+// They live in `src/`, imported by `main.nova`, for the reason
+// lexer_tests.nova gives.
+
+import diagnostic
+import driver
+import loader
+import token
+
+// A label of `file` on `start..end`.
+fn label(file: String, start: Int, end: Int, primary: Bool) -> Label {
+    Label { file: file, span: Span { start: start, end: end }, message: "here", primary: primary }
+}
+
+// `line_and_column`'s two numbers, as `LINE:COLUMN`.
+fn at(text: String, offset: Int) -> String {
+    let (line, column) = line_and_column(text, offset)
+    "${line}:${column}"
+}
+
+// `loci`'s pairs, as `FILE@START` joined by spaces.
+fn loci_of(labels: [Label]) -> String {
+    let mut parts: Vec<String> = Vec::new()
+    for (file, start) in loci(Vec::from_array(labels)).iter() { parts.push("${file}@${start}") }
+    " ".join(parts.to_array())
+}
+
+@test
+fn the_intermediate_is_named_for_the_outputs_file_name_as_s0_names_it() {
+    // nova-driver's `intermediate`: the whole file name, then `.nova.EXT`,
+    // so `-o out.ll` never names the output itself.
+    assert_eq(intermediate("out.ll", "ll"), "out.ll.nova.ll")
+    assert_eq(intermediate("x", "o"), "x.nova.o")
+    assert_eq(intermediate("a/b", "ll"), "a/b.nova.ll")
+    assert_eq(intermediate("./x", "ll"), "./x.nova.ll")
+    // A trailing `/`: `Path::file_name` is the last part, and
+    // `with_file_name` replaces it.
+    assert_eq(intermediate("dir/", "ll"), "dir.nova.ll")
+    assert_eq(intermediate("a/b/", "ll"), "a/b.nova.ll")
+    // A name-less output: `out`, inside it.
+    assert_eq(intermediate("..", "ll"), "../out.nova.ll")
+    assert_eq(intermediate("/", "ll"), "/out.nova.ll")
+    assert_eq(intermediate(".", "ll"), "./out.nova.ll")
+}
+
+@test
+fn the_default_output_is_files_stem_as_path_file_stem_gives_it() {
+    assert_eq(file_stem_or("a.b.nova", "out"), "a.b")
+    assert_eq(file_stem_or("dir/a.b.nova", "out"), "a.b")
+    // A leading dot is no extension; a trailing one is an empty one.
+    assert_eq(file_stem_or(".nova", "out"), ".nova")
+    assert_eq(file_stem_or("x.", "out"), "x")
+    // No stem: S0's `out`.
+    assert_eq(file_stem_or("..", "out"), "out")
+    let args = build_args(Vec::from_array(["selfhost", "build", "d/hello.nova", "--std", "s"]))
+    assert_eq(args.unwrap().output, "hello")
+}
+
+@test
+fn windows_is_known_by_a_drive_or_a_double_backslash() {
+    assert(windows_shape("C:\\x\\selfhost.exe"), "a drive")
+    assert(windows_shape("d:/x"), "a lowercase drive")
+    assert(windows_shape("\\\\?\\C:\\x"), "a verbatim prefix")
+    assert(windows_shape("\\\\server\\share\\x"), "a UNC prefix")
+    assert(!windows_shape("/x/selfhost"), "Unix's absolute path")
+    assert(!windows_shape("x"), "a bare name")
+    assert(!windows_shape("1:/x"), "a digit is no drive")
+    assert(!windows_shape("C"), "one byte")
+    assert(!windows_shape(""), "nothing")
+}
+
+@test
+fn the_command_lines_take_what_the_spec_says_and_refuse_the_rest() {
+    let build = |words: [String]| build_args(Vec::from_array(words)).is_some()
+    assert(build(["s", "build", "a.nova", "-o", "x", "--release", "--std", "d"]), "build")
+    assert(build(["s", "build", "--std=d", "--output=x", "a.nova"]), "= forms")
+    assert(build(["s", "build", "--std", "d", "--", "-a.nova"]), "-- ends options")
+    assert(!build(["s", "build", "a.nova"]), "no --std")
+    assert(!build(["s", "build", "--std", "d"]), "no FILE")
+    assert(!build(["s", "build", "a.nova", "--release", "--release", "--std", "d"]), "twice")
+    assert(!build(["s", "build", "a.nova", "--release=yes", "--std", "d"]), "--release=")
+}
+
+@test
+fn a_failure_is_printed_in_anyhows_layout() {
+    assert_eq(
+        failure_text("could not compile due to 1 previous error", []),
+        "Error: could not compile due to 1 previous error",
+    )
+    assert_eq(
+        failure_text("failed to open a", ["gone"]),
+        "Error: failed to open a\n\nCaused by:\n    gone",
+    )
+    // Empty lines are indented too.
+    assert_eq(
+        failure_text("h", ["clang failed (exit status: 3):\n\n"]),
+        "Error: h\n\nCaused by:\n    clang failed (exit status: 3):\n    \n    ",
+    )
+    // Several causes are numbered, a later line indented seven.
+    assert_eq(
+        failure_text("h", ["failed to spawn linker: \"x\"", "No such file\nor directory"]),
+        "Error: h\n\nCaused by:\n    0: failed to spawn linker: \"x\"\n    1: No such file\n       or directory",
+    )
+}
+
+@test
+fn line_and_column_count_line_feeds_and_characters_as_codespan_does() {
+    assert_eq(at("abc", 0), "1:1")
+    assert_eq(at("abc", 2), "1:3")
+    // A tab is one column.
+    assert_eq(at("\t\tx", 2), "1:3")
+    // `\r\n` is one line break, and `\r` a column before it.
+    assert_eq(at("a\r\nb\r\nc", 6), "3:1")
+    assert_eq(at("a\r\nb", 1), "1:2")
+    // A character of several bytes is one column: CJK, an emoji, Thai.
+    assert_eq(at("世界🎉 x", 11), "1:5")
+    assert_eq(at("ชื่อ x", 13), "1:6")
+    // On a line feed, the line it ends; past the end, the end.
+    assert_eq(at("ab\ncd", 2), "1:3")
+    assert_eq(at("ab\n", 3), "2:1")
+    assert_eq(at("ab", 9), "1:3")
+}
+
+@test
+fn a_files_locus_is_its_earliest_primary_label_else_its_earliest_secondary() {
+    // The earliest primary, though a secondary comes before it.
+    assert_eq(loci_of([label("a", 30, 35, true), label("a", 3, 9, false)]), "a@30")
+    assert_eq(loci_of([label("a", 30, 35, true), label("a", 10, 12, true)]), "a@10")
+    // No primary: the earliest secondary.
+    assert_eq(loci_of([label("a", 30, 35, false), label("a", 10, 12, false)]), "a@10")
+    // A primary after secondaries takes the locus.
+    assert_eq(loci_of([label("a", 3, 4, false), label("a", 40, 41, true)]), "a@40")
+    // Two files: one locus each, in the order the labels first name them.
+    assert_eq(
+        loci_of([label("b", 20, 21, true), label("a", 5, 6, false), label("b", 2, 3, false)]),
+        "b@20 a@5",
+    )
+    assert_eq(loci_of([]), "")
+}
+
+@test
+fn a_diagnostic_prints_its_header_and_one_locus_per_file() {
+    let mut d = Diagnostic::error("E0074", "method `len` is defined twice")
+    d.add_primary("main", Span { start: 5, end: 11 }, "here")
+    d.add_secondary("<std/strings>", Span { start: 3, end: 4 }, "also")
+    d.add_secondary("nowhere", Span { start: 0, end: 1 }, "a file S0 would not know")
+    d.add_note("a note, not printed")
+    let files = Vec::from_array(
+        [
+            SourceFile { label: "main", path: "d/main.nova", text: "impl String {" },
+            SourceFile { label: "<std/strings>", path: "<std/strings>", text: "\n\n\tx" },
+        ],
+    )
+    assert_eq(
+        diagnostic_lines(d, files),
+        "error[E0074]: method `len` is defined twice\n  ┌─ d/main.nova:1:6\n  ┌─ <std/strings>:3:2",
+    )
+    let mut w = Diagnostic::warning("E0021", "unreachable match arm")
+    w.add_primary("main", Span { start: 0, end: 1 }, "here")
+    assert_eq(
+        diagnostic_lines(w, files),
+        "warning[E0021]: unreachable match arm\n  ┌─ d/main.nova:1:1",
+    )
+    // No code: codespan leaves the brackets out; no label: no locus.
+    assert_eq(diagnostic_lines(Diagnostic::error("", "bare"), files), "error: bare")
+}
+
+@test
+fn a_tools_name_is_quoted_as_debug_quotes_it() {
+    assert_eq(debug_quoted("cc"), "\"cc\"")
+    assert_eq(debug_quoted("/x/no tool"), "\"/x/no tool\"")
+    assert_eq(debug_quoted("a\"b"), "\"a\\\"b\"")
+    assert_eq(debug_quoted("c:\\x\\y"), "\"c:\\\\x\\\\y\"")
+}
+
+@test
+fn a_failed_tools_cause_is_its_status_line_and_its_output_trimmed() {
+    let none = bytes_from_ints([])
+    assert_eq(status_cause("clang", 3, none, none), "clang failed (exit status: 3):\n\n")
+    let out = bytes_from_ints([32, 111, 117, 116, 10])
+    let err = bytes_from_ints([10, 101, 114, 114, 10, 10])
+    assert_eq(status_cause("linking", 1, out, err), "linking failed (exit status: 1):\nout\nerr")
+    // A signal: N's status is 128 + n (spec 5.5b §3.6).
+    assert_eq(status_cause("linking", 137, none, none), "linking failed (exit status: 137):\n\n")
+}
+
+@test
+fn a_tools_output_that_is_not_utf8_keeps_its_ascii_and_marks_each_other_byte() {
+    assert_eq(tool_text(bytes_from_ints([104, 105])), "hi")
+    // UTF-8 is kept whole.
+    assert_eq(tool_text(bytes_from_ints([226, 156, 147])).byte_len(), 3)
+    // Not UTF-8: `h`, U+FFFD (EF BF BD) for 0xFF, `i`.
+    let text = tool_text(bytes_from_ints([104, 255, 105]))
+    assert_eq(text.byte_len(), 5)
+    assert_eq(text.byte_at(0), 104)
+    assert_eq(text.byte_at(1), 239)
+    assert_eq(text.byte_at(2), 191)
+    assert_eq(text.byte_at(3), 189)
+    assert_eq(text.byte_at(4), 105)
+}
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index 47732fba..b2b159aa 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -65,6 +65,7 @@ import async_lower
 import builtins
 import diagnostic
 import driver
+import driver_tests
 import dump
 import emit_ll
 import emit_ll_tests
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/selfhost/src/driver.nova b/selfhost/src/driver.nova
index 64a19908..837a76b0 100644
--- a/selfhost/src/driver.nova
+++ b/selfhost/src/driver.nova
@@ -14,7 +14,13 @@
 // are gone, `built OUT` is on stdout and the exit code is 0. A failure
 // prints S0's words in anyhow's layout and exits 1; a command line it does
 // not take prints its usage line and exits 2. Without `--release` it does
-// the same, `-O2` too (decision 6), where S0 would use Cranelift.
+// the same, `-O2` too (decision 6), where S0 would use Cranelift. On
+// Windows, which N knows by its executable's path, `build` refuses
+// (§3.5).
+//
+// `build` prints each diagnostic, warnings too, as `SEVERITY[CODE]:
+// MESSAGE` and one `  ┌─ PATH:LINE:COL` line per file its labels name, on
+// stderr (§4.1).
 
 import builtins
 import diagnostic
@@ -31,6 +37,9 @@ import typeck_body
 /// take prints (spec 5.5b §3.1).
 pub const BUILD_USAGE: String = "usage: selfhost build FILE [-o OUT] [--release] --std DIR"
 
+/// N's refusal to build on Windows (spec 5.5b §3.5).
+pub const WINDOWS_REFUSAL: String = "selfhost build links on Linux and macOS only"
+
 // S0's words when neither `clang` nor `llc` can be spawned
 // (`compile_ir_to_object`); N looks for `clang` alone, and says the same.
 const NO_TOOLCHAIN: String = "no LLVM toolchain found for `--release`: install LLVM (so `clang` or `llc` is on PATH), or set NOVA_CLANG / NOVA_LLC to one"
@@ -119,10 +128,11 @@ pub fn build_args(argv: Vec<String>) -> Option<BuildArgs> {
 }
 
 /// `selfhost build` (spec 5.5b §3.2), in S0's order: the command line;
-/// std read from its directory; FILE loaded, checked and lowered; the
-/// `.ll` emitted and written beside the output; `clang`, the runtime and
-/// `cc` (`compile_and_link`); the object removed whatever happened, and on
-/// success the `.ll` too, and `built OUT` printed.
+/// the Windows refusal; std read from its directory; FILE loaded, checked
+/// and lowered, its diagnostics printed; the `.ll` emitted and written
+/// beside the output; `clang`, the runtime and `cc` (`compile_and_link`);
+/// the object removed whatever happened, and on success the `.ll` too, and
+/// `built OUT` printed.
 pub async fn build_command(argv: Vec<String>) {
     let args = match build_args(argv) {
         Some(a) => a
@@ -132,6 +142,10 @@ pub async fn build_command(argv: Vec<String>) {
             return
         }
     }
+    if on_windows() {
+        fail(WINDOWS_REFUSAL, [])
+        return
+    }
     let std_set = match load_std(args.std_dir, CompileTargetNative, false).await {
         Ok(s) => s
         Err(message) => {
@@ -141,9 +155,10 @@ pub async fn build_command(argv: Vec<String>) {
         }
     }
     let path = args.file
-    let program = match compile_root(path, std_set).await {
-        CompiledModule(p, _) => p
-        CompiledErrors(diagnostics) => {
+    let (lowered, warnings, files) = match compile_root(path, std_set).await {
+        CompiledModule(p, ds, fs) => (p, ds, fs)
+        CompiledErrors(diagnostics, files) => {
+            print_diagnostics(diagnostics, files, args.std_dir).await
             fail(could_not_compile(error_count(diagnostics)), [])
             return
         }
@@ -156,7 +171,8 @@ pub async fn build_command(argv: Vec<String>) {
             return
         }
     }
-    let ir = match compile_ir(program) {
+    print_diagnostics(warnings, files, args.std_dir).await
+    let ir = match compile_ir(lowered) {
         Ok(text) => text
         Err(words) => {
             fail("internal LLVM IR generation error (this is a compiler bug)", [words])
@@ -184,13 +200,40 @@ pub async fn build_command(argv: Vec<String>) {
     }
 }
 
-/// What `compile_root` made of a root: its lowered module and every
-/// diagnostic, warnings, on the way; the diagnostics of the stage that
-/// stopped it; the error that kept it from being read; or why N refuses
-/// its package's manifest.
+/// Whether `path`, the running executable's, is a Windows path (spec 5.5b
+/// §3.5): an ASCII letter and `:`, a drive, or `\\`, a `\\?\` or UNC
+/// prefix, at its start. Unix's absolute paths start with `/`.
+pub fn windows_shape(path: String) -> Bool {
+    if path.starts_with("\\\\") { return true }
+    if path.byte_len() < 2 || path.byte_at(1) != 58 { return false }
+    let c = path.byte_at(0)
+    (c >= 65 && c <= 90) || (c >= 97 && c <= 122)
+}
+
+// Whether N runs on Windows, by its executable's path (spec 5.5b §3.5);
+// taken for Unix when the system cannot say, so that `build` goes on to
+// §3.3's own error. No inherited state is read.
+fn on_windows() -> Bool {
+    match current_exe() {
+        Ok(path) => windows_shape(path)
+        Err(_) => false
+    }
+}
+
+/// One file a diagnostic's label may name: its label, as N's diagnostics
+/// carry it (`main`, `<std/core>`), the path S0's `FileDb` names it by
+/// (FILE as given, the path the loader joined and read, or the label for
+/// std), and its text.
+pub record SourceFile { pub label: String, pub path: String, pub text: String }
+
+/// What `compile_root` made of a root: its lowered module, every
+/// diagnostic, warnings, on the way, and the files they may name; the
+/// diagnostics of the stage that stopped it, and the files; the error
+/// that kept it from being read; or why N refuses its package's
+/// manifest.
 pub type Compiled =
-    | CompiledModule(MirModule, Vec<Diagnostic>)
-    | CompiledErrors(Vec<Diagnostic>)
+    | CompiledModule(MirModule, Vec<Diagnostic>, Vec<SourceFile>)
+    | CompiledErrors(Vec<Diagnostic>, Vec<SourceFile>)
     | CompiledIoError(IoError)
     | CompiledRefused(String)
 
@@ -218,9 +261,11 @@ pub async fn compile_root(path: String, std_set: StdSet) -> Compiled {
         Err(e) => return CompiledIoError(e)
     }
     let modules = load.modules
+    let mut files: Vec<SourceFile> = Vec::new()
+    for m in modules.iter() { files.push(SourceFile { label: m.name, path: m.path, text: m.text }) }
     let mut diagnostics: Vec<Diagnostic> = Vec::new()
     for d in load.diagnostics.iter() { diagnostics.push(d) }
-    if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics) }
+    if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics, files) }
     strip_test_functions(modules, false)
     let mut sources: Vec<ModuleSource> = Vec::new()
     for m in modules.iter() {
@@ -238,21 +283,147 @@ pub async fn compile_root(path: String, std_set: StdSet) -> Compiled {
     }
     let resolved = resolve_program_for(CompileTargetNative, sources, std_set)
     for d in resolved.diagnostics.iter() { diagnostics.push(d) }
-    if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics) }
+    if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics, files) }
     let (labels, texts) = labels_and_texts(sources, std_set)
     let mut tables = check_items(resolved.items, resolved.definitions, labels, texts)
     tables.check_bodies(resolved.newline_splits)
     for d in tables.diagnostics.iter() { diagnostics.push(d) }
-    if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics) }
+    if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics, files) }
     let program = full_module(tables)
     keep_the_entry_main(program, resolved.definitions)
     match lower_module_with(program, false) {
-        Ok(lowered) => CompiledModule(lowered, diagnostics)
+        Ok(lowered) => CompiledModule(lowered, diagnostics, files)
         Err(errors) => {
             for d in errors.iter() { diagnostics.push(d) }
-            CompiledErrors(diagnostics)
+            CompiledErrors(diagnostics, files)
+        }
+    }
+}
+
+/// A diagnostic's lines as `build` prints them (spec 5.5b §4.1): its
+/// header, `SEVERITY[CODE]: MESSAGE`, codespan's, the severity
+/// lowercase and `[CODE]` left out when there is no code; then one locus
+/// line per file its labels name (`loci`), `  ┌─ PATH:LINE:COL`, a label
+/// whose file is not in `files` left out, as S0's renderer leaves out a
+/// label whose file it does not know. No snippet, note, help or blank line.
+pub fn diagnostic_lines(d: Diagnostic, files: Vec<SourceFile>) -> String {
+    let mut out = StringBuilder::new()
+    out.push(severity_name(d.severity))
+    if d.code != "" { out.push("[${d.code}]") }
+    out.push(": ${d.message}")
+    for (label, start) in loci(d.labels).iter() {
+        match source_file(files, label) {
+            Some(f) => {
+                let (line, column) = line_and_column(f.text, start)
+                out.push("\n  ┌─ ${f.path}:${line}:${column}")
+            }
+            None => ()
         }
     }
+    out.to_string()
+}
+
+// A severity as codespan names it in a header.
+fn severity_name(severity: Severity) -> String {
+    match severity {
+        SeverityError => "error"
+        SeverityWarning => "warning"
+        SeverityNote => "note"
+        SeverityHelp => "help"
+    }
+}
+
+/// The loci codespan prints for `labels` (codespan-reporting 0.11.1,
+/// `term/views.rs`): one per file they name, in the order they first name
+/// it, each its file's label and a byte offset, the earliest start of its
+/// primary labels, or of its secondary ones when it has none; of two
+/// labels with one start and one style, the first.
+pub fn loci(labels: Vec<Label>) -> Vec<(String, Int)> {
+    let mut names: Vec<String> = Vec::new()
+    let mut starts: Vec<Int> = Vec::new()
+    let mut primary: Vec<Bool> = Vec::new()
+    for l in labels.iter() {
+        let mut i = 0
+        while i < names.len() && names.at(i) != l.file { i += 1 }
+        if i == names.len() {
+            names.push(l.file)
+            starts.push(l.span.start)
+            primary.push(l.primary)
+        } else if (l.primary && !primary.at(i))
+            || (l.primary == primary.at(i) && l.span.start < starts.at(i)) {
+            starts.set(i, l.span.start)
+            primary.set(i, l.primary)
+        }
+    }
+    let mut out: Vec<(String, Int)> = Vec::new()
+    let mut i = 0
+    while i < names.len() {
+        out.push((names.at(i), starts.at(i)))
+        i += 1
+    }
+    out
+}
+
+/// The line and column of byte `offset` in `text`, as codespan counts them
+/// (spec 5.5b §4.2): the line 1 plus the `\n` bytes before it, so `\r\n`
+/// is one line break; the column 1 plus the characters between its line's
+/// start and it, the bytes that are not UTF-8 continuation bytes, so a tab
+/// is one column, and so is a Thai mark, a CJK character or an emoji. An
+/// offset past the end counts to the end.
+pub fn line_and_column(text: String, offset: Int) -> (Int, Int) {
+    let end = if offset < text.byte_len() { offset } else { text.byte_len() }
+    let mut line = 1
+    let mut column = 1
+    let mut i = 0
+    while i < end {
+        let b = text.byte_at(i)
+        if b == 10 {
+            line += 1
+            column = 1
+        } else if b < 128 || b >= 192 {
+            column += 1
+        }
+        i += 1
+    }
+    (line, column)
+}
+
+// The file of `files` whose label is `label`.
+fn source_file(files: Vec<SourceFile>, label: String) -> Option<SourceFile> {
+    for f in files.iter() { if f.label == label { return Some(f) } }
+    None
+}
+
+// `files`, and each std module a label of `diagnostics` names, read from
+// under `std_dir` (`STD_MODULES`), its path its label, `<std/strings>`, as
+// S0's `FileDb` names an embedded std module.
+async fn with_std_files(
+    diagnostics: Vec<Diagnostic>,
+    files: Vec<SourceFile>,
+    std_dir: String,
+) -> Vec<SourceFile> {
+    let mut out: Vec<SourceFile> = Vec::new()
+    for f in files.iter() { out.push(f) }
+    for d in diagnostics.iter() {
+        for l in d.labels.iter() {
+            if source_file(out, l.file).is_some() { continue }
+            for (name, rel) in STD_MODULES.iter() {
+                if std_label(name) != l.file { continue }
+                match read_to_string("${std_dir}/${rel}").await {
+                    Ok(text) => out.push(SourceFile { label: l.file, path: l.file, text: text })
+                    Err(_) => ()
+                }
+            }
+        }
+    }
+    out
+}
+
+// Each of `diagnostics` on stderr, as `diagnostic_lines` gives it.
+async fn print_diagnostics(diagnostics: Vec<Diagnostic>, files: Vec<SourceFile>, std_dir: String) {
+    if diagnostics.len() == 0 { return }
+    let all = with_std_files(diagnostics, files, std_dir).await
+    for d in diagnostics.iter() { eprintln(diagnostic_lines(d, all)) }
 }
 
 // How many of `diagnostics` are errors.
@@ -331,13 +502,22 @@ fn run_tool(
         Err(e) => Err(["failed to spawn ${spawn}: ${debug_quoted(program)}", e.message])
         Ok(output) => {
             if output.status == 0 { return Ok(()) }
-            let stdout = tool_text(output.stdout).trim()
-            let stderr = tool_text(output.stderr).trim()
-            Err(["${failed} failed (exit status: ${output.status}):\n${stdout}\n${stderr}"])
+            Err([status_cause(failed, output.status, output.stdout, output.stderr)])
         }
     }
 }
 
+/// The cause of a tool that ran and failed, as S0's `run_tool` and
+/// `run_linker` word it: `FAILED failed (exit status: N):`, then its
+/// stdout and its stderr, each as text (`tool_text`) and trimmed, on lines
+/// of their own, empty ones too. A signal's status is N's `128 + n`, where
+/// Rust prints `signal: n (NAME)` (spec 5.5b §3.6).
+pub fn status_cause(failed: String, status: Int, stdout: Bytes, stderr: Bytes) -> String {
+    let out = tool_text(stdout).trim()
+    let err = tool_text(stderr).trim()
+    "${failed} failed (exit status: ${status}):\n${out}\n${err}"
+}
+
 /// `text` quoted as Rust's `{:?}` quotes a program's name, for the names
 /// N meets: in `"`, with `"` and `\` escaped.
 pub fn debug_quoted(text: String) -> String {
````

### Task 3: `check`, its lines compared with S0's

Spec §3.1, §4, §6.3, §11 item 3; decisions 3, 5, 8b and 11.

**Files:**
- Modify: `crates/nova-cli/tests/selfhost.rs`,
  `selfhost/src/driver_tests.nova`, `selfhost/src/driver.nova`,
  `selfhost/src/main.nova`

- [ ] **Step 1: The tests.** `patch_block t3-tests`:
  - **`selfhost.rs`:** `PROBES`' 24 `check` cases (spec §6.3: an unknown
    name with a help; a duplicate whose secondary label comes first; a
    multi-line span; tabs; a column after CJK, an emoji and Thai; Thai
    combining marks; a warning alone; two errors and a 2-digit line;
    `\r\n`; a sibling's error, from `d/` and from `./d/`; labels in two
    files; a label in std; a path dependency's error; a parse error; a
    clean program; float `%`; a missing FILE; a directory; three usage
    errors; N alone without `--std` or FILE); their inputs; the copy of
    `examples/05-geometry` with `GEOMETRY_ERROR`, made by `probe_dir`
    for the case that names it; `N_CHECK_USAGE`;
  - **`driver_tests.nova`:** `check`'s command line in the command-line
    `@test` (`check` takes no `-o`, `--output=` or `--release`).
- [ ] **Step 2: Run, and see them fail.** First the build, which ends with
  `Finished`. Then `cargo test --locked --workspace --test selfhost
  --no-run`, and `SH` (186 s). Expected: `68 passed; 2 failed; 2 ignored`:
  `n_build_and_check_answer_as_s0_does` (`24 of 39 probes failed`, every
  `check` case and every `check` usage case: N answers `usage: selfhost
  tokens|ast FILE...`, exit 2) and `the_selfhost_tests_pass` (N's
  tests do not build: `error[E0001]: cannot find function `check_args` in
  this scope`, at `src/driver_tests.nova:84:35`). N itself builds:
  `nova build` strips `@test` functions.
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`driver.nova`:** `CHECK_USAGE`; `check_args` and `command_args`
    (`build`'s command line, with `-o`, `--output` and `--release`
    refused for `check`); `check_command`: std, FILE through
    `compile_root(path, std_set, false)`, which lowers only a program
    that runs (a loose program or a package's `src/main.nova`, as
    `check_program_counted_for`), each diagnostic printed, then `ok:
    FILE`, or `found N error(s)` through `fail`; its refusals as
    `build`'s; `Compiled` carrying `Option<MirModule>`;
  - **`main.nova`:** `selfhost check` dispatched to `check_command`, and
    its usage comment.
- [ ] **Step 4: Run.** First the build, which ends with `Finished`.
  - `cargo test --locked --workspace --no-run`, then `S` in three runs
    (117, 1 and 157 s). Expected: `2837 1 10`.
  - `cargo test --locked --workspace --test selfhost --no-run`, then `SH`
    (156 s). Expected: `70 passed; 0 failed; 2 ignored`.
  - `C s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang`
    (62 s). Expected: `2 passed`.
  - `(cd selfhost && ../target/debug/nova test) 2>&1 | tail -n 1` (6 s).
    Expected: `test result: ok. 119 passed; 0 failed; 0 trapped; 119
    total`.
- [ ] **Step 5: The checks,** as the conventions list them (the
  mimicked Windows clippy turns off `3` items). Once more, to see that
  the mimic would have caught CI's failure: the Conventions' block, its
  `sed` line turning off only `probe_dir`'s block, the one `#[cfg(unix)]`
  indented four spaces:
  `sed -i "$S5,\$ s/^    #\[cfg(unix)\]/    #[cfg(any())]/" "$F"`.
  Expected: the count `1`; `error: constant `CC_ARGS` is never used`,
  CI's error of run 44672859855; `restored`; and Task 3's four files, as
  the Conventions list them.
- [ ] **Step 6: Commit** `selfhost: N's check, its lines compared with
      S0's`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/selfhost.rs b/crates/nova-cli/tests/selfhost.rs
index c507889c..a96b748c 100644
--- a/crates/nova-cli/tests/selfhost.rs
+++ b/crates/nova-cli/tests/selfhost.rs
@@ -113,9 +113,10 @@
 //! job on its Linux leg, builds every golden program `release_golden_suite`
 //! walks with it, runs each with the tables both files include
 //! (`golden/shared.rs`), and holds each binary equal to S0 `build
-//! --release`'s, byte for byte. On a probe set (§6.3) `build` answers as
-//! S0's does: the same stdout, exit code and files left behind, and N's
-//! stderr S0's as [`extracted_stderr`] extracts it;
+//! --release`'s, byte for byte. N's `selfhost check FILE --std DIR` prints
+//! each diagnostic's header and loci (§4). On a probe set (§6.3) both
+//! commands answer as S0's do: the same stdout, exit code and files left
+//! behind, and N's stderr S0's as [`extracted_stderr`] extracts it;
 //! [`n_build_and_check_answer_as_s0_does`] here, on every system, with no
 //! `clang`, and [`n_build_answers_as_s0_does_with_clang`], ignored here and
 //! run beside the golden walk.
@@ -7163,9 +7164,46 @@ fn the_ci_step_names_every_clang_test() {
 /// The inputs of the probe set (spec
 /// `docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`
 /// §6.3), each written into every case's directory: a path relative to it
-/// and its text. `dir.nova` is a directory, made beside them.
+/// and its text. `dir.nova` is a directory, made beside them, and a copy
+/// of `examples/05-geometry`, given an error in its library's second
+/// module, goes into the case that checks it.
 const PROBE_INPUTS: &[(&str, &str)] = &[
-    // E0021 alone: a warning.
+    // E0001, with a help.
+    (
+        "unknown.nova",
+        "fn main() {\n    let count = 3\n    println(\"${cuont}\")\n}\n",
+    ),
+    // E0002, its secondary label on an earlier line than its primary.
+    (
+        "dup.nova",
+        "fn helper() -> Int { 1 }\n\nfn helper() -> Int { 2 }\n\nfn main() {\n    \
+         println(\"${helper()}\")\n}\n",
+    ),
+    // E0010 on spans of several lines, the second's start on line 12.
+    (
+        "multiline.nova",
+        "fn pick(b: Bool) -> Int {\n    if b {\n        \"yes\"\n    } else {\n        \
+         \"no\"\n    }\n}\n\nfn main() {\n    let x: Int = if true {\n        1\n    } \
+         else {\n        \"two\"\n    }\n    println(\"${x} ${pick(true)}\")\n}\n",
+    ),
+    // Tabs before the span: one column each.
+    (
+        "tabs.nova",
+        "fn main() {\n\tlet x: Int = \"tab\"\n\t\tlet y: Bool = 1\n    println(\"${x} \
+         ${y}\")\n}\n",
+    ),
+    // L0001 after CJK, an emoji and Thai: a column per character.
+    (
+        "wide.nova",
+        "fn main() {\n    let s = \"世界🎉 สวัสดี\" ¤ 1\n    println(s)\n}\n",
+    ),
+    // L0001 on each character of a Thai name, combining marks among them.
+    (
+        "thai.nova",
+        "fn main() {\n    let ชื่อ = \"สวัสดี 世界 🎉\"\n    let n: Int = \"สวัสดี 世界 🎉\"\n    \
+         println(\"${ชื่อ} ${n}\")\n}\n",
+    ),
+    // E0021 alone: a warning, and `ok:`.
     (
         "warn.nova",
         "fn main() {\n    let v = 1\n    match v {\n        _ => println(\"a\")\n        \
@@ -7177,9 +7215,42 @@ const PROBE_INPUTS: &[(&str, &str)] = &[
         "fn main() {\n    let a: Int = true\n    println(\"${a}\")\n\n\n\n\n\n\n    \
          let b: String = 5\n    println(b)\n}\n",
     ),
+    // Windows line endings: `\r\n` is one line break.
+    (
+        "crlf.nova",
+        "fn main() {\r\n    let x: Int = \"crlf\"\r\n}\r\n",
+    ),
+    // A sibling's error: its locus names the path the loader joined.
+    (
+        "d/a.nova",
+        "import b::{twice}\n\nfn main() {\n    println(\"${twice(2)}\")\n}\n",
+    ),
+    (
+        "d/b.nova",
+        "pub fn twice(n: Int) -> Int {\n    n * \"2\"\n}\n",
+    ),
+    // E0074 labelled in two files, its primary in the second it names.
+    (
+        "g/a.nova",
+        "import b::{Shape}\n\nimpl Shape {\n    fn area(self) -> Int { 0 }\n}\n\nfn \
+         main() {\n    println(\"${Shape { w: 1 }.area()}\")\n}\n",
+    ),
+    (
+        "g/b.nova",
+        "pub record Shape { pub w: Int }\n\nimpl Shape {\n    pub fn area(self) -> Int \
+         { self.w }\n}\n",
+    ),
+    // E0074 labelled in the program and in std: `<std/strings>`'s locus.
+    (
+        "overlap.nova",
+        "impl String {\n    fn len(self) -> Int { 0 }\n}\n\nfn main() {\n    \
+         println(\"${\"ab\".len()}\")\n}\n",
+    ),
+    // A parse error.
+    ("parse.nova", "fn main() {\n    let x = (1 + \n}\n"),
     // Clean.
     ("clean.nova", "fn main() {\n    println(\"clean\")\n}\n"),
-    // Float `%`: the emitter refuses it.
+    // Float `%`: `check` is clean; the emitter refuses it.
     (
         "frem.nova",
         "fn main() {\n    let a = 5.5\n    let b = 2.0\n    println(\"${a % b}\")\n}\n",
@@ -7193,11 +7264,16 @@ const PROBE_INPUTS: &[(&str, &str)] = &[
 #[cfg(unix)]
 const CC_ARGS: &str = "#!/bin/sh\nprintf '%s\\n' \"$@\" > cc-args.txt\nexec cc \"$@\"\n";
 
+/// What the copy of `examples/05-geometry` gets appended to its library's
+/// `src/units.nova`: an error on its line 27, a 2-digit line.
+const GEOMETRY_ERROR: &str = "\npub fn broken() -> Int { \"no\" }\n";
+
 /// N's refusal to build on Windows (spec 5.5b §3.5).
 const N_REFUSES_WINDOWS: &str = "Error: selfhost build links on Linux and macOS only\n";
 
-/// N's usage line for `build` (spec 5.5b §3.1).
+/// N's usage lines (spec 5.5b §3.1).
 const N_BUILD_USAGE: &str = "usage: selfhost build FILE [-o OUT] [--release] --std DIR\n";
+const N_CHECK_USAGE: &str = "usage: selfhost check FILE --std DIR\n";
 
 /// How a probe's two sides are compared.
 #[derive(Clone, Copy, PartialEq, Eq)]
@@ -7322,6 +7398,17 @@ const PROBES: &[Probe] = {
         ("NOVA_LLC", InDir("no-such-tool")),
         ("NOVA_RUNTIME_LIB", InDir("no-such-runtime.a")),
     ];
+    const fn check(name: &'static str, args: &'static [&'static str]) -> Probe {
+        Probe {
+            name,
+            s0: Some(args),
+            n: args,
+            std: true,
+            vars: &[],
+            links: false,
+            expect: Same,
+        }
+    }
     const fn build(
         name: &'static str,
         args: &'static [&'static str],
@@ -7360,6 +7447,34 @@ const PROBES: &[Probe] = {
         }
     }
     &[
+        check(
+            "check an unknown name, with a help",
+            &["check", "unknown.nova"],
+        ),
+        check("check a secondary label first", &["check", "dup.nova"]),
+        check("check spans of several lines", &["check", "multiline.nova"]),
+        check("check after tabs", &["check", "tabs.nova"]),
+        check(
+            "check after CJK, an emoji and Thai",
+            &["check", "wide.nova"],
+        ),
+        check("check Thai combining marks", &["check", "thai.nova"]),
+        check("check a warning alone", &["check", "warn.nova"]),
+        check("check two errors, a 2-digit line", &["check", "two.nova"]),
+        check("check Windows line endings", &["check", "crlf.nova"]),
+        check("check a sibling's error", &["check", "d/a.nova"]),
+        check("check a sibling's error from ./", &["check", "./d/a.nova"]),
+        check("check labels in two files", &["check", "g/a.nova"]),
+        check("check a label in std", &["check", "overlap.nova"]),
+        check(
+            "check a path dependency's error",
+            &["check", "05-geometry/src/main.nova"],
+        ),
+        check("check a parse error", &["check", "parse.nova"]),
+        check("check a clean program", &["check", "clean.nova"]),
+        check("check float %", &["check", "frem.nova"]),
+        check("check a missing FILE", &["check", "nope.nova"]),
+        check("check a directory as FILE", &["check", "dir.nova"]),
         build(
             "build compile errors",
             &["build", "--release", "two.nova"],
@@ -7425,6 +7540,21 @@ const PROBES: &[Probe] = {
             &["build", "--release=yes", "clean.nova"],
             N_BUILD_USAGE,
         ),
+        usage(
+            "check an unknown option",
+            &["check", "--bogus", "clean.nova"],
+            N_CHECK_USAGE,
+        ),
+        usage(
+            "check a second FILE",
+            &["check", "clean.nova", "two.nova"],
+            N_CHECK_USAGE,
+        ),
+        usage(
+            "check -o",
+            &["check", "clean.nova", "-o", "x"],
+            N_CHECK_USAGE,
+        ),
         alone(
             "N builds without --std",
             &["build", "clean.nova", "--release"],
@@ -7435,12 +7565,23 @@ const PROBES: &[Probe] = {
             &["build", "--release", "--std", "std"],
             N_BUILD_USAGE,
         ),
+        alone(
+            "N checks without --std",
+            &["check", "clean.nova"],
+            N_CHECK_USAGE,
+        ),
+        alone(
+            "N checks without FILE",
+            &["check", "--std", "std"],
+            N_CHECK_USAGE,
+        ),
     ]
 };
 
 /// A fresh directory for probe `name` under `under`, holding
-/// [`PROBE_INPUTS`] and the directory `dir.nova`.
-fn probe_dir(under: &Path, name: &str) -> PathBuf {
+/// [`PROBE_INPUTS`], the directory `dir.nova` and, when `geometry`, the
+/// copy of `examples/05-geometry` with [`GEOMETRY_ERROR`].
+fn probe_dir(under: &Path, name: &str, geometry: bool) -> PathBuf {
     let dir = under.join(name.replace([' ', ',', '%', '.', '/', '\''], "-"));
     let _ = std::fs::remove_dir_all(&dir);
     for (path, text) in PROBE_INPUTS {
@@ -7457,6 +7598,24 @@ fn probe_dir(under: &Path, name: &str) -> PathBuf {
         std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
             .expect("make cc-args executable");
     }
+    if geometry {
+        let from = root().join("examples").join("05-geometry");
+        for rel in [
+            "nova.toml",
+            "src/main.nova",
+            "shapes/nova.toml",
+            "shapes/src/lib.nova",
+            "shapes/src/units.nova",
+        ] {
+            let to = dir.join("05-geometry").join(rel);
+            std::fs::create_dir_all(to.parent().expect("a parent")).expect("a directory");
+            std::fs::copy(from.join(rel), &to).expect("copy examples/05-geometry");
+        }
+        let units = dir.join("05-geometry/shapes/src/units.nova");
+        let mut text = std::fs::read_to_string(&units).expect("read units.nova");
+        text.push_str(GEOMETRY_ERROR);
+        std::fs::write(&units, text).expect("write units.nova");
+    }
     dir
 }
 
@@ -7612,7 +7771,8 @@ fn run_probes(probes: &[Probe], under: &Path, home: &Path, n: &Path) -> Vec<Stri
             s.spawn(|| loop {
                 let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                 let Some(probe) = probes.get(i) else { break };
-                let dir = probe_dir(under, probe.name);
+                let geometry = probe.n.iter().any(|a| a.starts_with("05-geometry"));
+                let dir = probe_dir(under, probe.name, geometry);
                 let before = files_under(&dir);
                 let vars = probe_vars(probe.vars, &dir);
                 let mut n_args: Vec<String> = probe.n.iter().map(|a| a.to_string()).collect();
@@ -7836,7 +7996,7 @@ fn n_build_answers_as_s0_does_with_clang() {
     let mut ran = 0;
     for probe in CLANG_PROBES {
         ran += 1;
-        let dir = probe_dir(&under, probe.name);
+        let dir = probe_dir(&under, probe.name, false);
         let before = files_under(&dir);
         let mut vars = probe_vars(probe.vars, &dir);
         if !vars.iter().any(|(name, _)| *name == "NOVA_RUNTIME_LIB") {
@@ -7884,7 +8044,7 @@ fn n_build_answers_as_s0_does_with_clang() {
             dir.display()
         );
     }
-    let dir = probe_dir(&under, "no runtime");
+    let dir = probe_dir(&under, "no runtime", false);
     let before = files_under(&dir);
     let args: Vec<String> = [
         "build",
@@ -7933,7 +8093,7 @@ fn n_build_answers_as_s0_does_with_clang() {
     let deep_exe = deep.join(format!("selfhost{EXE_SUFFIX}"));
     std::fs::copy(n, &deep_exe).expect("copy N into up/bin");
     std::fs::copy(&runtime, alone.join("libnova_runtime.a")).expect("copy the runtime");
-    let dir = probe_dir(&under, "a runtime two directories up");
+    let dir = probe_dir(&under, "a runtime two directories up", false);
     let before = files_under(&dir);
     let args: Vec<String> = [
         "build",
@@ -7971,7 +8131,7 @@ fn n_build_answers_as_s0_does_with_clang() {
     // harness, on a binary built here, refuses a wrong standard output and
     // a different binary.
     ran += 1;
-    let dir = probe_dir(&under, "without release");
+    let dir = probe_dir(&under, "without release", false);
     let before = files_under(&dir);
     let mut runs = Vec::new();
     for release in [true, false] {
diff --git a/selfhost/src/driver_tests.nova b/selfhost/src/driver_tests.nova
index 9a1a2f56..1cd1c3f9 100644
--- a/selfhost/src/driver_tests.nova
+++ b/selfhost/src/driver_tests.nova
@@ -1,10 +1,10 @@
 // N's own tests of its driver (5.5b, spec
 // docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md
 // §9): the intermediate's name, the default output's stem, the Windows
-// shapes, anyhow's layout, a diagnostic's line and column and its loci, the
+// shapes, anyhow's layout, `check`'s line and column and its loci, the
 // `{:?}` quoting of a tool's name, a failed tool's status line and the
 // U+FFFD fallback for its output. The probe tests in
-// crates/nova-cli/tests/selfhost.rs compare `build` with S0;
+// crates/nova-cli/tests/selfhost.rs compare `build` and `check` with S0;
 // these pin the pieces where no probe reaches, or where one would say
 // less: Windows on Linux, a name-less output, two causes of several lines.
 //
@@ -81,6 +81,7 @@ fn windows_is_known_by_a_drive_or_a_double_backslash() {
 @test
 fn the_command_lines_take_what_the_spec_says_and_refuse_the_rest() {
     let build = |words: [String]| build_args(Vec::from_array(words)).is_some()
+    let check = |words: [String]| check_args(Vec::from_array(words)).is_some()
     assert(build(["s", "build", "a.nova", "-o", "x", "--release", "--std", "d"]), "build")
     assert(build(["s", "build", "--std=d", "--output=x", "a.nova"]), "= forms")
     assert(build(["s", "build", "--std", "d", "--", "-a.nova"]), "-- ends options")
@@ -88,6 +89,10 @@ fn the_command_lines_take_what_the_spec_says_and_refuse_the_rest() {
     assert(!build(["s", "build", "--std", "d"]), "no FILE")
     assert(!build(["s", "build", "a.nova", "--release", "--release", "--std", "d"]), "twice")
     assert(!build(["s", "build", "a.nova", "--release=yes", "--std", "d"]), "--release=")
+    assert(check(["s", "check", "a.nova", "--std", "d"]), "check")
+    assert(!check(["s", "check", "a.nova", "-o", "x", "--std", "d"]), "check takes no -o")
+    assert(!check(["s", "check", "a.nova", "--output=x", "--std", "d"]), "nor --output=")
+    assert(!check(["s", "check", "a.nova", "--release", "--std", "d"]), "nor --release")
 }
 
 @test
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/selfhost/src/driver.nova b/selfhost/src/driver.nova
index 837a76b0..46054f45 100644
--- a/selfhost/src/driver.nova
+++ b/selfhost/src/driver.nova
@@ -1,11 +1,15 @@
 // N's driver (5.5b, spec
 // docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md
-// §3): `selfhost build`, a port of S0's file-mode `nova build --release`
-// (nova-cli's `build`, nova-driver's `build_program_release`, `link.rs`'s
-// `compile_ir_to_object`, `link_executable` and `link_cc`), and the
-// loading of std that every command of N's shares.
+// §3, §4): `selfhost build`, a port of S0's file-mode `nova build
+// --release` (nova-cli's `build`, nova-driver's `build_program_release`,
+// `link.rs`'s `compile_ir_to_object`, `link_executable` and `link_cc`);
+// `selfhost check`, S0's file-mode `nova check`, each diagnostic printed
+// as its header and loci, codespan's (`check_program_counted_for`,
+// nova-diagnostics' `render`); and the loading of std that every command
+// of N's shares.
 //
 //   selfhost build FILE [-o OUT | --output OUT] [--release] --std DIR
+//   selfhost check FILE --std DIR
 //
 // FILE is loaded as `selfhost mir` loads it, checked and lowered natively;
 // the emitter's text is written to `<OUT>.nova.ll`, `clang -O2 -c` makes
@@ -16,11 +20,13 @@
 // not take prints its usage line and exits 2. Without `--release` it does
 // the same, `-O2` too (decision 6), where S0 would use Cranelift. On
 // Windows, which N knows by its executable's path, `build` refuses
-// (§3.5).
+// (§3.5); `check` does not link, and runs there too.
 //
-// `build` prints each diagnostic, warnings too, as `SEVERITY[CODE]:
+// `check` loads, checks and, for a program that runs, lowers FILE as
+// `build` does, prints each diagnostic, warnings too, as `SEVERITY[CODE]:
 // MESSAGE` and one `  ┌─ PATH:LINE:COL` line per file its labels name, on
-// stderr (§4.1).
+// stderr, then `ok: FILE` on stdout, or `Error: found N error(s)` and the
+// exit code 1. `build` prints its diagnostics the same way.
 
 import builtins
 import diagnostic
@@ -37,6 +43,9 @@ import typeck_body
 /// take prints (spec 5.5b §3.1).
 pub const BUILD_USAGE: String = "usage: selfhost build FILE [-o OUT] [--release] --std DIR"
 
+/// `selfhost check`'s usage line (spec 5.5b §3.1).
+pub const CHECK_USAGE: String = "usage: selfhost check FILE --std DIR"
+
 /// N's refusal to build on Windows (spec 5.5b §3.5).
 pub const WINDOWS_REFUSAL: String = "selfhost build links on Linux and macOS only"
 
@@ -44,8 +53,9 @@ pub const WINDOWS_REFUSAL: String = "selfhost build links on Linux and macOS onl
 // (`compile_ir_to_object`); N looks for `clang` alone, and says the same.
 const NO_TOOLCHAIN: String = "no LLVM toolchain found for `--release`: install LLVM (so `clang` or `llc` is on PATH), or set NOVA_CLANG / NOVA_LLC to one"
 
-/// What `selfhost build`'s command line names: the root, the output, the
-/// std directory, and whether `--release` was given.
+/// What `selfhost build`'s or `check`'s command line names: the root, the
+/// output, the std directory, and whether `--release` was given; `check`'s
+/// has neither of those two.
 pub record BuildArgs {
     pub file: String,
     pub output: String,
@@ -62,7 +72,14 @@ pub record BuildArgs {
 /// that begins with `-` but is not `-`, as clap takes that for an option.
 /// The output is FILE's stem, `out` when it has none, when no `-o` names
 /// one.
-pub fn build_args(argv: Vec<String>) -> Option<BuildArgs> {
+pub fn build_args(argv: Vec<String>) -> Option<BuildArgs> { command_args(argv, true) }
+
+/// `selfhost check`'s command line (spec 5.5b §3.1): `build`'s, with
+/// neither an output nor `--release`.
+pub fn check_args(argv: Vec<String>) -> Option<BuildArgs> { command_args(argv, false) }
+
+// `build`'s command line when `build`, else `check`'s.
+fn command_args(argv: Vec<String>, build: Bool) -> Option<BuildArgs> {
     let mut file: Option<String> = None
     let mut output: Option<String> = None
     let mut std_dir: Option<String> = None
@@ -82,13 +99,16 @@ pub fn build_args(argv: Vec<String>) -> Option<BuildArgs> {
             continue
         }
         if a == "--release" {
-            if release { return None }
+            if release || !build { return None }
             release = true
             continue
         }
         let mut is_std = false
         let mut value = ""
-        if a.starts_with("--output=") {
+        let output_option = a.starts_with("--output=") || a == "-o" || a == "--output"
+        if output_option && !build {
+            return None
+        } else if a.starts_with("--output=") {
             value = a.byte_slice(9, a.byte_len())
         } else if a.starts_with("--std=") {
             is_std = true
@@ -155,7 +175,7 @@ pub async fn build_command(argv: Vec<String>) {
         }
     }
     let path = args.file
-    let (lowered, warnings, files) = match compile_root(path, std_set).await {
+    let (lowered, warnings, files) = match compile_root(path, std_set, true).await {
         CompiledModule(p, ds, fs) => (p, ds, fs)
         CompiledErrors(diagnostics, files) => {
             print_diagnostics(diagnostics, files, args.std_dir).await
@@ -172,7 +192,7 @@ pub async fn build_command(argv: Vec<String>) {
         }
     }
     print_diagnostics(warnings, files, args.std_dir).await
-    let ir = match compile_ir(lowered) {
+    let ir = match compile_ir(lowered.unwrap()) {
         Ok(text) => text
         Err(words) => {
             fail("internal LLVM IR generation error (this is a compiler bug)", [words])
@@ -200,6 +220,45 @@ pub async fn build_command(argv: Vec<String>) {
     }
 }
 
+/// `selfhost check` (spec 5.5b §4), as S0's file-mode `nova check`: the
+/// command line; std read from its directory; FILE loaded, checked and,
+/// when it runs, lowered; each diagnostic printed (§4.1); then `ok: FILE`,
+/// or `Error: found N error(s)` and the exit code 1. It does not link, so
+/// it runs on Windows too.
+pub async fn check_command(argv: Vec<String>) {
+    let args = match check_args(argv) {
+        Some(a) => a
+        None => {
+            eprintln(CHECK_USAGE)
+            exit(2)
+            return
+        }
+    }
+    let std_set = match load_std(args.std_dir, CompileTargetNative, false).await {
+        Ok(s) => s
+        Err(message) => {
+            eprintln(message)
+            exit(2)
+            return
+        }
+    }
+    let path = args.file
+    match compile_root(path, std_set, false).await {
+        CompiledModule(_, warnings, files) => {
+            print_diagnostics(warnings, files, args.std_dir).await
+            println("ok: ${path}")
+        }
+        CompiledErrors(diagnostics, files) => {
+            print_diagnostics(diagnostics, files, args.std_dir).await
+            let n = error_count(diagnostics)
+            let s = if n == 1 { "" } else { "s" }
+            fail("found ${n} error${s}", [])
+        }
+        CompiledIoError(e) => fail("failed to open ${path}", [e.message])
+        CompiledRefused(why) => fail("selfhost check cannot read ${path}: ${why}", [])
+    }
+}
+
 /// Whether `path`, the running executable's, is a Windows path (spec 5.5b
 /// §3.5): an ASCII letter and `:`, a drive, or `\\`, a `\\?\` or UNC
 /// prefix, at its start. Unix's absolute paths start with `/`.
@@ -226,32 +285,40 @@ fn on_windows() -> Bool {
 /// std), and its text.
 pub record SourceFile { pub label: String, pub path: String, pub text: String }
 
-/// What `compile_root` made of a root: its lowered module, every
-/// diagnostic, warnings, on the way, and the files they may name; the
-/// diagnostics of the stage that stopped it, and the files; the error
-/// that kept it from being read; or why N refuses its package's
+/// What `compile_root` made of a root: its lowered module, when it was
+/// lowered, every diagnostic, warnings, on the way, and the files they
+/// may name; the diagnostics of the stage that stopped it, and the files;
+/// the error that kept it from being read; or why N refuses its package's
 /// manifest.
 pub type Compiled =
-    | CompiledModule(MirModule, Vec<Diagnostic>, Vec<SourceFile>)
+    | CompiledModule(Option<MirModule>, Vec<Diagnostic>, Vec<SourceFile>)
     | CompiledErrors(Vec<Diagnostic>, Vec<SourceFile>)
     | CompiledIoError(IoError)
     | CompiledRefused(String)
 
 /// The root `path` loaded as `selfhost mir` loads it, a loose program or a
 /// module of the package whose `src/` or `tests/` it is directly in, its
-/// `@test` functions stripped, then resolved, checked and lowered natively
-/// in full, std's bodies with it, as S0's `build` does
-/// (`FrontendContext::check`, then `lower_module_with`): each stage's
-/// diagnostics after the last's, the first stage with an error the last.
-pub async fn compile_root(path: String, std_set: StdSet) -> Compiled {
+/// `@test` functions stripped, then resolved and checked natively in full,
+/// std's bodies with it, as S0's `build` and `check` do
+/// (`FrontendContext::check`), and lowered (`lower_module_with`) when
+/// `build`, or when the program runs, as `check` decides it
+/// (`check_program_counted_for`): a loose program, or a package's
+/// `src/main.nova`. Each stage's diagnostics follow the last's; the first
+/// stage with an error is the last.
+pub async fn compile_root(path: String, std_set: StdSet, build: Bool) -> Compiled {
     let mut graph: Option<Graph> = None
     let mut kind = RootLoose
+    let mut runs = true
     match package_of(path).await {
         Some((dir, k)) => {
             match read_graph(dir).await {
                 Ok(g) => graph = Some(g)
                 Err(why) => return CompiledRefused(why)
             }
+            runs = match k {
+                RootSrc => module_name(path) == "main"
+                _ => false
+            }
             kind = k
         }
         None => ()
@@ -289,10 +356,11 @@ pub async fn compile_root(path: String, std_set: StdSet) -> Compiled {
     tables.check_bodies(resolved.newline_splits)
     for d in tables.diagnostics.iter() { diagnostics.push(d) }
     if error_count(diagnostics) > 0 { return CompiledErrors(diagnostics, files) }
+    if !build && !runs { return CompiledModule(None, diagnostics, files) }
     let program = full_module(tables)
     keep_the_entry_main(program, resolved.definitions)
     match lower_module_with(program, false) {
-        Ok(lowered) => CompiledModule(lowered, diagnostics, files)
+        Ok(lowered) => CompiledModule(Some(lowered), diagnostics, files)
         Err(errors) => {
             for d in errors.iter() { diagnostics.push(d) }
             CompiledErrors(diagnostics, files)
@@ -300,8 +368,8 @@ pub async fn compile_root(path: String, std_set: StdSet) -> Compiled {
     }
 }
 
-/// A diagnostic's lines as `build` prints them (spec 5.5b §4.1): its
-/// header, `SEVERITY[CODE]: MESSAGE`, codespan's, the severity
+/// A diagnostic's lines as `check` and `build` print them (spec 5.5b
+/// §4.1): its header, `SEVERITY[CODE]: MESSAGE`, codespan's, the severity
 /// lowercase and `[CODE]` left out when there is no code; then one locus
 /// line per file its labels name (`loci`), `  ┌─ PATH:LINE:COL`, a label
 /// whose file is not in `files` left out, as S0's renderer leaves out a
diff --git a/selfhost/src/main.nova b/selfhost/src/main.nova
index b2b159aa..c4dd73dd 100644
--- a/selfhost/src/main.nova
+++ b/selfhost/src/main.nova
@@ -54,6 +54,12 @@
 //                             OUT` printed (spec
 //                             docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md
 //                             §3, driver.nova)
+//   selfhost check FILE --std DIR
+//                             checks FILE as S0's `nova check FILE` does,
+//                             each diagnostic printed as its header and
+//                             loci, `error[CODE]: MESSAGE` and `  ┌─
+//                             PATH:LINE:COL`, then `ok: FILE` or `Error:
+//                             found N error(s)` (spec 5.5b §4, driver.nova)
 //
 // A file with lex or parse errors is dumped with them, and the exit code
 // stays 0. A file that cannot be read prints `IOERR` and its kind after its
@@ -124,6 +130,10 @@ async fn main() {
         build_command(argv).await
         return
     }
+    if command == "check" {
+        check_command(argv).await
+        return
+    }
     // No file is refused, as S0's clap refuses it.
     let tokens = command == "tokens"
     if argv.len() < 3 || (!tokens && command != "ast") {
````

### Task 4: ADR 0059 and the records

Spec §13; decisions 1 to 19.

**Files:**
- Create: `docs/adr/0059-the-self-hosted-driver-build-and-check.md`
- Modify: `CHANGELOG.md`, `docs/phase-5-plan.md`,
  `nova-spec/00-MASTER-SPEC.md`, `nova-spec/14-CODEGEN.md`,
  `nova-spec/40-TOOLING.md`

- [ ] **Step 1: The records.** `patch_block t4-code`:
  - **ADR 0059,** the self-hosted driver, `build` and `check`: its Status
    `Accepted (2026-11-20)` and this branch; §1 the command lines (`--std`
    narrowing the 2026-11-02 amendment); §2 `build`'s steps; §3 `clang`,
    the runtime and `cc`, no `llc`; §4 Windows; §5 the messages and exit
    codes against S0's; §6 `check`'s lines and the extraction; §7 what N
    does not match (spec §3.6); §8 the shared tables, the golden test and
    CI, with `the_ci_step_names_every_clang_test`; §9 the probe tests,
    the clang test's cases and count, and the fixed temporary names; §10
    N's `@test`s; §11 part 2's 23 mutants and this plan's 64; §12 the
    figures of parts 1 and 2; §13 S1 building N, and S2 = S1, as measured
    at `ecfcd0f8` on 2026-11-20; §14 CI's figures; §15 the
    intermediate's name and what 5.6 inherits; §16 ADR 0058 §14's
    questions answered; §17 the departures and the decisions the spec
    does not settle; Consequences.
  - **`CHANGELOG.md`,** Unreleased: Added, N's `build` and `check`, S1's
    golden step, the shared tables, the CI-step test, the probe tests and
    the clang test's cases; nothing Changed.
  - **Dated notes:** `docs/phase-5-plan.md` §5.5 (5.5b's decisions and
    gate, a note for 5.6); `00-MASTER-SPEC.md`'s Phase 5;
    `14-CODEGEN.md` §2.2 and §11 (N runs `clang` and `cc` as S0 does, on
    Linux and macOS); `40-TOOLING.md` §1.1 (`nova build` and `nova check`
    unchanged). All dated 2026-11-20 and naming this branch (decision
    15).
- [ ] **Step 2: Read them against the tree.**
  - `git grep -n 'proto/5-5b\|split/5-5b\|scratchpad\|/home/[a-z]' --
    docs/adr/0059-the-self-hosted-driver-build-and-check.md CHANGELOG.md
    docs/phase-5-plan.md nova-spec` prints nothing (exit 1).
  - Each commit the ADR names exists and is on the pushed branch:
    `for h in 17ecca86 09f4257b 77d247c7 fe0e5f10; do git branch -a
    --contains $h | grep -c 'origin/claude/loving-archimedes-frtf5q-55b-ci'; done`
    prints `1` four times (`git fetch origin` first, if the branch is not
    known here).
  - §14's figures against decision 9 (runs 44672859855 and 44674948875,
    56.75, 86.12 and 70.30 s, `69 0 2`, `67 0 2`, `63 0 2`, 10.7 s and
    30.3 s), §13's against decision 17, §5's table against Task 2's and
    3's probes, §9's case list against `PROBES`, `CLANG_PROBES` and the
    clang test's three cases on N alone, §11's mutants against part 2's
    23 (1 to 23 in Task 5, Step 8) and against Step 8's outcomes (60
    caught, 37 and 41 by the tests added for them; 28, 31, 33 and 45
    accepted).
- [ ] **Step 3: The checks,** as the conventions list them (the mimic
  turns off `3` items, as at Task 3: Task 4 changes no Rust, so its
  `selfhost.rs` is Task 3's; its `git status
  --short` lists the six records, uncommitted); no code changed, so the
  Rust 1.100 clippy is not run again here.
- [ ] **Step 4: Commit**
      `docs: ADR 0059, the self-hosted driver, build and check, and the
      notes for 5.5b`.

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 7d74c279..ec805f11 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -29,6 +29,9 @@ dump, natively (spec
 Phase 5.5a, the self-hosted `.ll` emitter and the `.ll` dump (spec
 `docs/superpowers/specs/2026-11-18-phase-5-5a-the-ll-emitter-design.md`;
 ADR 0058).
+Phase 5.5b, the self-hosted driver and linking (spec
+`docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`;
+ADR 0059).
 
 ### Added
 
@@ -343,6 +346,40 @@ ADR 0058).
   gave, byte for byte, on every `.nova` file the repository tracks, and
   `nova test` and `nova build` on its 11 packages outside `crates/`; the
   packages under `crates/`, test fixtures, were not compared.
+- **The self-hosted driver, `selfhost build` and `selfhost check`:**
+  `selfhost build FILE [-o OUT] [--release] --std DIR` builds a program
+  as `nova build --release FILE -o OUT` does, on Linux and macOS: its
+  `.ll` written beside OUT as `<out>.nova.ll`, `clang -O2 -c` and `cc` run
+  on it with S0's lines, the runtime found from `NOVA_RUNTIME_LIB` or
+  beside the executable, `built OUT` printed, and S0's messages and exit
+  codes on every failure; without `--release` it builds the same way.
+  It refuses Windows with its own message. `selfhost check FILE --std
+  DIR` prints each diagnostic's code and locus, `error[CODE]: MESSAGE`
+  and `  ┌─ PATH:LINE:COL`, then `ok: FILE` or `Error: found N
+  error(s)`. std is read only from `--std`; there is no project mode
+  (5.5b, ADR 0059).
+- **S1 builds the golden suite, in CI:** a new step of the Selfhost
+  job's Linux leg installs `clang-18` 18.1.3 and runs
+  `s1_builds_the_golden_suite`: the self-hosted compiler, built by the
+  Rust one, builds every golden program the release walk builds with its
+  own `selfhost build`, runs each with the walk's stdin and variables
+  against its `.stdout`, and holds each binary equal, byte for byte, to
+  `nova build --release`'s for the same `-o` basename. The walk's tables
+  are now one file both test files include. A main-step test,
+  `the_ci_step_names_every_clang_test`, holds the step's `--exact` names
+  to the tests `selfhost.rs` ignores as needing `clang`, since a name
+  that matches no test would run nothing (ADR 0059).
+- **Two probe tests** hold N's `build` and `check` to S0's on the same
+  inputs: the same stdout, exit code and files left behind, and N's
+  stderr equal to S0's cut to each diagnostic's header and loci and to
+  anyhow's error. `n_build_and_check_answer_as_s0_does` runs 39 cases
+  with no `clang` in `selfhost.rs`'s main step on all three systems;
+  `n_build_answers_as_s0_does_with_clang` runs 10 with it, and N's
+  `build` with no runtime, with a runtime two directories up and without
+  `--release`, in the new step. What cannot match (clap's usage text,
+  Windows, a signal's status text, `check`'s snippets) is listed in ADR
+  0059. Nothing a user sees changes with 5.5b: S0's sources are
+  unchanged, so `nova build` and `nova check` are too.
 
 ### Changed
 
diff --git a/docs/adr/0059-the-self-hosted-driver-build-and-check.md b/docs/adr/0059-the-self-hosted-driver-build-and-check.md
new file mode 100644
index 00000000..4e7c8b8f
--- /dev/null
+++ b/docs/adr/0059-the-self-hosted-driver-build-and-check.md
@@ -0,0 +1,792 @@
+# ADR 0059 — The self-hosted driver, `build` and `check`
+
+## Status
+
+Accepted (2026-11-20). Branch `claude/loving-archimedes-frtf5q-55b`
+(`docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`),
+written as 5.5b's prototype was built. CI's figures are §14's, from
+the prototype's draft pull request Sakeerin/nova#146 (measurement only,
+closed unmerged); no line was passed.
+
+Part 1 built what §1 to §3 and §8 record, on the success path:
+- §1, N's command lines;
+- §2 and §3, `build`'s steps, `clang`, the runtime and `cc`;
+- §8, the golden tables shared by two test files,
+  `s1_builds_the_golden_suite`, and the CI steps.
+
+Part 2 built what §4 to §7 and §9 to §11 record:
+- §4, the Windows refusal;
+- §5, every message and exit code;
+- §6, `check`'s lines and the extraction of S0's stderr;
+- §7, what N does not match;
+- §9, both probe tests;
+- §10, N's `@test`s;
+- §11, the mutants.
+
+Part 3 measured S1 building N with its own `build` (§13) and wrote the
+records: §12 to §17 and the Consequences, with the figures of parts 1
+and 2 (§12).
+
+The plan's mutants and its fact-check added three things after part 3:
+- §9, the clang test's case of a runtime two directories up, and its
+  report counting the cases that ran;
+- §8, a main-step test that holds CI's step to the ignored tests;
+- §11, the plan's mutants, and the two that these additions kill.
+
+## Decision
+
+### 1. The command lines (the user's decision 5)
+
+```
+selfhost build FILE [-o OUT | --output OUT] [--release] --std DIR
+selfhost check FILE --std DIR
+```
+
+N's `main.nova` dispatches `build` and `check` to a new
+`selfhost/src/driver.nova` (803 lines, 596 of code; code lines exclude
+blank lines and lines that start with `//` after their indentation).
+`build_args` and `check_args` read the command line (spec §3.1):
+- **Each option once, in any order:** `-o OUT`, `--output OUT`,
+  `--output=OUT`, `--release`, `--std DIR`, `--std=DIR`, and one FILE.
+  `--` ends the options, and `-` alone is a FILE.
+- **Refused:** a second FILE or option, an unknown option,
+  `--release=…`, no FILE, no `--std`, an option without its value, an
+  empty value, and a separate value that starts with `-` but is not `-`
+  itself (clap takes that for an option). `-oX` and `-o=X` are unknown
+  options. `check` takes neither an output nor `--release`.
+- **A refusal** prints one line on stderr, `usage: selfhost build FILE
+  [-o OUT] [--release] --std DIR` or `usage: selfhost check FILE --std
+  DIR`, nothing on stdout, and exits 2. clap's exit code is matched; its
+  text is not (§7).
+- **The default output** is FILE's stem, relative, as S0's
+  `PathBuf::from(stem)`; `out` when FILE has no stem. The helper,
+  `file_stem_or`, is new in `loader.nova`, and `module_name` is now
+  `file_stem_or(path, "main")`. No `.exe`: N never links on Windows.
+
+**`--std` is required, never searched for** (spec §3.1, not asked): S0
+embeds std, so there is nothing to mirror; N's other commands take
+`--std`, and every harness passes it. This narrows the phase plan's
+2026-11-02 amendment to §5.5, "finding std next to the executable or in
+`$NOVA_HOME`": N finds std only where `--std` says.
+
+### 2. `build`, step by step
+
+`build_command`, in S0's order (spec §3.2):
+1. The command line (§1), then the Windows refusal (§4).
+2. **std,** read from `--std`. Unreadable: N's existing line, `error:
+   cannot read std's PATH: KIND`, exit 2, as `selfhost mir` gives it (§17).
+3. **FILE, loaded** as `selfhost mir` loads it: a loose program, or a
+   module directly in a package's `src/` or `tests/`, with its path
+   dependencies, its `@test` functions stripped. Unreadable: `failed to
+   open FILE`, caused by the `IoError`'s message, Rust's text. A manifest
+   N's reader refuses: `Error: selfhost build cannot read FILE: WHY`, exit
+   1 (differs: S0 syncs).
+4. **Resolved, checked and lowered in full** (`compile_root`), std's
+   bodies with it, as S0's `build` checks (`FrontendContext::check`), not
+   `selfhost mir`'s shared-std path (§17). Diagnostics print as §6 says,
+   warnings too; with errors, `could not compile due to N previous
+   error(s)`, exit 1.
+5. **Emitted** by N's `compile_ir` (5.5a). Its error: `internal LLVM IR
+   generation error (this is a compiler bug)`, caused by its words; no
+   `.ll` is written.
+6. **The `.ll` written** to `intermediate(out, "ll")`, `<out's file
+   name>.nova.ll` beside OUT, `out.nova.ll` for an OUT with no file name,
+   as S0's `intermediate`; `-o out.ll` gives `out.ll.nova.ll`. Not
+   written: `failed to write <ll>` and the message, exit 1.
+7. **`clang`, the runtime and `cc`** (§3), on `<out's file name>.nova.o`.
+8. **Clean-up:** the object is removed whatever happened, an error
+   ignored. On success the `.ll` is removed too, `built OUT` is printed
+   on stdout, and the exit code is 0. On any failure after step 6 the
+   `.ll` is kept, under the release head `release build failed; generated
+   LLVM IR left at <ll>`.
+
+**Without `--release`** (the user's decision 6) `build` runs the same
+steps, `-O2` included, so it gives the same binary and the same
+messages. S0 without `--release` uses Cranelift, another binary, and no
+release head on its failures. Recorded (differs).
+
+### 3. `clang`, the runtime and `cc`
+
+`compile_and_link` (spec §3.3), as S0's `compile_ir_to_object` and
+`link_executable` run them:
+- **`clang`:** `$NOVA_CLANG` or `clang`, available when `run(clang,
+  ["--version"])` is `Ok`, whatever its status. Then `clang -O2 -c LL -o
+  OBJ`, with and without `--release`; what it prints (a
+  `-Woverride-module` warning on every `.ll`) is dropped on success.
+- **No `llc`** (not asked). `NOVA_LLC` is not read. Without `clang`,
+  N gives S0's words verbatim, "no LLVM toolchain found for `--release`:
+  install LLVM (so `clang` or `llc` is on PATH), or set NOVA_CLANG /
+  NOVA_LLC to one". Recorded (differs): with `llc` and no `clang`, S0
+  builds and N does not.
+- **The runtime is looked for after `clang` has run,** as S0 looks for
+  it, so a missing runtime gives S0's messages at the cost of one compile
+  in that error alone (`find_runtime`):
+  1. `$NOVA_RUNTIME_LIB`; when set it must exist, else `NOVA_RUNTIME_LIB
+     points to X, which does not exist`;
+  2. no embedded runtime (the user's decision 5);
+  3. `current_exe()` (failing: `locating the nova executable` and the
+     message), then `libnova_runtime.a` in its directory, its parent and
+     its grandparent. S0's `build/` rule, for cargo's test binaries, is
+     not ported. None: S0's words verbatim, "could not find the Nova
+     runtime library (libnova_runtime.a) near EXE, and this nova does not
+     carry one. …", with N's own path.
+- **`cc`:** `$CC` or `cc`, run as `-o OUT OBJ RUNTIME -lpthread -ldl
+  -lm`, S0's `link_cc` line, on macOS too.
+- **A spawn failure** has two causes, `failed to spawn clang: "NAME"` (or
+  `linker`) and the system's message, NAME quoted as Rust's `{:?}` quotes
+  it for the names N meets: in `"`, with `"` and `\` escaped
+  (`debug_quoted`).
+- **A non-zero status** has one cause (`status_cause`): `clang failed
+  (exit status: N):` (or `linking failed`), then the tool's stdout and
+  its stderr, each as text and trimmed, on lines of their own, empty ones
+  too.
+- **A tool's output as text** (`tool_text`): `Bytes.to_string()`; when
+  that is `None`, the ASCII bytes kept and U+FFFD for each other byte.
+
+### 4. Windows (the user's decision 4)
+
+N's std has no OS query. **N knows Windows by `current_exe()`'s shape**
+(`windows_shape`, not asked): an ASCII letter and `:` (a drive), or
+`\\` (a `\\?\` or UNC prefix), at its start; Unix's absolute paths start
+with `/`. No inherited state is read.
+- `build` checks it after the command line, before std or FILE is read,
+  nothing written: `Error: selfhost build links on Linux and macOS only`,
+  exit 1.
+- If `current_exe()` fails, N is taken to be on Unix, and `build` goes on
+  to §3's own error.
+- `check` does not link, and runs on Windows.
+
+**Unverified on Windows locally.** The main probe test (§9) checks the
+refusal on CI's Windows leg, and an `@test` pins the shapes (`C:\x`,
+`\\?\C:\x`, `/x`, `x`).
+
+### 5. Messages and exit codes, against S0's
+
+One helper, `fail(head, causes)`, prints anyhow's layout (`failure_text`):
+`Error: HEAD`; when there are causes, a blank line, `Caused by:`, and
+each cause's lines indented four spaces, empty ones too; the causes
+numbered `0:`, `1:` when there are several, a numbered cause's later
+lines indented seven. Then exit 1. Each row below is S0's, byte for
+byte, on the probe set (§9), but where it says it differs:
+
+| Case | stdout | stderr | exit |
+|---|---|---|---|
+| built | `built OUT` | warnings' lines | 0 |
+| `check` clean | `ok: FILE` | warnings' lines | 0 |
+| usage | | N's one line (differs: clap's text) | 2 |
+| Windows, `build` | | `Error: selfhost build links on Linux and macOS only` (N's own) | 1 |
+| FILE missing or a directory | | `failed to open FILE`, caused by Rust's text (`os error 2` or `21`) | 1 |
+| compile errors | | §6's lines, then `could not compile due to N previous error(s)` (`build`) or `found N error(s)` (`check`) | 1 |
+| emitter error (float `%`) | | `internal LLVM IR generation error (this is a compiler bug)` and its words; no `.ll` left | 1 |
+| `.ll` not written (`-o nodir/x`) | | `failed to write nodir/x.nova.ll` and the message | 1 |
+| no `clang` | | the release head and S0's cause | 1 |
+| `clang` fails | | the release head and `clang failed (exit status: N):` with its output | 1 |
+| runtime missing, a text file, or not found | | the release head and S0's cause; with a text file, the linker's failure and its output, the same on both sides | 1 |
+| `cc` missing / fails | | the release head and `0: failed to spawn linker: "…"`, `1: …` / `linking failed (exit status: N):` and its output | 1 |
+| manifest refused | | `Error: selfhost build cannot read FILE: WHY`, or `selfhost check` (differs: S0 syncs) | 1 |
+| std unreadable | | `error: cannot read std's PATH: KIND` (N's own; S0 embeds std) | 2 |
+
+### 6. `check`'s lines, and the extraction of S0's stderr (the user's decision 3)
+
+`check_command`, as S0's file-mode `nova check`: the command line, std,
+FILE loaded and checked as `build` does, and lowered only when the
+program runs, as S0's `check_program_counted_for` decides it (a loose
+program, or a package's `src/main.nova`); a package module that does
+not run is not lowered. Float `%` gives `ok:`, as S0's.
+
+**N's lines** (`diagnostic_lines`), per diagnostic, in N's order, on
+stderr:
+- `SEVERITY[CODE]: MESSAGE`, codespan's header, the severity lowercase,
+  and no brackets when there is no code;
+- then one `  ┌─ PATH:LINE:COL` line, with two spaces, per file the labels
+  name, in the order the labels first name the files;
+- no snippet, note, help or blank line.
+
+Then `ok: FILE` on stdout, exit 0, or `Error: found N error(s)`,
+counting errors and not warnings, exit 1.
+
+**The locus rule** (`loci`; codespan-reporting 0.11.1, `term/views.rs`):
+a file's locus is its earliest primary label, else its earliest
+secondary; of two labels with one start and one style, the first.
+
+**Line and column** (`line_and_column`), from the label's start, a byte
+offset in its module's text:
+- line = 1 + the `\n` bytes before it, so `\r\n` counts once;
+- column = 1 + the bytes in `[line start, offset)` that are not UTF-8
+  continuation bytes (`0x80` to `0xBF`): a tab is one column, and so is a
+  Thai mark, a CJK character or an emoji.
+
+**PATH** is the name S0's `FileDb` gives: FILE as given; for a sibling or
+a dependency, the path N's loader joined and read; `<std/NAME>` for std.
+Confirmed on the probes: `d/b.nova` under `check d/a.nova`, `./d/b.nova`
+under `check ./d/a.nova`, `05-geometry/shapes/src/units.nova:27:26` for
+the path dependency's error, and `<std/strings>:193:6`. std's text is
+read from `--std` only when a label names it (`StdModule` keeps none),
+and a locus in a std file N cannot read is left out (§17). A label whose
+file is unknown is left out, as S0's renderer leaves it out.
+
+**The extraction** (`extracted_stderr`, in `selfhost.rs`): S0 runs with
+`NO_COLOR=1`, and its stderr is cut to
+- the lines that begin `error[`, `warning[`, `note[` or `help[`;
+- the lines whose first non-blank text is `┌─ `, re-indented to two
+  spaces, a `\` in their path read as `/` on Windows, as `settle_paths`
+  reads it for E0004;
+- and every line from the first that begins `Error: ` to the end:
+  anyhow's error and its causes, which span several lines (§17).
+
+N's stderr must equal the extraction byte for byte; stdout and exit
+codes must be equal. A message holding a line break would lose its
+second line in the extraction and fail the comparison loudly; none
+does today.
+
+### 7. What N does not match (spec §3.6)
+
+Recorded as (differs), and kept out of the probe set's comparison:
+- clap's usage text; the exit code, 2, and the empty stdout are compared,
+  and N's own line is pinned;
+- no FILE: S0 finds a project or `src/main.nova`; N exits 2;
+- project mode, the registry sync, `--target`, `--ssr`, `--ssg` and
+  `llc` (spec §15);
+- the no-runtime message's path, N's own executable's (the clang probe
+  test checks N's against `beside_exe`'s words with N's path);
+- Rust's `ExitStatus` text for a signal (`signal: 9 (SIGKILL)`), where
+  N's std gives `128 + n` and N prints `exit status: 137`;
+- a program name that `{:?}` escapes beyond `"` and `\`;
+- tool output with invalid UTF-8 that also holds valid multi-byte
+  characters, or with whitespace outside N's `trim` at its ends;
+- Windows: `build` refused (§4), and a locus's `\` read as `/`;
+- a Nova panic, an abort with 134, against Rust's 101;
+- `check`'s snippets, notes, help and colour.
+
+### 8. The harness: the shared tables, `s1_builds_the_golden_suite`, CI
+
+**The shared tables** (spec §6.1): `crates/nova-cli/tests/golden/shared.rs`
+(250 lines), included by `#[path = "golden/shared.rs"] mod golden;` in
+`run_tests.rs` and `selfhost.rs`. It holds `RELEASE_GOLDEN_EXCLUDED`,
+`_STDIN`, `_ENV`, `_STATUS`, `_LIMIT` and `_STDERR`,
+`PROCESS_TEST_VALUE` and `PROCESS_TEST_EQ`, `quoted_stderr`,
+`run_within_limit`, `run_release_binary`, `golden_programs()` (the walk,
+its table checks and the floor of 131), and two helpers part 1 added so
+the two walks share them: `golden_dir()` and `run_golden()`, one
+program's run and judgement. `run_tests.rs` lost 213 lines and gained
+12; `release_golden_suite` still gives 147 passed, 9 excluded.
+
+**`s1_builds_the_golden_suite`** (`selfhost.rs`; spec §6.2; the user's
+decisions 1, 7 and 8a), `#[ignore = "needs clang; the Selfhost job's
+Linux step runs it"]`:
+1. `clang` is `NOVA_CLANG` or `clang`, probed by `--version`; if it does
+   not spawn, the test fails under `NOVA_REQUIRE_CLANG=1` and otherwise
+   prints `s1 golden: not run (no clang)` and passes.
+2. S1 is `selfhost()`, built by Cranelift as every test of the file
+   builds it (`build_selfhost`).
+3. One runtime for both sides (`golden_runtime`): `NOVA_RUNTIME_LIB` if
+   set; else the only runtime `build_selfhost` unpacked under its fresh
+   `NOVA_HOME`; else the one beside S0. Passed to both as
+   `NOVA_RUNTIME_LIB` and named in the report.
+4. Per program, 4 threads: `selfhost build SRC -o DIR/s1/NAME --release
+   --std STD`, whose stdout must be exactly `built DIR/s1/NAME`; S0's
+   `nova build --release SRC -o DIR/s0/NAME`, one basename; both with
+   `NO_COLOR=1`, `RUST_BACKTRACE=0` and `RUST_LIB_BACKTRACE=0`, within
+   `RELEASE_GOLDEN_LIMIT`. S1's binary runs as `release_golden_suite`
+   runs S0's (`run_golden`): its stdout must equal its `.stdout`, and its
+   status the table's.
+5. The two binaries are read and compared byte for byte
+   (`same_binaries`; the user's decision 8a, no hash crate); a
+   difference names both sizes and the first differing offset.
+6. Each program's two binaries, any `.ll` or `.o`, and its temporary
+   directory are removed right after the comparison, and `DIR` at the
+   end. The report: `s1 golden: 147 passed, 9 excluded, <clang
+   version>, runtime <path>`.
+
+It was seen red first: all 147 failed on N's usage line of that time,
+`usage: selfhost tokens|ast FILE...`, exit 2.
+
+**CI** (spec §7), two steps in the Selfhost job after `cargo test
+--test selfhost`, on its Linux leg alone (`if: runner.os == 'Linux'`):
+- `Install clang 18.1.3 (Linux)`, the Test job's step verbatim,
+  asserting 18.1.3 (ADR 0048);
+- `S1 builds the golden suite (Linux)`, with `NOVA_REQUIRE_CLANG=1` and
+  `NOVA_CLANG=clang-18` and no `NOVA_RUNTIME_LIB`: `cargo test --locked
+  --workspace --all-features --no-fail-fast --test selfhost -- --ignored
+  --exact s1_builds_the_golden_suite
+  n_build_answers_as_s0_does_with_clang`. Its S1 is rebuilt in the
+  step's own process, and `golden_runtime` finds the release runtime its
+  `build_selfhost` unpacked (`NOVA_EMBED_RUNTIME=1`). The step has no
+  time line; its time is recorded (§14).
+
+**`the_ci_step_names_every_clang_test`** (`selfhost.rs`), in the main
+step on all three systems, holds that line to this file. With `--exact`,
+a name that matches no test runs nothing and the step still passes. So
+the test reads `ci.yml` from the repository and `selfhost.rs`'s own
+source (`include_str!`), as text, CRLF line endings allowed. It finds
+the step `S1 builds the golden suite (Linux)` and the names after
+`--exact` on its `run:` line, and asserts:
+- each name is a test of the file, marked `#[ignore]`;
+- the names are exactly the tests marked `#[ignore = "needs clang;
+  ..."]`, today `s1_builds_the_golden_suite` and
+  `n_build_answers_as_s0_does_with_clang`.
+
+It needs no crate, no network and no N, and takes under 10 ms.
+
+### 9. The probe tests (the user's decision 8b)
+
+S0 against N, each case in its own fixed directory under the test's,
+the inputs written afresh into each, a fresh `NOVA_HOME`, `NO_COLOR=1`,
+`RUST_BACKTRACE=0` and `RUST_LIB_BACKTRACE=0` on both sides; stdout, the
+exit code, §6's extraction of stderr, and the files each side leaves, a
+`.ll` among them, byte for byte. Each side's files are compared, then
+removed before the other side runs.
+
+**`n_build_and_check_answer_as_s0_does`,** in the main step on all three
+systems, 4 threads, 39 cases:
+- **19 `check` cases:** an unknown name with a help (E0001); a duplicate
+  whose secondary label comes first (E0002); spans of several lines
+  (E0010); after tabs; after CJK, an emoji and Thai; Thai combining
+  marks; a warning alone (`ok:`); two errors and a 2-digit line; `\r\n`
+  line endings; a sibling's error, from `d/` and from `./d/`; labels in
+  two files (E0074, the primary in the second); a label in
+  `<std/strings>`; a path dependency's error (a copy of
+  `examples/05-geometry`, an error appended to its library's
+  `src/units.nova`); a parse error; a clean program; float `%`; a missing
+  FILE; a directory as FILE.
+- **9 `build --release` cases:** compile errors; a missing FILE; a
+  directory as FILE; `-o nodir/x`; no `clang` (`NOVA_CLANG` and
+  `NOVA_LLC` naming a missing file) with `-o`, with the default name, and
+  with a warning, each leaving both `.ll`s, equal; no `clang` and no
+  runtime; float `%`, no `.ll`.
+- **7 usage cases,** compared on exit 2 and empty stdout, N's line
+  pinned: `build` with an unknown option, `-o` twice, a second FILE,
+  `--release=yes`; `check` with an unknown option, a second FILE, `-o`.
+- **4 cases on N alone,** which S0 has no form of: no `--std` and no FILE,
+  for `build` and for `check`.
+
+On Windows a case that builds is N's refusal alone (exit 1, nothing on
+stdout, nothing left), and S0 is not run for it. The extraction has a
+unit test of its own, `the_extraction_keeps_the_headers_the_loci_and_the_error`.
+
+**`n_build_answers_as_s0_does_with_clang`,** ignored, run in the CI step
+of §8, 10 cases with the golden walk's runtime as `NOVA_RUNTIME_LIB`
+where a case names none:
+- success with `-o`, with the default name from a relative FILE, and
+  with a warning; no `.ll` or `.o` left beside a binary built, and the
+  binaries equal;
+- `NOVA_CLANG=false`; `NOVA_CLANG=false` with `NOVA_RUNTIME_LIB` missing
+  (`clang`'s failure is the one reported, since the runtime is looked for
+  after `clang`);
+- `CC=false`; `CC` naming a missing file;
+- `cc`'s command line: `CC` a script that writes its arguments to
+  `cc-args.txt` and runs `cc` (`CC_ARGS`, Unix only), the file compared;
+- `NOVA_RUNTIME_LIB` naming a missing file, and naming a text file.
+
+Then, on N alone:
+- **no runtime:** N copied alone into a fixed directory, none in it or
+  the two above, `NOVA_RUNTIME_LIB` unset, against `beside_exe`'s words
+  as written there with N's path (CI's S0 carries a runtime);
+- **without `--release`:** the same stdout and the same binary as with
+  it (the user's decision 6);
+- **a runtime two directories up:** a copy of N in `up/bin` under that
+  fixed directory, the runtime beside `up`, `NOVA_RUNTIME_LIB` unset:
+  N builds and prints `built up`, as S0's `beside_exe` looks in the
+  executable's directory, its parent and its grandparent (spec §3.3);
+- **the harness's own checks:** on the binary built, `run_golden` refuses
+  a wrong stdout (`arrays`'s, and `fs_already_exists`'s `cleaned`, which
+  a check looser than equality would take), and `same_binaries` refuses
+  a binary one byte off, at that offset.
+
+Each case counts once as it runs, and the report gives "N of M" out of
+the cases that ran: 16 when all run, which is the 10 above, the three on
+N alone, and the harness's three refusals. Those three run only once
+`build` without `--release` has passed.
+
+**Fixed temporary names,** each under the temporary directory and
+removed by its test at the end: `nova-selfhost-golden`,
+`nova-selfhost-probe`, `nova-selfhost-probe-clang`,
+`nova-selfhost-alone`. Part 3's measurement used two more, outside any
+test (§13).
+
+### 10. N's `@test`s
+
+`selfhost/src/driver_tests.nova` (214 lines, 163 of code), 11 `@test`s,
+run under `the_selfhost_tests_pass` on both runtimes:
+`the_intermediate_is_named_for_the_outputs_file_name_as_s0_names_it` (a
+trailing `/`, an output with no name, `-o out.ll`);
+`the_default_output_is_files_stem_as_path_file_stem_gives_it`
+(`a.b.nova`, `.nova`, `x.`);
+`windows_is_known_by_a_drive_or_a_double_backslash`;
+`the_command_lines_take_what_the_spec_says_and_refuse_the_rest`;
+`a_failure_is_printed_in_anyhows_layout` (no cause, one, one with empty
+lines, two);
+`line_and_column_count_line_feeds_and_characters_as_codespan_does`
+(tab, `\r\n`, multi-byte);
+`a_files_locus_is_its_earliest_primary_label_else_its_earliest_secondary`
+(the earliest primary against an earlier secondary, two files);
+`a_diagnostic_prints_its_header_and_one_locus_per_file`;
+`a_tools_name_is_quoted_as_debug_quotes_it`;
+`a_failed_tools_cause_is_its_status_line_and_its_output_trimmed`;
+`a_tools_output_that_is_not_utf8_keeps_its_ascii_and_marks_each_other_byte`.
+N's `nova test` passes 119 (108 before), on the debug runtime and on the
+release runtime. `selfhost/src` is now 57 files and 32,356 lines.
+
+### 11. The mutants
+
+Each was applied in the working tree, the listed tests run, then
+reverted; none was committed. **All 23 were killed:**
+
+| Mutant | Edit | Killed by |
+|---|---|---|
+| M1 | the `.ll` named `NAME.ll`, not `NAME.nova.ll` | main probe (5 cases), an `@test`, clang probe (9) |
+| M2 | `clang` without `-O2` | clang probe (3), golden (147 of 147 differ) |
+| M3a | the runtime looked for before the `clang` probe | main probe (the 4 no-`clang` cases), clang probe (1) |
+| M3b | the runtime looked for between the `clang` probe and its run | clang probe: "clang fails, and no runtime" |
+| M4a | `cc` without `-lm` | clang probe: "cc's command line" |
+| M4b | `cc` without `-lpthread` | clang probe: "cc's command line" |
+| M4c | the runtime before the object on `cc`'s line | clang probe (5) |
+| M5 | the default output FILE's name, not its stem | main probe (1), an `@test`, clang probe (1) |
+| M6 | one line break before `Caused by:` | main probe (10), an `@test` |
+| M7 | a failure exits 2 | main probe (25); N's `@test`s do not call `fail` |
+| M8a | the drive's `:` test on `;` | an `@test` (on Linux and macOS; CI's Windows leg would also fail the main probe) |
+| M8b | `/` taken for Windows' `\\` | main probe (9), an `@test` |
+| M9 | the `.ll` kept on success | clang probe (4) |
+| C1 | the column counts bytes | main probe (the wide and Thai cases), an `@test` |
+| C2 | `\r` counted as a line break | main probe (CRLF), an `@test` |
+| C3a | a file's first label is its locus | **an `@test` alone** |
+| C3b | the later start wins | **an `@test` alone** |
+| C3c | the files' loci reversed | main probe (two files, std), two `@test`s |
+| C4 | `Error` for `error` in a header | main probe (15), an `@test` |
+| H1 | the binaries never compared | the clang probe's harness check; `s1_builds_the_golden_suite` alone does not kill it, since S0's and S1's binaries are equal anyway |
+| H2 | the stdout check loosened to `contains` | the clang probe's harness check on `fs_already_exists` |
+| H3a | the extraction without `warning[` | main probe (2), the extraction test |
+| H3b | loci not re-indented | main probe (5), the extraction test |
+
+**Four survived until a case was added:**
+- **M3b** survived until "clang fails, and no runtime" was added: no case
+  of spec §6.3 reaches a runtime missing with a `clang` that fails.
+- **M4a and M4b** survived until "cc's command line" was added: the
+  binaries stay byte-equal without `-lm` or `-lpthread`, which glibc
+  2.34's libc (and macOS's libSystem) already holds.
+- **H2** survived until `fs_already_exists` joined the harness check,
+  which had run only a binary whose output no `.stdout` held.
+
+The locus rule's two mutants (C3a, C3b) are killed by the `@test` alone:
+no diagnostic from source has a file whose first label is not its
+locus (S0 puts the primary first in E0002 and E0074), so no probe input
+reaches the rule (§17).
+
+**The plan's mutants** (2026-11-20, all at the split's tip, the tree
+of the prototype's head) are 64:
+- part 2's 23 again;
+- 20 of part 1's checks: the shared tables, the golden harness, the
+  usage line, `clang`'s and `cc`'s lines, the runtime's search, the
+  default stem, std's moved loading, and three of CI's step;
+- 21 of the words spec §3.4 lists.
+
+Two of them survived until a test was added:
+- **R2, the plan's mutant 37:** the runtime looked for in N's own
+  directory alone. It survived until the clang test's case of a runtime
+  two directories up was added (§9): every other case names the runtime
+  or has none near.
+- **CI1, the plan's mutant 41:** a name misspelt in CI's step. With
+  `--exact` the step ran one test and passed. It survived until
+  `the_ci_step_names_every_clang_test` was added (§8), which also kills
+  a name dropped from the step (63) and `#[ignore]` removed from the
+  clang test (64).
+
+Four are accepted:
+- **T5** (an environment entry dropped) is equivalent: `env` refuses
+  names that hold `=`.
+- **H5** (S1's `built OUT` line not checked) is covered by the clang
+  test, which compares that line.
+- **A1** (`clang`'s arguments in another order) gives the same command.
+- **W4** (the Windows refusal's words) is unreachable on Linux, and CI's
+  Windows leg catches it.
+
+### 12. The figures of parts 1 and 2
+
+Measured locally on 4 cores, `clang-18` 18.1.3, `cc` gcc 13.3, GNU ld
+2.42; S0 at opt-level 0 unless said; S1's golden runs on the release
+runtime (`NOVA_RUNTIME_LIB`, `0.4.0-be7e8b48`) unless said. Scripts and
+logs are outside the repository, not committed.
+
+**`s1_builds_the_golden_suite`** (part 1): 147 passed, 9 excluded, every
+binary byte-equal to S0 `--release`'s.
+
+| What | Figure |
+|---|---|
+| the test, wall (its `finished in`) | 49.7 s (44.38 s), 43.2 s (42.99 s) |
+| S1's 147 builds alone, 4 threads | 16.5 s wall, 60.5 s CPU |
+| S0 `--release`'s 147 builds alone, 4 threads | 16.4 s wall, 59.0 s CPU |
+| the chain by hand (`mir --ll`, `clang`, `cc`) | front end 26.0, `clang` 16.4, link 21.3 s CPU; 17.9 s wall |
+| S1's own build (Cranelift; its runtime not recorded) | 8.85 s, 9.00 s |
+
+**`selfhost.rs`'s main step** ("as CI runs it" is
+`NOVA_EMBED_RUNTIME=1 CARGO_PROFILE_DEV_OPT_LEVEL=1`; `finished in`):
+
+| Comparison | Style | Before | After |
+|---|---|---|---|
+| part 1 against the spec's base `17ecca86`, one session, not interleaved | plain | 216.78 s (67 passed) | 210.88 s (67 passed, 1 ignored) |
+| part 1 | as CI runs it | 55.01 s | 55.13 s |
+| part 2 against part 1's head `09f4257b`, interleaved | plain | 208.65, 211.08 s (67, 1 ignored) | 221.77, 227.85 s (69, 2 ignored) |
+| part 2 | as CI runs it | 58.13, 66.10 s | 70.36, 71.67 s |
+| part 2, the main probe test alone | as CI runs it | | 3.43, 3.37 s |
+
+Part 1 adds nothing measurable to the main step: its test is ignored
+there. Part 2 adds about 15 s plain and about 9 s as CI runs it, against
+the spec's projected 1 to 3 s (§17); the runs as CI runs them are noisy,
+and the probe test costs more under the other tests' load than alone.
+
+**The CI step, locally, as CI runs it** (no `NOVA_RUNTIME_LIB`, the
+runtime `build_selfhost` unpacked, the same checksum as the release
+runtime above; wall): part 1, the golden test alone, 33.8 s and 32.4 s;
+part 2, both ignored tests, 39.3 s and 39.1 s (`finished in` 39.04 and
+38.89 s). The spec projected about 40 s locally.
+
+**Verification** (part 2, at `77d247c7`): the full suite (`cargo test
+--workspace`, Node required) 2,837 passed, 1 failed (the IPv6 test, as on
+the base), 10 ignored, the base's figures, since `selfhost.rs` is `test =
+false`; `SH` 69 passed, 2 ignored; `release_golden_suite` 147 passed, 9
+excluded; `cargo fmt --check`, both clippy commands as CI runs them,
+`nova fmt --check std examples selfhost` and Rust 1.78's `cargo check
+--workspace` with `-D warnings` clean (its test targets cannot build on
+1.78, as before 5.5b: `assert_cmd` needs edition 2024); `Cargo.lock` and
+the manifests unchanged.
+
+### 13. S1 builds N, and S2 = S1 (part 3; spec §11 item 4)
+
+For 5.6, a measurement only; no test was added. Measured 2026-11-20 at
+`ecfcd0f8`, part 3's head, whose `selfhost/` and S0 sources the later
+commits leave unchanged; every build on the release runtime above, through
+`NOVA_RUNTIME_LIB`, with `NOVA_CLANG=clang-18`. `/usr/bin/time` is not
+installed on this machine, so a small wrapper measured each command:
+wall time, peak RSS as `wait4`'s `ru_maxrss` (what GNU `time -v`
+reports: the largest of the process and the children it waited for,
+`clang` and `cc` among them), and the process's own `VmHWM`, polled
+every 20 ms.
+
+**S1.** S1 is normally built by S0's `nova build` in `selfhost/`, by
+Cranelift, without `--release` (`build_selfhost`); on CI it links the
+release runtime its S0 embeds, and locally the debug one beside S0. The
+S1 that built N here was built that way, linked against the release
+runtime: 7.85 s, 102 MB, 9,376,184 bytes.
+
+**S1 builds N with its own `build`:** `selfhost build
+selfhost/src/main.nova -o nova-selfhost-s2/selfhost --release --std std`
+(the fixed directory `nova-selfhost-s2` in the temporary directory, kept;
+its binary is S2) exits 0 and prints `built …/selfhost`:
+
+| Run | Wall | Peak RSS (`ru_maxrss`) | S1's own `VmHWM` |
+|---|---:|---:|---:|
+| 1 | 13.29 s | 203.4 MB | 203.6 MB |
+| 2 | 12.97 s | 203.5 MB | 203.6 MB |
+
+Its split, one run each: the front end and the `.ll` written (`NOVA_CLANG`
+naming a missing file, so N stops at the `clang` probe) 1.77 s;
+`clang-18 -O2 -c` on the `.ll` 11.08 s, 200.3 MB; `cc` 0.15 s, 80 MB.
+So `clang` is about 85% of the build, and S1's own front end is the
+peak by about 3 MB. The `.ll` of N's root is 12,759,478 bytes, 406,232
+lines; ADR 0058 §11 counted 12,252,987 bytes for N's root at 5.5a, its
+`(Ll ok)` line included, before the driver.
+
+**S2 = S1 with one basename.** S0's `nova build --release
+selfhost/src/main.nova -o nova-selfhost-s1r/selfhost` (the sibling fixed
+directory, kept; its binary is the `--release` S1) took 11.74 s and
+12.36 s, S0's own peak 123.5 MB and `clang`'s 200.3 MB. **The two
+binaries are equal, byte for byte:** 9,354,680 bytes each, one SHA-256.
+Their `.ll`s, removed on success by both, were made again with
+`NOVA_CLANG` and `NOVA_LLC` naming a missing file, the same `-o`
+basename in two other directories: equal byte for byte. The Cranelift
+S1 (9,376,184 bytes) is a different binary from the `--release` one, as
+expected; what it builds is not: the research (spec §2 item 6) found
+the Cranelift- and the `clang`-built S1 giving the same binaries, and
+here the Cranelift S1's S2 is the `--release` S1.
+
+**S2 runs as S1 does.** On 24 commands, S1 and S2 gave the same stdout,
+stderr and exit code (each side's output directory's path normalised):
+- `check` on `tests/runtime/generics.nova`, `examples/05-geometry`'s
+  main, and N's own root (`ok:`);
+- `check` on ten of the probe set's inputs and a missing FILE;
+- `build --release` of float `%` and of compile errors; two usage
+  errors; `mir --ll` of `generics`;
+- `build --release` of `arrays`, `generics`, `async_tasks`,
+  `string_builder` and `examples/05-geometry`'s main: each pair of
+  binaries equal, and each golden one, run, printing its `.stdout`.
+
+**S2 builds N too:** twice, 12.84 s and 12.76 s, its own `VmHWM` 203.7
+MB, and the binary equal to S2, byte for byte, so S2 = S3 here; S2's own
+front end, measured as S1's, took 1.70 s. S2 is built by `clang -O2`,
+S1 by Cranelift, yet S2 builds N no faster: `clang` dominates.
+
+### 14. CI's figures
+
+From the prototype's draft pull request Sakeerin/nova#146 (measurement
+only, closed unmerged), two runs:
+- run 44672859855 on `77d247c7` (part 2's head);
+- run 44674948875 on `fe0e5f10`, the same code with `CC_ARGS` gated.
+
+**`selfhost.rs`'s main step** (`finished in`; passed, failed, ignored),
+against the Selfhost job's lines (ADR 0057 §19). The margin is the line
+minus the time:
+
+| System | Line | Run 44672859855 | Run 44674948875 | Counts |
+|---|---:|---:|---:|---|
+| Linux | 80 s | 56.35 s (margin 23.65) | 56.75 s (margin 23.25) | `69 0 2` |
+| macOS | 100 s | 71.15 s (margin 28.85) | 86.12 s (margin 13.88) | `67 0 2` |
+| Windows | 110 s | 63.56 s (margin 46.44) | 70.30 s (margin 39.70) | `63 0 2` |
+
+Against 5.5a, the margins were:
+- pull request's run 44475745997: 23.5, 13.5 and 15.5 s;
+- merge on `main`, run 44479648345: 25.1, 32.5 and 43.2 s.
+
+The main probe test's +9 s measured locally (§12) does not show above
+CI's run-to-run swing: macOS moved 15 s between two runs of the same
+code. No line was passed.
+
+**The new steps on Linux:**
+- **Install clang 18.1.3:** 9.1 s and 10.7 s. It prints `Ubuntu clang
+  version 18.1.3 (1ubuntu1)`; the runner already had it installed.
+- **S1 builds the golden suite:** 30.4 s and 30.3 s wall (`finished in
+  30.25s` and `30.21s`): `s1_builds_the_golden_suite` and
+  `n_build_answers_as_s0_does_with_clang`, 2 passed. This is below the
+  39 s measured locally (§12) and the 46 to 57 s the spec projected
+  (spec §10).
+
+**The whole Selfhost job:**
+
+| System | Run 44672859855 | Run 44674948875 |
+|---|---:|---:|
+| Linux | 3:31 | 3:26 |
+| macOS | 3:46 | 4:20 |
+| Windows | 3:52 | 3:55 |
+
+The longest job stays the Test job on Windows: 12:40 in both runs.
+
+**The Clippy job's `windows-latest` leg failed in the first run,** on the
+second clippy command (`cargo clippy --locked --all-features --test
+selfhost -- -D warnings`): `constant CC_ARGS is never used`, because its
+one use is under `#[cfg(unix)]`. The constant is now `#[cfg(unix)]` too,
+and no other item of the probe code is used under one system alone
+(§17). The second run passed all 19 jobs.
+
+**Not measured:** the Test job's totals. Its logs could be read only
+from their tail, and the full log's download was refused.
+
+### 15. The intermediate's name, and what 5.5b leaves to 5.6
+
+**The intermediate's name** (the user's decision 2) is S0's,
+`<out's file name>.nova.ll` beside OUT, with no emitter change. The
+`.ll`'s basename reaches the binary (`STT_FILE`, research §6), so two
+stages give equal binaries only when built with one `-o` basename:
+**5.6 builds every stage with the same `-o` basename** (§13 used
+`selfhost`), in directories of its own. No `source_filename` line, no
+constant name.
+
+**Left to 5.6** (spec §15):
+- S1 == S2 == S3 in CI, its job and its time; §13 is the first
+  measurement;
+- the boundary ADR and `v0.5.0`;
+- and, outside Phase 5's plan unless a spec takes them up: project mode,
+  the library refusal and the registry sync; `--target`, `--ssr`,
+  `--ssg`; `llc`; WASM and test mode's code generation; Cranelift in N;
+  an embedded runtime in N; Windows linking; rendered diagnostics;
+  `check`'s code and span on every input; any std builtin;
+  `source_filename` or any emitter change.
+
+### 16. ADR 0058 §14's questions, answered
+
+5.5b answers each of the six questions ADR 0058 §14 deferred:
+1. **Where S1's golden suite runs:** a new step of the Selfhost job's
+   `ubuntu-24.04` leg, `clang-18` installed there, S1 built by
+   Cranelift, its programs on the release runtime (§8).
+2. **The intermediate's name:** S0's `<out>.nova.ll`, one `-o` basename
+   for every stage (§15).
+3. **N's command line and exit codes:** §1 and §5; std only from
+   `--std` (§1); the runtime from `NOVA_RUNTIME_LIB` or beside the
+   executable (§3); no project mode, so no `current_dir`; stderr compared
+   by §6's extraction.
+4. **`check`'s output:** codes and spans, codespan's header and locus
+   lines (§6), ADR 0048 decision 1; not codespan's layout.
+5. **`build` without `--release`:** `-O2`, as with it (§2).
+6. **Platforms:** Linux and macOS; Windows refused with a message (§4);
+   no OS query in std.
+
+ADR 0058 is not changed.
+
+### 17. Departures from the spec, and decisions it does not settle
+
+**From the spec,** each recorded where it falls:
+- **§10's main-step projection, +1 to 3 s, measured +9 to +15 s
+  locally** (+9 s as CI runs it, +15 s plain; the probe test about
+  3.5 s alone), §12. Against 5.5a's pull request's margins of 23.5,
+  13.5 and 15.5 s on Linux, macOS and Windows this is the first risk;
+  CI's figures are §14's.
+- **§6.3's clang list gained three cases:**
+  - "clang fails, and no runtime" (the runtime's order) and "cc's
+    command line" (`cc`'s arguments), for M3b, M4a and M4b, which no
+    case of the list could kill (§11). The first is on every Unix; the
+    second is Unix only (`CC_ARGS`).
+  - On N alone, "a runtime two directories up" (spec §3.3's search),
+    for R2 (§11).
+- **§9's locus rule** ("the earliest primary against an earlier
+  secondary") **has no source input that reaches it:** only the `@test`
+  covers it (§11, C3a and C3b).
+- **The research's `frem.nova` never hit float `%`:** it wrote `{a % b}`
+  without `$`, so it built; its float `%` row was right only by 5.5a's
+  dump. The probe's `frem.nova` uses `${a % b}`.
+- **The harness's stdout check** first ran a binary whose output no
+  `.stdout` held, so it did not show that it was held to equality (H2);
+  `fs_already_exists` was added (§9).
+- **Local clippy runs only on Linux:** `CC_ARGS`, used only under
+  `#[cfg(unix)]`, was dead code on Windows, and the Clippy job's Windows
+  leg failed (§14); the constant is now `#[cfg(unix)]`. The other two
+  `#[cfg(unix)]` blocks of the probe code use only `std`.
+
+**Decisions the spec does not settle,** each with its cost if wrong
+(all cheap):
+- **`build` checks in full,** as S0's `build` does, not `selfhost mir`'s
+  shared-std path; all 147 binaries are equal either way. About 10
+  lines to switch.
+- **std's loading moved** (`load_std`, `read_std_modules`,
+  `labels_and_texts`, `io_kind_name`) from `main.nova` to
+  `driver.nova`, verbatim and public: Nova forbids import cycles, and
+  `check` needs them too.
+- **std unreadable** is N's existing `error: cannot read std's PATH:
+  KIND`, exit 2, as `selfhost mir` gives it; spec §3.4 has no row.
+- **The command line's edges** (§1): an empty value and a separate value
+  that starts with `-` refused; `-` alone a FILE; `-oX` and `-o=X`
+  refused. If wrong, a different exit code from clap's on unusual forms.
+- **A refused manifest's head names FILE,** `Error: selfhost build
+  cannot read FILE: WHY`, and `check`'s mirrors it, `Error: selfhost
+  check cannot read FILE: WHY`, exit 1.
+- **The CI line names both ignored tests,** as spec §7 writes it, and
+  `the_ci_step_names_every_clang_test` holds it to the tests `selfhost.rs`
+  ignores as needing `clang` (§8), since `--exact` runs nothing for a
+  name that matches no test.
+- **The extraction's tail is every line from the first `Error: ` line,**
+  not the last line alone: anyhow's causes span several lines. One line
+  to change.
+- **std's loci are read from `--std` on demand,** since `StdModule` keeps
+  no text; a std file N cannot read leaves its locus out.
+- **Each probe has its own directory,** and the files a side leaves are
+  compared, then removed before the other side runs.
+- **Three probe cases beyond §6.3's list** (above).
+- **`golden_dir()` and `run_golden()`** joined the shared file, so both
+  walks run and judge a binary the same way.
+
+## Consequences
+
+- N builds programs: `selfhost build` writes S0's `.ll`, runs `clang` and
+  `cc` as S0 does on Linux and macOS, and leaves the same binary, byte
+  for byte, as S0's `nova build --release` for the same `-o` basename:
+  on the 147 golden programs (§8), and on N itself (§13).
+- `selfhost check` prints each diagnostic's code and locus, S0's header
+  and locus lines; N's `build` and `check` give S0's stdout, exit codes
+  and messages on 39 probe cases without `clang` and 10 with it, §7's
+  exceptions listed.
+- N's `build` refuses Windows with its own message.
+- S1 builds N in about 13 s locally, at about 204 MB, and S2 builds N to
+  itself (§13): 5.6's S1 == S2 == S3 holds here with one `-o` basename.
+- `selfhost.rs`'s main step grew about 9 s locally as CI runs it (§12),
+  more than the spec projected; the new step takes about 39 s locally,
+  outside the 80 s line, and 30 s on CI; on CI `selfhost.rs` stays within
+every line (§14).
+- S0, its driver, the emitter and the runtime are unchanged, so ADR 0058
+  §16's rule is not engaged; no std builtin was added.
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index 4cc354f8..1f9bedfd 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -623,6 +623,45 @@ ADR 0058; spec
   the runtime, `check`'s output, N's `build` without `--release`, and
   Windows linking.
 
+**Amended 2026-11-20 (5.5b, branch `claude/loving-archimedes-frtf5q-55b`,
+ADR 0059; spec
+`docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`):**
+5.5b's questions, decided by the user on 2026-11-19 (ADR 0059 §16):
+- **The driver:** `selfhost build FILE [-o OUT] [--release] --std DIR`
+  and `selfhost check FILE --std DIR` (`selfhost/src/driver.nova`),
+  file mode only. `build` writes S0's intermediate, `<out>.nova.ll`
+  beside OUT, runs `clang -O2 -c` on it (`$NOVA_CLANG` or `clang`; no
+  `llc`), looks for the runtime after `clang` as S0 does
+  (`$NOVA_RUNTIME_LIB`, else beside the executable or two directories
+  above it; no embedded runtime), links with S0's `cc` line, and prints
+  `built OUT`; without `--release` it does the same, `-O2` too. Exit
+  codes 0, 1 and 2, and S0's messages in anyhow's layout. **std is found
+  only where `--std` names it,** never next to the executable or in
+  `$NOVA_HOME`, which narrows the 2026-11-02 amendment above. N's `build`
+  refuses Windows, which it knows by its executable's path, with its own
+  message; `check` runs there. `check` prints each diagnostic's
+  `SEVERITY[CODE]: MESSAGE` and `  ┌─ PATH:LINE:COL` lines, codespan's
+  header and loci (ADR 0048 decision 1), not codespan's layout. No std
+  builtin, no emitter change; S0 is unchanged.
+- **5.5b's gate,** the second item of the gate below with the user's
+  additions: on Ubuntu, in a new step of the Selfhost job's Linux leg
+  with `clang-18` 18.1.3 installed there, S1's own `build` builds the 147
+  golden programs on the release runtime, each runs with its table's
+  stdin and variables and matches its `.stdout` and status, and each
+  binary equals S0 `--release`'s for the same `-o` basename, byte for
+  byte; N's `build` and `check` give S0's stdout, exit codes and
+  messages on a probe set (39 cases without `clang` on Linux, macOS and
+  Windows, 10 with it on Linux, and N's `build` with no runtime, with a
+  runtime two directories up and without `--release`), S0's stderr cut
+  to its headers, loci and error, ADR 0059 §7's exceptions listed; N's
+  `build` refuses Windows on CI's Windows leg; `selfhost.rs`'s main
+  step is within the Selfhost job's lines and the new step's and the
+  install's times are recorded, or the user has ruled.
+- **For 5.6:** every stage is built with one `-o` basename, so that the
+  `.ll`'s name, which reaches the binary, is the same; S1 building N with
+  its own `build` gave the `--release` S1 byte for byte, and that S2
+  built N to itself (ADR 0059 §13).
+
 - **The emitter,** a port of `nova-codegen-llvm`, and `--emit-ll`.
 - **The driver:** the module loader, `clang` and `cc` through 5.0b's
   spawn, finding `libnova_runtime.a` as `link.rs` does, and the commands
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index da8c207c..b286ec40 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -874,6 +874,23 @@ S0's Cranelift backend. From here a change to the emitter, to
 `RtFunc`'s symbols or signatures, or to the runtime's ABI as the emitter
 spells it changes both compilers in one pull request.
 
+**Amended 2026-11-20 (branch `claude/loving-archimedes-frtf5q-55b`, ADR
+0059):** 5.5b, the driver and linking
+(`docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`).
+`selfhost/` builds programs: `selfhost build FILE [-o OUT] [--release]
+--std DIR` writes the `.ll` S0 writes, runs `clang` and `cc` on it as
+S0's `nova build --release` does, on Linux and macOS, finding the
+runtime from `NOVA_RUNTIME_LIB` or beside the executable, and leaves the
+same binary, byte for byte, for the same `-o` basename; it refuses
+Windows. `selfhost check FILE --std DIR` prints each diagnostic's code
+and locus, `error[CODE]: MESSAGE` and `PATH:LINE:COL`. A new CI step on
+Ubuntu builds the 147 golden programs with stage1's own `build`, runs
+them and holds each binary equal to S0 `--release`'s; two probe tests
+hold N's `build` and `check` to S0's stdout, exit codes and messages.
+std is found only where `--std` names it. Stage1 built N with its own
+`build`, and the result was, byte for byte, the stage1 that S0's `nova
+build --release` makes with the same `-o` basename.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/nova-spec/14-CODEGEN.md b/nova-spec/14-CODEGEN.md
index f696d3e5..aa925ff3 100644
--- a/nova-spec/14-CODEGEN.md
+++ b/nova-spec/14-CODEGEN.md
@@ -87,6 +87,17 @@ type-checks and is still refused by the emitter, an internal error in
 `build --release`. WASM's and test mode's code generation stay the Rust
 compiler's alone.
 
+**Amended 2026-11-20 (branch `claude/loving-archimedes-frtf5q-55b`, ADR
+0059):** the self-hosted compiler runs `clang` as `nova build --release`
+does, on Linux and macOS: `selfhost build` writes `<out>.nova.ll` beside
+the output, runs `$NOVA_CLANG` or `clang`, probed by `--version`, as
+`clang -O2 -c <ll> -o <out>.nova.o`, with and without `--release`,
+removes the object, and removes the `.ll` on success or leaves it on a
+failure, as S0 does. It never runs `llc`. Its binaries equal S0
+`--release`'s, byte for byte, for the same output basename, which the
+`.ll`'s name reaches; the 147 golden programs are held to that on
+Ubuntu's `clang-18` 18.1.3.
+
 ### 2.3 WASM (browser target)
 - Crates: `wasm-encoder` (write modules), `walrus` (manipulate)
 - Target: `nova build --target wasm` and `nova bundle`
@@ -390,6 +401,15 @@ token `nova_wake`, and either exports `nova_turn` and `nova_gc_switches`
 - Static linking by default (no shared libnova needed)
 - WASM: no linker needed, single module
 
+**Amended 2026-11-20 (branch `claude/loving-archimedes-frtf5q-55b`, ADR
+0059):** the self-hosted compiler links as `nova build` does on Linux
+and macOS: after `clang` has run it finds `libnova_runtime.a` from
+`NOVA_RUNTIME_LIB`, or in its executable's directory or one of the two
+above it (it carries no runtime), and runs `$CC` or `cc` as `-o <out>
+<obj> <runtime> -lpthread -ldl -lm`, the Rust compiler's line, with its
+messages. On Windows its `build` refuses with `Error: selfhost build
+links on Linux and macOS only`; it does not look for `link.exe`.
+
 ---
 
 ## 12. Tests
diff --git a/nova-spec/40-TOOLING.md b/nova-spec/40-TOOLING.md
index 9c2e295c..4b680791 100644
--- a/nova-spec/40-TOOLING.md
+++ b/nova-spec/40-TOOLING.md
@@ -157,6 +157,13 @@ build --help` says `--release` optimizes `app.wasm` for size with binaryen's
 (`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
 §4; ADR 0047).
 
+**Amended 2026-11-20 (branch `claude/loving-archimedes-frtf5q-55b`, ADR
+0059):** `nova build` and `nova check` are unchanged by 5.5b, which
+gives the self-hosted compiler its own `selfhost build` and `selfhost
+check`, held to `nova build --release` and `nova check` on a probe set
+(`docs/superpowers/specs/2026-11-19-phase-5-5b-the-driver-and-linking-design.md`;
+ADR 0059). No `--help` changes.
+
 ### 1.2 Project Template (`nova new`)
 
 ```
````

### Task 5: Verification, the mutants, the review, the PR, and the merge

Every step runs in a shell set up as the conventions say, the process
limit included.

- [ ] **Step 1: The full checks.** First, before the build, `df -h /`
  must show at least 6.3G free; if not, remove older duplicate test
  executables.

  ```sh
  cargo build --locked --workspace
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo clippy --locked --all-features --test selfhost -- -D warnings
  target/debug/nova fmt --check std examples selfhost
  target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost
  cargo test --locked --workspace --no-run
  S --lib --bins > "$P/exec55b/full-a.txt" 2>&1
  S --doc > "$P/exec55b/full-b.txt" 2>&1
  S --test '[!s]*' --test 's[!e]*' > "$P/exec55b/full-c.txt" 2>&1
  cat "$P"/exec55b/full-[abc].txt > "$P/exec55b/full.txt"
  grep '^test result' "$P/exec55b/full.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec55b/full.txt"
  grep -c 'Running tests/selfhost.rs' "$P/exec55b/full.txt"
  cargo test --locked --workspace -- --list > "$P/exec55b/final-list.txt" 2>&1
  cargo test --locked --workspace --test selfhost -- --list > "$P/exec55b/final-list-sh.txt" 2>&1
  python3 "$P/exec55b/names.py" "$P/exec55b/base-list.txt" "$P/exec55b/final-list.txt"
  comm -3 <(grep ': test$' "$P/exec55b/base-list-sh.txt" | sort) <(grep ': test$' "$P/exec55b/final-list-sh.txt" | sort)
  ```

  `names.py` compares Task 0's list with this one binary by binary, by
  name: a binary is its `Running` or `Doc-tests` line, its hash stripped,
  an integration test's prefixed with the crate whose unit tests cargo
  listed just before it; a test is a line ending `: test`. Write it
  before the run, as the replay ran it:

```sh
mkdir -p "$P/exec55b"
cat > "$P/exec55b/names.py" <<'EOF'
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
  - the suite, in three runs as the conventions give them (98 to 117, 1
    and 157 to 185 s): `1300 1 8`, `2 0 0` and `1535 0 2`, `2837 1 10`
    summed; the one failure is Task 0's IPv6 test; `0` binaries named
    `tests/selfhost.rs`;
  - `names.py` prints `binaries: base 111, now 111`, `tests: base 2848,
    now 2848` and `net added: 0`: the suite's tests are the base's, the
    golden walk's included;
  - `comm` prints `selfhost.rs`'s five added tests, 67 to 72:
    `n_build_and_check_answer_as_s0_does`,
    `n_build_answers_as_s0_does_with_clang` (ignored),
    `s1_builds_the_golden_suite` (ignored),
    `the_ci_step_names_every_clang_test` and
    `the_extraction_keeps_the_headers_the_loci_and_the_error`; none gone.
- [ ] **Step 2: CI's Linux leg, as the Test job runs it.** The `test`
  job's `ubuntu-24.04` leg runs the whole suite with the golden suite
  required, with CI's own flags, and with `RUST_BACKTRACE=1` set for each
  suite command, as `.github/workflows/ci.yml` exports it for every job.
  The suite runs in the conventions' three parts; `CI` is a function of
  this step's shell:

  ```sh
  CI() { RUST_BACKTRACE=1 NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --all-features --no-fail-fast "$@"; }
  cargo test --locked --workspace --all-features --no-run
  time CI --lib --bins > "$P/exec55b/ci-a.txt" 2>&1
  time CI --doc > "$P/exec55b/ci-b.txt" 2>&1
  time CI --test '[!s]*' --test 's[!e]*' > "$P/exec55b/ci-c.txt" 2>&1
  cat "$P"/exec55b/ci-[abc].txt > "$P/exec55b/ci.txt"
  grep '^test result' "$P/exec55b/ci.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec55b/ci.txt"
  grep -c 'release_golden_suite ... ok' "$P/exec55b/ci.txt"
  grep -c 'Running tests/selfhost.rs' "$P/exec55b/ci.txt"
  ```

  Expected, as the replay ran it at the last commit: `1300 1 8`, `2 0 0`
  and `1535 0 2` in 119, 1 and 192 s; `2837 1 10` summed, the one failure
  the IPv6 test, `1` (the golden suite ran and passed inside the third
  part) and `0`. If a part is killed at its timeout, or the shell
  restarts, run that part again alone, then the parts after it; never sum
  a part that did not end with its `test result` lines.
- [ ] **Step 3: The golden suite.** `time G`. Expected, in 21 s of wall
  time: `release golden: 147 passed, 9 excluded, Ubuntu clang version
  18.1.3 (1ubuntu1)` and `1 passed`: unchanged, its tables moved as they
  were.
- [ ] **Step 4: No S0 source changed** (decision 12 **(differs)**).

  ```sh
  X="':(exclude)crates/nova-cli/tests' ':(exclude)selfhost' ':(exclude)docs' ':(exclude)nova-spec' ':(exclude)CHANGELOG.md' ':(exclude).github'"
  eval git diff --name-only 17ecca86 HEAD -- . "$X"
  eval git ls-files -- . "$X" | grep -e '^crates/nova-cli/build_support.rs$' -e '^crates/nova-doc/assets/'
  git grep -h -o 'include_\(str\|bytes\)!([^)]*)' HEAD -- crates std ':(exclude)crates/*/tests/*' ':(exclude)crates/*/benches/*' | grep -c -e selfhost -e 'tests/' -e docs -e nova-spec -e CHANGELOG -e github
  git diff --stat 17ecca86 HEAD -- . ':(exclude)docs/superpowers'
  ```

  Expected:
  - the first prints nothing: no file outside the six excluded paths
    changed. Its pathspec is the whole tree but those paths, so it holds
    every S0 input wherever it lies (decision 12 says why no excluded
    path reaches S0's binary);
  - the second prints `crates/nova-cli/build_support.rs`,
    `crates/nova-doc/assets/nova.css` and `crates/nova-doc/assets/nova.js`:
    the S0 inputs outside a crate's `src/` and `build.rs`, which that
    pathspec holds;
  - the third prints `0`: no `include_str!` or `include_bytes!` outside a
    crate's `tests/` and `benches/` names a file under an excluded path;
  - the fourth lists the 14 files of the prototype's `82d22393` against
    `17ecca86`, 3,601 lines added and 308 removed.

  With Step 3's 147, nothing a user of S0 sees has changed. No base
  binary is needed or built.
- [ ] **Step 5: The CI step's names.** `SH --exact
  the_ci_step_names_every_clang_test` prints `1 passed`: the step names
  exactly the tests `selfhost.rs` ignores as needing `clang` (mutants
  41, 63 and 64 fail it). And `grep -o -- '--exact s1_builds_the_golden_suite .*' .github/workflows/ci.yml`
  prints the step's two names, for the ledger.
- [ ] **Step 6: The lockfile and Rust 1.78.**
  - `git diff --numstat 17ecca86 -- Cargo.lock '*Cargo.toml'`. Expected:
    nothing; `sha256sum Cargo.lock` as Task 0's.
  - Then:

    ```sh
    RUSTUP_TOOLCHAIN=1.78.0 RUSTFLAGS="-D warnings" CARGO_TARGET_DIR=/dev/shm/msrv-55b cargo check --locked --workspace
    rm -rf /dev/shm/msrv-55b
    ```

    Expected: clean, `Finished` in 28 s. It checks the libraries and
    binaries, as CI's MSRV job does, not the tests, whose Rust meets 1.78
    by review (Global Constraints).
- [ ] **Step 7: The differential's time, the new step, N's tests, and S1
  building N.**
  - Without the release runtime, as the build stands: `cargo test
    --locked --workspace --test selfhost --no-run`, then `time SH`, then
    `time RUST_BACKTRACE=1 SH`, `RUST_BACKTRACE=1` for that command alone,
    as CI's jobs export it. Expected: `70 passed; 0 failed; 2 ignored` in
    175.58 s, and again in 186.67 s; no child past the
    120 s `CHILD_LIMIT`.
  - Then as the Selfhost job runs, in one shell. **First the disk,**
    since this changes the opt-level and the runtime: `df -h /` must show
    at least 7.0G; if not, remove the `opt-level = 0` `nova-*` and
    `selfhost-*` executables in `target/debug/deps` first (the build
    below makes this level's; the replay read 6.5G and removed them,
    reaching 7.0G, 7,159 MiB). Then `export NOVA_EMBED_RUNTIME=1
    CARGO_PROFILE_DEV_OPT_LEVEL=1`, two of the job's variables; the third,
    the workflow's `RUST_BACKTRACE=1`, is not set here (the shell keeps
    the Runs block's `0`; `SH` under `RUST_BACKTRACE=1` is this step's
    first part). `time cargo build --locked --workspace` (5 s with this
    level's artifacts kept from before, 37 s from none), `df -h /`
    (at least 6.3G), `time cargo test --locked --workspace --test
    selfhost --no-run` (10 s; 23 s from none). Then three times `time
    SH`; then the new step's command twice, as `ci.yml` writes it, `time
    NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked
    --workspace --all-features --no-fail-fast --test selfhost -- --ignored
    --exact s1_builds_the_golden_suite
    n_build_answers_as_s0_does_with_clang --nocapture`; then N's tests on
    each runtime: `(cd selfhost &&
    NOVA_RUNTIME_LIB=$PWD/../target/debug/libnova_runtime.a
    ../target/debug/nova test) 2>&1 | tail -n 1` and `(cd selfhost &&
    ../target/debug/nova test) 2>&1 | tail -n 1`; then the job's last
    step, `time NOVA_RUNTIME_LIB=$PWD/target/debug/libnova_runtime.a cargo
    test --locked --workspace --all-features --no-fail-fast --test selfhost
    -- --exact the_selfhost_tests_pass`. Expected: `70 passed; 0 failed; 2
    ignored` three times, 52.87, 48.73 and 57.36 s (user CPU 170 to 204
    s); the new step `2 passed`, `70 filtered out`, twice in 33.51 and
    34.72 s, its line `s1 golden: 147 passed, 9 excluded, Ubuntu clang
    version 18.1.3 (1ubuntu1), runtime /tmp/nova-selfhost-home/runtime/0.4.0-be7e8b48/libnova_runtime.a`
    (CI's 30.21 s); `119 passed` on each runtime; `1 passed` in 1.71 s.
  - Then `unset NOVA_EMBED_RUNTIME CARGO_PROFILE_DEV_OPT_LEVEL`. Before
    the build, which changes the opt-level and the runtime back: `df -h
    /` at least 7.0G, or remove the `opt-level = 1` `nova-*` and
    `selfhost-*` executables this step built first. Then `cargo build
    --locked --workspace`, which ends with `Finished`: the debug-runtime
    `nova` at `opt-level = 0` again (`target/debug/nova version` prints
    `runtime: not embedded`).
  - Then S1 building N, ADR 0059 §13's method, on the release runtime the
    Selfhost-job part above unpacked under `nova-selfhost-home` in the
    temporary directory. Run it right after that part, before any other
    `selfhost.rs` test: `build_selfhost` makes `nova-selfhost-home` afresh
    (`fresh_dir("home")`) on each run, which removes the runtime there.
    After a restart, or if `s1n.sh` prints `no runtime at …`, run the
    Selfhost-job part's build and its first `SH` again first. With these
    scripts, as the replay ran them:
    `tv.py` runs a command and records its wall time, `wait4`'s
    `ru_maxrss` and its own polled `VmHWM`; `s1n.sh` builds S1 by
    Cranelift, has it build N twice, has S0's `--release` build N twice
    into a sibling directory with the same basename, makes both `.ll`s
    with no `clang`, has S2 build N twice, and compares:

```sh
mkdir -p "$P/exec55b"
cat > "$P/exec55b/tv.py" <<'EOF'
#!/usr/bin/env python3
# tv.py LOG -- CMD...: run CMD; append to LOG its wall time, user/sys CPU and
# max_rss_kb (wait4's ru_maxrss, what GNU time -v reports: the largest of the
# process and the children it waited for, so clang and cc too), and
# own_vmhwm_kb (the process's own VmHWM, polled every 20 ms), and its exit.
import os, sys, time, threading
log = sys.argv[1]; cmd = sys.argv[3:]
t0 = time.monotonic()
pid = os.fork()
if pid == 0:
    os.execvp(cmd[0], cmd)
hwm = [0]; done = [False]
def poll():
    while not done[0]:
        try:
            with open(f"/proc/{pid}/status") as f:
                for line in f:
                    if line.startswith("VmHWM:"):
                        hwm[0] = max(hwm[0], int(line.split()[1]))
        except OSError:
            pass
        time.sleep(0.02)
th = threading.Thread(target=poll, daemon=True); th.start()
_, status, ru = os.wait4(pid, 0)
done[0] = True
wall = time.monotonic() - t0
code = os.waitstatus_to_exitcode(status)
with open(log, "a") as f:
    f.write(f"cmd: {' '.join(cmd)}\nwall_s: {wall:.2f}\nuser_s: {ru.ru_utime:.2f}\nsys_s: {ru.ru_stime:.2f}\nmax_rss_kb: {ru.ru_maxrss}\nown_vmhwm_kb: {hwm[0]}\nexit: {code}\n\n")
sys.exit(code if code >= 0 else 1)
EOF
```

```sh
mkdir -p "$P/exec55b"
cat > "$P/exec55b/s1n.sh" <<'EOF'
#!/bin/bash
# s1n.sh: S1 builds N, and S2 = S1 (ADR 0059 §13's method), at the current
# tree, on the release runtime RT (default: the one the Selfhost job's tests
# unpack), with clang-18. Outputs under $P/exec55b/s1n/ (removed and made
# afresh: no /tmp/nova-* path is written). Run from the repository's root,
# after a `nova` build; prints one line per measurement.
set -eu
: "${P:?}"
D=$P/exec55b/s1n; RT=${RT:-/tmp/nova-selfhost-home/runtime/0.4.0-be7e8b48/libnova_runtime.a}
[ -f "$RT" ] || { echo "no runtime at $RT: run the Selfhost-job part (its build and SH) first"; exit 1; }
TV="python3 $P/exec55b/tv.py"
rm -rf "${D:?}"/out; mkdir -p $D/out/s1c $D/out/s2 $D/out/s1r $D/out/s3 $D/out/ll2 $D/out/ll1r; : > $D/tv.log
export NOVA_RUNTIME_LIB=$RT NOVA_CLANG=clang-18 NOVA_HOME=$D/out/home
(cd selfhost && $TV $D/tv.log -- ../target/debug/nova build -o $D/out/s1c/selfhost) > /dev/null
S1=$D/out/s1c/selfhost
for k in 1 2; do $TV $D/tv.log -- $S1 build selfhost/src/main.nova -o $D/out/s2/selfhost --release --std std > /dev/null; done
for k in 1 2; do $TV $D/tv.log -- target/debug/nova build --release selfhost/src/main.nova -o $D/out/s1r/selfhost > /dev/null; done
NOVA_CLANG=$D/no-clang NOVA_LLC=$D/no-llc $TV $D/tv.log -- $S1 build selfhost/src/main.nova -o $D/out/ll2/selfhost --release --std std > /dev/null 2>&1 || true
NOVA_CLANG=$D/no-clang NOVA_LLC=$D/no-llc target/debug/nova build --release selfhost/src/main.nova -o $D/out/ll1r/selfhost > /dev/null 2>&1 || true
for k in 1 2; do $TV $D/tv.log -- $D/out/s2/selfhost build selfhost/src/main.nova -o $D/out/s3/selfhost --release --std std > /dev/null; done
awk '/^cmd:/{c=$2" "$3" "$4} /^wall_s/{w=$2} /^max_rss_kb/{r=$2} /^own_vmhwm_kb/{h=$2} /^exit/{printf "%s wall %s s, ru_maxrss %.1f MB, own VmHWM %.1f MB, exit %s\n", c, w, r/1000, h/1000, $2}' $D/tv.log | sed "s#$D/out/##g; s#$PWD/##g"
for f in s1c s2 s1r s3; do echo "$f $(stat -c %s $D/out/$f/selfhost) $(sha256sum < $D/out/$f/selfhost | cut -c1-16)"; done
cmp $D/out/s2/selfhost $D/out/s1r/selfhost && echo "S2 == S1 (--release): equal"
cmp $D/out/s3/selfhost $D/out/s2/selfhost && echo "S3 == S2: equal"
cmp $D/out/ll2/selfhost.nova.ll $D/out/ll1r/selfhost.nova.ll && echo ".ll equal: $(stat -c %s $D/out/ll2/selfhost.nova.ll) bytes, $(wc -l < $D/out/ll2/selfhost.nova.ll) lines"
EOF
```

    Then `bash "$P/exec55b/s1n.sh"` (120 s). Expected, as the replay
    measured at the first split's last commit (the same `selfhost/` and
    S0 sources): S1's build 6.32 s, 102.6 MB; S1 building N 10.02 and 10.21
    s, `ru_maxrss` 203.5 MB, its own `VmHWM` 203.6 and 203.7 MB, exit 0;
    S0's `--release` 10.01 and 9.73 s, `ru_maxrss` 200.6 MB, its own 123.5
    MB; S1 to the `clang` probe 1.41 s (exit 1, no `clang`); S2 building N
    9.97 and 10.21 s; sizes and digests `s1c 9376184 1303bfe21f5022ee`,
    `s2 9354680 1366ce16e3f191e6`, `s1r` and `s3` as `s2`; then `S2 == S1
    (--release): equal`, `S3 == S2: equal` and `.ll equal: 12759478
    bytes, 406232 lines`. ADR 0059 §13 (decision 17): every size, digest,
    peak and equality the same; a time may differ with the machine's
    speed, and is recorded in the ledger, not in the ADR.
- [ ] **Step 8: The mutants.** One at a time, each edit is made, its
  commands run, and the file restored with `git checkout -- <file>`, all
  on the last commit, committed, in a shell under the process limit,
  with `NOVA_EMBED_RUNTIME=1` exported for the whole step (decision 16),
  N on the release runtime and the ignored tests finding it as the new
  step does. Of the Selfhost job's other variables,
  `CARGO_PROFILE_DEV_OPT_LEVEL=1` is not set (S0 at `opt-level = 0`, as
  the replay ran the mutants) and `RUST_BACKTRACE` stays the Runs block's
  `0`. The replay's runner does it, and the executor uses it as given,
  writing no runner of its own: `mutrun.py` reads the mutants from
  `mutants.json`, applies each one's edits, runs its commands, restores
  the file, checks `git status --short` before and after, and writes
  each result to `$P/exec55b/mut/<id>.json` and its log beside it. Write
  both before the step, from the repository's root:

```sh
mkdir -p "$P/exec55b"
cat > "$P/exec55b/mutrun.py" <<'EOF'
#!/usr/bin/env python3
"""mutrun.py [--force] [--check] IDS...: run the mutants of mutants.json one at
a time on the checked-out tree (the last commit), each file restored after
with `git checkout -- FILE` and `git status --short` checked empty before and
after. IDS are numbers or ranges (3-7). Each result goes to
$P/exec55b/mut/<id>.json (and its log to <id>.log). A result already there is
kept unless --force. --check only checks each edit's text. Run under the
process limit (the Conventions' Runs) with NOVA_EMBED_RUNTIME=1 exported, as the plan's
mutant step runs: the ignored tests then find the release runtime the
Selfhost job's build unpacks.

Per mutant: `cargo build --locked --workspace`; then
  SH --exact <its selfhost.rs tests> GUARD           (always: the guard)
  NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 SH --ignored --exact <clang, golden>
  G                                                   (the golden suite, when named)
  the CI step's command, as ci.yml writes it          (cistep)
"""
import json
import os
import re
import subprocess
import sys
import time

P = os.environ["P"]
REPO = os.getcwd()  # run from the repository's root
if not os.path.isdir(os.path.join(REPO, ".git")):
    raise SystemExit("run mutrun.py from the repository's root")
OUT = f"{P}/exec55b/mut"
os.makedirs(OUT, exist_ok=True)
MUTANTS = json.load(open(f"{P}/exec55b/mutants.json"))
GUARD = "the_resolution_dump_has_a_header_per_root_and_std_its_own_part"
NAMES = {"main": "n_build_and_check_answer_as_s0_does",
         "extract": "the_extraction_keeps_the_headers_the_loci_and_the_error",
         "ntest": "the_selfhost_tests_pass",
         "clang": "n_build_answers_as_s0_does_with_clang",
         "golden": "s1_builds_the_golden_suite"}
SH = ["cargo", "test", "--locked", "--workspace", "--test", "selfhost", "--"]
N_TESTS = set()
for _f in os.listdir(f"{REPO}/selfhost/src"):
    if _f.endswith("_tests.nova"):
        N_TESTS.update(re.findall(r"@test\s*\n\s*(?:async )?fn (\w+)", open(f"{REPO}/selfhost/src/{_f}").read()))
CLANG_ENV = {"NOVA_REQUIRE_CLANG": "1", "NOVA_CLANG": "clang-18"}


def sh(cmd, env=None, log=None, timeout=590):
    e = dict(os.environ, **(env or {}))
    t = time.time()
    r = subprocess.run(cmd, cwd=REPO, env=e, capture_output=True, text=True, errors="replace",
                       timeout=timeout)
    out = r.stdout + r.stderr
    if log is not None:
        log.write(f"$ {' '.join(cmd) if isinstance(cmd, list) else cmd}\n{out}\n[exit {r.returncode}, "
                  f"{time.time() - t:.1f} s]\n\n")
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


def cases_of(out):
    """The probe cases a probe test names as failing, and the golden programs."""
    cases = re.findall(r"^([A-Za-z][^`\n]*?) \(`", out, re.M)
    cases += re.findall(r"^((?:no runtime|without|the golden harness)[^\n]{0,100})", out, re.M)
    probes = re.findall(r"(\d+) of (\d+) (?:clang )?(?:probes|cases) failed", out)
    golden = re.findall(r"(\d+) of (\d+) golden programs failed", out)
    return cases, probes, golden


def apply(m, text):
    for k, (old, new) in enumerate(m["edits"]):
        occ = m.get("occ", 1) if k == 0 else 1
        n = text.count(old)
        if n < occ or (occ == 1 and n != 1):
            raise SystemExit(f"mutant {m['id']}: edit {k + 1}'s text occurs {n} times")
        at = -1
        for _ in range(occ):
            at = text.index(old, at + 1)
        text = text[:at] + new + text[at + len(old):]
    return text


def run(m, force):
    res_path = f"{OUT}/{m['id']}.json"
    if os.path.exists(res_path) and not force:
        print(f"{m['id']} kept")
        return
    if not status_clean():
        raise SystemExit("the tree is not clean")
    path = os.path.join(REPO, m["file"])
    orig = open(path, encoding="utf-8").read()
    open(path, "w", encoding="utf-8").write(apply(m, orig))
    t0 = time.time()
    log = open(f"{OUT}/{m['id']}.log", "w")
    res = {"id": m["id"], "key": m["key"], "failed": [], "compile": False, "guard_failed": False,
           "cases": {}, "counts": {}}
    try:
        rc, out = sh(["cargo", "build", "--locked", "--workspace"], log=log)
        if rc:
            res["compile"] = True
        runs = m["runs"]
        tests = [NAMES[r] for r in runs if r in ("main", "extract", "ntest")]
        tests += [r[2:] for r in runs if r.startswith("t:")]
        rc, out = sh(SH + ["--exact"] + tests + [GUARD], log=log)
        if "error: could not compile `" in out:
            res["compile"] = True
        f = failed_of(out)
        res["guard_failed"] = GUARD in f
        res["failed"] += [x for x in f if x != GUARD]
        c, pr, go = cases_of(out)
        if c:
            res["cases"]["main"] = c
        if pr:
            res["counts"]["main"] = pr[0]
        ign = [NAMES[r] for r in runs if r in ("clang", "golden")]
        if ign:
            rc, out = sh(SH + ["--ignored", "--exact"] + ign, env=CLANG_ENV, log=log)
            if "error: could not compile `" in out:
                res["compile"] = True
            res["failed"] += failed_of(out)
            c, pr, go = cases_of(out)
            if c:
                res["cases"]["clang"] = c
            if pr:
                res["counts"]["clang"] = pr[0]
            if go:
                res["counts"]["golden"] = go[0]
        if "G" in runs:
            rc, out = sh(["cargo", "test", "--locked", "--workspace", "--test", "run_tests", "--",
                          "release_golden_suite", "--nocapture"], env=CLANG_ENV, log=log)
            res["failed"] += failed_of(out)
        if "cistep" in runs:
            line = [l for l in open(os.path.join(REPO, ".github/workflows/ci.yml"))
                    if "--ignored --exact s1_builds_the_golden_suite" in l][0]
            cmd = line.split("run: ", 1)[1].strip()
            rc, out = sh(["bash", "-c", cmd], env=CLANG_ENV, log=log)
            res["failed"] += failed_of(out)
            res["cistep"] = re.findall(r"^test result: .*", out, re.M)
    finally:
        subprocess.run(["git", "checkout", "--", m["file"]], cwd=REPO, check=True)
        log.close()
    if not status_clean():
        raise SystemExit(f"mutant {m['id']}: the tree is not clean after it")
    res["n_failed"] = sorted({x for x in res["failed"] if x in N_TESTS})
    res["failed"] = [x for x in res["failed"] if x not in N_TESTS]
    res["caught"] = bool(res["failed"]) or res["compile"]
    res["seconds"] = round(time.time() - t0, 1)
    json.dump(res, open(res_path, "w"), indent=1)
    print(f"{m['id']} {m['key']}: {'CAUGHT' if res['caught'] else 'SURVIVED'} {res['seconds']} s "
          f"failed={res['failed']} n={res['n_failed']} counts={res['counts']} guard_failed={res['guard_failed']} "
          f"compile={res['compile']}", flush=True)


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
mkdir -p "$P/exec55b"
cat > "$P/exec55b/mutants.json" <<'EOF'
[
 {
  "id": 1,
  "key": "M1",
  "title": "the intermediate named without `.nova`",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "let file = \"${name}.nova.${ext}\"",
    "let file = \"${name}.${ext}\""
   ]
  ],
  "runs": [
   "main",
   "ntest",
   "clang"
  ]
 },
 {
  "id": 2,
  "key": "M2",
  "title": "`-O2` dropped",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "run_tool(clang, [\"-O2\", \"-c\", ll, \"-o\", obj], \"clang\", \"clang\")?",
    "run_tool(clang, [\"-c\", ll, \"-o\", obj], \"clang\", \"clang\")?"
   ]
  ],
  "runs": [
   "main",
   "clang",
   "golden"
  ]
 },
 {
  "id": 3,
  "key": "M3a",
  "title": "the runtime looked for before the `clang` probe",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "    let clang = env_or(\"NOVA_CLANG\", \"clang\")\n    if !tool_available(clang) { return Err([NO_TOOLCHAIN]) }\n",
    "    let runtime = find_runtime().await?\n    let clang = env_or(\"NOVA_CLANG\", \"clang\")\n    if !tool_available(clang) { return Err([NO_TOOLCHAIN]) }\n"
   ]
  ],
  "runs": [
   "main",
   "clang"
  ]
 },
 {
  "id": 4,
  "key": "M3b",
  "title": "the runtime looked for before `clang` runs",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "    run_tool(clang, [\"-O2\", \"-c\", ll, \"-o\", obj], \"clang\", \"clang\")?\n    let runtime = find_runtime().await?\n",
    "    let runtime = find_runtime().await?\n    run_tool(clang, [\"-O2\", \"-c\", ll, \"-o\", obj], \"clang\", \"clang\")?\n"
   ]
  ],
  "runs": [
   "main",
   "clang"
  ]
 },
 {
  "id": 5,
  "key": "M4a",
  "title": "`-lm` dropped",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"-lpthread\", \"-ldl\", \"-lm\"]",
    "\"-lpthread\", \"-ldl\"]"
   ]
  ],
  "runs": [
   "clang"
  ]
 },
 {
  "id": 6,
  "key": "M4b",
  "title": "`-lpthread` dropped",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"-lpthread\", \"-ldl\", \"-lm\"]",
    "\"-ldl\", \"-lm\"]"
   ]
  ],
  "runs": [
   "clang"
  ]
 },
 {
  "id": 7,
  "key": "M4c",
  "title": "the runtime before the object on `cc`'s line",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "[\"-o\", out, obj, runtime, \"-lpthread\"",
    "[\"-o\", out, runtime, obj, \"-lpthread\""
   ]
  ],
  "runs": [
   "clang"
  ]
 },
 {
  "id": 8,
  "key": "M5",
  "title": "the default output FILE's name, not its stem",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "        None => file_stem_or(root, \"out\")",
    "        None => file_name(root)"
   ]
  ],
  "runs": [
   "main",
   "ntest",
   "clang"
  ]
 },
 {
  "id": 9,
  "key": "M6",
  "title": "anyhow's blank line before `Caused by:` dropped",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "out.push(\"\\n\\nCaused by:\")",
    "out.push(\"\\nCaused by:\")"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 10,
  "key": "M7",
  "title": "a failure's exit code 2",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "    eprintln(failure_text(head, causes))\n    exit(1)",
    "    eprintln(failure_text(head, causes))\n    exit(2)"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 11,
  "key": "M8a",
  "title": "Windows's drive read at `;`",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "path.byte_at(1) != 58",
    "path.byte_at(1) != 59"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 12,
  "key": "M8b",
  "title": "Windows known by `/`",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "if path.starts_with(\"\\\\\\\\\") { return true }",
    "if path.starts_with(\"/\") { return true }"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 13,
  "key": "M9",
  "title": "the `.ll` kept on success",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "            let _ = remove_file(ll).await\n",
    ""
   ]
  ],
  "runs": [
   "main",
   "clang"
  ]
 },
 {
  "id": 14,
  "key": "C1",
  "title": "a column per byte",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "} else if b < 128 || b >= 192 {",
    "} else {"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 15,
  "key": "C2",
  "title": "`\\r` a line break too",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "        if b == 10 {",
    "        if b == 10 || b == 13 {"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 16,
  "key": "C3a",
  "title": "a file's first label its locus",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "        } else if (l.primary && !primary.at(i))\n            || (l.primary == primary.at(i) && l.span.start < starts.at(i)) {",
    "        } else if false {"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 17,
  "key": "C3b",
  "title": "the later start the locus",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "l.span.start < starts.at(i)) {",
    "l.span.start > starts.at(i)) {"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 18,
  "key": "C3c",
  "title": "the loci's files reversed",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "        out.push((names.at(i), starts.at(i)))",
    "        out.push((names.at(names.len() - 1 - i), starts.at(names.len() - 1 - i)))"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 19,
  "key": "C4",
  "title": "the severity capitalised",
  "group": "p2",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "SeverityError => \"error\"",
    "SeverityError => \"Error\""
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 20,
  "key": "H1",
  "title": "the golden binaries not compared",
  "group": "p2",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "    if a == b {\n        return Ok(());",
    "    if true {\n        return Ok(());"
   ]
  ],
  "runs": [
   "clang",
   "golden"
  ]
 },
 {
  "id": 21,
  "key": "H2",
  "title": "the golden harness's stdout check loosened",
  "group": "p2",
  "file": "crates/nova-cli/tests/golden/shared.rs",
  "edits": [
   [
    "    if out != expected {",
    "    if !expected.contains(out.trim()) {"
   ]
  ],
  "runs": [
   "clang",
   "golden",
   "G"
  ]
 },
 {
  "id": 22,
  "key": "H3a",
  "title": "the extraction drops `warning[`",
  "group": "p2",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "if [\"error[\", \"warning[\", \"note[\", \"help[\"]",
    "if [\"error[\", \"note[\", \"help[\"]"
   ]
  ],
  "runs": [
   "main",
   "extract"
  ]
 },
 {
  "id": 23,
  "key": "H3b",
  "title": "the extraction keeps a locus's own indent",
  "group": "p2",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "            out.push_str(\"  ┌─ \");\n            if windows {",
    "            out.push_str(&line[..line.len() - locus.len()]);\n            if windows {"
   ]
  ],
  "runs": [
   "main",
   "extract"
  ]
 },
 {
  "id": 24,
  "key": "T1",
  "title": "an exclusion dropped (`nova_test`)",
  "group": "p1",
  "file": "crates/nova-cli/tests/golden/shared.rs",
  "edits": [
   [
    "    (\"nova_test\", \"a `@test` file, which `nova test` runs\"),\n",
    ""
   ]
  ],
  "runs": [
   "G",
   "golden"
  ]
 },
 {
  "id": 25,
  "key": "T2",
  "title": "an exclusion's name changed",
  "group": "p1",
  "file": "crates/nova-cli/tests/golden/shared.rs",
  "edits": [
   [
    "    (\"net_timeout\", \"its test serves the port it connects to\"),",
    "    (\"net_timeouts\", \"its test serves the port it connects to\"),"
   ]
  ],
  "runs": [
   "G",
   "golden"
  ]
 },
 {
  "id": 26,
  "key": "T3",
  "title": "the stdin entry changed",
  "group": "p1",
  "file": "crates/nova-cli/tests/golden/shared.rs",
  "edits": [
   [
    "&[(\"io_streams\", \"hello-stdin\")]",
    "&[(\"io_streams\", \"hello-stdin!\")]"
   ]
  ],
  "runs": [
   "G",
   "golden"
  ]
 },
 {
  "id": 27,
  "key": "T4",
  "title": "the stdin entry dropped",
  "group": "p1",
  "file": "crates/nova-cli/tests/golden/shared.rs",
  "edits": [
   [
    "&[(\"io_streams\", \"hello-stdin\")]",
    "&[]"
   ]
  ],
  "runs": [
   "G",
   "golden"
  ]
 },
 {
  "id": 28,
  "key": "T5",
  "title": "an environment entry dropped",
  "group": "p1",
  "file": "crates/nova-cli/tests/golden/shared.rs",
  "edits": [
   [
    "    (\"process_env\", \"NOVA_PROCESS_TEST_EQ\", PROCESS_TEST_EQ),\n",
    ""
   ]
  ],
  "runs": [
   "G",
   "golden"
  ]
 },
 {
  "id": 29,
  "key": "T6",
  "title": "an environment entry's value changed",
  "group": "p1",
  "file": "crates/nova-cli/tests/golden/shared.rs",
  "edits": [
   [
    "    (\"process_env\", \"NOVA_PROCESS_TEST_VALUE\", PROCESS_TEST_VALUE),",
    "    (\"process_env\", \"NOVA_PROCESS_TEST_VALUE\", PROCESS_TEST_EQ),"
   ]
  ],
  "runs": [
   "G",
   "golden"
  ]
 },
 {
  "id": 30,
  "key": "H4",
  "title": "the golden harness's stdout check skipped",
  "group": "p1",
  "file": "crates/nova-cli/tests/golden/shared.rs",
  "edits": [
   [
    "    if out != expected {",
    "    if false {"
   ]
  ],
  "runs": [
   "clang",
   "golden",
   "G"
  ]
 },
 {
  "id": 31,
  "key": "H5",
  "title": "S1's `built OUT` line not checked",
  "group": "p1",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "                    if stdout == want {",
    "                    if true {"
   ]
  ],
  "runs": [
   "golden"
  ]
 },
 {
  "id": 32,
  "key": "U1",
  "title": "N's `build` usage line changed",
  "group": "p1",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"usage: selfhost build FILE [-o OUT] [--release] --std DIR\"",
    "\"usage: selfhost build FILE [-o OUT] --std DIR\""
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 33,
  "key": "A1",
  "title": "`clang`'s arguments in another order",
  "group": "p1",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "run_tool(clang, [\"-O2\", \"-c\", ll, \"-o\", obj], \"clang\", \"clang\")?",
    "run_tool(clang, [\"-O2\", \"-c\", \"-o\", obj, ll], \"clang\", \"clang\")?"
   ]
  ],
  "runs": [
   "clang",
   "golden"
  ]
 },
 {
  "id": 34,
  "key": "A2",
  "title": "`clang` asked to link (`-c` dropped)",
  "group": "p1",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "run_tool(clang, [\"-O2\", \"-c\", ll, \"-o\", obj], \"clang\", \"clang\")?",
    "run_tool(clang, [\"-O2\", ll, \"-o\", obj], \"clang\", \"clang\")?"
   ]
  ],
  "runs": [
   "clang",
   "golden"
  ]
 },
 {
  "id": 35,
  "key": "A3",
  "title": "`cc`'s `-o OUT` last",
  "group": "p1",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "[\"-o\", out, obj, runtime, \"-lpthread\", \"-ldl\", \"-lm\"]",
    "[obj, runtime, \"-lpthread\", \"-ldl\", \"-lm\", \"-o\", out]"
   ]
  ],
  "runs": [
   "clang",
   "golden"
  ]
 },
 {
  "id": 36,
  "key": "R1",
  "title": "`NOVA_RUNTIME_LIB` not read",
  "group": "p1",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "    match env(\"NOVA_RUNTIME_LIB\") {",
    "    match env(\"NOVA_RUNTIME_LIB_UNSET\") {"
   ]
  ],
  "runs": [
   "main",
   "clang",
   "golden"
  ]
 },
 {
  "id": 37,
  "key": "R2",
  "title": "the runtime looked for beside the executable only",
  "group": "p1",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "    while tried < 3 {",
    "    while tried < 1 {"
   ]
  ],
  "runs": [
   "main",
   "clang",
   "golden"
  ]
 },
 {
  "id": 38,
  "key": "S1",
  "title": "the default stem's fallback `main`",
  "group": "p1",
  "file": "selfhost/src/loader.nova",
  "edits": [
   [
    "    if name == \"\" || name == \"..\" { return fallback }",
    "    if name == \"\" || name == \"..\" { return \"main\" }"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 39,
  "key": "L1",
  "title": "std's test module never read (the moved `load_std`)",
  "group": "p1",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "    if test {\n        let mut with_test",
    "    if false {\n        let mut with_test"
   ]
  ],
  "runs": [
   "t:both_sides_take_test_in_every_position_alike",
   "t:n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test",
   "ntest"
  ]
 },
 {
  "id": 40,
  "key": "L2",
  "title": "an I/O error's kind misnamed (the moved `io_kind_name`)",
  "group": "p1",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "        NotFound => \"NotFound\"",
    "        NotFound => \"Missing\""
   ]
  ],
  "runs": [
   "t:unreadable_files_are_reported_alike"
  ]
 },
 {
  "id": 41,
  "key": "CI1",
  "title": "the CI step's test name misspelt",
  "group": "p1",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "--ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang",
    "--ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clagn"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test",
   "cistep"
  ]
 },
 {
  "id": 42,
  "key": "W1",
  "title": "`built OUT` worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "            println(\"built ${out}\")",
    "            println(\"built: ${out}\")"
   ]
  ],
  "runs": [
   "clang",
   "golden"
  ]
 },
 {
  "id": 43,
  "key": "W2",
  "title": "`ok: FILE` worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "            println(\"ok: ${path}\")",
    "            println(\"ok ${path}\")"
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 44,
  "key": "W3",
  "title": "`check`'s usage line changed",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"usage: selfhost check FILE --std DIR\"",
    "\"usage: selfhost check FILE\""
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 45,
  "key": "W4",
  "title": "the Windows refusal worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"selfhost build links on Linux and macOS only\"",
    "\"selfhost build links on Linux only\""
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 46,
  "key": "W5",
  "title": "`build`'s `failed to open` worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "            fail(\"failed to open ${path}\", [e.message])",
    "            fail(\"failed to read ${path}\", [e.message])"
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 47,
  "key": "W6",
  "title": "`check`'s `failed to open` worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "        CompiledIoError(e) => fail(\"failed to open ${path}\", [e.message])",
    "        CompiledIoError(e) => fail(\"failed to read ${path}\", [e.message])"
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 48,
  "key": "W7",
  "title": "`could not compile` worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"could not compile due to ${errors} previous error${s}\"",
    "\"could not build due to ${errors} previous error${s}\""
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 49,
  "key": "W8",
  "title": "`previous error`'s plural",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "    let s = if errors == 1 { \"\" } else { \"s\" }",
    "    let s = if errors == 2 { \"\" } else { \"s\" }"
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 50,
  "key": "W9",
  "title": "`found N error(s)` worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "            fail(\"found ${n} error${s}\", [])",
    "            fail(\"found ${n} problem${s}\", [])"
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 51,
  "key": "W10",
  "title": "`found N error(s)`'s plural",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "            let s = if n == 1 { \"\" } else { \"s\" }",
    "            let s = if n == 2 { \"\" } else { \"s\" }"
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 52,
  "key": "W11",
  "title": "the emitter's head worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"internal LLVM IR generation error (this is a compiler bug)\"",
    "\"internal LLVM IR generation error\""
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 53,
  "key": "W12",
  "title": "`failed to write` worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "            fail(\"failed to write ${ll}\", [e.message])",
    "            fail(\"failed to create ${ll}\", [e.message])"
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 54,
  "key": "W13",
  "title": "no `clang`'s words changed",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "found for `--release`: install LLVM (so",
    "found for `--release`: install clang (so"
   ]
  ],
  "runs": [
   "main"
  ]
 },
 {
  "id": 55,
  "key": "W14",
  "title": "the release head worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"release build failed; generated LLVM IR left at ${ll}\"",
    "\"release build failed; LLVM IR left at ${ll}\""
   ]
  ],
  "runs": [
   "main",
   "clang"
  ]
 },
 {
  "id": 56,
  "key": "W15",
  "title": "`NOVA_RUNTIME_LIB` missing worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "which does not exist\"])",
    "which is missing\"])"
   ]
  ],
  "runs": [
   "clang"
  ]
 },
 {
  "id": 57,
  "key": "W16",
  "title": "no runtime's words changed",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"could not find the Nova runtime library (${name})",
    "\"could not find the runtime library (${name})"
   ]
  ],
  "runs": [
   "clang"
  ]
 },
 {
  "id": 58,
  "key": "W17",
  "title": "a spawn failure worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "Err([\"failed to spawn ${spawn}: ${debug_quoted(program)}\", e.message])",
    "Err([\"could not spawn ${spawn}: ${debug_quoted(program)}\", e.message])"
   ]
  ],
  "runs": [
   "clang",
   "ntest"
  ]
 },
 {
  "id": 59,
  "key": "W18",
  "title": "`linking failed` worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"linker\", \"linking\")",
    "\"linker\", \"link\")"
   ]
  ],
  "runs": [
   "clang"
  ]
 },
 {
  "id": 60,
  "key": "W19",
  "title": "`clang failed` worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"-o\", obj], \"clang\", \"clang\")?",
    "\"-o\", obj], \"clang\", \"Clang\")?"
   ]
  ],
  "runs": [
   "clang"
  ]
 },
 {
  "id": 61,
  "key": "W20",
  "title": "a failed tool's status line worded otherwise",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "\"${failed} failed (exit status: ${status}):\\n${out}\\n${err}\"",
    "\"${failed} failed (status: ${status}):\\n${out}\\n${err}\""
   ]
  ],
  "runs": [
   "clang",
   "ntest"
  ]
 },
 {
  "id": 62,
  "key": "W21",
  "title": "`Error:` lowercased",
  "group": "words",
  "file": "selfhost/src/driver.nova",
  "edits": [
   [
    "    out.push(\"Error: ${head}\")",
    "    out.push(\"error: ${head}\")"
   ]
  ],
  "runs": [
   "main",
   "ntest"
  ]
 },
 {
  "id": 63,
  "key": "CI2",
  "title": "a name dropped from the CI step",
  "group": "ci",
  "file": ".github/workflows/ci.yml",
  "edits": [
   [
    "--ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang",
    "--ignored --exact s1_builds_the_golden_suite"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test",
   "cistep"
  ]
 },
 {
  "id": 64,
  "key": "CI3",
  "title": "the clang test no longer `#[ignore]`d",
  "group": "ci",
  "file": "crates/nova-cli/tests/selfhost.rs",
  "edits": [
   [
    "#[test]\n#[ignore = \"needs clang; the Selfhost job's Linux step runs it\"]\nfn n_build_answers_as_s0_does_with_clang()",
    "#[test]\nfn n_build_answers_as_s0_does_with_clang()"
   ]
  ],
  "runs": [
   "t:the_ci_step_names_every_clang_test",
   "cistep"
  ]
 }
]
EOF
```

  Then `python3 "$P/exec55b/mutrun.py" --check`, which prints `64
  mutants: every edit's text found`. These names are set, for the
  baseline below, and `C` is the Runs block's:

  ```sh
  MAIN=n_build_and_check_answer_as_s0_does
  EXTRACT=the_extraction_keeps_the_headers_the_loci_and_the_error
  NTEST=the_selfhost_tests_pass
  CLANG=n_build_answers_as_s0_does_with_clang
  GOLDEN=s1_builds_the_golden_suite
  CITEST=the_ci_step_names_every_clang_test
  GUARD=the_resolution_dump_has_a_header_per_root_and_std_its_own_part
  ```

  - First, unmutated. Before the build, which changes the runtime: `df
    -h /` at least 7.0G, or remove Step 7's `opt-level = 1` `nova-*` and
    `selfhost-*` executables first, if still there (the replay read 6.8G
    and removed them, reaching 7.0G, 7,095 MiB). Then `cargo build --locked
    --workspace` (12 s; `nova version` prints `runtime: embedded`),
    `cargo test --locked --workspace --test selfhost --no-run`, `df -h /`
    (at least 6.3G; the replay read 6.5G, 6,570 MiB), then `SH --exact $MAIN
    $EXTRACT $NTEST $CITEST $GUARD` (`5 passed`, 9 s), `C $CLANG $GOLDEN`
    (`2 passed`, 35 s) and `G` (`147 passed, 9 excluded`, 16 s), so that
    every mutant runs against a passing baseline.
  - Each mutant runs only the tests it should trip and `$GUARD`, a cheap
    test that builds N and compares its resolution dump with S0's, which
    must pass every time but where a mutant's own words reach it (mutant
    40): a mutant that broke everything would fail it.
  - Run them in six batches, each one command in the foreground,
    `python3 "$P/exec55b/mutrun.py" 1-16` and so on, sized so that each
    fits under 600 s at twice the replay's time: 1 to 16 (273 s), 17 to
    25 (266 s), 26 to 29 (255 s), 30 to 35 (247 s), 36 to 53 (266 s) and
    54 to 64 (227 s); 1,534 s in all, one run. The runner prints each
    mutant's line, `CAUGHT` or `SURVIVED`, the tests that failed, N's
    `@test`s that failed and the probe tests' counts, to compare with
    the list below; a clang count out of 13, not 16, means `build`
    without `--release` failed, so the golden harness's three refusals,
    which run on its binary, did not. A result already written is kept,
    so a batch run again skips the mutants it has done. If a batch is killed, or the
    shell restarts, `git checkout -- <file>` the file of the mutant it was
    on, see `git status --short` print nothing, and run that batch again.
  - Each edit replaces one exact text, which occurs once in its file
    unless the mutant names which occurrence, counted on the text as
    shown ("the first of 2"); "the line … goes" removes that whole line.
  - Each command list starts with `cargo build --locked --workspace`
    (the runner runs it). A mutant of N needs no Rust rebuilt:
    `selfhost.rs` builds N from `selfhost/` each run.
  - Each caught mutant must fail as written: the tests named, and no
    other among those run; a probe test's count of failing cases, in
    parentheses, as given; a name after `the_selfhost_tests_pass` in
    parentheses is one of N's `@test`s, which that test runs. Four pass
    by design, each with its reason below: mutants 28, 31, 33 and 45.
    Any other mutant that passes is a gap: record it in the ledger, name
    it in the PR, and give it to the review (Step 10), which says whether
    a check is added.
  - At the end, `git status --short` prints nothing, then `unset
    NOVA_EMBED_RUNTIME`; before the build, which changes the runtime
    back, `df -h /` at least 7.0G, or remove the `nova-*` and
    `selfhost-*` executables in `target/debug/deps` first (the replay
    removed them first); then `cargo build --locked
    --workspace` (`runtime: not embedded`).

  1. **The intermediate named without `.nova`.** `selfhost/src/driver.nova`: `let file = "${name}.nova.${ext}"` becomes `let file = "${name}.${ext}"`.
     - Run `SH --exact $MAIN $NTEST $GUARD`, then `C $CLANG`.
     - Expected: fails:
       `n_build_and_check_answer_as_s0_does` (5 of 39 probes), `the_selfhost_tests_pass` (N's `@test` `the_intermediate_is_named_for_the_outputs_file_name_as_s0_names_it`), `n_build_answers_as_s0_does_with_clang` (11 of 16 clang cases).
  2. **`-O2` dropped.** `selfhost/src/driver.nova`: `run_tool(clang, ["-O2", "-c", ll, "-o", obj], "clang", "clang")?` becomes `run_tool(clang, ["-c", ll, "-o", obj], "clang", "clang")?`.
     - Run `SH --exact $MAIN $GUARD`, then `C $CLANG $GOLDEN`.
     - Expected: fails:
       `n_build_answers_as_s0_does_with_clang` (4 of 16 clang cases), `s1_builds_the_golden_suite` (147 of 147 golden programs).
  3. **The runtime looked for before the `clang` probe.** `selfhost/src/driver.nova`: replace

     ```
         let clang = env_or("NOVA_CLANG", "clang")
         if !tool_available(clang) { return Err([NO_TOOLCHAIN]) }
     ```

     with

     ```
         let runtime = find_runtime().await?
         let clang = env_or("NOVA_CLANG", "clang")
         if !tool_available(clang) { return Err([NO_TOOLCHAIN]) }
     ```

     - Run `SH --exact $MAIN $GUARD`, then `C $CLANG`.
     - Expected: fails:
       `n_build_and_check_answer_as_s0_does` (4 of 39 probes), `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  4. **The runtime looked for before `clang` runs.** `selfhost/src/driver.nova`: replace

     ```
         run_tool(clang, ["-O2", "-c", ll, "-o", obj], "clang", "clang")?
         let runtime = find_runtime().await?
     ```

     with

     ```
         let runtime = find_runtime().await?
         run_tool(clang, ["-O2", "-c", ll, "-o", obj], "clang", "clang")?
     ```

     - Run `SH --exact $MAIN $GUARD`, then `C $CLANG`.
     - Expected: fails:
       `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  5. **`-lm` dropped.** `selfhost/src/driver.nova`: `"-lpthread", "-ldl", "-lm"]` becomes `"-lpthread", "-ldl"]`.
     - Run `SH --exact $GUARD`, then `C $CLANG`.
     - Expected: fails:
       `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  6. **`-lpthread` dropped.** `selfhost/src/driver.nova`: `"-lpthread", "-ldl", "-lm"]` becomes `"-ldl", "-lm"]`.
     - Run `SH --exact $GUARD`, then `C $CLANG`.
     - Expected: fails:
       `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  7. **The runtime before the object on `cc`'s line.** `selfhost/src/driver.nova`: `["-o", out, obj, runtime, "-lpthread"` becomes `["-o", out, runtime, obj, "-lpthread"`.
     - Run `SH --exact $GUARD`, then `C $CLANG`.
     - Expected: fails:
       `n_build_answers_as_s0_does_with_clang` (6 of 13 clang cases).
  8. **The default output FILE's name, not its stem.** `selfhost/src/driver.nova`: `None => file_stem_or(root, "out")` becomes `None => file_name(root)`.
     - Run `SH --exact $MAIN $NTEST $GUARD`, then `C $CLANG`.
     - Expected: fails:
       `n_build_and_check_answer_as_s0_does` (1 of 39 probes), `the_selfhost_tests_pass` (N's `@test` `the_default_output_is_files_stem_as_path_file_stem_gives_it`), `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  9. **Anyhow's blank line before `Caused by:` dropped.** `selfhost/src/driver.nova`: `out.push("\n\nCaused by:")` becomes `out.push("\nCaused by:")`.
     - Run `SH --exact $MAIN $NTEST $GUARD`.
     - Expected: fails:
       `n_build_and_check_answer_as_s0_does` (10 of 39 probes), `the_selfhost_tests_pass` (N's `@test` `a_failure_is_printed_in_anyhows_layout`).
  10. **A failure's exit code 2.** `selfhost/src/driver.nova`: replace

      ```
          eprintln(failure_text(head, causes))
          exit(1)
      ```

      with

      ```
          eprintln(failure_text(head, causes))
          exit(2)
      ```

      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (25 of 39 probes).
  11. **Windows's drive read at `;`.** `selfhost/src/driver.nova`: `path.byte_at(1) != 58` becomes `path.byte_at(1) != 59`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `windows_is_known_by_a_drive_or_a_double_backslash`).
  12. **Windows known by `/`.** `selfhost/src/driver.nova`: `if path.starts_with("\\\\") { return true }` becomes `if path.starts_with("/") { return true }`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (9 of 39 probes), `the_selfhost_tests_pass` (N's `@test` `windows_is_known_by_a_drive_or_a_double_backslash`).
  13. **The `.ll` kept on success.** `selfhost/src/driver.nova`: the line `let _ = remove_file(ll).await` goes.
      - Run `SH --exact $MAIN $GUARD`, then `C $CLANG`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (5 of 16 clang cases).
  14. **A column per byte.** `selfhost/src/driver.nova`: `} else if b < 128 || b >= 192 {` becomes `} else {`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (2 of 39 probes), `the_selfhost_tests_pass` (N's `@test` `line_and_column_count_line_feeds_and_characters_as_codespan_does`).
  15. **`\r` a line break too.** `selfhost/src/driver.nova`: `if b == 10 {` becomes `if b == 10 || b == 13 {`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (1 of 39 probes), `the_selfhost_tests_pass` (N's `@test` `line_and_column_count_line_feeds_and_characters_as_codespan_does`).
  16. **A file's first label its locus.** `selfhost/src/driver.nova`: replace

      ```
              } else if (l.primary && !primary.at(i))
                  || (l.primary == primary.at(i) && l.span.start < starts.at(i)) {
      ```

      with

      ```
              } else if false {
      ```

      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `a_files_locus_is_its_earliest_primary_label_else_its_earliest_secondary`).
  17. **The later start the locus.** `selfhost/src/driver.nova`: `l.span.start < starts.at(i)) {` becomes `l.span.start > starts.at(i)) {`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `a_files_locus_is_its_earliest_primary_label_else_its_earliest_secondary`).
  18. **The loci's files reversed.** `selfhost/src/driver.nova`: `out.push((names.at(i), starts.at(i)))` becomes `out.push((names.at(names.len() - 1 - i), starts.at(names.len() - 1 - i)))`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (2 of 39 probes), `the_selfhost_tests_pass` (N's `@test` `a_diagnostic_prints_its_header_and_one_locus_per_file`, `a_files_locus_is_its_earliest_primary_label_else_its_earliest_secondary`).
  19. **The severity capitalised.** `selfhost/src/driver.nova`: `SeverityError => "error"` becomes `SeverityError => "Error"`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (15 of 39 probes), `the_selfhost_tests_pass` (N's `@test` `a_diagnostic_prints_its_header_and_one_locus_per_file`).
  20. **The golden binaries not compared.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
          if a == b {
              return Ok(());
      ```

      with

      ```
          if true {
              return Ok(());
      ```

      - Run `SH --exact $GUARD`, then `C $CLANG $GOLDEN`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  21. **The golden harness's stdout check loosened.** `crates/nova-cli/tests/golden/shared.rs`: `if out != expected {` becomes `if !expected.contains(out.trim()) {`.
      - Run `SH --exact $GUARD`, then `C $CLANG $GOLDEN`, then `G`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  22. **The extraction drops `warning[`.** `crates/nova-cli/tests/selfhost.rs`: `if ["error[", "warning[", "note[", "help["]` becomes `if ["error[", "note[", "help["]`.
      - Run `SH --exact $MAIN $EXTRACT $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (2 of 39 probes), `the_extraction_keeps_the_headers_the_loci_and_the_error`.
  23. **The extraction keeps a locus's own indent.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
                  out.push_str("  ┌─ ");
                  if windows {
      ```

      with

      ```
                  out.push_str(&line[..line.len() - locus.len()]);
                  if windows {
      ```

      - Run `SH --exact $MAIN $EXTRACT $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (5 of 39 probes), `the_extraction_keeps_the_headers_the_loci_and_the_error`.
  24. **An exclusion dropped (`nova_test`).** `crates/nova-cli/tests/golden/shared.rs`: replace

      ```
          ("nova_test", "a `@test` file, which `nova test` runs"),
      ```

      with nothing.

      - Run `SH --exact $GUARD`, then `C $GOLDEN`, then `G`.
      - Expected: fails:
        `s1_builds_the_golden_suite` (1 of 148 golden programs), `release_golden_suite`.
  25. **An exclusion's name changed.** `crates/nova-cli/tests/golden/shared.rs`: `("net_timeout", "its test serves the port it connects to"),` becomes `("net_timeouts", "its test serves the port it connects to"),`.
      - Run `SH --exact $GUARD`, then `C $GOLDEN`, then `G`.
      - Expected: fails:
        `s1_builds_the_golden_suite`, `release_golden_suite`.
  26. **The stdin entry changed.** `crates/nova-cli/tests/golden/shared.rs`: `&[("io_streams", "hello-stdin")]` becomes `&[("io_streams", "hello-stdin!")]`.
      - Run `SH --exact $GUARD`, then `C $GOLDEN`, then `G`.
      - Expected: fails:
        `s1_builds_the_golden_suite` (1 of 147 golden programs), `release_golden_suite`.
  27. **The stdin entry dropped.** `crates/nova-cli/tests/golden/shared.rs`: `&[("io_streams", "hello-stdin")]` becomes `&[]`.
      - Run `SH --exact $GUARD`, then `C $GOLDEN`, then `G`.
      - Expected: fails:
        `s1_builds_the_golden_suite` (1 of 147 golden programs), `release_golden_suite`.
  28. **An environment entry dropped.** `crates/nova-cli/tests/golden/shared.rs`: the line `("process_env", "NOVA_PROCESS_TEST_EQ", PROCESS_TEST_EQ),` goes.
      - Run `SH --exact $GUARD`, then `C $GOLDEN`, then `G`.
      - Expected: passes, by design: equivalent for any runtime whose `env` is right. `process_env` prints `None` for `env("NOVA_PROCESS_TEST_EQ=x")` whether the variable is set or not: `env` refuses a name holding `=` without asking the system, and the entry is there so that a wrong `env` would find `y`; its `.stdout` holds no other use of it. The entry moved unchanged from `run_tests.rs` (5.0's), so no output 5.5b adds depends on it.
  29. **An environment entry's value changed.** `crates/nova-cli/tests/golden/shared.rs`: `("process_env", "NOVA_PROCESS_TEST_VALUE", PROCESS_TEST_VALUE),` becomes `("process_env", "NOVA_PROCESS_TEST_VALUE", PROCESS_TEST_EQ),`.
      - Run `SH --exact $GUARD`, then `C $GOLDEN`, then `G`.
      - Expected: fails:
        `s1_builds_the_golden_suite` (1 of 147 golden programs), `release_golden_suite`.
  30. **The golden harness's stdout check skipped.** `crates/nova-cli/tests/golden/shared.rs`: `if out != expected {` becomes `if false {`.
      - Run `SH --exact $GUARD`, then `C $CLANG $GOLDEN`, then `G`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (2 of 16 clang cases).
  31. **S1's `built OUT` line not checked.** `crates/nova-cli/tests/selfhost.rs`: `if stdout == want {` becomes `if true {`.
      - Run `SH --exact $GUARD`, then `C $GOLDEN`.
      - Expected: passes, by design: equivalent for N's code: N prints `built ` and OUT as given, one `println` for every OUT (`build_command`), and the clang probe test's three success cases compare that line with S0's byte for byte, so a wrong line fails them (mutant 42 fails both tests). The check stays as the walk's own statement of decision 7.
  32. **N's `build` usage line changed.** `selfhost/src/driver.nova`: `"usage: selfhost build FILE [-o OUT] [--release] --std DIR"` becomes `"usage: selfhost build FILE [-o OUT] --std DIR"`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (6 of 39 probes).
  33. **`clang`'s arguments in another order.** `selfhost/src/driver.nova`: `run_tool(clang, ["-O2", "-c", ll, "-o", obj], "clang", "clang")?` becomes `run_tool(clang, ["-O2", "-c", "-o", obj, ll], "clang", "clang")?`.
      - Run `SH --exact $GUARD`, then `C $CLANG $GOLDEN`.
      - Expected: passes, by design: equivalent: `clang`'s driver takes its options and its input in any order, so `-O2 -c -o OBJ LL` is the same command as `-O2 -c LL -o OBJ` and writes the same object; `clang`'s line is not compared (only `cc`'s is, by the `cc-args` case), and every binary is still S0's byte for byte.
  34. **`clang` asked to link (`-c` dropped).** `selfhost/src/driver.nova`: `run_tool(clang, ["-O2", "-c", ll, "-o", obj], "clang", "clang")?` becomes `run_tool(clang, ["-O2", ll, "-o", obj], "clang", "clang")?`.
      - Run `SH --exact $GUARD`, then `C $CLANG $GOLDEN`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (11 of 13 clang cases), `s1_builds_the_golden_suite` (147 of 147 golden programs).
  35. **`cc`'s `-o OUT` last.** `selfhost/src/driver.nova`: `["-o", out, obj, runtime, "-lpthread", "-ldl", "-lm"]` becomes `[obj, runtime, "-lpthread", "-ldl", "-lm", "-o", out]`.
      - Run `SH --exact $GUARD`, then `C $CLANG $GOLDEN`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  36. **`NOVA_RUNTIME_LIB` not read.** `selfhost/src/driver.nova`: `match env("NOVA_RUNTIME_LIB") {` becomes `match env("NOVA_RUNTIME_LIB_UNSET") {`.
      - Run `SH --exact $MAIN $GUARD`, then `C $CLANG $GOLDEN`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (9 of 13 clang cases), `s1_builds_the_golden_suite` (147 of 147 golden programs).
  37. **The runtime looked for beside the executable only.** `selfhost/src/driver.nova`: `while tried < 3 {` becomes `while tried < 1 {`.
      - Run `SH --exact $MAIN $GUARD`, then `C $CLANG $GOLDEN`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  38. **The default stem's fallback `main`.** `selfhost/src/loader.nova`: `if name == "" || name == ".." { return fallback }` becomes `if name == "" || name == ".." { return "main" }`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `the_default_output_is_files_stem_as_path_file_stem_gives_it`).
  39. **Std's test module never read (the moved `load_std`).** `selfhost/src/driver.nova`: replace

      ```
          if test {
              let mut with_test
      ```

      with

      ```
          if false {
              let mut with_test
      ```

      - Run `SH --exact $NTEST both_sides_take_test_in_every_position_alike n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test $GUARD`.
      - Expected: fails:
        `both_sides_take_test_in_every_position_alike`, `n_resolves_std_for_each_program_when_std_gives_a_diagnostic_or_a_test`.
  40. **An I/O error's kind misnamed (the moved `io_kind_name`).** `selfhost/src/driver.nova`: `NotFound => "NotFound"` becomes `NotFound => "Missing"`.
      - Run `SH --exact unreadable_files_are_reported_alike $GUARD`.
      - Expected: fails:
        `unreadable_files_are_reported_alike`, `the_resolution_dump_has_a_header_per_root_and_std_its_own_part` (the guard: its dump holds `missing.nova`'s I/O error).
  41. **The CI step's test name misspelt.** `.github/workflows/ci.yml`: `--ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang` becomes `--ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clagn`.
      - Run `SH --exact the_ci_step_names_every_clang_test $GUARD`, then the step's own command as `ci.yml` then has it, with `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18`.
      - Expected: fails:
        `the_ci_step_names_every_clang_test`.
        The step's own command passes all the same, `1 passed`, `71 filtered out`: what the test is for.
  42. **`built OUT` worded otherwise.** `selfhost/src/driver.nova`: `println("built ${out}")` becomes `println("built: ${out}")`.
      - Run `SH --exact $GUARD`, then `C $CLANG $GOLDEN`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (5 of 16 clang cases), `s1_builds_the_golden_suite` (147 of 147 golden programs).
  43. **`ok: FILE` worded otherwise.** `selfhost/src/driver.nova`: `println("ok: ${path}")` becomes `println("ok ${path}")`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (3 of 39 probes).
  44. **`check`'s usage line changed.** `selfhost/src/driver.nova`: `"usage: selfhost check FILE --std DIR"` becomes `"usage: selfhost check FILE"`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (5 of 39 probes).
  45. **The Windows refusal worded otherwise.** `selfhost/src/driver.nova`: `"selfhost build links on Linux and macOS only"` becomes `"selfhost build links on Linux only"`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: passes, by design: unreachable on Linux and macOS: N refuses only when its executable's path has Windows's shape, so no input here reaches the words. On CI's Windows leg the main probe test compares N's stderr with `N_REFUSES_WINDOWS` for each `build` case, which this mutant fails there (spec §3.5, §16).
  46. **`build`'s `failed to open` worded otherwise.** `selfhost/src/driver.nova`: `fail("failed to open ${path}", [e.message])` (the first of 2) becomes `fail("failed to read ${path}", [e.message])`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (2 of 39 probes).
  47. **`check`'s `failed to open` worded otherwise.** `selfhost/src/driver.nova`: `CompiledIoError(e) => fail("failed to open ${path}", [e.message])` becomes `CompiledIoError(e) => fail("failed to read ${path}", [e.message])`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (2 of 39 probes).
  48. **`could not compile` worded otherwise.** `selfhost/src/driver.nova`: `"could not compile due to ${errors} previous error${s}"` becomes `"could not build due to ${errors} previous error${s}"`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (1 of 39 probes).
  49. **`previous error`'s plural.** `selfhost/src/driver.nova`: `let s = if errors == 1 { "" } else { "s" }` becomes `let s = if errors == 2 { "" } else { "s" }`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (1 of 39 probes).
  50. **`found N error(s)` worded otherwise.** `selfhost/src/driver.nova`: `fail("found ${n} error${s}", [])` becomes `fail("found ${n} problem${s}", [])`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (14 of 39 probes).
  51. **`found N error(s)`'s plural.** `selfhost/src/driver.nova`: `let s = if n == 1 { "" } else { "s" }` becomes `let s = if n == 2 { "" } else { "s" }`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (13 of 39 probes).
  52. **The emitter's head worded otherwise.** `selfhost/src/driver.nova`: `"internal LLVM IR generation error (this is a compiler bug)"` becomes `"internal LLVM IR generation error"`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (1 of 39 probes).
  53. **`failed to write` worded otherwise.** `selfhost/src/driver.nova`: `fail("failed to write ${ll}", [e.message])` becomes `fail("failed to create ${ll}", [e.message])`.
      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (1 of 39 probes).
  54. **No `clang`'s words changed.** `selfhost/src/driver.nova`: replace

      ```
      found for `--release`: install LLVM (so
      ```

      with

      ```
      found for `--release`: install clang (so
      ```

      - Run `SH --exact $MAIN $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (4 of 39 probes).
  55. **The release head worded otherwise.** `selfhost/src/driver.nova`: `"release build failed; generated LLVM IR left at ${ll}"` becomes `"release build failed; LLVM IR left at ${ll}"`.
      - Run `SH --exact $MAIN $GUARD`, then `C $CLANG`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (4 of 39 probes), `n_build_answers_as_s0_does_with_clang` (7 of 16 clang cases).
  56. **`NOVA_RUNTIME_LIB` missing worded otherwise.** `selfhost/src/driver.nova`: `which does not exist"])` becomes `which is missing"])`.
      - Run `SH --exact $GUARD`, then `C $CLANG`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  57. **No runtime's words changed.** `selfhost/src/driver.nova`: `"could not find the Nova runtime library (${name})` becomes `"could not find the runtime library (${name})`.
      - Run `SH --exact $GUARD`, then `C $CLANG`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  58. **A spawn failure worded otherwise.** `selfhost/src/driver.nova`: `Err(["failed to spawn ${spawn}: ${debug_quoted(program)}", e.message])` becomes `Err(["could not spawn ${spawn}: ${debug_quoted(program)}", e.message])`.
      - Run `SH --exact $NTEST $GUARD`, then `C $CLANG`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (1 of 16 clang cases).
  59. **`linking failed` worded otherwise.** `selfhost/src/driver.nova`: `"linker", "linking")` becomes `"linker", "link")`.
      - Run `SH --exact $GUARD`, then `C $CLANG`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (2 of 16 clang cases).
  60. **`clang failed` worded otherwise.** `selfhost/src/driver.nova`: `"-o", obj], "clang", "clang")?` becomes `"-o", obj], "clang", "Clang")?`.
      - Run `SH --exact $GUARD`, then `C $CLANG`.
      - Expected: fails:
        `n_build_answers_as_s0_does_with_clang` (2 of 16 clang cases).
  61. **A failed tool's status line worded otherwise.** `selfhost/src/driver.nova`: `"${failed} failed (exit status: ${status}):\n${out}\n${err}"` becomes `"${failed} failed (status: ${status}):\n${out}\n${err}"`.
      - Run `SH --exact $NTEST $GUARD`, then `C $CLANG`.
      - Expected: fails:
        `the_selfhost_tests_pass` (N's `@test` `a_failed_tools_cause_is_its_status_line_and_its_output_trimmed`), `n_build_answers_as_s0_does_with_clang` (4 of 16 clang cases).
  62. **`Error:` lowercased.** `selfhost/src/driver.nova`: `out.push("Error: ${head}")` becomes `out.push("error: ${head}")`.
      - Run `SH --exact $MAIN $NTEST $GUARD`.
      - Expected: fails:
        `n_build_and_check_answer_as_s0_does` (25 of 39 probes), `the_selfhost_tests_pass` (N's `@test` `a_failure_is_printed_in_anyhows_layout`).
  63. **A name dropped from the CI step.** `.github/workflows/ci.yml`: `--ignored --exact s1_builds_the_golden_suite n_build_answers_as_s0_does_with_clang` becomes `--ignored --exact s1_builds_the_golden_suite`.
      - Run `SH --exact the_ci_step_names_every_clang_test $GUARD`, then the step's own command as `ci.yml` then has it, with `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18`.
      - Expected: fails:
        `the_ci_step_names_every_clang_test`.
        The step's own command passes all the same, `1 passed`, `71 filtered out`: what the test is for.
  64. **The clang test no longer `#[ignore]`d.** `crates/nova-cli/tests/selfhost.rs`: replace

      ```
      #[test]
      #[ignore = "needs clang; the Selfhost job's Linux step runs it"]
      fn n_build_answers_as_s0_does_with_clang()
      ```

      with

      ```
      #[test]
      fn n_build_answers_as_s0_does_with_clang()
      ```

      - Run `SH --exact the_ci_step_names_every_clang_test $GUARD`, then the step's own command as `ci.yml` then has it, with `NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18`.
      - Expected: fails:
        `the_ci_step_names_every_clang_test`.
        The step's own command passes all the same, `1 passed`, `71 filtered out`: what the test is for.
- [ ] **Step 9: The review's inputs.** The full run's figures, the new
  tests by binary and `selfhost.rs`'s apart, the golden suite's line and
  time, Step 4's diffs, the CI step's names, the differential's times,
  the new step's, N's tests, S1 building N, and the mutants' outcome, with
  each gap, go in the ledger.
- [ ] **Step 10: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec
    and its amendment, ADR 0059 and this plan, and is told to hunt for:
    - a Windows-only difference: a `cfg(unix)` item used where it is not
      compiled, N's refusal reached otherwise than by its executable's
      shape, a locus's `\` (Review Focus 1);
    - a binary that is not S0's: `clang`'s or `cc`'s line, the runtime's
      order or search, the `.ll` or the object left, another runtime on
      one side (Review Focus 2);
    - a failure not in S0's words or layout: each head and cause of spec
      §3.4, the `Caused by:` block, the exit code (Review Focus 3);
    - `check`'s lines: the locus rule, a line or column counted wrong, a
      std locus, the extraction keeping or dropping a line it should not
      (Review Focus 4);
    - the command lines: a shape taken on one side and refused on the
      other, `check` taking `build`'s options (Review Focus 5);
    - a way for the harness to pass when the two sides differ: a table
      entry, the walk's floor, a check skipped, a case's leftovers not
      compared, a name in the CI step that runs no test, and whether
      `the_ci_step_names_every_clang_test` reads the step as CI does
      (Review Focus 6);
    - std's loading as it was before the move (Review Focus 7);
    - CI's time against the lines (Review Focus 8; decision 19);
    - a commit between Task 1 and Task 4 that builds wrong code or fails
      the suite, and code a task adds that its tests do not reach
      (decision 11);
    - the records: ADR 0059, the CHANGELOG, the dated notes, the spec's
      amendment, every count (147 and 9; 39 probes, 15 of `build` and 24
      of `check`; the clang test's 16 cases: `CLANG_PROBES`' 10, three on
      N alone, no runtime, a runtime two directories up and `build`
      without `--release`, and the golden harness's three refusals; 11
      `@test`s, 119 in all; 64 mutants) and figure against the tree,
      CI's runs (decision 9);
    - each mutant of Step 8 that passes, the accepted ones included, and
      whether a check is added;
    - MSRV 1.78, a new crate, a change to `Cargo.lock` or to a manifest,
      a test that touches the internet, a temporary name that is not
      fixed or missing from the Global Constraints' list.
  - Each finding is fixed in one commit, `5.5b: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, the Conventions' checks run again in full: `cargo
    fmt`, both clippy commands on the local stable and again on
    `+1.100.0` (`$P/exec55b/target-1100` made afresh after `df -h /` shows at
    least 6.5G, before the first of the two commands; removed after), the mimicked Windows clippy, and both
    `nova fmt --check` lines. Then Steps 1 and 2, `SH`, and every mutant
    of Step 8 whose file a fix changed, each with its commands and
    expected outcome as Step 8 gives them, in Step 8's batches.
- [ ] **Step 11: The PR.**
  - **Stop here, and have the main session ask the user** whether to push
    the branch and open the PR (Global Constraints). No push of this
    branch has been allowed: the user's answer of 2026-11-20 covered only
    the prototype's `…-55b-ci` and draft Sakeerin/nova#146. Nothing is
    pushed, and no PR opened, before their word; record their answer in
    the ledger.
  - Before asking, check that every commit the PR would carry ends with
    the two trailer lines of the Conventions: `git log --format=%B
    origin/main..HEAD`, its commits `17ecca86` (the spec's), the plan's,
    each task's and the review's fixes.
  - On the user's word: push, `git push -u origin
    claude/loving-archimedes-frtf5q-55b`, then open the PR to `main` with
    the repository's template headings. The body names no local path, no
    person and no model ID. It ends with this line, a blank line, and the
    session's address, exactly:

    ```
    🤖 Generated with [Claude Code](https://claude.com/claude-code)

    https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
    ```

  - It holds:
    - its commits: first `17ecca86`, the spec's; then the plan's, with
      the spec's amendment (decision 18); then each task's, and the
      review's fixes;
    - what 5.5b adds: N's `selfhost build` and `selfhost check`; the
      golden tables shared; S1's golden walk and the clang probe test in
      a new step of the Selfhost job's Linux leg, `clang-18` installed
      there; the main probe test and the extraction test; N's `@test`s;
      ADR 0059 and the notes;
    - what a user sees change: nothing; no S0 source, std, manifest or
      lockfile changed (Step 4), and the golden suite is 147 as before;
    - the departures, marked **(differs)** here: decision 11 (the cut,
      `build` and `check` separated), decision 12 (no user-visible
      change shown by the diff, without a base binary) and decision 14
      (§10's projections, §6.3's three added clang cases, §9's locus rule
      reached by the `@test` alone, the research's `frem.nova`, local
      clippy and `CC_ARGS`, the six mutants that survived until a case
      or test was added), each recorded in the
      spec's amendment; and the user's decisions 4 and 5, which the
      spec marks (differs) (Windows refused; no project mode, no std
      builtin);
    - the figures: the Test job's suite `2837 1 10` here and the counts
      expected on CI (decision 19), `selfhost.rs`'s 72 (2 ignored), the
      golden suite's `147 passed, 9 excluded`, `selfhost.rs`'s times here
      and CI's from runs 44672859855 and 44674948875 and `main`'s
      44479648345, the new step's, S1 building N, `Cargo.lock` unchanged;
    - the user's decisions and this plan's, numbered as here, with
      decisions 11, 12 and 14 marked as differing;
    - the mutants, the tests added for mutants 37 and 41, those
      accepted, and the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems; on the
    Selfhost job, `selfhost.rs` `70 passed; 0 failed; 2 ignored` on
    Linux, `68`, `2` on macOS and `64`, `2` on Windows,
    `the_ci_step_names_every_clang_test` passing on each, then, on Linux
    alone, the install printing `Ubuntu clang version 18.1.3 (1ubuntu1)`
    and the new step `2 passed; 0 failed; 0 ignored; 0 measured; 70
    filtered out` (a `1 passed` there means a name in the step runs no
    test, which the main step's test should have stopped: stop and
    report), then `the_selfhost_tests_pass` on the debug
    runtime; on the Test job's `ubuntu-24.04` leg, `cargo test`'s log
    holds `test release_golden_suite ... ok` and no `tests/selfhost.rs`;
    the Test job's main step, `cargo test --locked --workspace
    --all-features --no-fail-fast`, its `test result:` lines summed,
    `2838 passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9` on macOS
    and `2815, 0, 9` on Windows (decision 19); the advisory `-- --ignored`
    step after it is not counted, and what it shows is recorded as it is.
    Record each system's time for `selfhost.rs`, the install's and the
    new step's, the jobs' totals and the run's address in the ledger, and
    compare each with run 44674948875's in the PR's body.
  - **If a probe fails on CI and not here,** record its report, stop and
    ask the user: Windows's refusal or a locus's path is the likeliest
    (Review Focus 1). **If `selfhost.rs` takes more than 110 s on
    Windows, 100 s on macOS or 80 s on Linux,** ask the user (decision
    19). Never lower a count.
- [ ] **Step 12: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 11's.
