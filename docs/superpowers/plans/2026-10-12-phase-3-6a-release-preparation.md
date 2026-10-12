# Phase 3.6a, "Release preparation", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** everything `v0.3.0` needs is ready, and nothing is published.
The 19 crates `nova` needs carry one workspace version, exact pins on each
other, the real repository, a README and both licence texts; the other four
are `publish = false`. A CI job packages the 19 on three systems and
installs `nova` from the packages as crates.io would. A workflow that only
the maintainer starts publishes them in order, and can resume. Install
scripts for Linux, macOS and Windows, attached to every release, check the
archive against `SHA256SUMS`, and `release.yml` runs both against its own
archives. The extension gets its store fields, the gate a workflow, and the
release a runbook. `nova new`'s templates are written as `nova fmt` writes
them, so a new project passes `nova fmt --check`. `Cargo.lock` does not
change.

**Architecture:**
- **Manifests:** `[workspace.package]` holds the version and `homepage`;
  `[workspace.dependencies]` pins the 19 published crates exactly; each
  member's normal and build dependency on one is `{ workspace = true }`;
  dev-dependencies stay paths, which `cargo package` strips.
- **Scripts:** `.github/scripts/package-gate.sh` (package and verify the
  19, compare their lockfiles, install `nova` from the packages, run
  `gate.sh`) and `.github/scripts/publish-crates.sh` (the publish list, the
  sparse-index check, the rate limit); `tools/install/install.sh` (POSIX
  `sh`) and `tools/install/install.ps1` (Windows PowerShell 5.1 and 7).
- **Workflows:** `ci.yml` gains the `package` job; `publish-crates.yml` and
  `release-gate.yml` run only by hand; `release.yml` reads the workspace's
  version, attaches the install scripts, and runs `install.sh` on its Linux
  archive and `install.ps1`, PATH on, on its Windows one.
- **`nova-cli`:** the templates in `src/template.rs`, in `nova fmt`'s
  layout; three new test files and one new test in `project.rs`.
- **Docs:** `docs/releasing.md`, `tools/vscode-nova/CHANGELOG.md`, and
  ADR 0036.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- No new crate, and no `Cargo.lock` change at all in 3.6a: every task
  checks that `git diff --stat a17f140 -- Cargo.lock` prints nothing.
- Tests: `crates/nova-cli/tests/release_manifests.rs` (the manifests,
  through `cargo metadata`), `publish_script.rs` (Unix) and
  `install_scripts.rs` (the scripts, against the loopback server of
  `crates/nova-index/tests/support/mod.rs`), and a new test in
  `project.rs`. The workflows and scripts are linted with actionlint and
  shellcheck.

**Spec:** `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`,
approved by the user on 2026-10-12 and amended while planning (its header
says how: §7.1's templates, §6.3's Windows PATH step, decisions 24 and 25).
- Read it before Task 1: it is the authority this plan argues from.
- Its §19 lists the decisions, and its §2 the code as it stood.

**Built from a prototype.** Four agents built every part on a local branch,
`proto-36a`, from `a17f140` (17 commits, never pushed): the manifests and
the package gate (report A), the publish script and workflow and the
templates (B), the install scripts and `release.yml` (C), the extension,
the release gate and the runbook (D). Their reports and the ledger are the
source of every `Expected:` below, and of most decisions. This plan puts
their work in test-first order. Its code blocks were copied from the
prototype's files by `$P/gen36a.py`, never typed. `$P/check36a.py` carries
out every extraction command below, in this plan's order, on a copy of
`a17f140`, and compares each task's files with the prototype byte for byte.
Only six files end differently from the prototype, on purpose: four carry
decisions 41 and 42, `install_scripts.rs` decision 49, and `release.yml`
the Windows PATH step of decision 40, which the prototype does not have.

**Checked on 2026-10-12, before execution:** by script, and by a fresh fact-check.
- **The script.** The code blocks are copied from the prototype by
  `$P/gen36a.py`. `$P/check36a.py` carries out every extraction command in
  this plan's order on a copy of `a17f140`. Each diff applies, and each
  task's files equal the prototype's byte for byte. Six files end
  differently, on purpose (decisions 40 to 42 and 49). It also runs
  `msrv_test` on the three new test files (with and without `--cfg
  windows`), Task 2's list extraction, Task 4's refusal step, Task 9's
  version check and the nine mutant commands. It passes.
- **The fact-check** read the plan against the amended spec, the
  prototype, the four reports and the ledger, and re-ran what is cheap on
  this machine:
  - the logs behind every failing and passing `Expected:`, and every test
    name the plan cites;
  - the package lists (368 files: a git checkout lists each crate's
    `.cargo_vcs_info.json`), and `cargo +1.78 metadata` on the final
    manifests;
  - Task 5's grep, the runbook's 16 stops, and the records sweep's 23
    hits at `a17f140`;
  - `install.ps1`'s parse and mutant 7 with PowerShell 7;
  - actionlint and shellcheck.
- **It found two faults, fixed here:**
  - CI runs `cargo test` on Windows under PowerShell 7. The test starts
    Windows PowerShell through `cargo`, so it would inherit PowerShell 7's
    `PSModulePath` and could not load its own cmdlets (Microsoft's
    `about_PSModulePath`). The five Windows archive cases would have failed
    at `Compress-Archive`. Task 6 Step 2 now starts Windows PowerShell
    without that variable (decision 49).
  - `release.yml`'s Windows PATH step checked only that the directory was
    on the user's PATH. It passed a copy of `install.ps1` that replaced the
    whole PATH. The step now also requires:
    - the directory first;
    - every earlier entry kept exactly;
    - the value still `REG_EXPAND_SZ`;
    - the directory there once after a second run.

    It also gets a 10-minute timeout and `-NonInteractive`. It was run on
    Linux against a stand-in registry (`$P/winsim-36a.py`), the first run
    anywhere of `install.ps1`'s PATH code. It passed. Five faults put into
    that code each made it fail; the draft step passed all five
    (decision 40).
- **Restructured:** the old Tasks 6 and 7 are now one task, the install
  scripts and their test, so no commit carries a known failing test
  (decision 39). The tasks after it are renumbered: 7 is `release.yml`
  and 12 the final verification. Every cross-reference follows.
- **Also corrected:**
  - Task 6's account of which cases run on each system;
  - the quotes of `install.ps1`'s "already" line in decisions 30 and 40;
  - decision 35 now says the spec amendment goes to the user with this
    plan;
  - decision 48 now says which tasks are test-first.
- **Not checked here:**
  - the full suite on the final tree (Task 12's 2017 passed is computed,
    and no full run was made of the prototype's final state);
  - anything on Windows or macOS, including Windows PowerShell 5.1 itself.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`.
  No `Option::is_none_or` (1.82), no `std::path::absolute` (1.79), no
  `iter::repeat_n` (1.82).
- **`cargo +1.78` cannot build nova-cli's test targets:** cargo 1.78 cannot
  parse `assert_cmd` 2.2.1's manifest (it needs `edition2024`). A new test
  file that does not use `assert_cmd` is type-checked with `rustc +1.78`
  instead, as report A did (`msrv_test`, Conventions). `project.rs` uses
  `assert_cmd`, so its new test is checked only by reading: closures,
  slices and `format!`.
- **No new crate, and `Cargo.lock` does not change.** Every task runs
  `git diff --stat a17f140 -- Cargo.lock`, which must print nothing. Any
  change to it stops the task: ask the user. A new dev-dependency would
  change it too (the lock records dev-dependencies), which is why the
  tests use only what nova-cli already has.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`
  or `?` (agent.md §4). The only non-test Rust this plan changes is
  `template.rs`'s string constants.
- **Tests never touch the internet.** Every request goes to the loopback
  server; the tests remove `http_proxy`, `https_proxy`, `all_proxy` and
  their upper-case forms from the scripts' environment. The publish test's
  `cargo` is a fake that the test names in `CARGO`, so no real `cargo
  publish` can run.
- **A test that needs `$NOVA_HOME` sets it to a fresh directory.** Every
  test directory has a fixed name under the system's temporary directory,
  removed and made again at the test's start.
- **Nothing is published, no tag is pushed, no secret is added, and no
  workflow is started by hand.** `publish-crates.yml` and
  `release-gate.yml` are written and linted, never run. Merge by rebase,
  on the user's word only. Never enable auto-merge.
- **This machine is Linux.** Windows and macOS results come from CI, which
  runs all three systems. What first runs there is named in each task.
- **Never give `/dev/null` as an output path to a tool that replaces its
  output file** (`vsce package --out`, and any writer that renames into
  place): it happened once while prototyping and replaced the device with
  a regular file. Use a file in `$P`. A shell redirection, `> /dev/null`,
  is fine.
- **`gate.sh` refuses any `NOVA_` variable.** A local run of it, or of
  `package-gate.sh`, which calls it, sets `HOME` to a scratch directory so
  that `~/.nova` lands there and not in the real home, keeping
  `CARGO_HOME` and `RUSTUP_HOME` pointing at the real ones.
- **`cargo package` refuses a dirty tree** for the files it packages:
  `package-gate.sh` runs on committed crates.

## Conventions

- **Scratch directory:** `P=/tmp/claude-0/-home-user-nova/1504a555-d3ea-5356-b003-28f2066a0d92/scratchpad`,
  outside the repository. Set it in every shell before a command that
  names `$P`.
- **This plan:** `PLAN=/home/user/nova/docs/superpowers/plans/2026-10-12-phase-3-6a-release-preparation.md`
  once it is committed; before that, `PLAN=$P/plan36a.md`. The two are
  byte-identical, and every line number below is this file's. The plan is
  generated: an edit by hand shifts every range after it, so edit
  `$P/plan36a.tmpl.md`, run `python3 -I $P/gen36a.py`, then
  `python3 -I $P/check36a.py` (which must end `check36a: PASS`).
- **Code blocks are copied by line range, never retyped.** A new file is
  `sed -n 'N,Mp' "$PLAN" > <file>`. A change is a `diff` block,
  extracted to `$P/36a-<name>.diff` and applied from the repository's root
  with `git apply`. If a diff does not apply, stop: the tree is not what
  the plan expects. Files are made with the default mode (644), as the
  repository's other scripts are; CI runs them with `bash` or `sh`.
- **The ledger:** `$P/ledger-36a.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures.
- **Commit messages** are written to `$P/msg-36a-<task>.txt` and committed
  with `git commit -F`. Each ends with these two lines, exactly:

  ````text
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
  ````

- **The baseline's full output** is `$P/fixfinal-full.txt` (Task 0).
- **Lint tools,** downloaded while prototyping: actionlint 1.7.7 and
  shellcheck 0.10.0. In every shell that lints:

  ````bash
  AL="$P/actionlint-dl/actionlint -shellcheck $P/shellcheck-dl/shellcheck-v0.10.0/shellcheck"
  SC=$P/shellcheck-dl/shellcheck-v0.10.0/shellcheck
  ````

  If they are missing, download the same versions into new, empty
  directories under `$P`.
- **The 1.78 check** runs in its own target directory, so the metadata it
  writes is the only metadata there:
  `CARGO_TARGET_DIR="$P/target-178" RUSTFLAGS="-D warnings" cargo +1.78 check --locked --workspace`
  (about 40 s from cold). Then a new test file is type-checked against it
  with this shell function (define it once per shell; run it from the
  repository's root):

  ````bash
  msrv_test() {  # FILE CRATE... [-- RUSTC-FLAG...]: type-check a nova-cli test file with rustc 1.78
    local file=$1 externs=()
    shift
    while [ $# -gt 0 ] && [ "$1" != -- ]; do
      externs+=(--extern "$1=$(ls "$P"/target-178/debug/deps/lib"$1"-*.rmeta)")
      shift
    done
    [ $# -gt 0 ] && shift
    mkdir -p "$P/msrv36a"
    CARGO_MANIFEST_DIR="$PWD/crates/nova-cli" CARGO=cargo CARGO_BIN_EXE_nova="$PWD/target/debug/nova" \
      rustc +1.78 --edition 2021 --test --emit=metadata -D warnings \
      --crate-name "$(basename "$file" .rs)" -L "dependency=$P/target-178/debug/deps" \
      "${externs[@]}" "$@" --out-dir "$P/msrv36a" "$file" && echo "1.78: $file type-checks"
  }
  ````

  Expected: `1.78: <file> type-checks`, and nothing else. The crates a
  file needs are named in its task: `serde_json` and `toml_edit` for
  `release_manifests.rs`; `base64` and `serde_json` for the two files that
  include the loopback server's module (its `fake_github.rs` needs both).
  Flags after `--` go to `rustc`.
- **The checks,** at the end of every task that changes Rust or scripts,
  each on what the task touched: `cargo fmt --all`, then `cargo fmt --all
  -- --check` (expected: no output); `cargo clippy --locked -p nova-cli
  --all-targets -- -D warnings` (expected: `Finished`); the 1.78 check;
  `msrv_test` on a new test file; `$SC` on a new script; `$AL` on a changed
  workflow; and `git diff --stat a17f140 -- Cargo.lock` (expected: no
  output).

As the 3.5b plan's: each task is test-first (the failing test, its run and
the expected failure, the code, its run and the expected pass), copies the
code blocks below by line range rather than retyping them, runs the checks
on what it touched, and ends with one commit whose message ends with the
session's attribution lines. A script or workflow with no test of its own
is checked by running or linting it, as its task says. A difference from an
`Expected:` is fixed in the smallest way and recorded in the ledger as a
ruling, with its cost if wrong.

## Review Focus

The inputs most likely to bite, most likely first. Each has its test in
the task that owns the code:

1. **The package route on Windows and macOS,** where `package-gate.sh`
   meets Git Bash's paths, GNU tar's `D:` and bash 3.2, and has never run
   → "Packaged nova (os)" (Task 2), first run in the pull request's CI;
   decisions 6 to 9.
2. **`install.ps1` on Windows PowerShell 5.1, and its PATH code,** which no
   test runs → `install_scripts.rs`'s Windows cases (Task 6, in CI), which
   start Windows PowerShell without the `PSModulePath` CI's PowerShell 7
   would pass on (decision 49), and `release.yml`'s Windows step with PATH
   on, run twice (Task 7, in CI, decision 40); mutant 7.
3. **The publish order and resuming a stopped run,** where a mistake is
   permanent on crates.io → the list's order test (Task 1), the publish
   script's nine tests (Task 3); mutants 3, 4 and 5.
4. **Exact pins and the bump,** which must move together → Task 1's tests,
   mutants 1, 2 and 8, and the runbook's bump commands (Task 10).
5. **The templates,** which every user's first project is made from →
   `every_template_passes_fmt_check_as_written` (Task 5), mutant 9, and the
   release gate's `nova fmt --check` (Task 9).
6. **`install.sh` under `dash` and macOS's `sh`, and its checksum** → the
   Unix cases of `install_scripts.rs` (Task 6) run it with `sh`; mutant 6.
7. **The token,** which must never be printed → `publish_script.rs`'s
   helper checks every run (Task 3); the workflow puts it in the publish
   step's `env:` alone (Task 4).
8. **What the tarballs carry** (spec §14 risk 5) → Task 2's review of the
   19 package lists.

## Decisions: where this plan settles what the spec leaves open

Each names its source: a prototype report (A, B, C or D), the ledger, or
this plan. Where the prototype disagreed with the spec or with itself, the
decision says which wins (decisions 15, 27, 32, 33, 36, 41, 42).

1. All 23 members say `version.workspace = true`, and every normal and build dependency between members, the four unpublished crates' included, is `{ workspace = true }`. Reason: one rule for every member, which §11.1's test then holds for all 23; the unpublished manifests are never packaged, and `Cargo.lock` does not change (A).
2. All 23 members say `homepage.workspace = true`. Reason: §3.3 gives the workspace a `homepage`, and without the opt-in no crate shows it; the four unpublished ones too, for one rule (A).
3. Each published library's description is "The Nova language's <part> (internal to the nova toolchain)", and nova-cli's is §3.3's text exactly; the four unpublished crates keep their descriptions (A).
4. `release_manifests.rs` reads the root's `[workspace.dependencies]` with `toml_edit`, already a normal dependency of nova-cli and so usable by its integration tests; a new dev-dependency would change `Cargo.lock`. It checks a little more than §11.1: every member's version, the pins' set and paths, that no published crate depends on an unpublished one, `homepage`, and "internal" in each library's description (A).
5. `release.yml`'s three version reads move to the root `Cargo.toml` in Task 1, not Task 7, and `Cargo.toml` joins its pull-request paths. Reason: after Task 1, nova-cli's manifest has no `version = "..."` line, so the pull request's release run would fail; the root's only line starting `version = ` is `[workspace.package]`'s (A).
6. `package-gate.sh` runs `cargo fetch --locked` first and everything after it `--offline`. Reason: a fresh runner's registry is empty, and an offline resolve over a cache that a locked fetch filled can pick only the locked versions (A).
7. The emulated install has no `--locked`: the packaged nova-cli lock records the nova crates as registry entries with checksums, which the `[patch.crates-io]` paths would change ("cannot update the lock file ... because --locked was passed"). Step 2's lockfile comparison stands in for it. The install resolves afresh, offline: on a fresh runner it can find only the locked versions; where the registry cache holds newer compatible ones (a restored cache, a developer's machine), it may build those, which `cargo install nova-cli --locked` from crates.io would not (A; this plan).
8. On Windows, `package-gate.sh` turns WORKDIR into `D:/a/...` with `cygpath -m` (cargo and the config's patch paths take it, and it has no backslashes), and runs every `tar` from the package directory with relative names, since GNU tar reads `D:` as a remote host; `tr -d '\r'` guards a CRLF checkout; nothing needs bash 4, for macOS's `/bin/bash` 3.2 (A). Written for CI; not run here.
9. `package-gate.sh` sets `CARGO_INCREMENTAL=0` and `CARGO_PROFILE_DEV_DEBUG=0` for the packaging, exports `CARGO_TARGET_DIR=$WORKDIR/target` for the whole script, requires WORKDIR not to exist (as the other gate scripts do), and checks that `root/bin` holds only `nova` (as the install job does) (A).
10. CI's install job keeps its `cargo package -p nova-std --locked` step, which the package job now covers: removing it is not 3.6a's (A; a deferred minor).
11. The publish script's rate-limit match is `status 429|too many requests|try again after`, case-insensitive. Reason: a bare `429` would match cargo's own `Packaged N files, 429.0KiB` (B).
12. A version is on crates.io when its index file has a line with `"vers":"<version>"`, the closing quote included, so `0.3.0` never matches `0.3.0-alpha.1`; a yanked version counts, since crates.io never takes the same version twice (B).
13. Usage errors (no argument, a malformed VERSION, a `NOVA_PUBLISH_WAIT` that is not a number) exit 2, as the other scripts' do; a stop exits 1 (B).
14. The summary also prints when the run stops, with `failed` and `not tried` beside `published` and `skipped`. Reason: a stopped run is when the maintainer most needs to see what is up before starting again (B; §5.2 says "It ends with one line per crate").
15. "Up to 30 times" is 30 attempts in all: the 30th refusal stops the run (`gives_up_after_thirty_rate_limited_attempts`). §5.2's "tries the same crate again, up to 30 times" can be read as 31; the difference is one 11-minute wait, inside the 345 minutes either way. The prototype wins (B).
16. cargo's output is shown as it comes, through `tee`, and is not printed again when the run stops ("its output is above"); no `curl --retry`, since a stopped run resumes (B).
17. `publish-crates.yml`'s refusal step also refuses when the environment has no secret. It sees only `secrets.CARGO_REGISTRY_TOKEN != ''`, never the value, so a forgotten secret fails in seconds and not after the package gate; the value is only in the publish step's `env:` (B).
18. The job that holds the token checks out with `persist-credentials: false` and restores no `Swatinem/rust-cache`: it runs build scripts, so it neither keeps a git credential on disk nor restores a cache another workflow saved (B, after D's reasoning).
19. The fake `cargo`'s protocol (spec §18): a `#!/bin/sh` script, written by each test, that appends `"$*"` to `cargo.log`, takes the crate after `-p`, exits 3 when `CARGO_REGISTRY_TOKEN` did not reach it, and answers from an `answers` file: no line or `<crate> ok` succeeds; `<crate> 429 <n>` fails its first `n` runs for that crate with crates.io's real 429 text; `<crate> fail` always fails with a 400 (B).
20. `publish_script.rs` runs the script under `/bin/bash` on macOS, which is bash 3.2 even when a Homebrew bash is first on PATH. bash 3.2 could not be run here (the proxy refused ftp.gnu.org); CI's macOS Test job is its first run (B).
21. `publish_script.rs` has three tests beyond §11.2: the 30-attempt cap, an unreachable index, and a missing or malformed version (B).
22. `install.ps1` writes the user's PATH through the registry: `HKCU\Environment`'s `Path`, read with `DoNotExpandEnvironmentNames` and written back as `REG_EXPAND_SZ`, as rustup does. `[Environment]::SetEnvironmentVariable('Path', ..., 'User')` would store every `%USERPROFILE%\...` entry expanded, as `REG_SZ`. It tells running programs by removing a user variable that never exists through `[Environment]`, which makes .NET broadcast `WM_SETTINGCHANGE`, with no P/Invoke (C).
23. The directory goes first, in the user's PATH and the session's, as rustup's does, so this `nova` wins over another one (C).
24. Both scripts are plain ASCII, and `install.sh` has Unix line ends and starts `#!/bin/sh`. Reason: Windows PowerShell 5.1 reads a script with no byte-order mark in the system's code page, and `irm | iex` would choke on a mark; `the_scripts_are_plain_ascii` holds it (C).
25. `install.ps1` runs in one `& { }` block under strict mode and `$ErrorActionPreference = 'Stop'`, so `iex` leaves nothing in the session. Run as a file, an error goes to stderr and exits 1; through `iex` at a prompt it is thrown, so the window stays open. The target comes from `$env:OS` and `PROCESSOR_ARCHITEW6432`, else `PROCESSOR_ARCHITECTURE`. The latest version is read from the `Location` of `<base>/latest`, with an `HttpClient` whose redirects are off, in 5.1 and 7 alike (spec §18) (C).
26. `install_scripts.rs` goes beyond §11.3: `install.ps1` on an unknown system (Windows) and the ASCII test; the mismatch test also checks that a failed install leaves the old binary (§6.1 step 5); the fake server answers `/releases/tag/v<latest>` with a page, as GitHub does, because `install.sh`'s `curl -L` follows the redirect; each packed staging directory is removed once archived (a run had left 2.2 GB); on Unix `HOME` is the test's own directory too, so no default reaches the real home (C).
27. `release.yml`'s `install.sh` step serves a copy of the release's Linux archive and `SHA256SUMS` from `$RUNNER_TEMP/srv/releases/download/v<version>/`, the layout `<base>/download/v<version>/` has, where §6.3 says "served from `release/dist`": the same files, so the prototype wins. It waits for the server with a quiet `curl --retry-connrefused`, and the workflow's header comment now lists the install scripts among the assets (C).
28. The release gate's crates.io job has no cargo cache, so the nested runtime build resolves from what the install itself fetched (§14 risk 1). Cost if wrong: slower gate runs (D; ledger).
29. Both release-gate jobs check that the first line of `nova version` is `nova <version>`, and the crates.io job that only `nova` was installed: they catch a script that ignored `NOVA_VERSION`, or a crates.io install of the wrong version (D).
30. On Windows the release gate runs `install.ps1` under PowerShell 7 and then Windows PowerShell 5.1, which is what `irm | iex` runs in on a Windows with nothing else; the second install replaces the first, and its PATH branch says `<dir> is on your PATH already` (D).
31. The release gate does not run `registry-gate.sh`: §8 names `gate.sh`, `packages-gate.sh` and `doc-gate.sh` (D).
32. The extension's CHANGELOG heading is `## [Unreleased]`: package.json still says 0.2.0. 3.6b's release commit renames it `## [0.3.0] - <date>`, with "The first published version." under it (the runbook's step 1.3). §7 says "0.3.0 is the first published version"; the prototype's heading wins until the bump (D).
33. The runbook makes the index token in its step 5 and runs `nova login` on the clean machine (6.2), whose `$NOVA_HOME` holds the credentials; §9 item 5 puts `gh auth token | nova login` in step 5, which the runbook keeps as the alternative. The prototype wins (D).
34. The runbook adds a deployment tag rule `v*` to the `crates-io` environment and `gh run rerun --failed` for a Release run that failed for a passing reason. The store, token and UI steps it could not try are written from documentation and checked at 3.6b (D; deferred minors).
35. The templates are fixed in 3.6a (spec amended, §7.1, decision 24): a test runs `nova new`, `nova new --lib`, `nova init` and `nova init --lib`, and requires `nova fmt --check` to pass on each project. The 3.0 spec gets a dated note rather than an edit of the template it quotes, which records what 3.0 approved (B; ledger). The amendment (8eef2a2, spec decisions 24 and 25) is flagged to the user with this plan, as 3.5b's closure fix was (ledger).
36. The release gate's format step is §8's plain `nova fmt --check` on the project `gate.sh` made (B's 8af3fc4 over D's format-then-check workaround, which the fixed templates no longer need).
37. The 1.78 check runs with `CARGO_TARGET_DIR=$P/target-178`, and each new test file is type-checked against that directory's metadata with `rustc +1.78` (`msrv_test`). Reason: cargo 1.78 cannot build nova-cli's test targets, and a separate directory holds one metadata file per crate (this plan).
38. Task 1's 24 manifests are one diff applied with `git apply`, and its 38 licence copies one `cp` loop: they are copies of the root's files, which the test compares byte for byte (this plan).
39. The install scripts and their test are one task, Task 6, with one commit: `install_scripts.rs` first and its failing run (0 of 8 on Linux), then `install.sh` (7 of 8: the ASCII test, which reads both scripts, then fails on the missing `install.ps1`, which is `install.ps1`'s failing test), then `install.ps1` (8 of 8). So no commit carries a known failing test, as a commit of `install.sh` with the whole test would (the ledger's ruling; this plan).
40. `release.yml`'s Windows PATH step (§6.3 as amended, decision 25), which the prototype does not have, is written by this plan. In the Windows build job, after the smoke test, a `pwsh` step with a 10-minute timeout serves the job's own zip and a `SHA256SUMS` line made with `Get-FileHash` from `$RUNNER_TEMP\srv` with `python -m http.server`, and runs `install.ps1` in `powershell` (Windows PowerShell 5.1, started by `pwsh` itself, so that it builds its own `PSModulePath`; decision 49) with `NOVA_INSTALL_BASE`, `NOVA_VERSION` and `NOVA_HOME` and without `NOVA_INSTALL_NO_PATH`. It requires `runtime: embedded` from the installed `nova version`, and reads `HKCU\Environment`'s `Path` unexpanded before and after: the install directory must come first, every earlier entry must still be there exactly, and the value must be `REG_EXPAND_SZ` (decisions 22, 23). It then runs `install.ps1` again, which takes the `is on your PATH already` branch, and the directory must be there once: running the script again is how `nova` is updated (§6.2), and the release gate, the only other run of that branch, comes after publishing. §6.3 asks only that the directory be there; a draft of this step checked that alone, and passed a copy of `install.ps1` that replaced the user's whole PATH. It ran on Linux with PowerShell 7.4.6 and a stand-in registry (`$P/winsim-36a.py`, Task 7 Step 2); it first runs on Windows in the pull request's CI (this plan; the fact-check).
41. The amended spec numbers the template rule §7.1, but the prototype's `template.rs`, `project.rs` and the 3.0 spec's note cite §8, written before the amendment; and the note is headed "(Phase 3.6a)", where the file's 2026-10-08 note, and Task 11's notes, name the branch. The spec and the file's style win: Task 5's Step 4 changes the three citations to §7.1 and the heading to `(branch `claude/loving-archimedes-frtf5q`)`.
42. `docs/releasing.md` agrees with the final scripts on the asset count (eight and `SHA256SUMS`, as `release.yml`'s count check), the install URLs, the workflows' names and inputs, and the defaults. Two passages do not, and Task 10's Step 2 fixes them: "`release.yml` without its publish job, which needs step 2's section" names the wrong step (the section is item 2 of step 1, the CHANGELOG's), and step 2's account of the Release run leaves out its two install runs (§6.3, decision 25). The plan wins.
43. `crates/nova-cli/README.md` already names the install scripts, as §3.3 asks. They work only from `v0.3.0`, the first release that carries them, which is also the first time crates.io shows the README; decision 21's texts (the extension's README, its error message, the root README) still change in 3.6b (A).
44. Mutant 7 runs only on Windows. It is taken as covered by the prototype's reproduction with PowerShell 7 (repeatable with `$P/mut7-36a.sh`) and by the pull request's Windows run of the unmutated test. A run on a throwaway branch needs the user's word to push (this plan).
45. The PR body's check for local paths and names allows the account `Sakeerin` in repository URLs and the publisher ID `sakeerin`: they are the project's own identifiers, which the shipped files name (this plan).
46. CHANGELOG's `[Unreleased]` also records the template change, under Changed (B; the user's list for Task 11).
47. §14 risk 5's review of the tarballs' file lists was made while planning: nothing stray in any of the 19. Task 2 repeats it on the real tree (this plan).
48. The prototype's 17 commits become 11 tasks with one commit each, then Task 12's final verification. Tasks 1, 3, 5 and 6 are test-first; the others are checked by running or linting what they make. The order is the spec's §17, with the templates after the publish workflow and `release.yml`'s version reads in Task 1 (decision 5) (this plan).
49. `install_scripts.rs` starts Windows PowerShell through one helper, `windows_powershell()`, which removes `PSModulePath` from its environment (Task 6 Step 2). CI runs `cargo test` on Windows under PowerShell 7, the runner's default shell, which puts its own module paths first in `PSModulePath`; Windows PowerShell started through an intermediate process, here `cargo` and the test binary, inherits them, cannot load its own `Microsoft.PowerShell.*` modules, and its cmdlets are not found (Microsoft's `about_PSModulePath`, "Starting Windows PowerShell from PowerShell 7"). So the prototype's five Windows archive cases would fail at their first `Compress-Archive`, and `install.ps1` at its first cmdlet. Without the variable, Windows PowerShell builds its default, as on a user's machine. `release.yml`'s step starts `powershell` from `pwsh` directly, which PowerShell 7 handles itself, and the release gate's `shell: powershell` step is started by the runner (this plan; the fact-check).

## Contracts between tasks

- **The publish list** (`.github/scripts/publish-crates.sh`, Task 1, kept
  byte for byte by Task 3): a line `crates=(`, one crate per line,
  indented, then `)` alone on its line; `#` lines and blank lines are
  skipped. Three parsers read it: `release_manifests.rs` and
  `publish_script.rs` (`lines().map(str::trim)` after the `crates=(`
  line), and `package-gate.sh` (an `awk` over `/^crates=\(/` to `/^\)/`,
  which requires 19 entries). Nothing else in the script may start a line
  with `crates=(`.
- **The version:** the root `Cargo.toml` has exactly one line starting
  `version = `, `[workspace.package]`'s. `release.yml` and
  `package-gate.sh` read it with
  `tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1`,
  `publish-crates.yml` with the same `sed` alone, the Windows PATH step
  with `Select-String`, and `release_manifests.rs` with `toml_edit`.
- **`package-gate.sh WORKDIR`** (Task 2): run from the repository's root,
  with WORKDIR absent and outside any cargo workspace (the repository's
  `target/` is inside one), no `NOVA_` variable set, and committed crates.
  It prints `package-gate: ` lines with each step's duration and ends
  `package-gate: passed in <n> s`. Task 4's workflow calls it as `ci.yml`
  does.
- **`publish-crates.sh VERSION`** (Task 3): `CARGO`, `NOVA_PUBLISH_INDEX`
  and `NOVA_PUBLISH_WAIT` may be set; cargo reads `CARGO_REGISTRY_TOKEN`.
  It prints `skip <crate> <version> (already on crates.io)`, `published
  <crate> <version>`, `rate limited: waiting <n> s`, and `summary for
  <version>:` with one `  %-24s %s` line per crate. Exit 0, 1 on a stop, 2
  on usage.
