# Phase 4.8, "The gate and `v0.4.0`", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Phase 4's gate measured, Phase 4 closed, and its release. CI's
`browser` job runs Lighthouse 13.5.0 three times, with its mobile preset,
on 06's `/` and 07's `/`, `/posts/1` and `/login`, and fails when a run
fails or a page's median Performance score is under 95. `nova build
--release` for WASM works with binaryen's own `wasm-opt`: it passes
`--enable-bulk-memory` after `-Oz`, since the backend writes `memory.copy`
and `memory.fill`, which binaryen otherwise refuses; `nova-codegen-wasm`'s
tests validate every module with the MVP and bulk memory alone; and the
same job builds 06 and 07 with `--release` and binaryen 132 and drives both
builds in Chromium; the rules the Lighthouse step applies are tested
without Lighthouse, under `node --test`. `gate.sh`, which CI's `install`
and `package` jobs, the release's smoke test and the Release gate run,
builds its new project for WASM and runs it under Node, which CI requires
with `--node`. The root README and `nova-cli`'s
gain a "Web" section, stale lines are corrected, and ADR 0047 records Phase
4's boundary and calls it complete before the tag. Then `v0.4.0`, by the
runbook: the release pull request, each publish and the tag on the user's
word, and a records pull request.

**Architecture:**
- **`crates/nova-cli/src/bundle.rs`:** `WASM_OPT_FLAGS`, `-Oz` and
  `--enable-bulk-memory`, which `wasm_opt` passes before `app.wasm -o
  app.wasm`. **`crates/nova-cli/src/cmd/run.rs`:** `--release`'s help text.
- **`crates/nova-codegen-wasm/src/lib.rs`:** the test helper `valid()`,
  whose validator takes the MVP and bulk memory alone.
- **`tests/browser/lighthouse.mjs`** (new): builds and serves 06 and 07,
  runs Lighthouse's CLI on the four pages, judges each page's median, and
  stops both servers. **`lighthouse-rules.mjs`** (new): the pages, the run
  count, the threshold, a run's failure, the median and the verdict, which
  **`lighthouse.test.mjs`** (new) tests. **`spa.mjs`** and **`blog.mjs`:**
  an optional `--release`, which checks `dist/manifest.json` says
  `release` and `wasm_opt`; `blog.mjs` builds with `--ssr --release` then.
- **`.github/workflows/ci.yml`,** the `browser` job: Lighthouse and binaryen
  in the job's one `npm install`, the step "Release builds with binaryen's
  wasm-opt", then the steps "The Lighthouse gate's rules" and "Lighthouse
  on both examples", which run after a failure of the release builds too;
  and the `install` job's `gate.sh --node`.
- **`.github/scripts/gate.sh`:** `nova build --target wasm` in the new
  project, its `dist/` files, the magic number, and `node dist/app.js`,
  required by `--node`, which `package-gate.sh` and `release.yml`'s smoke
  test pass too, and `release-gate.yml` when the tag's `gate.sh` takes it,
  which `crates/nova-cli/tests/release_manifests.rs` checks it can tell.
- **Records:** ADR 0047; the "Web" sections; `nova-cli`'s description and
  keywords; `docs/releasing.md`; `agent.md`, `skill.md` and the examples'
  READMEs; dated notes in `00-MASTER-SPEC.md`, `30-FRONTEND.md`,
  `40-TOOLING.md`, `60-EXAMPLES.md` and the phase plan; `CHANGELOG.md`.
- **The release:** the version 0.4.0, by `docs/releasing.md` (Task 6).

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** before Task 6's release
  pull request, which changes only the workspace's own versions (spec §7).
- JS: `tests/browser/lighthouse.mjs`, `spa.mjs` and `blog.mjs`, ES modules
  run by Node 22, at least 22.19, with Playwright 1.56.1, Lighthouse 13.5.0
  and binaryen 132.0.0, which the CI job alone installs.
- Shell: `gate.sh`, under `bash` on all three systems, with `od`, and `node`
  where it is on `PATH`.
- Tests: `crates/nova-cli/tests/wasm_tests.rs`'s
  `release_runs_wasm_opt_when_it_is_on_path` and its size tests;
  `nova-codegen-wasm`'s unit tests; `release_manifests.rs`'s new
  `the_release_gate_passes_node_to_a_gate_sh_that_takes_it`;
  `tests/browser/lighthouse.test.mjs`, under `node --test`; the `browser`
  job's scripts; `gate.sh`.

**Spec:** `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`,
approved by the user on 2026-10-22 (`f5ed8b3`).
- Read it before Task 1: it is the authority this plan argues from.
- Its header holds the user's eight decisions, its §12 the seven decisions
  made while writing it, and its §2 the code as it stood at `24e7d7f`.
  This plan's decisions say where the code departs from the spec, or fills
  in what it left open; decisions 7 and 25 correct two statements of its
  §2.

**Checked on 2026-10-22, before execution:**
- **A prototype built 4.8 whole** on a local branch from `f5ed8b3`, in this
  tree, in two parts, a gap fix, the fact-check's fixes and the second
  check's:
  - part A, the code and CI: `c9cb49c` (the flag, the stand-in test and
    the codegen's check) and `ff1b081` (Lighthouse, the scripts'
    `--release`, CI and `gate.sh`). Before the fix, binaryen 132's
    `wasm-opt` refused every release build, hello world's, 06's and 07's:
    "memory.fill operations require bulk memory". It read every
    instruction the backend writes: beyond the MVP, `memory.copy` and
    `memory.fill` alone. With the flag, each release build validates in V8
    and its pages work. It ran the `browser` job's steps as CI does, every
    Lighthouse score 100, and forced a failure once by hand, an unloadable
    page and a threshold of 101: the script exited 1 with its
    `lighthouse: failed` line. It ran `gate.sh --test` on a lone debug
    `nova` built with `NOVA_EMBED_RUNTIME=1`, with `node` on `PATH` and
    without. Its local timings: the install 26 s (117 packages, 295 MB),
    the Lighthouse step 133 s, the release step 14 s. The full suite gave
    `2419 1 10`, the base's;
  - part B, the records (`1642f0a`): ADR 0047 and the rest of Task 4. It
    found no bug in part A, and found the spec's two errors (decisions 7
    and 25), the missing notes of `13-RUNTIME.md` and `14-CODEGEN.md`,
    which ADR 0047 records, and `gate.sh`'s comment citing "§7.1" for the
    user's decision 8;
  - the gap fix (`5f15c33 4.8 prototype: close five test gaps`), after the
    mutants' first run (below): `lighthouse-rules.mjs` and its test, and
    the test's step in CI (decisions 29 and 30); `gate.sh --node` and its
    three callers (decision 31); the records' Lighthouse scores said as
    measured; and `gate.sh`'s comment citing the spec's §8 and its header;
  - the fact-check's fixes (`55e6629`), below;
  - the second check's fixes (`845345c`), below.

  The parts' departures from the spec, and what they found, are this
  plan's decisions 11 to 26; the split's are 27 and 28, the gap fix's 29
  to 31, and the fact-check's 32 and 33, with 8, 16 to 18, 25 and 29 to
  31 changed; the second check changed 16, 17, 28, 30 and 31.
- **A fact-check** read this plan's draft against the prototype at
  `5f15c33`, the spec, the code and the runbook, and probed the rules and
  the workflows. Nothing would break CI; it found seven things, each fixed
  in `55e6629 4.8 prototype: the fact-check's fixes` or here, each code fix
  with a check that fails without it:
  1. Task 6 had the agent push the tag, which this session's credential
     cannot (`v0.3.0`'s push was refused with HTTP 403; the user pushed
     `v0.3.1` and `v0.3.2`): the user makes and pushes it now, with the
     commands in full, and the agent reads the Release run (Task 6, Step
     5);
  2. nothing said what to do if CI's Lighthouse differs from the local
     runs, though a slower runner scores lower and one failed run fails
     the step: Tasks 5 and 6 now stop and ask the user, record the lines
     and their benchmark indexes, rerun only on the user's word, and never
     change `PASS` or `RUNS` without it; the records quote CI's medians;
     and every page's line, a failing one's too, gives its benchmark
     indexes (decision 16);
  3. the run count was not enforced: two runs passed, the higher their
     median; `judge` now fails other than three (decision 29, mutant 36),
     and the stand-in step catches a measuring loop one run short and a
     failing verdict that exits 0 (decision 30, mutants 34 and 35);
  4. `gate.sh`'s header said the Release gate passes `--node`, and left out
     `publish-crates.yml`, which runs it through `package-gate.sh`: the
     header names each caller now (decision 31);
  5. the Release gate can take `--node` for a tag whose `gate.sh` takes it:
     its two steps now `grep` for the option, which `v0.3.1`'s and
     `v0.3.2`'s do not match, so `v0.4.0`'s gate requires its own Node run
     (decision 31, mutants 40 and 41, then CI only, now run here against
     the second check's test);
  6. records: the "Web" sections and `gate.sh`'s comment left out
     `dist/package.json`, now named; four records gave sizes without their
     compressor, and decision 25 credited figures to scripts that do not
     print them, both corrected; decision 8 and Task 4 put `PASS` in
     `lighthouse.mjs`, now `lighthouse-rules.mjs`; ADR 0047's last line,
     which keeps ADR 0025's wording, has decision 32;
  7. nits: `lighthouse.mjs` printed no `lighthouse:` line when it threw,
     and now does, with status 1 (mutants 38 and 39); a Lighthouse failure
     skipped the release builds' step, which now comes first (decision
     17); `ci.yml`'s overlong comment line is wrapped; Task 5 now expects
     `release.yml`'s run on the pull request; and two risks, an orphaned
     Chromium on SIGKILL and Lighthouse's unlocked dependencies, are
     decision 33. CI's install command, never run as one command, was run
     once into scratch: 117 packages in 10 seconds, `wasm-opt version 132`,
     Lighthouse 13.5.0, and Playwright 1.56.1 resolving (decision 17).
- **A second check** read the draft against `55e6629` and found no fault
  in the code, and eight smaller things, each fixed in `845345c 4.8
  prototype: the second check's fixes` or here; each code fix has a check
  that fails without it, shown by breaking the fix, but for the steps'
  order and their `if:`s (5), which no check here can run:
  1. the Release gate's `--node` could go in silence: a `gate.sh` whose
     `--node)` line was reformatted would no longer match the steps'
     `grep`, `v0.4.0`'s gate would run without `--node`, its Node run
     skipped, and no log would show it; and mutants 40 and 41 could not be
     run here. Each step now prints its flags, the line carries a comment,
     and `release_manifests.rs`'s
     `the_release_gate_passes_node_to_a_gate_sh_that_takes_it` holds the
     workflow and the script to each other: with the line drifted by a
     space it fails, "no line holds "--node) with_node=--node", which
     release-gate.yml greps for" (Task 3, Step 3; decision 31; mutants 40
     to 43). It is 4.8's one new test, so the full suite gives `2420 1
     10`;
  2. Task 6's Step 3 read `git diff --stat`'s counts as lines changed: it
     now expects `--numstat`'s 21, 23, 1 and 2 lines, each removed and
     added, and quotes `--stat`'s 42, 46, 2 and 4, as the bump gave on a
     scratch copy of the split's last tree;
  3. Task 6's Step 5 made and pushed the tag even when `origin/main`'s
     subject was not the release commit's: both now run only after a
     `test` of the subject passes;
  4. the rule for Lighthouse in CI, record, stop and ask, and never change
     `PASS` or `RUNS`, covered two pull requests' runs alone: it now covers
     CI on `main` after each merge (Task 5, Step 8; Task 6, Step 4) and
     the records pull request's run (Task 6, Step 9);
  5. with the release builds first, their failure skipped the rules' test
     and Lighthouse: by `if:`s on the steps' ids, both now run after it,
     though not after an earlier step's failure or in a cancelled run, and
     Lighthouse only once the rules pass (decision 17). actionlint checks
     the ids the conditions name (mutant 44); nothing here evaluates the
     conditions or checks the order (mutant 45, not run);
  6. ADR 0047 said the job's steps ran in an order they no longer have:
     "What runs" now lists them in the job's order, names the run-count
     rule and says when the last two run, and the transcript says it is
     from the job's earlier order;
  7. `lighthouse.mjs`'s line for a failure before the verdict kept the
     message's first line alone, so a failed build's reason was on stderr
     only; and a child that exits at once raised an unhandled EPIPE,
     status 1 but no `lighthouse:` line. The line now gives the reason, and
     `run` ignores the EPIPE (decision 16); Task 2's stand-in adds a `nova`
     that exits at once, which, with the handler taken out, gave no
     `lighthouse:` line in 5 runs of 5 (mutants 46 to 48);
  8. records: the CHANGELOG's `gate.sh` entry names `dist/package.json`;
     three overlong lines are wrapped (`lighthouse.mjs`,
     `lighthouse.test.mjs`, `30-FRONTEND.md`), and a short one, in 07's
     README, is mended.
- **A replay applied it** to `f5ed8b3` on a scratch branch, task by task,
  as the steps say.
  - A script split the prototype's final tree into five commits: Task 1's
    tests (`wasm_tests.rs` and the codegen's `valid()`), Task 1's code
    (`bundle.rs` and `cmd/run.rs`), Task 2 (the scripts, the rules and
    their test, and `ci.yml`'s `browser` job), Task 3 (`gate.sh`, and the
    three callers that pass `--node`, the `install` job's in `ci.yml`
    among them) and Task 4 (the records). Every file 4.8 changes changes in
    one commit, whole, but `ci.yml`, whose `browser` job's three hunks are
    Task 2's and whose `install` job's one is Task 3's (decision 28); the
    split's last tree is `845345c`'s. The split was made of `1642f0a`,
    then of `5f15c33`, then of `55e6629`, then of `845345c`: Task 1's two
    patches are as they were each time, and Tasks 2 to 4's take the fixes;
    Task 3 holds the Release gate's two steps and the test of their `grep`
    (decision 28).
  - The patches below are those commits. Each applies with plain `git
    apply` in order on `f5ed8b3`, with no warning: no added line holds
    trailing whitespace, a raw CR or a `\u` escape, and the only non-ASCII
    characters added are `§`, `—` and `…`.
  - Task 1's tests fail cleanly before its code: the stand-in test at
    `wasm_tests.rs:948`, its `wasm-opt` given `-Oz` alone; the codegen's
    narrower check passes there, as it should (decision 27).
  - Every "Expected:" below is the walk's, from a fresh `NOVA_HOME`. The
    `browser` job ran from `ci.yml`'s own text, step by step, through
    `ci_step` (Conventions), after the runtime library was removed and
    built again as the job builds it, with Playwright from
    `/opt/node-tools` and part A's Lighthouse and binaryen in a scratch
    prefix laid out as CI's. `gate.sh` ran as CI's jobs run it, on a lone
    `nova` that embeds its runtime, with `--node`, and without `node`,
    with `--node` and without; the Release gate's two steps, but their
    call of `gate.sh`, printed `gate.sh --test --node`. Task 2's stand-in
    ran from Conventions' block. Every step was replayed again on each
    split.
- **The mutants.** 48 breakages of what 4.8 adds, each a text replaced in
  one file, run one at a time at the split's last commit (Task 5, Step 4):
  the flags (5), the codegen's check (2), the Lighthouse step, its rules
  and its stand-in (24), `spa.mjs` and `blog.mjs` (3), CI's release step,
  install and conditions (6), `gate.sh` (5) and the Release gate (3). A
  mutant of a script, of the rules or of `gate.sh` ran what CI runs,
  through `ci_step` or `gate_run`, or Task 2's stand-in, through
  `lh_stand_in`; one of the Release gate's steps or of `gate.sh`'s
  `--node)` line, `release_manifests`; one of a step's `id`, actionlint.
  - **The first run,** of 28 on `1642f0a`, caught 22. Five passed, each a
    gap: `lighthouse.mjs` with its median read past the scores' end, whose
    `undefined` is not under 95 (mutant 10), with one run a page (11), with
    a page whose run failed reporting no failure (12), and with `/login`
    left out (15), its verdict being exercised on passing pages alone; and
    `gate.sh` skipping its Node run where `node` is on `PATH` (28). The
    gap fix closed all five (decisions 29 to 31).
  - **The second,** of 33 on the gap fix, `5f15c33`, caught all 32 it ran:
    mutants 9 to 12 and 15 ran the rules' test, and 29 to 33 broke the
    rules' own subjects, a run with no score, a median that is not a
    number, Lighthouse's exit status, a runtime error and a missing report.
  - **The third,** of 41 on the fact-check's fixes, `55e6629`, adds the
    measuring loop one run short (34) and a failing verdict that exits 0
    (35), which the stand-in catches; the run count unchecked (36) and a
    failing page's line without its benchmark indexes (37), which the
    rules' test catches; a failure before the verdict without its
    `lighthouse:` line (38) or with status 0 (39), which the stand-in
    catches; and the Release gate's two steps without `--node` (40, 41),
    then not run. All 38 it ran failed as written.
  - **The fourth,** of 48 on the second check's fixes, `845345c`, runs 40
    and 41 against the new test, and adds `gate.sh`'s `--node)` line
    reformatted (42) and a Release gate step that does not print its flags
    (43), which the test catches; the release step's `id` renamed (44),
    which actionlint catches; the rules' step without its `if:` (45); and,
    in `lighthouse.mjs`, a child's broken stdin unhandled (46), a failed
    build's line without its reason (47) and the reason taken from the last
    line, not the `error` line (48), which the stand-in catches. Of the 46
    run, all 46 failed as written:
    - the stand-in test for the flag dropped, `-Oz` dropped and the flags
      swapped; binaryen's own `wasm-opt` refusing 06's build without the
      flag ("memory.fill operations require bulk memory", then `Fatal:
      error validating input`); and `spa.mjs --release` refusing a
      manifest whose `wasm_opt` said false while `wasm-opt` ran;
    - the codegen's `tests::memory_accesses_lower_in_line`, with bulk
      memory taken out of `valid()`, and with an `i64.extend32_s` written
      after `memory.grow`'s `i64.extend_i32_s`. Run on the base's
      `valid()`, that second mutant passed all 34 tests: the narrowed check
      is what catches it (decision 11);
    - the rules' test, under the threshold at 101, the median read past the
      end, one run a page, a failed run's failure dropped, `/login` left
      out, 29 to 33, 36 and 37, each failing the tests it names;
    - the stand-in, under 34, 35, 38, 39 and 46 to 48, each with its
      `stand-in: NOT` line;
    - `the_release_gate_passes_node_to_a_gate_sh_that_takes_it`, under 40
      to 43: a step with no run of `gate.sh` with the flags, under either
      call made `--test` alone; "no line holds", under the line
      reformatted; and a step with no echo of the flags;
    - actionlint, under 44: `property "release" is not defined`;
    - Lighthouse's step, under its Node floor raised, the score read from a
      category it does not run, `/posts/99` measured in place of
      `/posts/1`, on which Lighthouse exits 1 each run, "Runtime error
      encountered: Lighthouse was unable to reliably load the page you
      requested … (Status code: 404)", and each server left running, which
      leaves the step running after its `lighthouse:` line, stopped at six
      minutes;
    - `spa.mjs` not taking `--release`, which then refuses its arguments
      with its usage; `blog.mjs --release` building without `--release`,
      or running the server from `target/debug/`; CI building 06 without
      `--release` in the release step; binaryen's `.bin` kept off `PATH`,
      so `wasm-opt --version` is not found; `shrank` comparing the other
      way; each of `gate.sh`'s three checks inverted; and its Node run
      skipped, which `--node` fails: "gate: --node, but there is no node
      on PATH to run the WASM build".

    Two are not run: binaryen left out of CI's install (24), which runs in
    CI alone, and the rules' step without its `if:` (45), since nothing here
    evaluates GitHub's conditions.
- **The gate passed in the replay,** locally: every step of the `browser`
  job, in its new order: the release builds with binaryen 132, 06's client
  11,974 bytes gzipped by zlib, from 20,997, and 07's 27,510, from 49,472,
  `app.wasm` 16,525 bytes from 40,491, and 57,179 from 116,579, as part A
  measured; then the rules' eight tests; then Lighthouse's four pages,
  every run 100 in the job's own step.
- **The final checks ran on the split's last commit:**
  - `cargo fmt`, clippy with `-D warnings`, `nova fmt` and actionlint were
    clean after every task;
  - the full suite gave 2420 passed, 1 failed (the IPv6 test) and 10
    ignored: the base's figures and 4.8's one new test, which a comparison
    by binary finds added, and none removed;
  - the size tests print hello world at 5,006 bytes and 06 at 21,070, by
    `flate2` at level 9, as at the base;
  - Rust 1.78's check was clean, and `Cargo.lock` did not change.
- **What the checks left standing,** each recorded or handed on:
  - every page's median here was 100, so no run of Lighthouse tells a
    threshold of 95 from one of 90, or the median of three from the best of
    three; `lighthouse.test.mjs` pins both, on scores the real runs do not
    give (decision 30). Runs do vary: of the first replay's 94 runs that
    gave a score, its job's and its mutants', 93 scored 100 and one, a run
    of 07's `/`, scored 93, its page's median still 100; of the second
    replay's 45, 44 scored 100 and one, of `/posts/1`, 99; of the third's
    45, 44 scored 100 and one, again of `/posts/1`, 99; and of the fourth's
    45, 44 scored 100 and one, a run of 06's `/` under mutant 17, 94, under
    the threshold, its page's median still 100. The records say so since
    the gap fix: part A's runs all scored 100, and one of the 94 scored 93
    (ADR 0047's "Measured again", the CHANGELOG, `60-EXAMPLES.md` §6 and
    §7);
  - the Release gate's run with `--node`, and Lighthouse's numbers on
    GitHub's runner, are CI's to show: the first when `v0.4.0` is released
    (Task 6, Step 8), where each step's log prints its flags, the second in
    Task 5's CI run, with the user's word on any difference; here,
    `release_manifests.rs` holds the gate's `grep` to `gate.sh`'s line;
  - the order of the `browser` job's last three steps, and the `if:`s that
    run the rules' test and Lighthouse after a failure of the release
    builds, have no check that fails without them: `ci_step` runs every
    step whatever its condition, actionlint checks only their syntax and
    the ids they name, and nothing here evaluates GitHub's conditions;
    decision 17 argues them from GitHub's documentation, and CI shows them
    only when a release build fails (mutant 45, not run);
  - the two accepted risks of decision 33;
  - the 180-second limit, the signals and the servers' stop are tested by
    no test; mutants 16 and 17 show a server left running leaves the step
    running;
  - CI's Node minor: the job's `node-version: 22` names none, and CI's logs
    could not be read from here; `lighthouse.mjs` refuses one below 22.19
    (decision 15), and Task 5's CI run shows it;
  - `blog.mjs --release` needs the runner's `clang` for the server's
    release build (decision 14), and `shrank` reads the debug builds at the
    paths the earlier steps left them (decision 17);
  - `13-RUNTIME.md` and `14-CODEGEN.md` lack dated notes for Phase 4's
    later sub-phases (ADR 0047, "Found outside this boundary"); 4.8 adds
    none;
  - the spec's §2 and §12, decision 7, keep their text; decisions 7 and 25
    correct them, and ADR 0047 says the same;
  - this machine ran `gate.sh` on Linux alone; Windows and macOS run it in
    CI's `install` and `package` jobs and in the Release gate.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`. No
  `std::sync::LazyLock` or `LazyCell` (1.80), no `Option::is_none_or`
  (1.82), no `std::path::absolute` (1.79). 4.8's Rust is one constant, one
  call and a test helper's features.
- **No new crate.** `wasmparser`'s `WasmFeatures`, which the codegen's test
  helper names, is in `wasmparser` 0.243.0, already in the lock.
- **`Cargo.lock` does not change in Tasks 1 to 5.** It changes in Task 6's
  release pull request alone, and there only in the workspace's own
  packages' versions, `0.3.2` to `0.4.0`, as every release has: approving
  the spec approved that change (its §7, item 3), and nothing else in the
  lock. Any other change to it stops the task: ask the user.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`.
- **Tests never touch the internet.** Lighthouse and binaryen run in CI's
  `browser` job, installed by that job; nothing in `cargo test` or `nova`
  installs a package, and nothing installs `wasm-opt` for a user. Every
  server a script starts binds `127.0.0.1:0`: `lighthouse.mjs` serves 06's
  `dist/` on port 0 and starts the blog's server with `--address
  127.0.0.1:0`, reading the port from its `blog: serving
  http://127.0.0.1:<port>/` line, as `blog.mjs` does. **No test uses port
  3000.**
- **npm installs happen in CI only.** The local runs in Tasks 2 and 5 use
  Playwright 1.56.1 from `/opt/node-tools` and the Lighthouse 13.5.0 and
  binaryen 132.0.0 installs the prototype made in scratch, `$P/s48/lh` and
  `$P/s48/binaryen`, linked into one npm prefix laid out as CI's
  (Conventions). Where either install is missing, the local Lighthouse run
  and the release builds are skipped, and the ledger says so: CI runs them.
  The stand-in step (Task 2) needs Playwright alone. CI's own install
  command was run once, into scratch, while the plan was checked (decision
  17); the steps run none.
- **`lighthouse.test.mjs` runs under Node's own test runner,** `node
  --test`, with `node:test` and `node:assert` alone: no npm package, no
  browser, no network.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets.
  Run the WASM tests here with `NOVA_REQUIRE_NODE=1`: Node 22 is installed.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary. A build of the
  workspace writes both; Task 0's build puts them there. Every run step
  starts with `cargo build --locked -p nova-cli -p nova-runtime`
  (Conventions), so the `nova` the steps run by hand is the tree's.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and a test whose binary goes fails.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other (4.0's ledger). Never build in another worktree.
- **Keep `df -h /` above 6G.** `target/debug/incremental` and older
  duplicate test executables in `target/debug/deps` may be removed; Task
  5's Rust 1.78 check uses its own target directory, removed after.
- **Never use the Write or Edit tool for text that holds a `\u` escape,**
  a backslash and `u` followed by four hex digits or by a code in braces:
  they decode the escape into the character. None of 4.8's patches holds
  one, and neither does this plan but for this line; apply the patches
  with `patch_block`, from the committed plan, and write anything that
  holds one with a script.
- **Nothing writes into the repository but the patches.** The browser
  scripts copy the examples into their work directories, outside the
  tree, and build there; `gate.sh` makes its project in its work
  directory. Task 2 checks it.
- **Nothing is published, and no tag is pushed, until Task 6,** and there
  each publish, the tag and each merge wait for the user's word. Merge by
  rebase, on the user's word only. Never enable auto-merge.
- **This machine is Linux.** Windows and macOS results come from CI:
  `gate.sh` runs there under Git Bash in the `install` and `package` jobs
  and in the Release gate, with `node` on the runners' `PATH`.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-48x.md`. Every ruling made while executing goes
  there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-48-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-48-<task>.txt"`.
- **Runs.** The replay ran every command with these variables set. Each
  shell sets them too, with three shorthands, and runs every command from
  the repository's root:

  ```sh
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-48"
  unset NOVA_RUNTIME_LIB
  mkdir -p "$NOVA_HOME"
  export RUNNER_TEMP="$P/runner-48" PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers
  W() { NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests -- "$@"; }
  ci_step() { python3 "$P/ci_step-48.py" "$@"; }
  lh_stand_in() { bash "$P/lh-stand-in-48.sh" "$P/lh-stand-in-48"; }
  gate_run() {
    rm -rf "$P/gate-48/work"
    env -u NOVA_HOME -u NOVA_REQUIRE_NODE -u NOVA_RUNTIME_LIB bash .github/scripts/gate.sh "$P/gate-48/bin/nova" "$P/gate-48/work" --test --node
  }
  ```

  So `W release_runs_wasm_opt_when_it_is_on_path` runs that one test of
  `wasm_tests`; `ci_step NAME` runs a step of CI's `browser` job (below);
  `lh_stand_in` runs `lighthouse.mjs` with a stand-in Lighthouse (below);
  and `gate_run` runs `gate.sh --test --node` on Task 3's `nova` in a
  fresh work directory, as CI's `install` job does, with no `NOVA_`
  variable set, which the script refuses.
- **CI's `browser` job, run here from `ci.yml`'s own text.** Task 2 writes
  `$P/ci_step-48.py` from this block, with a shell's here-document, not the
  Write tool:

  ```sh
  cat > "$P/ci_step-48.py" <<'EOF'
  """Run steps of CI's `browser` job as .github/workflows/ci.yml has them:
  each step's `run:` text under `bash -e`, from the repository's root, as a
  runner runs a step with no `shell:`. `--job` runs every step after the
  build and the install; names run those steps."""
  import subprocess
  import sys

  import yaml

  REPO = "/home/user/nova"
  BUILD = "cargo build -p nova-cli -p nova-runtime"
  INSTALL = "Install Playwright, Chromium, Lighthouse and binaryen"
  steps = yaml.safe_load(open(REPO + "/.github/workflows/ci.yml"))["jobs"]["browser"]["steps"]
  runs = [(s["name"], s["run"]) for s in steps if "run" in s]
  assert runs[0][0] == BUILD and runs[2][0] == INSTALL, runs[:3]
  if sys.argv[1:] == ["--job"]:
      chosen = [r for r in runs if r[0] not in (BUILD, INSTALL)]
  else:
      chosen = [(n, dict(runs)[n]) for n in sys.argv[1:]]
  for name, text in chosen:
      print(f"### {name}", flush=True)
      code = subprocess.run(["bash", "-e", "-c", text], cwd=REPO).returncode
      if code != 0:
          print(f"### {name}: exit {code}", flush=True)
          sys.exit(code)
  print("### every step passed", flush=True)
  EOF
  ```

  Task 2 also writes `$P/lh-stand-in-48.sh` from this block, the same way:
  `lighthouse.mjs` with a stand-in for Lighthouse, which checks the run
  count, the failing verdict and its status, and the `lighthouse:` line of
  a failure before the verdict, with no Lighthouse and with a `nova` that
  exits at once, without Lighthouse or a browser run:

  ```sh
  cat > "$P/lh-stand-in-48.sh" <<'EOF'
  #!/bin/bash
  # lh-stand-in.sh DIR: `tests/browser/lighthouse.mjs`, run from the
  # repository's root on `target/debug/nova`, with a stand-in for Lighthouse,
  # in DIR, emptied first. DIR is an npm directory: Playwright from
  # /opt/node-tools, and a `lighthouse/cli/index.js` that writes a report
  # whose Performance score is 0.4 and benchmark index 1000 where
  # --output-path says, and exits 0. The script must run three runs on each
  # of the four pages, fail each page, print a failing `lighthouse:` line and
  # exit 1. Then, from DIR/no-cli, which has Playwright and no Lighthouse,
  # it must print a failing `lighthouse:` line with its reason and exit 1.
  # Then, on DIR/nova-fails, a `nova` that exits at once, before it reads
  # its stdin, with an `error:` line and a last line after it, it must
  # print a failing `lighthouse:` line with the `error:` line as the build's
  # reason, and exit 1. Prints the three runs' lines and what it checked;
  # exits 0 when all held.
  set -u
  dir=$1
  rm -rf "$dir"
  mkdir -p "$dir/node_modules/lighthouse/cli" "$dir/no-cli/node_modules"
  for d in "$dir" "$dir/no-cli"; do
    ln -s /opt/node-tools/node_modules/playwright /opt/node-tools/node_modules/playwright-core "$d/node_modules/"
  done
  printf '#!/bin/sh\necho "error: a stand-in nova that fails at once" >&2\necho "a last line, not the reason" >&2\nexit 2\n' > "$dir/nova-fails"
  chmod +x "$dir/nova-fails"
  cat > "$dir/node_modules/lighthouse/cli/index.js" <<'JS'
  // A stand-in for Lighthouse's CLI: a report whose Performance score is 0.4,
  // where --output-path says, and status 0.
  const { writeFileSync } = require("node:fs");
  const flag = "--output-path=";
  const out = process.argv.find((a) => a.startsWith(flag)).slice(flag.length);
  const report = {
    lighthouseVersion: "stand-in",
    categories: { performance: { score: 0.4 } },
    audits: {},
    environment: { benchmarkIndex: 1000, hostUserAgent: "HeadlessChrome/0.0.0.0" },
    runWarnings: [],
  };
  writeFileSync(out, JSON.stringify(report));
  JS
  page="scores 40, 40, 40, median 40; medians FCP none, LCP none, TBT none, CLS none, SI none; benchmark index 1000, 1000, 1000"
  summary="lighthouse: failed, 06's / and 07's /, /posts/1 and /login, 3 runs each, mobile: 06 /'s median is 40, under 95; 07 /'s median is 40, under 95; 07 /posts/1's median is 40, under 95; 07 /login's median is 40, under 95"
  no_cli="lighthouse: failed, before its verdict: Lighthouse's CLI is not at $dir/no-cli/node_modules/lighthouse/cli/index.js"
  fails="lighthouse: failed, before its verdict: nova build --target wasm of 06 exited with 2: error: a stand-in nova that fails at once"
  ok=0
  held() {
    if [ "$2" = yes ]; then echo "stand-in: $1"; else echo "stand-in: NOT $1"; ok=1; fi
  }
  yes_if() { if "$@"; then echo yes; else echo no; fi; }

  out=$(node tests/browser/lighthouse.mjs target/debug/nova "$dir/work" "$dir" 2> "$dir/stderr.txt")
  code=$?
  echo "$out"
  held "exit 1" "$(yes_if [ "$code" = 1 ])"
  pages=0
  for name in "06 /" "07 /" "07 /posts/1" "07 /login"; do
    grep -qxF -- "$name: $page" <<< "$out" && pages=$((pages + 1))
  done
  held "four pages, each scoring 40 in three runs" "$(yes_if [ "$pages" = 4 ])"
  held "the failing lighthouse: line" "$(yes_if grep -qxF -- "$summary" <<< "$out")"

  out=$(node tests/browser/lighthouse.mjs target/debug/nova "$dir/work-no-cli" "$dir/no-cli" 2> "$dir/stderr-no-cli.txt")
  code=$?
  echo "$out"
  held "exit 1 with no Lighthouse" "$(yes_if [ "$code" = 1 ])"
  held "a failing lighthouse: line, with its reason" "$(yes_if grep -qxF -- "$no_cli" <<< "$out")"

  out=$(node tests/browser/lighthouse.mjs "$dir/nova-fails" "$dir/work-nova-fails" "$dir" 2> "$dir/stderr-nova-fails.txt")
  code=$?
  echo "$out"
  held "exit 1 with a nova that fails at once" "$(yes_if [ "$code" = 1 ])"
  held "a failing lighthouse: line, with the build's reason" "$(yes_if grep -qxF -- "$fails" <<< "$out")"
  exit $ok
  EOF
  ```

  The build is the step's own (Task 2 removes the runtime library first),
  and the install is replaced by the scratch prefix: `$P/pw-48`, laid out
  as CI's `"$RUNNER_TEMP/pw"`, whose `node_modules` links Playwright from
  `/opt/node-tools` and part A's Lighthouse and binaryen, with `wasm-opt`
  in its `.bin`. `python3`'s `yaml` (PyYAML 6.0.1) reads the workflow.