- **The install scripts** (Task 6): `NOVA_VERSION`, `NOVA_HOME` and
  `NOVA_INSTALL_BASE` (default `https://github.com/Sakeerin/nova/releases`),
  and `NOVA_INSTALL_NO_PATH=1` for `install.ps1`. They fetch
  `<base>/download/v<version>/nova-<version>-<target>.tar.gz` (`.zip` on
  Windows) and `SHA256SUMS`, install into `$NOVA_HOME/bin` (default
  `~/.nova/bin`, `%USERPROFILE%\.nova\bin`), print `nova: ` lines and the
  installed `nova version`, and send errors to stderr with exit 1. Neither
  reads a variable named `VERSION`, which `release-gate.yml` sets for
  every step.

## File map

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-cli/tests/release_manifests.rs` (new) | 1 | The manifests' rules (§11.1) |
| `Cargo.toml`, the 22 `crates/*/Cargo.toml`, `std/Cargo.toml` | 1 | The version, pins, metadata, `publish = false` (§3) |
| `crates/README.md`, `crates/nova-cli/README.md`, 38 licence copies (new) | 1 | What each tarball carries (§3.3) |
| `.github/scripts/publish-crates.sh` (new) | 1, 3 | The publish list (1); the publish (3, §5.2) |
| `.github/workflows/release.yml` | 1, 7 | The workspace's version (1); the install scripts as assets, and both run on the release's archives (7, §6.3) |
| `.github/scripts/package-gate.sh` (new), `.github/workflows/ci.yml` | 2 | The package route, "Packaged nova (os)" (§4) |
| `crates/nova-cli/tests/publish_script.rs` (new) | 3 | The publish script against a fake cargo (§11.2) |
| `.github/workflows/publish-crates.yml` (new) | 4 | "Publish to crates.io" (§5.1) |
| `crates/nova-cli/src/template.rs`, `crates/nova-cli/tests/project.rs`, the 3.0 spec | 5 | The templates in `nova fmt`'s layout (§7.1) |
| `crates/nova-cli/tests/install_scripts.rs`, `tools/install/install.sh`, `tools/install/install.ps1` (new) | 6 | The install scripts, PATH, and their test (§6, §11.3) |
| `tools/vscode-nova/CHANGELOG.md` (new), `tools/vscode-nova/package.json` | 8 | The listing (§7) |
| `.github/workflows/release-gate.yml` (new) | 9 | "Release gate" (§8) |
| `docs/releasing.md` (new) | 10 | The runbook (§9) |
| `docs/adr/0036-publishing-nova.md` (new), dated notes, `CHANGELOG.md` | 11 | The records (§10) |

---

### Task 0: Baseline

Recorded on 2026-10-12, in the ledger. The tree at `a17f140`, this plan's
base, is `main` (`21d8e87`) plus one document, the spec. The full suite
ran on `2f6803c`, the 3.5b branch's head, whose tree is `21d8e87`'s:
`cargo test --workspace --locked --all-features --no-fail-fast` gave 1992
passed, 1 failed, 10 ignored. The one failure is
`nova-runtime net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`,
which needs IPv6 `::1` and is environmental here. Its per-test output is
`$P/fixfinal-full.txt`. Nothing to run. Every later full run compares
against it.

---

### Task 1: The manifests (spec §3, §11.1)

**Files:**
- Create: `crates/nova-cli/tests/release_manifests.rs`
- Create: `.github/scripts/publish-crates.sh` (the list only; Task 3
  completes it)
- Modify: `Cargo.toml` and the 23 members' `Cargo.toml`
- Create: `LICENSE-MIT` and `LICENSE-APACHE` in each of the 19 published
  crates' directories; `crates/README.md`; `crates/nova-cli/README.md`
- Modify: `.github/workflows/release.yml` (its three version reads and its
  pull-request paths; decision 5)

**Interfaces:** the publish list (Contracts). After this task a bump is
the workspace's `version`, the 19 pins, the extension's `package.json` and
`package-lock.json`, and `Cargo.lock` (§3.2).

- [ ] **Step 1: Write the failing test, and the list it reads**

Create `crates/nova-cli/tests/release_manifests.rs`, lines 436-750 of this plan:

Run: `cd /home/user/nova && sed -n '436,750p' "$PLAN" > crates/nova-cli/tests/release_manifests.rs`

````rust
//! The workspace's manifests are ready for crates.io (spec
//! `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
//! §3, §11.1): the 19 crates `nova` needs are published, the other four are
//! not, every dependency between workspace crates is an exact pin on the
//! workspace's version, `publish-crates.sh` uploads them in dependency
//! order, each tarball carries a README and both licence texts, and the
//! VS Code extension has the same version.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// The members that `publish = false` keeps off crates.io (spec §3.1).
const UNPUBLISHED: [&str; 4] = [
    "nova-bench-http",
    "nova-bundler",
    "nova-codegen-wasm",
    "nova-test",
];

const REPOSITORY: &str = "https://github.com/Sakeerin/nova";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `[workspace.package]`'s `version` in the root `Cargo.toml`.
fn workspace_version() -> String {
    let manifest: toml_edit::DocumentMut = read(&root().join("Cargo.toml"))
        .parse()
        .expect("the root Cargo.toml parses");
    manifest
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .expect("[workspace.package] sets no version")
        .to_string()
}

/// One workspace member, as `cargo metadata` describes it.
struct Member {
    version: String,
    manifest_dir: PathBuf,
    published: bool,
    /// Its normal and build dependencies on other members, with their
    /// requirements.
    deps: Vec<(String, String)>,
    readme: Option<String>,
    repository: Option<String>,
    homepage: Option<String>,
    description: String,
}

fn members() -> BTreeMap<String, Member> {
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--no-deps",
            "--offline",
            "--format-version",
            "1",
        ])
        .arg("--manifest-path")
        .arg(root().join("Cargo.toml"))
        .output()
        .expect("running cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("cargo metadata's JSON");
    let packages = metadata["packages"].as_array().expect("packages");
    let names: BTreeSet<&str> = packages
        .iter()
        .map(|p| p["name"].as_str().expect("a package name"))
        .collect();
    let text = |value: &Value| value.as_str().map(str::to_string);
    packages
        .iter()
        .map(|p| {
            let name = p["name"].as_str().expect("a package name").to_string();
            // `publish = false` is `[]`; no `publish` key is `null`.
            let published = match &p["publish"] {
                Value::Null => true,
                Value::Array(registries) => !registries.is_empty(),
                other => panic!("{name}: publish is {other}"),
            };
            let deps = p["dependencies"]
                .as_array()
                .expect("dependencies")
                .iter()
                .filter(|d| names.contains(d["name"].as_str().expect("a dependency name")))
                .filter(|d| d["kind"].is_null() || d["kind"] == "build")
                .map(|d| {
                    (
                        d["name"].as_str().expect("a dependency name").to_string(),
                        d["req"].as_str().expect("a requirement").to_string(),
                    )
                })
                .collect();
            let manifest = PathBuf::from(p["manifest_path"].as_str().expect("manifest_path"));
            let member = Member {
                version: p["version"].as_str().expect("a version").to_string(),
                manifest_dir: manifest.parent().expect("a directory").to_path_buf(),
                published,
                deps,
                readme: text(&p["readme"]),
                repository: text(&p["repository"]),
                homepage: text(&p["homepage"]),
                description: text(&p["description"]).unwrap_or_default(),
            };
            (name, member)
        })
        .collect()
}

fn published(members: &BTreeMap<String, Member>) -> BTreeSet<&str> {
    members
        .iter()
        .filter(|(_, m)| m.published)
        .map(|(name, _)| name.as_str())
        .collect()
}

/// The `crates=( … )` list in `.github/scripts/publish-crates.sh`, one
/// crate per line.
fn publish_list() -> Vec<String> {
    let path = root()
        .join(".github")
        .join("scripts")
        .join("publish-crates.sh");
    let script = read(&path);
    let mut lines = script.lines().map(str::trim);
    assert!(
        lines.any(|line| line == "crates=("),
        "{}: no `crates=(` line",
        path.display()
    );
    let list: Vec<String> = lines
        .take_while(|line| *line != ")")
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    assert!(!list.is_empty(), "{}: an empty list", path.display());
    list
}

#[test]
fn exactly_the_four_crates_nova_does_not_need_are_unpublished() {
    let members = members();
    let unpublished: BTreeSet<&str> = members
        .iter()
        .filter(|(_, m)| !m.published)
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        unpublished,
        UNPUBLISHED.into_iter().collect::<BTreeSet<_>>()
    );
    assert_eq!(published(&members).len(), 19);
}

#[test]
fn every_member_has_the_workspace_version() {
    let version = workspace_version();
    for (name, member) in members() {
        assert_eq!(member.version, version, "{name}'s version");
    }
}

#[test]
fn the_workspace_pins_each_published_crate_exactly() {
    let version = workspace_version();
    let manifest: toml_edit::DocumentMut = read(&root().join("Cargo.toml"))
        .parse()
        .expect("the root Cargo.toml parses");
    let table = manifest
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table_like())
        .expect("[workspace.dependencies]");
    let members = members();
    let mut pinned = BTreeSet::new();
    for (name, item) in table.iter() {
        let Some(member) = members.get(name) else {
            continue;
        };
        pinned.insert(name);
        let pin = item.get("version").and_then(|v| v.as_str());
        assert_eq!(pin, Some(format!("={version}").as_str()), "{name}'s pin");
        let path = item
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| panic!("{name}: no path"));
        let dir = root().join(path);
        assert_eq!(
            dir.canonicalize().expect("the pin's path exists"),
            member
                .manifest_dir
                .canonicalize()
                .expect("the member's directory"),
            "{name}'s path"
        );
    }
    assert_eq!(pinned, published(&members));
}

#[test]
fn every_dependency_between_members_is_an_exact_pin_on_the_workspace_version() {
    let version = workspace_version();
    let members = members();
    let published = published(&members);
    for (name, member) in &members {
        for (dep, req) in &member.deps {
            // A path dependency with no version reads `*`.
            assert_eq!(req, &format!("={version}"), "{name}'s dependency on {dep}");
            if member.published {
                assert!(
                    published.contains(dep.as_str()),
                    "{name} is published but depends on {dep}, which is not"
                );
            }
        }
    }
}

#[test]
fn the_publish_list_is_the_published_crates_in_dependency_order() {
    let members = members();
    let list = publish_list();
    let listed: BTreeSet<&str> = list.iter().map(String::as_str).collect();
    assert_eq!(
        listed.len(),
        list.len(),
        "a crate is listed twice: {list:?}"
    );
    assert_eq!(listed, published(&members));
    for (position, name) in list.iter().enumerate() {
        for (dep, _) in &members[name].deps {
            let before = list[..position].contains(dep);
            assert!(before, "{name} is listed before its dependency {dep}");
        }
    }
}

#[test]
fn every_published_crate_carries_a_readme_both_licences_and_its_links() {
    let members = members();
    let mit = std::fs::read(root().join("LICENSE-MIT")).expect("the root's LICENSE-MIT");
    let apache = std::fs::read(root().join("LICENSE-APACHE")).expect("the root's LICENSE-APACHE");
    for (name, member) in members.iter().filter(|(_, m)| m.published) {
        let readme = member
            .readme
            .as_deref()
            .unwrap_or_else(|| panic!("{name} sets no readme"));
        let readme = member.manifest_dir.join(readme);
        assert!(
            readme.is_file(),
            "{name}: {} is not a file",
            readme.display()
        );
        for (file, root_text) in [("LICENSE-MIT", &mit), ("LICENSE-APACHE", &apache)] {
            let path = member.manifest_dir.join(file);
            let copy = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            assert!(
                &copy == root_text,
                "{} differs from the root's",
                path.display()
            );
        }
        assert_eq!(member.repository.as_deref(), Some(REPOSITORY), "{name}");
        assert_eq!(member.homepage.as_deref(), Some(REPOSITORY), "{name}");
        assert!(
            member.description.contains("Nova"),
            "{name}'s description does not name Nova: {:?}",
            member.description
        );
        if name != "nova-cli" {
            assert!(
                member.description.contains("internal"),
                "{name}'s description does not say it is internal: {:?}",
                member.description
            );
        }
    }
}

#[test]
fn the_extension_has_the_workspace_version() {
    let version = workspace_version();
    let extension = root().join("tools").join("vscode-nova");
    let json = |file: &str| -> Value {
        let path = extension.join(file);
        serde_json::from_str(&read(&path)).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    };
    let package = json("package.json");
    assert_eq!(package["version"], version.as_str(), "package.json");
    let lock = json("package-lock.json");
    assert_eq!(lock["version"], version.as_str(), "package-lock.json");
    assert_eq!(
        lock["packages"][""]["version"],
        version.as_str(),
        "package-lock.json's packages[\"\"]"
    );
}
````

Create `.github/scripts/publish-crates.sh`, the list alone, in §2's order
(cargo's own `cargo publish --workspace --dry-run` order), lines 759-781 of this plan:

Run: `cd /home/user/nova && sed -n '759,781p' "$PLAN" > .github/scripts/publish-crates.sh`

````bash
#!/usr/bin/env bash
# Publishes the 19 crates to crates.io in this order (spec docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md §5.2).
crates=(
  nova-diagnostics
  nova-std
  nova-ast
  nova-lexer
  nova-runtime
  nova-parser
  nova-pm
  nova-fmt
  nova-index
  nova-resolver
  nova-hir
  nova-mir
  nova-typeck
  nova-codegen-cranelift
  nova-codegen-llvm
  nova-driver
  nova-doc
  nova-lsp
  nova-cli
)
````

- [ ] **Step 2: Run it to see it fail**

Run: `cd /home/user/nova && RUST_BACKTRACE=0 cargo test --locked -p nova-cli --test release_manifests 2>&1 | grep -E "^test |sets no|left|right|listed|^test result"`
Expected: FAIL, `test result: FAILED. 0 passed; 7 failed` (the order of the
lines varies):
- `every_member_has_the_workspace_version`,
  `every_dependency_between_members_is_an_exact_pin_on_the_workspace_version`,
  `the_extension_has_the_workspace_version` and
  `the_workspace_pins_each_published_crate_exactly` panic with
  `[workspace.package] sets no version`;
- `every_published_crate_carries_a_readme_both_licences_and_its_links`
  with `nova-ast sets no readme`;
- `the_publish_list_is_the_published_crates_in_dependency_order` with
  `assertion left == right failed`: left the list's 19, right all 23
  crates;
- `exactly_the_four_crates_nova_does_not_need_are_unpublished` with left
  `{}` and right the four.

- [ ] **Step 3: The manifests**

One diff for the root `Cargo.toml` and the 23 members', lines 809-1637 of this plan:

Run: `cd /home/user/nova && sed -n '809,1637p' "$PLAN" > "$P/36a-t1-manifests.diff" && git apply "$P/36a-t1-manifests.diff"`

````diff
diff --git a/Cargo.toml b/Cargo.toml
index e699200..76e64dc 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -3,12 +3,47 @@ members = ["crates/*", "std"]
 resolver = "2"
 
 [workspace.package]
+# Every member's version. A release changes it together with the pins on
+# the workspace's own crates below, the VS Code extension's package.json
+# and package-lock.json, and Cargo.lock (spec
+# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
+# §3.2); nova-cli's tests/release_manifests.rs holds them to it.
+version = "0.2.0"
 edition = "2021"
 rust-version = "1.78"
 license = "MIT OR Apache-2.0"
-repository = "https://github.com/novalang/nova"
+repository = "https://github.com/Sakeerin/nova"
+homepage = "https://github.com/Sakeerin/nova"
 
 [workspace.dependencies]
+# The 19 crates published to crates.io, for the normal and build
+# dependencies between them (spec
+# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
+# §3.2). Each pin is exact and equals the version above: the crates are
+# released together, and their APIs between each other are not kept
+# stable, so nova-cli is never built against another release's
+# nova-driver, even by a `cargo install` without `--locked`.
+# Dev-dependencies stay `{ path = ... }`, which `cargo package` strips.
+nova-ast = { path = "crates/nova-ast", version = "=0.2.0" }
+nova-cli = { path = "crates/nova-cli", version = "=0.2.0" }
+nova-codegen-cranelift = { path = "crates/nova-codegen-cranelift", version = "=0.2.0" }
+nova-codegen-llvm = { path = "crates/nova-codegen-llvm", version = "=0.2.0" }
+nova-diagnostics = { path = "crates/nova-diagnostics", version = "=0.2.0" }
+nova-doc = { path = "crates/nova-doc", version = "=0.2.0" }
+nova-driver = { path = "crates/nova-driver", version = "=0.2.0" }
+nova-fmt = { path = "crates/nova-fmt", version = "=0.2.0" }
+nova-hir = { path = "crates/nova-hir", version = "=0.2.0" }
+nova-index = { path = "crates/nova-index", version = "=0.2.0" }
+nova-lexer = { path = "crates/nova-lexer", version = "=0.2.0" }
+nova-lsp = { path = "crates/nova-lsp", version = "=0.2.0" }
+nova-mir = { path = "crates/nova-mir", version = "=0.2.0" }
+nova-parser = { path = "crates/nova-parser", version = "=0.2.0" }
+nova-pm = { path = "crates/nova-pm", version = "=0.2.0" }
+nova-resolver = { path = "crates/nova-resolver", version = "=0.2.0" }
+nova-runtime = { path = "crates/nova-runtime", version = "=0.2.0" }
+nova-std = { path = "std", version = "=0.2.0" }
+nova-typeck = { path = "crates/nova-typeck", version = "=0.2.0" }
+
 # Parsing & errors
 chumsky = "0.9"
 ariadne = "0.4"
diff --git a/crates/nova-ast/Cargo.toml b/crates/nova-ast/Cargo.toml
index 858e308..3ad7537 100644
--- a/crates/nova-ast/Cargo.toml
+++ b/crates/nova-ast/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-ast"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Abstract Syntax Tree node definitions for Nova"
+homepage.workspace = true
+description = "The Nova language's syntax tree (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,6 +16,6 @@ description = "Abstract Syntax Tree node definitions for Nova"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
+nova-diagnostics = { workspace = true }
 smol_str = { workspace = true }
 serde = { workspace = true }
diff --git a/crates/nova-bench-http/Cargo.toml b/crates/nova-bench-http/Cargo.toml
index fc4860e..ba3d20f 100644
--- a/crates/nova-bench-http/Cargo.toml
+++ b/crates/nova-bench-http/Cargo.toml
@@ -1,11 +1,16 @@
 [package]
 name = "nova-bench-http"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
+homepage.workspace = true
 description = "Keep-alive HTTP load generator for benchmarking Nova's std/http (no dependencies)"
+# Not on crates.io: `nova` does not need it (spec
+# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
+# §3.1).
+publish = false
 
 [[bin]]
 name = "nova-bench-http"
diff --git a/crates/nova-bundler/Cargo.toml b/crates/nova-bundler/Cargo.toml
index 0f224e6..6d8b58b 100644
--- a/crates/nova-bundler/Cargo.toml
+++ b/crates/nova-bundler/Cargo.toml
@@ -1,11 +1,16 @@
 [package]
 name = "nova-bundler"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
+homepage.workspace = true
 description = "Frontend bundler for Nova web projects"
+# Not on crates.io: `nova` does not need it (spec
+# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
+# §3.1).
+publish = false
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,6 +19,6 @@ description = "Frontend bundler for Nova web projects"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
+nova-diagnostics = { workspace = true }
 anyhow = { workspace = true }
 tracing = { workspace = true }
diff --git a/crates/nova-cli/Cargo.toml b/crates/nova-cli/Cargo.toml
index 44c0072..4a48aaa 100644
--- a/crates/nova-cli/Cargo.toml
+++ b/crates/nova-cli/Cargo.toml
@@ -1,11 +1,15 @@
 [package]
 name = "nova-cli"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "The nova compiler and toolchain CLI"
+homepage.workspace = true
+description = "The Nova programming language: compiler, formatter, language server, package manager and documentation generator"
+readme = "README.md"
+keywords = ["nova", "programming-language", "compiler", "lsp", "formatter"]
+categories = ["compilers", "command-line-utilities", "development-tools"]
 
 [[bin]]
 name = "nova"
@@ -16,22 +20,22 @@ path = "src/main.rs"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-lexer = { path = "../nova-lexer" }
-nova-ast = { path = "../nova-ast" }
-nova-parser = { path = "../nova-parser" }
-nova-driver = { path = "../nova-driver" }
-nova-pm = { path = "../nova-pm" }
-nova-fmt = { path = "../nova-fmt" }
-nova-lsp = { path = "../nova-lsp" }
+nova-diagnostics = { workspace = true }
+nova-lexer = { workspace = true }
+nova-ast = { workspace = true }
+nova-parser = { workspace = true }
+nova-driver = { workspace = true }
+nova-pm = { workspace = true }
+nova-fmt = { workspace = true }
+nova-lsp = { workspace = true }
 # `nova doc` (spec
 # docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md
 # §4).
-nova-doc = { path = "../nova-doc" }
+nova-doc = { workspace = true }
 # The package index: syncing, packing and publishing (spec
 # docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
 # §9).
-nova-index = { path = "../nova-index" }
+nova-index = { workspace = true }
 # `nova add json@1.4` checks its requirement.
 semver = { workspace = true }
 # `nova add` and `nova remove` edit nova.toml, keeping its comments and
@@ -45,7 +49,7 @@ tracing-subscriber = { workspace = true }
 # docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §4.1).
 # The runtime is already linked into `nova` through nova-codegen-cranelift,
 # so this adds no code; only a direct dependent receives the metadata.
-nova-runtime = { path = "../nova-runtime" }
+nova-runtime = { workspace = true }
 
 [build-dependencies]
 flate2 = { workspace = true }
diff --git a/crates/nova-codegen-cranelift/Cargo.toml b/crates/nova-codegen-cranelift/Cargo.toml
index 427829f..b18ca6c 100644
--- a/crates/nova-codegen-cranelift/Cargo.toml
+++ b/crates/nova-codegen-cranelift/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-codegen-cranelift"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Cranelift codegen backend for Nova (fast debug builds)"
+homepage.workspace = true
+description = "The Nova language's Cranelift code generator (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,10 +16,10 @@ description = "Cranelift codegen backend for Nova (fast debug builds)"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-mir = { path = "../nova-mir" }
-nova-hir = { path = "../nova-hir" }
-nova-runtime = { path = "../nova-runtime" }
+nova-diagnostics = { workspace = true }
+nova-mir = { workspace = true }
+nova-hir = { workspace = true }
+nova-runtime = { workspace = true }
 anyhow = { workspace = true }
 tracing = { workspace = true }
 rustc-hash = { workspace = true }
diff --git a/crates/nova-codegen-llvm/Cargo.toml b/crates/nova-codegen-llvm/Cargo.toml
index 91b6aa4..539b3bb 100644
--- a/crates/nova-codegen-llvm/Cargo.toml
+++ b/crates/nova-codegen-llvm/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-codegen-llvm"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "LLVM codegen backend for Nova (release builds)"
+homepage.workspace = true
+description = "The Nova language's LLVM IR code generator (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,9 +16,9 @@ description = "LLVM codegen backend for Nova (release builds)"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-hir = { path = "../nova-hir" }
-nova-mir = { path = "../nova-mir" }
+nova-diagnostics = { workspace = true }
+nova-hir = { workspace = true }
+nova-mir = { workspace = true }
 anyhow = { workspace = true }
 tracing = { workspace = true }
 
diff --git a/crates/nova-codegen-wasm/Cargo.toml b/crates/nova-codegen-wasm/Cargo.toml
index b4dcfcb..005e704 100644
--- a/crates/nova-codegen-wasm/Cargo.toml
+++ b/crates/nova-codegen-wasm/Cargo.toml
@@ -1,11 +1,16 @@
 [package]
 name = "nova-codegen-wasm"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
+homepage.workspace = true
 description = "WebAssembly codegen backend for Nova (browser builds)"
+# Not on crates.io: `nova` does not need it (spec
+# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
+# §3.1).
+publish = false
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,7 +19,7 @@ description = "WebAssembly codegen backend for Nova (browser builds)"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-mir = { path = "../nova-mir" }
+nova-diagnostics = { workspace = true }
+nova-mir = { workspace = true }
 anyhow = { workspace = true }
 tracing = { workspace = true }
diff --git a/crates/nova-diagnostics/Cargo.toml b/crates/nova-diagnostics/Cargo.toml
index dce9d8d..4096a0c 100644
--- a/crates/nova-diagnostics/Cargo.toml
+++ b/crates/nova-diagnostics/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-diagnostics"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Error reporting infrastructure for the Nova compiler"
+homepage.workspace = true
+description = "The Nova language's diagnostics (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
diff --git a/crates/nova-doc/Cargo.toml b/crates/nova-doc/Cargo.toml
index 534cb01..af85dc8 100644
--- a/crates/nova-doc/Cargo.toml
+++ b/crates/nova-doc/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-doc"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Documentation generator for Nova"
+homepage.workspace = true
+description = "The Nova language's documentation generator (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,13 +16,13 @@ description = "Documentation generator for Nova"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-ast = { path = "../nova-ast" }
-nova-lexer = { path = "../nova-lexer" }
-nova-resolver = { path = "../nova-resolver" }
-nova-typeck = { path = "../nova-typeck" }
-nova-driver = { path = "../nova-driver" }
-nova-pm = { path = "../nova-pm" }
+nova-diagnostics = { workspace = true }
+nova-ast = { workspace = true }
+nova-lexer = { workspace = true }
+nova-resolver = { workspace = true }
+nova-typeck = { workspace = true }
+nova-driver = { workspace = true }
+nova-pm = { workspace = true }
 # `///` docs as Markdown (spec
 # docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md
 # §9.1).
diff --git a/crates/nova-driver/Cargo.toml b/crates/nova-driver/Cargo.toml
index 85edd1d..16957e2 100644
--- a/crates/nova-driver/Cargo.toml
+++ b/crates/nova-driver/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-driver"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Compilation pipeline orchestration for Nova"
+homepage.workspace = true
+description = "The Nova language's compilation pipeline (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,17 +16,17 @@ description = "Compilation pipeline orchestration for Nova"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-ast = { path = "../nova-ast" }
-nova-lexer = { path = "../nova-lexer" }
-nova-parser = { path = "../nova-parser" }
-nova-resolver = { path = "../nova-resolver" }
-nova-typeck = { path = "../nova-typeck" }
-nova-hir = { path = "../nova-hir" }
-nova-mir = { path = "../nova-mir" }
-nova-pm = { path = "../nova-pm" }
-nova-codegen-cranelift = { path = "../nova-codegen-cranelift" }
-nova-codegen-llvm = { path = "../nova-codegen-llvm" }
+nova-diagnostics = { workspace = true }
+nova-ast = { workspace = true }
+nova-lexer = { workspace = true }
+nova-parser = { workspace = true }
+nova-resolver = { workspace = true }
+nova-typeck = { workspace = true }
+nova-hir = { workspace = true }
+nova-mir = { workspace = true }
+nova-pm = { workspace = true }
+nova-codegen-cranelift = { workspace = true }
+nova-codegen-llvm = { workspace = true }
 # Unpacking the runtime library nova-cli embeds (spec
 # 2026-10-07-phase-3-0-foundations-design.md §4.2).
 flate2 = { workspace = true }
diff --git a/crates/nova-fmt/Cargo.toml b/crates/nova-fmt/Cargo.toml
index ff07933..a2619db 100644
--- a/crates/nova-fmt/Cargo.toml
+++ b/crates/nova-fmt/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-fmt"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Opinionated code formatter for Nova"
+homepage.workspace = true
+description = "The Nova language's formatter (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,9 +16,9 @@ description = "Opinionated code formatter for Nova"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-ast = { path = "../nova-ast" }
-nova-lexer = { path = "../nova-lexer" }
-nova-parser = { path = "../nova-parser" }
+nova-diagnostics = { workspace = true }
+nova-ast = { workspace = true }
+nova-lexer = { workspace = true }
+nova-parser = { workspace = true }
 thiserror = { workspace = true }
 anyhow = { workspace = true }
diff --git a/crates/nova-hir/Cargo.toml b/crates/nova-hir/Cargo.toml
index add5f86..75e57ff 100644
--- a/crates/nova-hir/Cargo.toml
+++ b/crates/nova-hir/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-hir"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "High-level IR for Nova (desugared AST)"
+homepage.workspace = true
+description = "The Nova language's high-level IR (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,5 +16,5 @@ description = "High-level IR for Nova (desugared AST)"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-resolver = { path = "../nova-resolver" }
+nova-diagnostics = { workspace = true }
+nova-resolver = { workspace = true }
diff --git a/crates/nova-index/Cargo.toml b/crates/nova-index/Cargo.toml
index 78e4945..05b2a55 100644
--- a/crates/nova-index/Cargo.toml
+++ b/crates/nova-index/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-index"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "The package index for Nova: reading, downloading, packing and publishing"
+homepage.workspace = true
+description = "The Nova language's package index client (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,8 +16,8 @@ description = "The package index for Nova: reading, downloading, packing and pub
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-pm = { path = "../nova-pm" }
+nova-diagnostics = { workspace = true }
+nova-pm = { workspace = true }
 semver = { workspace = true }
 # The index's lines and config.json (spec
 # docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
diff --git a/crates/nova-lexer/Cargo.toml b/crates/nova-lexer/Cargo.toml
index a8db55d..7bd49b6 100644
--- a/crates/nova-lexer/Cargo.toml
+++ b/crates/nova-lexer/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-lexer"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Lexer for the Nova programming language"
+homepage.workspace = true
+description = "The Nova language's lexer (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,7 +16,7 @@ description = "Lexer for the Nova programming language"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
+nova-diagnostics = { workspace = true }
 logos = { workspace = true }
 thiserror = { workspace = true }
 
diff --git a/crates/nova-lsp/Cargo.toml b/crates/nova-lsp/Cargo.toml
index ed474ad..22019c3 100644
--- a/crates/nova-lsp/Cargo.toml
+++ b/crates/nova-lsp/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-lsp"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Language Server Protocol implementation for Nova"
+homepage.workspace = true
+description = "The Nova language's language server (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,19 +16,19 @@ description = "Language Server Protocol implementation for Nova"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-lexer = { path = "../nova-lexer" }
-nova-ast = { path = "../nova-ast" }
-nova-parser = { path = "../nova-parser" }
-nova-resolver = { path = "../nova-resolver" }
-nova-typeck = { path = "../nova-typeck" }
-nova-hir = { path = "../nova-hir" }
-nova-driver = { path = "../nova-driver" }
-nova-pm = { path = "../nova-pm" }
-nova-fmt = { path = "../nova-fmt" }
+nova-diagnostics = { workspace = true }
+nova-lexer = { workspace = true }
+nova-ast = { workspace = true }
+nova-parser = { workspace = true }
+nova-resolver = { workspace = true }
+nova-typeck = { workspace = true }
+nova-hir = { workspace = true }
+nova-driver = { workspace = true }
+nova-pm = { workspace = true }
+nova-fmt = { workspace = true }
 # Hover hides an example's hidden lines by the site's rule (spec
 # docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md §11.4).
-nova-doc = { path = "../nova-doc" }
+nova-doc = { workspace = true }
 lsp-server = { workspace = true }
 lsp-types = { workspace = true }
 serde_json = { workspace = true }
diff --git a/crates/nova-mir/Cargo.toml b/crates/nova-mir/Cargo.toml
index b6664d7..cfc5e33 100644
--- a/crates/nova-mir/Cargo.toml
+++ b/crates/nova-mir/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-mir"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Mid-level IR for Nova (3-address style)"
+homepage.workspace = true
+description = "The Nova language's mid-level IR (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,9 +16,9 @@ description = "Mid-level IR for Nova (3-address style)"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-hir = { path = "../nova-hir" }
-nova-resolver = { path = "../nova-resolver" }
+nova-diagnostics = { workspace = true }
+nova-hir = { workspace = true }
+nova-resolver = { workspace = true }
 rustc-hash = { workspace = true }
 
 [dev-dependencies]
diff --git a/crates/nova-parser/Cargo.toml b/crates/nova-parser/Cargo.toml
index f115bb3..acc72af 100644
--- a/crates/nova-parser/Cargo.toml
+++ b/crates/nova-parser/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-parser"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Parser for the Nova programming language"
+homepage.workspace = true
+description = "The Nova language's parser (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,9 +16,9 @@ description = "Parser for the Nova programming language"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-lexer = { path = "../nova-lexer" }
-nova-ast = { path = "../nova-ast" }
+nova-diagnostics = { workspace = true }
+nova-lexer = { workspace = true }
+nova-ast = { workspace = true }
 thiserror = { workspace = true }
 
 [dev-dependencies]
diff --git a/crates/nova-pm/Cargo.toml b/crates/nova-pm/Cargo.toml
index 22858ee..9e77800 100644
--- a/crates/nova-pm/Cargo.toml
+++ b/crates/nova-pm/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-pm"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Package manager for Nova"
+homepage.workspace = true
+description = "The Nova language's package manager (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,8 +16,8 @@ description = "Package manager for Nova"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-lexer = { path = "../nova-lexer" }
+nova-diagnostics = { workspace = true }
+nova-lexer = { workspace = true }
 crc32fast = { workspace = true }
 semver = { workspace = true }
 toml_edit = { workspace = true }
diff --git a/crates/nova-resolver/Cargo.toml b/crates/nova-resolver/Cargo.toml
index ad9f5c7..1f1c3b8 100644
--- a/crates/nova-resolver/Cargo.toml
+++ b/crates/nova-resolver/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-resolver"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Name resolution and module graph for Nova"
+homepage.workspace = true
+description = "The Nova language's name resolver (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,12 +16,12 @@ description = "Name resolution and module graph for Nova"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-ast = { path = "../nova-ast" }
-nova-lexer = { path = "../nova-lexer" }
-nova-parser = { path = "../nova-parser" }
+nova-diagnostics = { workspace = true }
+nova-ast = { workspace = true }
+nova-lexer = { workspace = true }
+nova-parser = { workspace = true }
 # std's sources (spec 2026-10-07-phase-3-0-foundations-design.md §3).
-nova-std = { path = "../../std" }
+nova-std = { workspace = true }
 indexmap = { workspace = true }
 rustc-hash = { workspace = true }
 thiserror = { workspace = true }
diff --git a/crates/nova-runtime/Cargo.toml b/crates/nova-runtime/Cargo.toml
index 541d75a..c90d6cb 100644
--- a/crates/nova-runtime/Cargo.toml
+++ b/crates/nova-runtime/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-runtime"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "GC and async runtime for Nova (linked into compiled binaries)"
+homepage.workspace = true
+description = "The Nova language's runtime: garbage collector and async executor (internal to the nova toolchain)"
+readme = "../README.md"
 # `links` lets this crate's build script hand metadata to the crates that
 # depend on it: nova-cli's build script reads DEP_NOVA_RUNTIME_MANIFEST_DIR
 # to find this source and build the staticlib it embeds
@@ -22,7 +24,7 @@ bench = false
 crate-type = ["rlib", "staticlib"]
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
+nova-diagnostics = { workspace = true }
 anyhow = { workspace = true }
 tracing = { workspace = true }
 
diff --git a/crates/nova-test/Cargo.toml b/crates/nova-test/Cargo.toml
index 43ef946..fa067e9 100644
--- a/crates/nova-test/Cargo.toml
+++ b/crates/nova-test/Cargo.toml
@@ -1,11 +1,16 @@
 [package]
 name = "nova-test"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
+homepage.workspace = true
 description = "Test runner for Nova projects"
+# Not on crates.io: `nova` does not need it (spec
+# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
+# §3.1).
+publish = false
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,6 +19,6 @@ description = "Test runner for Nova projects"
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
+nova-diagnostics = { workspace = true }
 anyhow = { workspace = true }
 tracing = { workspace = true }
diff --git a/crates/nova-typeck/Cargo.toml b/crates/nova-typeck/Cargo.toml
index 0a30940..52dd2f5 100644
--- a/crates/nova-typeck/Cargo.toml
+++ b/crates/nova-typeck/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-typeck"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "Type checker and inference for Nova (Hindley-Milner + extensions)"
+homepage.workspace = true
+description = "The Nova language's type checker (internal to the nova toolchain)"
+readme = "../README.md"
 
 [lib]
 # Not a benchmark target: `cargo bench` would otherwise build this under
@@ -14,10 +16,10 @@ description = "Type checker and inference for Nova (Hindley-Milner + extensions)
 bench = false
 
 [dependencies]
-nova-diagnostics = { path = "../nova-diagnostics" }
-nova-ast = { path = "../nova-ast" }
-nova-hir = { path = "../nova-hir" }
-nova-resolver = { path = "../nova-resolver" }
+nova-diagnostics = { workspace = true }
+nova-ast = { workspace = true }
+nova-hir = { workspace = true }
+nova-resolver = { workspace = true }
 indexmap = { workspace = true }
 rustc-hash = { workspace = true }
 thiserror = { workspace = true }
diff --git a/std/Cargo.toml b/std/Cargo.toml
index 66cf541..2b301bd 100644
--- a/std/Cargo.toml
+++ b/std/Cargo.toml
@@ -1,11 +1,13 @@
 [package]
 name = "nova-std"
-version = "0.2.0"
+version.workspace = true
 edition.workspace = true
 rust-version.workspace = true
 license.workspace = true
 repository.workspace = true
-description = "The Nova standard library's sources, embedded for the compiler"
+homepage.workspace = true
+description = "The Nova language's standard library sources (internal to the nova toolchain)"
+readme = "../crates/README.md"
 
 [lib]
 path = "lib.rs"
````

What it does, by manifest:
- the root: `[workspace.package]` gains `version = "0.2.0"` (3.6b's
  release commit sets 0.3.0) and `homepage`, and `repository` becomes
  `https://github.com/Sakeerin/nova`; `[workspace.dependencies]` gains the
  19 exact pins, alphabetically (nova-std's path is `std`), with comments
  saying what a bump touches and why the pins are exact;
- every member: `version.workspace = true` and `homepage.workspace = true`
  (decisions 1, 2); each normal, build and target-specific dependency on a
  workspace crate becomes `{ workspace = true }`, its comments kept;
  dev-dependencies stay `{ path = ... }`;
- the 19 published: a description naming Nova (decision 3) and `readme`
  (`"README.md"` for nova-cli, `"../crates/README.md"` for nova-std,
  `"../README.md"` for the other 17); nova-cli alone: §3.3's keywords and
  categories;
- `nova-bench-http`, `nova-bundler`, `nova-codegen-wasm` and `nova-test`:
  `publish = false`, with a comment citing §3.1.

- [ ] **Step 4: The licence texts**

Copies of the root's, byte for byte, in the 18 published `crates/nova-*`
directories and `std/` (copies, not symlinks: a Windows checkout turns a
symlink into a text file):

Run: `cd /home/user/nova && for d in crates/nova-ast crates/nova-cli crates/nova-codegen-cranelift crates/nova-codegen-llvm crates/nova-diagnostics crates/nova-doc crates/nova-driver crates/nova-fmt crates/nova-hir crates/nova-index crates/nova-lexer crates/nova-lsp crates/nova-mir crates/nova-parser crates/nova-pm crates/nova-resolver crates/nova-runtime crates/nova-typeck std; do cp LICENSE-MIT LICENSE-APACHE "$d/"; done`

Run: `cd /home/user/nova && git status --short -- 'crates/*/LICENSE-*' 'std/LICENSE-*' | wc -l`
Expected: `38`.