- **The build.** Each "Run" step below starts with

  ```sh
  cargo build --locked -p nova-cli -p nova-runtime
  ```

  which ends with `Finished`, so that the `nova` the steps run by hand,
  and the runtime library beside it, are the tree's (Global Constraints).
  The replay began every step with it.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the
  repository's root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-22-phase-4-8-the-gate-and-v0.4.0.md   # this file, committed
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
  nothing; no 4.8 patch adds a line with trailing whitespace, so `git
  apply` warns for none. A block that does not apply means the tree differs
  from the plan's: stop, and record why in the ledger.
- **Each task** is its patch, its run and its checks. Task 1 is test first:
  its tests' patch, their run and the expected failure, then its code's
  patch and the expected pass. Tasks 2 to 4 are a patch each: the browser
  scripts, the Lighthouse rules' test and CI, `gate.sh` and its callers,
  and the records; Task 5's mutants show the test's, the scripts' and the
  gate's checks bite. Each task then runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy, `cargo clippy --locked --all-targets
    --all-features -- -D warnings`, as CI runs it, which ends with
    `Finished` and no warning;
  - `cargo run --locked -q -p nova-cli -- fmt --check std examples
    tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app
    tests/ssr`, which prints nothing. 4.8 adds no Nova source;
  - where `$P/actionlint-dl/actionlint` is (actionlint 1.7.7), that
    program, run from the root with no argument, which reads every
    workflow and prints nothing; it checks, among the rest, the step ids
    that `ci.yml`'s `if:`s name (decision 17).

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost if
  wrong.

## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
check in the task that owns the code:

1. **The flags against what the backend emits:** `WASM_OPT_FLAGS`, `-Oz`
   and `--enable-bulk-memory`, before `app.wasm -o app.wasm`; the
   backend's instructions beyond the MVP are `memory.copy` and
   `memory.fill` alone (`MemOp::Copy` and `MemOp::Fill` in
   `crates/nova-codegen-wasm/src/lib.rs`), and binaryen 132's own
   defaults, sign extension and mutable globals, stay; `valid()` takes the
   MVP and bulk memory alone, so a module that needs another feature fails
   `cargo test` before a user's `--release` build does (decisions 11 and
   12) → `release_runs_wasm_opt_when_it_is_on_path` (Task 1), the codegen's
   tests (Task 1), the release builds with binaryen (Task 2); mutants 1 to
   7.
2. **Lighthouse's flags, its median and threshold:** the mobile preset,
   its default, with simulated throttling; `--only-categories=performance`,
   `--output=json`, `--output-path`, `--quiet`, `--chrome-flags="--headless=new
   --no-sandbox"`, `CHROME_PATH` Playwright's Chromium; three runs a page;
   the median of three, `values[1]` once sorted, and a page failing when it
   is under 95 or has other than three runs; the four pages (decisions 8,
   9, 16 and 29). Every median here was 100, so no run of Lighthouse tells
   a wrong median, a lowered threshold, a wrong run count or a page left
   out from the right ones: `lighthouse.test.mjs` does, and the stand-in
   step checks the loop that makes the runs; the reviewer reads what they
   pin → `lighthouse-rules.mjs`, `lighthouse.test.mjs` and the stand-in
   (Task 2); mutants 8 to 11, 15, 34 and 36.
3. **Lighthouse's failure paths and the servers stopped:** a run failing on
   a status other than 0, on the 180-second limit (SIGINT, then SIGKILL),
   on no report or one that does not parse, on a `runtimeError`, on no
   Performance score; every run running before the verdict; a page with a
   failed run failing whatever its other scores; `process.exitCode = 1`,
   for a failing verdict and for a failure before it, which also prints a
   `lighthouse: failed, before its verdict:` line with its reason, a failed
   build's or server's first `error` line or else its last; a child that
   exits before it reads its stdin, whose EPIPE is ignored; the benchmark
   indexes on every page's line, a failing one's too; both servers stopped
   in `finally`, so the job's step ends (decisions 16 and 29) → `runOf`
   and `judge`, tested by `lighthouse.test.mjs` with a failed run, a run
   with no score and a page with no runs, the stand-in's statuses and
   lines, a `nova` that exits at once among them, and Lighthouse failing a
   `404` (Task 2); mutants 12 to 14, 16, 17, 29 to 33, 35, 37 to 39 and 46
   to 48. The timeout, the signals and the servers are
   exercised by no test, only by mutants 16 and 17, which leave the step
   running; a SIGKILL can leave Chromium running (decision 33).
4. **The release scripts' checks:** `spa.mjs --release` and `blog.mjs
   --release` refusing a manifest that does not say `release` and
   `wasm_opt`; `blog.mjs --release` building with `--ssr --release` and
   running the server from `target/release/`; without the argument, both
   as at the base; a last argument other than `--release` refused
   (decisions 13 and 14) → the release step (Task 2); mutants 5 and 18 to
   20.
5. **CI's PATH scoping and installs:** the three packages in one `npm
   install --no-save` into `"$RUNNER_TEMP/pw"`, each pinned; binaryen's
   `.bin` on `PATH` in the release step alone, so every earlier step builds
   as `nova` does with no `wasm-opt`; `wasm-opt --version` first; `shrank`
   reading the debug builds of the earlier steps, at their paths; the
   release builds before the rules' test and Lighthouse, so a failure of
   Lighthouse cannot hide theirs, and the `if:`s that run those two after a
   failure of the release builds too, but not after an earlier step's or
   in a cancelled run, Lighthouse only once the rules pass, which nothing
   here evaluates; the rules' test, a step that needs no install; Node
   22's minor, which the job does not pin; Lighthouse's unlocked
   dependencies (decisions 15, 17, 30 and 33) → the job, run from
   `ci.yml`, and actionlint (Task 2); mutants 21 to 24, 44 and 45.
6. **`gate.sh`'s checks on all three systems:** `nova build --target wasm`
   in the new project; the four `dist/` files, non-empty; the magic number
   by `od -A n -t x1 -N 4`, which Git Bash on Windows and macOS's `od`
   both have; `node dist/app.js` where `node` is on `PATH`, which GitHub's
   runners have, and the skip printed where it is not, but under
   `--node`, which fails then; `--test` and `--node` in any order, any
   other refused; CI's three callers passing `--node`, and the Release
   gate passing it when the tag's `gate.sh` takes it, by a `grep` that
   `v0.3.2`'s does not match, for the `--node)` line `gate.sh`'s comment
   marks, each step printing the flags it passes; the header naming each
   caller; `set -euo pipefail` around each (decisions 18 and 31) →
   `gate.sh` with `node`, without it, and without it under `--node`, the
   `grep` on three `gate.sh`s, and `release_manifests.rs`'s test of the
   two steps against `gate.sh`, which fails when the line drifts (Task 3);
   mutants 25 to 28 and 40 to 43.
7. **ADR 0047's inventory and backlog against ADRs 0038 to 0046 and the
   master spec:** each Phase 4 position, each phase-plan sub-phase and
   decision, each 60-EXAMPLES and 30-FRONTEND promise, with its status and
   record; the three deviations it decides; the backlog's items each with
   its ADR; the crate counts (decision 7) → Task 4, Step 2.
8. **The documentation's claims:** the two "Web" sections against `nova
   build --target wasm`, `nova dev`, `--release`, `--ssr`, `--ssg`, `std/ui`
   and its modules, and `std/dom`; the dated notes' figures against the
   replay's (Task 2) and the size tests' (Task 5), each with its
   compressor (decision 25); the Lighthouse scores, said as measured: part
   A's runs all 100, and one of the first replay's 94 at 93, every median
   100 (Checked on); `agent.md`'s and `skill.md`'s rows (decision 21) →
   Task 4, Step 2.
9. **The runbook's changes:** `old=` read from `Cargo.toml`; the 21 lines
   of `Cargo.toml`, 23 of `Cargo.lock`, 1 of `package.json` and 2 of
   `package-lock.json` that the bump changes; the upload's time for one
   new crate; step 6.2 replaced for `v0.4.0` by the user's check (decision
   22) → Task 4, Step 2, and Task 6, which runs them.

## Decisions

Made while writing the spec (its §12), prototyping, splitting and
replaying, each as built and each read against the code. Where one departs
from the spec, it says so and why, and is marked **(differs)**; one that
fills in what the spec left to the prototype says so, unmarked. ADR 0047
records the user's decisions and the deviations it decides.

**The spec's decisions (its §12), as built**

1. **Debug builds are measured** (spec §12.1, §3). `lighthouse.mjs` builds
   06 with `nova build --target wasm` and 07 with `nova build --ssr`, as
   the size tests and the other browser steps build them; the release
   builds are a step of their own (decision 5).
2. **Compression stays out** (spec §12.2). `ui::ssr::Dist` and
   `lighthouse.mjs`'s static server send no compression and no caching
   header, and every page's median was 100 in the prototype and in the
   replay: of the replay's 94 runs that gave a score, 93 scored 100 and
   one, a run of 07's `/`, 93 (Checked on). ADR 0047's backlog keeps
   caching headers in `Dist`.
3. **`std/dom` is documented by a pointer** (spec §12.3, §5). Both "Web"
   sections say `std/dom` is not on std's documentation site and point at
   `std/dom/lib.nova`, the root README by path and `nova-cli`'s by a link
   to it on GitHub; ADR 0047 decides it, and puts `std/dom` on the site on
   the backlog.
4. **binaryen comes from npm, `binaryen@132.0.0`, pinned** (spec §12.4,
   §4), in the job's one `npm install --no-save` into `"$RUNNER_TEMP/pw"`,
   beside `playwright@1.56.1` and `lighthouse@13.5.0`. The release step's
   `wasm-opt --version` prints `wasm-opt version 132 (version_132)`.
5. **CI drives 06 and 07 on their `--release` builds** (spec §12.5, §4):
   the step "Release builds with binaryen's wasm-opt" builds 06 with `nova
   build --target wasm --release` and runs `spa.mjs --release` on it, runs
   `blog.mjs --release`, which builds 07 with `nova build --ssr
   --release`, and checks with `shrank` that each `app.wasm` is smaller
   than the debug build's: 16,525 bytes from 40,491, and 57,179 from
   116,579 (mutants 4, 5 and 18 to 23).
6. **CHANGELOG's `0.4.0` section and ADR 0047's record of the user's check
   come in the release pull request** (spec §12.6, §7): Task 6, Step 3.
7. **`v0.4.0` publishes the 20 crates `publish-crates.sh` lists, one of
   them for the first time.** **(differs)** A correction to the spec: its
   §2 says "20 crates are published … `nova-codegen-wasm` joined them in
   4.0", and its §12, decision 7, "the same 20 crates as v0.3.2;
   `nova-codegen-wasm` already among them". `v0.3.2` published 19 (ADR
   0037's record of 2026-10-14: "the 19 crates (run 38511828598)"); the
   4.0 spec's §8 put `nova-codegen-wasm` into `publish-crates.sh`, which
   now lists 20, `nova-codegen-wasm` 16th, before `nova-driver`; and
   `nova-codegen-wasm`, whose name is free on crates.io (spec §2), first
   reaches crates.io with `v0.4.0`, as the phase plan's 4.8 entry foresaw
   ("If `nova-cli` depends on `nova-codegen-wasm` …, they join the
   published crates, with their names checked on crates.io first"). ADR
   0047 says the same, in its inventory's row for that entry and in "Found
   outside this boundary"; the phase plan's amendment of 2026-10-22 says it
   too; and the runbook's dated note in step 3 gives `v0.4.0` one new crate
   and 19 new versions, within crates.io's burst of 5, so its token keeps
   **publish-new** (Task 6, Step 6).

**The user's decisions on the gate, as built**

8. **"Lighthouse 95+" is a median of 95 or more,** following the user's
   decision 1. The master spec's gate says "pass Lighthouse 95+", the phase
   plan's decision 12 that the job "fails below 95", and `60-EXAMPLES.md`
   §7 and `30-FRONTEND.md` §11 "> 95"; the user's decision 1 is "the
   median of its three Performance scores is 95 or more".
   `lighthouse-rules.mjs` sets `PASS = 95`, and its `judge`, which
   `lighthouse.mjs` applies, fails a page whose median is under it (mutant
   9). ADR 0047 records the reading, for the user's review
   with it, and the notes of `60-EXAMPLES.md` §7 and `30-FRONTEND.md` §7
   name it; every median here was 100, which meets either reading.
9. **The pages, the preset and the runs** (the user's decisions 1 and 2,
   spec §3): 06's `/`, and 07's `/`, `/posts/1` and `/login`; three runs
   each; Lighthouse's CLI with its default mobile preset and simulated
   throttling, `--only-categories=performance`, `--output=json`,
   `--output-path` into the work directory, `--quiet` and
   `--chrome-flags=--headless=new --no-sandbox`, with `CHROME_PATH` set to
   the Chromium Playwright installed. `/admin` needs the admin's session
   and is not measured (ADR 0047).
10. **07 has no size limit, and a KB is 1,024 bytes** (the user's decision
    4). `blog.mjs` prints the client's size and limits nothing; ADR 0047's
    table gives the limits as "30 KiB, 30,720" and "50 KiB, 51,200".

**Decisions the prototype made, read against the code**

11. **`nova-codegen-wasm`'s test helper validates with the MVP and bulk
    memory alone.** **(differs)** Spec §4 changes `bundle.rs` and the
    stand-in test. `valid()` used `wasmparser::Validator::new()`, whose
    default features include sign extension and much more; it now builds
    the validator with `WasmFeatures::MVP | WasmFeatures::BULK_MEMORY`, so
    a backend change that writes a feature `WASM_OPT_FLAGS` does not enable
    fails `cargo test` before it fails a user's `--release` build. It sits
    in `mod tests` and narrows a check the backend already meets: on Task
    1's tests alone, the codegen's 34 tests pass. Mutant 6 takes bulk
    memory out of the check, and `tests::memory_accesses_lower_in_line`
    fails "bulk memory support is not enabled"; mutant 7 writes an
    `i64.extend32_s` after `memory.grow`'s `i64.extend_i32_s`, a no-op of
    the sign-extension feature, and the same test fails "sign extension
    operations support is not enabled", where the base's `valid()` passed
    all 34 with it (Checked on).
12. **`--enable-bulk-memory` alone, after `-Oz`.** Spec §4 left the list to
    the prototype ("the flag list is what the backend emits"). The
    backend's instructions beyond the MVP are `memory.copy` and
    `memory.fill` (`MemOp::Copy`, `MemOp::Fill`); it writes no
    sign-extension or saturating conversion, no global, at most one result,
    one table and one memory. binaryen 132 enables sign extension and
    mutable globals by default, and they stay; `--enable-bulk-memory-opt`,
    which its error names, would also do. `WASM_OPT_FLAGS` is `["-Oz",
    "--enable-bulk-memory"]`, passed before `app.wasm -o app.wasm`; the
    stand-in test pins that order, which binaryen does not need (mutant 3).
    Without the flag binaryen's `wasm-opt` refuses every build, "memory.fill
    operations require bulk memory" (mutant 4).
13. **`spa.mjs` and `blog.mjs` take an optional trailing `--release`.**
    Spec §4 left "whatever option they need" to the prototype. `spa.mjs`
    builds nothing: with `--release` it reads `dist/manifest.json` first,
    fails unless `release` and `wasm_opt` are both `true`, and its line
    ends with the client's size, `app.wasm` and `app.js` gzipped at level
    9 by Node's zlib, as `blog.mjs` measures its own. `blog.mjs --release`
    builds with `nova build --ssr --release`, checks the manifest the same
    way, and runs the server from `target/release/`. Without the argument
    both print what they printed at the base, which part A diffed and the
    replay's lines show; a last argument other than `--release`, or one
    more, is refused with the usage (mutants 5, 18 to 20).
14. **`blog.mjs --release` builds the server for release too, through
    LLVM and the runner's `clang`.** **(differs)** Spec §4 speaks of the
    clients. `nova build --ssr --release` builds both, as 4.6 made it, so
    the step also needs `clang`, which `ubuntu-latest` has, and the blog
    runs on a server built with `--release`; the step's comment says so.
15. **`lighthouse.mjs` refuses Node below 22.19.** **(differs)** Spec §3
    asks the job's Node 22 to be at least 22.19, the floor of Lighthouse
    13's `engines`; `actions/setup-node`'s `node-version: 22` names no
    minor, and CI's logs could not be read from where 4.8 was built. The
    script checks `process.versions.node` before anything else and throws
    "Lighthouse 13 needs Node 22.19 or later, and this is Node <version>"
    (mutant 8); the job's `node-version` stays `22`.
16. **What a failed run is, and the script's lines** (filling in spec §3's
    "a run fails"). A run fails when Lighthouse exits other than 0, is
    still running after 180 seconds (then SIGINT, on which it closes its
    Chromium, and SIGKILL 10 seconds later), leaves no report or one that
    does not parse, or a report with a `runtimeError` or no Performance
    score (`runOf`, decision 29); its output is printed to stderr. Every
    run runs; then the script fails, with status 1, when a page has a run
    that failed or gave no score, runs other than three in number, or a
    median under 95 or that is not a number (`judge`). Each page's line
    gives its scores and their median, the median of FCP, LCP, TBT, CLS
    and SI, and any run warnings, and on every line, a failing page's too,
    the runs' benchmark indexes, Lighthouse's measure of the host's speed,
    by which a CI result can be set beside a local one; the last line
    begins `lighthouse:`, with the failures or, when every page passed, the
    medians and the versions of Lighthouse, Chromium and Node. A failure
    before the verdict, a build, a server's start, the Node floor or a
    missing Lighthouse, ends with `lighthouse: failed, before its verdict:`
    and its reason, and status 1, as spec §3's item 5 asks. The reason is in
    that line, since the second check (it held only the message's first
    line, and a build's reason went to stderr alone): a build's,
    `--set-password`'s or the server's failure gives the first line of its
    stderr that begins `error`, which names the fault where `nova` and cargo
    print one, or else its last line that is not empty, its whole output
    going to stderr; and the line holds the message's lines joined by " / ",
    cut to 200 characters. The last line alone would give clap's `For more
    information, try '--help'.` for a bad flag; the `error` line gives
    `error: unexpected argument '--no-such-flag' found`. The server's exit
    is read on `close`, once its stderr has been read to the end. A child
    that exits before it reads its stdin breaks the pipe: `run` now ignores
    the write's EPIPE, which killed the script with an unhandled error and
    no `lighthouse:` line, 3 runs of 3 in the check's probe and 5 of 5
    here. Both servers are stopped in `finally` (mutants 12 to 17, 29 to 39
    and 46 to 48).
17. **CI's install, PATH and order** (spec §4, as built). One `npm install
    --no-save` of the three packages, then `playwright install --with-deps
    chromium`; after the blog's step, the release builds (`id: release`),
    then "The Lighthouse gate's rules" (`id: rules`, decision 30), then the
    Lighthouse step, so a failure of Lighthouse does not skip the release
    builds and hide their result, as the fact-check found it would. The
    second check found the converse: a failure of the release builds
    skipped both later steps. So the rules' step has `if: success() ||
    (!cancelled() && steps.release.outcome == 'failure')`, and the
    Lighthouse step `if: steps.rules.outcome == 'success' && !cancelled()`.
    By GitHub's documentation, an `if:` with no status function in it is
    read as `success() && (…)`; `success()` is true while no earlier step
    of the job has failed or been cancelled; and `steps.<id>.outcome` is a
    step's result before `continue-on-error`, one of `success`, `failure`,
    `cancelled` and `skipped`, a step skipped after an earlier failure being
    `skipped`. So the rules' test runs when every step before it passed, or
    when the release step itself ran and failed, but not after an earlier
    step's failure, which leaves the release step `skipped`, and not in a
    cancelled run; Lighthouse's condition holds a status function, so no
    `success()` is implied, and it runs when the rules' test ran and
    passed, and the run is not cancelled. Case by case: every step passes,
    both run; a step before the release builds fails, neither does; the
    release builds fail, both run, and the job fails; the rules' test fails,
    Lighthouse does not run, as before; a cancelled run starts neither.
    actionlint checks the ids the conditions name (mutant 44); nothing here
    evaluates the conditions, and nothing checks the steps' order (mutant
    45, not run; Checked on). The rest as built: binaryen's `.bin` on
    `PATH` in the release step alone, so every earlier step builds as
    `nova` does where there is no `wasm-opt`; `wasm-opt --version` first;
    and `shrank` comparing each release `app.wasm` with the debug build at
    the path an earlier step left it, `$RUNNER_TEMP/counter-spa` and
    `$RUNNER_TEMP/blog/fullstack-blog/dist` (mutants 21 to 24). The
    install, run once as one command into scratch while the plan was
    checked, added 117 packages in 10 seconds, 295 MB, with no conflict;
    its `wasm-opt --version` and `lighthouse --version` print 132 and
    13.5.0, and its Playwright 1.56.1 resolves.
18. **`gate.sh` checks the WASM build, and runs it where it can.**
    **(differs)** The user's decision 8 asks `gate.sh` to run `nova build
    --target wasm`. After the native build it also checks that `dist/`
    holds `index.html`, `app.js`, `app.wasm` and `manifest.json`, each
    non-empty, that `app.wasm` begins with `0061736d`, read by `od -A n
    -t x1 -N 4`, and, where `node` is on `PATH`, that `node dist/app.js`
    prints `Hello, Nova!`, what the native build prints; without `node` it
    says so and passes, unless `--node` requires the run (decision 31).
    `nova new`'s template compiles for WASM as it is. Git Bash on Windows
    and macOS have `od`, and GitHub's runners have `node` (mutants 25 to
    28).
19. **`nova build --help` names no flags.** **(differs)** The spec did not
    list the help text. `--release`'s doc said it "runs `wasm-opt -Oz`";
    it now says it "optimizes app.wasm for size with binaryen's `wasm-opt`
    when it is on PATH", so it cannot go stale with the flags again. The
    READMEs and `40-TOOLING.md`'s note name the flags. No test reads the
    help text; Task 1 shows it.
20. **`nova-cli`'s keywords: `wasm` replaces `formatter`.** **(differs)**
    Spec §5 asks the description and keywords to mention the web. The
    description now names "compiler to native code and to WebAssembly for
    the web, dev server"; crates.io takes five keywords, so `wasm` takes
    `formatter`'s place, and the description still names the formatter.
21. **Notes and guides beyond spec §9's list.** **(differs)** Spec §9 lists
    the notes of `00-MASTER-SPEC.md`, `40-TOOLING.md`, `60-EXAMPLES.md` §6
    and §7 and the phase plan, and §5 `agent.md`. Part B also wrote a dated
    note in `30-FRONTEND.md` §7, on the flag and the Lighthouse reading;
    corrected `skill.md`'s WASM rows (no `walrus`, `wasmparser` added, the
    JS shim) and its frontend rows (DOM bindings, SSR and SSG, the bundler,
    live reload in place of HMR and `wasm-bindgen`), which named what Phase
    4 did not build; and added Lighthouse and the release build to both
    examples' READMEs.
22. **The runbook: two lines corrected, two dated notes.** **(differs)**
    Spec §5 corrects `docs/releasing.md`'s three stale lines. `old=` is now
    read from `Cargo.toml` with `perl` and echoed, and the bump's count is
    21 lines of `Cargo.toml` (the version and 20 pins); the third, the
    upload's time for 19 new crates, keeps its text, and a dated note in
    step 3 gives `v0.4.0`'s. A second dated note, at step 6.2, records the
    user's decision 6: for `v0.4.0` the maintainer installs the published
    `nova` and opens both examples with it in place of that run, since its
    step 5 cannot run again (`shapes` 0.1.0 is in the index).
23. **CHANGELOG's Unreleased: four entries under Added.** **(differs)**
    Spec §9 puts the documentation and `gate.sh` under Changed. As built,
    Added holds the Lighthouse step, the CI test of release builds with
    binaryen, `gate.sh`'s WASM build and the "Web" sections, each new;
    Changed holds the help text and the crate's metadata; Fixed holds
    `--release` with a real `wasm-opt`, with both examples' sizes.
24. **ADR 0047's record of the gate holds the prototype's transcript; CI's
    run comes in the release pull request.** **(differs)** Spec §6 puts
    the 4.8 pull request's CI run in the gate's record, and §7 has the
    release pull request add only the user's check. The ADR is written
    before its pull request runs, so it records the `browser` job's steps
    as part A ran them, and says the release pull request adds the 4.8
    pull request's run, with its Lighthouse lines and sizes, beside the
    user's check (Task 6, Step 3).
25. **Each size names its compressor.** **(differs)** A correction to the
    spec's §2, which gives 06 as "20,997 (21,070 with the test's own
    build)": the size test's build is byte for byte the CI step's (the
    replay compared them), and the difference is the compressor. The size
    tests gzip with `flate2` at level 9 (hello world 5,006 bytes, 06
    21,070); zlib at level 9, as `blog.mjs` measures its client and
    `spa.mjs --release` its own, gives 4,959 and 20,997 for the debug
    builds, figures no script prints: part A measured them with a script of
    its own, and the replay gave 20,997 for the size test's own build of
    06. The release figures, 11,974 and 27,510, and 07's 49,472 are the
    scripts' own, by zlib. ADR 0047's table, the notes, the examples'
    READMEs and the CHANGELOG say "by zlib" or "by `flate2`" at each.
26. **ADR 0047 decides three deviations; the blog's JSON store is not
    among them.** **(differs)** Spec §6 lists "the blog's JSON store" with
    the bundler and `std/dom` as deviations the ADR decides. The user
    decided the JSON file on 2026-10-14 (ADR 0038) and ADR 0046 built it,
    so the ADR lists it under "Rows whose record already explains them".
    The third deviation it decides is the reading of "Lighthouse 95+"
    (decision 8). `nova-bundler` stays a stub, by the user's decision 7.

**The split's decisions**

27. **Task 1's tests hold the codegen's helper, which passes before the
    code.** The stand-in test's new expectation and `valid()`'s features
    are tests; the stand-in test fails on the base's `bundle.rs` at
    `wasm_tests.rs:948`, and `valid()` passes there, since it narrows a
    check the backend meets (decision 11). `nova build --help`'s text is
    the doc of `BuildCmd.release` in `crates/nova-cli/src/cmd/run.rs`,
    which clap prints, so it rides in Task 1's code, not the records.
28. **Every other file changes in one task, whole, but `ci.yml`.** Task 2
    is the scripts, the rules and their test, and the `browser` job; Task 3
    `gate.sh` and the three places CI runs it with `--node`, the `install`
    job's step among them, so `ci.yml` changes in Tasks 2 and 3: Task 2's
    patch holds its three hunks in the `browser` job, Task 3's the one in
    the `install` job. Task 3 also holds `release_manifests.rs`'s test of
    the Release gate's `grep` (decision 31): it checks two files of that
    task against each other, so it comes with them, and its failure is
    shown by breaking `gate.sh`'s line there; a commit of the test alone
    before them would fail only for want of the steps it reads. Task 4 is
    the documentation and the records, `nova-cli`'s `Cargo.toml` among
    them, whose metadata no build or test reads but `cargo package`.

**The gap fix's decisions**

The mutants' first run, on `1642f0a`, left five gaps (Checked on); the
prototype's `5f15c33 4.8 prototype: close five test gaps` closed them, and
made the records say what was measured.

29. **`lighthouse.mjs`'s rules live in `lighthouse-rules.mjs`, which it
    imports.** **(differs)** Spec §3 describes one script. The module holds
    `RUNS` and `PASS`, `METRICS`, `pages(spa, blog)`, `median`, `runOf(done,
    lhr, report)`, which turns Lighthouse's exit and report into a run or
    its failure, and `judge(name, runs)`; it holds no state and touches
    nothing but its arguments, so importing it runs nothing and needs no
    Playwright, where exports behind a check of `import.meta.url` against
    `process.argv[1]` would have left the script's top level to guard.
    `judge` now also fails a page with a run that gave no score and one
    whose median is not a number: with the median read past the end, the
    first run's `judge` took `undefined`, which is not under 95, and passed
    every page (mutant 10). Since the fact-check it also fails a page whose
    runs are not `RUNS`, three, in number, "07 /login had 2 runs, not 3":
    with the measuring loop's bound one short, two runs passed, the higher
    their median (mutant 34); and a failing page's line keeps its runs'
    benchmark indexes, `runOf` passing a failed run's on when its report has
    one, "none" when not. A passing page's line is as before, byte for
    byte.
30. **`tests/browser/lighthouse.test.mjs` tests the rules, under `node
    --test`, as a step of the `browser` job before Lighthouse.**
    **(differs)** Spec §8 tests no script's logic. Eight tests, Node's
    `node:test` and `node:assert` alone, no package: the four pages and
    their URLs, three runs and 95; the median of 93, 100 and 99 is 99, in
    any order, the scores left as they are; a page passing on 93, 100 and
    99, and on 95, 100 and 95; failing on a median of 94; failing on a run
    that failed, its line keeping the benchmark indexes, and on a run with
    no score or no runs at all; failing on two runs and on four; and `runOf`
    failing a run on an exit other than 0, a signal, no report, a runtime
    error, no score and a score of another category. The step "The
    Lighthouse gate's rules" runs `node --test
    tests/browser/lighthouse.test.mjs`, in under a second, after the release
    builds (decision 17). Every median here being 100, no run of Lighthouse could tell
    these rules from wrong ones; the test fails under mutants 9 to 12, 15,
    29 to 33, 36 and 37, and under 13 and 14 too, which the step of
    Lighthouse also catches. What the test cannot reach, the measuring
    loop, the status and the line of a failure before the verdict, Task 2's
    stand-in step checks, locally and not in CI: `lighthouse.mjs` run with
    a stub Lighthouse that reports a score of 0.4, which must give four
    failing pages of three runs each, a failing `lighthouse:` line and
    status 1; then with no Lighthouse at all, which must give a
    `lighthouse: failed, before its verdict:` line and status 1; then, since
    the second check, on a `nova` that exits at once, before it reads its
    stdin, printing an `error:` line and a last line after it, which must
    give `lighthouse: failed, before its verdict: nova build --target wasm
    of 06 exited with 2: error: a stand-in nova that fails at once` and
    status 1 (mutants 34, 35, 38, 39 and 46 to 48).
31. **`gate.sh --node` requires the Node run, and CI passes it.**
    **(differs)** The user's decision 8 does not say where the Node run
    must happen. `gate.sh` now takes `--test` and `--node`, in any order,
    after its two arguments, and refuses any other with its usage and
    status 2; with `--node` and no `node` on `PATH` it fails, "gate:
    --node, but there is no node on PATH to run the WASM build". CI's
    `install` job, `package-gate.sh`, which the `package` job and
    `publish-crates.yml` run, and `release.yml`'s smoke test pass `--node`,
    each with a comment: every runner has `node`. An option, not a `grep`
    of the gate's output in each caller, so that the requirement and its
    message live in the script, which the callers name. The Release gate
    (`release-gate.yml`) runs the gate script of the tag it checks, so its
    two steps pass `--node` only when that script takes it, `grep -qF --
    '--node) with_node=--node' .github/scripts/gate.sh`: the new `gate.sh`
    matches, and `v0.3.1`'s and `v0.3.2`'s do not, so a re-run for an older
    tag still works, and `v0.4.0`'s gate requires its own Node run. Both
    steps run under `shell: bash`, Git Bash on Windows. Since the second
    check, which found that a reformatted `--node)` line would drop
    `--node` from `v0.4.0`'s gate in silence, its Node run skipped: each
    step prints the flags it passes, `echo "gate.sh ${flags[*]}"`, so its
    log shows whether `--node` was taken; the `--node)` line carries a
    comment that `release-gate.yml` greps for it; and
    `crates/nova-cli/tests/release_manifests.rs`'s
    `the_release_gate_passes_node_to_a_gate_sh_that_takes_it` reads both
    files: each of the two steps that run `gate.sh` must set
    `flags=(--test)`, grep for a fixed string, print the flags and pass
    `"${flags[@]}"`, in that order, and the string must be the same in both
    and on a line of `gate.sh`. It reads files alone, and fails under that
    reformat (mutant 42), either call without the flags (40, 41) and a step
    without its echo (43). It is the one test 4.8 adds: the full suite
    gives `2420 1 10`, the base's 2419 passed and this one. `gate.sh`'s
    Node run skipped where `node` is on `PATH` fails under `--node` (mutant
    28).

**The fact-check's decisions**

A fact-check read this plan against the prototype at `5f15c33` and found
seven things (Checked on); the prototype's `55e6629 4.8 prototype: the
fact-check's fixes` answered them, and the decisions above say how where
they changed. A second check read it against `55e6629` and found no fault
in the code and eight smaller things (Checked on); `845345c 4.8
prototype: the second check's fixes` answered them, in decisions 16, 17,
30 and 31, and in Tasks 2, 3, 5 and 6. Two more decisions came of the
first:

32. **ADR 0047's last line keeps ADR 0025's wording,** "Tagging `v0.4.0`
    completes Phase 4 within this boundary." **(differs)** Spec §6 gives the
    consequence as "Phase 4 is complete; `v0.4.0` is its release". The
    user's decision 5 chose the `v0.2.0` model, whose ADR 0025 ends the
    same way, and it fits Task 6's order: the ADR merges with the 4.8 pull
    request, the release pull request adds CI's run and the user's check,
    and the tag, the last step on `main`, is what the line names. The ADR's
    Consequences say Phase 4's Definition of Done item by item, the tag the
    sixth.
33. **Two risks are accepted, not closed.** A run stopped by SIGKILL can
    leave its Chromium running, since `chrome-launcher` starts it detached;
    SIGINT, sent first, closes it (the fact-check confirmed it), and CI's
    runner is discarded after the job; locally, `pgrep -af chrome` after a
    stopped run finds any. And Lighthouse's dependencies below its own
    version are not locked: the job's `npm install --no-save` pins the
    three packages and resolves the rest when it runs, as it already did
    for Playwright's; the one real install resolved 117 packages.

---

### Task 0: The baseline

The branch is at this plan's commit, whose parent is `f5ed8b3`, the
approved spec. The plan's commit adds this file alone, so the patches apply
to it as to `f5ed8b3`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-48`.
  - `git diff --stat f5ed8b3 HEAD` lists only
    `docs/superpowers/plans/2026-10-22-phase-4-8-the-gate-and-v0.4.0.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 5's review reads from it.
  - `df -h /` shows at least 8G free; if not, remove
    `target/debug/incremental` and scratch directories of earlier
    sub-phases first.
- [ ] **Step 2: Node, the browser, Lighthouse, binaryen and `clang`.**
  - `node --version` prints `v22.22.0`, at least 22.19, which Lighthouse 13
    needs.
  - `ls -d /opt/node-tools/node_modules/playwright /opt/pw-browsers` lists
    both.
  - `grep '"version"' "$P/s48/lh/node_modules/lighthouse/package.json"
    "$P/s48/binaryen/node_modules/binaryen/package.json"` prints `13.5.0`
    and `132.0.0`: part A's installs, which Tasks 2 and 5 link into their
    npm prefix. Where either is missing, those local runs are skipped:
    say so in the ledger.
  - `command -v wasm-opt` prints nothing: no `wasm-opt` is on `PATH`, so
    the debug steps build as a user without one does.
  - `python3 -c 'import yaml; print(yaml.__version__)'` prints `6.0.1`,
    for `ci_step`.
  - `command -v clang` prints a path, for `blog.mjs --release`'s server and
    4.6's `release_applies_to_the_client_and_the_server`.
- [ ] **Step 3: The suite.** In a shell set up as the conventions say:

  ```sh
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-48.txt" 2>&1
  grep '^test result' "$P/base-48.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-48.txt"
  ```

  Expected: `2419 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. (`f5ed8b3` is `24e7d7f`, `main` after 4.7, and the spec;
  these figures are 4.7's last full run, after its review's fixes, on the
  tree `24e7d7f` holds. The replay did not run the suite at `f5ed8b3`
  again.) Task 5 expects one more, `2420 1 10`: 4.8 adds one test, Task 3's
  `the_release_gate_passes_node_to_a_gate_sh_that_takes_it` (decision 31).
- [ ] **Step 4: The ledger.** Start `$P/ledger-48x.md`, headed with the
  base commit and the baseline's figures.

### Task 1: `--release` and binaryen's `wasm-opt`

Spec §4; the user's decision 3; decisions 11, 12, 19 and 27.

**Files:**
- Modify:
  - `crates/nova-cli/src/bundle.rs` (`WASM_OPT_FLAGS`, `wasm_opt`)
  - `crates/nova-cli/src/cmd/run.rs` (`--release`'s help text)
- Test:
  - `crates/nova-cli/tests/wasm_tests.rs` (the stand-in `wasm-opt` and
    `release_runs_wasm_opt_when_it_is_on_path`)
  - `crates/nova-codegen-wasm/src/lib.rs` (the test helper `valid()`)

- [ ] **Step 1: The tests.** `patch_block t1-tests`. The patch holds:
  - `wasm_tests.rs`: the stand-in `wasm-opt`, a Nova program the test
    builds, now copies its input, the argument before `-o`, to its output,
    the last, whatever flags come first, and still prints its arguments;
    `release_runs_wasm_opt_when_it_is_on_path` expects the line `wasm-opt:
    -Oz --enable-bulk-memory app.wasm -o app.wasm`, and its comments cite
    the 4.8 spec.
  - `nova-codegen-wasm`'s `valid()`, in `mod tests`: the validator takes
    `WasmFeatures::MVP | WasmFeatures::BULK_MEMORY` alone, where
    `Validator::new()` took wasmparser's defaults (decision 11).
- [ ] **Step 2: Run, and see the stand-in test fail.** First the build
  (Conventions), which ends with `Finished` in a few seconds: `nova-cli`
  rebuilds.
  - `W release_runs_wasm_opt_when_it_is_on_path` (about 8 s). Expected:
    `0 passed; 1 failed; 0 ignored; 0 measured; 229 filtered out`: the test
    panics at `crates/nova-cli/tests/wasm_tests.rs:948:5`, printing the
    stand-in's line, `wasm-opt: -Oz app.wasm -o app.wasm`, then `built
    dist`: the base's `bundle.rs` passes `-Oz` alone.
  - `cargo test -p nova-codegen-wasm`. Expected: `34 passed; 0 failed`:
    the backend writes nothing beyond the MVP and bulk memory, so the
    narrower check passes before any code changes (decision 27).
  - `target/debug/nova build --help | grep -A3 -e "--release$"`, which
    also shows `--ssr`'s text, whose line ends with `--release`. Expected,
    for `--release`: the base's text, "… With `--target wasm`, runs `wasm-opt -Oz` on
    app.wasm when `wasm-opt` is on PATH. With `--ssr` or `--ssg`, both
    builds".
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`bundle.rs`:** `const WASM_OPT_FLAGS: [&str; 2] = ["-Oz",
    "--enable-bulk-memory"]`, with a doc that says why each is there and
    what the backend writes, citing the 4.8 spec §4; `wasm_opt` passes
    `.args(WASM_OPT_FLAGS)` and then `["app.wasm", "-o", "app.wasm"]`
    (decision 12).
  - **`cmd/run.rs`:** `--release`'s doc, which clap prints: "With
    `--target wasm`, optimizes app.wasm for size with binaryen's
    `wasm-opt` when it is on PATH" (decision 19).
- [ ] **Step 4: Run.** First the build.
  - `W release_runs_wasm_opt_when_it_is_on_path`. Expected: `1 passed; 0
    failed; 0 ignored; 0 measured; 229 filtered out`.
  - `cargo test -p nova-codegen-wasm`. Expected: `34 passed`.
  - `target/debug/nova build --help | grep -A3 -e "--release$"`. Expected:
    "Optimizing build via the LLVM backend (emits LLVM IR and compiles it
    with a discovered `clang`/`llc`); the default is the fast Cranelift
    backend. With `--target wasm`, optimizes app.wasm for size with
    binaryen's `wasm-opt` when it is on PATH. With `--ssr` or `--ssg`,
    both builds".
  - `NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests` (about
    19 s). Expected: `230 passed; 0 failed; 0 ignored`, as at the base.
- [ ] **Step 5: The checks,** as the conventions list them.
- [ ] **Step 6: Commit** `cli: --release passes --enable-bulk-memory to wasm-opt`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 47306e064ace32e2ac0c5e2343f1038c5048cf08..d97a72b532fc450b77824d6f3f40ff88333da5cd 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -852,8 +852,9 @@ fn a_public_that_leads_outside_is_not_copied() {
         .contains("x.txt"));
 }
 
-/// A stand-in for `wasm-opt`: it copies its input, the argument after
-/// `-Oz`, to its output, the one after `-o`, and prints its arguments.
+/// A stand-in for `wasm-opt`: it prints its arguments, and copies its
+/// input, the argument before `-o`, to its output, the last, after `-o`,
+/// whatever flags come first.
 const WASM_OPT: &str = "fn main() { block_on(go()) }
 
 async fn go() {
@@ -868,8 +869,8 @@ async fn go() {
         i = i + 1
     }
     println(line)
-    match argv.get(2) {
-        Some(input) => match argv.get(4) {
+    match argv.get(argv.len() - 3) {
+        Some(input) => match argv.get(argv.len() - 1) {
             Some(output) => match read(input).await {
                 Ok(bytes) => {
                     let _ = write(output, bytes).await
@@ -883,8 +884,11 @@ async fn go() {
 }
 ";
 
-/// `--release` runs `wasm-opt -Oz` when it is on `PATH`, says so in one
-/// line when it is not, and fails when it fails (spec 4.4 §7, §9.3).
+/// `--release` runs `wasm-opt -Oz --enable-bulk-memory` when it is on
+/// `PATH`, says so in one line when it is not, and fails when it fails
+/// (spec 4.4 §7, §9.3; the flag, the 4.8 spec
+/// `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+/// §4).
 #[test]
 fn release_runs_wasm_opt_when_it_is_on_path() {
     let dir = fresh("release");
@@ -934,14 +938,15 @@ fn release_runs_wasm_opt_when_it_is_on_path() {
         "{manifest}"
     );
 
-    // The stand-in: run with `-Oz` on `app.wasm`, in `dist/`.
+    // The stand-in: run with `-Oz` and bulk memory, which the backend's
+    // `memory.copy` and `memory.fill` need, on `app.wasm`, in `dist/`.
     let a = nova_at(&dir, &home)
         .env("PATH", path_with("bin-ok"))
         .args(build)
         .assert()
         .success();
     assert!(
-        stdout(&a).contains("wasm-opt: -Oz app.wasm -o app.wasm\n"),
+        stdout(&a).contains("wasm-opt: -Oz --enable-bulk-memory app.wasm -o app.wasm\n"),
         "{}",
         stdout(&a)
     );
diff --git a/crates/nova-codegen-wasm/src/lib.rs b/crates/nova-codegen-wasm/src/lib.rs
index 8d640bbfe5e86f80f3a9d49b9870a109bf00f77d..2f53c3b2488dbbcb3f74b5c7d01d975c4aa04e17 100644
--- a/crates/nova-codegen-wasm/src/lib.rs
+++ b/crates/nova-codegen-wasm/src/lib.rs
@@ -1362,10 +1362,16 @@ mod tests {
         }
     }
 