- [ ] **Step 5: The READMEs**

`crates/README.md`, which the 18 libraries share (cargo packages a README
outside the crate's directory, at the package's root, as `README.md`),
lines 1677-1698 of this plan:

Run: `cd /home/user/nova && sed -n '1677,1698p' "$PLAN" > crates/README.md`

````markdown
# Part of the Nova toolchain

This crate is one of the libraries that make up `nova`, the toolchain of
the [Nova programming language](https://github.com/Sakeerin/nova): its
compiler, formatter, language server, package manager and documentation
generator.

Its API is internal to `nova`. It is published only so that `nova` can be
installed from crates.io, and it changes with every release, without
notice. Every crate of a release depends on the others at exactly that
release's version.

To install `nova`:

```sh
cargo install nova-cli --locked
```

The [repository](https://github.com/Sakeerin/nova) has the language's
documentation, the ready-built releases and the install scripts. The
crates are licensed under either of the MIT licence or the Apache License,
Version 2.0, at your option.
````

`crates/nova-cli/README.md`, nova-cli's crates.io page (decision 43),
lines 1707-1766 of this plan:

Run: `cd /home/user/nova && sed -n '1707,1766p' "$PLAN" > crates/nova-cli/README.md`

````markdown
# nova

`nova` is the toolchain of Nova, a statically typed programming language
with type inference, pattern matching and async functions. One binary
makes, runs, builds and tests projects, and it is also Nova's formatter
(`nova fmt`), language server (`nova lsp`), package manager (`nova add`,
`nova publish`) and documentation generator (`nova doc`).

## Install

From crates.io:

```sh
cargo install nova-cli --locked
```

`--locked` builds with the dependency versions the release was tested
with. The install also builds Nova's runtime library and embeds it in
`nova`, so the installed binary needs nothing beside it.

Or a ready-built `nova` from the latest GitHub release, checked against
the release's `SHA256SUMS`. On Linux and macOS, into `~/.nova/bin`:

```sh
curl -sSfL https://github.com/Sakeerin/nova/releases/latest/download/install.sh | sh
```

On Windows, into `%USERPROFILE%\.nova\bin`, which the script adds to your
PATH:

```powershell
irm https://github.com/Sakeerin/nova/releases/latest/download/install.ps1 | iex
```

## What it needs

`nova run` needs nothing else. `nova build` and `nova test` write native
executables, so they need a C linker: `cc` on Linux and macOS, and the
Visual Studio Build Tools on Windows.

## Start

```sh
nova new hello
cd hello
nova run
```

## Links

- [The repository](https://github.com/Sakeerin/nova): the language's
  documentation, the changelog and the releases.
- The VS Code extension is attached to each release as
  `nova-vscode-<version>.vsix`.

The other `nova-*` crates on crates.io are `nova`'s own libraries. Their
API is internal and changes with every release.

Licensed under either of the MIT licence or the Apache License, Version
2.0, at your option.
````

The members glob `crates/*` matches `crates/README.md` too; cargo 1.97 and
1.78 both skip it, silently.

- [ ] **Step 6: `release.yml` reads the workspace's version (decision 5)**

Lines 1779-1828 of this plan:

Run: `cd /home/user/nova && sed -n '1779,1828p' "$PLAN" > "$P/36a-t1-release.diff" && git apply "$P/36a-t1-release.diff"`

````diff
diff --git a/.github/workflows/release.yml b/.github/workflows/release.yml
index 5df7add..64c527e 100644
--- a/.github/workflows/release.yml
+++ b/.github/workflows/release.yml
@@ -18,6 +18,7 @@ on:
       - crates/nova-runtime/build.rs
       - crates/nova-cli/Cargo.toml
       - crates/nova-runtime/Cargo.toml
+      - Cargo.toml
       - Cargo.lock
       - tools/vscode-nova/**
 
@@ -59,7 +60,9 @@ jobs:
         shell: bash
         run: |
           set -euo pipefail
-          version=$(tr -d '\r' < crates/nova-cli/Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
+          # The workspace's version: [workspace.package]'s, the only line
+          # of the root Cargo.toml that starts with `version = `.
+          version=$(tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
           name="nova-$version-${{ matrix.target }}"
           mkdir -p "dist/$name"
           cp README.md LICENSE-MIT LICENSE-APACHE "dist/$name/"
@@ -134,7 +137,7 @@ jobs:
         shell: bash
         run: |
           set -euo pipefail
-          version=$(tr -d '\r' < crates/nova-cli/Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
+          version=$(tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
           site="$RUNNER_TEMP/std-docs"
           mkdir "$site"
           (cd "$site" && "$GITHUB_WORKSPACE/target/debug/nova" doc --std)
@@ -161,7 +164,7 @@ jobs:
         shell: bash
         run: |
           set -euo pipefail
-          version=$(tr -d '\r' < crates/nova-cli/Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
+          version=$(tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
           # An archive is nova-<version>-<target>; the extension is
           # nova-vscode-<version>.vsix; std's site is
           # nova-std-docs-<version>.tar.gz.
@@ -177,7 +180,7 @@ jobs:
           bash .github/scripts/release-notes.sh "$version" CHANGELOG.md > release/notes.md
           if [ "$GITHUB_EVENT_NAME" = "push" ]; then
             if [ "$GITHUB_REF_NAME" != "v$version" ]; then
-              echo "::error::the tag $GITHUB_REF_NAME does not match nova-cli's version $version"
+              echo "::error::the tag $GITHUB_REF_NAME does not match the workspace's version $version"
               exit 1
             fi
             tag=$GITHUB_REF_NAME
````

- [ ] **Step 7: Run the tests**

Run: `cd /home/user/nova && cargo test --locked -p nova-cli --test release_manifests --test vscode_extension --test build_support 2>&1 | grep -E "^test result|FAILED"`
Expected: three lines, in this order: `test result: ok. 12 passed`
(`build_support`, which still picks the runtime's route now that its
manifest has `workspace = true` lines), `test result: ok. 7 passed` and
`test result: ok. 2 passed`.

Run: `cd /home/user/nova && cargo build --locked --workspace 2>&1 | tail -n 1 && grep -c '^version = ' Cargo.toml && cargo +1.78 metadata --no-deps --offline --format-version 1 > "$P/t1-metadata-178.json" && echo "1.78 reads the manifests"`
Expected: `Finished ...`, `1`, `1.78 reads the manifests`.

- [ ] **Step 8: Checks and commit**

The checks (Conventions), with
`msrv_test crates/nova-cli/tests/release_manifests.rs serde_json toml_edit`.
`$SC .github/scripts/publish-crates.sh` reports only `SC2034` (`crates
appears unused`), until Task 3. `$AL .github/workflows/release.yml`
reports the four findings it reported at `a17f140`, all in prepare's
"Checksums" step: three `SC2010` (`ls | grep`) and one `SC2035`.

Stage: `git add Cargo.toml crates/*/Cargo.toml std/Cargo.toml crates/*/LICENSE-MIT crates/*/LICENSE-APACHE std/LICENSE-MIT std/LICENSE-APACHE crates/README.md crates/nova-cli/README.md crates/nova-cli/tests/release_manifests.rs .github/scripts/publish-crates.sh .github/workflows/release.yml`
Run: `git diff --cached --stat | tail -n 1`
Expected: `67 files changed`.
Message: `crates: one workspace version, exact pins between the crates, four unpublished`,
body: the workspace sets the version once and pins the 19 crates nova
needs on each other exactly, in [workspace.dependencies]; nova-bench-http,
nova-bundler, nova-codegen-wasm and nova-test are publish = false. Each
published crate gets the real repository and homepage, a description
naming Nova, a README and both licence texts, and release.yml reads the
workspace's version. release_manifests.rs holds them to it (spec 3.6a §3,
§11.1). Cargo.lock is unchanged.

---

### Task 2: The package gate, in CI (spec §4)

**Files:**
- Create: `.github/scripts/package-gate.sh`
- Modify: `.github/workflows/ci.yml` (a `package` job after `install`)

**Interfaces:** `package-gate.sh WORKDIR` (Contracts).

- [ ] **Step 1: The script**

`.github/scripts/package-gate.sh`, lines 1880-2022 of this plan:

Run: `cd /home/user/nova && sed -n '1880,2022p' "$PLAN" > .github/scripts/package-gate.sh`

````bash
#!/usr/bin/env bash
# The package route (spec
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# §4): the 19 crates publish-crates.sh lists package and verify as
# crates.io would check them; each packaged Cargo.lock names only what the
# workspace's names, at the same versions; `nova` installs from the unpacked
# packages as `cargo install nova-cli` would from crates.io, a release build
# that embeds the runtime; and that nova passes gate.sh --test. CI's
# `package` job runs it, from the repository's root, on all three systems.
#
# Usage: package-gate.sh WORKDIR
#   WORKDIR  where to package, unpack and install; it must not exist yet
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: package-gate.sh WORKDIR" >&2
  exit 2
fi
mkdir "$1"
workdir=$(cd "$1" && pwd)
# On Windows, Git Bash's /d/a/... paths mean nothing to cargo, and the
# config file below takes no backslashes: D:/a/... suits both.
if command -v cygpath > /dev/null 2>&1; then
  workdir=$(cygpath -m "$workdir")
fi

# The workspace's version: [workspace.package]'s, the only line of the
# root Cargo.toml that starts with `version = `.
version=$(tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
# The crates, in publish order, from publish-crates.sh's `crates=(` list,
# so that there is one list.
crates=()
while IFS= read -r crate; do
  crates+=("$crate")
done < <(tr -d '\r' < .github/scripts/publish-crates.sh | awk '
  /^crates=\(/ { listing = 1; next }
  listing && /^\)/ { exit }
  listing { gsub(/[ \t]/, ""); if ($0 != "" && $0 !~ /^#/) print }')
if [ "${#crates[@]}" -ne 19 ] || [ -z "$version" ]; then
  echo "package-gate: expected 19 crates and a version, found ${#crates[@]} and '$version'" >&2
  exit 1
fi
echo "package-gate: ${#crates[@]} crates at $version, in $workdir"

export CARGO_TARGET_DIR="$workdir/target"
began=$SECONDS
step_began=$SECONDS
step_done() {
  echo "package-gate: $1 took $((SECONDS - step_began)) s"
  step_began=$SECONDS
}

# 0. Everything Cargo.lock names, for every platform, so that the rest runs
#    offline: the packages' verification and the install find exactly the
#    workspace's versions and nothing newer.
cargo fetch --locked
step_done "fetching the locked dependencies"

# 1. Package and verify, crates.io's own check, with cargo's local overlay
#    standing in for the not yet published nova crates. Verification builds
#    in the dev profile, so it never embeds the runtime; step 3 does.
package_args=()
for crate in "${crates[@]}"; do
  package_args+=(-p "$crate")
done
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo package --locked --offline "${package_args[@]}"
packages="$workdir/target/package"
total=0
for crate in "${crates[@]}"; do
  file="$crate-$version.crate"
  # Relative names: GNU tar takes the `D:` of D:/a/... for a remote host.
  files=$(cd "$packages" && tar -tzf "$file" | wc -l | tr -d ' ')
  bytes=$(wc -c < "$packages/$file" | tr -d ' ')
  total=$((total + bytes))
  printf 'package-gate: %-36s %4s files %9s bytes\n' "$file" "$files" "$bytes"
done
echo "package-gate: ${#crates[@]} packages, $total bytes"
step_done "packaging and verifying"

# 2. Each packaged Cargo.lock names only packages the workspace's
#    Cargo.lock has, at the same versions, so `cargo install nova-cli
#    --locked` builds what CI built.
mkdir "$workdir/unpacked"
for crate in "${crates[@]}"; do
  (cd "$packages" && tar -xzf "$crate-$version.crate" -C ../../unpacked)
done
lock_entries() {
  tr -d '\r' < "$1" | awk '
    /^\[\[package\]\]/ { name = "" }
    /^name = / { name = $3; gsub(/"/, "", name) }
    /^version = / && name != "" { v = $3; gsub(/"/, "", v); print name " " v }' |
    LC_ALL=C sort -u
}
lock_entries Cargo.lock > "$workdir/workspace-lock.txt"
for crate in "${crates[@]}"; do
  lock="$workdir/unpacked/$crate-$version/Cargo.lock"
  if [ ! -f "$lock" ]; then
    echo "package-gate: $crate's package has no Cargo.lock" >&2
    exit 1
  fi
  extra=$(lock_entries "$lock" | LC_ALL=C comm -23 - "$workdir/workspace-lock.txt")
  if [ -n "$extra" ]; then
    echo "package-gate: $crate's Cargo.lock has what the workspace's has not:" >&2
    echo "$extra" >&2
    exit 1
  fi
done
echo "package-gate: every packaged Cargo.lock agrees with the workspace's"
step_done "comparing the lockfiles"

# 3. Install as crates.io would: from the unpacked nova-cli, its 18
#    libraries patched in for crates.io's copies. Cargo reads config from
#    the current directory up, and nova-cli's build script runs its nested
#    runtime build from nova-cli's unpacked directory, so that build reads
#    the same patches; it ignores CARGO_TARGET_DIR.
mkdir "$workdir/unpacked/.cargo"
{
  echo "[patch.crates-io]"
  for crate in "${crates[@]}"; do
    if [ "$crate" != nova-cli ]; then
      echo "$crate = { path = \"$workdir/unpacked/$crate-$version\" }"
    fi
  done
} > "$workdir/unpacked/.cargo/config.toml"
(cd "$workdir/unpacked" &&
  cargo install --path "nova-cli-$version" --offline --root "$workdir/root")
installed=$(ls -A "$workdir/root/bin")
case "$installed" in
  nova|nova.exe) ;;
  *)
    echo "package-gate: the install put more than nova into bin/: $installed" >&2
    exit 1
    ;;
esac
step_done "installing nova from the packages"

# 4. The installed nova carries its runtime, and makes, runs, builds and
#    tests a project.
bash .github/scripts/gate.sh "$workdir/root/bin/nova" "$workdir/gate" --test
step_done "the gate"

echo "package-gate: passed in $((SECONDS - began)) s"
````

Its steps are §4's, with a step 0 (decision 6): `cargo fetch --locked`;
`cargo package --locked --offline` with a `-p` for each of the 19, which
packages and verifies them in cargo's own order, then prints each
`.crate`'s file count and size in the list's order; each packaged
`Cargo.lock` against the workspace's; the install from the unpacked
packages, with no `--locked` (decision 7); `gate.sh --test`. The nested
runtime build finds the patches because nova-cli's build script runs it
from nova-cli's unpacked directory, and cargo reads config from the
current directory up.

- [ ] **Step 2: Run it, on Linux**

It runs on the committed crates (Task 1's commit), with `HOME` moved
(Global Constraints). It takes about 4 minutes and 1.5 GB.

Run: `cd /home/user/nova && rm -rf "$P/pg36a" "$P/home-pg36a" && mkdir "$P/home-pg36a" && env HOME="$P/home-pg36a" CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" bash .github/scripts/package-gate.sh "$P/pg36a" > "$P/t2-package-gate.log" 2>&1; echo "exit $?"; grep -E '^(package-gate|gate|runtime):' "$P/t2-package-gate.log"`
Expected: `exit 0`, then these lines, as the prototype's run printed them
(durations vary; the sizes may differ by a few bytes):

````text
package-gate: 19 crates at 0.2.0, in /tmp/…/pg36a
package-gate: fetching the locked dependencies took 0 s
package-gate: nova-diagnostics-0.2.0.crate           14 files     16240 bytes
package-gate: nova-std-0.2.0.crate                   25 files    133108 bytes
package-gate: nova-ast-0.2.0.crate                   12 files     10687 bytes
package-gate: nova-lexer-0.2.0.crate                 23 files     26300 bytes
package-gate: nova-runtime-0.2.0.crate               30 files    252631 bytes
package-gate: nova-parser-0.2.0.crate                23 files     37528 bytes
package-gate: nova-pm-0.2.0.crate                    24 files     37013 bytes
package-gate: nova-fmt-0.2.0.crate                   27 files     51181 bytes
package-gate: nova-index-0.2.0.crate                 27 files     45059 bytes
package-gate: nova-resolver-0.2.0.crate               9 files     53011 bytes
package-gate: nova-hir-0.2.0.crate                    8 files     28066 bytes
package-gate: nova-mir-0.2.0.crate                   12 files     91177 bytes
package-gate: nova-typeck-0.2.0.crate                12 files    191047 bytes
package-gate: nova-codegen-cranelift-0.2.0.crate      8 files     22551 bytes
package-gate: nova-codegen-llvm-0.2.0.crate           9 files     22752 bytes
package-gate: nova-driver-0.2.0.crate                20 files     77959 bytes
package-gate: nova-doc-0.2.0.crate                   22 files     61224 bytes
package-gate: nova-lsp-0.2.0.crate                   22 files     48847 bytes
package-gate: nova-cli-0.2.0.crate                   41 files    230197 bytes
package-gate: 19 packages, 1436578 bytes
package-gate: packaging and verifying took 106 s
package-gate: every packaged Cargo.lock agrees with the workspace's
package-gate: comparing the lockfiles took 0 s
package-gate: installing nova from the packages took 129 s
runtime: embedded
gate: passed
package-gate: the gate took 1 s
package-gate: passed in 236 s
````

In the log: `Locking 174 packages to latest compatible versions` in the
install (the offline resolve), and no warning that a patch was unused.
Then: `ls -l "$P/pg36a/root/bin"` shows `nova` alone, about 25.0 MB (the
prototype's: 24,997,376 bytes, with a 7,525,643-byte runtime payload).
Ledger the step durations and the sizes, then remove both directories:
`rm -rf "$P/pg36a" "$P/home-pg36a"`. `ls "$HOME/.nova"` still fails, as
it did before: nothing reached the real home.

- [ ] **Step 3: The CI job**

In `.github/workflows/ci.yml`, after the `install` job, lines 2092-2124 of this plan:

Run: `cd /home/user/nova && sed -n '2092,2124p' "$PLAN" > "$P/36a-t2-ci.diff" && git apply "$P/36a-t2-ci.diff"`

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index ec91101..57c716f 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -244,6 +244,28 @@ jobs:
       - name: std survives packaging
         run: cargo package -p nova-std --locked
 
+  # The package route (spec
+  # docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
+  # §4): the 19 crates crates.io will get package and verify, their
+  # lockfiles agree with the workspace's, and a `nova` installed from the
+  # packages, as `cargo install nova-cli` installs from crates.io, embeds
+  # its runtime and passes the Phase 3.0 gate. The install job above
+  # installs from the checkout; this one from what would be uploaded.
+  package:
+    name: Packaged nova (${{ matrix.os }})
+    runs-on: ${{ matrix.os }}
+    strategy:
+      fail-fast: false
+      matrix:
+        os: [ubuntu-latest, windows-latest, macos-latest]
+    steps:
+      - uses: actions/checkout@v4
+      - uses: dtolnay/rust-toolchain@stable
+      - uses: Swatinem/rust-cache@v2
+      - name: Package the 19 crates, install nova from them, run the gate
+        shell: bash
+        run: bash .github/scripts/package-gate.sh "$RUNNER_TEMP/package-gate"
+
   # The VS Code extension (spec
   # docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §8): it
   # is platform-neutral, and the Rust gate test drives `nova lsp` on all
````

It runs on ubuntu, windows and macos with `fail-fast: false`, set up as
the `install` job is. Its first runs on Windows and macOS are the pull
request's (Review Focus 1).

- [ ] **Step 4: Review the 19 package lists once (spec §14 risk 5)**

Run: `cd /home/user/nova && for c in $(sed -n '/^crates=(/,/^)/s/^  \(nova-[a-z-]*\)$/\1/p' .github/scripts/publish-crates.sh); do cargo package --list --locked --offline -p "$c" | sed "s|^|$c: |"; done > "$P/t2-package-lists.txt" && wc -l < "$P/t2-package-lists.txt" && grep -v -E ': (\.cargo_vcs_info\.json|Cargo\.toml|Cargo\.toml\.orig|Cargo\.lock|LICENSE-MIT|LICENSE-APACHE|README\.md|(src|tests)/.*)$' "$P/t2-package-lists.txt"`
Expected: `368` lines in all (the 19 file counts of Step 2), and only
these, outside the manifests, licences, READMEs, `src/` and `tests/`:
`nova-std`'s `lib.rs` and its 17 `<module>/lib.nova`; `nova-lexer`'s
`benches/lex.rs`; `nova-runtime`'s `build.rs`; `nova-doc`'s
`assets/nova.css` and `assets/nova.js`; `nova-cli`'s `build.rs` and
`build_support.rs`. Anything else is a stray file: stop and ask.

- [ ] **Step 5: Checks and commit**

`$SC .github/scripts/package-gate.sh` and `$AL .github/workflows/ci.yml`:
no output. `git diff --stat a17f140 -- Cargo.lock`: no output.
Stage `.github/scripts/package-gate.sh` and `.github/workflows/ci.yml`.
Message: `ci: package-gate.sh and "Packaged nova", the package route on three systems`,
body: the 19 crates package and verify with cargo package --locked, their
lockfiles name only what the workspace's does, nova installs from the
unpacked packages with a [patch.crates-io] for the 18 libraries, a release
build that embeds the runtime, and the gate passes on it (spec 3.6a §4).
CI runs it on Linux, macOS and Windows. Locally: <the total> s.

---

### Task 3: The publish script (spec §5.2, §11.2)

**Files:**
- Create: `crates/nova-cli/tests/publish_script.rs` (Unix only)
- Modify: `.github/scripts/publish-crates.sh` (replaced whole; its list is
  unchanged)

**Interfaces:** `publish-crates.sh VERSION` (Contracts).

- [ ] **Step 1: Write the failing test**

`crates/nova-cli/tests/publish_script.rs`, lines 2172-2627 of this plan, with
the fake `cargo` of decision 19:

Run: `cd /home/user/nova && sed -n '2172,2627p' "$PLAN" > crates/nova-cli/tests/publish_script.rs`

````rust
//! The publish script (spec
//! `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
//! §5.2, §11.2): `.github/scripts/publish-crates.sh`, run with `bash`
//! against a fake `cargo` and a sparse index served by the loopback server.
//! Nothing reaches the internet, and no real cargo publishes anything.
//!
//! The fake `cargo` appends its arguments, one line per run, to
//! `cargo.log`, and answers as the test's `answers` file says, one line per
//! crate:
//! - no line, or `<crate> ok`: it succeeds;
//! - `<crate> 429 <n>`: its first `n` runs for that crate fail as crates.io
//!   refuses for its rate limit on new crates, then it succeeds;
//! - `<crate> fail`: every run for that crate fails for another reason.
//!
//! It also fails when `CARGO_REGISTRY_TOKEN` did not reach it.

#![cfg(unix)]

#[path = "../../nova-index/tests/support/mod.rs"]
mod support;

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use support::{Response, Server};

const TOKEN: &str = "secret-token-value";
const VERSION: &str = "0.3.0";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn script() -> PathBuf {
    root()
        .join(".github")
        .join("scripts")
        .join("publish-crates.sh")
}

/// The script's `crates=( … )` list, one crate per line, so that the
/// order lives in the script alone.
fn publish_list() -> Vec<String> {
    let text = std::fs::read_to_string(script()).expect("read publish-crates.sh");
    let mut lines = text.lines().map(str::trim);
    assert!(lines.any(|line| line == "crates=("), "no `crates=(` line");
    let list: Vec<String> = lines
        .take_while(|line| *line != ")")
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_string)
        .collect();
    assert_eq!(list.len(), 19, "{list:?}");
    list
}

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-publish-script-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// The fake `cargo` (see the module's comment).
const FAKE_CARGO: &str = r#"#!/bin/sh
dir=$FAKE_CARGO_DIR
echo "$*" >> "$dir/cargo.log"
crate=
previous=
for arg in "$@"; do
  if [ "$previous" = "-p" ]; then crate=$arg; fi
  previous=$arg
done
if [ -z "${CARGO_REGISTRY_TOKEN:-}" ]; then
  echo "fake cargo: no CARGO_REGISTRY_TOKEN" >&2
  exit 3
fi
answer=$(awk -v crate="$crate" '$1 == crate { $1 = ""; sub(/^ /, ""); print; exit }' "$dir/answers")
runs=$(grep -cx "publish --locked -p $crate" "$dir/cargo.log")
case $answer in
  "429 "*)
    if [ "$runs" -le "${answer#429 }" ]; then
      echo "error: failed to publish to registry at https://crates.io" >&2
      echo "" >&2
      echo "Caused by:" >&2
      echo "  the remote server responded with an error (status 429 Too Many Requests): You have published too many new crates in a short period of time. Please try again after Mon, 12 Oct 2026 12:00:00 GMT or email help@crates.io to have your limit increased." >&2
      exit 101
    fi
    ;;
  fail)
    echo "error: failed to publish to registry at https://crates.io" >&2
    echo "" >&2
    echo "Caused by:" >&2
    echo "  the remote server responded with an error (status 400 Bad Request): fake cargo refused $crate" >&2
    exit 101
    ;;
esac
echo "   Uploading $crate (fake)"
echo "   Published $crate (fake)" >&2
"#;

/// A sparse index on the loopback server: each `(path, status, body)`,
/// and 404 for any other path.
fn index(files: &[(String, u16, String)]) -> Server {
    let files: HashMap<String, (u16, String)> = files
        .iter()
        .map(|(path, status, body)| (path.clone(), (*status, body.clone())))
        .collect();
    Server::start(move |request| match files.get(&request.path) {
        Some((status, body)) => Response::with_status(*status, body.clone()),
        None => Response::status(404),
    })
}

/// A crate's path in the sparse index: every listed name has four
/// characters or more, so `<chars 1-2>/<chars 3-4>/<name>`.
fn index_path(name: &str) -> String {
    assert!(name.len() >= 4, "{name}");
    format!("/{}/{}/{name}", &name[0..2], &name[2..4])
}

/// One line of a crate's index file, as crates.io writes it.
fn index_line(name: &str, version: &str, yanked: bool) -> String {
    format!(
        r#"{{"name":"{name}","vers":"{version}","deps":[],"cksum":"{}","features":{{}},"yanked":{yanked}}}"#,
        "0".repeat(64)
    )
}

/// A crate's index file listing `versions`.
fn index_file(name: &str, versions: &[(&str, bool)]) -> (String, u16, String) {
    let body: String = versions
        .iter()
        .map(|(version, yanked)| index_line(name, version, *yanked) + "\n")
        .collect();
    (index_path(name), 200, body)
}

struct Run {
    success: bool,
    code: Option<i32>,
    stdout: String,
    stderr: String,
    /// The fake cargo's runs, one line of arguments each.
    log: Vec<String>,
}

impl Run {
    fn output(&self) -> String {
        format!("--- stdout\n{}--- stderr\n{}", self.stdout, self.stderr)
    }
}

/// `bash publish-crates.sh ARGS` in a fresh directory called `name`, with
/// the fake cargo answering as `answers` says, `index_url` as the sparse
/// index, no wait after a rate limit, and the token set. Whatever happens,
/// the token's value is printed nowhere.
fn publish(name: &str, args: &[&str], answers: &str, index_url: &str) -> Run {
    let dir = fresh(name);
    let cargo = dir.join("cargo");
    std::fs::write(&cargo, FAKE_CARGO).expect("write the fake cargo");
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755))
        .expect("make the fake cargo executable");
    std::fs::write(dir.join("answers"), answers).expect("write the answers");
    // On macOS, the system's bash 3.2, which a Homebrew bash first on PATH
    // would hide: the script must run under both.
    let bash = if cfg!(target_os = "macos") {
        "/bin/bash"
    } else {
        "bash"
    };
    let mut command = Command::new(bash);
    command
        .arg(script())
        .args(args)
        .current_dir(&dir)
        .env("CARGO", &cargo)
        .env("FAKE_CARGO_DIR", &dir)
        .env("NOVA_PUBLISH_INDEX", index_url)
        .env("NOVA_PUBLISH_WAIT", "0")
        .env("CARGO_REGISTRY_TOKEN", TOKEN);
    // Loopback only: a proxy in the environment must not see the requests.
    for proxy in [
        "http_proxy",
        "HTTP_PROXY",
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
    ] {
        command.env_remove(proxy);
    }
    let output = command.output().expect("run bash publish-crates.sh");
    let run = Run {
        success: output.status.success(),
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        log: read_log(&dir),
    };
    assert!(
        !run.stdout.contains(TOKEN) && !run.stderr.contains(TOKEN),
        "the token was printed:\n{}",
        run.output()
    );
    run
}

fn read_log(dir: &Path) -> Vec<String> {
    match std::fs::read_to_string(dir.join("cargo.log")) {
        Ok(text) => text.lines().map(str::to_string).collect(),
        Err(_) => Vec::new(),
    }
}

fn publish_args(crates: &[String]) -> Vec<String> {
    crates
        .iter()
        .map(|crate_name| format!("publish --locked -p {crate_name}"))
        .collect()
}

/// The summary's lines, after its heading.
fn summary(run: &Run) -> Vec<String> {
    let mut lines = run.stdout.lines();
    assert!(
        lines.any(|line| line == format!("summary for {VERSION}:")),
        "no summary:\n{}",
        run.output()
    );
    lines.map(str::to_string).collect()
}

fn summary_line(crate_name: &str, what: &str) -> String {
    format!("  {crate_name:<24} {what}")
}

#[test]
fn publishes_every_crate_in_the_lists_order() {
    let list = publish_list();
    let server = index(&[]);
    let run = publish("in-order", &[VERSION], "", &server.url);
    assert!(run.success, "{}", run.output());
    assert_eq!(run.log, publish_args(&list), "{}", run.output());
    let published: Vec<String> = run
        .stdout
        .lines()
        .filter(|line| line.starts_with("published "))
        .map(str::to_string)
        .collect();
    let expected: Vec<String> = list
        .iter()
        .map(|crate_name| format!("published {crate_name} {VERSION}"))
        .collect();
    assert_eq!(published, expected, "{}", run.output());
    let expected: Vec<String> = list
        .iter()
        .map(|crate_name| summary_line(crate_name, "published"))
        .collect();
    assert_eq!(summary(&run), expected, "{}", run.output());
    // Each crate's index file was asked for once, before its publish.
    let asked: Vec<String> = server
        .requests()
        .iter()
        .map(|request| format!("{} {}", request.method, request.path))
        .collect();
    let expected: Vec<String> = list
        .iter()
        .map(|crate_name| format!("GET {}", index_path(crate_name)))
        .collect();
    assert_eq!(asked, expected);
}

#[test]
fn skips_a_crate_the_index_lists_at_the_version_and_publishes_one_listed_at_others() {
    let list = publish_list();
    let (at_version, yanked_at_version, at_others) = (&list[0], &list[1], &list[2]);
    let server = index(&[
        index_file(at_version, &[("0.2.0", false), (VERSION, false)]),
        // A yanked version is still taken.
        index_file(yanked_at_version, &[(VERSION, true)]),
        // A prerelease of the same numbers, and longer versions that start
        // with them, are other versions.
        index_file(
            at_others,
            &[
                ("0.2.0", false),
                ("0.3.0-alpha.1", false),
                ("0.3.0-rc.1", false),
                ("0.3.01", false),
                ("10.3.0", false),
            ],
        ),
    ]);
    let run = publish("skips", &[VERSION], "", &server.url);
    assert!(run.success, "{}", run.output());
    assert_eq!(run.log, publish_args(&list[2..]), "{}", run.output());
    for skipped in [at_version, yanked_at_version] {
        assert!(
            run.stdout
                .lines()
                .any(|line| line == format!("skip {skipped} {VERSION} (already on crates.io)")),
            "{}",
            run.output()
        );
    }
    assert!(
        run.stdout
            .lines()
            .any(|line| line == format!("published {at_others} {VERSION}")),
        "{}",
        run.output()
    );
    let summary = summary(&run);
    assert_eq!(summary[0], summary_line(at_version, "skipped"));
    assert_eq!(summary[1], summary_line(yanked_at_version, "skipped"));
    assert_eq!(summary[2], summary_line(at_others, "published"));
}

#[test]
fn waits_out_the_rate_limit_retries_the_same_crate_then_goes_on() {
    let list = publish_list();
    let limited = &list[3];
    let server = index(&[]);
    let run = publish(
        "rate-limit",
        &[VERSION],
        &format!("{limited} 429 2\n"),
        &server.url,
    );
    assert!(run.success, "{}", run.output());
    let mut expected = publish_args(&list[..4]);
    expected.insert(3, expected[3].clone());
    expected.insert(3, expected[3].clone());
    expected.extend(publish_args(&list[4..]));
    assert_eq!(run.log, expected, "{}", run.output());
    let waits = run
        .stdout
        .lines()
        .filter(|line| *line == "rate limited: waiting 0 s")
        .count();
    assert_eq!(waits, 2, "{}", run.output());
    assert_eq!(summary(&run)[3], summary_line(limited, "published"));
}

#[test]
fn gives_up_after_thirty_rate_limited_attempts() {
    let list = publish_list();
    let limited = &list[2];
    let server = index(&[]);
    let run = publish(
        "rate-limit-30",
        &[VERSION],
        &format!("{limited} 429 30\n"),
        &server.url,
    );
    assert!(!run.success, "{}", run.output());
    let mut expected = publish_args(&list[..2]);
    for _ in 0..30 {
        expected.push(format!("publish --locked -p {limited}"));
    }
    assert_eq!(run.log, expected, "{}", run.output());
    assert!(run.stderr.contains("30 attempts"), "{}", run.output());
}

#[test]
fn stops_at_any_other_failure_with_cargos_output_and_publishes_nothing_after_it() {
    let list = publish_list();
    let failing = &list[5];
    let server = index(&[]);
    let run = publish(
        "failure",
        &[VERSION],
        &format!("{failing} fail\n"),
        &server.url,
    );
    assert!(!run.success, "{}", run.output());
    // Once: no retry, and no crate after it.
    assert_eq!(run.log, publish_args(&list[..6]), "{}", run.output());
    assert!(
        run.output().contains(&format!(
            "(status 400 Bad Request): fake cargo refused {failing}"
        )),
        "{}",
        run.output()
    );
    assert!(!run.stdout.contains("rate limited"), "{}", run.output());
    let summary = summary(&run);
    assert_eq!(summary[4], summary_line(&list[4], "published"));
    assert_eq!(summary[5], summary_line(failing, "failed"));
    assert_eq!(summary[6], summary_line(&list[6], "not tried"));
    // The index was asked for it and the five before it, and no other.
    assert_eq!(server.requests().len(), 6);
}

#[test]
fn stops_when_the_index_answers_neither_200_nor_404() {
    let list = publish_list();
    let broken = &list[4];
    let server = index(&[(index_path(broken), 500, "oops".to_string())]);
    let run = publish("index-500", &[VERSION], "", &server.url);
    assert!(!run.success, "{}", run.output());
    assert_eq!(run.log, publish_args(&list[..4]), "{}", run.output());
    let url = format!("{}{}", server.url, index_path(broken));
    assert!(run.stderr.contains(&url), "{}", run.output());
    assert!(run.stderr.contains("500"), "{}", run.output());
}

#[test]
fn stops_when_the_index_cannot_be_reached() {
    let list = publish_list();
    // Nothing listens on loopback port 1.
    let run = publish("index-down", &[VERSION], "", "http://127.0.0.1:1");
    assert!(!run.success, "{}", run.output());
    assert!(run.log.is_empty(), "{}", run.output());
    assert!(
        run.stderr
            .contains(&format!("http://127.0.0.1:1{}", index_path(&list[0]))),
        "{}",
        run.output()
    );
}

#[test]
fn never_prints_the_token() {
    // `publish` checks every run; this one goes through a rate limit and a
    // failure, the paths that print cargo's output.
    let list = publish_list();
    let server = index(&[]);
    let answers = format!("{} 429 1\n{} fail\n", list[0], list[1]);
    let run = publish("token", &[VERSION], &answers, &server.url);
    assert!(!run.success, "{}", run.output());
    assert_eq!(run.log.len(), 3, "{}", run.output());
}

#[test]
fn refuses_a_missing_or_malformed_version_before_anything_runs() {
    let server = index(&[]);
    let run = publish("no-version", &[], "", &server.url);
    assert_eq!(run.code, Some(2), "{}", run.output());
    assert!(
        run.stderr.contains("usage: publish-crates.sh VERSION"),
        "{}",
        run.output()
    );
    for bad in ["v0.3.0", "0.3", "0.3.0; true", ""] {
        let run = publish("bad-version", &[bad], "", &server.url);
        assert_eq!(run.code, Some(2), "{bad}: {}", run.output());
        assert!(run.log.is_empty(), "{bad}: {}", run.output());
    }
    assert!(server.requests().is_empty());
}
````

- [ ] **Step 2: Run it to see it fail**

The list-only script prints nothing and exits 0.

Run: `cd /home/user/nova && RUST_BACKTRACE=0 cargo test --locked -p nova-cli --test publish_script 2>&1 | grep -E "^test |left:|^test result"`
Expected: FAIL, `test result: FAILED. 0 passed; 9 failed`, each at its
first assertion: on the log in the tests that expect a publish (`left:
[]`, against the `publish --locked -p ...` lines), on `left: Some(0)`
against `right: Some(2)` in
`refuses_a_missing_or_malformed_version_before_anything_runs`, and on
`!run.success` in the others.

- [ ] **Step 3: The script**

Replace `.github/scripts/publish-crates.sh` with lines 2649-2815 of this plan:

Run: `cd /home/user/nova && sed -n '2649,2815p' "$PLAN" > .github/scripts/publish-crates.sh`

````bash
#!/usr/bin/env bash
# Publishes the 19 crates to crates.io, in the order below (spec
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# §5.2). A crate whose sparse index file already lists VERSION is skipped,
# so a run that stopped can be started again from the same tag. When
# crates.io refuses a crate for its rate limit on new crates, the script
# waits and tries that crate again, up to 30 times; any other failure stops
# the run before the next crate. It ends with one line per crate.
# publish-crates.yml runs it, on the tag, after package-gate.sh.
#
# Usage: publish-crates.sh VERSION
#   VERSION  the version to publish, without the v (the workspace's)
#
# The environment may set:
#   CARGO               the cargo to run (default: cargo)
#   NOVA_PUBLISH_INDEX  the sparse index's base URL
#                       (default: https://index.crates.io)
#   NOVA_PUBLISH_WAIT   seconds to wait after a rate-limit refusal
#                       (default: 660)
# cargo reads CARGO_REGISTRY_TOKEN from the environment. Nothing here prints
# it: no `set -x`, and no line names it but this one.
#
# release_manifests.rs and package-gate.sh read the list: keep `crates=(`,
# one crate per line, and `)` on its own line.
set -euo pipefail

crates=(
  nova-diagnostics
  nova-std
  nova-ast
  nova-lexer
  nova-runtime
  nova-parser
  nova-pm
  nova-fmt
  nova-index
  nova-resolver
  nova-hir
  nova-mir
  nova-typeck
  nova-codegen-cranelift
  nova-codegen-llvm
  nova-driver
  nova-doc
  nova-lsp
  nova-cli
)

# The attempts one crate gets while crates.io answers with its rate limit.
max_attempts=30

if [ $# -ne 1 ]; then
  echo "usage: publish-crates.sh VERSION" >&2
  exit 2
fi
version=$1
# A regex in a variable: bash 3.2 (macOS) and later read it the same way.
version_pattern='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'
if ! [[ $version =~ $version_pattern ]]; then
  echo "publish-crates: '$version' is not a version such as 0.3.0" >&2
  exit 2
fi
cargo=${CARGO:-cargo}
index=${NOVA_PUBLISH_INDEX:-https://index.crates.io}
index=${index%/}
wait=${NOVA_PUBLISH_WAIT:-660}
case $wait in
  '' | *[!0-9]*)
    echo "publish-crates: NOVA_PUBLISH_WAIT must be a number of seconds, not '$wait'" >&2
    exit 2
    ;;
esac

scratch=$(mktemp -d "${TMPDIR:-/tmp}/publish-crates.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

# What happened to each crate, by its place in the list.
results=()

summary() {
  echo "summary for $version:"
  local i=0 crate
  for crate in "${crates[@]}"; do
    printf '  %-24s %s\n' "$crate" "${results[$i]:-not tried}"
    i=$((i + 1))
  done
}

# Stop the run: the reason on stderr, then the summary.
stop() {
  echo "publish-crates: $1" >&2
  summary
  exit 1
}

# A crate's file in the sparse index: its name in lower case, under 1/, 2/,
# 3/<first character>/ or <characters 1-2>/<characters 3-4>/.
index_path() {
  local name
  name=$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')
  case ${#name} in
    1) printf '1/%s' "$name" ;;
    2) printf '2/%s' "$name" ;;
    3) printf '3/%s/%s' "${name:0:1}" "$name" ;;
    *) printf '%s/%s/%s' "${name:0:2}" "${name:2:2}" "$name" ;;
  esac
}

# Sets `listed` to yes when crates.io has the crate at $version: its index
# file has a line with "vers":"<version>", the closing quote included so
# that 0.3.0 never matches 0.3.0-alpha.1. A yanked version is listed too,
# and cannot be published again. 404 means the crate has no version yet;
# any other answer stops the run.
check_index() {
  local url status
  url="$index/$(index_path "$1")"
  if ! status=$(curl -sS --connect-timeout 30 --max-time 120 \
    -o "$scratch/index" -w '%{http_code}' "$url"); then
    stop "could not read the index at $url"
  fi
  case $status in
    200)
      if grep -qF "\"vers\":\"$version\"" "$scratch/index"; then
        listed=yes
      else
        listed=no
      fi
      ;;
    404) listed=no ;;
    *) stop "the index answered $status for $url" ;;
  esac
}

echo "publish-crates: ${#crates[@]} crates at $version, index $index"
i=0
for crate in "${crates[@]}"; do
  check_index "$crate"
  if [ "$listed" = yes ]; then
    echo "skip $crate $version (already on crates.io)"
    results[i]=skipped
    i=$((i + 1))
    continue
  fi
  attempt=1
  while :; do
    echo "publish-crates: cargo publish --locked -p $crate (attempt $attempt)"
    # cargo's output is shown as it comes, and kept to be read below.
    if "$cargo" publish --locked -p "$crate" 2>&1 | tee "$scratch/cargo.log"; then
      echo "published $crate $version"
      results[i]=published
      break
    fi
    if ! grep -qiE 'status 429|too many requests|try again after' "$scratch/cargo.log"; then
      results[i]=failed
      stop "cargo publish failed for $crate (its output is above); nothing after it was published"
    fi
    if [ "$attempt" -ge "$max_attempts" ]; then
      results[i]=failed
      stop "crates.io still refused $crate for its rate limit after $max_attempts attempts"
    fi
    echo "rate limited: waiting $wait s"
    sleep "$wait"
    attempt=$((attempt + 1))
  done
  i=$((i + 1))
done
summary
````

The `crates=(` block is Task 1's, byte for byte:
`diff <(git show HEAD:.github/scripts/publish-crates.sh | sed -n '/^crates=(/,/^)/p') <(sed -n '/^crates=(/,/^)/p' .github/scripts/publish-crates.sh)`
prints nothing.

- [ ] **Step 4: Run the tests**

Run: `cd /home/user/nova && cargo test --locked -p nova-cli --test publish_script --test release_manifests 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 9 passed` (in about 0.9 s) and `test result:
ok. 7 passed`.

- [ ] **Step 5: Checks and commit**

The checks, with `msrv_test crates/nova-cli/tests/publish_script.rs base64 serde_json`.
`$SC .github/scripts/publish-crates.sh`: no output.
Stage `.github/scripts/publish-crates.sh` and
`crates/nova-cli/tests/publish_script.rs`.
Message: `ci: publish-crates.sh, the 19 crates to crates.io in order`,
body: each crate the sparse index already lists at the version is skipped,
so a stopped run resumes from the same tag; a rate-limit refusal waits and
retries the same crate, up to 30 attempts; any other failure stops the run
with a summary of what is up. publish_script.rs runs it against a fake
cargo and a loopback index, and checks the token is never printed (spec
3.6a §5.2, §11.2).

---

### Task 4: The publish workflow (spec §5.1)

**Files:**
- Create: `.github/workflows/publish-crates.yml`

**Interfaces:** run only by hand, on the tag `v<version>`, with the input
`version`; never in 3.6a.

- [ ] **Step 1: The workflow**

`.github/workflows/publish-crates.yml`, lines 2859-2946 of this plan:

Run: `cd /home/user/nova && sed -n '2859,2946p' "$PLAN" > .github/workflows/publish-crates.yml`

````yaml
# Publishes the 19 crates to crates.io (spec
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# §5.1). It runs only when started by hand, from the Actions tab ("Run
# workflow", with the tag v<version> chosen under "Use workflow from") or
# with `gh workflow run publish-crates.yml --ref v<version> -f
# version=<version>`; never on a push, a pull request or a tag. It refuses
# unless it runs on the tag v<version> and the workspace's version is
# <version>, so crates.io receives exactly the tagged tree the GitHub
# release was built from. It repeats CI's package gate, then
# publish-crates.sh uploads the crates in order, skipping any crates.io
# already has, so a run that stopped can be started again on the same tag.
#
# The token is the `crates-io` environment's secret CARGO_REGISTRY_TOKEN,
# which the maintainer adds before a run and deletes after it
# (docs/releasing.md); only the publish step's environment holds it.
name: Publish to crates.io

on:
  workflow_dispatch:
    inputs:
      version:
        description: The version to publish, without the v; the run must be on the tag v<version>
        required: true
        type: string

permissions:
  contents: read

# Two runs never overlap, and a second one waits rather than cancelling
# the first mid-publish.
concurrency:
  group: publish-crates
  cancel-in-progress: false

env:
  CARGO_TERM_COLOR: always

jobs:
  publish:
    name: Publish
    runs-on: ubuntu-latest
    # The maintainer can require their own approval in this environment.
    environment: crates-io
    # 19 new crates at one every 15 minutes, after the package gate; a
    # hosted runner's job may last 360.
    timeout-minutes: 345
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
      # The input reaches the shell through `env:` only, never pasted into
      # the script, so no version string can inject a command.
      - name: Refuse unless this is the tag v<version>, at that version, with a token
        shell: bash
        env:
          VERSION: ${{ inputs.version }}
          # Whether the secret is set, not the secret itself.
          HAS_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN != '' }}
        run: |
          set -euo pipefail
          if [ "$GITHUB_REF" != "refs/tags/v$VERSION" ]; then
            echo "::error::this run is on $GITHUB_REF; start it on the tag v$VERSION"
            exit 1
          fi
          # [workspace.package]'s, the only line of the root Cargo.toml that
          # starts with `version = `.
          workspace=$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -n 1)
          if [ "$workspace" != "$VERSION" ]; then
            echo "::error::the workspace's version is '$workspace', not '$VERSION'"
            exit 1
          fi
          if [ "$HAS_TOKEN" != true ]; then
            echo "::error::the crates-io environment has no secret CARGO_REGISTRY_TOKEN"
            exit 1
          fi
          echo "Publishing $VERSION from $GITHUB_REF ($GITHUB_SHA)."
      - uses: dtolnay/rust-toolchain@stable
      # No Swatinem/rust-cache: this job holds the token, so it restores
      # nothing another workflow saved.
      - name: Package the 19 crates, install nova from them, run the gate
        shell: bash
        run: bash .github/scripts/package-gate.sh "$RUNNER_TEMP/package-gate"
      - name: Publish the 19 crates, skipping any crates.io already has
        shell: bash
        env:
          VERSION: ${{ inputs.version }}
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: bash .github/scripts/publish-crates.sh "$VERSION"
````

`${{ inputs.version }}` reaches the shell only through `env:`; the token
is in the publish step's `env:` alone (decisions 17, 18). The workflow
must be on the default branch for "Run workflow" to appear; a run then uses
the copy at the chosen ref, the tag. If the `crates-io` environment does
not exist when it first runs, GitHub creates it with no protection, which
is why the runbook creates it first.

- [ ] **Step 2: Lint it, and run its refusal step by hand**

Run: `cd /home/user/nova && $AL .github/workflows/publish-crates.yml; echo "exit $?"`
Expected: `exit 0`.

The refusal step, extracted and run with four cases (a tag at the
workspace's version with a token; a branch; a tag at another version; no
token):

Run: `cd /home/user/nova && python3 -I -c 'import sys, yaml; w = yaml.safe_load(open(sys.argv[1])); print(w["jobs"]["publish"]["steps"][1]["run"], end="")' .github/workflows/publish-crates.yml > "$P/t4-refusal.sh" && for c in "refs/tags/v0.2.0 0.2.0 true" "refs/heads/main 0.2.0 true" "refs/tags/v0.3.0 0.3.0 true" "refs/tags/v0.2.0 0.2.0 false"; do set -- $c; GITHUB_REF=$1 VERSION=$2 HAS_TOKEN=$3 GITHUB_SHA=0123abc bash "$P/t4-refusal.sh"; echo "exit $?"; done`
Expected:

````text
Publishing 0.2.0 from refs/tags/v0.2.0 (0123abc).
exit 0
::error::this run is on refs/heads/main; start it on the tag v0.2.0
exit 1
::error::the workspace's version is '0.2.0', not '0.3.0'
exit 1
::error::the crates-io environment has no secret CARGO_REGISTRY_TOKEN
exit 1
````

- [ ] **Step 3: Commit**

Stage `.github/workflows/publish-crates.yml`.
Message: `ci: "Publish to crates.io", run by hand on the release's tag`,
body: workflow_dispatch only, in the crates-io environment, refusing
unless it runs on the tag v<version> at that workspace version with the
secret set; it repeats the package gate, then publish-crates.sh uploads
the crates with the token in that step's environment alone (spec 3.6a
§5.1). Nothing runs it in 3.6a.

---

### Task 5: The templates (spec §7.1)

**Files:**
- Modify: `crates/nova-cli/src/template.rs` (`MAIN`, `LIB`, the library's
  `tests/` file, the module comment)
- Modify: `crates/nova-cli/tests/project.rs` (a new test; the constants
  that pin the templates)
- Modify: `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
  (a dated note in §6.3)

**Interfaces:** none change. `nova init` uses the same `template::files`.

- [ ] **Step 1: Write the failing test**

At the end of `crates/nova-cli/tests/project.rs`, lines 3010-3062 of this plan:

Run: `cd /home/user/nova && sed -n '3010,3062p' "$PLAN" > "$P/36a-t5-test.diff" && git apply "$P/36a-t5-test.diff"`

````diff
diff --git a/crates/nova-cli/tests/project.rs b/crates/nova-cli/tests/project.rs
index 51bf68f..693f486 100644
--- a/crates/nova-cli/tests/project.rs
+++ b/crates/nova-cli/tests/project.rs
@@ -650,3 +650,48 @@ fn init_keeps_its_plain_refusal_unless_a_library_is_missing() {
         .failure();
     assert_eq!(stderr(&out), plain);
 }
+
+// === Phase 3.6a: the templates are in `nova fmt`'s layout (spec
+// docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
+// §8: the release gate runs `nova fmt --check` on the project `nova new`
+// made) ===
+
+#[test]
+fn every_template_passes_fmt_check_as_written() {
+    let dir = fresh_dir("templates-formatted");
+    let home = dir.join("home");
+    std::fs::create_dir_all(&home).unwrap();
+    let nova_in = |cwd: &Path| {
+        let mut command = nova();
+        command.current_dir(cwd).env("NOVA_HOME", &home);
+        command
+    };
+    nova_in(&dir).args(["new", "program"]).assert().success();
+    nova_in(&dir)
+        .args(["new", "--lib", "library"])
+        .assert()
+        .success();
+    for (name, args) in [
+        ("init-program", &["init"][..]),
+        ("init-library", &["init", "--lib"]),
+    ] {
+        std::fs::create_dir_all(dir.join(name)).unwrap();
+        nova_in(&dir.join(name)).args(args).assert().success();
+    }
+    let mut unformatted = String::new();
+    for project in ["program", "library", "init-program", "init-library"] {
+        let checked = nova_in(&dir.join(project))
+            .args(["fmt", "--check"])
+            .assert();
+        let output = checked.get_output();
+        if !output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty() {
+            unformatted.push_str(&format!(
+                "{project}: nova fmt --check exited {:?}\n{}{}",
+                output.status.code(),
+                String::from_utf8_lossy(&output.stdout),
+                String::from_utf8_lossy(&output.stderr)
+            ));
+        }
+    }
+    assert!(unformatted.is_empty(), "{unformatted}");
+}
````

- [ ] **Step 2: Run it to see it fail**

Run: `cd /home/user/nova && RUST_BACKTRACE=0 cargo test --locked -p nova-cli --test project every_template 2>&1 | grep -E "^test |exited|would reformat|^test result"`
Expected: FAIL; every project, and both files of each library:

````text
test every_template_passes_fmt_check_as_written ... FAILED
program: nova fmt --check exited Some(1)
would reformat: /tmp/nova-project-templates-formatted/program/src/main.nova
library: nova fmt --check exited Some(1)
would reformat: /tmp/nova-project-templates-formatted/library/src/lib.nova
would reformat: /tmp/nova-project-templates-formatted/library/tests/library_test.nova
init-program: nova fmt --check exited Some(1)
would reformat: /tmp/nova-project-templates-formatted/init-program/src/main.nova
init-library: nova fmt --check exited Some(1)
would reformat: /tmp/nova-project-templates-formatted/init-library/src/lib.nova
would reformat: /tmp/nova-project-templates-formatted/init-library/tests/init_library_test.nova
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 24 filtered out; finished in …
````

- [ ] **Step 3: The templates, the constants and the 3.0 spec's note**

`crates/nova-cli/src/template.rs`, lines 3092-3149 of this plan:

Run: `cd /home/user/nova && sed -n '3092,3149p' "$PLAN" > "$P/36a-t5-template.diff" && git apply "$P/36a-t5-template.diff"`

````diff
diff --git a/crates/nova-cli/src/template.rs b/crates/nova-cli/src/template.rs
index 6b9ccb3..ced1799 100644
--- a/crates/nova-cli/src/template.rs
+++ b/crates/nova-cli/src/template.rs
@@ -2,32 +2,26 @@
 //! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
 //! §6.3, and for a library
 //! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
-//! §5.5), with `\n` line endings on every system.
+//! §5.5), with `\n` line endings on every system. Each `.nova` file is
+//! already in `nova fmt`'s layout, so `nova fmt --check` passes on a new
+//! project (spec
+//! `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
+//! §8): a body holding one expression stays on one line.
 
 const MAIN: &str = concat!(
-    "fn greeting() -> String {\n",
-    "    \"Hello, Nova!\"\n",
-    "}\n",
+    "fn greeting() -> String { \"Hello, Nova!\" }\n",
     "\n",
-    "fn main() {\n",
-    "    println(greeting())\n",
-    "}\n",
+    "fn main() { println(greeting()) }\n",
     "\n",
     "@test\n",
-    "fn greeting_says_hello() {\n",
-    "    assert_eq(greeting(), \"Hello, Nova!\")\n",
-    "}\n",
+    "fn greeting_says_hello() { assert_eq(greeting(), \"Hello, Nova!\") }\n",
 );
 
 const LIB: &str = concat!(
-    "pub fn greeting() -> String {\n",
-    "    \"Hello, Nova!\"\n",
-    "}\n",
+    "pub fn greeting() -> String { \"Hello, Nova!\" }\n",
     "\n",
     "@test\n",
-    "fn greeting_says_hello() {\n",
-    "    assert_eq(greeting(), \"Hello, Nova!\")\n",
-    "}\n",
+    "fn greeting_says_hello() { assert_eq(greeting(), \"Hello, Nova!\") }\n",
 );
 
 /// What a template makes.