-    /// Compile and validate, returning the module's bytes.
+    /// Compile and validate, returning the module's bytes. The validator
+    /// takes the MVP and bulk memory alone, the one feature `nova build
+    /// --release` enables for `wasm-opt` (`WASM_OPT_FLAGS` in
+    /// `crates/nova-cli/src/bundle.rs`; the 4.8 spec
+    /// `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+    /// §4): a module that needs another fails here, and its flag goes there.
     fn valid(m: &Module) -> Output {
         let out = compile_module(m).expect("compiles");
-        wasmparser::Validator::new()
+        let features = wasmparser::WasmFeatures::MVP | wasmparser::WasmFeatures::BULK_MEMORY;
+        wasmparser::Validator::new_with_features(features)
             .validate_all(&out.wasm)
             .expect("a valid module");
         out
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-cli/src/bundle.rs b/crates/nova-cli/src/bundle.rs
index 1674ca72f407efbd57f68b84dd75586ce9ec053d..1081d8a9b4a4a69690f4d07dd0b2a5baf5275f39 100644
--- a/crates/nova-cli/src/bundle.rs
+++ b/crates/nova-cli/src/bundle.rs
@@ -285,11 +285,29 @@ fn quote(s: &str) -> String {
     out
 }
 
-/// `wasm-opt -Oz app.wasm -o app.wasm` in `out`, when `wasm-opt` is on
-/// `PATH`: whether it ran. One that fails fails the build.
+/// What [`wasm_opt`] passes before its file: `-Oz`, and the flag of each
+/// WASM feature beyond the MVP that `nova-codegen-wasm` writes, since
+/// binaryen refuses an input that uses a feature it has not enabled (the
+/// 4.8 spec
+/// `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+/// §4):
+/// - `--enable-bulk-memory`: the backend writes `memory.copy` and
+///   `memory.fill`.
+///
+/// The backend writes no other such feature: no sign-extension or
+/// saturating float-to-int instruction, no global, at most one result, one
+/// table and one memory; `nova-codegen-wasm`'s tests validate its modules
+/// with the MVP and bulk memory alone. binaryen's own defaults, sign
+/// extension and mutable globals, stay as they are.
+const WASM_OPT_FLAGS: [&str; 2] = ["-Oz", "--enable-bulk-memory"];
+
+/// `wasm-opt -Oz --enable-bulk-memory app.wasm -o app.wasm` in `out`
+/// ([`WASM_OPT_FLAGS`]), when `wasm-opt` is on `PATH`: whether it ran. One
+/// that fails fails the build.
 fn wasm_opt(out: &Path) -> Result<bool> {
     let status = std::process::Command::new("wasm-opt")
-        .args(["-Oz", "app.wasm", "-o", "app.wasm"])
+        .args(WASM_OPT_FLAGS)
+        .args(["app.wasm", "-o", "app.wasm"])
         .current_dir(out)
         .status();
     match status {
diff --git a/crates/nova-cli/src/cmd/run.rs b/crates/nova-cli/src/cmd/run.rs
index d7e0d2f0ce9453e1c8408ad5eb176efe7949f6f3..64f65a6caac4bccb32122025ec6473733411142f 100644
--- a/crates/nova-cli/src/cmd/run.rs
+++ b/crates/nova-cli/src/cmd/run.rs
@@ -75,8 +75,9 @@ pub struct BuildCmd {
 
     /// Optimizing build via the LLVM backend (emits LLVM IR and compiles it
     /// with a discovered `clang`/`llc`); the default is the fast Cranelift
-    /// backend. With `--target wasm`, runs `wasm-opt -Oz` on app.wasm when
-    /// `wasm-opt` is on PATH. With `--ssr` or `--ssg`, both builds.
+    /// backend. With `--target wasm`, optimizes app.wasm for size with
+    /// binaryen's `wasm-opt` when it is on PATH. With `--ssr` or `--ssg`,
+    /// both builds.
     #[arg(long)]
     release: bool,
 
````


### Task 2: Lighthouse, release builds in Chromium, and CI

Spec §3, §4; the user's decisions 1 to 4; decisions 1, 4, 5, 8, 9, 13 to
17, 29 and 30.

**Files:**
- Create: `tests/browser/lighthouse.mjs`, `tests/browser/lighthouse-rules.mjs`
- Test: `tests/browser/lighthouse.test.mjs`
- Modify:
  - `tests/browser/spa.mjs`, `tests/browser/blog.mjs` (`--release`)
  - `.github/workflows/ci.yml` (the `browser` job)

- [ ] **Step 1: The scripts and CI.** `patch_block t2-code`:
  - **`tests/browser/lighthouse.mjs`,** run as `node
    tests/browser/lighthouse.mjs <nova> <work dir> <dir holding
    node_modules/playwright and node_modules/lighthouse>`. It checks Node
    is 22.19 or later (decision 15), that Playwright's Chromium and
    Lighthouse's CLI are where it expects, then empties the work
    directory, copies 06 and 07 into it less what a build or a run there
    may leave (`dist`, `target`, and 07's `data/blog.json`, `users.json`,
    each with its `.tmp`, and `secret`), builds 06 with `nova build
    --target wasm` and 07 with `nova build --ssr`, with `NOVA_HOME` beside
    them, sets 07's admin's password with `--set-password admin
    --iterations 1000`, serves 06's `dist/` on `127.0.0.1:0`, and starts
    07's server with `--address 127.0.0.1:0`. Then three runs of
    Lighthouse's CLI on each of 06's `/` and 07's `/`, `/posts/1` and
    `/login` (decision 9), a line per page, with its benchmark indexes,
    and a `lighthouse:` line; it fails, with status 1, when a run failed, a
    page had other than three runs, or a median is under 95, and a failure
    before the verdict ends with a `lighthouse: failed, before its
    verdict:` line, which gives its reason, a failed build's or server's
    first stderr line that begins `error` or else its last, and status 1
    too; a child that exits before it reads its stdin does not stop it
    (decisions 8 and 16). Both servers are stopped at the end, pass or
    fail. The pages, the run count, the
    threshold, what a failed run is, the median and the verdict are
    `lighthouse-rules.mjs`'s, which it imports.
  - **`tests/browser/lighthouse-rules.mjs`:** `RUNS`, `PASS`, `METRICS`,
    `pages`, `median`, `runOf` and `judge`, with no state and no input but
    their arguments (decision 29).
  - **`tests/browser/lighthouse.test.mjs`:** eight tests of them under
    `node --test`, Node's built-ins alone (decision 30).
  - **`spa.mjs`:** an optional `--release` after its two arguments, which
    checks `dist/manifest.json` for `release` and `wasm_opt` and adds the
    client's gzipped size to its line (decision 13).
  - **`blog.mjs`:** an optional `--release` after its three, which builds
    with `nova build --ssr --release`, checks the manifest the same way,
    runs the server from `target/release/`, and ends its line with that
    (decisions 13 and 14).
  - **CI's `browser` job:** its comment names 4.8's gate; one `npm install
    --no-save` of `playwright@1.56.1 lighthouse@13.5.0 binaryen@132.0.0`,
    the step renamed to say so; after the blog's, the step "Release builds
    with binaryen's wasm-opt", with binaryen's `.bin` on `PATH` for that
    step alone, `wasm-opt --version`, 06's release build and `spa.mjs
    --release`, `blog.mjs --release`, and `shrank` for each `app.wasm`,
    its `id` `release`; then the step "The Lighthouse gate's rules", `node
    --test tests/browser/lighthouse.test.mjs`, its `id` `rules`; then
    "Lighthouse on both examples"; the last two with `if:`s that run them
    after a failure of the release builds too, but not after an earlier
    step's, Lighthouse only once the rules pass (decisions 4, 5, 17 and
    30).
- [ ] **Step 2: The stand-in, here and not in CI** (decision 30). It
  needs Playwright's install at `/opt/node-tools` and no Lighthouse. Build
  first (Conventions), write `$P/lh-stand-in-48.sh` from its block, then:

  ```sh
  export PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers
  lh_stand_in
  ```

  Expected, in about 5 seconds, status 0 and these lines: the first five
  `lighthouse.mjs`'s with the stand-in, three checks, its line with no
  Lighthouse, two checks, its line on a `nova` that exits at once, and two
  checks, `<dir>` being `$P/lh-stand-in-48`:

  ```text
  06 /: scores 40, 40, 40, median 40; medians FCP none, LCP none, TBT none, CLS none, SI none; benchmark index 1000, 1000, 1000
  07 /: scores 40, 40, 40, median 40; medians FCP none, LCP none, TBT none, CLS none, SI none; benchmark index 1000, 1000, 1000
  07 /posts/1: scores 40, 40, 40, median 40; medians FCP none, LCP none, TBT none, CLS none, SI none; benchmark index 1000, 1000, 1000
  07 /login: scores 40, 40, 40, median 40; medians FCP none, LCP none, TBT none, CLS none, SI none; benchmark index 1000, 1000, 1000
  lighthouse: failed, 06's / and 07's /, /posts/1 and /login, 3 runs each, mobile: 06 /'s median is 40, under 95; 07 /'s median is 40, under 95; 07 /posts/1's median is 40, under 95; 07 /login's median is 40, under 95
  stand-in: exit 1
  stand-in: four pages, each scoring 40 in three runs
  stand-in: the failing lighthouse: line
  lighthouse: failed, before its verdict: Lighthouse's CLI is not at <dir>/no-cli/node_modules/lighthouse/cli/index.js
  stand-in: exit 1 with no Lighthouse
  stand-in: a failing lighthouse: line, with its reason
  lighthouse: failed, before its verdict: nova build --target wasm of 06 exited with 2: error: a stand-in nova that fails at once
  stand-in: exit 1 with a nova that fails at once
  stand-in: a failing lighthouse: line, with the build's reason
  ```

  A line `stand-in: NOT …` names a check that failed; then the status is
  1.
- [ ] **Step 3: Run the `browser` job, as `ci.yml` has it,** where Task 0
  found Playwright, Lighthouse and binaryen. As in CI, the runtime library
  beside `nova` comes from the job's own build, with no `NOVA_RUNTIME_LIB`;
  the install step's packages come from the scratch prefix, laid out as
  CI's `"$RUNNER_TEMP/pw"`; and each step's commands are `ci.yml`'s own,
  through `ci_step` (Conventions; write `$P/ci_step-48.py` first).

  ```sh
  rm -f target/debug/libnova_runtime.a
  cargo build --locked -p nova-cli -p nova-runtime
  ls target/debug/libnova_runtime.a
  rm -rf "$P/pw-48" && mkdir -p "$P/pw-48/node_modules/.bin"
  ln -s /opt/node-tools/node_modules/playwright /opt/node-tools/node_modules/playwright-core "$P/s48/lh/node_modules/lighthouse" "$P/s48/binaryen/node_modules/binaryen" "$P/pw-48/node_modules/"
  ln -s ../binaryen/bin/wasm-opt "$P/pw-48/node_modules/.bin/wasm-opt"
  rm -rf "$RUNNER_TEMP" && mkdir -p "$RUNNER_TEMP" && ln -s "$P/pw-48" "$RUNNER_TEMP/pw"
  git status --short --ignored --untracked-files=all examples tests > "$P/status-48.txt"
  ci_step --job
  git status --short --ignored --untracked-files=all examples tests | diff "$P/status-48.txt" -
  ```

  Expected, in about 3 minutes, each step's name first, as `### <name>`,
  and `### every step passed` at the end:
  - the build ends with `Finished`, and `ls` names the library it put back;
  - "Build the counters for WASM": three lines `built <its directory>`;
  - "Click the counters in Chromium": `counter: three clicks, and the count
    reads 3`, twice, and `counter-spa: +, +, +, -, and the count reads 2`,
    as at the base;
  - "nova dev and the router in Chromium": `dev: counts at build
    <start>.1, reloads into .2 on an edit, into .3 after a missed one,
    shows the overlay for the type error of .4 and keeps it over a reload,
    and counts again at .5`, where `<start>` is the server's start in
    milliseconds, such as the replay's `1792695823716`; and `router: a link
    to /posts/2 without a reload, back home, a reload at /posts/2 that
    stays at build <start>.1, /nowhere not found, and a query read`;
  - "SSR and SSG in Chromium": `ssr: / hydrates in place (36 nodes kept +
    1 joined) and counts, a link to /about without a reload, /posts/2 in
    place (26 + 1), /nowhere 404 in place (28 + 1), and SSG's /about/
    arrives rendered and hydrates in place (28 + 1); the client is 41688
    bytes gzipped`;
  - "The blog in Chromium": `blog: every page hydrates in place (nodes
    kept: / 37, /posts/1 48, /login 24, /nowhere 15, /posts/99 15, /admin
    53, /admin/new 29, /admin/posts/2/edit 30), and so does each form's
    answer at the path it was posted to (nodes kept: a comment that fails
    a check, 400 at /posts/1/comments 59; a comment without its token, 403
    at /posts/1/comments 15; a wrong password, 401 at /login 26; an edit
    that fails a check, 400 at /admin/posts/4 31), the comment form's
    button is disabled while a field is blank and the comment posts, a
    comment holding a NUL comes back without it, a script in a comment and
    in a title stays text, a form without its token gets 403, and the
    admin logs in and creates, edits and deletes a post; the client is
    49472 bytes gzipped`, as at the base;
  - "Release builds with binaryen's wasm-opt": `wasm-opt version 132
    (version_132)`; `built <RUNNER_TEMP>/counter-spa-release`;
    `counter-spa: +, +, +, -, and the count reads 2; the client is 11974
    bytes gzipped, from a release build that wasm-opt optimized`; the
    blog's line as above, ending `the client is 27510 bytes gzipped, from
    a release build that wasm-opt optimized`; then
    `<RUNNER_TEMP>/counter-spa-release/app.wasm: 16525 bytes, from 40491
    in the debug build` and
    `<RUNNER_TEMP>/blog-release/fullstack-blog/dist/app.wasm: 57179 bytes,
    from 116579 in the debug build`;
  - "The Lighthouse gate's rules", in under a second: Node's TAP report,
    `ok 1 - four pages, 06's / and 07's /, /posts/1 and /login, three runs
    each, 95 to pass` through `ok 8 - a run fails when Lighthouse exits
    other than 0, leaves no report, or reports an error or no score`, then
    `# tests 8`, `# pass 8` and `# fail 0`;
  - "Lighthouse on both examples", in about 2 minutes, the replay's lines:

    ```text
    06 /: scores 100, 100, 100, median 100; medians FCP 680 ms, LCP 1054 ms, TBT 0 ms, CLS 0.000, SI 680 ms; benchmark index 2425, 2414, 2384
    07 /: scores 100, 100, 100, median 100; medians FCP 901 ms, LCP 1651 ms, TBT 0 ms, CLS 0.000, SI 901 ms; benchmark index 2544, 2440, 2496
    07 /posts/1: scores 100, 100, 100, median 100; medians FCP 901 ms, LCP 1652 ms, TBT 4 ms, CLS 0.000, SI 901 ms; benchmark index 2656, 2373, 2593
    07 /login: scores 100, 100, 100, median 100; medians FCP 902 ms, LCP 1652 ms, TBT 0 ms, CLS 0.001, SI 902 ms; benchmark index 2424, 2450, 2582
    lighthouse: 06's / and 07's /, /posts/1 and /login, 3 runs each, mobile, every median 95 or more (06 / 100, 07 / 100, 07 /posts/1 100, 07 /login 100); Lighthouse 13.5.0, HeadlessChrome/141.0.0.0, Node 22.22.0
    ```

    The metrics and the benchmark indexes vary from run to run, and so,
    now and then, does a score: every score was 100 here and in the
    prototype's run, but of the first replay's 94 runs one, of 07's `/`,
    scored 93, in the second replay's job one of `/posts/1` scored 99, and
    in the fourth replay's mutants one of 06's `/` scored 94; every median
    was 100. Record the lines, with their benchmark indexes,
    in the ledger: Task 5's CI run is set beside them;
  - `diff` prints nothing: the builds write outside the tree, and the
    scripts work in copies.
- [ ] **Step 4: The checks,** as the conventions list them, actionlint
  among them: it checks the ids `release` and `rules` that the two `if:`s
  name (mutant 44). Nothing here runs the `if:`s themselves: `ci_step`
  runs every step whatever its condition (decision 17).
- [ ] **Step 5: Commit** `ci: Lighthouse on both examples, and release builds with binaryen`.

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 2b865c5cc16d2c4bfdf19327ccf247cb01d642c3..91ff91e445820dd1a1af192b650b941600ef308d 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -116,9 +116,22 @@ jobs:
   # comment form's checks and post work, that the admin creates, edits and
   # deletes a post, that a script in a title or a comment stays text, and
   # that a form without its CSRF token is refused; it kills the server at
-  # the end. Playwright, pinned, is installed here only,
-  # into a scratch directory; nothing in `cargo test` or `nova` installs a
-  # package. Ubuntu only: one browser job suffices.
+  # the end. And 4.8's, Phase 4's gate measured (spec
+  # docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
+  # §3 and §4): 06 and 07 are built with `--release` and binaryen's
+  # `wasm-opt`, which must run and shrink both clients, and `spa.mjs` and
+  # `blog.mjs` drive those builds; `lighthouse.test.mjs` tests the rules
+  # `lighthouse.mjs` applies, under `node --test`, without Lighthouse; and
+  # `lighthouse.mjs` builds 06 and 07 in `$RUNNER_TEMP`, serves both on
+  # port 0, runs Lighthouse three times, mobile, on 06's `/` and 07's `/`,
+  # `/posts/1` and `/login`, fails when a run fails or a page's median
+  # Performance score is under 95, and stops both servers at the end. The
+  # release builds come before Lighthouse, so that a Lighthouse failure
+  # does not hide their result, and the rules' test and Lighthouse run
+  # after a failure of the release builds too, so that it does not hide
+  # theirs. Playwright, Lighthouse and binaryen, each pinned, are installed
+  # here only, into a scratch directory; nothing in `cargo test` or `nova`
+  # installs a package. Ubuntu only: one browser job suffices.
   browser:
     name: Browser (Chromium)
     runs-on: ubuntu-latest
@@ -141,9 +154,9 @@ jobs:
           target/debug/nova build --target wasm tests/dom/counter.nova -o "$RUNNER_TEMP/counter"
           target/debug/nova build --target wasm tests/dom/ui_counter.nova -o "$RUNNER_TEMP/ui_counter"
           (cd examples/06-counter-spa && ../../target/debug/nova build --target wasm -o "$RUNNER_TEMP/counter-spa")
-      - name: Install Playwright and Chromium
+      - name: Install Playwright, Chromium, Lighthouse and binaryen
         run: |
-          npm install --no-save --prefix "$RUNNER_TEMP/pw" playwright@1.56.1
+          npm install --no-save --prefix "$RUNNER_TEMP/pw" playwright@1.56.1 lighthouse@13.5.0 binaryen@132.0.0
           "$RUNNER_TEMP/pw/node_modules/.bin/playwright" install --with-deps chromium
       - name: Click the counters in Chromium
         run: |
@@ -158,6 +171,40 @@ jobs:
         run: node tests/browser/ssr.mjs target/debug/nova "$RUNNER_TEMP/ssr" "$RUNNER_TEMP/pw"
       - name: The blog in Chromium
         run: node tests/browser/blog.mjs target/debug/nova "$RUNNER_TEMP/blog" "$RUNNER_TEMP/pw"
+      # binaryen's `wasm-opt` is on PATH in this step alone, so the other
+      # steps build as `nova` does where there is none. `blog.mjs --release`
+      # builds the server with `--release` too, through the runner's
+      # `clang`. `shrank` compares each release `app.wasm` with the debug
+      # build's the steps above drove.
+      - name: Release builds with binaryen's wasm-opt
+        id: release
+        run: |
+          export PATH="$RUNNER_TEMP/pw/node_modules/.bin:$PATH"
+          wasm-opt --version
+          (cd examples/06-counter-spa && ../../target/debug/nova build --target wasm --release -o "$RUNNER_TEMP/counter-spa-release")
+          node tests/browser/spa.mjs "$RUNNER_TEMP/counter-spa-release" "$RUNNER_TEMP/pw" --release
+          node tests/browser/blog.mjs target/debug/nova "$RUNNER_TEMP/blog-release" "$RUNNER_TEMP/pw" --release
+          shrank() {
+            debug=$(stat -c %s "$1/app.wasm")
+            release=$(stat -c %s "$2/app.wasm")
+            echo "$2/app.wasm: $release bytes, from $debug in the debug build"
+            test "$release" -lt "$debug"
+          }
+          shrank "$RUNNER_TEMP/counter-spa" "$RUNNER_TEMP/counter-spa-release"
+          shrank "$RUNNER_TEMP/blog/fullstack-blog/dist" "$RUNNER_TEMP/blog-release/fullstack-blog/dist"
+      # The rules' test, and Lighthouse once the rules pass, run after a
+      # failure of the release builds too, but not after an earlier step's:
+      # `success()` is true while no step has failed, and the release step's
+      # outcome is `failure` only when it ran and failed (a step skipped
+      # after an earlier failure is `skipped`). `!cancelled()` keeps a
+      # cancelled run from starting either.
+      - name: The Lighthouse gate's rules
+        id: rules
+        if: success() || (!cancelled() && steps.release.outcome == 'failure')
+        run: node --test tests/browser/lighthouse.test.mjs
+      - name: Lighthouse on both examples
+        if: steps.rules.outcome == 'success' && !cancelled()
+        run: node tests/browser/lighthouse.mjs target/debug/nova "$RUNNER_TEMP/lighthouse" "$RUNNER_TEMP/pw"
 
   fmt:
     name: Rustfmt
diff --git a/tests/browser/blog.mjs b/tests/browser/blog.mjs
index cae3715df79ea6d9b233a55d8ac0142fd0b51fea..e31e1b38d67f9588e1397f365376006d9b5d3fa0 100644
--- a/tests/browser/blog.mjs
+++ b/tests/browser/blog.mjs
@@ -59,6 +59,14 @@
 // `app.wasm` and `app.js` gzipped, printed and not limited. The server is
 // killed at the end, pass or fail. Every wait polls, for 30 seconds at
 // most. Playwright is the job's own install, as for `counter.mjs`.
+//
+// With `--release` after the three, as CI's step of release builds runs it
+// with binaryen's `wasm-opt` on `PATH` (spec
+// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
+// §4), it builds with `nova build --ssr --release`, checks that
+// `dist/manifest.json` says the client is a release build that `wasm-opt`
+// optimized, runs the server from `target/release/`, and its line ends
+// with that after the client's size. Without it, nothing changes.
 
 import { spawn } from "node:child_process";
 import { cp, mkdir, readFile, rm } from "node:fs/promises";
@@ -68,7 +76,11 @@ import { createInterface } from "node:readline";
 import { fileURLToPath } from "node:url";
 import { gzipSync } from "node:zlib";
 
-const [nova, work, modules] = process.argv.slice(2).map((p) => resolve(p));
+const [nova, work, modules] = process.argv.slice(2, 5).map((p) => resolve(p));
+const release = process.argv[5] === "--release";
+if (process.argv.length > 6 || (process.argv.length === 6 && !release)) {
+  throw new Error("usage: node tests/browser/blog.mjs <nova> <work dir> <dir holding node_modules/playwright> [--release]");
+}
 const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
 const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
 const LIMIT = 30000;
@@ -79,6 +91,8 @@ const XSS = `<script>window.novaXss = 1; alert("xss")</script><img src=x onerror
 // The same, short enough for a comment's name, of 60 characters at most.
 const NAME_XSS = `<img src=x onerror="window.novaXss=3;alert(3)">`;
 const EXE = process.platform === "win32" ? ".exe" : "";
+// Where `nova build --ssr` puts the server.
+const PROFILE = release ? "release" : "debug";
 // The character a browser's parser drops from a page's text.
 const NUL = String.fromCharCode(0);
 
@@ -103,7 +117,7 @@ function run(command, args, dir, env, input = null) {
 
 // The server in `dir`: its URL, its stderr so far, and a kill.
 async function startServer(dir, env) {
-  const exe = join(dir, "target", "debug", `fullstack-blog-server${EXE}`);
+  const exe = join(dir, "target", PROFILE, `fullstack-blog-server${EXE}`);
   const child = spawn(exe, ["--address", "127.0.0.1:0"], { cwd: dir, env, stdio: ["ignore", "pipe", "pipe"] });
   let stderr = "";
   child.stderr.on("data", (chunk) => {
@@ -258,9 +272,16 @@ await cp(example, dir, {
   filter: (from) => !left.some((l) => relative(example, from) === l || relative(example, from).startsWith(l + sep)),
 });
 
-const build = await run(nova, ["build", "--ssr"], dir, env);
-if (build.code !== 0) throw new Error(`nova build --ssr exited with ${build.code}:\n${build.output}`);
-const exe = join(dir, "target", "debug", `fullstack-blog-server${EXE}`);
+const flags = release ? ["--ssr", "--release"] : ["--ssr"];
+const build = await run(nova, ["build", ...flags], dir, env);
+if (build.code !== 0) throw new Error(`nova build ${flags.join(" ")} exited with ${build.code}:\n${build.output}`);
+if (release) {
+  const manifest = JSON.parse(await readFile(join(dir, "dist", "manifest.json"), "utf8"));
+  if (manifest.release !== true || manifest.wasm_opt !== true) {
+    throw new Error(`dist/manifest.json says release ${manifest.release} and wasm_opt ${manifest.wasm_opt}, not a release build that wasm-opt optimized`);
+  }
+}
+const exe = join(dir, "target", PROFILE, `fullstack-blog-server${EXE}`);
 const set = await run(exe, ["--set-password", "admin", "--iterations", "1000"], dir, env, `${PASSWORD}\n`);
 if (set.code !== 0 || set.output.trim() !== "blog: set the password of admin") {
   throw new Error(`--set-password exited with ${set.code}:\n${set.output}`);
@@ -499,7 +520,7 @@ try {
   }
 
   console.log(
-    `blog: every page hydrates in place (nodes kept: ${kept.join(", ")}), and so does each form's answer at the path it was posted to (nodes kept: ${answers.join("; ")}), the comment form's button is disabled while a field is blank and the comment posts, a comment holding a NUL comes back without it, a script in a comment and in a title stays text, a form without its token gets 403, and the admin logs in and creates, edits and deletes a post; the client is ${gzipped} bytes gzipped`,
+    `blog: every page hydrates in place (nodes kept: ${kept.join(", ")}), and so does each form's answer at the path it was posted to (nodes kept: ${answers.join("; ")}), the comment form's button is disabled while a field is blank and the comment posts, a comment holding a NUL comes back without it, a script in a comment and in a title stays text, a form without its token gets 403, and the admin logs in and creates, edits and deletes a post; the client is ${gzipped} bytes gzipped${release ? ", from a release build that wasm-opt optimized" : ""}`,
   );
 } catch (e) {
   console.error(`the server's stderr:\n${server.stderr()}`);
diff --git a/tests/browser/lighthouse-rules.mjs b/tests/browser/lighthouse-rules.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..5f50f0263bb4bf60df1c4df656036e833c14e5f5
--- /dev/null
+++ b/tests/browser/lighthouse-rules.mjs
@@ -0,0 +1,102 @@
+// The rules of the Phase 4 gate's measurement (spec
+// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
+// §3, the user's decisions 1 and 2): the pages Lighthouse measures, how
+// many runs each gets, what a run that failed is, the median, and the
+// verdict over a page's runs. `lighthouse.mjs` applies them to Lighthouse's
+// runs, and `lighthouse.test.mjs` tests them without Lighthouse or a
+// browser, under `node --test`, as CI's `browser` job runs it before the
+// measurement. They hold no state and touch nothing outside their
+// arguments.
+
+// The runs each page gets, and the median a page needs to pass.
+export const RUNS = 3;
+export const PASS = 95;
+
+// The main metrics of a report, by their audits, each with its unit.
+export const METRICS = [
+  ["FCP", "first-contentful-paint", "ms"],
+  ["LCP", "largest-contentful-paint", "ms"],
+  ["TBT", "total-blocking-time", "ms"],
+  ["CLS", "cumulative-layout-shift", ""],
+  ["SI", "speed-index", "ms"],
+];
+
+// The pages Lighthouse measures, each `[name, label, url]`: 06's `/`, on
+// the static server at `spa`, and 07's `/`, `/posts/1` and `/login`, on its
+// own server at `blog`. `label` names the page's reports.
+export function pages(spa, blog) {
+  return [
+    ["06 /", "06", new URL("/", spa).href],
+    ["07 /", "07-home", new URL("/", blog).href],
+    ["07 /posts/1", "07-post", new URL("/posts/1", blog).href],
+    ["07 /login", "07-login", new URL("/login", blog).href],
+  ];
+}
+
+// The middle value once sorted, which for three runs is their median; the
+// values themselves are left as they are.
+export const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];
+
+// One run, from Lighthouse's exit (`code`, or the `signal` that stopped it)
+// and its report at `report`, parsed, or null when there was none or it did
+// not parse: `{ failure, bench }` when the run failed, with the report's
+// benchmark index if it gave one, or its score out of 100, its metrics, its
+// benchmark index, its warnings and the versions it ran on. A run fails
+// when Lighthouse exits other than 0, leaves no report, or a report with a
+// runtime error or no Performance score.
+export function runOf(done, lhr, report) {
+  const score = lhr?.categories?.performance?.score;
+  let failure = null;
+  if (done.code !== 0) {
+    failure = `Lighthouse exited with ${done.code ?? done.signal}`;
+  } else if (lhr === null) {
+    failure = `no report at ${report}`;
+  } else if (lhr.runtimeError) {
+    failure = `${lhr.runtimeError.code}: ${lhr.runtimeError.message}`;
+  } else if (typeof score !== "number") {
+    failure = "no Performance score";
+  }
+  if (failure !== null) return { failure, bench: lhr?.environment?.benchmarkIndex };
+  return {
+    failure,
+    score: Math.round(score * 100),
+    metrics: METRICS.map(([, id]) => lhr.audits?.[id]?.numericValue),
+    bench: lhr.environment?.benchmarkIndex,
+    warnings: lhr.runWarnings ?? [],
+    version: lhr.lighthouseVersion,
+    browser: /HeadlessChrome\/[\d.]+|Chrome\/[\d.]+/.exec(lhr.environment?.hostUserAgent ?? "")?.[0],
+  };
+}
+
+// A page's verdict over its runs: its line, its failures, and its median
+// (null when a run failed). A run that failed, or that gave no score,
+// fails the page; so do runs other than `RUNS` in number, a median under
+// `PASS`, and one that is not a number. The line gives the runs' scores
+// and, when every run gave one, their median and the median of each
+// metric; then the runs' benchmark indexes, Lighthouse's measure of the
+// host's speed, which tell a slow runner from a slow page; then any
+// warnings.
+export function judge(name, runs) {
+  const failures = runs.flatMap((r, i) => {
+    if (r.failure != null) return [`${name}'s run ${i + 1} failed (${r.failure})`];
+    if (typeof r.score !== "number") return [`${name}'s run ${i + 1} failed (no Performance score)`];
+    return [];
+  });
+  const scored = (r) => r.failure == null && typeof r.score === "number";
+  let line = `${name}: scores ${runs.map((r) => (scored(r) ? r.score : "failed")).join(", ")}`;
+  const bench = runs.map((r) => (typeof r.bench === "number" ? Math.round(r.bench) : "none")).join(", ");
+  if (failures.length > 0) return { line: `${line}; benchmark index ${bench}`, failures, score: null };
+  const score = median(runs.map((r) => r.score));
+  if (runs.length !== RUNS) failures.push(`${name} had ${runs.length} runs, not ${RUNS}`);
+  if (typeof score !== "number") failures.push(`${name} has no median`);
+  if (score < PASS) failures.push(`${name}'s median is ${score}, under ${PASS}`);
+  const metrics = METRICS.map(([short, , unit], i) => {
+    const value = median(runs.map((r) => r.metrics?.[i]));
+    if (typeof value !== "number") return `${short} none`;
+    return unit === "" ? `${short} ${value.toFixed(3)}` : `${short} ${Math.round(value)} ${unit}`;
+  });
+  line += `, median ${score}; medians ${metrics.join(", ")}; benchmark index ${bench}`;
+  const warnings = [...new Set(runs.flatMap((r) => r.warnings ?? []))];
+  if (warnings.length > 0) line += `; warnings: ${warnings.join(" | ")}`;
+  return { line, failures, score };
+}
diff --git a/tests/browser/lighthouse.mjs b/tests/browser/lighthouse.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..35042cb96dd69c24b2d016dac755fc813a41ce84
--- /dev/null
+++ b/tests/browser/lighthouse.mjs
@@ -0,0 +1,322 @@
+// The Phase 4 gate's measurement (spec
+// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
+// §3, the user's decisions 1 and 2), run by CI's `browser` job last, after
+// the release builds and the rules' test:
+//
+//     node tests/browser/lighthouse.mjs <nova> <work dir> <dir holding node_modules/playwright and node_modules/lighthouse>
+//
+// It copies `examples/06-counter-spa` and `examples/07-fullstack-blog` into
+// the work directory, emptied first, leaving out what a build or a run there
+// may have left, as `blog.mjs` does, and builds each there with `NOVA_HOME`
+// beside them. 06 is built with `nova build --target wasm`, a debug build,
+// as the size tests and `spa.mjs` measure it, and its `dist/` is served by a
+// static server on `127.0.0.1:0`, as `counter.mjs` serves one. 07 is built
+// with `nova build --ssr`; the admin's password is set through
+// `fullstack-blog-server --set-password admin --iterations 1000`, and the
+// server is started on `--address 127.0.0.1:0`, its port read from the line
+// `blog: serving http://127.0.0.1:<port>/`, as `blog.mjs` does.
+//
+// Lighthouse's CLI, from the npm directory's `lighthouse` (pinned by the
+// job to 13.5.0), then runs three times on each of 06's `/` and 07's `/`,
+// `/posts/1` and `/login`: the mobile preset, its default, with its
+// simulated throttling; the Performance category alone; a JSON report
+// written into the work directory; and Chromium headless without its
+// sandbox, the Chromium Playwright installed, named by `CHROME_PATH`. Each
+// page's line gives its three scores and their median, the median of each
+// main metric, and the runs' benchmark indexes, also when a run failed. A
+// run fails when Lighthouse exits other than 0, is stopped after three
+// minutes, or leaves no report, or a report with a runtime error or no
+// score. Every run runs; then the script fails when a run failed, a page
+// had other than three runs, or a page's median is under 95. Its last line
+// begins `lighthouse:` and says which, and, when every page passed, the
+// versions of Lighthouse, Chromium and Node; a failure before the verdict,
+// such as a build that fails, gets a `lighthouse: failed` line too, with
+// its reason: for a build or a server, the first line of its stderr that
+// begins `error`, or else its last, cut to 200 characters, its whole
+// output going to stderr. Lighthouse 13 needs Node 22.19 or later, which
+// the script checks first. Both servers are stopped at the end, pass or
+// fail. Lighthouse and Playwright are the job's own install: nothing in
+// `cargo test` or `nova` installs a package.
+//
+// The pages, the number of runs, what a failed run is, the median and the
+// verdict are `lighthouse-rules.mjs`'s, which `lighthouse.test.mjs` tests
+// without Lighthouse; this script runs Lighthouse and applies them.
+
+import { spawn } from "node:child_process";
+import { existsSync } from "node:fs";
+import { createServer } from "node:http";
+import { cp, mkdir, readFile, rm } from "node:fs/promises";
+import { createRequire } from "node:module";
+import { dirname, extname, join, relative, resolve, sep } from "node:path";
+import { createInterface } from "node:readline";
+import { fileURLToPath } from "node:url";
+
+import { PASS, RUNS, judge, pages, runOf } from "./lighthouse-rules.mjs";
+
+const [nova, work, modules] = process.argv.slice(2).map((p) => resolve(p));
+const lighthouse = join(modules, "node_modules", "lighthouse", "cli", "index.js");
+const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
+const LIMIT = 30000;
+// A run takes about 11 s on a developer's machine; one still going after
+// this long is stopped, and fails.
+const RUN_LIMIT = 180000;
+const PASSWORD = "correct horse battery";
+const EXE = process.platform === "win32" ? ".exe" : "";
+
+const types = {
+  ".html": "text/html",
+  ".js": "text/javascript",
+  ".json": "application/json",
+  ".wasm": "application/wasm",
+  ".css": "text/css",
+};
+
+// Runs `command` in `dir` to its end, `input` on its stdin: its status, its
+// output and its stderr alone, once its stdout and stderr have closed. A
+// child that exits before it reads its stdin breaks the pipe; the write's
+// EPIPE is ignored, since the status says how the run went.
+function run(command, args, dir, env, input = null) {
+  return new Promise((done, failed) => {
+    const child = spawn(command, args, { cwd: dir, env, stdio: ["pipe", "pipe", "pipe"] });
+    let output = "";
+    let stderr = "";
+    child.stdout.on("data", (chunk) => {
+      output += chunk;
+    });
+    child.stderr.on("data", (chunk) => {
+      output += chunk;
+      stderr += chunk;
+    });
+    child.on("error", failed);
+    child.on("close", (code) => done({ code, output, stderr }));
+    child.stdin.on("error", () => {});
+    child.stdin.end(input ?? "");
+  });
+}
+
+// The reason a build or a server failed, from its stderr (or its output,
+// when its stderr is empty): the first line that begins `error`, which
+// names the fault where `nova` and cargo print one, or else the last line
+// that is not empty.
+function reason(stderr, output) {
+  const lines = (stderr.trim() === "" ? output : stderr)
+    .split("\n")
+    .map((line) => line.trim())
+    .filter((line) => line !== "");
+  return lines.find((line) => /^error/i.test(line)) ?? lines.at(-1) ?? "no output";
+}
+
+// A step that failed: its whole output to stderr, and an error naming it
+// and its reason, for the `lighthouse:` line.
+function failure(step, { output, stderr }) {
+  console.error(`${step}:\n${output}`);
+  return new Error(`${step}: ${reason(stderr, output)}`);
+}
+
+// `text` as one line of at most 200 characters, its lines joined by " / ".
+function oneLine(text) {
+  const line = String(text)
+    .split("\n")
+    .map((part) => part.trim())
+    .filter((part) => part !== "")
+    .join(" / ");
+  return line.length > 200 ? `${line.slice(0, 197)}...` : line;
+}
+
+// `example` copied to `dir`, without the paths in `left`.
+async function copy(example, dir, left) {
+  const from = join(repo, "examples", example);
+  await cp(from, dir, {
+    recursive: true,
+    filter: (path) => !left.some((l) => relative(from, path) === l || relative(from, path).startsWith(l + sep)),
+  });
+}
+
+// `dist/`'s files, and nothing outside it, on 127.0.0.1:0: the server and
+// its URL.
+async function serve(dist) {
+  const server = createServer(async (req, res) => {
+    const path = decodeURIComponent(new URL(req.url, "http://127.0.0.1").pathname);
+    const file = resolve(join(dist, path === "/" ? "index.html" : path));
+    if (file !== dist && !file.startsWith(dist + sep)) {
+      res.writeHead(403).end();
+      return;
+    }
+    try {
+      const body = await readFile(file);
+      res.writeHead(200, { "content-type": types[extname(file)] ?? "application/octet-stream" });
+      res.end(body);
+    } catch {
+      res.writeHead(404).end();
+    }
+  });
+  await new Promise((ready) => server.listen(0, "127.0.0.1", ready));
+  return { server, url: `http://127.0.0.1:${server.address().port}/` };
+}
+
+// The blog's server in `dir`: its URL, its stderr so far, and a kill.
+async function startServer(dir, env) {
+  const exe = join(dir, "target", "debug", `fullstack-blog-server${EXE}`);
+  const child = spawn(exe, ["--address", "127.0.0.1:0"], { cwd: dir, env, stdio: ["ignore", "pipe", "pipe"] });
+  let stderr = "";
+  child.stderr.on("data", (chunk) => {
+    stderr += chunk;
+  });
+  const server = { url: null, stderr: () => stderr, kill: () => child.kill() };
+  try {
+    server.url = await new Promise((found, failed) => {
+      const timer = setTimeout(() => failed(new Error("the blog's server gave no URL within 30 s")), LIMIT);
+      createInterface({ input: child.stdout }).on("line", (line) => {
+        const url = /^blog: serving (http:\/\/127\.0\.0\.1:\d+\/)$/.exec(line)?.[1];
+        if (url) {
+          clearTimeout(timer);
+          found(url);
+        }
+      });
+      // "close", not "exit": its stderr has been read to the end by then.
+      child.on("close", (code) => failed(new Error(`the blog's server exited with ${code}`)));
+    });
+  } catch (e) {
+    child.kill();
+    throw failure(e.message, { output: stderr, stderr });
+  }
+  return server;
+}
+
+// One run of Lighthouse's CLI on `url`, its report written to `report`:
+// its exit status, or the signal that stopped it, and its output. One that
+// runs past `RUN_LIMIT` gets SIGINT, on which Lighthouse closes its
+// Chromium, and then SIGKILL.
+function lighthouseRun(url, report, chrome) {
+  const args = [
+    lighthouse,
+    url,
+    "--only-categories=performance",
+    "--output=json",
+    `--output-path=${report}`,
+    "--quiet",
+    "--chrome-flags=--headless=new --no-sandbox",
+  ];
+  return new Promise((done, failed) => {
+    const child = spawn(process.execPath, args, {
+      env: { ...process.env, CHROME_PATH: chrome },
+      stdio: ["ignore", "pipe", "pipe"],
+    });
+    let output = "";
+    child.stdout.on("data", (chunk) => {
+      output += chunk;
+    });
+    child.stderr.on("data", (chunk) => {
+      output += chunk;
+    });
+    let kill = null;
+    const timer = setTimeout(() => {
+      output += `\nstopped after ${RUN_LIMIT / 1000} s`;
+      child.kill("SIGINT");
+      kill = setTimeout(() => child.kill("SIGKILL"), 10000);
+    }, RUN_LIMIT);
+    child.on("error", failed);
+    child.on("close", (code, signal) => {
+      clearTimeout(timer);
+      clearTimeout(kill);
+      done({ code, signal, output });
+    });
+  });
+}
+
+// `RUNS` runs on `url`, each report at `<reports>/<label>-<n>.json`: one
+// entry per run, `runOf`'s, with its score out of 100, its metrics, its
+// benchmark index and its warnings, or why it failed.
+async function measure(label, url, chrome, reports) {
+  const runs = [];
+  for (let n = 1; n <= RUNS; n++) {
+    const report = join(reports, `${label}-${n}.json`);
+    const done = await lighthouseRun(url, report, chrome);
+    let lhr = null;
+    try {
+      lhr = JSON.parse(await readFile(report, "utf8"));
+    } catch {
+      // No report, or not one that parses: the run failed.
+    }
+    const run = runOf(done, lhr, report);
+    if (run.failure !== null) {
+      console.error(`${label}, run ${n}: ${run.failure}; Lighthouse's output:\n${done.output}`);
+    }
+    runs.push(run);
+  }
+  return runs;
+}
+
+// Every failure, a thrown one too (a build, a server that does not start,
+// Node below the floor), ends with one `lighthouse:` line that says why,
+// and a status of 1; both servers are stopped whatever happens.
+let statics = null;
+let server = null;
+try {
+  // Lighthouse 13 needs Node 22.19 or later, by its `engines`.
+  const [major, minor] = process.versions.node.split(".").map(Number);
+  if (major < 22 || (major === 22 && minor < 19)) {
+    throw new Error(`Lighthouse 13 needs Node 22.19 or later, and this is Node ${process.versions.node}`);
+  }
+  const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
+  const chrome = chromium.executablePath();
+  if (!existsSync(chrome)) throw new Error(`Playwright's Chromium is not at ${chrome}`);
+  if (!existsSync(lighthouse)) throw new Error(`Lighthouse's CLI is not at ${lighthouse}`);
+
+  const env = { ...process.env, NOVA_HOME: join(work, "nova-home"), NO_COLOR: "1" };
+  const spa = join(work, "counter-spa");
+  const blog = join(work, "fullstack-blog");
+  const reports = join(work, "reports");
+  await rm(work, { recursive: true, force: true });
+  await mkdir(reports, { recursive: true });
+  await copy("06-counter-spa", spa, ["dist", "target"]);
+  await copy("07-fullstack-blog", blog, [
+    "dist",
+    "target",
+    join("data", "blog.json"),
+    join("data", "blog.json.tmp"),
+    join("data", "users.json"),
+    join("data", "users.json.tmp"),
+    join("data", "secret"),
+  ]);
+
+  const built = await run(nova, ["build", "--target", "wasm"], spa, env);
+  if (built.code !== 0) throw failure(`nova build --target wasm of 06 exited with ${built.code}`, built);
+  const ssr = await run(nova, ["build", "--ssr"], blog, env);
+  if (ssr.code !== 0) throw failure(`nova build --ssr of 07 exited with ${ssr.code}`, ssr);
+  const exe = join(blog, "target", "debug", `fullstack-blog-server${EXE}`);
+  const set = await run(exe, ["--set-password", "admin", "--iterations", "1000"], blog, env, `${PASSWORD}\n`);
+  if (set.code !== 0 || set.output.trim() !== "blog: set the password of admin") {
+    throw failure(`--set-password exited with ${set.code}`, set);
+  }
+
+  statics = await serve(join(spa, "dist"));
+  server = await startServer(blog, env);
+  const failures = [];
+  const medians = [];
+  let about = null;
+  for (const [name, label, url] of pages(statics.url, server.url)) {
+    const runs = await measure(label, url, chrome, reports);
+    const page = judge(name, runs);
+    console.log(page.line);
+    failures.push(...page.failures);
+    medians.push(`${name} ${page.score ?? "none"}`);
+    const first = runs.find((r) => r.failure === null);
+    about ??= first ? `Lighthouse ${first.version}, ${first.browser}, Node ${process.versions.node}` : null;
+  }
+  const what = `06's / and 07's /, /posts/1 and /login, ${RUNS} runs each, mobile`;
+  if (failures.length > 0) {
+    console.log(`lighthouse: failed, ${what}: ${failures.join("; ")}`);
+    process.exitCode = 1;
+  } else {
+    console.log(`lighthouse: ${what}, every median ${PASS} or more (${medians.join(", ")}); ${about}`);
+  }
+} catch (e) {
+  if (server) console.error(`the blog server's stderr:\n${server.stderr()}`);
+  console.error(e);
+  console.log(`lighthouse: failed, before its verdict: ${oneLine(e?.message ?? e)}`);
+  process.exitCode = 1;
+} finally {
+  server?.kill();
+  statics?.server.close();
+}
diff --git a/tests/browser/lighthouse.test.mjs b/tests/browser/lighthouse.test.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..36dd18ca0c5fd6a4fd95039bbc253dda002788a7
--- /dev/null
+++ b/tests/browser/lighthouse.test.mjs
@@ -0,0 +1,135 @@
+// The Lighthouse gate's rules, `lighthouse-rules.mjs`, tested without
+// Lighthouse or a browser (spec
+// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
+// §3, the user's decisions 1 and 2): the four pages and three runs each, the
+// median of three, and the verdict over a page's runs, a run that failed or
+// gave no score, and runs other than three in number, among them. Every
+// page here scores 100 or nearly, so `lighthouse.mjs`'s own runs would not
+// notice a wrong median or a dropped failure; these do. Node's own test
+// runner runs them, as CI's `browser` job does before the measurement:
+//
+//     node --test tests/browser/lighthouse.test.mjs
+
+import assert from "node:assert/strict";
+import { test } from "node:test";
+
+import { PASS, RUNS, judge, median, pages, runOf } from "./lighthouse-rules.mjs";
+
+// A run that scored `score`, as `runOf` returns one.
+function run(score) {
+  return {
+    failure: null,
+    score,
+    metrics: [901.2, 1651.7, 3, 0.0007, 901.2],
+    bench: 2312.5,
+    warnings: [],
+    version: "13.5.0",
+    browser: "HeadlessChrome/141.0.0.0",
+  };
+}
+
+// A report as Lighthouse writes one, its Performance score `score`.
+function report(score) {
+  return {
+    lighthouseVersion: "13.5.0",
+    categories: { performance: { score } },
+    audits: {
+      "first-contentful-paint": { numericValue: 901.2 },
+      "largest-contentful-paint": { numericValue: 1651.7 },
+      "total-blocking-time": { numericValue: 3 },
+      "cumulative-layout-shift": { numericValue: 0.0007 },
+      "speed-index": { numericValue: 901.2 },
+    },
+    environment: {
+      benchmarkIndex: 2312.5,
+      hostUserAgent: "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) HeadlessChrome/141.0.0.0 Safari/537.36",
+    },
+    runWarnings: [],
+  };
+}
+
+const exited = (code) => ({ code, signal: null });
+
+test("four pages, 06's / and 07's /, /posts/1 and /login, three runs each, 95 to pass", () => {
+  assert.deepEqual(pages("http://127.0.0.1:41001/", "http://127.0.0.1:41002/"), [
+    ["06 /", "06", "http://127.0.0.1:41001/"],
+    ["07 /", "07-home", "http://127.0.0.1:41002/"],
+    ["07 /posts/1", "07-post", "http://127.0.0.1:41002/posts/1"],
+    ["07 /login", "07-login", "http://127.0.0.1:41002/login"],
+  ]);
+  assert.equal(RUNS, 3);
+  assert.equal(PASS, 95);
+});
+
+test("the median of three is the middle score, in any order, and leaves them as they are", () => {
+  const scores = [93, 100, 99];
+  assert.equal(median(scores), 99);
+  assert.deepEqual(scores, [93, 100, 99]);
+  assert.equal(median([100, 93, 100]), 100);
+  assert.equal(median([94, 90, 100]), 94);
+});
+
+test("a page passes on a median of 95 or more, a low run among its three", () => {
+  const page = judge("07 /", [run(93), run(100), run(99)]);
+  assert.deepEqual(page.failures, []);
+  assert.equal(page.score, 99);
+  assert.equal(
+    page.line,
+    "07 /: scores 93, 100, 99, median 99; medians FCP 901 ms, LCP 1652 ms, TBT 3 ms, CLS 0.001, SI 901 ms; benchmark index 2313, 2313, 2313",
+  );
+  assert.deepEqual(judge("06 /", [run(95), run(100), run(95)]).failures, []);
+});
+
+test("a median under 95 fails the page", () => {
+  const page = judge("07 /login", [run(100), run(94), run(90)]);
+  assert.equal(page.score, 94);
+  assert.deepEqual(page.failures, ["07 /login's median is 94, under 95"]);
+});
+
+test("a failed run fails the page, whatever the other two scored, its line keeping the benchmark indexes", () => {
+  const page = judge("07 /posts/1", [run(100), { failure: "Lighthouse exited with 1" }, run(100)]);
+  assert.deepEqual(page.failures, ["07 /posts/1's run 2 failed (Lighthouse exited with 1)"]);
+  assert.equal(page.score, null);
+  assert.equal(page.line, "07 /posts/1: scores 100, failed, 100; benchmark index 2313, none, 2313");
+  const errored = judge("07 /", [run(100), { failure: "NO_FCP: no paint", bench: 811.4 }, run(100)]);
+  assert.equal(errored.line, "07 /: scores 100, failed, 100; benchmark index 2313, 811, 2313");
+});
+
+test("a run that gave no score fails the page, and so does a page with no runs", () => {
+  const page = judge("06 /", [run(100), run(100), { failure: null }]);
+  assert.deepEqual(page.failures, ["06 /'s run 3 failed (no Performance score)"]);
+  assert.equal(page.score, null);
+  assert.deepEqual(judge("06 /", []).failures, ["06 / had 0 runs, not 3", "06 / has no median"]);
+});
+
+test("two runs, or four, fail the page, however high they scored", () => {
+  const two = judge("07 /login", [run(90), run(100)]);
+  assert.deepEqual(two.failures, ["07 /login had 2 runs, not 3"]);
+  assert.match(two.line, /^07 \/login: scores 90, 100, median 100; /);
+  const four = judge("07 /login", [run(100), run(100), run(100), run(100)]);
+  assert.deepEqual(four.failures, ["07 /login had 4 runs, not 3"]);
+});
+
+test("a run fails when Lighthouse exits other than 0, leaves no report, or reports an error or no score", () => {
+  const ok = runOf(exited(0), report(0.99), "r.json");
+  assert.equal(ok.failure, null);
+  assert.equal(ok.score, 99);
+  assert.deepEqual(ok.metrics, [901.2, 1651.7, 3, 0.0007, 901.2]);
+  assert.equal(ok.bench, 2312.5);
+  assert.equal(ok.version, "13.5.0");
+  assert.equal(ok.browser, "HeadlessChrome/141.0.0.0");
+  assert.deepEqual(runOf(exited(1), report(1), "r.json"), { failure: "Lighthouse exited with 1", bench: 2312.5 });
+  assert.deepEqual(runOf({ code: null, signal: "SIGKILL" }, null, "r.json"), {
+    failure: "Lighthouse exited with SIGKILL",
+    bench: undefined,
+  });
+  assert.deepEqual(runOf(exited(0), null, "r.json"), { failure: "no report at r.json", bench: undefined });
+  const errored = { ...report(null), runtimeError: { code: "ERRORED_DOCUMENT_REQUEST", message: "(Status code: 404)" } };
+  assert.deepEqual(runOf(exited(0), errored, "r.json"), {
+    failure: "ERRORED_DOCUMENT_REQUEST: (Status code: 404)",
+    bench: 2312.5,
+  });
+  assert.deepEqual(runOf(exited(0), report(null), "r.json"), { failure: "no Performance score", bench: 2312.5 });
+  const other = { ...report(1), categories: { accessibility: { score: 1 } } };
+  assert.deepEqual(runOf(exited(0), other, "r.json"), { failure: "no Performance score", bench: 2312.5 });
+});
diff --git a/tests/browser/spa.mjs b/tests/browser/spa.mjs
index 7cab785ab338e472820643cb8ac380e2ce404ed0..01a6a7ffa2f992675d8381f29d04fd92cc9f103f 100644
--- a/tests/browser/spa.mjs
+++ b/tests/browser/spa.mjs
@@ -9,15 +9,44 @@
 // opens the page, checks the count reads 0, clicks `+` three times and `-`
 // once, finding the buttons by their text, and checks the count reads 2 and
 // the console holds no error. Playwright is the job's own install, as for `counter.mjs`.
+//
+// With `--release` after the two, as CI's step of release builds runs it on
+// a `nova build --target wasm --release` with binaryen's `wasm-opt` on
+// `PATH` (spec
+// docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
+// §4), it first checks that `dist/manifest.json` says the build is a
+// release build that `wasm-opt` optimized, and its line ends with that and
+// the client's size, `app.wasm` and `app.js` gzipped at the best level, as
+// `blog.mjs` measures its own. Without it, nothing changes.
 
 import { createServer } from "node:http";
 import { readFile } from "node:fs/promises";
 import { createRequire } from "node:module";
 import { extname, join, resolve, sep } from "node:path";
+import { gzipSync } from "node:zlib";
 
-const [dist, modules] = process.argv.slice(2).map((p) => resolve(p));
+const [dist, modules] = process.argv.slice(2, 4).map((p) => resolve(p));
+const release = process.argv[4] === "--release";
+if (process.argv.length > 5 || (process.argv.length === 5 && !release)) {
+  throw new Error("usage: node tests/browser/spa.mjs <dist> <dir holding node_modules/playwright> [--release]");
+}
 const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
 
+// Under `--release`: what the line adds, once the manifest says the build
+// is a release build that `wasm-opt` optimized.
+let built = "";
+if (release) {
+  const manifest = JSON.parse(await readFile(join(dist, "manifest.json"), "utf8"));
+  if (manifest.release !== true || manifest.wasm_opt !== true) {
+    throw new Error(`dist/manifest.json says release ${manifest.release} and wasm_opt ${manifest.wasm_opt}, not a release build that wasm-opt optimized`);
+  }
+  let gzipped = 0;
+  for (const name of ["app.wasm", "app.js"]) {
+    gzipped += gzipSync(await readFile(join(dist, name)), { level: 9 }).length;
+  }
+  built = `; the client is ${gzipped} bytes gzipped, from a release build that wasm-opt optimized`;
+}
+
 const types = {
   ".html": "text/html",
   ".js": "text/javascript",
@@ -64,7 +93,7 @@ try {
   for (const label of ["+", "+", "+", "-"]) await button(label).click();
   await reads("Count: 2");
   if (errors.length > 0) throw new Error(`the console has errors: ${errors.join("; ")}`);
-  console.log("counter-spa: +, +, +, -, and the count reads 2");
+  console.log(`counter-spa: +, +, +, -, and the count reads 2${built}`);
 } finally {
   await browser.close();
   server.close();
````


### Task 3: `gate.sh` on WASM

The user's decision 8; spec §8; decisions 18 and 31.

**Files:**
- Modify:
  - `.github/scripts/gate.sh`
  - `.github/workflows/ci.yml` (the `install` job's step)
  - `.github/scripts/package-gate.sh` (its call of `gate.sh`)
  - `.github/workflows/release.yml` (the smoke test)
  - `.github/workflows/release-gate.yml` (its two steps that run
    `gate.sh`)
- Test: `crates/nova-cli/tests/release_manifests.rs` (the Release gate's
  `grep` against `gate.sh`)

- [ ] **Step 1: The gate.** `patch_block t3-code`. After the native build
  `gate.sh` runs `nova build --target wasm` in its new project, checks the
  four files of `dist/`, the magic number of `dist/app.wasm`, and, where
  `node` is on `PATH`, that `node dist/app.js` prints `Hello, Nova!`; it
  takes `--test` and `--node` in any order, and with `--node` a `PATH`
  without `node` fails it; its comment names 4.8, the spec's §8 and the
  jobs that run it (decision 18). CI's `install` job, `package-gate.sh`
  and `release.yml`'s smoke test pass `--node`; the Release gate's two
  steps pass it when the tag's `gate.sh` takes it, and print the flags
  they pass; a comment at `gate.sh`'s `--node)` line says the Release gate
  greps for it; and `release_manifests.rs` gains
  `the_release_gate_passes_node_to_a_gate_sh_that_takes_it`, which reads
  the workflow and the script and holds them to each other (decision 31).
- [ ] **Step 2: Run it as CI's jobs do,** on a `nova` that carries its
  runtime library, alone in a directory, with no `NOVA_` variable set, and
  `--test --node`, as `gate_run` passes them: a debug build with
  `NOVA_EMBED_RUNTIME=1`, copied out, then the ordinary build again.

  ```sh
  NOVA_EMBED_RUNTIME=1 cargo build --locked -p nova-cli
  rm -rf "$P/gate-48" && mkdir -p "$P/gate-48/bin" && cp target/debug/nova "$P/gate-48/bin/nova"
  cargo build --locked -p nova-cli -p nova-runtime
  gate_run
  mkdir -p "$P/gate-48/nonode"
  for t in ar as cat cc clang env gcc grep head ld mkdir od tee tr; do ln -s "$(command -v $t)" "$P/gate-48/nonode/$t"; done
  env -i HOME="$HOME" PATH="$P/gate-48/nonode" /usr/bin/bash .github/scripts/gate.sh "$P/gate-48/bin/nova" "$P/gate-48/work-nonode" --test
  env -i HOME="$HOME" PATH="$P/gate-48/nonode" /usr/bin/bash .github/scripts/gate.sh "$P/gate-48/bin/nova" "$P/gate-48/work-nonode-required" --test --node
  ```

  Expected: the embedded build ends with `Finished` (about 15 s, its
  nested release build of the runtime cached; the first one takes
  minutes), and so does the build after it. `gate_run`, in about a second:

  ```text
  nova 0.3.2
  target: x86_64-unknown-linux-gnu
  runtime: embedded
  created `demo`: nova.toml, .gitignore, README.md, src/main.nova
  built target/debug/demo
  built dist
  gate: the WASM build prints Hello, Nova! under Node
  running 1 test
  test greeting_says_hello ... ok

  test result: ok. 1 passed; 0 failed; 0 trapped; 1 total
  gate: passed
  ```

  The run without `node` prints the same, but for `gate: no node on PATH,
  so the WASM build is built but not run` in place of the Node line, and
  passes. The last, without `node` but with `--node`, stops after `built
  dist` with `gate: --node, but there is no node on PATH to run the WASM
  build`, and exits 1. Windows and macOS run it in CI (Global
  Constraints).
- [ ] **Step 3: The Release gate's test of the tag's `gate.sh`.** Its two
  steps pass `--node` when `grep -qF -- '--node) with_node=--node'
  .github/scripts/gate.sh` matches, in the tag's checkout, and print the
  flags before they run `gate.sh`. The grep on three `gate.sh`s, the two
  steps' text but their call of `gate.sh`, and the test, then the test with
  `gate.sh`'s line drifted by a space:

  ```sh
  grep -cF -- '--node) with_node=--node' .github/scripts/gate.sh
  git show 54ac836:.github/scripts/gate.sh | grep -cF -- '--node) with_node=--node'
  git show 230f4ba:.github/scripts/gate.sh | grep -cF -- '--node) with_node=--node'
  python3 -c 'import subprocess, yaml
  w = yaml.safe_load(open(".github/workflows/release-gate.yml"))
  for job in w["jobs"].values():
      for step in job["steps"]:
          if "bash .github/scripts/gate.sh" in step.get("run", ""):
              text = "\n".join(l for l in step["run"].splitlines() if not l.startswith("bash .github/scripts/gate.sh"))
              subprocess.run(["bash", "-c", text], check=True)'
  cargo test -p nova-cli --test release_manifests
  cp .github/scripts/gate.sh "$P/gate-48.sh"
  sed -i 's/^    --node) with_node=--node ;;$/    --node)  with_node=--node ;;/' .github/scripts/gate.sh
  cargo test -p nova-cli --test release_manifests the_release_gate 2>&1 | grep -E "no line holds|^test result"
  cp "$P/gate-48.sh" .github/scripts/gate.sh
  cargo test -p nova-cli --test release_manifests the_release_gate 2>&1 | grep "^test result"
  ```

  Expected: `1`, then `0` and `0`: `v0.3.2`'s commit and `v0.3.1`'s
  (`git ls-remote --tags origin 'v0.3.*'` names both), whose `gate.sh`
  takes no `--node`, so a re-run of the Release gate for them passes
  none. The steps run under `shell: bash`, Git Bash on Windows. Then
  `gate.sh --test --node` twice, what each step prints. Then `8 passed`,
  `the_release_gate_passes_node_to_a_gate_sh_that_takes_it` among them.
  With the line drifted:

  ```text
  <repository>/crates/nova-cli/../../.github/scripts/gate.sh: no line holds "--node) with_node=--node", which release-gate.yml greps for
  test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.00s
  ```

  and, once restored from the copy (the task is not committed yet, so not
  with `git checkout`), `test result: ok. 1 passed; 0 failed; 0 ignored; 0
  measured; 7 filtered out`.
- [ ] **Step 4: The checks,** as the conventions list them, actionlint
  among them.
- [ ] **Step 5: Commit** `ci: gate.sh builds its project for WASM, and runs it under Node`.

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/.github/scripts/gate.sh b/.github/scripts/gate.sh
index b28b8619b746d2f4095dd0a868928ffaa13832a6..1d67d33e457c16567fce7285887be795afbc9f3b 100644
--- a/.github/scripts/gate.sh
+++ b/.github/scripts/gate.sh
@@ -2,21 +2,50 @@
 # The Phase 3.0 gate (spec
 # docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §7 and
 # §8): a `nova` with nothing beside it carries its runtime library, and
-# makes, runs and builds a new project; with --test it also tests it. CI's
-# `install` job and release.yml's smoke test both run this script.
+# makes, runs and builds a new project; with --test it also tests it. Since
+# 4.8 (spec
+# docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
+# §8, and the user's decision 8 in its header) it also builds the project
+# for WASM: `dist/` holds the page, the shim, the module and the manifest
+# (and the `package.json` that makes the shim a module to Node), the module
+# begins with WASM's magic number, and, where `node` is on PATH, the shim
+# run under Node prints what the native build prints. With --node that run
+# is required, and a PATH without `node` fails the gate. Its callers, each
+# on runners that have `node`: CI's `install` job, with --node;
+# package-gate.sh, which CI's `package` job and publish-crates.yml run,
+# with --node; release.yml's smoke test, with --node; and the Release gate
+# (release-gate.yml), whose two jobs run the gate.sh of the tag they check
+# and pass --node when that gate.sh takes it, from v0.4.0 on.
 #
-# Usage: gate.sh NOVA WORKDIR [--test]
+# Usage: gate.sh NOVA WORKDIR [--test] [--node]
 #   NOVA     the nova executable to check
 #   WORKDIR  where to make the project; it must not exist yet
+#   --test   also run `nova test` in it
+#   --node   require the WASM build's run under Node
 set -euo pipefail
 
-if [ $# -lt 2 ] || [ $# -gt 3 ]; then
-  echo "usage: gate.sh NOVA WORKDIR [--test]" >&2
+usage() {
+  echo "usage: gate.sh NOVA WORKDIR [--test] [--node]" >&2
   exit 2
+}
+if [ $# -lt 2 ]; then
+  usage
 fi
 nova=$1
 work=$2
-with_test=${3:-}
+shift 2
+with_test=""
+with_node=""
+for flag in "$@"; do
+  case "$flag" in
+    --test) with_test=--test ;;
+    # release-gate.yml greps for the next line, as it stands, to pass --node
+    # only to a gate.sh that takes it; change the two together
+    # (crates/nova-cli/tests/release_manifests.rs checks that they agree).
+    --node) with_node=--node ;;
+    *) usage ;;
+  esac
+done
 
 if env | grep -q '^NOVA_'; then
   env | grep '^NOVA_' >&2
@@ -49,6 +78,32 @@ if [ "$built" != "Hello, Nova!" ]; then
   exit 1
 fi
 
+"$nova" build --target wasm
+for file in index.html app.js app.wasm manifest.json; do
+  if [ ! -s "dist/$file" ]; then
+    echo "gate: nova build --target wasm wrote no dist/$file" >&2
+    exit 1
+  fi
+done
+magic=$(od -A n -t x1 -N 4 dist/app.wasm | tr -d ' \n')
+if [ "$magic" != "0061736d" ]; then
+  echo "gate: dist/app.wasm begins with $magic, not WASM's magic number" >&2
+  exit 1
+fi
+if command -v node > /dev/null; then
+  wasm=$(node dist/app.js)
+  if [ "$wasm" != "Hello, Nova!" ]; then
+    echo "gate: the WASM build printed '$wasm' under Node" >&2
+    exit 1
+  fi
+  echo "gate: the WASM build prints Hello, Nova! under Node"
+elif [ "$with_node" = "--node" ]; then
+  echo "gate: --node, but there is no node on PATH to run the WASM build" >&2
+  exit 1
+else
+  echo "gate: no node on PATH, so the WASM build is built but not run"
+fi
+
 if [ "$with_test" = "--test" ]; then
   "$nova" test | tee test.txt
   if ! grep -q '1 passed; 0 failed' test.txt; then
diff --git a/.github/scripts/package-gate.sh b/.github/scripts/package-gate.sh
index fa63888c3d4653fa8c40e70e96428c646db2ab8b..e2573cfac89efbc413013eb2925ecdc9bdaf21ab 100644
--- a/.github/scripts/package-gate.sh
+++ b/.github/scripts/package-gate.sh
@@ -5,8 +5,9 @@
 # crates.io would check them; each packaged Cargo.lock names only what the
 # workspace's names, at the same versions; `nova` installs from the unpacked
 # packages as `cargo install nova-cli` would from crates.io, a release build
-# that embeds the runtime; and that nova passes gate.sh --test. CI's
-# `package` job runs it, from the repository's root, on all three systems.
+# that embeds the runtime; and that nova passes gate.sh --test --node, its
+# WASM build run under Node, which every runner has. CI's `package` job runs
+# it, from the repository's root, on all three systems.
 #
 # Usage: package-gate.sh WORKDIR
 #   WORKDIR  where to package, unpack and install; it must not exist yet
@@ -137,7 +138,7 @@ step_done "installing nova from the packages"
 
 # 4. The installed nova carries its runtime, and makes, runs, builds and
 #    tests a project.
-bash .github/scripts/gate.sh "$workdir/root/bin/nova" "$workdir/gate" --test
+bash .github/scripts/gate.sh "$workdir/root/bin/nova" "$workdir/gate" --test --node
 step_done "the gate"
 
 echo "package-gate: passed in $((SECONDS - began)) s"
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 91ff91e445820dd1a1af192b650b941600ef308d..894905e4ba123c72ae5bf5c6dd8f249c10ba8212 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -343,9 +343,13 @@ jobs:
             nova|nova.exe) ;;
             *) echo "::error::the install put more than nova into bin/"; exit 1 ;;
           esac
+      # --node: the runner has `node`, so the WASM build must run under it
+      # (spec
+      # docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
+      # §8).
       - name: The installed nova makes, runs, builds and tests a project
         shell: bash
-        run: bash .github/scripts/gate.sh "$RUNNER_TEMP/nova/bin/nova" "$RUNNER_TEMP/gate" --test
+        run: bash .github/scripts/gate.sh "$RUNNER_TEMP/nova/bin/nova" "$RUNNER_TEMP/gate" --test --node
       # The Phase 3.3a gate (spec
       # docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §8).
       - name: The installed nova builds a program against a path library
diff --git a/.github/workflows/release-gate.yml b/.github/workflows/release-gate.yml
index 852f775d1ddbcf944886fd1769a62b2aebdd1182..0c9bcd4db7021de0e2955d2508533e8ec5df07e0 100644
--- a/.github/workflows/release-gate.yml
+++ b/.github/workflows/release-gate.yml
@@ -71,9 +71,18 @@ jobs:
             echo "::error::nova version printed '$first', not 'nova $VERSION'"
             exit 1
           fi
+      # --node, so the WASM build must run under the runner's `node`, when
+      # the tag's gate.sh takes it: from v0.4.0 on; earlier tags' do not.
+      # The grep's string is gate.sh's own `--node)` line, which
+      # release_manifests.rs checks; the step prints the flags it passes.
       - name: The installed nova makes, runs, builds and tests a project
         shell: bash
-        run: bash .github/scripts/gate.sh "$RUNNER_TEMP/crates-io/bin/nova" "$RUNNER_TEMP/gate" --test
+        run: |
+          set -euo pipefail
+          flags=(--test)
+          if grep -qF -- '--node) with_node=--node' .github/scripts/gate.sh; then flags+=(--node); fi
+          echo "gate.sh ${flags[*]}"
+          bash .github/scripts/gate.sh "$RUNNER_TEMP/crates-io/bin/nova" "$RUNNER_TEMP/gate" "${flags[@]}"
       # gate.sh made the project with `nova new demo` in its WORKDIR. The
       # templates are written in nova fmt's layout, so the check finds
       # nothing to change.
@@ -149,8 +158,14 @@ jobs:
             exit 1
           fi
           echo "nova=$nova" >> "$GITHUB_OUTPUT"
+      # --node when the tag's gate.sh takes it, as in the job above.
       - name: The installed nova makes, runs, builds and tests a project
         shell: bash
         env:
           INSTALLED: ${{ steps.installed.outputs.nova }}
-        run: bash .github/scripts/gate.sh "$INSTALLED" "$RUNNER_TEMP/gate" --test
+        run: |
+          set -euo pipefail
+          flags=(--test)
+          if grep -qF -- '--node) with_node=--node' .github/scripts/gate.sh; then flags+=(--node); fi
+          echo "gate.sh ${flags[*]}"
+          bash .github/scripts/gate.sh "$INSTALLED" "$RUNNER_TEMP/gate" "${flags[@]}"
diff --git a/.github/workflows/release.yml b/.github/workflows/release.yml
index 8936ae06ab7b2112578ea1e0ee30c49b6d8fb37b..3f016ddca4953afe0b1be3fe9284d64357763cf8 100644
--- a/.github/workflows/release.yml
+++ b/.github/workflows/release.yml
@@ -111,6 +111,8 @@ jobs:
             tar -C dist -czf "dist/$name.tar.gz" "$name"
             echo "ARCHIVE=dist/$name.tar.gz" >> "$GITHUB_ENV"
           fi
+      # --node: the runner has `node`, so gate.sh's WASM build must run
+      # under it.
       - name: Smoke-test the archive
         if: matrix.smoke
         shell: bash
@@ -122,7 +124,7 @@ jobs:
             *.zip) 7z x -o"$unpacked" "$ARCHIVE" > /dev/null ;;
             *) tar -C "$unpacked" -xzf "$ARCHIVE" ;;
           esac
-          bash .github/scripts/gate.sh "$(ls -d "$unpacked"/nova-*)/nova" "$RUNNER_TEMP/smoke"
+          bash .github/scripts/gate.sh "$(ls -d "$unpacked"/nova-*)/nova" "$RUNNER_TEMP/smoke" --node
       # install.ps1 (spec
       # docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
       # §6.3), run in Windows PowerShell 5.1 as a user would, with
diff --git a/crates/nova-cli/tests/release_manifests.rs b/crates/nova-cli/tests/release_manifests.rs
index 0150b72ddd63fc8b6c914f0d1c11bd1cd8eee07c..6a7584e70c2c951c93ffc80a6f9ecd69874dcfc1 100644
--- a/crates/nova-cli/tests/release_manifests.rs
+++ b/crates/nova-cli/tests/release_manifests.rs
@@ -7,6 +7,9 @@
 //! workspace's version, `publish-crates.sh` uploads them in dependency
 //! order, each tarball carries a README and both licence texts, and the
 //! VS Code extension has the same version, and its lock the same name.
+//! And the Release gate passes `--node` to a `gate.sh` that takes it (spec
+//! `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+//! §8).
 
 use std::collections::{BTreeMap, BTreeSet};
 use std::path::{Path, PathBuf};
@@ -318,3 +321,91 @@ fn the_extension_has_the_workspace_version_and_its_name() {
         "package-lock.json's packages[\"\"] name"
     );
 }