@@ -79,8 +73,8 @@ pub fn files(name: &str, kind: Kind) -> Vec<(String, String)> {
             files.push((
                 format!("tests/{import}_test.nova"),
                 format!(
-                    "import {import}\n\n@test\nfn greeting_is_public() {{\n    \
-                     assert_eq(greeting(), \"Hello, Nova!\")\n}}\n"
+                    "import {import}\n\n@test\n\
+                     fn greeting_is_public() {{ assert_eq(greeting(), \"Hello, Nova!\") }}\n"
                 ),
             ));
         }
````

The constants in `crates/nova-cli/tests/project.rs` that pin the
templates, lines 3158-3188 of this plan:

Run: `cd /home/user/nova && sed -n '3158,3188p' "$PLAN" > "$P/36a-t5-constants.diff" && git apply "$P/36a-t5-constants.diff"`

````diff
diff --git a/crates/nova-cli/tests/project.rs b/crates/nova-cli/tests/project.rs
index 693f486..9d6ee68 100644
--- a/crates/nova-cli/tests/project.rs
+++ b/crates/nova-cli/tests/project.rs
@@ -349,7 +349,7 @@ fn an_import_resolves_when_run_from_a_subdirectory() {
         .stdout("from util\n");
 }
 
-const TEMPLATE_MAIN: &str = "fn greeting() -> String {\n    \"Hello, Nova!\"\n}\n\nfn main() {\n    println(greeting())\n}\n\n@test\nfn greeting_says_hello() {\n    assert_eq(greeting(), \"Hello, Nova!\")\n}\n";
+const TEMPLATE_MAIN: &str = "fn greeting() -> String { \"Hello, Nova!\" }\n\nfn main() { println(greeting()) }\n\n@test\nfn greeting_says_hello() { assert_eq(greeting(), \"Hello, Nova!\") }\n";
 
 fn read(path: impl AsRef<Path>) -> String {
     let path = path.as_ref();
@@ -518,7 +518,7 @@ fn init_suggests_the_name_flag_for_a_bad_directory_name() {
 // === Phase 3.3a: library templates (spec
 // docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §5.5) ===
 
-const TEMPLATE_LIB: &str = "pub fn greeting() -> String {\n    \"Hello, Nova!\"\n}\n\n@test\nfn greeting_says_hello() {\n    assert_eq(greeting(), \"Hello, Nova!\")\n}\n";
+const TEMPLATE_LIB: &str = "pub fn greeting() -> String { \"Hello, Nova!\" }\n\n@test\nfn greeting_says_hello() { assert_eq(greeting(), \"Hello, Nova!\") }\n";
 
 #[test]
 fn new_lib_writes_a_library_and_its_tests_pass() {
@@ -537,7 +537,7 @@ fn new_lib_writes_a_library_and_its_tests_pass() {
     assert_eq!(read(project.join("src").join("lib.nova")), TEMPLATE_LIB);
     assert_eq!(
         read(project.join("tests").join("json_api_test.nova")),
-        "import json_api\n\n@test\nfn greeting_is_public() {\n    assert_eq(greeting(), \"Hello, Nova!\")\n}\n"
+        "import json_api\n\n@test\nfn greeting_is_public() { assert_eq(greeting(), \"Hello, Nova!\") }\n"
     );
     assert!(read(project.join("README.md")).contains("`nova test` runs the tests in"));
     assert!(!project.join("src").join("main.nova").exists());
````

The 3.0 spec's dated note, after the `src/main.nova` block it quotes
(decision 35), lines 3197-3224 of this plan:

Run: `cd /home/user/nova && sed -n '3197,3224p' "$PLAN" > "$P/36a-t5-spec30.diff" && git apply "$P/36a-t5-spec30.diff"`

````diff
diff --git a/docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md b/docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md
index e826254..5267333 100644
--- a/docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md
+++ b/docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md
@@ -429,6 +429,23 @@ writes is unchanged.
   }
   ```
 
+  **Amended 2026-10-12 (Phase 3.6a):** `src/main.nova` is written in
+  `nova fmt`'s layout, which keeps a body holding one expression on one
+  line (3.1 spec, "Blocks"), so `nova fmt --check` passes on a new
+  project
+  (`docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
+  §8). The library template and its `tests/` file (3.3a spec §5.5) follow
+  the same layout. What the program prints is unchanged:
+
+  ```nova
+  fn greeting() -> String { "Hello, Nova!" }
+
+  fn main() { println(greeting()) }
+
+  @test
+  fn greeting_says_hello() { assert_eq(greeting(), "Hello, Nova!") }
+  ```
+
 - Every file is written with `\n` line endings, on every system.
 - **How it differs from `40-TOOLING.md` §1.2's template, on purpose:**
   - it has no `authors` placeholder, which would be published as written
````

- [ ] **Step 4: The citations, and the note's heading (decision 41)**

The three texts above cite the 3.6a spec's §8, where the release gate's
check is; the amended spec's rule for the templates is §7.1. The note's
heading names the branch, as the file's earlier note does. Lines 3236-3285 of this plan:

Run: `cd /home/user/nova && sed -n '3236,3285p' "$PLAN" > "$P/36a-t5-citations.diff" && git apply "$P/36a-t5-citations.diff"`

````diff
diff --git a/crates/nova-cli/src/template.rs b/crates/nova-cli/src/template.rs
index ced1799..34845a8 100644
--- a/crates/nova-cli/src/template.rs
+++ b/crates/nova-cli/src/template.rs
@@ -6,7 +6,7 @@
 //! already in `nova fmt`'s layout, so `nova fmt --check` passes on a new
 //! project (spec
 //! `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
-//! §8): a body holding one expression stays on one line.
+//! §7.1): a body holding one expression stays on one line.
 
 const MAIN: &str = concat!(
     "fn greeting() -> String { \"Hello, Nova!\" }\n",
diff --git a/crates/nova-cli/tests/project.rs b/crates/nova-cli/tests/project.rs
index 9d6ee68..ab530b0 100644
--- a/crates/nova-cli/tests/project.rs
+++ b/crates/nova-cli/tests/project.rs
@@ -653,8 +653,8 @@ fn init_keeps_its_plain_refusal_unless_a_library_is_missing() {
 
 // === Phase 3.6a: the templates are in `nova fmt`'s layout (spec
 // docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
-// §8: the release gate runs `nova fmt --check` on the project `nova new`
-// made) ===
+// §7.1: every project `nova new` and `nova init` make passes `nova fmt
+// --check`, which the release gate, §8, runs) ===
 
 #[test]
 fn every_template_passes_fmt_check_as_written() {
diff --git a/docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md b/docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md
index 5267333..8f6a004 100644
--- a/docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md
+++ b/docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md
@@ -429,12 +429,12 @@ writes is unchanged.
   }
   ```
 
-  **Amended 2026-10-12 (Phase 3.6a):** `src/main.nova` is written in
-  `nova fmt`'s layout, which keeps a body holding one expression on one
-  line (3.1 spec, "Blocks"), so `nova fmt --check` passes on a new
-  project
+  **Amended 2026-10-12 (branch `claude/loving-archimedes-frtf5q`):**
+  `src/main.nova` is written in `nova fmt`'s layout, which keeps a body
+  holding one expression on one line (3.1 spec, "Blocks"), so `nova fmt
+  --check` passes on a new project
   (`docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
-  §8). The library template and its `tests/` file (3.3a spec §5.5) follow
+  §7.1). The library template and its `tests/` file (3.3a spec §5.5) follow
   the same layout. What the program prints is unchanged:
 
   ```nova
````

- [ ] **Step 5: Run the tests**

Run: `cd /home/user/nova && cargo test --locked -p nova-cli --test fmt --test project 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 16 passed` and `test result: ok. 25 passed`.

Run: `cd /home/user/nova && git grep -n -E 'greeting_says_hello|greeting_is_public|fn greeting\(\)' -- crates docs README.md examples tools .github std nova-spec | grep -v -E '^docs/superpowers/plans/'`
Expected: only `template.rs`, `project.rs`, the 3.0 spec (its quoted 3.0
template and the new note), `crates/nova-driver/tests/packages.rs`, which
writes its own `greeting()`, and `.github/scripts/packages-gate.sh`, which
writes its own `src/lib.nova` and checks only the name of the template's
`greeting_is_public` test. The gate scripts check what the programs print,
which does not change.

- [ ] **Step 6: The gates, with an embedding `nova` (optional, about 2 minutes)**

A plain debug `nova` says `runtime: not embedded`, which `gate.sh`
refuses, so this builds one that embeds into its own target directory
(about 1 minute, 684 MB; `NOVA_EMBED_RUNTIME` is read at build time only):

Run: `cd /home/user/nova && CARGO_TARGET_DIR="$P/target-embed36a" NOVA_EMBED_RUNTIME=1 cargo build --locked -p nova-cli 2>&1 | tail -n 1 && rm -rf "$P/gates36a" "$P/home-gates36a" && mkdir -p "$P/gates36a" "$P/home-gates36a" && for g in "gate.sh gate --test" "packages-gate.sh packages" "registry-gate.sh registry"; do set -- $g; env HOME="$P/home-gates36a" CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}" bash ".github/scripts/$1" "$P/target-embed36a/debug/nova" "$P/gates36a/$2" ${3:-} 2>&1 | tail -n 1; done && (cd "$P/gates36a/gate/demo" && "$P/target-embed36a/debug/nova" fmt --check; echo "fmt --check exit $?")`
Expected: `Finished ...`, `gate: passed`, `packages-gate: passed`,
`registry-gate: passed`, `fmt --check exit 0`. Then
`rm -rf "$P/target-embed36a" "$P/gates36a" "$P/home-gates36a"`.

- [ ] **Step 7: Checks and commit**

The checks; `project.rs` is not type-checked with 1.78 (Global
Constraints).
Stage `crates/nova-cli/src/template.rs`, `crates/nova-cli/tests/project.rs`
and `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`.
Message: `nova-cli: the templates nova new writes are in nova fmt's layout`,
body: a body holding one expression stays on one line, as nova fmt writes
it, so a new project passes nova fmt --check; a test runs nova new, new
--lib, init and init --lib and checks each project (spec 3.6a §7.1, an
amendment). What the programs print is unchanged. The 3.0 spec gets a
dated note.

---

### Task 6: The install scripts (spec §6, §11.3)

**Files:**
- Create: `crates/nova-cli/tests/install_scripts.rs`
- Create: `tools/install/install.sh`
- Create: `tools/install/install.ps1`

**Interfaces:** the install scripts (Contracts); `NOVA_INSTALL_NO_PATH=1`
leaves PATH alone. The test has nine cases. On Linux eight run: seven run
`install.sh` with `sh` (dash on Ubuntu), and `the_scripts_are_plain_ascii`
reads both scripts. On Windows seven run: the five archive cases run
`install.ps1` in Windows PowerShell 5.1 with `NOVA_INSTALL_NO_PATH=1`, then
`install_ps1_on_an_unknown_system_names_cargo_install` and the ASCII case.
The Windows cases first run in the pull request's CI ("Test
(windows-latest)").

Both scripts are one task and one commit (decision 39): the test and its
failing run, then `install.sh` and the cases it passes, then `install.ps1`
and every case.

- [ ] **Step 1: Write the failing test**

`crates/nova-cli/tests/install_scripts.rs`, lines 3354-3910 of this plan:

Run: `cd /home/user/nova && sed -n '3354,3910p' "$PLAN" > crates/nova-cli/tests/install_scripts.rs`

````rust
//! The install scripts, `tools/install/install.sh` and `install.ps1` (spec
//! `docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
//! §6, §11.3), against a fake release that the loopback server serves: the
//! host's archive, packed as release.yml packs it around this test's own
//! `nova`, and its `SHA256SUMS`.
//!
//! `install.sh` runs (with `sh`, which is dash on Ubuntu) on Linux and
//! macOS; `install.ps1` runs in Windows PowerShell 5.1 on Windows, with
//! `NOVA_INSTALL_NO_PATH=1` so the user's PATH is never touched. Each run
//! has `NOVA_HOME` (and on Unix `HOME`) in the test's own directory, no
//! proxy, and `NOVA_INSTALL_BASE` at the loopback server, so every request
//! goes there. On a system the scripts have no archive for, only the tests
//! that need none run.

#[path = "../../nova-index/tests/support/mod.rs"]
mod support;

use std::collections::HashMap;
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use support::{Response, Server};

/// The release the `latest` redirect names, and an older one.
const LATEST: &str = "1.2.3";
const OLDER: &str = "1.2.2";

/// The target whose archive the scripts pick on this system (spec §6.1
/// step 1), or `None` where there is none.
fn host_target() -> Option<&'static str> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Some("x86_64-pc-windows-msvc")
    } else {
        None
    }
}

/// The host's target, or `None` after saying the test is skipped.
fn target_or_skip() -> Option<&'static str> {
    let target = host_target();
    if target.is_none() {
        eprintln!("skipped: the install scripts have no archive for this system");
    }
    target
}

/// `nova-<version>-<target>.tar.gz`, or `.zip` on Windows (release.yml).
fn archive_name(version: &str, target: &str) -> String {
    let extension = if cfg!(windows) { "zip" } else { "tar.gz" };
    format!("nova-{version}-{target}.{extension}")
}

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("the repository holds crates/nova-cli")
        .to_path_buf()
}

/// A fresh, empty directory with a fixed name, so each run replaces the last.
fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-install-scripts-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    dir
}

/// This test's own `nova`, which the fake archives carry.
fn nova() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_nova"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Run `command`, which must succeed, and return its stdout.
fn run_ok(command: &mut Command) -> String {
    let output = command
        .output()
        .unwrap_or_else(|e| panic!("running {command:?}: {e}"));
    assert!(
        output.status.success(),
        "{command:?} failed\n{}\n{}",
        stdout(&output),
        stderr(&output)
    );
    stdout(&output)
}

/// The SHA-256 of `path`, in lower-case hex, from the system's own tool.
fn sha256(path: &Path) -> String {
    let printed = if cfg!(windows) {
        run_ok(
            Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg("(Get-FileHash -LiteralPath $env:HASH_FILE -Algorithm SHA256).Hash")
                .env("HASH_FILE", path),
        )
    } else if Command::new("sha256sum").arg("--version").output().is_ok() {
        run_ok(Command::new("sha256sum").arg(path))
    } else {
        run_ok(Command::new("shasum").args(["-a", "256"]).arg(path))
    };
    let hash = printed
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    assert!(
        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
        "not a SHA-256: {printed}"
    );
    hash
}

/// Pack release `version`'s archive for `target` in `dir` as release.yml
/// does: one directory, `nova-<version>-<target>`, holding `nova` (or
/// `nova.exe`), `README.md` and both licences; `tar -czf` on Unix and
/// `Compress-Archive` on Windows. Returns the archive's path. The packed
/// directory is removed, as each copy of a debug `nova` is large.
fn pack(dir: &Path, version: &str, target: &str) -> PathBuf {
    let name = format!("nova-{version}-{target}");
    let stage = dir.join(format!("stage-{version}"));
    let inner = stage.join(&name);
    std::fs::create_dir_all(&inner).expect("create the archive's directory");
    for file in ["README.md", "LICENSE-MIT", "LICENSE-APACHE"] {
        std::fs::copy(repo().join(file), inner.join(file)).expect("copy a release file");
    }
    std::fs::copy(nova(), inner.join(format!("nova{EXE_SUFFIX}"))).expect("copy nova");
    let archive = archive_name(version, target);
    if cfg!(windows) {
        run_ok(
            Command::new("powershell")
                .args(["-NoProfile", "-NonInteractive", "-Command"])
                .arg("Compress-Archive -LiteralPath $env:PACK_SOURCE -DestinationPath $env:PACK_ARCHIVE")
                .env("PACK_SOURCE", &inner)
                .env("PACK_ARCHIVE", stage.join(&archive)),
        );
    } else {
        run_ok(
            Command::new("tar")
                .arg("-czf")
                .arg(&archive)
                .arg(&name)
                .current_dir(&stage),
        );
    }
    std::fs::remove_dir_all(&inner).expect("remove the packed directory");
    stage.join(archive)
}

/// What a fake release's `SHA256SUMS` says of its archive.
enum Sums {
    /// The archive's own SHA-256.
    Right,
    /// Another SHA-256.
    Wrong,
    /// Nothing: the archive has no line.
    Missing,
}

/// Release `version`'s files for `target`, each by its path under
/// `/releases/download/`: the archive, and a `SHA256SUMS` that also lists
/// another asset, as a real release's does.
fn release(dir: &Path, version: &str, target: &str, sums: Sums) -> HashMap<String, Vec<u8>> {
    let archive = pack(dir, version, target);
    let name = archive.file_name().unwrap().to_str().unwrap().to_string();
    let hash = sha256(&archive);
    let other = format!(
        "{}  nova-vscode-{version}.vsix\n",
        "0123456789abcdef".repeat(4)
    );
    let text = match sums {
        Sums::Right => format!("{hash}  {name}\n{other}"),
        Sums::Wrong => {
            let wrong = if hash.starts_with("ab") { "cd" } else { "ab" }.repeat(32);
            format!("{wrong}  {name}\n{other}")
        }
        Sums::Missing => other,
    };
    let bytes = std::fs::read(&archive).expect("read the archive");
    HashMap::from([
        (format!("v{version}/{name}"), bytes),
        (format!("v{version}/SHA256SUMS"), text.into_bytes()),
    ])
}

/// A loopback stand-in for `github.com/Sakeerin/nova/releases`: `latest`
/// redirects to `tag/v<LATEST>`, which answers with a page, and
/// `download/<path>` answers with `files`' entry; anything else is a 404.
fn serve(files: HashMap<String, Vec<u8>>) -> Server {
    let url = Arc::new(Mutex::new(String::new()));
    let base = Arc::clone(&url);
    let server = Server::start(move |request| {
        let path = request.path.as_str();
        if path == "/releases/latest" {
            let base = base.lock().unwrap().clone();
            return Response::redirect(&format!("{base}/releases/tag/v{LATEST}"));
        }
        if path == format!("/releases/tag/v{LATEST}") {
            return Response::ok(format!("<html><title>Nova {LATEST}</title></html>"));
        }
        match path
            .strip_prefix("/releases/download/")
            .and_then(|file| files.get(file))
        {
            Some(bytes) => Response::ok(bytes.clone()),
            None => Response::status(404),
        }
    });
    *url.lock().unwrap() = server.url.clone();
    server
}

/// The paths the server was asked for, in order.
fn paths(server: &Server) -> Vec<String> {
    server.requests().into_iter().map(|r| r.path).collect()
}

/// The host's install script, run in `dir` against `server`, with
/// `NOVA_HOME` in `dir/nova-home`, on Unix `HOME` in `dir/home`, no
/// `NOVA_VERSION` and no proxy. On Windows, `NOVA_INSTALL_NO_PATH=1`.
fn script(dir: &Path, server: &Server) -> Command {
    let mut command = if cfg!(windows) {
        let mut command = Command::new("powershell");
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(repo().join("tools").join("install").join("install.ps1"))
            .env("NOVA_INSTALL_NO_PATH", "1");
        command
    } else {
        let home = dir.join("home");
        std::fs::create_dir_all(&home).expect("create HOME");
        let mut command = Command::new("sh");
        command
            .arg(repo().join("tools").join("install").join("install.sh"))
            .env("HOME", home);
        command
    };
    command
        .current_dir(dir)
        .env("NOVA_INSTALL_BASE", format!("{}/releases", server.url))
        .env("NOVA_HOME", dir.join("nova-home"))
        .env_remove("NOVA_VERSION");
    for proxy in [
        "http_proxy",
        "HTTP_PROXY",
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
    ] {
        command.env_remove(proxy);
    }
    command
}

fn run(command: &mut Command) -> Output {
    command.output().expect("run the install script")
}

/// `dir/nova-home/bin`, where the scripts install.
fn bin(dir: &Path) -> PathBuf {
    dir.join("nova-home").join("bin")
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// `bin` holds this test's `nova` and nothing else.
fn assert_installed(bin: &Path) {
    let exe = format!("nova{EXE_SUFFIX}");
    assert_eq!(entries(bin), [exe.as_str()], "{}", bin.display());
    assert!(
        std::fs::read(bin.join(&exe)).unwrap() == std::fs::read(nova()).unwrap(),
        "{} is not the release's nova",
        bin.join(&exe).display()
    );
}

/// `out` holds every line this test's `nova version` prints, in order.
fn assert_prints_version(out: &str) {
    let ours = run_ok(Command::new(nova()).arg("version"));
    let ours: Vec<&str> = ours.lines().collect();
    assert!(ours[0].starts_with("nova "), "{ours:?}");
    let printed: Vec<&str> = out.lines().collect();
    assert!(
        printed
            .windows(ours.len())
            .any(|lines| lines == ours.as_slice()),
        "the installed nova version is not in:\n{out}"
    );
}

/// The paths of release `version`'s two downloads.
fn downloads(version: &str, target: &str) -> [String; 2] {
    [
        format!(
            "/releases/download/v{version}/{}",
            archive_name(version, target)
        ),
        format!("/releases/download/v{version}/SHA256SUMS"),
    ]
}

#[test]
fn installs_the_latest_release_found_through_the_redirect() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("latest");
    let server = serve(release(&dir, LATEST, target, Sums::Right));

    let output = run(&mut script(&dir, &server));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "{out}\n{err}");
    assert_installed(&bin(&dir));
    assert_prints_version(&out);

    // install.sh's curl follows the redirect to the release's page;
    // install.ps1 only reads where it points.
    let mut expected = vec!["/releases/latest".to_string()];
    if !cfg!(windows) {
        expected.push(format!("/releases/tag/v{LATEST}"));
    }
    expected.extend(downloads(LATEST, target));
    assert_eq!(paths(&server), expected);

    // NOVA_HOME is not under HOME, so install.sh names the directory itself.
    if !cfg!(windows) {
        let line = format!(
            "nova: add nova to your PATH: export PATH=\"{}:$PATH\"",
            bin(&dir).display()
        );
        assert!(out.lines().any(|l| l == line), "no `{line}` in:\n{out}");
    }
}

#[test]
fn nova_version_fetches_that_version_and_never_asks_for_latest() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("chosen-version");
    let server = serve(release(&dir, OLDER, target, Sums::Right));

    let output = run(script(&dir, &server).env("NOVA_VERSION", OLDER));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "{out}\n{err}");
    assert_installed(&bin(&dir));
    assert_prints_version(&out);
    assert_eq!(paths(&server), downloads(OLDER, target));
}

#[test]
fn a_checksum_mismatch_installs_nothing() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("mismatch");
    let server = serve(release(&dir, LATEST, target, Sums::Wrong));
    // An older nova is installed already; a failed install leaves it.
    let exe = bin(&dir).join(format!("nova{EXE_SUFFIX}"));
    std::fs::create_dir_all(bin(&dir)).unwrap();
    std::fs::write(&exe, "an older nova").unwrap();

    let output = run(&mut script(&dir, &server));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(!output.status.success(), "{out}\n{err}");
    let archive = archive_name(LATEST, target);
    assert!(
        err.contains(&archive),
        "the error does not name {archive}:\n{err}"
    );
    assert_eq!(std::fs::read_to_string(&exe).unwrap(), "an older nova");
    assert_eq!(entries(&bin(&dir)), [format!("nova{EXE_SUFFIX}")]);
}

#[test]
fn an_archive_missing_from_sha256sums_installs_nothing() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("missing");
    let server = serve(release(&dir, LATEST, target, Sums::Missing));

    let output = run(&mut script(&dir, &server));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(!output.status.success(), "{out}\n{err}");
    let archive = archive_name(LATEST, target);
    assert!(
        err.contains(&archive),
        "the error does not name {archive}:\n{err}"
    );
    assert!(err.contains("SHA256SUMS"), "{err}");
    assert!(!dir.join("nova-home").exists(), "something was installed");
}

#[test]
fn running_it_again_replaces_the_binary() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("again");
    let server = serve(release(&dir, LATEST, target, Sums::Right));

    let first = run(&mut script(&dir, &server));
    assert!(
        first.status.success(),
        "{}\n{}",
        stdout(&first),
        stderr(&first)
    );
    assert_installed(&bin(&dir));
    // Stand in for an older nova; the second run must replace it.
    std::fs::write(bin(&dir).join(format!("nova{EXE_SUFFIX}")), "an older nova").unwrap();

    let second = run(&mut script(&dir, &server));
    let (out, err) = (stdout(&second), stderr(&second));
    assert!(second.status.success(), "{out}\n{err}");
    assert_installed(&bin(&dir));
    assert_prints_version(&out);
}

#[cfg(unix)]
#[test]
fn install_sh_prints_the_path_line_only_when_the_directory_is_not_on_path() {
    let Some(target) = target_or_skip() else {
        return;
    };
    let dir = fresh("path");
    let server = serve(release(&dir, LATEST, target, Sums::Right));
    // NOVA_HOME is HOME's .nova, as by default, so the line shows $HOME.
    let nova_home = dir.join("home").join(".nova");
    let path = std::env::var("PATH").unwrap_or_default();
    let line = "nova: add nova to your PATH: export PATH=\"$HOME/.nova/bin:$PATH\"";

    let output = run(script(&dir, &server)
        .env("NOVA_HOME", &nova_home)
        .env("PATH", &path));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "{out}\n{err}");
    assert!(out.lines().any(|l| l == line), "no `{line}` in:\n{out}");

    let on_path = format!("{}:{path}", nova_home.join("bin").display());
    let output = run(script(&dir, &server)
        .env("NOVA_HOME", &nova_home)
        .env("PATH", on_path));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(output.status.success(), "{out}\n{err}");
    assert!(
        !out.contains("add nova to your PATH"),
        "a PATH line, though nova is on PATH:\n{out}"
    );
    assert_installed(&nova_home.join("bin"));
}

#[cfg(unix)]
#[test]
fn install_sh_on_an_unknown_system_names_cargo_install() {
    use std::os::unix::fs::PermissionsExt;

    let dir = fresh("unknown-system");
    let server = serve(HashMap::new());
    let fake = dir.join("fake-bin");
    std::fs::create_dir_all(&fake).unwrap();
    let uname = fake.join("uname");
    let path = format!(
        "{}:{}",
        fake.display(),
        std::env::var("PATH").unwrap_or_default()
    );

    for (os, arch) in [("FreeBSD", "amd64"), ("Linux", "aarch64")] {
        std::fs::write(
            &uname,
            format!("#!/bin/sh\ncase \"$1\" in\n  -m) echo {arch} ;;\n  *) echo {os} ;;\nesac\n"),
        )
        .unwrap();
        std::fs::set_permissions(&uname, std::fs::Permissions::from_mode(0o755)).unwrap();

        let output = run(script(&dir, &server).env("PATH", &path));
        let (out, err) = (stdout(&output), stderr(&output));
        assert!(!output.status.success(), "{out}\n{err}");
        let message = format!(
            "no ready-built nova for {os} {arch}; install it with: cargo install nova-cli --locked"
        );
        assert!(err.contains(&message), "no `{message}` in:\n{err}");
    }
    assert_eq!(paths(&server), Vec::<String>::new());
    assert!(!dir.join("nova-home").exists());
}

#[cfg(windows)]
#[test]
fn install_ps1_on_an_unknown_system_names_cargo_install() {
    let dir = fresh("unknown-system");
    let server = serve(HashMap::new());

    let output = run(script(&dir, &server)
        .env("PROCESSOR_ARCHITECTURE", "ARM64")
        .env_remove("PROCESSOR_ARCHITEW6432"));
    let (out, err) = (stdout(&output), stderr(&output));
    assert!(!output.status.success(), "{out}\n{err}");
    let message =
        "no ready-built nova for Windows_NT ARM64; install it with: cargo install nova-cli --locked";
    assert!(err.contains(message), "no `{message}` in:\n{err}");
    assert_eq!(paths(&server), Vec::<String>::new());
    assert!(!dir.join("nova-home").exists());
}

/// Windows PowerShell 5.1 reads a script that has no byte-order mark in
/// the system's code page, and `irm | iex` would choke on a mark, so both
/// scripts are plain ASCII; `install.sh` also has Unix line ends, for `sh`.
#[test]
fn the_scripts_are_plain_ascii() {
    for name in ["install.sh", "install.ps1"] {
        let path = repo().join("tools").join("install").join(name);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        if let Some(at) = bytes.iter().position(|b| !b.is_ascii()) {
            panic!("{name} has a byte that is not ASCII at offset {at}");
        }
    }
    let sh =
        std::fs::read_to_string(repo().join("tools").join("install").join("install.sh")).unwrap();
    assert!(
        sh.starts_with("#!/bin/sh\n"),
        "install.sh does not start with #!/bin/sh"
    );
    assert!(!sh.contains('\r'), "install.sh has a carriage return");
}
````

- [ ] **Step 2: Windows PowerShell gets its own module path (decision 49)**

On Windows, CI runs `cargo test` under PowerShell 7 (the runner's default
shell), which puts its own module paths first in `PSModulePath`. The test
starts Windows PowerShell through `cargo` and the test binary, so it would
inherit them, and then could not load its own `Microsoft.PowerShell.*`
modules: `Compress-Archive`, `Get-FileHash` and the cmdlets `install.ps1`
uses would not be found. The three places that start `powershell` go
through one helper that removes the variable, as Microsoft's
`about_PSModulePath` advises. Lines 3927-3973 of this plan:

Run: `cd /home/user/nova && sed -n '3927,3973p' "$PLAN" > "$P/36a-t6-psmodulepath.diff" && git apply "$P/36a-t6-psmodulepath.diff"`

````diff
diff --git a/crates/nova-cli/tests/install_scripts.rs b/crates/nova-cli/tests/install_scripts.rs
index 1a706b9..52140e5 100644
--- a/crates/nova-cli/tests/install_scripts.rs
+++ b/crates/nova-cli/tests/install_scripts.rs
@@ -101,11 +101,23 @@ fn run_ok(command: &mut Command) -> String {
     stdout(&output)
 }
 
+/// Windows PowerShell 5.1, started without the `PSModulePath` this test
+/// inherited. Under PowerShell 7, which runs CI's `cargo test` on Windows,
+/// that variable leads with PowerShell 7's module paths, and Windows
+/// PowerShell started through another process, as here, then cannot load
+/// its own `Microsoft.PowerShell.*` modules; without the variable it builds
+/// its default (Microsoft's `about_PSModulePath`).
+fn windows_powershell() -> Command {
+    let mut command = Command::new("powershell");
+    command.env_remove("PSModulePath");
+    command
+}
+
 /// The SHA-256 of `path`, in lower-case hex, from the system's own tool.
 fn sha256(path: &Path) -> String {
     let printed = if cfg!(windows) {
         run_ok(
-            Command::new("powershell")
+            windows_powershell()
                 .args(["-NoProfile", "-NonInteractive", "-Command"])
                 .arg("(Get-FileHash -LiteralPath $env:HASH_FILE -Algorithm SHA256).Hash")
                 .env("HASH_FILE", path),
@@ -144,7 +156,7 @@ fn pack(dir: &Path, version: &str, target: &str) -> PathBuf {
     let archive = archive_name(version, target);
     if cfg!(windows) {
         run_ok(
-            Command::new("powershell")
+            windows_powershell()
                 .args(["-NoProfile", "-NonInteractive", "-Command"])
                 .arg("Compress-Archive -LiteralPath $env:PACK_SOURCE -DestinationPath $env:PACK_ARCHIVE")
                 .env("PACK_SOURCE", &inner)
@@ -236,7 +248,7 @@ fn paths(server: &Server) -> Vec<String> {
 /// `NOVA_VERSION` and no proxy. On Windows, `NOVA_INSTALL_NO_PATH=1`.
 fn script(dir: &Path, server: &Server) -> Command {
     let mut command = if cfg!(windows) {
-        let mut command = Command::new("powershell");
+        let mut command = windows_powershell();
         command
             .args([
                 "-NoProfile",
````

- [ ] **Step 3: Run it to see it fail**

Each archive test packs a gzip of the debug `nova` (about 185 MB), so the
run takes about 20 s and leaves about 1 GB under the temporary directory,
which the next run replaces.

Run: `cd /home/user/nova && RUST_BACKTRACE=0 cargo test --locked -p nova-cli --test install_scripts 2>&1 | grep -E "^test |cannot open|No such file|^test result"`
Expected: FAIL, `test result: FAILED. 0 passed; 8 failed`: seven tests
with `sh: 0: cannot open /home/user/nova/tools/install/install.sh: No such file`
in their message, and `the_scripts_are_plain_ascii` with
`/home/user/nova/tools/install/install.sh: No such file or directory (os error 2)`.

- [ ] **Step 4: `install.sh`**

`tools/install/install.sh`, lines 3995-4171 of this plan:

Run: `cd /home/user/nova && sed -n '3995,4171p' "$PLAN" > tools/install/install.sh`

````bash
#!/bin/sh
# Install nova, the Nova toolchain, on Linux or macOS from a GitHub
# release: this system's archive, checked against the release's
# SHA256SUMS, into $NOVA_HOME/bin (by default ~/.nova/bin). Running it
# again updates nova; deleting ~/.nova uninstalls it. Spec:
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# section 6.
#
#   curl -sSfL https://github.com/Sakeerin/nova/releases/latest/download/install.sh | sh
#
# Environment:
#   NOVA_VERSION       the version to install, such as 0.3.0
#                      (default: the latest release)
#   NOVA_HOME          nova's home; nova goes into its bin directory
#                      (default: ~/.nova)
#   NOVA_INSTALL_BASE  where the releases are
#                      (default: https://github.com/Sakeerin/nova/releases)
#
# It edits no file: when the directory is not on PATH, it prints the line
# to add. It needs curl, tar, and sha256sum or shasum. POSIX sh: it runs
# under dash. Everything happens in main, called on the last line, so a
# download cut short runs nothing.
set -eu

say() {
  printf 'nova: %s\n' "$*"
}

die() {
  printf 'nova: error: %s\n' "$*" >&2
  exit 1
}

need() {
  command -v "$1" > /dev/null 2>&1 || die "this script needs $1, which is not installed"
}

# This system's release target (spec section 6.1, step 1).
detect_target() {
  os=$(uname -s)
  arch=$(uname -m)
  case "$os $arch" in
    "Linux x86_64" | "Linux amd64") target=x86_64-unknown-linux-gnu ;;
    "Darwin arm64") target=aarch64-apple-darwin ;;
    "Darwin x86_64") target=x86_64-apple-darwin ;;
    *) die "no ready-built nova for $os $arch; install it with: cargo install nova-cli --locked" ;;
  esac
}

# NOVA_VERSION, or the latest release's version, read from where
# <base>/latest redirects: .../releases/tag/v<version> (step 2).
pick_version() {
  if [ -n "${NOVA_VERSION:-}" ]; then
    version=$NOVA_VERSION
  else
    say "finding the latest release at $base/latest"
    if ! effective=$(curl -sSfL -o "$tmp/latest" -w '%{url_effective}' "$base/latest"); then
      die "could not reach $base/latest"
    fi
    case "$effective" in
      */tag/*) ;;
      *) die "could not tell the latest version: $base/latest led to $effective" ;;
    esac
    version=${effective##*/tag/}
    version=${version%%[/?#]*}
  fi
  version=${version#v}
  case "$version" in
    "" | *[!0-9A-Za-z.+-]*) die "'$version' is not a version" ;;
  esac
}

fetch() {
  say "downloading $1"
  curl -sSfL -o "$2" "$1" || die "could not download $1"
}

sha256_of() {
  if [ "$sha_tool" = sha256sum ]; then
    sha256sum "$1"
  else
    shasum -a 256 "$1"
  fi
}

main() {
  need curl
  need tar
  if command -v sha256sum > /dev/null 2>&1; then
    sha_tool=sha256sum
  elif command -v shasum > /dev/null 2>&1; then
    sha_tool=shasum
  else
    die "this script needs sha256sum or shasum, and neither is installed"
  fi

  base=${NOVA_INSTALL_BASE:-https://github.com/Sakeerin/nova/releases}
  base=${base%/}
  nova_home=${NOVA_HOME:-}
  if [ -z "$nova_home" ]; then
    [ -n "${HOME:-}" ] || die "neither NOVA_HOME nor HOME is set"
    nova_home=$HOME/.nova
  fi
  bin=$nova_home/bin

  detect_target
  tmp_root=${TMPDIR:-/tmp}
  tmp=$(mktemp -d "${tmp_root%/}/nova-install.XXXXXX") || die "could not make a temporary directory"
  pick_version
  name=nova-$version-$target
  archive=$name.tar.gz
  say "installing nova $version for $target"

  # Step 3: the archive and SHA256SUMS, into the temporary directory.
  fetch "$base/download/v$version/$archive" "$tmp/$archive"
  fetch "$base/download/v$version/SHA256SUMS" "$tmp/SHA256SUMS"

  # Step 4: the archive's SHA-256 against its line in SHA256SUMS.
  expected=$(awk -v name="$archive" '{ sub(/\r$/, "") } $2 == name || $2 == "*" name { print tolower($1); exit }' "$tmp/SHA256SUMS")
  [ -n "$expected" ] || die "SHA256SUMS has no line for $archive, so it cannot be checked; nothing was installed"
  actual=$(sha256_of "$tmp/$archive" | awk '{ print tolower($1) }')
  [ "$actual" = "$expected" ] || die "$archive does not match its SHA-256 in SHA256SUMS (expected $expected, got $actual); nothing was installed"
  say "checked $archive against SHA256SUMS"

  mkdir "$tmp/unpacked"
  tar -xzf "$tmp/$archive" -C "$tmp/unpacked" || die "could not unpack $archive"
  [ -f "$tmp/unpacked/$name/nova" ] || die "$archive holds no $name/nova"

  # Step 5: written beside the old nova, then renamed over it, so a failed
  # install leaves the old one.
  mkdir -p "$bin" || die "could not make $bin"
  bin=$(CDPATH='' cd -- "$bin" && pwd)
  staged=$bin/.nova-install.$$
  cp "$tmp/unpacked/$name/nova" "$staged" || die "could not write to $bin"
  chmod +x "$staged"
  mv -f "$staged" "$bin/nova" || die "could not replace $bin/nova"
  staged=
  say "installed $bin/nova"

  # Step 6: the installed nova says what it is.
  if ! printed=$("$bin/nova" version); then
    die "nova was installed in $bin, but \`nova version\` failed"
  fi
  printf '%s\n' "$printed"

  # Step 7: no file is edited; the line to add, if the directory is not on
  # PATH, with $HOME as it is when the directory is under it.
  case ":$PATH:" in
    *":$bin:"* | *":$bin/:"*) ;;
    *)
      shown=$bin
      home_dir=${HOME:-}
      home_dir=${home_dir%/}
      if [ -n "$home_dir" ]; then
        case "$bin" in
          "$home_dir"/*) shown="\$HOME${bin#"$home_dir"}" ;;
        esac
      fi
      say "add nova to your PATH: export PATH=\"$shown:\$PATH\""
      ;;
  esac
}

tmp=
staged=
cleanup() {
  if [ -n "$staged" ]; then
    rm -f "$staged"
  fi
  if [ -n "$tmp" ]; then
    rm -rf "$tmp"
  fi
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

main "$@"
````

Everything is in `main`, called on the last line, so a download cut short
runs nothing, and under `curl | sh` no command reads the rest of the script
from stdin.

- [ ] **Step 5: Run the cases `install.sh` passes**

Run: `cd /home/user/nova && RUST_BACKTRACE=0 cargo test --locked -p nova-cli --test install_scripts 2>&1 | grep -E "^test |No such file|^test result"`
Expected: `test result: FAILED. 7 passed; 1 failed`, in about 23 s. The one
failure is `the_scripts_are_plain_ascii`, with
`/home/user/nova/tools/install/install.ps1: No such file or directory (os error 2)`:
`install.ps1`'s failing test, which Step 6 makes pass.

Run: `cd /home/user/nova && sh -n tools/install/install.sh && $SC -s sh tools/install/install.sh && echo clean`
Expected: `clean`.

- [ ] **Step 6: `install.ps1`**

`tools/install/install.ps1`, lines 4196-4427 of this plan (decisions 22 to 25):

Run: `cd /home/user/nova && sed -n '4196,4427p' "$PLAN" > tools/install/install.ps1`

````powershell
# Install nova, the Nova toolchain, on Windows from a GitHub release: the
# x86_64-pc-windows-msvc archive, checked against the release's
# SHA256SUMS, into $env:NOVA_HOME\bin (by default %USERPROFILE%\.nova\bin),
# which it adds to the user's PATH. Running it again updates nova;
# deleting %USERPROFILE%\.nova uninstalls it. Spec:
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# section 6.
#
#   irm https://github.com/Sakeerin/nova/releases/latest/download/install.ps1 | iex
#
# Environment:
#   NOVA_VERSION          the version to install, such as 0.3.0
#                         (default: the latest release)
#   NOVA_HOME             nova's home; nova goes into its bin directory
#                         (default: %USERPROFILE%\.nova)
#   NOVA_INSTALL_BASE     where the releases are
#                         (default: https://github.com/Sakeerin/nova/releases)
#   NOVA_INSTALL_NO_PATH  1 leaves PATH alone
#
# It runs in Windows PowerShell 5.1 and PowerShell 7, and is plain ASCII,
# because 5.1 reads a script without a byte-order mark in the system's
# code page. Everything happens in one script block, so `iex` leaves
# nothing behind in the session. An error goes to stderr and exits 1 when
# the script runs as a file; through `iex` at a prompt it is thrown
# instead, so the window stays open.

& {
    Set-StrictMode -Version Latest
    $ErrorActionPreference = 'Stop'
    # Invoke-WebRequest's progress bar slows 5.1's downloads many times over.
    $ProgressPreference = 'SilentlyContinue'

    function Say([string] $Message) {
        Write-Host "nova: $Message"
    }

    # This system's release target (spec section 6.1, step 1). A 32-bit
    # PowerShell on 64-bit Windows sees x86 in PROCESSOR_ARCHITECTURE and
    # the machine's AMD64 in PROCESSOR_ARCHITEW6432.
    function Get-NovaTarget {
        $os = $env:OS
        $arch = $env:PROCESSOR_ARCHITEW6432
        if (-not $arch) { $arch = $env:PROCESSOR_ARCHITECTURE }
        if ($os -eq 'Windows_NT' -and $arch -eq 'AMD64') {
            return 'x86_64-pc-windows-msvc'
        }
        if (-not $os) { $os = [Environment]::OSVersion.Platform.ToString() }
        if (-not $arch) { $arch = 'unknown' }
        throw "no ready-built nova for $os $arch; install it with: cargo install nova-cli --locked"
    }

    # The latest release's version, from where <base>/latest redirects:
    # .../releases/tag/v<version> (step 2). The redirect is read, not
    # followed.
    function Get-LatestVersion([string] $Base) {
        Add-Type -AssemblyName System.Net.Http
        $handler = New-Object System.Net.Http.HttpClientHandler
        $handler.AllowAutoRedirect = $false
        $client = New-Object System.Net.Http.HttpClient($handler)
        try {
            $response = $client.GetAsync("$Base/latest").GetAwaiter().GetResult()
            $location = $response.Headers.Location
            $status = [int] $response.StatusCode
            $response.Dispose()
        } catch {
            throw "could not reach $Base/latest: $($_.Exception.GetBaseException().Message)"
        } finally {
            $client.Dispose()
        }
        if ($null -eq $location) {
            throw "could not tell the latest version: $Base/latest answered $status, not a redirect"
        }
        $text = $location.OriginalString
        if ($text -notmatch '/tag/([^/?#]+)') {
            throw "could not tell the latest version: $Base/latest redirects to $text"
        }
        return $Matches[1]
    }

    function Save-Download([string] $Url, [string] $Path) {
        Say "downloading $Url"
        try {
            Invoke-WebRequest -Uri $Url -OutFile $Path -UseBasicParsing
        } catch {
            throw "could not download ${Url}: $($_.Exception.GetBaseException().Message)"
        }
    }

    # The same directory, whatever the case and a trailing backslash, and
    # with any %VARIABLE% in the PATH entry expanded.
    function Test-SameDirectory([string] $Entry, [string] $Dir) {
        $expanded = [Environment]::ExpandEnvironmentVariables($Entry).TrimEnd('\')
        return [string]::Equals($expanded, $Dir.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)
    }

    # Step 7: the user's PATH, as the registry holds it, and this session's.
    # The value is read with its %VARIABLES% unexpanded and written back as
    # REG_EXPAND_SZ, as rustup does; [Environment]::SetEnvironmentVariable
    # would store every entry expanded, as REG_SZ.
    function Add-ToUserPath([string] $Dir) {
        $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey('Environment')
        try {
            $current = [string] $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
            $entries = @($current -split ';' | Where-Object { $_ -ne '' })
            $present = @($entries | Where-Object { Test-SameDirectory $_ $Dir }).Count -gt 0
            if ($present) {
                Say "$Dir is on your PATH already"
            } else {
                $updated = (@($Dir) + $entries) -join ';'
                $key.SetValue('Path', $updated, [Microsoft.Win32.RegistryValueKind]::ExpandString)
                # Setting a user variable through [Environment] tells running
                # programs, Explorer among them, to read the environment
                # again; this one is only removed, as it never exists.
                [Environment]::SetEnvironmentVariable('NOVA_INSTALL_PATH_CHANGED', $null, 'User')
                Say "added $Dir to your PATH; open a new terminal for other programs to see it"
            }
        } finally {
            $key.Close()
        }
        $session = @($env:Path -split ';' | Where-Object { $_ -ne '' })
        if (@($session | Where-Object { Test-SameDirectory $_ $Dir }).Count -eq 0) {
            $env:Path = "$Dir;$env:Path"
        }
    }

    function Install-Nova {
        # Windows PowerShell 5.1 may not offer TLS 1.2, which GitHub needs.
        [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

        $target = Get-NovaTarget
        $base = $env:NOVA_INSTALL_BASE
        if (-not $base) { $base = 'https://github.com/Sakeerin/nova/releases' }
        $base = $base.TrimEnd('/')
        $novaHome = $env:NOVA_HOME
        if (-not $novaHome) {
            if (-not $env:USERPROFILE) { throw 'neither NOVA_HOME nor USERPROFILE is set' }
            $novaHome = Join-Path $env:USERPROFILE '.nova'
        }
        $bin = Join-Path $novaHome 'bin'

        $version = $env:NOVA_VERSION
        if (-not $version) {
            Say "finding the latest release at $base/latest"
            $version = Get-LatestVersion $base
        }
        $version = $version -replace '^v', ''
        if ($version -notmatch '^[0-9A-Za-z.+-]+$') { throw "'$version' is not a version" }
        $name = "nova-$version-$target"
        $archive = "$name.zip"
        Say "installing nova $version for $target"

        $tmp = Join-Path ([IO.Path]::GetTempPath()) ('nova-install-' + [Guid]::NewGuid().ToString('N'))
        New-Item -ItemType Directory -Path $tmp | Out-Null
        $staged = $null
        try {
            # Step 3: the archive and SHA256SUMS, into the temporary directory.
            $zip = Join-Path $tmp $archive
            $sums = Join-Path $tmp 'SHA256SUMS'
            Save-Download "$base/download/v$version/$archive" $zip
            Save-Download "$base/download/v$version/SHA256SUMS" $sums

            # Step 4: the archive's SHA-256 against its line in SHA256SUMS.
            # Get-FileHash and Expand-Archive are module functions, which do
            # not see this scope's $ErrorActionPreference, hence -ErrorAction.
            $expected = $null
            foreach ($line in [IO.File]::ReadAllLines($sums)) {
                $fields = @($line.Trim() -split '\s+')
                if ($fields.Count -eq 2 -and $fields[1].TrimStart('*') -ceq $archive) {
                    $expected = $fields[0].ToLowerInvariant()
                    break
                }
            }
            if ($null -eq $expected) {
                throw "SHA256SUMS has no line for $archive, so it cannot be checked; nothing was installed"
            }
            $actual = (Get-FileHash -LiteralPath $zip -Algorithm SHA256 -ErrorAction Stop).Hash.ToLowerInvariant()
            if ($actual -cne $expected) {
                throw "$archive does not match its SHA-256 in SHA256SUMS (expected $expected, got $actual); nothing was installed"
            }
            Say "checked $archive against SHA256SUMS"

            $unpacked = Join-Path $tmp 'unpacked'
            Expand-Archive -LiteralPath $zip -DestinationPath $unpacked -ErrorAction Stop
            $source = Join-Path (Join-Path $unpacked $name) 'nova.exe'
            if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
                throw "$archive holds no $name\nova.exe"
            }

            # Step 5: written beside the old nova, then moved over it, so a
            # failed install leaves the old one.
            New-Item -ItemType Directory -Path $bin -Force | Out-Null
            $bin = (Get-Item -LiteralPath $bin).FullName
            $exe = Join-Path $bin 'nova.exe'
            $staged = Join-Path $bin ('nova-install-' + [Guid]::NewGuid().ToString('N') + '.tmp')
            Copy-Item -LiteralPath $source -Destination $staged
            Move-Item -LiteralPath $staged -Destination $exe -Force
            $staged = $null
            Say "installed $exe"

            # Step 6: the installed nova says what it is.
            try {
                $printed = @(& $exe version)
            } catch {
                throw "nova was installed in $bin, but did not run: $($_.Exception.Message)"
            }
            if ($LASTEXITCODE -ne 0) { throw "nova was installed in $bin, but 'nova version' failed" }
            foreach ($printedLine in $printed) { Write-Host $printedLine }
        } finally {
            if ($staged -and (Test-Path -LiteralPath $staged)) {
                Remove-Item -LiteralPath $staged -Force -ErrorAction SilentlyContinue
            }
            Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
        }

        if ($env:NOVA_INSTALL_NO_PATH -eq '1') {
            Say "NOVA_INSTALL_NO_PATH is 1, so PATH was left alone; nova is in $bin"
        } else {
            Add-ToUserPath $bin
        }
    }

    try {
        Install-Nova
    } catch {
        $message = "nova: error: $($_.Exception.Message)"
        if (Get-Variable -Name PSCommandPath -ValueOnly -ErrorAction SilentlyContinue) {
            [Console]::Error.WriteLine($message)
            exit 1
        }
        throw $message
    }
}
````

- [ ] **Step 7: Run every case**

Run: `cd /home/user/nova && cargo test --locked -p nova-cli --test install_scripts 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 8 passed`, in about 23 s.

- [ ] **Step 8: PowerShell, if `$P/pwsh-dl` is there (optional)**

PowerShell 7.4.6 for Linux was downloaded while prototyping, with its
SHA-256 checked. With it, the script parses, and the case of mutant 7 runs
to the end against a loopback release (a zip whose stand-in `nova.exe`
keeps its exec bit):

Run: `cd /home/user/nova && PS_FILE=tools/install/install.ps1 "$P/pwsh-dl/pwsh/pwsh" -NoProfile -NonInteractive -Command '$e = $null; $t = $null; [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path $env:PS_FILE), [ref]$t, [ref]$e) | Out-Null; "parse errors: " + $e.Count' && bash "$P/mut7-36a.sh" tools/install/install.ps1 "$P/mut7-work" | tail -n 3`
Expected:

````text
parse errors: 0
install.ps1 exited with 0; the server was asked for:
GET /releases/download/v1.2.2/nova-1.2.2-x86_64-pc-windows-msvc.zip
GET /releases/download/v1.2.2/SHA256SUMS
````

Not verified anywhere but CI: Windows PowerShell 5.1 itself
(`System.Net.Http`, `Invoke-WebRequest -UseBasicParsing -OutFile`, 5.1's
`Expand-Archive`), replacing a real `nova.exe`, and the PATH code against a
real registry, which Task 7's `release.yml` step runs. The PATH code's
logic did run while planning, on Linux against a stand-in registry, through
that step (Task 7, Step 2).

- [ ] **Step 9: Checks and commit**

The checks (Conventions), with
`msrv_test crates/nova-cli/tests/install_scripts.rs base64 serde_json`, and
again as
`msrv_test crates/nova-cli/tests/install_scripts.rs base64 serde_json -- --cfg windows`,
which type-checks the Windows-only test that clippy on Linux does not see.
Stage `crates/nova-cli/tests/install_scripts.rs`, `tools/install/install.sh`
and `tools/install/install.ps1`.
Message: `install: install.sh and install.ps1, nova from a release's archive after SHA256SUMS`,
body: install.sh, POSIX sh for Linux and macOS, and install.ps1, Windows
PowerShell 5.1 and 7: the latest release through the latest redirect
(install.ps1 reads its Location without following it), or NOVA_VERSION;
the archive checked against SHA256SUMS; nova written beside the old one
and moved over it in $NOVA_HOME/bin. install.sh prints the PATH line and
edits no file; install.ps1 puts the directory first in the user's PATH
through the registry, keeping REG_EXPAND_SZ, unless NOVA_INSTALL_NO_PATH=1.
Both are plain ASCII, and install.ps1 runs in one script block, so
irm | iex leaves nothing behind. install_scripts.rs runs them against a
fake release on loopback, starting Windows PowerShell without the test's
PSModulePath (spec 3.6a §6, §11.3).

---

### Task 7: `release.yml` (spec §6.3)

**Files:**
- Modify: `.github/workflows/release.yml` (the prototype's `prepare`
  changes; the Windows PATH step, new in this plan)

**Interfaces:** none. The release's assets are now eight: four archives,
the `.vsix`, std's site and the two install scripts, and `SHA256SUMS`
covers them.

- [ ] **Step 1: `prepare` attaches the scripts and runs `install.sh`**

Lines 4500-4588 of this plan:

Run: `cd /home/user/nova && sed -n '4500,4588p' "$PLAN" > "$P/36a-t7-prepare.diff" && git apply "$P/36a-t7-prepare.diff"`

````diff
diff --git a/.github/workflows/release.yml b/.github/workflows/release.yml
index 64c527e..f4c8870 100644
--- a/.github/workflows/release.yml
+++ b/.github/workflows/release.yml
@@ -2,10 +2,13 @@ name: Release
 
 # On a `v*` tag: build the four targets, pack and smoke-test each archive,
 # package the VS Code extension, and publish a GitHub release with the
-# archives, the `.vsix`, std's documentation site and SHA256SUMS. On a
-# pull request that changes how `nova` is built or released, run every job
-# but the publish, so a broken release shows before any tag exists (spec
-# docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §7).
+# archives, the `.vsix`, std's documentation site, the install scripts and
+# SHA256SUMS. On a pull request that changes how `nova` is built or
+# released, run every job but the publish, so a broken release shows before
+# any tag exists (spec
+# docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §7;
+# the install scripts,
+# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md §6.3).
 on:
   push:
     tags: ['v*']
@@ -21,6 +24,7 @@ on:
       - Cargo.toml
       - Cargo.lock
       - tools/vscode-nova/**
+      - tools/install/**
 
 # Only `publish` may write to the repository.
 permissions:
@@ -165,14 +169,19 @@ jobs:
         run: |
           set -euo pipefail
           version=$(tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
+          # The install scripts are assets of every release, so that
+          # releases/latest/download/ serves the latest release's own, and
+          # SHA256SUMS covers them.
+          cp tools/install/install.sh tools/install/install.ps1 release/dist/
           # An archive is nova-<version>-<target>; the extension is
           # nova-vscode-<version>.vsix; std's site is
           # nova-std-docs-<version>.tar.gz.
           archives=$(ls release/dist | grep -c -E '^nova-[0-9]' || true)
           vsix=$(ls release/dist | grep -c -E '^nova-vscode-.+\.vsix$' || true)
           docs=$(ls release/dist | grep -c -E '^nova-std-docs-.+\.tar\.gz$' || true)
-          if [ "$archives" -ne 4 ] || [ "$vsix" -ne 1 ] || [ "$docs" -ne 1 ]; then
-            echo "::error::expected 4 archives, 1 .vsix and std's site, found $archives, $vsix and $docs"
+          scripts=$(ls release/dist | grep -c -E '^install\.(sh|ps1)$' || true)
+          if [ "$archives" -ne 4 ] || [ "$vsix" -ne 1 ] || [ "$docs" -ne 1 ] || [ "$scripts" -ne 2 ]; then
+            echo "::error::expected 4 archives, 1 .vsix, std's site and the 2 install scripts, found $archives, $vsix, $docs and $scripts"
             ls release/dist
             exit 1
           fi
@@ -190,6 +199,36 @@ jobs:
           prerelease=""
           case "$tag" in *-*) prerelease=" --prerelease" ;; esac
           echo "publish runs: gh release create $tag --verify-tag --title \"Nova $version\" --notes-file notes.md$prerelease dist/*"
+      # install.sh, run as a user would with NOVA_VERSION, against this
+      # release's own Linux archive and SHA256SUMS, served over loopback the
+      # way <base>/download/v<version>/ serves them: the real archive's
+      # layout, checksum and binary.
+      - name: Install the Linux archive with install.sh
+        shell: bash
+        run: |
+          set -euo pipefail
+          version=$(tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
+          srv="$RUNNER_TEMP/srv"
+          mkdir -p "$srv/releases/download/v$version"
+          cp "release/dist/nova-$version-x86_64-unknown-linux-gnu.tar.gz" release/dist/SHA256SUMS \
+            "$srv/releases/download/v$version/"
+          # A fixed port: the runner is fresh. The trap stops the server
+          # however the step ends.
+          port=8431
+          python3 -m http.server --bind 127.0.0.1 "$port" --directory "$srv" > "$RUNNER_TEMP/http.log" 2>&1 &
+          server=$!
+          trap 'kill "$server" 2> /dev/null || true' EXIT
+          base="http://127.0.0.1:$port/releases"
+          # Wait until the server answers; quiet, since the first try may be
+          # refused (curl exits non-zero if it never answers).
+          curl -sf --retry 10 --retry-connrefused --retry-delay 1 "$base/download/v$version/SHA256SUMS" > /dev/null
+          NOVA_INSTALL_BASE="$base" NOVA_VERSION="$version" NOVA_HOME="$RUNNER_TEMP/install-home" \
+            sh tools/install/install.sh
+          "$RUNNER_TEMP/install-home/bin/nova" version | tee "$RUNNER_TEMP/installed-version.txt"
+          if ! grep -qx 'runtime: embedded' "$RUNNER_TEMP/installed-version.txt"; then
+            echo "::error::the nova install.sh installed does not carry its runtime library"
+            exit 1
+          fi
       - uses: actions/upload-artifact@v4
         with:
           name: release
````

`prepare` copies both scripts into `release/dist` before anything is
counted or hashed, and its count check expects them; a new step serves the
release's own Linux archive and `SHA256SUMS` over loopback, runs
`install.sh` with `NOVA_VERSION`, and requires `runtime: embedded` from the
installed `nova` (decision 27). The pull-request paths gain
`tools/install/**`.

- [ ] **Step 2: The Windows build runs `install.ps1` with PATH on (decision 40)**

Not in the prototype; written for spec §6.3 as amended. In the `build`
job, after "Smoke-test the archive" and before the upload, lines 4606-4719 of this plan:

Run: `cd /home/user/nova && sed -n '4606,4719p' "$PLAN" > "$P/36a-t7-windows.diff" && git apply "$P/36a-t7-windows.diff"`

````diff
diff --git a/.github/workflows/release.yml b/.github/workflows/release.yml
index f4c8870..7db4748 100644
--- a/.github/workflows/release.yml
+++ b/.github/workflows/release.yml
@@ -91,6 +91,109 @@ jobs:
             *) tar -C "$unpacked" -xzf "$ARCHIVE" ;;
           esac
           bash .github/scripts/gate.sh "$(ls -d "$unpacked"/nova-*)/nova" "$RUNNER_TEMP/smoke"
+      # install.ps1 (spec
+      # docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
+      # §6.3), run in Windows PowerShell 5.1 as a user would, with
+      # NOVA_VERSION, against this job's own archive and a SHA256SUMS line
+      # for it, served over loopback the way <base>/download/v<version>/
+      # serves them, and with PATH on. The installed nova must carry its
+      # runtime, and the user's PATH in the registry must then start with
+      # the install directory and keep its other entries as they were, as
+      # REG_EXPAND_SZ; a second run, which is how nova is updated, must
+      # leave the directory there once. No test runs install.ps1's PATH
+      # code; the runner is thrown away after the job, so its PATH may
+      # change.
+      - name: Install the archive with install.ps1, PATH on
+        if: runner.os == 'Windows'
+        shell: pwsh
+        timeout-minutes: 10
+        run: |
+          $ErrorActionPreference = 'Stop'
+          # The workspace's version, as "Pack the archive" reads it.
+          $version = (Select-String -Path Cargo.toml -Pattern '^version = "(.*)"$' -CaseSensitive | Select-Object -First 1).Matches[0].Groups[1].Value
+          $archive = Split-Path -Leaf $env:ARCHIVE
+          $srv = Join-Path $env:RUNNER_TEMP 'srv'
+          $download = Join-Path $srv "releases\download\v$version"
+          New-Item -ItemType Directory -Path $download -Force | Out-Null
+          Copy-Item -LiteralPath $env:ARCHIVE -Destination $download
+          # The archive's line, as sha256sum writes it: the SHA-256 in lower
+          # case, two spaces, the file's name.
+          $hash = (Get-FileHash -LiteralPath (Join-Path $download $archive) -Algorithm SHA256).Hash.ToLowerInvariant()
+          [IO.File]::WriteAllText((Join-Path $download 'SHA256SUMS'), "$hash  $archive`n")
+          # The user's PATH as the registry holds it, %VARIABLES% unexpanded,
+          # which is what install.ps1 reads and writes, and what a new
+          # terminal starts from.
+          function Read-UserPath {
+            $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment')
+            try {
+              $text = [string] $key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
+              $kind = if ($text) { $key.GetValueKind('Path') } else { $null }
+            } finally {
+              $key.Close()
+            }
+            [pscustomobject] @{ Text = $text; Kind = $kind; Entries = @($text -split ';' | Where-Object { $_ -ne '' }) }
+          }
+          # Started by pwsh itself, so Windows PowerShell builds its own
+          # PSModulePath; through another process it would inherit pwsh's,
+          # and its cmdlets would not load.
+          function Invoke-InstallPs1 {
+            powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File tools/install/install.ps1
+            if ($LASTEXITCODE -ne 0) { throw "install.ps1 exited with $LASTEXITCODE" }
+          }
+          # A fixed port: the runner is fresh. `finally` stops the server
+          # however the step ends.
+          $port = 8431
+          $server = Start-Process -FilePath python -ArgumentList '-m', 'http.server', '--bind', '127.0.0.1', "$port", '--directory', $srv -PassThru -NoNewWindow -RedirectStandardOutput (Join-Path $env:RUNNER_TEMP 'http.out') -RedirectStandardError (Join-Path $env:RUNNER_TEMP 'http.err')
+          try {
+            $base = "http://127.0.0.1:$port/releases"
+            # Wait until the server answers.
+            $ready = $false
+            for ($i = 0; $i -lt 30 -and -not $ready; $i++) {
+              try {
+                Invoke-WebRequest -Uri "$base/download/v$version/SHA256SUMS" -UseBasicParsing | Out-Null
+                $ready = $true
+              } catch {
+                Start-Sleep -Seconds 1
+              }
+            }
+            if (-not $ready) { throw "the loopback server at $base never answered" }
+            $installHome = Join-Path $env:RUNNER_TEMP 'install-home'
+            $bin = Join-Path $installHome 'bin'
+            $env:NOVA_INSTALL_BASE = $base
+            $env:NOVA_VERSION = $version
+            $env:NOVA_HOME = $installHome
+            Remove-Item Env:NOVA_INSTALL_NO_PATH -ErrorAction SilentlyContinue
+            $before = Read-UserPath
+            Invoke-InstallPs1
+            $printed = @(& (Join-Path $bin 'nova.exe') version)
+            if ($LASTEXITCODE -ne 0) { throw "the installed nova version exited with $LASTEXITCODE" }
+            if ($printed -notcontains 'runtime: embedded') {
+              throw "the nova install.ps1 installed does not carry its runtime library: $($printed -join ' / ')"
+            }
+            $after = Read-UserPath
+            if ($after.Kind -ne [Microsoft.Win32.RegistryValueKind]::ExpandString) {
+              throw "after install.ps1, the user's PATH is a $($after.Kind) value, not REG_EXPAND_SZ"
+            }
+            # -ne compares without case.
+            if ($after.Entries.Count -eq 0 -or $after.Entries[0].TrimEnd('\') -ne $bin.TrimEnd('\')) {
+              throw "after install.ps1, the user's PATH does not start with ${bin}: $($after.Text)"
+            }
+            # -cnotcontains compares exactly, so an entry that was expanded
+            # counts as lost.
+            $lost = @($before.Entries | Where-Object { $after.Entries -cnotcontains $_ })
+            if ($lost.Count -gt 0) {
+              throw "install.ps1 dropped or changed entries of the user's PATH: $($lost -join ';')"
+            }
+            Invoke-InstallPs1
+            $again = Read-UserPath
+            $count = @($again.Entries | Where-Object { $_.TrimEnd('\') -eq $bin.TrimEnd('\') }).Count
+            if ($count -ne 1) {
+              throw "after a second run, the user's PATH holds $bin $count times: $($again.Text)"
+            }
+            Write-Host "The user's PATH starts with $bin, keeps its other entries, and still holds it once after a second run."
+          } finally {
+            Stop-Process -Id $server.Id -Force -ErrorAction SilentlyContinue
+          }
       - uses: actions/upload-artifact@v4
         with:
           name: archive-${{ matrix.target }}
````

What it relies on, from `install.ps1` (Task 6) and the job:
- `ARCHIVE` is `dist/nova-<version>-x86_64-pc-windows-msvc.zip`, which
  "Pack the archive" writes to `$GITHUB_ENV`; the version is read from the
  root `Cargo.toml`, as that step reads it.
- `install.ps1` parses `SHA256SUMS` by splitting each line on white space,
  so the line is what `sha256sum` writes: the hash in lower case, two
  spaces, the name.
- `install.ps1` installs into `$env:NOVA_HOME\bin`, made absolute with
  `Get-Item`, and writes that path first in `HKCU\Environment`'s `Path`,
  read with `DoNotExpandEnvironmentNames` and written as `REG_EXPAND_SZ`
  (decisions 22, 23). The step reads the value the same way before and
  after: the directory must come first (compared without case, a trailing
  `\` ignored), every entry that was there must still be there exactly, and
  the value must be `REG_EXPAND_SZ`. It then runs `install.ps1` again, as an
  update does, and the directory must be there once.
- `pwsh` starts `powershell` itself, so Windows PowerShell gets its own
  `PSModulePath` (decision 49); started through another program, it would
  not load its cmdlets.
- `python` (the runner's Python 3) serves `$RUNNER_TEMP\srv` with
  `http.server`, so `http://127.0.0.1:8431/releases/download/v<version>/`
  is `<base>/download/v<version>/` for the base
  `http://127.0.0.1:8431/releases`.
- The `$env:` changes end with the step, so the job's later steps see no
  `NOVA_` variable, and the runner gives each step the environment it
  started with, not the registry's new PATH; `finally` stops the server
  however the step ends; `timeout-minutes: 10` bounds a hang, which the job,
  with no timeout of its own, would otherwise let run for 6 hours.

On the runner it prints `install.ps1`'s lines twice. The first run:
`nova: installing nova 0.2.0 for x86_64-pc-windows-msvc`, two `nova:
downloading http://127.0.0.1:8431/releases/download/v0.2.0/...` lines,
`nova: checked nova-0.2.0-x86_64-pc-windows-msvc.zip against SHA256SUMS`,
`nova: installed <RUNNER_TEMP>\install-home\bin\nova.exe`, the three lines
of `nova version`, and `nova: added <RUNNER_TEMP>\install-home\bin to your
PATH; open a new terminal for other programs to see it`. The second the
same, but its last line is `nova: <RUNNER_TEMP>\install-home\bin is on
your PATH already`. Then the step's `The user's PATH starts with
<RUNNER_TEMP>\install-home\bin, keeps its other entries, and still holds
it once after a second run.` **It first runs in the pull request's CI**
("Build (x86_64-pc-windows-msvc)"), since the pull request changes
`Cargo.toml` and `tools/install/**`.

While planning, the step ran on Linux (`$P/winsim-36a.py`): PowerShell
7.4.6 stood in for `pwsh` and for `powershell`, run as GitHub's `shell:
pwsh` runs a step, `/usr/bin/python` for `python`, `OS=Windows_NT`, a zip
whose stand-in `nova.exe` prints `runtime: embedded`, and a JSON file for
`HKCU\Environment` in both the step and a copy of `install.ps1` (it expands
`%VARIABLES%` unless asked not to, as the registry does). The server, the
`SHA256SUMS` line, the wait, both installs and every check passed, printing
the lines above, and the server was stopped. Five faults put into the
copy's PATH code each failed the step with its own message: the value
written as `REG_SZ`, the user's PATH replaced by the directory alone, the
directory appended instead of put first, the entries read expanded, and the
directory added again on the second run. The draft of this step, which
checked only that the directory was there, passed all five.

- [ ] **Step 3: Lint it, and run it on Linux (optional)**

While planning, the whole edited file was written to `$P/release36a.yml`
and linted there (`$P/lint36a.sh`), with the result below.

Run: `cd /home/user/nova && $AL -oneline .github/workflows/release.yml | sed -E 's/: shellcheck reported issue in this script//' | cut -c1-80`
Expected: exactly five findings, all in prepare's "Checksums, notes, and
the release command" step: four `SC2010` (`ls | grep`) and one `SC2035`.
At `a17f140` it had three `SC2010` and the `SC2035`; the new `scripts=`
count line adds one `SC2010`, written in the style of the lines above it.
The two new steps add none (actionlint does not lint PowerShell).

Optional, with `$P/pwsh-dl`: the new step's script parses, and runs
against the stand-in registry.
Run: `cd /home/user/nova && python3 -I -c 'import sys, yaml; w = yaml.safe_load(open(sys.argv[1])); print([s for s in w["jobs"]["build"]["steps"] if s.get("name") == "Install the archive with install.ps1, PATH on"][0]["run"], end="")' .github/workflows/release.yml > "$P/t7-step.ps1" && PS_FILE="$P/t7-step.ps1" "$P/pwsh-dl/pwsh/pwsh" -NoProfile -NonInteractive -Command '$e = $null; $t = $null; [System.Management.Automation.Language.Parser]::ParseFile($env:PS_FILE, [ref]$t, [ref]$e) | Out-Null; "parse errors: " + $e.Count'`
Expected: `parse errors: 0`.
Run: `cd /home/user/nova && python3 -I "$P/winsim-36a.py" .github/workflows/release.yml tools/install/install.ps1 "$P/winsim-work" "the Windows step"`
Expected: `== the Windows step: exit 0`, then the step's last lines, ending
with `nova: <RUNNER_TEMP>/install-home/bin is on your PATH already` and
`The user's PATH starts with <RUNNER_TEMP>/install-home/bin, keeps its
other entries, and still holds it once after a second run.`; the stand-in
registry's `Path` holding the directory, then the two entries it started
with, `%USERPROFILE%` unexpanded, as `ExpandString`; and `server left
running: none`. Then `rm -rf "$P/winsim-work"`.

- [ ] **Step 4: Commit**

`git diff --stat a17f140 -- Cargo.lock`: no output.
Stage `.github/workflows/release.yml`.
Message: `ci: each release attaches the install scripts, and runs both on its own archives`,
body: prepare copies install.sh and install.ps1 into the assets before the
count and SHA256SUMS, and runs install.sh against the release's Linux
archive over loopback; the Windows build runs install.ps1 in Windows
PowerShell 5.1 against its own zip with PATH on, twice: the installed nova
must carry its runtime, and the user's PATH in the registry must start
with the install directory, keep its other entries as they were and stay
REG_EXPAND_SZ, and hold the directory once after the second run (spec 3.6a
§6.3, amended).

---

### Task 8: The extension (spec §7)

**Files:**
- Create: `tools/vscode-nova/CHANGELOG.md`
- Modify: `tools/vscode-nova/package.json` (`homepage`, `bugs`,
  `keywords`)

**Interfaces:** none. The publisher stays `sakeerin`, which
`vscode_extension.rs` pins; `package-lock.json` needs no change (its root
entry has no homepage or keywords).

- [ ] **Step 1: The CHANGELOG (decision 32)**

`tools/vscode-nova/CHANGELOG.md`, lines 4837-4881 of this plan:

Run: `cd /home/user/nova && sed -n '4837,4881p' "$PLAN" > tools/vscode-nova/CHANGELOG.md`

````markdown
# Changelog

All notable changes to the Nova extension for VS Code are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
The extension's version is always the version of Nova it is released
with.

## [Unreleased]

### Added

- **The Nova language.** `.nova` files get the language `nova`: a
  grammar that colours comments, strings, numbers, attributes, keywords,
  function names and types, and comment toggling, bracket matching,
  auto-closing pairs and indentation.
- **A client for `nova lsp`.** The extension runs the `nova` you have
  installed, as its language server, and gives VS Code what it answers:
  - **diagnostics,** for the whole project on every edit, unsaved
    buffers included, and for `nova.toml`;
  - **completion:** after `.`, a value's fields and methods, std's
    included; elsewhere, the locals, the names in scope, the primitive
    types and the keywords;
  - **formatting,** as `nova fmt` formats, following `.editorconfig`;
  - **hover:** a name's declaration, a local's inferred type, and its
    `///` docs;
  - **go to definition,** into other modules, dependencies and std;
  - **find references** and **rename**, which refuses, saying why, a
    rename that would change what a name means;
  - **quick fixes:** make a local mutable, import a name, "did you mean",
    make an item public, remove an unreachable arm;
  - **organize imports;**
  - **semantic highlighting,** every name coloured by what it means. A
    local or parameter declared `mut` carries the `mutable` modifier,
    which a theme can style.
- **Settings.** `nova.server.path` names the `nova` to run; empty, the
  default, means `nova` from your PATH. `nova.trace.server` (`off`,
  `messages`, `verbose`) traces the messages between VS Code and
  `nova lsp`.
- **Nova: Restart Language Server,** a command that restarts it, for
  example after installing a newer `nova`.
- If `nova` cannot be started, an error names the setting and how to
  install `nova`.

The extension needs VS Code 1.91 or later and an installed `nova`.
````

It has no relative link: vsce rewrites relative links against the
repository's root, which would be wrong for `tools/vscode-nova/`.

- [ ] **Step 2: The listing's fields**

Lines 4894-4922 of this plan:

Run: `cd /home/user/nova && sed -n '4894,4922p' "$PLAN" > "$P/36a-t8-package.diff" && git apply "$P/36a-t8-package.diff"`

````diff
diff --git a/tools/vscode-nova/package.json b/tools/vscode-nova/package.json
index 6f8f572..14c627a 100644
--- a/tools/vscode-nova/package.json
+++ b/tools/vscode-nova/package.json
@@ -9,6 +9,10 @@
     "type": "git",
     "url": "https://github.com/Sakeerin/nova"
   },
+  "homepage": "https://github.com/Sakeerin/nova/tree/main/tools/vscode-nova",
+  "bugs": {
+    "url": "https://github.com/Sakeerin/nova/issues"
+  },
   "engines": {
     "vscode": "^1.91.0"
   },
@@ -16,6 +20,13 @@
     "Programming Languages",
     "Formatters"
   ],
+  "keywords": [
+    "nova",
+    "programming language",
+    "language server",
+    "lsp",
+    "formatter"
+  ],
   "activationEvents": [
     "onLanguage:nova"
   ],
````

- [ ] **Step 3: Run the tests**

Run: `cd /home/user/nova && cargo test --locked -p nova-cli --test release_manifests --test vscode_extension 2>&1 | grep -E "^test result|FAILED"`
Expected: `test result: ok. 7 passed` and `test result: ok. 2 passed`.

Optional, with the npm registry reachable (this is no test, and fetches
packages):
`cd tools/vscode-nova && npm ci && npm run compile && npx vsce package --out "$P/t8-nova.vsix"; cd ../..`,
never `--out /dev/null` (Global Constraints). Expected: 363 files, about
587 KB, with vsce's usual warning that the extension is not bundled;
`unzip -l "$P/t8-nova.vsix" | grep -i changelog` shows
`extension/changelog.md`. Then remove `tools/vscode-nova/node_modules` and
`tools/vscode-nova/out`.

- [ ] **Step 4: Commit**

Stage `tools/vscode-nova/CHANGELOG.md` and `tools/vscode-nova/package.json`.
Message: `vscode-nova: a CHANGELOG, and the listing's homepage, bugs and keywords`,
body: the CHANGELOG the stores show lists what the extension does under
[Unreleased], which 3.6b's release commit names 0.3.0; package.json gains
homepage, bugs and keywords; the publisher stays sakeerin (spec 3.6a §7).

---

### Task 9: The release gate (spec §8)

**Files:**
- Create: `.github/workflows/release-gate.yml`

**Interfaces:** run only by hand, with the input `version`, after the
crates and the GitHub release are up; never in 3.6a. Every step sees
`VERSION`; `gate.sh` gets the installed path through `INSTALLED`, never a
`NOVA_` variable.

- [ ] **Step 1: The workflow**

`.github/workflows/release-gate.yml`, lines 4967-5122 of this plan (decisions
28 to 31 and 36):

Run: `cd /home/user/nova && sed -n '4967,5122p' "$PLAN" > .github/workflows/release-gate.yml`

````yaml
name: Release gate

# The Phase 3 gate, run on fresh runners against what a release published
# (spec docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# §8): `nova` installed from crates.io, and `nova` installed by the
# release's own install script, each on Linux, macOS and Windows. It runs
# only when started from the Actions tab, after the crates and the GitHub
# release of that version are up (docs/releasing.md). It uses no secret and
# publishes nothing. Its log, with the maintainer's own run on a clean
# machine, is the gate's record.
on:
  workflow_dispatch:
    inputs:
      version:
        description: The released version to check, without the v (for example 0.3.0)
        required: true
        type: string

permissions:
  contents: read

env:
  CARGO_TERM_COLOR: always
  VERSION: ${{ inputs.version }}

jobs:
  # `cargo install nova-cli` from crates.io, then the gates CI's `install`
  # job runs on a `cargo install --path` copy.
  crates-io:
    name: From crates.io (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - name: The version is a version
        shell: bash
        run: |
          if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
            echo "::error::'$VERSION' is not a version like 0.3.0 (give it without the v)"
            exit 1
          fi
      # The gate scripts and examples/05-geometry, as the tag has them.
      - uses: actions/checkout@v4
        with:
          ref: v${{ inputs.version }}
      - uses: dtolnay/rust-toolchain@stable
      # No Swatinem/rust-cache here, unlike CI: a restored cache would fill
      # ~/.cargo/registry before the install, and the nested runtime build
      # must resolve offline from what this install itself downloaded
      # (spec §4 and §14, risk 1).
      - name: Install nova from crates.io
        shell: bash
        run: cargo install nova-cli --version "=$VERSION" --locked --root "$RUNNER_TEMP/crates-io"
      - name: Nothing but nova was installed
        shell: bash
        run: |
          set -euo pipefail
          ls -A "$RUNNER_TEMP/crates-io/bin"
          case "$(ls -A "$RUNNER_TEMP/crates-io/bin")" in
            nova|nova.exe) ;;
            *) echo "::error::the install put more than nova into bin/"; exit 1 ;;
          esac
      - name: The installed nova is the version asked for
        shell: bash
        run: |
          set -euo pipefail
          first=$("$RUNNER_TEMP/crates-io/bin/nova" version | head -n 1 | tr -d '\r')
          if [ "$first" != "nova $VERSION" ]; then
            echo "::error::nova version printed '$first', not 'nova $VERSION'"
            exit 1
          fi
      - name: The installed nova makes, runs, builds and tests a project
        shell: bash
        run: bash .github/scripts/gate.sh "$RUNNER_TEMP/crates-io/bin/nova" "$RUNNER_TEMP/gate" --test
      # gate.sh made the project with `nova new demo` in its WORKDIR. The
      # templates are written in nova fmt's layout, so the check finds
      # nothing to change.
      - name: The project nova new made passes nova fmt --check
        shell: bash
        run: |
          set -euo pipefail
          cd "$RUNNER_TEMP/gate/demo"
          "$RUNNER_TEMP/crates-io/bin/nova" fmt --check
      - name: The installed nova builds a program against a path library
        shell: bash
        run: bash .github/scripts/packages-gate.sh "$RUNNER_TEMP/crates-io/bin/nova" "$RUNNER_TEMP/packages"
      - name: The installed nova documents an example that uses a dependency, and runs its doc tests
        shell: bash
        run: bash .github/scripts/doc-gate.sh "$RUNNER_TEMP/crates-io/bin/nova" "$RUNNER_TEMP/doc"

  # The release's install script, from the release's own assets, then
  # gate.sh on the `nova` it installed into ~/.nova/bin.
  install-script:
    name: From the install script (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    steps:
      - name: The version is a version
        shell: bash
        run: |
          if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then
            echo "::error::'$VERSION' is not a version like 0.3.0 (give it without the v)"
            exit 1
          fi
      # The gate scripts, as the tag has them.
      - uses: actions/checkout@v4
        with:
          ref: v${{ inputs.version }}
      # NOVA_VERSION is set for the script alone: gate.sh refuses to run
      # with any NOVA_ variable in its environment.
      - name: Install nova with install.sh
        if: runner.os != 'Windows'
        shell: bash
        run: curl -sSfL "https://github.com/Sakeerin/nova/releases/download/v$VERSION/install.sh" | NOVA_VERSION="$VERSION" sh
      # PowerShell 7, then Windows PowerShell 5.1, which is what `irm | iex`
      # runs in on a Windows that has nothing else; the second install
      # replaces the first, and the gate checks it.
      - name: Install nova with install.ps1 (PowerShell 7)
        if: runner.os == 'Windows'
        shell: pwsh
        run: |
          $env:NOVA_VERSION = $env:VERSION
          irm "https://github.com/Sakeerin/nova/releases/download/v$env:VERSION/install.ps1" | iex
      - name: Install nova with install.ps1 (Windows PowerShell 5.1)
        if: runner.os == 'Windows'
        shell: powershell
        run: |
          $env:NOVA_VERSION = $env:VERSION
          irm "https://github.com/Sakeerin/nova/releases/download/v$env:VERSION/install.ps1" | iex
      - name: Where the script installed nova
        id: installed
        shell: bash
        run: |
          set -euo pipefail
          if [ "$RUNNER_OS" = Windows ]; then
            nova="$USERPROFILE/.nova/bin/nova.exe"
          else
            nova="$HOME/.nova/bin/nova"
          fi
          ls -l "$nova"
          first=$("$nova" version | head -n 1 | tr -d '\r')
          if [ "$first" != "nova $VERSION" ]; then
            echo "::error::nova version printed '$first', not 'nova $VERSION'"
            exit 1
          fi
          echo "nova=$nova" >> "$GITHUB_OUTPUT"
      - name: The installed nova makes, runs, builds and tests a project
        shell: bash
        env:
          INSTALLED: ${{ steps.installed.outputs.nova }}
        run: bash .github/scripts/gate.sh "$INSTALLED" "$RUNNER_TEMP/gate" --test
````

Two jobs on fresh Ubuntu, Windows and macOS runners: "From crates.io (os)"
installs `nova-cli --version =<version> --locked` and runs `gate.sh
--test`, `nova fmt --check` on the project it made, `packages-gate.sh` and
`doc-gate.sh`; "From the install script (os)" runs the release's own
script with `NOVA_VERSION` and `gate.sh --test` on what it installed. macOS
runners are arm64, so `x86_64-apple-darwin` is still run by nothing.

- [ ] **Step 2: Lint it**

Run: `cd /home/user/nova && $AL .github/workflows/release-gate.yml; echo "exit $?"`
Expected: `exit 0`.

Its version check, by hand:

Run: `for v in 0.3.0 0.3.0-rc.1 v0.3.0 0.3 "0.3.0; rm"; do VERSION=$v bash -c 'if [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]]; then echo "$VERSION: a version"; else echo "$VERSION: refused"; fi'; done`
Expected: `0.3.0: a version`, `0.3.0-rc.1: a version`, then `refused` for
`v0.3.0`, `0.3` and `0.3.0; rm`.

- [ ] **Step 3: Commit**

Stage `.github/workflows/release-gate.yml`.
Message: `ci: the release gate, run by hand against what a release published`,
body: from crates.io and from the release's install script, on fresh
Linux, macOS and Windows runners: the Phase 3.0 gate, the new project's
format, and the package and doc gates; install.ps1 under PowerShell 7 and
5.1; no cache, no secret, nothing published (spec 3.6a §8). It first runs
in 3.6b.

---

### Task 10: The runbook (spec §9)

**Files:**
- Create: `docs/releasing.md`

- [ ] **Step 1: The runbook**

`docs/releasing.md`, lines 5167-5716 of this plan:

Run: `cd /home/user/nova && sed -n '5167,5716p' "$PLAN" > docs/releasing.md`

````markdown
# Releasing Nova

The steps of a release, from `v0.3.0` on, in the order they run (spec
`docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md`
§9). A release publishes the same version everywhere at once: the GitHub
release with its archives and install scripts, the 19 crates on crates.io,
and the VS Code extension on the Marketplace and Open VSX. The package
index is set up once, at the first release, whose gate publishes `shapes`
to it.

**[stop]** marks a step that publishes something, touches an account or
cannot be undone. Whoever works through this runbook, a person or an agent
working for the maintainer, waits there for the maintainer's explicit word
before doing it. Everything between two stops is preparation that can be
redone.

Below, `0.3.0` stands for the version being released. The commands set it
once, in a shell variable:

```sh
version=0.3.0
```

The commands are for a POSIX shell (on Windows, Git Bash) on the machine
the maintainer releases from, with `git`, `gh` (logged in to GitHub as
the repository's owner), Rust, Node.js 22 with `npm`, and `perl`.

## Before the first release

Once, before `v0.3.0`; a later release needs none of this again, except
that tokens are made fresh each time. **[stop]** Every item here is the
maintainer's own, on their accounts.

### Accounts

- **crates.io.** Log in at <https://crates.io> with the GitHub account
  that will own the crates, and verify its e-mail address under Account
  Settings: crates.io refuses a publish from an account without one.
- **The VS Code Marketplace.** At
  <https://marketplace.visualstudio.com/manage>, signed in with a
  Microsoft account, create a publisher whose ID is `sakeerin`. It must
  equal `publisher` in `tools/vscode-nova/package.json`, which
  `crates/nova-cli/tests/vscode_extension.rs` pins. Publishing takes an
  Azure DevOps personal access token (step 4), made at
  <https://dev.azure.com> under User settings → Personal access tokens.
- **Open VSX.** Log in at <https://open-vsx.org> with GitHub, link an
  Eclipse Foundation account (made at <https://accounts.eclipse.org> with
  the same GitHub account) under Settings, and sign the Publisher
  Agreement there. Then make an access token under Settings → Access
  Tokens and create the namespace, once:

  ```sh
  npx ovsx create-namespace sakeerin -p <open-vsx-token>
  ```

  `npx` asks to download `ovsx` first; it is not one of the extension's
  dependencies.

### The GitHub environment `crates-io`

The publish workflow runs in this environment, and its token lives there
only for the length of a publish (step 3).

1. In the repository, Settings → Environments → New environment, named
   `crates-io`.
2. Deployment protection rules: tick **Required reviewers** and add the
   maintainer. Leave "Prevent self-review" off, or the maintainer cannot
   approve their own run.
3. Deployment branches and tags: **Selected branches and tags**, then add
   a rule of type Tag with the pattern `v*`. The workflow itself refuses
   anything but the tag `v<version>`; this makes GitHub refuse it too.
4. Add no secret yet.

## 1. The release commit

A pull request whose last commit is the release commit:

1. **The version.** It is set in the root `Cargo.toml` (the workspace's
   `version` and the 19 `=` pins in `[workspace.dependencies]`), in
   `tools/vscode-nova/package.json`, and in
   `tools/vscode-nova/package-lock.json` (the two `version` lines under
   `"name": "nova"`; the lock holds other packages' versions too). From a
   checkout of `main`, on a new branch:

   ```sh
   old=0.2.0   # the version main has now
   perl -pi -e "s/^version = \"\Q$old\E\"\$/version = \"$version\"/; s/version = \"=\Q$old\E\"/version = \"=$version\"/" Cargo.toml
   perl -pi -e "s/^  \"version\": \"\Q$old\E\",\$/  \"version\": \"$version\",/" tools/vscode-nova/package.json
   perl -0pi -e "s/(\"name\": \"nova\",\n\s*\"version\": )\"\Q$old\E\"/\$1\"$version\"/g" tools/vscode-nova/package-lock.json
   cargo update --workspace
   git diff --stat
   ```

   `cargo update --workspace` moves only the workspace's own 23 packages
   in `Cargo.lock` (add `--offline` without a network). The diff shows 20
   lines changed in `Cargo.toml`, 23 in `Cargo.lock`, 1 in `package.json`
   and 2 in `package-lock.json`. `npm version` is not used: it rewrites
   `package.json`'s layout. Then:

   ```sh
   cargo test --locked -p nova-cli --test release_manifests --test vscode_extension
   ```

   holds the pins and the extension to the workspace's version.
2. **`CHANGELOG.md`.** `## [Unreleased]` becomes
   `## [0.3.0] - <the date, YYYY-MM-DD>`, under a lead paragraph that says
   what the release is (for `v0.3.0`: Phase 3 is complete, within the
   boundary its ADR records), and a fresh, empty `## [Unreleased]` goes
   above it. Phase 2's `[0.2.0]` section, from the commit "docs: finalize
   CHANGELOG for the v0.2.0 release", is the model. `release.yml` takes the
   release notes from this section, and fails without it.
3. **`tools/vscode-nova/CHANGELOG.md`,** which the stores show:
   `## [Unreleased]` becomes `## [0.3.0] - <the date>` (for `v0.3.0`, with
   the line "The first published version." under it). If the extension
   did not change since the last release, add a section that says
   "Released with Nova 0.3.0; no change to the extension." The extension's
   version always moves with Nova's.
4. Anything else the release's own spec puts in the release commit (for
   `v0.3.0`: the texts that say how to install `nova`, spec §7).

The pull request runs CI, including "Packaged nova", which packages the
19 crates at the new version and installs `nova` from them, and
`release.yml` without its publish job, which needs step 2's section. Wait
until all of it is green.

**[stop]** Merge it with **Rebase and merge** (`gh pr merge <number>
--rebase`): the repository has no merge commits, and every release tag
points at its CHANGELOG commit. Then wait for CI on `main` to pass on the
merged commits.

## 2. The tag and the GitHub release

Rebasing gave the commits new hashes, so tag the one on `main`:

```sh
git fetch origin main
git log -1 --format='%H %s' origin/main   # the CHANGELOG commit
commit=$(git rev-parse origin/main)
git tag -a "v$version" -m "Nova v$version" "$commit"
```

For a phase's release the message says so, as `v0.2.0`'s did: for
example `-m "Nova v0.3.0 — Phase 3 (Tooling)" -m "Phase 3 is complete,
within the boundary docs/adr/<its ADR> records."`.

**[stop]** Push it:

```sh
git push origin "v$version"
```

The tag starts `release.yml`, which builds the four targets, smoke-tests
three of them, and creates the GitHub release `v0.3.0` (not a draft, and
a prerelease only if the version has a `-`). Follow it with
`gh run watch`, picking the Release run. Then check the release:

```sh
gh release view "v$version" --repo Sakeerin/nova --json isPrerelease,assets --jq '.isPrerelease, .assets[].name'
mkdir "release-$version" && cd "release-$version"
gh release download "v$version" --repo Sakeerin/nova
sha256sum --check SHA256SUMS   # macOS: shasum -a 256 --check SHA256SUMS
curl -sSI https://github.com/Sakeerin/nova/releases/latest | grep -i '^location'
```

There are eight assets and `SHA256SUMS`, and every line of the check says
`OK`:

```
nova-0.3.0-aarch64-apple-darwin.tar.gz
nova-0.3.0-x86_64-apple-darwin.tar.gz
nova-0.3.0-x86_64-pc-windows-msvc.zip
nova-0.3.0-x86_64-unknown-linux-gnu.tar.gz
nova-std-docs-0.3.0.tar.gz
nova-vscode-0.3.0.vsix
install.sh
install.ps1
SHA256SUMS
```

`releases/latest` redirects to `…/releases/tag/v0.3.0`; the install
scripts find the latest version through it. The release's notes are the
CHANGELOG section of step 1.

A GitHub release can be deleted and made again; a crate cannot. That is
why the tag comes before crates.io.

## 3. crates.io

The workflow `.github/workflows/publish-crates.yml`, "Publish to
crates.io", publishes the 19 crates in dependency order with
`.github/scripts/publish-crates.sh` (spec §5). It runs only when started
by hand, only on the tag `v<version>` whose workspace version is
`<version>`, and it first repeats CI's package check
(`.github/scripts/package-gate.sh`).

1. **[stop]** Make a token at crates.io, Account Settings → API Tokens →
   New Token:
   - name: for example `nova v0.3.0`;
   - expiration: a custom date, the next day;
   - scopes: **publish-new** and **publish-update** only;
   - crates: the pattern `nova-*`.
2. **[stop]** Add it as the environment's secret, in Settings →
   Environments → `crates-io` → Environment secrets → Add environment
   secret, named `CARGO_REGISTRY_TOKEN`; or, pasting the token at the
   prompt:

   ```sh
   gh secret set CARGO_REGISTRY_TOKEN --env crates-io --repo Sakeerin/nova
   ```

3. **[stop]** Start the workflow: Actions → "Publish to crates.io" → Run
   workflow, "Use workflow from" the tag `v0.3.0`, version `0.3.0`; or

   ```sh
   gh workflow run publish-crates.yml --repo Sakeerin/nova --ref "v$version" -f version="$version"
   ```

   The run waits for the environment's reviewer: approve it under "Review
   deployments".
4. Wait. The package check takes about as long as CI's install job. The
   upload of 19 new crates then takes about 2 h 20 min: crates.io allows a
   burst of 5 new crates and then one every 10 minutes, and the script
   prints `rate limited: waiting <n> s` and tries again. A later release
   only adds versions to existing crates, which crates.io allows 30 at a
   time and then one a minute, so its upload takes minutes. The job's
   limit is 345 minutes. It ends with one line per crate, published or
   skipped.
5. Check what crates.io serves:

   ```sh
   curl -sS https://index.crates.io/no/va/nova-cli | tail -n 1
   ```

   The last line names `"vers":"0.3.0"`.
6. **Afterwards, whatever the result,** revoke the token (crates.io,
   Account Settings → API Tokens → Revoke) and delete the secret:

   ```sh
   gh secret delete CARGO_REGISTRY_TOKEN --env crates-io --repo Sakeerin/nova
   ```

   A run that stopped can be started again from the same tag with a new
   token (section 7).

## 4. The extension

The extension is published by hand, from the release's own `.vsix`. No
store token goes into GitHub.

```sh
git clone --depth 1 --branch "v$version" https://github.com/Sakeerin/nova "nova-$version"
cd "nova-$version/tools/vscode-nova"
npm ci   # so that npx runs the locked @vscode/vsce
gh release download "v$version" --repo Sakeerin/nova --pattern "nova-vscode-$version.vsix" --pattern SHA256SUMS
grep " nova-vscode-$version.vsix\$" SHA256SUMS | sha256sum --check   # macOS: shasum -a 256 --check
```

1. **[stop]** The Marketplace. Make an Azure DevOps personal access token
   with Organization **All accessible organizations**, Scopes **Custom
   defined → Marketplace → Manage**, and a short expiry. Then:

   ```sh
   read -rs VSCE_PAT && export VSCE_PAT   # paste the token; it is not shown
   npx @vscode/vsce publish --packagePath "nova-vscode-$version.vsix"
   unset VSCE_PAT
   ```

   (`vsce` can also sign in with Microsoft Entra ID instead of a token,
   with `--azure-credential` after `az login`; its documentation says
   which Azure DevOps currently expects.)
2. **[stop]** Open VSX, with a token from open-vsx.org's Settings →
   Access Tokens:

   ```sh
   read -rs OVSX_PAT && export OVSX_PAT
   npx ovsx publish "nova-vscode-$version.vsix"
   unset OVSX_PAT
   ```

   (`-p <token>` in place of `OVSX_PAT` does the same, and leaves the
   token in the shell's history.)
3. Check the listings, which can take a few minutes to appear:
   <https://marketplace.visualstudio.com/items?itemName=sakeerin.nova>
   and <https://open-vsx.org/extension/sakeerin/nova>. Each shows version
   0.3.0, the README, the CHANGELOG, and the repository, homepage and
   issues links. Revoke the Azure DevOps token when done.

## 5. The index (first release only)

`nova` reads the package index at
`https://raw.githubusercontent.com/Sakeerin/nova-index/main/` unless
`NOVA_INDEX` says otherwise, and `nova publish` writes to the repository
its `config.json` names (ADR 0031; spec
`docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md`
§3.2, §7, §11.2).

1. **[stop]** Create the repository, public, with its `config.json` on
   `main`:

   ```sh
   gh repo create Sakeerin/nova-index --public --description "The Nova package index" --clone
   cd nova-index
   printf '%s\n' '{"dl": "https://github.com/Sakeerin/nova-index/releases/download/{name}-{version}/{name}-{version}.nova-pkg", "api": "Sakeerin/nova-index"}' > config.json
   git add config.json
   git commit -m "The index's config"
   git branch -M main
   git push -u origin main
   curl -sS https://raw.githubusercontent.com/Sakeerin/nova-index/main/config.json
   ```

   The last command prints the file back; give it a few minutes if it
   says 404 at first.
2. **[stop]** Make the token `nova publish` uses: github.com, Settings →
   Developer settings → Personal access tokens → Fine-grained tokens →
   Generate new token, with resource owner `Sakeerin`, repository access
   **Only select repositories** → `Sakeerin/nova-index`, repository
   permission **Contents: Read and write** (Metadata: read-only comes with
   it), and a short expiry. It goes into `nova login` on the machine that
   publishes, in step 6.2. `gh auth token | nova login` works too, with
   `gh`'s own, broader token.

The index is append-only: a published version stays, and a name keeps the
case it was first published with.

## 6. The gate

### 6.1 In CI

**[stop]** Once the crates and the GitHub release are up (steps 2 and 3),
start the workflow `.github/workflows/release-gate.yml`: Actions →
"Release gate" → Run workflow (from `main`), version `0.3.0`; or

```sh
gh workflow run release-gate.yml --repo Sakeerin/nova -f version="$version"
```

On fresh Linux, macOS and Windows runners it installs `nova` from
crates.io (`cargo install nova-cli --version =0.3.0 --locked`) and runs
the install gates on it, and installs it with the release's `install.sh`
or `install.ps1` and runs `gate.sh --test` on that. It publishes nothing.
All six jobs must pass. Keep the run's address for the record.

If the crates.io jobs fail, decide quickly: for 72 hours after publishing,
crates.io still lets an owner delete a crate (section 7).

### 6.2 On a clean machine

The maintainer's own run, recorded, on a machine of their choosing: a
fresh virtual machine, or a new user account on an existing one (so
nothing of an earlier `nova`, `~/.nova` or `~/.cargo` is in reach). It
needs:
- Rust, from <https://rustup.rs> (in a new user account too: rustup
  installs per user);
- a C linker, which `nova build` and `nova test` use: `cc` on Linux (for
  example the `build-essential` package), the Xcode Command Line Tools on
  macOS (`xcode-select --install`), the Visual Studio Build Tools with
  "Desktop development with C++" on Windows;
- VS Code;
- `curl` and `tar`, which current Windows has too.

The commands, in order, each with what it should show:

1. Install from crates.io; time it.

   ```sh
   cargo install nova-cli --locked
   nova version
   ```

   `nova 0.3.0`, the target, and `runtime: embedded`.
2. A project:

   ```sh
   nova new hello
   cd hello
   nova run
   ```

   `Hello, Nova!`.
3. **Autocomplete.** Install the extension from the store: in VS Code's
   Extensions view, search for Nova and install the one by `sakeerin`, or
   `code --install-extension sakeerin.nova`. Open the folder (`code .`),
   open `src/main.nova`, and replace it with this, cramped on purpose for
   step 4:

   ```nova
   record Point{x:Int,y:Int}

   fn main() {
       let p = Point{x:3,y:4}
       println("${p.x + p.y}")
   }
   ```

   Then, inside `main`, type `p.` on a new line: completion offers `x` and
   `y`. Take a screenshot, then delete the line again and save.
4. Format:

   ```sh
   nova fmt --check
   nova fmt
   nova fmt --check
   nova run
   ```

   The first check prints `would reformat: …main.nova` and exits 1;
   `nova fmt` rewrites it as `record Point { x: Int, y: Int }` and
   `Point { x: 3, y: 4 }`; the second check prints nothing and exits 0;
   `nova run` prints `7`.
5. **[stop]** Publish `shapes`, the library of `examples/05-geometry`, to
   the real index. This is permanent.

   ```sh
   cd ..
   curl -sSfL "https://github.com/Sakeerin/nova/archive/refs/tags/v$version.tar.gz" | tar xz
   cp -R nova-*/examples/05-geometry/shapes shapes
   cd shapes
   read -rs TOKEN                                  # the token of step 5
   printf %s "$TOKEN" | nova login
   curl -sS -H "Authorization: Bearer $TOKEN" https://api.github.com/rate_limit | grep -A 4 '"core"'
   nova package
   nova publish
   curl -sS -H "Authorization: Bearer $TOKEN" https://api.github.com/rate_limit | grep -A 4 '"core"'
   unset TOKEN
   curl -sS https://api.github.com/repos/Sakeerin/nova-index | grep '"size"'
   ```

   `nova login` says it is logged in to `Sakeerin/nova-index`; `nova
   publish` prints `published shapes 0.1.0 to Sakeerin/nova-index; other
   machines may take up to five minutes to see it`. The difference between
   the two `"used"` counts is the API requests the publish made, and
   `"size"` the repository's size in KB; ADR 0031 waits for both figures.
   In PowerShell 7, `Read-Host -MaskInput | nova login` replaces the
   `read` and `printf` lines.
6. A second project that depends on it, from the index:

   ```sh
   cd ..
   nova new measure
   cd measure
   nova add shapes
   cp ../nova-*/examples/05-geometry/src/main.nova src/main.nova
   nova run
   ```

   `nova add` prints `added shapes = "0.1.0" to [dependencies]` (if it
   says the index has no `shapes`, wait five minutes and try again).
   `nova run` prints:

   ```
   area 3.141592653589793, perimeter 6.283185307179586
   area 6, perimeter 10
   total area 9.141592653589793
   ```

7. Log out: delete `~/.nova/credentials.toml`
   (`%USERPROFILE%\.nova\credentials.toml`), and revoke the token unless
   it is kept for later publishes.

### 6.3 The transcript

Keep the run as a transcript, from the terminal's own output, with the
screenshot of step 3. Before it goes into the records, remove the home
directory (write `~`), user and machine names, IP addresses, and anything
a token could be read from; never paste a token. 3.6b's records take it:
the gate's record in the ADR for Phase 3's boundary, and the index's
figures in ADR 0031.

```text
Release:        v0.3.0
Date:           YYYY-MM-DD
Machine:        <OS and version>, <architecture>; <fresh VM image | new user account>
Rust:           <rustc --version>
VS Code:        <version>; extension sakeerin.nova <version>, from <Marketplace | Open VSX>
Release gate:   <the Actions run's address>, <all six jobs passed?>

1. cargo install nova-cli --locked     <duration>
   nova version                        <its three lines>
2. nova new hello; nova run            <output>
3. completion after `p.`               <the items offered>   [screenshot]
4. nova fmt --check; nova fmt; nova fmt --check; nova run
                                       <exit codes, output>
5. nova login                          <output>
   nova publish                        <output, with its sha256>
   API requests used by the publish    <after - before>
   nova-index size                     <KB>
6. nova add shapes                     <output>
   nova run                            <output>
Anything that went differently:        <notes>
```

## 7. If something fails

**A Release run on the tag fails** (step 2). If the failure passes (a
runner, the network), re-run the run's failed jobs (`gh run rerun
<run-id> --failed`): the release is made once they pass.

**An archive is wrong, or the Release run needs a change to pass** (step
2). Do this before step 3, while nothing else is published. **[stop]**
Delete the GitHub release, if one was made, and the tag:

```sh
gh release delete "v$version" --repo Sakeerin/nova --yes
git push origin --delete "v$version"
git tag -d "v$version"
```

Fix it on `main` in a pull request, and release the next patch version
(`0.3.1`) from step 1. A tag that was pushed is not moved to another
commit.

**A crate is rejected in the middle of the publish** (step 3). The script
stops at the first failure with cargo's output; the crates before it are
on crates.io for good, at that version.
- A passing failure (the network, crates.io's limit outlasting the
  retries, the job's time): **[stop]** start the workflow again on the
  same tag, with a fresh token if the first expired. Crates already on
  crates.io at that version are skipped (`skip <crate> <version> (already
  on crates.io)`), and the run continues where it stopped.
- A failure that needs a change (a manifest, a name taken since, a check
  crates.io makes that the package gate does not): fix it on `main`, then
  release the next patch version from step 1. Every crate moves to it,
  even those already published, because the pins between them are exact;
  nothing builds a mix. The half-published version's crates stay, with
  no `nova-cli` of that version to use them, since it is published last;
  yank them if wanted (below).

**Yank or delete.** `cargo yank --version 0.3.0 <crate>` (a token with
the **yank** scope) hides a version from new resolution, keeps it for
lockfiles that already name it, and can be undone with `--undo`; it is
the normal way to withdraw a version. Deleting removes a whole crate: an
owner can do it within 72 hours of publishing (crates.io → the crate →
Settings), and after that only under conditions the 19, which depend on
each other, do not meet (`docs/phase-3-plan.md` §6, risk 5). Delete only
when a publish must be undone entirely, for example a file that should
never have been in a tarball.

**The extension** (step 4). A refused token: make a new one. A refused
name or listing: the `.vsix` is the release's, so a change to it is a new
patch version. A version once published to a store cannot be published
there again.

**The index** (step 5, 6.2). `nova publish` commits by appending the
index line last, so a publish that failed can be run again and finishes.
A published version stays: there is no yank in the index.

**The release gate or the clean-machine run fails** (step 6). Record what
failed. **[stop]** The maintainer decides: within 72 hours of the
crates.io publish the crates can still be deleted (above); otherwise, or
as well, fix it and release the next patch version.
````

- [ ] **Step 2: What it must say of the final scripts (decision 42)**

Lines 5726-5755 of this plan:

Run: `cd /home/user/nova && sed -n '5726,5755p' "$PLAN" > "$P/36a-t10-fixes.diff" && git apply "$P/36a-t10-fixes.diff"`

````diff
diff --git a/docs/releasing.md b/docs/releasing.md
index 3a5c188..2baac65 100644
--- a/docs/releasing.md
+++ b/docs/releasing.md
@@ -120,8 +120,8 @@ A pull request whose last commit is the release commit:
 
 The pull request runs CI, including "Packaged nova", which packages the
 19 crates at the new version and installs `nova` from them, and
-`release.yml` without its publish job, which needs step 2's section. Wait
-until all of it is green.
+`release.yml` without its publish job, whose release notes come from
+item 2's section. Wait until all of it is green.
 
 **[stop]** Merge it with **Rebase and merge** (`gh pr merge <number>
 --rebase`): the repository has no merge commits, and every release tag
@@ -150,9 +150,11 @@ git push origin "v$version"
 ```
 
 The tag starts `release.yml`, which builds the four targets, smoke-tests
-three of them, and creates the GitHub release `v0.3.0` (not a draft, and
-a prerelease only if the version has a `-`). Follow it with
-`gh run watch`, picking the Release run. Then check the release:
+three of them, installs the Windows archive with `install.ps1` (PATH on)
+and the Linux one with `install.sh`, and creates the GitHub release
+`v0.3.0` (not a draft, and a prerelease only if the version has a `-`).
+Follow it with `gh run watch`, picking the Release run. Then check the
+release:
 
 ```sh
 gh release view "v$version" --repo Sakeerin/nova --json isPrerelease,assets --jq '.isPrerelease, .assets[].name'
````

- [ ] **Step 3: Check it against what is built**

- The asset list in its step 2 (eight and `SHA256SUMS`) is what
  `release.yml`'s count check expects (4, 1, 1 and 2).
- `grep -n -E 'publish-crates\.(yml|sh)|release-gate\.yml|package-gate\.sh|releases/latest|install\.(sh|ps1)' docs/releasing.md`:
  every file it names exists, and each workflow's input is `version`.
- Its bump commands change 20 lines of `Cargo.toml` (the version and the
  19 pins), 1 of `package.json` and 2 of `package-lock.json`, which D
  checked on a scratch copy; `npm version` is not used, since it rewrites
  `package.json`'s layout.
- `grep -c '\*\*\[stop\]\*\*' docs/releasing.md` counts the stops (16);
  each publishes something, touches an account, or cannot be undone.
- `grep -n -i -E '/home/|/tmp/|scratchpad|claude-0' docs/releasing.md`: no
  output. The account `Sakeerin` and the publisher `sakeerin` are the
  repository's own.

What it says of the stores, Azure DevOps, Open VSX, the token pages and
GitHub's labels is from documentation, not tried (decision 34); 3.6b
checks it as it goes.

- [ ] **Step 4: Commit**

Stage `docs/releasing.md`.
Message: `docs: the release runbook`,
body: every step of a release from v0.3.0 on, in order, each publish
marked [stop] for the maintainer's word: the release commit, the tag and
the GitHub release, crates.io, the extension, the index, the gate in CI
and on a clean machine with its transcript, and what to do when something
fails (spec 3.6a §9).

---

### Task 11: The records (spec §10)

**Files:**
- Create: `docs/adr/0036-publishing-nova.md`
- Modify: `docs/phase-3-plan.md` (§4's 3.6 entry, §2 item 8),
  `nova-spec/40-TOOLING.md` (§10), `nova-spec/50-TESTING.md` (§4.3),
  `docs/adr/0027-runtime-and-std-in-an-installed-nova.md`, `CHANGELOG.md`

- [ ] **Step 1: ADR 0036, "Publishing nova"**

Write it in the form of `docs/adr/0035-doc-tests.md`: Status (Accepted,
2026-10-12, Phase 3.6a, branch `claude/loving-archimedes-frtf5q`, the
spec's path), Context, Decision, Consequences. Context: what 3.6 needs
(plan §4's 3.6 entry), the blockers of plan §2 item 8, ADR 0027's "first
runs for real at 3.6", and the user's seven decisions of 2026-10-12. Its
Decision records, each in a line or two, from the spec and from the
ledger's rulings, never from this plan's expectations alone:
- the 19 crates and the four left out (`publish = false`), and that the
  names were free on 2026-10-12;
- the workspace version; exact pins in `[workspace.dependencies]`, and
  why; path dev-dependencies, which cargo strips; `Cargo.lock` unchanged;
- the metadata: repository and homepage, descriptions, the shared README,
  both licence texts as copies;
- the package gate, and what it cannot prove: crates.io serves the real
  dependencies, the nested runtime build's offline resolution from
  `~/.cargo/registry` first runs for real at 3.6b's gate, and
  verification builds in the dev profile, which never embeds;
- the publish workflow: by hand, only on the tag `v<version>` at that
  version with the secret set, the package gate first, the `crates-io`
  environment, 345 minutes; the token: `publish-new` and `publish-update`,
  `nova-*`, a day, deleted after;
- the publish script: skip what the index lists (yanked included), the
  rate-limit match, 30 attempts, 660 s, the summary, resuming from the tag;
- the install scripts: their URLs
  (`https://github.com/Sakeerin/nova/releases/latest/download/install.sh`
  and `install.ps1`), `$NOVA_HOME/bin`, the `SHA256SUMS` check, PATH
  (`install.ps1` through the registry, first, `REG_EXPAND_SZ`;
  `install.sh` prints the line), ASCII, the environment variables;
  `release.yml` attaches both and runs both on its own archives;
- the templates in `nova fmt`'s layout (the amendment);
- the extension by hand, from the release's `.vsix`, under `sakeerin`;
- the gate's two halves: `release-gate.yml` and the maintainer's run from
  `docs/releasing.md` §6.2.

It names the spec and ADRs 0026 (no `self update`), 0027 (the package
route) and 0031 (the index's figures, which 3.6b's run fills). Its
Consequences record real figures: the package gate on Linux, from this
plan's Task 2 run as the ledger has it, else the prototype's, named as
such (236 s: fetch 0 s, packaging and verifying 106 s, the
install 129 s, the gate 1 s; 19 tarballs of 1,436,578 bytes, the largest
`nova-runtime`'s 252,631 against crates.io's 10 MB; an installed `nova` of
24,997,376 bytes carrying a 7,525,643-byte runtime; about 1.5 GB of work
directory); the publish test's 9 cases in about 0.9 s and the install
test's 8 in about 23 s on Linux; the 19 new crates' upload of about 2 h
20 min at crates.io's limit (a burst of 5, then one every 10 minutes);
the 72 hours after which a published crate stays; what first runs in CI
(the package route on Windows and macOS, `install.ps1` under Windows
PowerShell 5.1, which the test starts without the `PSModulePath` CI's
PowerShell 7 would pass on, and its PATH code, bash 3.2); and what 3.6a
leaves out (spec §15).

- [ ] **Step 2: Dated notes**

Each begins `**Amended 2026-10-12 (branch `claude/loving-archimedes-frtf5q`):**`,
as the files' earlier notes do:
- `docs/phase-3-plan.md` §4's 3.6 entry: 3.6 is two sub-phases (the user,
  2026-10-12); 3.6a prepared everything and published nothing (the spec,
  ADR 0036), its tests passing on Linux here and the pull request's CI
  running all three systems; 3.6b publishes, runs the gate and tags.
- `docs/phase-3-plan.md` §2 item 8, after its 2026-10-07 note: 3.6a
  removes the other two blockers: the dependencies between workspace
  crates carry exact versions, through `[workspace.dependencies]`, and
  `repository` is `https://github.com/Sakeerin/nova`; the 19 package and
  verify in CI.
- `nova-spec/40-TOOLING.md` §10, after its last note: the scripts are
  release assets at
  `https://github.com/Sakeerin/nova/releases/latest/download/install.sh`
  and `install.ps1` (there is no `novalang.dev`); they install into
  `$NOVA_HOME/bin`, `~/.nova/bin` by default, after checking `SHA256SUMS`;
  `install.ps1` adds the directory to the user's PATH and `install.sh`
  prints the line; `NOVA_VERSION` and `NOVA_INSTALL_BASE`; usable from
  `v0.3.0`, the first release that carries them; §10.3's `self update`
  and `self uninstall` stay out (ADR 0026): run the script again to
  update, delete `~/.nova` to uninstall.
- `nova-spec/50-TESTING.md` §4.3: as built (3.0's `release.yml` and
  3.6a): four targets (x86_64-unknown-linux-gnu, x86_64-apple-darwin
  cross-built and never run, aarch64-apple-darwin,
  x86_64-pc-windows-msvc), no linux-aarch64; the install scripts are
  attached, covered by `SHA256SUMS`, and run on the release's own archives;
  std's site is a release asset (3.5a), and there is no `novalang.dev`.
- `docs/adr/0027-runtime-and-std-in-an-installed-nova.md`, at "The package
  route first runs for real at 3.6's crates.io install": since 3.6a, CI's
  "Packaged nova (os)" runs it, emulated, on every pull request, on three
  systems (ADR 0036); its first real run is 3.6b's gate.

- [ ] **Step 3: CHANGELOG**

`CHANGELOG.md` `[Unreleased]`, after 3.5b's entries, as 3.5b's follow
3.5a's:
- Added: the install scripts (`install.sh`, `install.ps1`, as release
  assets, checked against `SHA256SUMS`); the "Publish to crates.io" and
  "Release gate" workflows, run by hand; the package check, "Packaged nova
  (os)", in CI; the crates' READMEs and licence texts; the extension's
  CHANGELOG; the release runbook, `docs/releasing.md`.
- Changed: the crates' metadata (the real repository and homepage,
  descriptions, nova-cli's keywords and categories); `publish = false` on
  `nova-bench-http`, `nova-bundler`, `nova-codegen-wasm` and `nova-test`;
  one workspace version, with exact pins between the crates; the templates
  `nova new` and `nova init` write are in `nova fmt`'s layout (decision
  46).

The root README's install section, the extension's README and its error
text change in 3.6b's release commit (spec decision 21).

- [ ] **Step 4: The sweep**

Run: `cd /home/user/nova && git grep -n -i -E "novalang/nova|install\.sh|install\.ps1|crates\.io" -- docs nova-spec README.md ARCHITECTURE.md agent.md | grep -v -E '^docs/superpowers/' | wc -l`,
then the same without `wc -l`. Every hit must agree with what is built, be
a dated record of its own time, be covered by Step 2's notes, or be one of
decision 21's texts that 3.6b changes (the root README). Ledger the count
before (23 at `a17f140`) and after, and anything noticed outside these files and not
changed (for example `nova-spec/00-MASTER-SPEC.md`'s placeholder table,
which names itself a placeholder).

- [ ] **Step 5: Check and commit**

- `cargo run -q -p nova-cli -- fmt --check std examples`: no output.
- Every path the records cite exists (`ls` each).
- The diff holds no `/home/`, `/tmp/`, `scratchpad` or personal name.
- Stage the six files.
- Message: `docs: records for Phase 3.6a: ADR 0036, notes, CHANGELOG`,
  with a body in the style of 3.5b's records commit.

---

### Task 12: Final verification, mutants, and the PR body

**Files:** none in the repository; `$P/pr-body-36a.md`.

- [ ] **Step 1: Everything, on Linux**

Each to a file in `$P`, reading the tails:
- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- the 1.78 check, then `msrv_test` on the three new test files (and
  `install_scripts.rs` again with `-- --cfg windows`)
- `cargo run -q -p nova-cli -- fmt --check std examples`
- `$SC` on `package-gate.sh` and `publish-crates.sh`, `$SC -s sh` on
  `install.sh`; `$AL` on the four workflows (Task 7's five findings in
  `release.yml`, none elsewhere)
- `git diff --stat a17f140 -- Cargo.lock` (no output)
- `cargo test --workspace --locked --all-features --no-fail-fast 2>&1 | tee $P/full-final-36a.txt`, then the counting one-liner:
  `grep -E '^test result:' $P/full-final-36a.txt | awk '{p+=$4; f+=$6; i+=$8} END {print p" passed, "f" failed, "i" ignored"}'`

Expected: all clean, and `2017 passed, 1 failed, 10 ignored`: Task 0's
1992 and the 25 new tests (7 + 9 + 8 + 1), computed, not yet measured; the
one failure is the environmental IPv6 test. Then compare test by test:

Run: `key() { grep -o -E 'test .+ \.\.\. (ok|FAILED|ignored)' "$1" | LC_ALL=C sort; }; diff <(key "$P/fixfinal-full.txt") <(key "$P/full-final-36a.txt")`
Expected: 25 lines, each `> test <name> ... ok`: the seven of
`release_manifests`, the nine of `publish_script`, the eight of
`install_scripts` and `every_template_passes_fmt_check_as_written`. No `<`
line: no test missing or changed outcome. (The key is not anchored, as
two tests print before their result on the same line.) Ledger the counts
beside Task 0's.

- [ ] **Step 2: The package gate on the finished tree**

Task 2 Step 2's command, with `$P/pg36a-final` and `$P/home-pg36a-final`.
Expected: as Task 2's, but `nova-cli`'s line shows 43 files (its two new
test files), so its size and the total are larger. Ledger the figures
beside Task 2's, then remove both directories.

- [ ] **Step 3: The spec's mutants (§11.5)**

Each on committed work, undone with `git checkout -- <file>` and a clean
`git status --short` and `git diff --stat a17f140 -- Cargo.lock` before the
next, each failing its named test. Ledger each with the test that failed.
1. A pin set to `0.2.1`:
   `perl -pi -e 's/^(nova-hir = .*version = "=)0\.2\.0"/${1}0.2.1"/' Cargo.toml`.
   `cargo test --locked -p nova-cli --test release_manifests` refuses
   first ("failed to select a version for the requirement `nova-hir =
   "=0.2.1"`"); the already built test binary still runs, since `cargo
   metadata --no-deps` does not resolve:
   `"$(ls -t target/debug/deps/release_manifests-* | grep -v '\.d$' | head -n 1)"`
   → `the_workspace_pins_each_published_crate_exactly` ("nova-hir's pin")
   and `every_dependency_between_members_is_an_exact_pin_on_the_workspace_version`
   ("nova-codegen-cranelift's dependency on nova-hir").
2. `publish = false` dropped from nova-test:
   `perl -ni -e 'print unless /^publish = false$/' crates/nova-test/Cargo.toml`
   → four tests: `exactly_the_four_crates_nova_does_not_need_are_unpublished`,
   `the_publish_list_is_the_published_crates_in_dependency_order`,
   `the_workspace_pins_each_published_crate_exactly` and
   `every_published_crate_carries_a_readme_both_licences_and_its_links`
   ("nova-test sets no readme").
3. Lexer before diagnostics in the list:
   `perl -0pi -e 's/  nova-diagnostics\n(.*?)  nova-lexer\n/  nova-lexer\n$1  nova-diagnostics\n/s' .github/scripts/publish-crates.sh`
   → `the_publish_list_is_the_published_crates_in_dependency_order`
   ("nova-lexer is listed before its dependency nova-diagnostics").
4. The "already published" check removed:
   `perl -0pi -e 's/  if \[ "\$listed" = yes \]; then\n.*?\n  fi\n//s' .github/scripts/publish-crates.sh`
   → `skips_a_crate_the_index_lists_at_the_version_and_publishes_one_listed_at_others`.
5. Every failure retried, not just the rate limit:
   `perl -0pi -e 's/    if ! grep -qiE .*?\n    fi\n//s' .github/scripts/publish-crates.sh`
   → `stops_at_any_other_failure_with_cargos_output_and_publishes_nothing_after_it`
   (the failing crate runs 30 times) and `never_prints_the_token` (31
   runs, not 3).
6. `install.sh`'s checksum comparison removed:
   `perl -ni -e 'print unless /^  \[ "\$actual" = "\$expected" \]/' tools/install/install.sh`
   → `a_checksum_mismatch_installs_nothing` alone.
7. `install.ps1` ignoring `NOVA_VERSION` (Windows only; decision 44):
   `perl -pe 's/^(\s*)\$version = \$env:NOVA_VERSION$/$1\$version = \$null/' tools/install/install.ps1 > "$P/install-mut7.ps1"`
   (a copy; the repository's file is untouched), then
   `bash "$P/mut7-36a.sh" "$P/install-mut7.ps1" "$P/mut7-work" | tail -n 4`.
   Expected: `nova: error: could not download
   http://127.0.0.1:18937/releases/download/v1.2.3/nova-1.2.3-x86_64-pc-windows-msvc.zip:
   ... 404 (Not Found).`, `install.ps1 exited with 1`, and the requests
   `GET /releases/latest` and the v1.2.3 archive: the Windows test
   `nova_version_fetches_that_version_and_never_asks_for_latest` would fail
   at its success assertion and its request list. In CI terms, on a
   throwaway branch with the user's word: the same one-line change, pushed,
   fails "Test (windows-latest)" in `install_scripts`; then delete the
   branch.
8. A licence copy edited:
   `perl -pi -e 's/Permission is hereby granted/Permission is granted/' crates/nova-pm/LICENSE-MIT`
   → `every_published_crate_carries_a_readme_both_licences_and_its_links`
   (".../crates/nova-pm/LICENSE-MIT differs from the root's").
9. A template back on three lines:
   `perl -0pi -e 's/    "fn greeting\(\) -> String \{ \\"Hello, Nova!\\" \}\\n",\n/    "fn greeting() -> String {\\n",\n    "    \\"Hello, Nova!\\"\\n",\n    "}\\n",\n/' crates/nova-cli/src/template.rs`
   → `every_template_passes_fmt_check_as_written` (`program` and
   `init-program` would be reformatted), and the tests that compare
   `TEMPLATE_MAIN`. Task 5's Step 2 was this mutant in effect.

Mutants 1, 2, 3 and 8 were run on the prototype (report A), 4 and 5 (B),
6 and 7 (C, 7 with PowerShell 7); mutant 9 is new with the amendment.

- [ ] **Step 4: The PR body**

Write `$P/pr-body-36a.md`, with no local path and no personal name, under
the repository's PR template headings (`## Summary`, `## Changes`,
`## Testing`, `## Phase Gate Criteria`), as PR #110 was, then
`## Final review`, `## Decisions to review` and `## Deferred minors`:
1. Summary: what 3.6a makes ready, and that nothing is published; the
   spec, the plan and ADR 0036; 3.6b, in outline (spec §16).
2. Changes: by area (the manifests; the package gate; the publish script
   and workflow; the install scripts and `release.yml`; the extension; the
   release gate; the runbook); the templates, the one Rust change outside
   tests, as a spec amendment; dependencies (no new package, `Cargo.lock`
   unchanged).
3. Testing: the template's checklist, then Linux (local) against `main` at
   the branch base, Windows and macOS "from this PR's CI"; new tests by
   file; the package gate's figures; each of the spec's nine mutants
   failing its named test (7 as decision 44 says); what runs first in this
   PR's CI ("Packaged nova" on Windows and macOS, `install.ps1`'s test
   cases, `release.yml`'s Windows PATH step, the publish script under bash
   3.2).
4. Phase Gate Criteria: spec §13, each with its test or job.
5. Final review: "To be filled in after the review."
6. Decisions to review: the spec's decisions (1 to 25), this plan's
   decisions, and every ledger `Ruling` with its cost if wrong, in
   collapsed lists.
7. Deferred minors: the install job's now redundant `cargo package -p
   nova-std` (decision 10); C's improvements not made (Rosetta picking the
   arm64 archive, running the staged binary before it replaces the old
   one, a glibc or musl check); `x86_64-apple-darwin` run by nothing; the
   TLS 1.2 prefix for `irm` on old Windows; `curl --retry` in the publish
   script; the runbook's store and token steps to check at 3.6b
   (decision 34); and any from the final review.
8. The session's attribution lines for pull requests.

Run: `grep -n -i -E "/home/|/tmp/|claude-0|scratchpad" "$P/pr-body-36a.md"`
Expected: no output.
Run: `grep -o -i -E "[[:alnum:]./:-]*sakeerin[[:alnum:]./-]*" "$P/pr-body-36a.md" | sort -u`
Expected: only the repository's URLs and names (`github.com/Sakeerin/nova...`,
`Sakeerin/nova-index`) and the publisher (`sakeerin`, `sakeerin.nova`)
(decision 45).

There is no commit in this task. The branch is pushed and the PR opened
only after the final review and its fix pass, as the workflow says.