+
+/// The first line of `step`, a step of the Release gate, that `wanted`
+/// accepts; `what` names it in the panic when there is none.
+fn line_of(step: &[&str], what: &str, wanted: impl Fn(&str) -> bool) -> usize {
+    step.iter()
+        .position(|line| wanted(line))
+        .unwrap_or_else(|| panic!("release-gate.yml: a step with no {what}: {step:?}"))
+}
+
+/// The Release gate runs the `gate.sh` of the tag it checks, and its two
+/// steps that run it pass `--node` only when that `gate.sh` takes it: each
+/// greps the script for a fixed string, its `--node)` line, prints the
+/// flags and passes them. A `gate.sh` whose line drifted from that string
+/// would be run without `--node`, and would pass with its Node run skipped,
+/// so the string must be in `gate.sh`, and both steps must look for it and
+/// pass what they found.
+#[test]
+fn the_release_gate_passes_node_to_a_gate_sh_that_takes_it() {
+    let workflow_path = root()
+        .join(".github")
+        .join("workflows")
+        .join("release-gate.yml");
+    let workflow = read(&workflow_path);
+    let gate_path = root().join(".github").join("scripts").join("gate.sh");
+    let gate = read(&gate_path);
+
+    // A step begins at a `- ` six spaces in and holds the lines indented
+    // further; its lines are kept trimmed.
+    let mut steps: Vec<Vec<&str>> = Vec::new();
+    let mut step: Option<Vec<&str>> = None;
+    for line in workflow.lines() {
+        if let Some(first) = line.strip_prefix("      - ") {
+            steps.extend(step.replace(vec![first.trim()]));
+        } else if line.starts_with("        ") {
+            if let Some(step) = step.as_mut() {
+                step.push(line.trim());
+            }
+        } else if !line.trim().is_empty() {
+            steps.extend(step.take());
+        }
+    }
+    steps.extend(step);
+    let gating: Vec<&Vec<&str>> = steps
+        .iter()
+        .filter(|step| {
+            step.iter()
+                .any(|line| line.starts_with("bash .github/scripts/gate.sh "))
+        })
+        .collect();
+    assert_eq!(
+        gating.len(),
+        2,
+        "{}: the steps that run gate.sh",
+        workflow_path.display()
+    );
+
+    let grep = "if grep -qF -- '";
+    let found = ".github/scripts/gate.sh; then flags+=(--node); fi";
+    let mut strings = BTreeSet::new();
+    for step in gating {
+        let start = line_of(step, "`flags=(--test)`", |line| line == "flags=(--test)");
+        let test = line_of(step, "grep of gate.sh", |line| {
+            line.starts_with(grep) && line.ends_with(found)
+        });
+        let echo = line_of(step, "echo of the flags", |line| {
+            line == r#"echo "gate.sh ${flags[*]}""#
+        });
+        let call = line_of(step, "run of gate.sh with the flags", |line| {
+            line.starts_with("bash .github/scripts/gate.sh ") && line.ends_with(r#" "${flags[@]}""#)
+        });
+        assert!(
+            start < test && test < echo && echo < call,
+            "{}: a step out of order: {step:?}",
+            workflow_path.display()
+        );
+        let string = step[test][grep.len()..]
+            .split('\'')
+            .next()
+            .expect("the grep's string");
+        assert!(
+            gate.lines().any(|line| line.contains(string)),
+            "{}: no line holds {string:?}, which release-gate.yml greps for",
+            gate_path.display()
+        );
+        strings.insert(string);
+    }
+    assert_eq!(strings.len(), 1, "the two steps grep for {strings:?}");
+}
````


### Task 4: The documentation and the records

Spec §5, §6, §9; the user's decisions 5 to 8; decisions 2, 3, 7, 8, 10 and
19 to 26.

**Files:**
- Create: `docs/adr/0047-phase-4-boundary.md`
- Modify:
  - `README.md`, `crates/nova-cli/README.md` (the "Web" sections)
  - `crates/nova-cli/Cargo.toml` (`description`, `keywords`)
  - `docs/releasing.md` (`old=`, the 21 lines, two dated notes)
  - `agent.md`, `skill.md` (their rows)
  - `examples/06-counter-spa/README.md`,
    `examples/07-fullstack-blog/README.md`
  - `nova-spec/00-MASTER-SPEC.md`, `30-FRONTEND.md` (§7),
    `40-TOOLING.md` (§1.1), `60-EXAMPLES.md` (§6, §7)
  - `docs/phase-4-plan.md` (4.8's entry), `CHANGELOG.md`

- [ ] **Step 1: The records.** `patch_block t4-code`. Each dated note reads
  "Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`)".
  ADR 0047 follows ADR 0037's form: its context, the user's eight
  decisions, the inventory of 46 rows, the three deviations it decides,
  the rows whose record already explains them, the gate's record, the
  backlog, what was found outside the boundary, and its consequences.
- [ ] **Step 2: Read them against the code.** Each claim a record makes is
  one this plan's code makes true, and each departure ADR 0047 names is one
  of this plan's decisions:
  - the user's decisions and the spec's §12, against decisions 1 to 10;
  - the flags, against `WASM_OPT_FLAGS` and the backend's `MemOp::Copy`
    and `MemOp::Fill`; the help text, against `cmd/run.rs`;
  - Lighthouse, against `lighthouse-rules.mjs`'s `PASS`, `RUNS`, pages
    and verdict, and `lighthouse.mjs`'s arguments and failure paths; the
    release builds and the install, against the `browser` job; `gate.sh`'s
    checks and its callers' `--node`, against the script and the
    workflows;
  - the figures, against the runs: the Lighthouse lines and sizes of Task
    2, part A's in ADR 0047's transcript, the replay's alike; the scores
    said as measured: part A's runs all 100, and of the first replay's 94,
    one at 93, a run of 07's `/`, every median 100 (ADR 0047's "Measured
    again", the CHANGELOG, `60-EXAMPLES.md` §6 and §7); the size
    tests' 5,006 and 21,070 bytes by `flate2`, and 4,959 and 20,997 by
    zlib, which no script prints; the release sizes, 11,974 and 27,510
    bytes gzipped by zlib, 16,525 and 57,179 raw; each with its compressor
    named wherever a record gives it: the CHANGELOG, `30-FRONTEND.md`,
    `60-EXAMPLES.md`, both examples' READMEs and ADR 0047 (decision 25);
  - ADR 0047's inventory and backlog, against the master spec's Phase 4
    list and gate, `docs/phase-4-plan.md`'s §3 and §4, `60-EXAMPLES.md`
    §6 and §7, `30-FRONTEND.md` §7 to §13, and ADRs 0038 to 0046, each row
    to its record; its crate counts, against `publish-crates.sh`'s list of
    20 and ADR 0037's record of 19 (decision 7);
  - the "Web" sections, against `nova build`'s and `nova dev`'s flags and
    std's `ui`, `ui::html`, `ui::router`, `ui::ssr` and `dom`, and their
    lists of `dist/` against `bundle.rs`'s `BUILT`, `package.json` among
    them; `agent.md`'s
    and `skill.md`'s rows, against ADRs 0039, 0041, 0044, 0045 and 0047;
  - the runbook, against the root `Cargo.toml`: one `version` line and 20
    `=` pins, so 21 lines; `old=`'s `perl`, which prints `0.3.2`.

  A record that says what the code does not is fixed here, and the ledger
  says so.
- [ ] **Step 3: Run.** First the build.
  - `cargo test -p nova-cli --bin nova`. Expected: `41 passed; 0 failed`.
  - `cargo test -p nova-cli --test release_manifests`. Expected: `8
    passed`, Task 3's test among them.
  - `cargo test -p nova-cli --test vscode_extension`. Expected: `3
    passed`.
  - `cargo package --list -p nova-cli --allow-dirty --offline | grep -n
    README`. Expected: `7:README.md`: the README with the "Web" section is
    the one crates.io shows.
- [ ] **Step 4: The checks,** as the conventions list them.
- [ ] **Step 5: Commit** `docs: ADR 0047, Phase 4's boundary, and the notes for 4.8`.

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 658833edeb3fc6ed463c44654cda199c69502050..53c99d0e9a5dbd78e60c1d206eea5a857181562a 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -150,6 +150,34 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
   page hydrates in place, the admin creates, edits and deletes a post, a
   script in a title or a comment stays text, and a form without its CSRF
   token is refused.
+- **Lighthouse in CI, Phase 4's gate measured,**
+  `tests/browser/lighthouse.mjs`: the `browser` job installs Lighthouse
+  13.5.0 and runs it three times on each of 06's `/` and 07's `/`,
+  `/posts/1` and `/login`, with its mobile preset, and fails when a page's
+  median Performance score is under 95, a run fails, or a page gets other
+  than three runs; each page's line gives its runs' benchmark indexes,
+  Lighthouse's measure of the host's speed. The rules it
+  applies, the pages, the runs, the median and the verdict, are tested
+  first without Lighthouse, under `node --test`
+  (`tests/browser/lighthouse.test.mjs`). Where 4.8 was built, every
+  page's median was 100: the prototype's runs all scored 100, and the step
+  run again, 94 runs in all, gave one 93, a run of 07's `/` (ADR 0047).
+- **A CI test of release builds with binaryen's `wasm-opt`:** the same job
+  builds 06 and 07 with `--release`, checks that `wasm-opt` ran and made
+  each `app.wasm` smaller, and drives both builds in Chromium.
+- **`gate.sh` builds its new project for WASM** too, so CI's install and
+  package jobs, the release's smoke test and the Release gate test a
+  packaged and an installed `nova` on WASM: `dist/` holds the page, the
+  shim, the module, the manifest and the `package.json` that makes the
+  shim a module to Node, the module begins with WASM's magic number, and,
+  where `node` is on PATH, `node dist/app.js` prints what the native
+  build prints. With `--node`, which CI's install and package jobs
+  and the release's smoke test pass, and the Release gate when the tag's
+  `gate.sh` takes it, from `v0.4.0` on, a PATH without `node` fails it.
+- **A "Web" section in the README and in `nova-cli`'s README,** which
+  crates.io shows: `nova build --target wasm`, `nova dev`, `--release` and
+  `wasm-opt`, `--ssr` and `--ssg`, `std/ui` and its modules, `std/dom`,
+  which std's documentation site leaves out, and the two examples.
 
 ### Changed
 
@@ -175,6 +203,20 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
   `fallback`.
 - **The status line names 301, 302, 303, 401 and 403** by their reason
   phrases, `303 See Other` where it was `303 Redirection`.
+- **`nova build --help`** says that `--release` optimizes `app.wasm` for
+  size with binaryen's `wasm-opt`, where it named `wasm-opt -Oz`.
+- **`nova-cli`'s description and keywords on crates.io** name WebAssembly
+  and the web; the keyword `wasm` replaces `formatter`.
+
+### Fixed
+
+- **`nova build --target wasm --release` with binaryen's `wasm-opt` on
+  PATH** failed for every program: it passed `-Oz` alone, and `wasm-opt`
+  refuses the `memory.copy` and `memory.fill` the backend writes unless
+  bulk memory is enabled. It now passes `--enable-bulk-memory` too, and
+  `--ssr` and `--ssg` with `--release` build their client the same way.
+  With binaryen 132, 06's client is 11,974 bytes gzipped by zlib at level
+  9, from 20,997, and 07's 27,510, from 49,472 (ADR 0047).
 
 ## [0.3.2] - 2026-10-14
 
diff --git a/README.md b/README.md
index f0da3df80996ebb2d0cf3f85c4f9faeb19c03cb5..082baada9070c9a0ac23951fbc92198a72d837e3 100644
--- a/README.md
+++ b/README.md
@@ -77,6 +77,48 @@ them after the library's other tests, and `nova test --doc` runs them
 alone. A line of an example that starts with `# ` runs but is not
 shown.
 
+### Web
+
+`nova build --target wasm` compiles a program to WebAssembly, with no
+other toolchain: `dist/` holds the page (the project's own `index.html`,
+or a generated one), `app.js`, `app.wasm`, `manifest.json`, a
+`package.json` that makes `app.js` a module to Node, and `public/` copied
+into `assets/`. Any static server can serve it, with `app.wasm` as
+`application/wasm`, and `node dist/app.js` runs a program that does not
+touch the page. `nova dev` builds the same page and serves it on
+`http://127.0.0.1:3000/`; when a source changes it builds again and
+reloads the page, or shows the build's errors over it. `--port` chooses
+another port, and `--open` opens a browser.
+
+With `--release`, a build for WASM also runs binaryen's `wasm-opt` on
+`app.wasm`, as `wasm-opt -Oz --enable-bulk-memory`, when `wasm-opt` is on
+your PATH (CI tests binaryen 132, from npm's `binaryen`); without it, it
+says so in one line and leaves `app.wasm` as a build without `--release`
+makes it.
+
+In a project with `src/server.nova` beside `src/main.nova`, `nova build
+--ssr` builds the client from `src/main.nova` into `dist/`, and the server
+natively into `target/debug/<name>-server`, which needs the C linker `nova
+build` needs. A server that renders its pages with `ui::ssr` sends HTML
+that the client adopts with `hydrate`. `nova build --ssg` also runs the
+server once, and `ui::ssr::serve` then writes `dist/<path>/index.html` for
+each route other than `*` with no parameters.
+
+A program reaches the page through std modules it imports: `import ui`
+(signals, effects, memos, `mount` and `hydrate`), `import ui::html` (the
+element builders a `view { … }` calls), `import ui::router` (`Router`,
+`HistoryRouter`, `link` and `navigate`) and, natively, `import ui::ssr`
+(`serve`, `page` and `Dist`, for the server). std's documentation site
+documents all four. `import dom`, the WASM-only layer beneath them
+(elements, listeners, `location` and `history`, timers, `fetch`), is not
+on the site: its documentation is its source, `std/dom/lib.nova`.
+
+Two examples put it together: `examples/06-counter-spa`, a counter written
+as a component, runs with `nova dev` in its directory; and
+`examples/07-fullstack-blog`, a blog whose server renders every page, with
+sessions, forms and an admin, builds with `nova build --ssr`, then sets its
+admin's password and starts its server, as its README shows.
+
 ---
 
 ## How to use this bundle
diff --git a/agent.md b/agent.md
index 3a480f7302cadf994fd034d64ff779362422b66d..971a16055ccbc9977b0ac859ffdd72d313104ca2 100644
--- a/agent.md
+++ b/agent.md
@@ -118,14 +118,14 @@ These are from `nova-spec/00-MASTER-SPEC.md §5`. Violating them will cause CI f
 | `nova-mir` | 3-address IR | `nova-hir` |
 | `nova-codegen-cranelift` | Debug-mode object files | `cranelift` |
 | `nova-codegen-llvm` | Release-mode object files | — (textual LLVM IR, compiled by `clang` or `llc`) |
-| `nova-codegen-wasm` | WASM modules | `wasm-encoder`, `walrus` |
+| `nova-codegen-wasm` | WASM modules, and the JS shim that runs them (ADR 0039) | `wasm-encoder` (no `walrus`) |
 | `nova-runtime` | GC + async runtime (Rust, linked) | `httparse`, `ring`; its own single-threaded executor (ADR 0009) |
 | `nova-driver` | Pipeline orchestration | all crates above |
 | `nova-cli` | `nova` binary, CLI arg parsing | `nova-driver`, `clap` |
 | `nova-fmt` | Opinionated formatter | `nova-ast` |
 | `nova-lsp` | LSP server | `tower-lsp` |
 | `nova-pm` | Package manager | `nova-diagnostics`, `nova-lexer`, `toml_edit` |
-| `nova-bundler` | Frontend bundler | `nova-codegen-wasm` |
+| `nova-bundler` | A stub, `publish = false`; a WASM build's bundle is `nova-cli`'s `bundle.rs` (ADR 0047) | — |
 | `nova-doc` | Doc generator | `nova-ast` |
 | `nova-test` | A stub; the test runner is `nova-cli`'s `cmd/test.rs` | — |
 
diff --git a/crates/nova-cli/Cargo.toml b/crates/nova-cli/Cargo.toml
index 112b9dd2e0f80b6dc44e7a9e0bcbe3f5edb0a5cd..703c9cbf8a55c86e09522999d93c6ea6902a78b3 100644
--- a/crates/nova-cli/Cargo.toml
+++ b/crates/nova-cli/Cargo.toml
@@ -6,9 +6,9 @@ rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
 homepage.workspace = true
-description = "The Nova programming language: compiler, formatter, language server, package manager and documentation generator"
+description = "The Nova programming language: compiler to native code and to WebAssembly for the web, dev server, formatter, language server, package manager and documentation generator"
 readme = "README.md"
-keywords = ["nova", "programming-language", "compiler", "lsp", "formatter"]
+keywords = ["nova", "programming-language", "compiler", "wasm", "lsp"]
 categories = ["compilers", "command-line-utilities", "development-tools"]
 
 [[bin]]
diff --git a/crates/nova-cli/README.md b/crates/nova-cli/README.md
index fcb82c94c2dc037f1929360772dbd6c054209629..16710b33c3caab0b1a736e44d4aed27dc7756af1 100644
--- a/crates/nova-cli/README.md
+++ b/crates/nova-cli/README.md
@@ -36,7 +36,8 @@ irm https://github.com/Sakeerin/nova/releases/latest/download/install.ps1 | iex
 
 `nova run` needs nothing else. `nova build` and `nova test` write native
 executables, so they need a C linker: `cc` on Linux and macOS, and the
-Visual Studio Build Tools on Windows.
+Visual Studio Build Tools on Windows. A build for WebAssembly, and
+`nova dev`, need no linker.
 
 ## Start
 
@@ -46,6 +47,35 @@ cd hello
 nova run
 ```
 
+## Web
+
+- `nova build --target wasm` compiles a program to WebAssembly into
+  `dist/`: the page, `app.js`, `app.wasm`, `manifest.json`, a
+  `package.json` for Node, and `public/` as `assets/`. It needs nothing
+  else installed.
+- `nova dev` builds it and serves it on `http://127.0.0.1:3000/`,
+  building again and reloading the page when a source changes, and showing
+  the build's errors over it; `--port` and `--open` change where and how.
+- With `--release`, a build for WebAssembly also runs `wasm-opt -Oz
+  --enable-bulk-memory` on `app.wasm` when binaryen's `wasm-opt` is on
+  your PATH, and says so when it is not.
+- `nova build --ssr` builds a project's client from `src/main.nova` and
+  its server, natively, from `src/server.nova`, which renders the pages
+  the client hydrates; `nova build --ssg` also writes a page for each
+  route other than `*` with no parameters.
+
+A program imports what it uses: `ui` (signals, effects, `mount`,
+`hydrate`), `ui::html` (the element builders `view { … }` calls),
+`ui::router` and, natively, `ui::ssr`; std's documentation site,
+attached to each GitHub release, documents them. `dom`, the WASM-only
+layer beneath them, is not on the site: read
+[`std/dom/lib.nova`](https://github.com/Sakeerin/nova/blob/main/std/dom/lib.nova).
+The repository's
+[`examples/06-counter-spa`](https://github.com/Sakeerin/nova/tree/main/examples/06-counter-spa)
+and
+[`examples/07-fullstack-blog`](https://github.com/Sakeerin/nova/tree/main/examples/07-fullstack-blog)
+say in their READMEs how to run each.
+
 ## Links
 
 - [The repository](https://github.com/Sakeerin/nova): the language's
diff --git a/docs/adr/0047-phase-4-boundary.md b/docs/adr/0047-phase-4-boundary.md
new file mode 100644
index 0000000000000000000000000000000000000000..d3cc3c8825692ef71d8b734f19700db85e8912b2
--- /dev/null
+++ b/docs/adr/0047-phase-4-boundary.md
@@ -0,0 +1,463 @@
+# ADR 0047 — Phase 4's boundary
+
+## Status
+
+Accepted (2026-10-22). Branch `claude/loving-archimedes-frtf5q-48`
+(`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+§6). The gate's measurement, the order of the release and `nova-bundler`'s
+place are the user's decisions of 2026-10-22, listed below.
+
+## Context
+
+`nova-spec/00-MASTER-SPEC.md` §7 calls a phase done when:
+1. its crates compile and pass CI;
+2. its gate criteria are met with reproducible commands;
+3. its new public surface is documented;
+4. `CHANGELOG.md` is updated;
+5. an ADR records each decision that deviates from the spec;
+6. a `v0.{phase}.0` milestone is tagged.
+
+Phase 4, "Frontend / WASM", makes its promises in three places:
+- **the master spec's §3:** the Phase 4 list of nine numbered entries,
+  cited below as "position N", and the Phase 4 gate:
+  "`examples/06-counter-spa` and `examples/07-fullstack-blog` work in
+  browser, pass Lighthouse 95+";
+- **`docs/phase-4-plan.md`:** its §3 decisions, cited as "decision N", and
+  sub-phases 4.0 to 4.8, each with its own gate;
+- **`nova-spec/60-EXAMPLES.md` §6 and §7,** which give each example's
+  gate, and **`nova-spec/30-FRONTEND.md`,** which describes more than the
+  gate needs; its rows below are cited as "30-FRONTEND §N".
+
+Earlier ADRs already narrowed Phase 4:
+- 0038: the scope, what the gate's two examples need, and the list of
+  what Phase 4 leaves out;
+- 0039 and 0040: the WASM backend, its runtime in Nova, and its collector
+  and executor;
+- 0041: `std/dom` and the boundary with JS;
+- 0042 and 0043: `std/ui`, and the component syntax with the bundle's last
+  pieces;
+- 0044: the router and `nova dev`;
+- 0045: SSR, hydration and SSG;
+- 0046: the blog, and what std gained for it.
+
+Phase 4 was built in eight pull requests, #117 to #124, one for each of
+4.0 to 4.7, merged from 2026-10-15 to 2026-10-22. No release carries it
+yet: `v0.3.2`, released on 2026-10-14, is the last.
+
+The user decided on 2026-10-22, and approved with the 4.8 spec:
+1. **Lighthouse measures the mobile preset,** with its default simulated
+   throttling, three runs a page; a page passes when the median of its
+   three Performance scores is 95 or more.
+2. **It measures the public pages,** 06's `/` and 07's `/`, `/posts/1` and
+   `/login`, in a new step of CI's `browser` job,
+   `tests/browser/lighthouse.mjs`, on the Chromium the job installs for
+   Playwright, with `lighthouse@13.5.0` pinned.
+3. **`nova build --release` is fixed for a real `wasm-opt`,** and CI tests
+   it with binaryen itself.
+4. **07 gets no size limit;** its size stays printed. A KB is 1,024 bytes,
+   as the size tests measure, and the records say KiB.
+5. **Phase 4 is complete before the tag,** as Phase 2 was (ADR 0025): the
+   4.8 pull request, with the gate, the fix, the documentation and this
+   ADR; then the release pull request, with the version; then a records
+   pull request, with the publishes' run IDs. This gate needs nothing
+   published.
+6. **The user opens both examples from `main`,** by their READMEs, after
+   the 4.8 pull request merges and before the tag; after the release, the
+   published `nova` is tried again in place of the runbook's step 6.2.
+7. **`nova-bundler` stays a stub** (below).
+8. **The release also** runs `nova build --target wasm` in `gate.sh`; gives
+   the root README and `nova-cli`'s README a "Web" section; and publishes
+   the VS Code extension at 0.4.0 to both stores.
+
+## Decision
+
+### The inventory
+
+"Position N" is the master spec's Phase 4 list. "4.N" is a sub-phase of
+`docs/phase-4-plan.md`, and "decision N" one of its §3 decisions.
+"60-EXAMPLES §N" and "30-FRONTEND §N" are sections of those files in
+`nova-spec/`.
+
+| Promise | Source | Status | Record |
+|---|---|---|---|
+| A WASM backend, on `wasm-encoder` and `walrus` | position 1; decisions 1 to 3 | shipped differently: on `wasm-encoder` 0.243.0 alone, with its runtime written in Nova (`std/rt`) and a collector between turns | ADRs 0038, 0039, 0040 |
+| A bundler, `crates/nova-bundler`: bundle, tree-shake, code-split | position 2; decision 8 | shipped differently: the bundle is `nova-cli`'s, tree shaking is monomorphization from `main`, there is no code splitting, and `nova-bundler` is a stub | ADR 0043; decided here |
+| `std/dom`, bindings generated from `web-sys` descriptors | position 3 | shipped differently: a written subset over `extern "js"`, imported with `import dom` | ADRs 0038, 0041 |
+| `std/dom` on std's documentation site | the master spec's §7, item 3 | left out: the site documents native std; the READMEs point at its source | decided here |
+| `std/ui`: signals, effects, memos, components | position 4; decision 7 | shipped; a component is written with `component` and `view { … }` (RFC 0001) | ADRs 0042, 0043 |
+| `std/ui/html`, element builders | position 5 | shipped, imported with `import ui::html` | ADR 0042 |
+| `std/ui/router`, a client router | position 6 | shipped: `Router` by the hash, `HistoryRouter` by the history API, `link` | ADR 0044 |
+| `nova dev`, a dev server with HMR | position 7; decision 9 | shipped differently: a full build in a child process, and a reload over Server-Sent Events, with an overlay for the build's errors | ADRs 0038, 0044 |
+| SSR: render to a string natively, hydrate in WASM | position 8; decision 10 | shipped differently: `nova build --ssr` builds the project's own server, which renders with `render_hydratable`; `hydrate` adopts the server's nodes, and builds the view again on a mismatch | ADR 0045 |
+| SSG: compile-time route enumeration | position 9 | shipped differently: `nova build --ssg` runs the built server once, which writes a page for each route with no parameter | ADR 0045 |
+| The gate: both examples "work in browser" | the master spec's Phase 4 gate | met in Chromium, in CI's `browser` job; the user's own check is to come | "The gate's record" |
+| The gate: "pass Lighthouse 95+" | the master spec's Phase 4 gate; 60-EXAMPLES §7; 30-FRONTEND §11; decision 12 | met where 4.8 was built, every median 100; CI's run is to be recorded | decided here; "The gate's record" |
+| 06: `nova dev` opens a browser, and the counter responds to clicks | 60-EXAMPLES §6 | met: `nova dev --open` opens one, and CI's `browser` job clicks the counter under `nova dev` | ADR 0044 |
+| 06: under 50 KB gzipped | 60-EXAMPLES §6; 30-FRONTEND §11; decision 12 | met: 21,070 bytes where 4.8 was built, by a test that runs on all three systems | ADR 0043; "The gate's record" |
+| Hello world under 30 KB gzipped | 30-FRONTEND §11; decision 12 | met: 5,006 bytes where 4.8 was built, by a test that runs on all three systems | ADR 0039; "The gate's record" |
+| 07: pages that work, a login, the admin's create, edit and delete, sessions in cookies, CSRF tokens, no XSS | 60-EXAMPLES §7; decision 11 | shipped | ADR 0046 |
+| 07: SQLite, through `std/db` | 60-EXAMPLES §7 | replaced: JSON files, each written whole, then renamed into place | ADRs 0038, 0046 |
+| 07: hydration for the comments | 60-EXAMPLES §7 | shipped differently: every page hydrates whole; no islands | ADR 0046 |
+| 07: `routes/` and `components/` | 60-EXAMPLES §7 | shipped differently: a flat `src/` | ADR 0046 |
+| A CI job: Lighthouse, the gzipped sizes, Playwright | decision 12 | shipped: Playwright and Lighthouse in the `browser` job, the sizes in the `test` job on all three systems | "The gate's record" |
+| The user opens both examples in their own browser | decision 12 | to come, from `main`, after the 4.8 pull request merges | "The gate's record" |
+| std per target, and std modules a program imports | decisions 5 and 6 | shipped: `dom`, `ui`, `ui::html`, `ui::router` and, natively, `ui::ssr` | ADRs 0040, 0041, 0042, 0044, 0045 |
+| Async on the browser's event loop, with no `block_on` on WASM | decision 4 | shipped differently: `block_on` works, and holds the page while it waits | ADR 0040 |
+| `wasm-opt` under `--release`, never required | decision 2 | shipped in 4.4; fixed here for binaryen's own `wasm-opt` | ADR 0043; "Found outside this boundary" |
+| 4.0: the WASM target and the first programs under Node | 4.0 | shipped; its gate, as amended, met on all three systems | ADR 0039 |
+| 4.1: the language on WASM | 4.1 | shipped; its gate met: 94 of `tests/runtime/`'s 134 programs, and `examples/03-producer-consumer` | ADR 0040 |
+| 4.2: `std/dom` and the shim | 4.2 | shipped; its gate met under the fake DOM and in Chromium | ADR 0041 |
+| 4.3: `std/ui`, reactivity and element builders | 4.3 | shipped; its gate met under the fake DOM, in Chromium, and on both targets | ADR 0042 |
+| 4.4: component syntax and `06-counter-spa` | 4.4 | shipped; its gate met | ADR 0043 |
+| 4.5: the router and `nova dev` | 4.5 | shipped; its gate met in Chromium | ADR 0044 |
+| 4.6: SSR, hydration and SSG | 4.6 | shipped; its gate met in Chromium | ADR 0045 |
+| 4.7: `examples/07-fullstack-blog` | 4.7 | shipped; its gate met in Chromium, and over HTTP on all three systems | ADR 0046 |
+| 4.8: the CI job's Lighthouse run, and the size checks | 4.8 | shipped | this ADR |
+| 4.8: the user opens both examples | 4.8 | to come | "The gate's record" |
+| 4.8: the records | 4.8 | done: the READMEs' "Web" sections, the dated notes, `CHANGELOG.md` and this ADR | this ADR |
+| 4.8: `v0.4.0`, by the runbook | 4.8 | to come: the release pull request, then each publish on the user's word | the user's decision 5 |
+| 4.8: `nova-codegen-wasm` and `nova-bundler` published if `nova-cli` depends on them | 4.8 | decided: `nova-codegen-wasm` is in the publish list since 4.0, and first reaches crates.io with `v0.4.0`; nothing depends on `nova-bundler`, which stays unpublished | ADR 0039; decided here |
+| Examples 08 to 10 | 30-FRONTEND §12 | left out | ADR 0038; backlog |
+| Code splitting with `lazy()` | position 2; 30-FRONTEND §7.3 | left out | ADR 0038; backlog |
+| JS interop for programs: npm resolution, `@js_import`, `@js_export` | 30-FRONTEND §9 | left out: only std declares JS functions | ADRs 0038, 0041; backlog |
+| Source maps | 30-FRONTEND §8 | left out | ADRs 0038, 0044; backlog |
+| `axum` for the dev server | 30-FRONTEND §8 | left out: `nova dev` is written on `std::net` | ADRs 0038, 0044 |
+| Reference counting on WASM, or the WASM GC proposal | `nova-spec/13-RUNTIME.md` §8 | left out: mark-sweep between turns | ADRs 0038, 0040 |
+| `wasm-bindgen-test`, and visual regression snapshots | 30-FRONTEND §13 | left out: a fake DOM in `cargo test`, and Chromium in CI | ADRs 0038, 0041; backlog |
+| The LSP rows marked Phase 4: inlay hints, code lens, call hierarchy | `nova-spec/40-TOOLING.md` §3.1 | left out | ADR 0038; backlog |
+| Todo MVC under 100 KB, and the counter's time to interactive under 100 ms | 30-FRONTEND §11 | left out: there is no Todo MVC, and no measure of time to interactive | ADR 0038 (Todo MVC); backlog |
+
+**Status words:**
+- *Shipped:* a `v0.4.0` user can rely on it, within the limits its record
+  states; *shipped differently* when its record changed how.
+- *Replaced:* something else does its job, as its record says.
+- *Left out:* nothing implements it.
+- *Met:* a gate's checks passed, as the row and its record state them.
+  *Done* says the same of a step that is not a feature.
+- *To come:* a step of the release, which the release pull request or the
+  records pull request adds to this ADR.
+- *Backlog:* where a deferred item goes.
+- *Decided here:* a section below makes the decision.
+
+### Deviations no earlier record decides
+
+#### The bundler: decided, `nova-cli`'s; `nova-bundler` stays a stub
+
+Position 2 names a crate, `crates/nova-bundler`, that bundles,
+tree-shakes and splits code. The phase plan's §2 found it a one-line stub,
+`publish = false`, beside `nova-codegen-wasm`, and its decision 8 set the
+bundle's output. The 4.0 spec kept the crate "unpublished and empty until
+4.3"; 4.3 moved the bundler's pieces to 4.4, and ADR 0043 built them in
+`nova-cli`, in `crates/nova-cli/src/bundle.rs`: the project's page,
+`assets/` from `public/`, `manifest.json` and `wasm-opt`. No record since
+rules on the crate. Of position 2's three parts:
+- **the bundle** is `nova build --target wasm`'s, in `nova-cli`;
+- **tree shaking** is monomorphization from `main`: MIR holds only the
+  functions a program reaches (the plan's §2 item 2), and `std/rt`'s too
+  (ADR 0039, decision 2);
+- **code splitting** with `lazy()` is left out (ADR 0038).
+
+The phase plan's 4.8 entry publishes `nova-bundler` if `nova-cli` depends
+on it, and nothing does.
+
+**Decision (the user, 2026-10-22, decision 7 above):** position 2 is
+shipped differently, in `nova-cli`, as Phase 3's test runner is
+`nova-cli`'s while `nova-test` stays a stub (ADR 0037). `nova-bundler`
+stays a one-line stub, `publish = false`, and is not published; `v0.4.0`
+publishes the same 20 crates `publish-crates.sh` lists. Moving the bundle
+into the crate goes to the backlog.
+
+#### `std/dom` on std's site: decided, a pointer to its source
+
+The master spec's §7 asks for documentation of the new public surface.
+std's documentation site, which `nova doc --std` writes and each release
+attaches, documents the modules `std/lib.rs` lists in `ALL`, among them
+`ui`, `ui/html`, `ui/router` and `ui/ssr`. `std/dom` is not in `ALL`: only
+a WASM build compiles it, and the site is native std's (4.2's spec §3.2,
+as `std/lib.rs` says). Its 67 public items are documented in
+`std/dom/lib.nova` alone.
+
+**Decision (the 4.8 spec's §12, decision 3):** the "Web" sections of the
+root README and of `nova-cli`'s README name `std/dom`, say it is not on
+the site, and point at `std/dom/lib.nova`. `std/ui` and its modules, which
+the site documents, are the surface a program is meant to use. `std/dom`
+on the site goes to the backlog.
+
+#### "Lighthouse 95+": decided, a median of 95 or more over three mobile runs
+
+The master spec's gate reads "pass Lighthouse 95+". `60-EXAMPLES.md` §7
+asks 07 for "Lighthouse Performance > 95", and `30-FRONTEND.md` §11 for a
+"Lighthouse Performance score: > 95"; the phase plan's decision 12 fails
+the job "below 95 for Performance". The specs name no page, preset, number
+of runs or build, and two of them say more than 95 where the gate and the
+plan say 95 or more.
+
+**Decision (made here, for the user's review with this ADR, on the user's
+decisions 1 and 2 above):** the gate's "Lighthouse 95+" is
+`tests/browser/lighthouse.mjs` in CI's `browser` job:
+- Lighthouse 13.5.0, its mobile preset with the default simulated
+  throttling, the Performance category alone;
+- three runs on each public page, 06's `/` and 07's `/`, `/posts/1` and
+  `/login`; `/admin` needs the admin's session, and is left out;
+- debug builds, the ones the size tests measure and the other browser
+  steps drive (the 4.8 spec's §12, decision 1);
+- a page passes when the median of its three scores is 95 or more, and the
+  step fails when a run fails.
+
+The two specs' "> 95" is read as the gate's 95 or more. Every median
+measured 100 (below), which meets either reading.
+
+### Rows whose record already explains them
+
+These rows have no section above:
+- **The blog's JSON store:** the user decided on a JSON file in place of
+  SQLite on 2026-10-14 (ADR 0038), and ADR 0046 built it, with the users in
+  a file of their own; `60-EXAMPLES.md` §7's note of 2026-10-21 says so.
+- **`walrus` and `axum`:** ADR 0038 left both out, ADR 0039 builds the
+  backend on `wasm-encoder` alone, and ADR 0044 writes `nova dev` on
+  `std::net`. No crate depends on either.
+- **`std/dom`'s written subset:** ADR 0038 left generated bindings out,
+  and ADR 0041 reaches JS through `extern "js"` declarations only std may
+  write.
+- **`nova dev`'s reload:** the user chose live reload over HMR on
+  2026-10-14 (ADR 0038), and ADR 0044 records what departs from
+  `30-FRONTEND.md` §8.
+- **`block_on` on WASM:** the user's decision of 2026-10-15 (ADR 0040).
+- **SSR and SSG:** ADR 0045 records what departs from `30-FRONTEND.md`
+  §7.4 and the master spec's positions 8 and 9.
+- **07's flat `src/` and whole-page hydration:** the user's decisions of
+  2026-10-21 (ADR 0046).
+- **06's page:** its `index.html` loads `./app.js` and calls `init()`, and
+  its source uses `import ui`, `value()` and `mount("#app", Counter)`, as
+  `60-EXAMPLES.md` §6's note of 2026-10-17 records (ADR 0043).
+- **`nova-codegen-wasm` on crates.io:** ADR 0039 publishes it from the next
+  release; the 4.0 spec's §8 put it into the publish list, the 20th crate.
+- **The size checks:** hello world's limit of 30 KB and 06's of 50 KB are
+  KiB, 30,720 and 51,200 bytes, as their tests have measured them since
+  4.0 and 4.4, and 07 has no limit; both are the user's decision 4 above.
+
+### The gate's record
+
+**What runs.** CI's `browser` job, on `ubuntu-latest`, in this order:
+- **in Chromium, through Playwright** (`playwright@1.56.1`): `counter.mjs`
+  clicks 4.2's and 4.3's counters; `spa.mjs` clicks 06's; `dev.mjs` runs
+  06 under `nova dev`, edits it and shows the overlay; `router.mjs` and
+  `ssr.mjs` drive 4.5's and 4.6's test apps; and `blog.mjs` drives 07:
+  every page hydrates in place, the forms, the admin's create, edit and
+  delete, scripts that stay text, and a form without its token refused;
+- **the release builds:** with binaryen's `wasm-opt` 132 on `PATH` for
+  this step alone, 06 and 07 are built with `--release`; each `app.wasm`
+  must be smaller than the debug build's, and `spa.mjs` and `blog.mjs`
+  drive both builds;
+- **the rules' test:** `lighthouse.test.mjs` tests the rules
+  `lighthouse.mjs` applies, from `lighthouse-rules.mjs`, under
+  `node --test`, without Lighthouse: the four pages, three runs each, the
+  median of three, and a page failed by a median under 95, a failed run, a
+  run with no score, or other than three runs;
+- **Lighthouse** (`lighthouse.mjs`), as decided above, once the rules'
+  test has passed.
+
+The rules' test and Lighthouse run after a failure of the release builds
+too, though not after an earlier step's, so that neither the release
+builds' result nor Lighthouse's hides the other.
+
+The `test` job runs the size tests on Linux, macOS and Windows:
+`hello_world_is_under_30_kb_gzipped` and
+`the_counter_spa_is_under_50_kb_gzipped` in
+`crates/nova-cli/tests/wasm_tests.rs`, which gzip `app.wasm` and `app.js`
+each at level 9 and add the two.
+
+**Measured where 4.8 was built,** on 2026-10-22: the `browser` job's steps
+as the job then had them, run in order on a development container, Ubuntu
+24.04 on x86_64, with Node 22.22.0 and Playwright's Chromium 141, every
+step passing. The job then ran Lighthouse before the release builds, and
+had no step for the rules' test yet, so the Lighthouse lines come before
+the release builds' here. Lines are shortened where marked `…`, the two
+`blog:` lines and the scratch paths of the last two:
+
+```text
+counter: three clicks, and the count reads 3
+counter: three clicks, and the count reads 3
+counter-spa: +, +, +, -, and the count reads 2
+dev: counts at build 1792684328264.1, reloads into .2 on an edit, into .3 after a missed one, shows the overlay for the type error of .4 and keeps it over a reload, and counts again at .5
+router: a link to /posts/2 without a reload, back home, a reload at /posts/2 that stays at build 1792684333287.1, /nowhere not found, and a query read
+ssr: / hydrates in place (36 nodes kept + 1 joined) and counts, a link to /about without a reload, /posts/2 in place (26 + 1), /nowhere 404 in place (28 + 1), and SSG's /about/ arrives rendered and hydrates in place (28 + 1); the client is 41688 bytes gzipped
+blog: every page hydrates in place (nodes kept: / 37, /posts/1 48, /login 24, /nowhere 15, /posts/99 15, /admin 53, /admin/new 29, /admin/posts/2/edit 30), … the client is 49472 bytes gzipped
+06 /: scores 100, 100, 100, median 100; medians FCP 679 ms, LCP 1054 ms, TBT 0 ms, CLS 0.000, SI 679 ms; benchmark index 1796, 2397, 2228
+07 /: scores 100, 100, 100, median 100; medians FCP 903 ms, LCP 1803 ms, TBT 0 ms, CLS 0.000, SI 903 ms; benchmark index 2401, 2515, 2463
+07 /posts/1: scores 100, 100, 100, median 100; medians FCP 901 ms, LCP 1652 ms, TBT 6 ms, CLS 0.000, SI 901 ms; benchmark index 2122, 2486, 2258
+07 /login: scores 100, 100, 100, median 100; medians FCP 901 ms, LCP 1651 ms, TBT 0 ms, CLS 0.001, SI 901 ms; benchmark index 2342, 2062, 2325
+lighthouse: 06's / and 07's /, /posts/1 and /login, 3 runs each, mobile, every median 95 or more (06 / 100, 07 / 100, 07 /posts/1 100, 07 /login 100); Lighthouse 13.5.0, HeadlessChrome/141.0.0.0, Node 22.22.0
+wasm-opt version 132 (version_132)
+counter-spa: +, +, +, -, and the count reads 2; the client is 11974 bytes gzipped, from a release build that wasm-opt optimized
+blog: every page hydrates in place (nodes kept: / 37, /posts/1 48, /login 24, /nowhere 15, /posts/99 15, /admin 53, /admin/new 29, /admin/posts/2/edit 30), … the client is 27510 bytes gzipped, from a release build that wasm-opt optimized
+…/counter-spa-release/app.wasm: 16525 bytes, from 40491 in the debug build
+…/blog-release/fullstack-blog/dist/app.wasm: 57179 bytes, from 116579 in the debug build
+```
+
+**Measured again** while the plan was checked, on the same container: of
+the Lighthouse step's 94 runs that gave a score, 93 scored 100 and one, a
+run of 07's `/`, scored 93; every page's median was 100, as above. Runs
+vary; the median of three takes one low run in its stride.
+
+**The sizes,** `app.wasm` and `app.js` gzipped at level 9 by zlib, as
+`blog.mjs`, and `spa.mjs` under `--release`, measure them, and added, in
+bytes, with `app.wasm`'s raw size after it; "release" is `--release` with
+binaryen 132's `wasm-opt` on `PATH`. The size tests compress with
+`flate2`, whose output is a little larger: there, hello world's debug
+build is 5,006 bytes and 06's 21,070.
+
+| | debug | release | limit |
+|---|---|---|---|
+| hello world | 4,959 (4,120) | 3,658 (1,332) | 30 KiB, 30,720 |
+| 06's client | 20,997 (40,491) | 11,974 (16,525) | 50 KiB, 51,200 |
+| 07's client | 49,472 (116,579) | 27,510 (57,179) | none; printed |
+
+**The 4.8 pull request's CI run.** It had not run when this ADR was
+written. The release pull request adds its address here, with the lines
+its `browser` job printed: the Lighthouse step's page lines and its
+`lighthouse:` line, and the sizes the release builds' step printed.
+
+**The user's check.** To come: after the 4.8 pull request merges, the user
+builds both examples from `main`, by their READMEs, and opens each in
+their own browser. The release pull request adds what they report.
+
+**The publishes.** To come: the records pull request adds the runs of the
+tag's Release workflow, of "Publish to crates.io" and of "Release gate",
+the extension's two listings, and the published `nova` tried again on
+both examples, in place of the runbook's step 6.2.
+
+### The backlog
+
+The backlog is unscheduled. Each item is built when a program or a user
+needs it, and the list promises no date, no phase and no order.
+
+- **Left out of Phase 4 by ADR 0038:** examples 08 to 10; code splitting
+  with `lazy()`; npm resolution for `@js_import`, and user-level
+  `@js_import` and `@js_export` (ADR 0041 too); source maps, for a build
+  and under `nova dev` (ADR 0044 too); HMR that keeps state across a
+  change (ADR 0044 too); bindings generated from `web-sys` descriptors;
+  the WASM GC proposal; `wasm-bindgen-test`, and visual regression
+  snapshots; the LSP rows `40-TOOLING.md` §3.1 marks Phase 4 (inlay hints,
+  code lens and call hierarchy); SQLite, and a `std/db`.
+- **`std/ui` and its views** (ADRs 0042, 0043, 0045, 0046): a keyed
+  `for_each` (0042, 0045); `untrack`, context, resources and error
+  boundaries (0042); `if` and `for` in a view, fragments, and boolean
+  properties such as `checked` (0043); checking attributes during
+  hydration, partial hydration or islands, and streaming HTML with
+  asynchronous data in `render` (0045); refusing `javascript:` URLs,
+  preventing a submit, and `fetch` in shared components (0046).
+- **The language** (ADRs 0043, 0046): named arguments outside `view`
+  (0043); handlers that await, and async file access from a handler
+  (0046); nested module directories (0046, after ADR 0003).
+- **The router** (ADR 0044): nested routers and layouts; guards, lazy
+  routes and a base path for history mode; scroll restoration; an accessor
+  for the current location.
+- **`nova dev`** (ADRs 0044, 0045, 0046): a development mode for SSR, which
+  serves a server-rendered program (0044, 0045, 0046); an overlay for a
+  runtime panic; HTTPS, and access from other machines (`--host`);
+  incremental builds, in place of a full build in a child process (0044).
+- **SSR and SSG** (ADR 0045): pre-rendering routes with parameters, and a
+  list of paths a program gives; a `nova serve` command; `nova check` of
+  `src/server.nova` as a second root.
+- **`std/http`, `std/fs` and serving** (ADR 0046; the 4.8 spec's §11):
+  chunked bodies, `HEAD`, compression and environment variables (0046);
+  caching headers in `Dist` (4.8); `Secure` cookies by default, HTTPS, and
+  a limit on login attempts (0046); a sync before a rename, and file
+  modes, in `std/fs`, which the blog's README works around (0046).
+- **The blog** (ADR 0046): more than one admin through its pages;
+  Markdown; images; search; pages of posts.
+- **Decided here:** `std/dom` on std's documentation site; the bundle in
+  `crates/nova-bundler`; a measure of the counter's time to interactive,
+  which `30-FRONTEND.md` §11 sets under 100 ms.
+
+Taken from earlier backlogs:
+- **ADR 0037's "Phase 4":** `nova build --target wasm`, `nova bundle` and
+  `nova dev` are built; the LSP rows are left out by ADR 0038, and are on
+  this backlog.
+- **ADR 0025's "a router beyond exact paths and `GET`":** built in part,
+  by `Server::fallback` (ADR 0045) and routes by method with `:name`
+  patterns and `405` (ADR 0046). `use_middleware`, the `Handler` and
+  `Middleware` aliases, and the HTTP client stay on ADR 0025's backlog.
+
+### Found outside this boundary
+
+Met while closing Phase 4. The first is fixed here; the others are
+recorded, not decided.
+- **`nova build --release` failed with a real `wasm-opt`.** 4.4 passed it
+  `-Oz` alone, but the backend writes `memory.copy` and `memory.fill`,
+  which binaryen refuses unless bulk memory is enabled ("memory.fill
+  operations require bulk memory"): with binaryen's `wasm-opt` on `PATH`,
+  every release build for WASM failed. 4.4's test ran a stand-in
+  `wasm-opt` (its spec's §9.3), since neither its host nor CI had one. 4.8
+  passes `--enable-bulk-memory` too (`WASM_OPT_FLAGS` in
+  `crates/nova-cli/src/bundle.rs`), the stand-in's test expects it,
+  `nova-codegen-wasm`'s tests validate every module with the MVP and bulk
+  memory alone, so a module that needs another feature fails there, and
+  CI builds and drives both examples with binaryen 132.
+- **The phase plan's §5 asks for dated notes in `30-FRONTEND.md`,
+  `14-CODEGEN.md` and `13-RUNTIME.md` as each sub-phase lands.**
+  `30-FRONTEND.md` has them for 4.0 to 4.8, but `14-CODEGEN.md`'s last is
+  4.2's and `13-RUNTIME.md`'s 4.3's: 4.5's router state, a second
+  collector root natively and `std/rt`'s state word 112 on WASM, is
+  recorded in ADR 0044 and `30-FRONTEND.md` §6's note, not in
+  `13-RUNTIME.md`.
+- **The 4.8 spec says `v0.3.2` published 20 crates** (its §2, and its
+  §12, decision 7). It published 19 (ADR 0037's note of 2026-10-14);
+  `nova-codegen-wasm`, the 20th in `publish-crates.sh` since 4.0, was not
+  on crates.io on 2026-10-22, and first reaches it with `v0.4.0`, as the
+  runbook's note of 2026-10-14 in its step 3 says.
+
+## Consequences
+
+- The master spec's §7 Definition of Done, item by item:
+  1. the crates compile and pass CI: the 4.8 pull request's run, which the
+     release pull request records here;
+  2. the gate is met with reproducible commands: the `browser` job's
+     steps and the size tests, measured above where 4.8 was built, with
+     CI's run and the user's check to be recorded here by the release pull
+     request;
+  3. the new public surface is documented: the "Web" sections of the root
+     README and `nova-cli`'s, std's documentation site for `std/ui` and its
+     modules, `std/dom`'s own source, the examples' READMEs, and the dated
+     notes of `40-TOOLING.md` and `30-FRONTEND.md`;
+  4. `CHANGELOG.md`'s Unreleased section holds Phase 4, and the release
+     pull request makes it `[0.4.0]`;
+  5. ADRs 0038 to 0046 and this one record the deviations;
+  6. the tag `v0.4.0` is the release's.
+- What a `v0.4.0` user can rely on is the "shipped" rows, within the
+  limits their records state.
+- The backlog promises nothing.
+- Dated notes point here from:
+  - the master spec's Phase 4 gate;
+  - `nova-spec/40-TOOLING.md` §1.1 and `nova-spec/30-FRONTEND.md` §7, on
+    `--release` and `wasm-opt`;
+  - `nova-spec/60-EXAMPLES.md` §6 and §7, with each example's Lighthouse
+    results;
+  - `docs/phase-4-plan.md`'s 4.8 entry;
+  - `docs/releasing.md`'s step 6.2, which `v0.4.0` replaces.
+- The guides are corrected in place instead, because they describe the
+  code as it is now: `agent.md`'s rows for `nova-codegen-wasm` and
+  `nova-bundler`, `skill.md`'s WASM and frontend rows, and the examples'
+  READMEs. Of the runbook's lines that were wrong for `v0.4.0`, the version
+  `main` has and the 21 lines the bump changes in `Cargo.toml` are
+  corrected in place, and the upload's time and step 6.2 get dated notes.
+- `CHANGELOG.md`'s Unreleased section gains the Lighthouse step, `gate.sh`'s
+  WASM build, the "Web" documentation, and the fix to `--release`.
+- The release pull request adds to this ADR the 4.8 pull request's CI run
+  and the user's check; the records pull request adds the publishes' runs
+  and the published `nova`'s check.
+
+**Tagging `v0.4.0` completes Phase 4 within this boundary.**
+
+## References
+
+- Spec: `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+- `nova-spec/00-MASTER-SPEC.md` §3 and §7; `docs/phase-4-plan.md`;
+  `nova-spec/60-EXAMPLES.md` §6 and §7; `nova-spec/30-FRONTEND.md`
+- `docs/releasing.md`
+- ADRs 0025, 0037, and 0038 to 0046
+- Pull requests #117 to #124
+- `tests/browser/lighthouse.mjs`, `lighthouse-rules.mjs` and
+  `lighthouse.test.mjs`, `.github/workflows/ci.yml`'s `browser` job,
+  `.github/scripts/gate.sh`, and `crates/nova-cli/tests/wasm_tests.rs`
diff --git a/docs/phase-4-plan.md b/docs/phase-4-plan.md
index c01995825e39559c486cd4bed2b14a8ad3a81126..74bec8f071fa2685332fac353b2fed0ee2b67952 100644
--- a/docs/phase-4-plan.md
+++ b/docs/phase-4-plan.md
@@ -642,6 +642,30 @@ ADR 0046).
     first.
   - Every publish waits for the user's word.
 
+**Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`):** by the
+user's decisions of 2026-10-22, Lighthouse runs in a new step of CI's
+`browser` job, `tests/browser/lighthouse.mjs`, on Playwright's Chromium with
+`lighthouse@13.5.0` pinned: its mobile preset, with the default simulated
+throttling, three runs on each of 06's `/` and 07's `/`, `/posts/1` and
+`/login`, and a page passes when the median of its Performance scores is 95
+or more. Debug builds are measured, as the size checks measure them. The
+size checks stand: hello world under 30 KB and 06 under 50 KB, a KB being
+1,024 bytes, and 07's size printed with no limit. `nova build --release`
+passes `--enable-bulk-memory` to `wasm-opt` as well as `-Oz`, since
+binaryen refuses the backend's `memory.copy` and `memory.fill` without it,
+and CI builds and drives both examples with binaryen's own `wasm-opt`;
+`gate.sh` builds its project for WASM too. Phase 4 is complete before the
+tag, as Phase 2 was: the 4.8 pull request holds the gate, the fix, the
+documentation and ADR 0047; the user then opens both examples from `main`,
+by their READMEs; the release pull request makes the version `0.4.0`; and a
+records pull request adds the publishes' run IDs. Of the two crates above,
+`nova-codegen-wasm` is in the 20 `publish-crates.sh` lists since 4.0, and
+first reaches crates.io with `v0.4.0`; nothing depends on `nova-bundler`,
+which stays a stub, and the bundle stays `nova-cli`'s. The extension is
+published at 0.4.0 to both stores
+(`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`;
+ADR 0047).
+
 ## 5. Cross-cutting
 
 - **Testing.**
diff --git a/docs/releasing.md b/docs/releasing.md
index 462c422b9ef8048ff0981960a7a0f83bace8957f..ae76ad91e6f250fec2cd602715dde60344302ddf 100644
--- a/docs/releasing.md
+++ b/docs/releasing.md
@@ -124,7 +124,9 @@ A pull request whose last commit is the release commit:
    From a checkout of `main`, on a new branch:
 
    ```sh
-   old=0.2.0   # the version main has now
+   # the version main has now
+   old=$(perl -ne 'print $1 if /^version = "(.*)"$/' Cargo.toml)
+   echo "$old"
    perl -pi -e "s/^version = \"\Q$old\E\"\$/version = \"$version\"/; s/version = \"=\Q$old\E\"/version = \"=$version\"/" Cargo.toml
    perl -pi -e "s/^  \"version\": \"\Q$old\E\",\$/  \"version\": \"$version\",/" tools/vscode-nova/package.json
    perl -0pi -e "s/(\"name\": \"vscode-nova\",\n\s*\"version\": )\"\Q$old\E\"/\$1\"$version\"/g" tools/vscode-nova/package-lock.json
@@ -133,7 +135,7 @@ A pull request whose last commit is the release commit:
    ```
 
    `cargo update --workspace` moves only the workspace's own 23 packages
-   in `Cargo.lock` (add `--offline` without a network). The diff shows 20
+   in `Cargo.lock` (add `--offline` without a network). The diff shows 21
    lines changed in `Cargo.toml`, 23 in `Cargo.lock`, 1 in `package.json`
    and 2 in `package-lock.json`. `npm version` is not used: it rewrites
    `package.json`'s layout. Then:
@@ -324,6 +326,12 @@ nothing published.
    next release adds one crate, `nova-codegen-wasm`
    (`docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md`
    §8), so its token keeps **publish-new** as well.
+
+   **Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`):**
+   `v0.4.0` is that release: one new crate, `nova-codegen-wasm`, and 19
+   new versions of crates that exist. The new crate falls within the
+   burst of 5, so its upload takes minutes, as `0.3.1`'s and `0.3.2`'s
+   did, not the 2 h 20 min of 19 new crates above.
 5. Check what crates.io serves:
 
    ```sh
@@ -504,6 +512,16 @@ data and extensions directories.
 - **With it,** and after **Nova: Restart Language Server**, completion
   after `p.` offered the fields `x` and `y`.
 
+**Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`):**
+step 5 below cannot run again: `shapes` 0.1.0 is in the index, where a
+published version stays (section 5). `0.3.1` and `0.3.2` ran the Release
+gate and an install of the extension from a store in place of this run.
+For `v0.4.0`, by the user's decision of 2026-10-22, the maintainer installs
+the published `nova` and builds and opens both Phase 4 examples with it,
+by their READMEs, in place of this run (spec
+`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`,
+decision 6; `docs/adr/0047-phase-4-boundary.md`).
+
 The commands, in order, each with what it should show:
 
 1. Install from crates.io; time it.
diff --git a/examples/06-counter-spa/README.md b/examples/06-counter-spa/README.md
index 7245cf6682c4dd808ba580387313d40809d5ff13..b60f03f053fd9f6385158a69163c047560ddfe40 100644
--- a/examples/06-counter-spa/README.md
+++ b/examples/06-counter-spa/README.md
@@ -40,9 +40,11 @@ away: three clicks of `+` and one of `-` leave `Count: 2`.
 - It differs from `nova-spec/60-EXAMPLES.md` §6 in three places, each the
   API 4.3 built: `import ui` and `import ui::html::{button, div}`,
   `count.value()`, and `mount("#app", Counter)`.
-- `nova build --target wasm --release` runs `wasm-opt -Oz` on `app.wasm`
-  when `wasm-opt` is on `PATH`; the gate's size, under 50 KB of `app.wasm`
-  and `app.js` gzipped, is measured without it.
+- `nova build --target wasm --release` runs binaryen's `wasm-opt -Oz
+  --enable-bulk-memory` on `app.wasm` when `wasm-opt` is on `PATH`, which
+  takes `app.wasm` and `app.js` from 20,997 bytes gzipped (by zlib at
+  level 9) to 11,974 with binaryen 132; the gate's size, under 50 KiB, is
+  measured without it.
 - `cargo test` clicks it under a fake DOM, and CI's `browser` job in
   Chromium (`tests/browser/spa.mjs`); the 4.4 spec is
   `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`.
@@ -50,3 +52,8 @@ away: three clicks of `+` and one of `-` leave `Count: 2`.
   counts, an edit reloads it, and a type error shows the overlay until the
   fix; the 4.5 spec is
   `docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`.
+- The same job runs Lighthouse on its page three times, mobile, and needs
+  a median Performance score of 95 or more
+  (`tests/browser/lighthouse.mjs`), and drives its `--release` build with
+  binaryen's `wasm-opt`; the 4.8 spec is
+  `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`.
diff --git a/examples/07-fullstack-blog/README.md b/examples/07-fullstack-blog/README.md
index c6cf3d50c0f09aa7a09af7ca311b07c7489757c6..9c14898f2e0619e7d5d4e2b8ece8b9db705ab102 100644
--- a/examples/07-fullstack-blog/README.md
+++ b/examples/07-fullstack-blog/README.md
@@ -149,3 +149,10 @@ to delete it, and `Write a new post` opens the editor.
   (`run_tests.rs`); CI's `browser` job drives it in Chromium
   (`tests/browser/blog.mjs`). The 4.7 spec is
   `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`.
+- The same job runs Lighthouse three times, mobile, on `/`, `/posts/1` and
+  `/login`, and needs a median Performance score of 95 or more for each
+  (`tests/browser/lighthouse.mjs`). It also builds the blog with
+  `nova build --ssr --release` and binaryen's `wasm-opt` on `PATH`, which
+  takes the client from 49,472 bytes gzipped (by zlib at level 9) to
+  27,510 with binaryen 132, and drives that build; the 4.8 spec is
+  `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`.
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 85a2d7d5af63e3ff781d883869fe8949ab15fd72..19fdf2d0f6ec200b3155f5faca4606ca9ced65c1 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -709,6 +709,21 @@ hashes, by the user's decisions of 2026-10-21 (ADR 0046), and a JSON file in
 place of SQLite (ADR 0038). CI's `browser` job drives it in Chromium; the
 Lighthouse run, for both examples, is 4.8's.
 
+**Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`):**
+Phase 4's boundary is `docs/adr/0047-phase-4-boundary.md`. Its inventory
+takes Phase 4's promises from the list above, `docs/phase-4-plan.md` and
+`nova-spec/60-EXAMPLES.md` §6 and §7, and says whether each shipped,
+shipped differently or was left out, and where each is recorded; position
+2's bundler is `nova-cli`'s, and `crates/nova-bundler` stays a stub. The
+gate's "Lighthouse 95+" is a step of CI's `browser` job,
+`tests/browser/lighthouse.mjs`: three runs of Lighthouse's mobile preset on
+each of 06's `/` and 07's `/`, `/posts/1` and `/login`, each page's median
+Performance score 95 or more, by the user's decisions of 2026-10-22. Where
+4.8 was built, every median was 100. The job's other steps drive both
+examples in Chromium, and the user opens both from `main` before the tag;
+the release pull request records CI's run and the user's check in that
+ADR. Tagging `v0.4.0` completes Phase 4 within that boundary.
+
 ---
 
 ### Phase 5 — Self-hosting (week 81–104)
diff --git a/nova-spec/30-FRONTEND.md b/nova-spec/30-FRONTEND.md
index 14bf823b0f49f58f53918ad1795c1941e74d7e59..948d8a4e93af8fda70169a7bdb004a2c81c5e325 100644
--- a/nova-spec/30-FRONTEND.md
+++ b/nova-spec/30-FRONTEND.md
@@ -336,6 +336,18 @@ fails. An output directory that holds the project or lies inside `public/`
 is refused, and a link at a name the build writes, symbolic or, on Unix,
 hard, is removed first (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §7; ADR 0043).
 
+**Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`):** step 4
+of §7.2 runs `wasm-opt -Oz --enable-bulk-memory`, since the backend writes
+`memory.copy` and `memory.fill`, which binaryen's `wasm-opt` refuses
+without bulk memory: 4.4's `-Oz` alone failed every build with binaryen on
+`PATH`. With binaryen 132, `06-counter-spa`'s `app.wasm` and `app.js` come
+to 11,974 bytes gzipped by zlib at level 9, from 20,997, and
+`07-fullstack-blog`'s client to 27,510, from 49,472. §11's Lighthouse
+target is met as ADR 0047 reads it, a median Performance score of 95 or
+more over three mobile runs, measured in CI's `browser` job
+(`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+§3, §4; ADR 0047).
+
 ### 7.1 Output Layout
 ```
 dist/
diff --git a/nova-spec/40-TOOLING.md b/nova-spec/40-TOOLING.md
index a235961f9fb6dc60151b8bc14e5ecabd102c9079..3cfd641337db1cd65e0a4ba9907e4ee8c73fbb65 100644
--- a/nova-spec/40-TOOLING.md
+++ b/nova-spec/40-TOOLING.md
@@ -122,6 +122,24 @@ src/server.nova` runs the server
 (`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
 §7; ADR 0045).
 
+**Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`):** the
+2026-10-14 note's refusal is gone. Since 4.4, `nova build --target wasm
+--release` builds: it runs `wasm-opt` on `app.wasm` when `wasm-opt` is on
+`PATH`, says so in one line when it is not, fails when `wasm-opt` fails, and
+`manifest.json` says whether it ran
+(`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
+§7; ADR 0043). Since 4.8 it runs `wasm-opt -Oz --enable-bulk-memory app.wasm
+-o app.wasm` in the output directory: the backend writes `memory.copy` and
+`memory.fill`, which binaryen's `wasm-opt` refuses without the flag, so with
+binaryen on `PATH` every such build failed, where 4.4's test had run a
+stand-in. `--ssr` and `--ssg` with `--release` do the same for the client.
+CI's `browser` job builds both Phase 4 examples that way, with binaryen 132,
+and drives them in Chromium; nothing installs `wasm-opt` for a user. `nova
+build --help` says `--release` optimizes `app.wasm` for size with binaryen's
+`wasm-opt`
+(`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+§4; ADR 0047).
+
 ### 1.2 Project Template (`nova new`)
 
 ```
diff --git a/nova-spec/60-EXAMPLES.md b/nova-spec/60-EXAMPLES.md
index 06f7411066a832cc5723e954ead61a408a9cade2..2a2cd7c2dcebadbec7f88b82b5933fe640c48a67 100644
--- a/nova-spec/60-EXAMPLES.md
+++ b/nova-spec/60-EXAMPLES.md
@@ -669,6 +669,23 @@ restores the file and sees the overlay go and the page count again
 as `wasm_tests.rs` measures them
 (`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md` §5.4; ADR 0044).
 
+**Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`):** the
+Phase 4 gate's Lighthouse run measures this example in CI's `browser` job
+(`tests/browser/lighthouse.mjs`): built with `nova build --target wasm`, a
+debug build, and served from a static server, `/` gets three runs of
+Lighthouse 13.5.0's mobile preset, with its simulated throttling, for the
+Performance category alone, and passes when the median of the three scores
+is 95 or more. Where 4.8 was built, its median was 100: the prototype's
+three runs each scored 100, with median FCP 679 ms, LCP 1,054 ms, TBT 0 ms
+and CLS 0, and so did each of this page's runs when the step was run again.
+The size limit is 50 KiB, 51,200 bytes, as `wasm_tests.rs` has measured it
+since 4.4: 21,070 bytes there, by `flate2`, and 20,997 by zlib at the same
+level. With `--release` and binaryen's `wasm-opt` on `PATH`, `app.wasm`
+drops from 40,491 bytes to 16,525, and the client to 11,974 by zlib;
+CI drives that build too
+(`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+§3, §4; ADR 0047).
+
 ---
 
 ## 7. `07-fullstack-blog` (Phase 4 gate)
@@ -738,6 +755,25 @@ refused with `403`. Lighthouse is 4.8's. Its client, built without
 (`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
 §6; ADR 0046).
 
+**Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`):**
+"Lighthouse Performance > 95" is measured in CI's `browser` job
+(`tests/browser/lighthouse.mjs`), with the bar the master spec's gate sets,
+a median of 95 or more (ADR 0047): built with `nova build --ssr`, a debug
+build, its server started on port 0 with the admin's password set, `/`,
+`/posts/1` and `/login` each get three runs of Lighthouse 13.5.0's mobile
+preset, with its simulated throttling, for the Performance category alone.
+`/admin` needs the admin's session and is not measured. Where 4.8 was
+built, each page's median was 100: the prototype's runs all scored 100,
+with median LCP 1,803 ms for `/`, 1,652 ms for `/posts/1` and 1,651 ms for
+`/login`, TBT at most 6 ms, and CLS at most 0.001; when the step was run
+again, one run of `/` scored 93. The example has no size limit, by the
+user's decision of 2026-10-22, and `blog.mjs` prints its client's size.
+With `nova build --ssr --release` and binaryen's `wasm-opt` on `PATH`, its
+`app.wasm` drops from 116,579 bytes to 57,179, and the client from 49,472
+bytes gzipped by zlib to 27,510; CI drives that build through `blog.mjs` too
+(`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
+§3, §4; ADR 0047).
+
 ---
 
 ## 8. Additional Examples (post-Phase 6)
diff --git a/skill.md b/skill.md
index cc9e268b71d190fe8f8cef1ab786d75bca187187..0c852b9d75f4fe446664f26b10d9f2c208da77a2 100644
--- a/skill.md
+++ b/skill.md
@@ -114,10 +114,10 @@ the budget (`docs/adr/0029-the-language-server.md`).
 
 | Skill | Notes |
 |---|---|
-| `wasm-encoder` | Low-level WASM binary encoding |
-| `walrus` | Higher-level WASM IR and transforms |
+| `wasm-encoder` | Low-level WASM binary encoding; there is no `walrus` (ADR 0039) |
+| `wasmparser` | Validating the modules tests build |
 | WebAssembly spec | Modules, types, memory model, tables |
-| JS interop | ABI for calling JS from WASM and vice versa |
+| JS interop | The JS shim, and `extern "js"` declarations in std (ADRs 0039, 0041) |
 
 ---
 
@@ -204,11 +204,10 @@ assets (`docs/adr/0031-package-index-and-publishing.md`).
 | Skill | Notes |
 |---|---|
 | Signals / fine-grained reactivity | SolidJS model — no virtual DOM |
-| DOM bindings | `web-sys`-style descriptor bindgen |
-| SSR / SSG | Render-to-string on native, hydrate on WASM |
-| Bundler / tree-shaking | Module graph, dead-code elimination |
-| HMR (hot module replacement) | Dev server WebSocket protocol |
-| `wasm-bindgen` concepts | ABI between WASM and JS |
+| DOM bindings | A written subset, `std/dom`, over `extern "js"` (ADR 0041) |
+| SSR / SSG | Hydratable HTML rendered natively, hydrated on WASM (ADR 0045) |
+| Bundler / tree-shaking | `nova-cli`'s bundle; monomorphization keeps only what `main` reaches (ADR 0047) |
+| Live reload | `nova dev`'s rebuilds, and Server-Sent Events to the page (ADR 0044) |
 
 ---
 
````

### Task 5: Verification, the mutants, the review, the PR, and the merge

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 8G
  free; if not, remove `target/debug/incremental` and scratch directories
  of earlier sub-phases.

  ```sh
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo run --locked -q -p nova-cli -- fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-48x.txt" 2>&1
  grep '^test result' "$P/full-48x.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-48x.txt"
  W kb_gzipped --nocapture
  ```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with
    no warning or error;
  - the suite, in about 4 minutes: `2420 1 10`, the baseline's figures
    and the one test 4.8 adds, `release_manifests`'s
    `the_release_gate_passes_node_to_a_gate_sh_that_takes_it` (Task 3);
    4.8 also changes `release_runs_wasm_opt_when_it_is_on_path` and the
    codegen's `valid()`, which 19 of its library's 34 tests call. A
    comparison of the two runs' `test` lines by name, binary by binary,
    finds that one added and none removed. A comparison of the names alone may list
    `io::tests::a_successful_stdout_write_reports_its_byte_count`: the
    runtime's io tests write without a line feed, so a run can print that
    name after a program's output, on its line. It is the same test, not a
    new one;
  - the one failure is Task 0's IPv6 test;
  - the sizes: `2 passed`, printing `hello world: 5006 bytes gzipped` and
    `06-counter-spa: 21070 bytes gzipped`, by `flate2` at level 9, against
    30,720 and 51,200, as at the base: debug builds, which 4.8 does not
    change. Record them in the ledger.
- [ ] **Step 2: The lockfile.** `git diff --numstat f5ed8b3 -- Cargo.lock`.
  Expected: nothing.
- [ ] **Step 3: Rust 1.78.**

  ```sh
  RUSTFLAGS="-D warnings" CARGO_TARGET_DIR="$P/msrv-48" cargo +1.78 check --locked --workspace
  ```

  Expected: clean, in a minute or so (the replay's two runs took 72 and 32
  seconds). It checks the libraries and binaries, not the tests, as 4.6's
  and 4.7's did: `assert_cmd` 2.2.1's manifest does not parse on 1.78. Then
  `rm -rf "$P/msrv-48"`.
- [ ] **Step 4: The mutants.** One at a time, each edit is made, its command
  run, and the file restored with `git checkout -- <file>`.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - A mutant of `crates/` that a `ci_step` drives has its command build
    `nova` first; once its file is restored, run `cargo build --locked -p
    nova-cli -p nova-runtime` again, so the next mutant's
    `target/debug/nova` is the tree's. A `W` or `cargo test` command
    rebuilds what it needs.
  - The mutants of `lighthouse-rules.mjs` that run "The Lighthouse gate's
    rules" take a second each, those that run actionlint less, those that
    run `release_manifests` a few seconds, and those that run
    `lh_stand_in` about five (Task 2's stand-in; write its script first,
    as Conventions say); those that run Lighthouse's step take about two
    minutes, and two of them six.
  - Two are not run here: binaryen left out of CI's install, which runs in
    CI alone, and the rules' step without its `if:`, since nothing here
    evaluates GitHub's conditions.
  - A `ci_step` or `gate_run` command needs Task 2's runner, whose debug
    builds the release step's `shrank` reads, and Task 3's
    `$P/gate-48/bin/nova`: if either is gone, run Task 2's Step 2 or Task
    3's Step 2 again first.
  - "Hangs" means the command is still running after six minutes, twice
    the step's time: stop it, and every process it started (`kill -- -<its
    process group>`), and check with `pgrep -af fullstack-blog-server` that
    no server is left.
  - Each caught mutant must fail as written: its tests, or its command's
    status other than 0 with the text named. A mutant that passes is a
    gap. The replay's gaps are marked below, each with what it shows:
    record each in the ledger, name it in the PR, and give it to the
    review (Step 6), which says whether a check is added. Any other mutant
    that passes is a new gap: record it the same way.
  - At the end, `git status --short` prints nothing.

  1. **`--enable-bulk-memory` dropped.** `crates/nova-cli/src/bundle.rs`:
     `const WASM_OPT_FLAGS: [&str; 2] = ["-Oz", "--enable-bulk-memory"];` becomes `const WASM_OPT_FLAGS: [&str; 1] = ["-Oz"];`.
     - Run `W release_runs_wasm_opt_when_it_is_on_path`.
     - Expected: fails: `release_runs_wasm_opt_when_it_is_on_path`, "wasm-opt: -Oz app.wasm -o app.wasm".
  2. **`-Oz` dropped.** `crates/nova-cli/src/bundle.rs`:
     `const WASM_OPT_FLAGS: [&str; 2] = ["-Oz", "--enable-bulk-memory"];` becomes `const WASM_OPT_FLAGS: [&str; 1] = ["--enable-bulk-memory"];`.
     - Run `W release_runs_wasm_opt_when_it_is_on_path`.
     - Expected: fails: `release_runs_wasm_opt_when_it_is_on_path`, "wasm-opt: --enable-bulk-memory app.wasm -o app.wasm".
  3. **The flags in the other order.** `crates/nova-cli/src/bundle.rs`:
     `const WASM_OPT_FLAGS: [&str; 2] = ["-Oz", "--enable-bulk-memory"];` becomes `const WASM_OPT_FLAGS: [&str; 2] = ["--enable-bulk-memory", "-Oz"];`.
     - Run `W release_runs_wasm_opt_when_it_is_on_path`.
     - Expected: fails: `release_runs_wasm_opt_when_it_is_on_path`, "wasm-opt: --enable-bulk-memory -Oz app.wasm -o app.wasm".
  4. **`--enable-bulk-memory` dropped, under binaryen's own `wasm-opt`.** `crates/nova-cli/src/bundle.rs`:
     `const WASM_OPT_FLAGS: [&str; 2] = ["-Oz", "--enable-bulk-memory"];` becomes `const WASM_OPT_FLAGS: [&str; 1] = ["-Oz"];`.
     - Run `cargo build --locked -p nova-cli -p nova-runtime && ci_step "Release builds with binaryen's wasm-opt"`.
     - Expected: exits 1, its output holding `memory.fill operations require bulk memory`:

       ```text
       [wasm-validator error in function 3] unexpected false: memory.fill operations require bulk memory [--enable-bulk-memory-opt], on
       Fatal: error validating input
       Error: wasm-opt failed: exit status: 1
       ```
  5. **`wasm_opt` says it did not run when it ran.** `crates/nova-cli/src/bundle.rs`:
     `Ok(s) if s.success() => Ok(true),` becomes `Ok(s) if s.success() => Ok(false),`.
     - Run `cargo build --locked -p nova-cli -p nova-runtime && ci_step "Release builds with binaryen's wasm-opt"`.
     - Expected: exits 1, its output holding `says release true and wasm_opt false`:

       ```text
       Error: dist/manifest.json says release true and wasm_opt false, not a release build that wasm-opt optimized
       ```
  6. **`valid()` takes the MVP alone.** `crates/nova-codegen-wasm/src/lib.rs`:
     `let features = wasmparser::WasmFeatures::MVP | wasmparser::WasmFeatures::BULK_MEMORY;` becomes `let features = wasmparser::WasmFeatures::MVP;`.
     - Run `cargo test -p nova-codegen-wasm`.
     - Expected: fails: `tests::memory_accesses_lower_in_line`, "a valid module: BinaryReaderError { inner: BinaryReaderErrorInner { message: "bulk memory support is not enabled", kind: Custom, offset: 110, needed_hint: None } }".
  7. **The backend writes a sign-extension instruction, `i64.extend32_s`.** `crates/nova-codegen-wasm/src/lib.rs`:
     replace

     ```
                     // -1, "cannot grow", stays -1.
                     self.out.push(I::I64ExtendI32S);
     ```

     with

     ```
                     // -1, "cannot grow", stays -1.
                     self.out.push(I::I64ExtendI32S);
                     self.out.push(I::I64Extend32S);
     ```

     - Run `cargo test -p nova-codegen-wasm`.
     - Expected: fails: `tests::memory_accesses_lower_in_line`, "a valid module: BinaryReaderError { inner: BinaryReaderErrorInner { message: "sign extension operations support is not enabled", kind: Custom, offset: 137, needed_hint: None } }".
  8. **The Node floor raised to 22.23.** `tests/browser/lighthouse.mjs`:
     `if (major < 22 || (major === 22 && minor < 19)) {` becomes `if (major < 22 || (major === 22 && minor < 23)) {`.
     - Run `ci_step "Lighthouse on both examples"`.
     - Expected: exits 1, its output holding `Lighthouse 13 needs Node 22.19 or later, and this is Node 22.`:

       ```text
       Error: Lighthouse 13 needs Node 22.19 or later, and this is Node 22.22.0
       ```
  9. **The threshold 101.** `tests/browser/lighthouse-rules.mjs`:
     `const PASS = 95;` becomes `const PASS = 101;`.
     - Run `ci_step "The Lighthouse gate's rules"`.
     - Expected: exits 1, its output holding `not ok 1 - four pages, 06's / and 07's /, /posts/1 and /login, three runs each, 95 to pass`:

       ```text
       not ok 1 - four pages, 06's / and 07's /, /posts/1 and /login, three runs each, 95 to pass
       not ok 3 - a page passes on a median of 95 or more, a low run among its three
       not ok 4 - a median under 95 fails the page
       not ok 7 - two runs, or four, fail the page, however high they scored
       ```
  10. **The median read past the scores' end.** `tests/browser/lighthouse-rules.mjs`:
      `[Math.floor(values.length / 2)]` becomes `[values.length]`.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 2 - the median of three is the middle score`:

        ```text
        not ok 2 - the median of three is the middle score, in any order, and leaves them as they are
        not ok 3 - a page passes on a median of 95 or more, a low run among its three
        not ok 4 - a median under 95 fails the page
        not ok 7 - two runs, or four, fail the page, however high they scored
        ```
  11. **One run a page.** `tests/browser/lighthouse-rules.mjs`:
      `const RUNS = 3;` becomes `const RUNS = 1;`.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 1 - four pages, 06's / and 07's /, /posts/1 and /login, three runs each, 95 to pass`:

        ```text
        not ok 1 - four pages, 06's / and 07's /, /posts/1 and /login, three runs each, 95 to pass
        not ok 3 - a page passes on a median of 95 or more, a low run among its three
        not ok 4 - a median under 95 fails the page
        not ok 6 - a run that gave no score fails the page, and so does a page with no runs
        not ok 7 - two runs, or four, fail the page, however high they scored
        ```
  12. **A page whose run failed reports no failure.** `tests/browser/lighthouse-rules.mjs`:
      ``if (failures.length > 0) return { line: `${line}; benchmark index ${bench}`, failures, score: null };`` becomes ``if (failures.length > 0) return { line: `${line}; benchmark index ${bench}`, failures: [], score: null };``.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 5 - a failed run fails the page`:

        ```text
        not ok 5 - a failed run fails the page, whatever the other two scored, its line keeping the benchmark indexes
        not ok 6 - a run that gave no score fails the page, and so does a page with no runs
        ```
  13. **The score read from the accessibility category.** `tests/browser/lighthouse-rules.mjs`:
      `const score = lhr?.categories?.performance?.score;` becomes `const score = lhr?.categories?.accessibility?.score;`.
      - Run `ci_step "Lighthouse on both examples"`.
      - Expected: exits 1, its output holding `06 /'s run 1 failed (no Performance score)`:

        ```text
        lighthouse: failed, 06's / and 07's /, /posts/1 and /login, 3 runs each, mobile: 06 /'s run 1 failed (no Performance score); 06 /'s run 2 failed (no Performance score); 06 /'s run 3 failed (no Performance score); 07 /'s run 1 failed (no Performance score); 07 /'s run 2 failed (no Performance score); 07 /'s run 3 failed (no Performance score); 07 /posts/1's run 1 failed (no Performance score); 07 / …
        ```
  14. **`/posts/99`, a `404`, measured in place of `/posts/1`.** `tests/browser/lighthouse-rules.mjs`:
      `["07 /posts/1", "07-post", new URL("/posts/1", blog).href],` becomes `["07 /posts/1", "07-post", new URL("/posts/99", blog).href],`.
      - Run `ci_step "Lighthouse on both examples"`.
      - Expected: exits 1, its output holding `07 /posts/1's run 1 failed`:

        ```text
        07 /posts/1: scores failed, failed, failed; benchmark index 2158, 2541, 2405
        lighthouse: failed, 06's / and 07's /, /posts/1 and /login, 3 runs each, mobile: 07 /posts/1's run 1 failed (Lighthouse exited with 1); 07 /posts/1's run 2 failed (Lighthouse exited with 1); 07 /posts/1's run 3 failed (Lighthouse exited with 1)
        ```
  15. **`/login` left out of the pages.** `tests/browser/lighthouse-rules.mjs`:
      `["07 /login", "07-login", new URL("/login", blog).href],`: the line goes.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 1 - four pages, 06's / and 07's /, /posts/1 and /login, three runs each, 95 to pass`:

        ```text
        not ok 1 - four pages, 06's / and 07's /, /posts/1 and /login, three runs each, 95 to pass
        ```
  16. **The blog's server not stopped.** `tests/browser/lighthouse.mjs`:
      `server?.kill();`: the line goes.
      - Run `ci_step "Lighthouse on both examples"`.
      - Expected: hangs: the step's lines are printed, the `lighthouse:` line the last, and the step does not end; stop it after six minutes (the replay stopped it at 360 s).
  17. **06's static server not stopped.** `tests/browser/lighthouse.mjs`:
      `statics?.server.close();`: the line goes.
      - Run `ci_step "Lighthouse on both examples"`.
      - Expected: hangs: the step's lines are printed, the `lighthouse:` line the last, and the step does not end; stop it after six minutes (the replay stopped it at 360 s).
  18. **`spa.mjs` does not take `--release`.** `tests/browser/spa.mjs`:
      `const release = process.argv[4] === "--release";` becomes `const release = false;`.
      - Run `ci_step "Release builds with binaryen's wasm-opt"`.
      - Expected: exits 1, its output holding `Error: usage: node tests/browser/spa.mjs`:

        ```text
        Error: usage: node tests/browser/spa.mjs <dist> <dir holding node_modules/playwright> [--release]
        ```
  19. **`blog.mjs --release` builds without `--release`.** `tests/browser/blog.mjs`:
      `const flags = release ? ["--ssr", "--release"] : ["--ssr"];` becomes `const flags = ["--ssr"];`.
      - Run `ci_step "Release builds with binaryen's wasm-opt"`.
      - Expected: exits 1, its output holding `says release false and wasm_opt false`:

        ```text
        Error: dist/manifest.json says release false and wasm_opt false, not a release build that wasm-opt optimized
        ```
  20. **`blog.mjs --release` runs the server from `target/debug/`.** `tests/browser/blog.mjs`:
      `const PROFILE = release ? "release" : "debug";` becomes `const PROFILE = "debug";`.
      - Run `ci_step "Release builds with binaryen's wasm-opt"`.
      - Expected: exits 1, its output holding `ENOENT`:

        ```text
        Error: spawn <RUNNER_TEMP>/blog-release/fullstack-blog/target/debug/fullstack-blog-server ENOENT
        ```
  21. **CI builds 06 without `--release` in the release step.** `.github/workflows/ci.yml`:
      `(cd examples/06-counter-spa && ../../target/debug/nova build --target wasm --release -o "$RUNNER_TEMP/counter-spa-release")` becomes `(cd examples/06-counter-spa && ../../target/debug/nova build --target wasm -o "$RUNNER_TEMP/counter-spa-release")`.
      - Run `ci_step "Release builds with binaryen's wasm-opt"`.
      - Expected: exits 1, its output holding `says release false and wasm_opt false`:

        ```text
        Error: dist/manifest.json says release false and wasm_opt false, not a release build that wasm-opt optimized
        ```
  22. **binaryen's `.bin` not put on `PATH`.** `.github/workflows/ci.yml`:
      `export PATH="$RUNNER_TEMP/pw/node_modules/.bin:$PATH"`: the line goes.
      - Run `ci_step "Release builds with binaryen's wasm-opt"`.
      - Expected: exits 127, its output holding `wasm-opt: command not found`:

        ```text
        bash: line 1: wasm-opt: command not found
        ```
  23. **`shrank` compares the other way.** `.github/workflows/ci.yml`:
      `test "$release" -lt "$debug"` becomes `test "$debug" -lt "$release"`.
      - Run `ci_step "Release builds with binaryen's wasm-opt"`.
      - Expected: exits 1, its output holding `bytes, from`:

        ```text
        <RUNNER_TEMP>/counter-spa-release/app.wasm: 16525 bytes, from 40491 in the debug build
        ```
  24. **binaryen left out of the install.** `.github/workflows/ci.yml`:
      `playwright@1.56.1 lighthouse@13.5.0 binaryen@132.0.0` becomes `playwright@1.56.1 lighthouse@13.5.0`.
      - **Not run here:** the install step runs in CI alone: nothing here installs a package, and the replay's npm prefix stands in for it; in CI, `wasm-opt --version` would fail, `.bin` holding no `wasm-opt`.
  25. **The `dist/` check inverted.** `.github/scripts/gate.sh`:
      `if [ ! -s "dist/$file" ]; then` becomes `if [ -s "dist/$file" ]; then`.
      - Run `gate_run`.
      - Expected: exits 1, its output holding `gate: nova build --target wasm wrote no dist/index.html`:

        ```text
        gate: nova build --target wasm wrote no dist/index.html
        ```
  26. **The magic number `0061736e`.** `.github/scripts/gate.sh`:
      `if [ "$magic" != "0061736d" ]; then` becomes `if [ "$magic" != "0061736e" ]; then`.
      - Run `gate_run`.
      - Expected: exits 1, its output holding `gate: dist/app.wasm begins with 0061736d, not WASM's magic number`:

        ```text
        gate: dist/app.wasm begins with 0061736d, not WASM's magic number
        ```
  27. **The Node run's comparison inverted.** `.github/scripts/gate.sh`:
      `if [ "$wasm" != "Hello, Nova!" ]; then` becomes `if [ "$wasm" = "Hello, Nova!" ]; then`.
      - Run `gate_run`.
      - Expected: exits 1, its output holding `gate: the WASM build printed 'Hello, Nova!' under Node`:

        ```text
        gate: the WASM build printed 'Hello, Nova!' under Node
        ```
  28. **The Node run skipped where `node` is on `PATH`.** `.github/scripts/gate.sh`:
      `if command -v node > /dev/null; then` becomes `if false; then`.
      - Run `gate_run`.
      - Expected: exits 1, its output holding `gate: --node, but there is no node on PATH to run the WASM build`:

        ```text
        gate: --node, but there is no node on PATH to run the WASM build
        ```
  29. **A run that gave no score does not fail its page.** `tests/browser/lighthouse-rules.mjs`:
      ``if (typeof r.score !== "number") return [`${name}'s run ${i + 1} failed (no Performance score)`];``: the line goes.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 6 - a run that gave no score fails the page`:

        ```text
        not ok 6 - a run that gave no score fails the page, and so does a page with no runs
        ```
  30. **A median that is not a number does not fail its page.** `tests/browser/lighthouse-rules.mjs`:
      ``if (typeof score !== "number") failures.push(`${name} has no median`);``: the line goes.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 6 - a run that gave no score fails the page`:

        ```text
        not ok 6 - a run that gave no score fails the page, and so does a page with no runs
        ```
  31. **Lighthouse's exit status not read.** `tests/browser/lighthouse-rules.mjs`:
      `if (done.code !== 0) {` becomes `if (false) {`.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 8 - a run fails when Lighthouse exits other than 0`:

        ```text
        not ok 8 - a run fails when Lighthouse exits other than 0, leaves no report, or reports an error or no score
        ```
  32. **A report's runtime error not read.** `tests/browser/lighthouse-rules.mjs`:
      `} else if (lhr.runtimeError) {` becomes `} else if (false) {`.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 8 - a run fails when Lighthouse exits other than 0`:

        ```text
        not ok 8 - a run fails when Lighthouse exits other than 0, leaves no report, or reports an error or no score
        ```
  33. **A missing report not checked.** `tests/browser/lighthouse-rules.mjs`:
      `} else if (lhr === null) {` becomes `} else if (false) {`.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 8 - a run fails when Lighthouse exits other than 0`:

        ```text
        not ok 8 - a run fails when Lighthouse exits other than 0, leaves no report, or reports an error or no score
        ```
  34. **The measuring loop makes two runs a page.** `tests/browser/lighthouse.mjs`:
      `for (let n = 1; n <= RUNS; n++) {` becomes `for (let n = 1; n < RUNS; n++) {`.
      - Run `lh_stand_in`.
      - Expected: exits 1, its output holding `stand-in: NOT four pages, each scoring 40 in three runs`:

        ```text
        stand-in: NOT four pages, each scoring 40 in three runs
        stand-in: NOT the failing lighthouse: line
        lighthouse: failed, 06's / and 07's /, /posts/1 and /login, 3 runs each, mobile: 06 / had 2 runs, not 3; 06 /'s median is 40, under 95; 07 / had 2 runs, not 3; 07 /'s median is 40, under 95; 07 /posts/1 had 2 runs, not 3; 07 /posts/1's median is 40, under 95; 07 /login had 2 runs, not 3; 07 /login's median is 40, under 95
        ```
  35. **A failing verdict exits 0.** `tests/browser/lighthouse.mjs`:
      replace

      ```
          console.log(`lighthouse: failed, ${what}: ${failures.join("; ")}`);
          process.exitCode = 1;
      ```

      with

      ```
          console.log(`lighthouse: failed, ${what}: ${failures.join("; ")}`);
      ```

      - Run `lh_stand_in`.
      - Expected: exits 1, its output holding `stand-in: NOT exit 1`:

        ```text
        stand-in: NOT exit 1
        ```
  36. **Runs other than three in number pass.** `tests/browser/lighthouse-rules.mjs`:
      ``if (runs.length !== RUNS) failures.push(`${name} had ${runs.length} runs, not ${RUNS}`);``: the line goes.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 7 - two runs, or four, fail the page`:

        ```text
        not ok 6 - a run that gave no score fails the page, and so does a page with no runs
        not ok 7 - two runs, or four, fail the page, however high they scored
        ```
  37. **A failed page's line leaves out the benchmark indexes.** `tests/browser/lighthouse-rules.mjs`:
      ``if (failures.length > 0) return { line: `${line}; benchmark index ${bench}`, failures, score: null };`` becomes `if (failures.length > 0) return { line, failures, score: null };`.
      - Run `ci_step "The Lighthouse gate's rules"`.
      - Expected: exits 1, its output holding `not ok 5 - a failed run fails the page`:

        ```text
        not ok 5 - a failed run fails the page, whatever the other two scored, its line keeping the benchmark indexes
        ```
  38. **A failure before the verdict prints no lighthouse: line.** `tests/browser/lighthouse.mjs`:
      ``console.log(`lighthouse: failed, before its verdict: ${oneLine(e?.message ?? e)}`);``: the line goes.
      - Run `lh_stand_in`.
      - Expected: exits 1, its output holding `stand-in: NOT a failing lighthouse: line, with its reason`:

        ```text
        stand-in: NOT a failing lighthouse: line, with its reason
        stand-in: NOT a failing lighthouse: line, with the build's reason
        ```
  39. **A failure before the verdict exits 0.** `tests/browser/lighthouse.mjs`:
      replace

      ```
        process.exitCode = 1;
      } finally {
      ```

      with

      ```
      } finally {
      ```

      - Run `lh_stand_in`.
      - Expected: exits 1, its output holding `stand-in: NOT exit 1 with no Lighthouse`:

        ```text
        stand-in: NOT exit 1 with no Lighthouse
        stand-in: NOT exit 1 with a nova that fails at once
        ```
  40. **The Release gate's crates.io job passes no --node.** `.github/workflows/release-gate.yml`:
      `bash .github/scripts/gate.sh "$RUNNER_TEMP/crates-io/bin/nova" "$RUNNER_TEMP/gate" "${flags[@]}"` becomes `bash .github/scripts/gate.sh "$RUNNER_TEMP/crates-io/bin/nova" "$RUNNER_TEMP/gate" --test`.
      - Run `cargo test -p nova-cli --test release_manifests`.
      - Expected: fails: `the_release_gate_passes_node_to_a_gate_sh_that_takes_it`, "release-gate.yml: a step with no run of gate.sh with the flags".
  41. **The Release gate's install-script job passes no --node.** `.github/workflows/release-gate.yml`:
      `bash .github/scripts/gate.sh "$INSTALLED" "$RUNNER_TEMP/gate" "${flags[@]}"` becomes `bash .github/scripts/gate.sh "$INSTALLED" "$RUNNER_TEMP/gate" --test`.
      - Run `cargo test -p nova-cli --test release_manifests`.
      - Expected: fails: `the_release_gate_passes_node_to_a_gate_sh_that_takes_it`, "release-gate.yml: a step with no run of gate.sh with the flags".
  42. **`gate.sh`'s `--node)` line reformatted, so the Release gate's grep misses it.** `.github/scripts/gate.sh`:
      `--node) with_node=--node ;;` becomes `--node)  with_node=--node ;;`.
      - Run `cargo test -p nova-cli --test release_manifests`.
      - Expected: fails: `the_release_gate_passes_node_to_a_gate_sh_that_takes_it`, "<repository>/crates/nova-cli/../../.github/scripts/gate.sh: no line holds "--node) with_node=--node", which release-gate.yml greps for".
  43. **The Release gate's install-script step does not print its flags.** `.github/workflows/release-gate.yml`:
      replace

      ```
                echo "gate.sh ${flags[*]}"
                bash .github/scripts/gate.sh "$INSTALLED"
      ```

      with

      ```
                bash .github/scripts/gate.sh "$INSTALLED"
      ```

      - Run `cargo test -p nova-cli --test release_manifests`.
      - Expected: fails: `the_release_gate_passes_node_to_a_gate_sh_that_takes_it`, "release-gate.yml: a step with no echo of the flags".
  44. **The release step's `id` renamed, which the two `if:`s name.** `.github/workflows/ci.yml`:
      `id: release` becomes `id: release-builds`.
      - Run `"$P/actionlint-dl/actionlint"`.
      - Expected: exits 1, its output holding `property "release" is not defined`:

        ```text
        .github/workflows/ci.yml:203:43: property "release" is not defined in object type {release-builds: {conclusion: string; outcome: string; outputs: {string => string}}} [expression]
        ```
  45. **The rules' test without its `if:`, so a failure of the release builds skips it and Lighthouse.** `.github/workflows/ci.yml`:
      `if: success() || (!cancelled() && steps.release.outcome == 'failure')`: the line goes.
      - **Not run here:** GitHub evaluates `if:` on its runners alone: `ci_step.py` runs every step whatever its condition, and actionlint accepts a step with none, so nothing here fails; the order of the steps is unchecked the same way.
  46. **A child's broken stdin not handled.** `tests/browser/lighthouse.mjs`:
      `child.stdin.on("error", () => {});`: the line goes.
      - Run `lh_stand_in`.
      - Expected: exits 1, its output holding `stand-in: NOT a failing lighthouse: line, with the build's reason`:

        ```text
        stand-in: NOT a failing lighthouse: line, with the build's reason
        ```
  47. **A failed build's line without its reason.** `tests/browser/lighthouse.mjs`:
      ``return new Error(`${step}: ${reason(stderr, output)}`);`` becomes `return new Error(step);`.
      - Run `lh_stand_in`.
      - Expected: exits 1, its output holding `stand-in: NOT a failing lighthouse: line, with the build's reason`:

        ```text
        lighthouse: failed, before its verdict: nova build --target wasm of 06 exited with 2
        stand-in: NOT a failing lighthouse: line, with the build's reason
        ```
  48. **The reason taken from the last line, not the `error` line.** `tests/browser/lighthouse.mjs`:
      `return lines.find((line) => /^error/i.test(line)) ?? lines.at(-1) ?? "no output";` becomes `return lines.at(-1) ?? "no output";`.
      - Run `lh_stand_in`.
      - Expected: exits 1, its output holding `stand-in: NOT a failing lighthouse: line, with the build's reason`:

        ```text
        lighthouse: failed, before its verdict: nova build --target wasm of 06 exited with 2: a last line, not the reason
        stand-in: NOT a failing lighthouse: line, with the build's reason
        ```

- [ ] **Step 5: The review's inputs.** The full run's figures and the sizes,
  as Step 1 measured them, Task 2's Lighthouse lines and release sizes,
  Task 3's gate lines, and the mutants' outcome, with each gap, go in the
  ledger.
- [ ] **Step 6: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0047 and this plan, and is told to hunt for:
    - a flag the backend needs and `WASM_OPT_FLAGS` lacks, or one it
      passes for nothing; an instruction beyond the MVP and bulk memory in
      `nova-codegen-wasm`, which `valid()` would refuse; `wasm_opt`'s
      status read wrongly into `manifest.json` (decisions 11 and 12);
    - Lighthouse run otherwise than decided: a preset, a category, a flag,
      a page or a run count; a median that is not the middle of three, or
      not a number, passing; a threshold other than 95; a failed run, a
      stopped run or a report without a score that does not fail the step;
      a server left running, or the step ending before every run ran; a
      rule of `lighthouse-rules.mjs` that `lighthouse.test.mjs` would let
      pass, or a test that no wrong rule fails; a run count other than
      three passing; a failure, before the verdict or in it, that exits 0
      or prints no `lighthouse:` line, or one without its reason, which the
      stand-in checks, a `nova` that exits at once among its runs
      (decisions 8, 9, 15, 16, 29 and 30);
    - `spa.mjs` or `blog.mjs` passing a release build that `wasm-opt` did
      not optimize, or a debug build under `--release`; either changing
      what it prints without `--release` (decisions 13 and 14);
    - CI: a package unpinned or installed elsewhere; binaryen's `.bin` on
      `PATH` beyond its step; `shrank` reading a path no earlier step
      wrote; a step that fails without failing the job; the job's Node
      below 22.19; the two `if:`s, which must run the rules' test and
      Lighthouse after a failure of the release builds, and not after an
      earlier step's or in a cancelled run, and which nothing here
      evaluates (decisions 15 and 17);
    - `gate.sh`: a check that cannot fail, or one that fails on Windows'
      Git Bash or on macOS; `--node` and `--test` parsed wrongly, or a
      caller in CI that should pass `--node` and does not; the Release
      gate's `grep`, which must match `v0.4.0`'s `gate.sh` and not an older
      tag's, and `release_manifests.rs`'s test of it, which must fail when
      the two drift apart (decision 31);
    - ADR 0047: an inventory row without its record or with the wrong
      status; a backlog item missing, or without its ADR; the deviations
      it decides against the user's decisions and the spec's §12; the
      crate counts (decision 7);
    - a record that says what the code does not: the "Web" sections, the
      dated notes, `agent.md`, `skill.md`, the examples' READMEs, the
      CHANGELOG, the runbook's lines; a size without its compressor
      (decision 25); "every run" scoring 100, where the replay saw one run
      at 93 (Checked on);
    - the five gaps of the mutants' first run and the tests that closed
      them, and any mutant of Step 4 that passes;
    - MSRV 1.78, and tests that touch the internet or use port 3000.
  - Each finding is fixed in one commit, `4.8: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Step 1 runs again, and Task 2's Step 2 if a script or
    `ci.yml` changed.
- [ ] **Step 7: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-48`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 4.8 adds: the Lighthouse step, the fix to `--release` and its
      test with binaryen in CI, `gate.sh` on WASM, the "Web" sections and
      ADR 0047, which calls Phase 4 complete before the tag;
    - the gate's evidence: the replay's Lighthouse lines, every median
      100; the release builds' sizes, 06's client 11,974 bytes gzipped
      from 20,997 and 07's 27,510 from 49,472; the size tests' 5,006 and
      21,070 bytes by `flate2`, against 30,720 and 51,200;
    - the user's eight decisions, and this plan's decisions, numbered as
      here, with 7, 11, 14, 15, 18 to 26 and 29 to 32 marked as differing
      from the spec, 7 and 25 among them as corrections to its §2;
    - that `Cargo.lock` is unchanged, and that Task 6's release pull
      request changes only the workspace's own versions in it;
    - the mutants, the five gaps the first run found and the tests that
      closed them, the fact-check's seven findings and the second check's
      eight, and what came of each, and the review's findings and what
      came of each.
  - Read CI once it ends. Expected: green on all three systems.
    - The `browser` job: its release step printing `wasm-opt version 132`
      and both `shrank` lines; its rules step `# pass 8` and `# fail 0`;
      and its Lighthouse step a line per page, with its benchmark indexes,
      and a `lighthouse:` line, every median 95 or more.
    - The `install` and `package` jobs' `gate.sh --node` printing `gate:
      the WASM build prints Hello, Nova! under Node` on each system.
    - `release.yml`'s run on the pull request, since its `on:` paths name
      `.github/scripts/**` and `crates/nova-cli/Cargo.toml`, which this
      pull request changes: it builds the four targets, and its smoke test
      runs `gate.sh --node` on three of them, without publishing.

    Record the runs' addresses, the Lighthouse lines with their benchmark
    indexes, and the sizes in the ledger: Task 6's release pull request adds
    them to ADR 0047.
  - **If Lighthouse's step fails, or its medians differ from the replay's
    (Task 2),** record its lines and benchmark indexes, and the local ones
    beside them, then stop and ask the user. Run the job again only on
    their word, and never change `PASS` or `RUNS` without it: a slower
    runner scores lower (simulated throttling slows the host's own CPU by
    a fixed factor), and the benchmark indexes tell a slow runner from a
    slow page. If it fails on Node's version, the job's `node-version`
    needs a minor: stop and ask the user.
- [ ] **Step 8: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main
    --stat`, which prints nothing: the merged tree is the PR's.
  - Then read CI's run on `main` once it ends: green, as Step 7's. Its
    `browser` job runs Lighthouse again, and Step 7's rule holds for it:
    if it fails, or its medians differ from the replay's, record its lines
    and benchmark indexes, stop and ask the user, run it again only on
    their word, and never change `PASS` or `RUNS` without it.

---

## The release

### Task 6: The release `v0.4.0`, by `docs/releasing.md`

Spec §7; the user's decisions 5, 6 and 8; decisions 6, 7 and 24. After
Task 5's merge, by the runbook as `main` then has it, whose sections this
task cites rather than copies; read each before its step. Every step the
runbook marks **[stop]**, every merge and the user's own check wait for the
user's explicit word, named below as **[stop]**. Between two stops is
preparation that can be redone.

- [ ] **Step 1: The user's check (decision 6).** **[stop]** After the 4.8
  pull request merges, the user builds and runs both examples from
  `main`, by `examples/06-counter-spa/README.md` and
  `examples/07-fullstack-blog/README.md`, and opens each in their own
  browser. Wait for their word that both work; record what they report,
  in their words, in the ledger. A failure stops the release: it is a
  4.8 fix, by its own pull request.
- [ ] **Step 2: The release branch.** From an up-to-date `main`:

  ```sh
  git fetch origin main
  git checkout -b claude/loving-archimedes-frtf5q-48-release origin/main
  version=0.4.0
  ```

- [ ] **Step 3: The release commit** (the runbook's §1, items 1 to 4).
  - **The version** (§1, item 1): its block of commands, as written there,
    with `cargo update --workspace --offline`, then `git diff --numstat`.
    Expected: `echo "$old"` prints `0.3.2`; `git diff --stat`, which counts
    a changed line once removed and once added, shows 46 for `Cargo.lock`,
    42 for `Cargo.toml`, 4 for `tools/vscode-nova/package-lock.json` and 2
    for `tools/vscode-nova/package.json`; and `git diff --numstat` prints

    ```text
    23	23	Cargo.lock
    21	21	Cargo.toml
    2	2	tools/vscode-nova/package-lock.json
    1	1	tools/vscode-nova/package.json
    ```

    the runbook's 21 lines of `Cargo.toml` (the version and the 20 pins),
    23 of `Cargo.lock`, 2 of `package-lock.json` and 1 of `package.json`,
    each changed in place. The replay ran the block on a scratch copy of
    the split's last tree. **`Cargo.lock` changes in its workspace's own
    packages alone** (Global Constraints):

    ```sh
    git diff -U0 Cargo.lock | grep '^[-+][^-+]' | sort | uniq -c
    git diff -U1 Cargo.lock | grep '^ name = ' | sort
    ```

    print `23 +version = "0.4.0"` and `23 -version = "0.3.2"`, and 23
    names, each a package of this workspace (`cargo metadata --no-deps
    --format-version 1` lists them). Anything else in the lock stops the
    task: ask the user. Then §1's `cargo test --locked -p nova-cli --test
    release_manifests --test vscode_extension`. Expected: `8 passed` and
    `3 passed`.
  - **`CHANGELOG.md`** (§1, item 2): `## [Unreleased]` becomes `## [0.4.0]
    - <the date>`, under a lead paragraph in the model of `[0.2.0]`'s:
    Phase 4, "Frontend / WASM", is complete, within the boundary
    `docs/adr/0047-phase-4-boundary.md` records; this is the `v0.{phase}.0`
    milestone; both gate examples work in Chromium, and their Lighthouse
    medians are the ones CI's run of the 4.8 pull request printed (Task 5,
    Step 7), quoted as printed, not the replay's; what the ADR leaves out
    is on its backlog. Phase 4's
    gate needs nothing published, so the paragraph says the phase is
    complete, not that its gate follows (§1, item 2's note of 2026-10-12
    does not apply; the user's decision 5). A fresh, empty `##
    [Unreleased]` goes above it.
  - **`tools/vscode-nova/CHANGELOG.md`** (§1, item 3, and its note of
    2026-10-14): a `## [0.4.0] - <the date>` section above `[0.3.2]`'s,
    with a real entry (the user's decision 8): the extension's grammar
    colours `component` before a component's name and `view` before its
    `{` as keywords (4.4's `9b44c1b`), and `nova lsp` 0.4.0, which it
    starts, gives them semantic tokens and hover; read `git log --oneline
    v0.3.2..HEAD -- tools/vscode-nova crates/nova-lsp` (fetch the tag
    first) for anything else, and write only what changed.
  - **ADR 0047** (§1, item 4; decision 24): "The gate's record" gains the
    4.8 pull request's CI run, its address with the Lighthouse step's page
    lines, their medians and benchmark indexes as CI printed them, its
    `lighthouse:` line and the release step's sizes, from Task 5's Step 7,
    and the user's check from Step 1; the inventory's rows "to
    come" for both, and the Consequences' items 1 and 2, say they are
    recorded.
  - Then the checks, as the conventions list them, and `cargo build
    --locked --workspace`, which ends with `Finished`; `target/debug/nova
    --version` prints `nova 0.4.0`.
  - Commit, as the runbook's model commits are named: `release: v0.4.0,
    Phase 4 complete`, its message written as the conventions say.
- [ ] **Step 4: The release pull request** (the runbook's §1, its last
  paragraphs). Push the branch and open the pull request to `main`; its
  body names the version, the ADR, the four files of the bump and the lock's
  23 version lines, and no local path. Wait until CI is green, "Packaged
  nova" (the 20 crates at `0.4.0`, `gate.sh --node` on WASM among its
  checks) and `release.yml` without its publish job among it. Its
  `browser` job runs Lighthouse again: if it fails, or its medians differ
  from those ADR 0047 now quotes, record its lines and benchmark indexes,
  stop and ask the user, as in Task 5's Step 7; run it again only on their
  word, and never change `PASS` or `RUNS` without it. **[stop]** Merge with
  **Rebase and merge**, on the user's word; then wait for CI on `main` to
  pass on the merged commits. Its `browser` job runs Lighthouse too, under
  the same rule: if it fails, or its medians differ from those ADR 0047
  quotes, record its lines and benchmark indexes, stop and ask the user,
  rerun only on their word, and never change `PASS` or `RUNS` without it;
  the tag waits.
- [ ] **Step 5: The tag and the GitHub release** (the runbook's §2).
  **[stop]** The user makes the tag and pushes it, on `main`'s CHANGELOG
  commit, once CI on `main` has passed (§2's note of 2026-10-14). This
  session's credential cannot push a tag: `v0.3.0`'s push was refused with
  HTTP 403, and the user pushed `v0.3.1` and `v0.3.2`. Give them these
  commands in full, from a checkout of the repository whose `origin` is
  `https://github.com/Sakeerin/nova.git` (`v0.3.0`'s first push went to
  another remote; push to that address if `origin` is not it):

  ```sh
  git fetch origin main
  git log -1 --format='%H %s' origin/main
  test "$(git log -1 --format=%s origin/main)" = "release: v0.4.0, Phase 4 complete" &&
    git tag -a v0.4.0 -m "Nova v0.4.0 — Phase 4 (Frontend / WASM)" -m "Phase 4 is complete, within the boundary docs/adr/0047-phase-4-boundary.md records." "$(git rev-parse origin/main)" &&
    git push origin v0.4.0
  git ls-remote --tags origin v0.4.0
  ```

  The second command must print the release commit of Step 3, `release:
  v0.4.0, Phase 4 complete`, as rebased onto `main`; the third makes and
  pushes the tag only when `origin/main`'s subject is that, and otherwise
  makes none, and the last then prints nothing: stop, and ask. The
  message is the `v0.2.0` model's, for a phase's release. Once the user says it is
  pushed, check it yourself: `git ls-remote --tags origin v0.4.0` names a
  tag object, and the Actions list a Release run on the push of `v0.4.0`.
  Then follow `release.yml` and check the release as §2 says: eight assets
  and `SHA256SUMS`, named for `0.4.0`, every line of the check `OK`, and
  `releases/latest` redirecting to `…/releases/tag/v0.4.0`. Record the
  run's address. The runbook's §2 gives the same commands, with
  `version` and `commit` variables and without the `ls-remote`, and names
  no one to push; it stands for `v0.4.0`.
- [ ] **Step 6: crates.io** (the runbook's §3, and its note of 2026-10-22
  in item 4).
  - **[stop]** Item 1, the token: **publish-new** and **publish-update**,
    since `nova-codegen-wasm` is new (decision 7).
  - **[stop]** Item 2, the environment's secret.
  - **[stop]** Item 3, the workflow from the tag `v0.4.0`, version
    `0.4.0`, and its deployment approved under "Review deployments".
  - Item 4. Expected: `nova-codegen-wasm: not on crates.io` and each of
    the 19 others `on crates.io, owned by <logins>`; the upload within
    minutes, `nova-codegen-wasm` inside the burst of 5; one line per
    crate at the end, 20 published.
  - Item 5, and the same for the new crate: `curl -sS
    https://index.crates.io/no/va/nova-cli | tail -n 1` and `curl -sS
    https://index.crates.io/no/va/nova-codegen-wasm | tail -n 1` each name
    `"vers":"0.4.0"`.
  - Item 6, whatever the result: revoke the token and delete the secret.
  - Record the run's address.
- [ ] **Step 7: The extension** (the runbook's §4). Its block of commands,
  which downloads `nova-vscode-0.4.0.vsix` and checks its sum.
  - **[stop]** Item 1, the Marketplace (or the manage page's upload, as
    §4's note of 2026-10-14 records).
  - **[stop]** Item 2, Open VSX.
  - Item 3: both listings show 0.4.0, the README, the CHANGELOG's
    `[0.4.0]` and the links. Revoke the Azure DevOps token.
- [ ] **Step 8: The gate** (the runbook's §6). §5, the index, is the first
  release's only.
  - **[stop]** §6.1: start "Release gate" with version `0.4.0`. Expected:
    all six jobs pass. `v0.4.0`'s `gate.sh` takes `--node`, so both of the
    gate's steps pass it, and each `gate.sh` run prints `gate: the WASM
    build prints Hello, Nova! under Node` or fails (decision 31); read one
    log to see the flag taken. Record the run's address.
  - §6.2, by its note of 2026-10-22 (the user's decision 6): **[stop]**
    the user installs the published `nova` (`cargo install nova-cli
    --locked`, or the release's install script) on a machine of their
    choosing, and builds and opens both examples with it, by their
    READMEs. Record their report in the ledger. §6.2's step 5, publishing
    `shapes`, does not run again.
- [ ] **Step 9: The records pull request** (spec §7, item 5).
  - From an up-to-date `main`, on
    `claude/loving-archimedes-frtf5q-48-records`: ADR 0047's "The gate's
    record" gains the runs of Step 5 (the Release workflow), Step 6
    ("Publish to crates.io") and Step 8 ("Release gate"), the extension's
    two listings, and the user's check of the published `nova`, in place of
    the runbook's step 6.2; its rows "to come" say they are done.
    `CHANGELOG.md`'s `[0.4.0]` section gains the same run IDs, in the
    model of the records `0.3.1` and `0.3.2` added (ADR 0037's notes of
    2026-10-14).
  - The checks, as the conventions list them; commit `docs: v0.4.0's
    records`; push; open the pull request; wait for CI. Its `browser` job
    runs Lighthouse again, under Task 5's rule: if it fails, or its medians
    differ from those ADR 0047 quotes, record its lines and benchmark
    indexes, stop and ask the user, rerun only on their word, and never
    change `PASS` or `RUNS` without it; the same after the merge, for CI
    on `main`.
  - **[stop]** Merge by rebase, on the user's word. Then `git fetch origin
    main` and `git diff <head> origin/main --stat`, which prints nothing.
