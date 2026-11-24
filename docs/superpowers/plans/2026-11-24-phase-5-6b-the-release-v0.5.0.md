# Phase 5.6b, "the release `v0.5.0`", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** `v0.5.0` is released by the runbook, `docs/releasing.md`, as
`v0.4.0` was. One pull request whose last commit is the release commit,
`release: v0.5.0, Phase 5 released`: the workspace's version 0.4.0 → 0.5.0
in four files (the root `Cargo.toml`'s `version` and 20 exact pins, the
extension's `package.json` and `package-lock.json`, and `Cargo.lock`'s 23
`version` lines of the workspace's own packages, nothing else in it);
`CHANGELOG.md`'s `[Unreleased]` becomes `[0.5.0]`, dated with the commit,
under the lead the spec drafts ("Phase 5, "Self-hosting", is released"; N
lives in `selfhost/` and is not shipped; S1 == S2 == S3 in `main`'s CI run
45142347493; the boundary in ADR 0061, to come), with a fresh, empty
`[Unreleased]` above it and 5.2b in the list of sub-phases; the extension's
CHANGELOG gains a `[0.5.0]` section with no change to the extension; the
README a `### Self-hosting` subsection; the runbook two dated notes (the
token takes publish-update alone; the maintainer's try of the published
`nova` replaces step 6.2). No code, test, workflow or crate changes: the
existing tests hold the versions, and are shown red with the manifests at
0.5.0 and the extension at 0.4.0, then green. After the merge, each step
on the user's word: the tag (the user's), `release.yml` and the GitHub
release, the 20 crates on crates.io (the user's token), the extension in
both stores (the user's uploads), the Release gate, and the user's try of
the published `nova`. The records pull request, with ADR 0061, follows
them and is not in this plan.

**Architecture:**
- **The release commit** (Task 1), in three patch blocks applied in
  order and committed once (decision 13): `t1-versions` (`Cargo.toml`,
  `Cargo.lock`), then the red; `t1-extension`
  (`tools/vscode-nova/package.json`, `package-lock.json`), then the green;
  `t1-records` (`CHANGELOG.md`, `README.md`, `docs/releasing.md`,
  `tools/vscode-nova/CHANGELOG.md`); then `redate.sh` gives the records
  and the message the commit's date (decision 15).
- **Verification** (Task 2): the suite, `selfhost.rs`, the gate on both
  runtimes, the golden suite, the checks, the package list, 14 mutants of
  the release commit's files, the review and one fix pass, the pull
  request and its CI, and the merge.
- **After the merge** (Task 3): the runbook's steps 2 to 6, the user's
  steps and the main session's writes each a stop for the user's word,
  each step with what to check: the main session's calls to its GitHub
  tools, or the user's own commands (decision 24).

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021. **No Rust file changes.**
- **No new crate; `Cargo.lock` changes in its 23 workspace `version`
  lines alone** (decision 8). Anything else in the lock stops
  the work and goes to the user.
- `release.yml`, `publish-crates.yml` and `release-gate.yml` run as they
  are; nothing under `.github/` changes.

**Spec:**
`docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`,
approved by the user on 2026-11-24 as drafted (`3f72bd38`), and its dated
amendment of 2026-11-24, which this plan's commit appends (decision 22).
- Read its header, §1 to §13, and the amendment, before Task 1.
- The user's decisions are decisions 1 to 10 here, the approval decision
  11, and the answer that allowed the prototype's push to CI decision 12.

**Base:** `3f72bd38` ("docs: the Phase 5.6b spec is approved"), `main`
`ed165975` (5.6a's merge, #149) and the spec's commit, whose full suite
gives `2837 1 10` here (the IPv6 test the only failure) and `selfhost.rs`
71 passed and 3 ignored. The branch is
`claude/loving-archimedes-frtf5q-56b`, from `3f72bd38`, then this plan's
commit (decision 22).

**Checked on 2026-11-24, before execution:**
- **A prototype made the release commit whole,** `4f55d6ed`, one commit
  on `3f72bd38`: the runbook's commands (`docs/releasing.md:126-135`)
  with `version=0.5.0` and `cargo update --workspace --offline` (offline,
  "Locking 23 packages"), then the records, the spec's drafts copied by
  script. It is on `origin` as the head of draft Sakeerin/nova#150,
  closed unmerged, whose ref (`refs/pull/150/head`) keeps `4f55d6ed` and
  `3f72bd38` under it whatever becomes of the branch it was pushed from.
- **CI, measured from the prototype** (decision 12), on #150: CI run
  45316548924, 19 of 19
  jobs green, and Release run 45316548687, its seven jobs green and
  "Publish the release" skipped, as on a pull request.
  - **Release:** "Prepare the release" found `## [0.5.0]`, packed
    `SHA256SUMS` over 8 files and printed `publish runs: gh release create
    v0.5.0 --verify-tag --title "Nova 0.5.0" --notes-file notes.md
    dist/*`; the extension packed as `nova-vscode-0.5.0.vsix`; the four
    archives `nova-0.5.0-<target>`.
  - **Packaged nova:** `package-gate: 20 crates at 0.5.0` on three
    systems; `nova-cli-0.5.0.crate` 1,759 files, as at 0.4.0.
  - **The gate:** S1 == S2 == S3 at 9,539,848 bytes each, N's `.ll`
    12,760,111 bytes, on the runtime `0.5.0-4f8a6f1b`; the step 26.66 s.
  - **`selfhost.rs`:** `71 0 3` in 53.90 s on Linux, `69 0 3` in 80.55 s
    on macOS, `65 0 3` in 73.06 s on Windows, under the 80, 100 and 110 s
    lines.
  - **The Test job:** `2838 0 10`, `2838 0 9`, `2815 0 9`;
    `release_manifests` 8, `vscode_extension` 3, `project` 25. MSRV
    (1.78) passed.
  - **`main` at `ed165975`,** run 45142347493, the baseline at 0.4.0:
    green, 19 of 19; `selfhost.rs` 55.57, 31.18 and 108.49 s; the same
    counts.
- **The split** (decisions 13 and 14): a script made a local branch
  from `3f72bd38` with one commit, the release commit with the
  prototype's message; its tree is `4f55d6ed`'s (`git diff` between the
  two prints nothing). Its three patch
  blocks are the prototype's diff over their files, and each applies with
  plain `git apply`, in order, on `3f72bd38`, with no output, each applied
  tree the expected one (`3f72bd38`'s with that block's files and the
  earlier blocks' from the prototype); no added line holds trailing
  whitespace, a CR or a backslash-u escape. **The split's commit is a
  local reference, not a commit the executor makes** or pushes, and this
  plan names it by `4f55d6ed`, which has its tree and is on `origin`:
  Task 1's commit has that tree, and its hash, dated by the executor, is
  its own. The
  runbook's commands, run again on a scratch copy of `3f72bd38`, gave the
  four version files byte for byte as `t1-versions` and `t1-extension`
  leave them.
- **The replay,** under the process limit, each step as written here:
  - **at `3f72bd38`, the baseline** (Task 0): `nova 0.4.0`; the suite in
    three runs `1300 1 8`, `2 0 0` and `1535 0 2` in 108, 1 and 182 s,
    `2837 1 10` summed, with `release_manifests` 8, `vscode_extension` 3
    and `project` 25 in the third; 2,848 tests listed, 74 in
    `selfhost.rs`; `SH` `71 passed; 0 failed; 3 ignored` in 171.23 s;
    `release-notes.sh 0.5.0` exits 1;
  - **Task 1 on the working tree at `3f72bd38`:** `t1-versions`, then the
    red: `release_manifests` 7 passed and 1 failed, `vscode_extension` 2
    and 1, each for the extension's version; `t1-extension`, then the
    green, 8 and 3; `t1-records`, then the tree equal to
    `4f55d6ed`'s; `redate.sh` run with another day and back, and
    `specblocks.py`;
  - **at the split's tip, `4f55d6ed`'s tree** (Task 2): `cargo build` with nothing
    to compile and `nova 0.5.0`; `release-notes.sh 0.5.0` exit 0, 614
    lines, 40,673 bytes, 40,612 characters, the lead first; the suite
    `1300 1 8`, `2 0 0` and `1535 0 2` in 118, 4 and 211 s, `2837 1 10`,
    and the list the base's, 111 binaries and 2,848 tests, net 0; `SH`
    `71 0 3` in 174.58 s; the gate on the debug runtime `1 passed` in
    51.52 s; the gate as CI's step runs it (`NOVA_EMBED_RUNTIME=1`,
    `opt-level = 1`) `1 passed` in 39.00 s, on the runtime
    `0.5.0-04a13feb` this machine builds; the golden suite `147 passed, 9
    excluded`; `cargo fmt`, both clippy commands on the local stable
    (1.97) and both `nova fmt --check` lines clean; `nova-cli`'s package
    list 1,759 files;
  - **the move between the two:** after the replay, `3f72bd38` and the
    tip again each built with nothing to compile, the plain build, the
    tests and `selfhost.rs`'s: both build sets are in `target/` (decision
    18).
- **The mutants** (decision 19): 14 breakages of the release commit's
  files, run one at a time, in one run, at the split's tip (Task 2, Step
  7), in 38 s, all 14 caught: 7 by cargo, the version tests and
  `release-notes.sh` (a pin, the workspace's version, the lock, the
  extension's three version lines, the CHANGELOG's heading), and the 7
  records by `R`, `specblocks.py` and the `[Unreleased]` grep (the lead,
  5.2b, the extension's section, the README's subsection and the
  runbook's two notes by the first, the fresh `[Unreleased]` by the
  second). The runner finds the commit's day in the tree. Three earlier
  forms of it, the first with the day written in, the second with other
  titles, the third without `R`, gave the same results at the same tip
  for the 7 the tests catch, and left the 7 records uncaught; the results
  here are the plan's form's.
- **Another day** (decision 15): on a commit of the split's tree dated
  2026-11-25 by `redate.sh` (detached, on no branch, then left), Task 1,
  Step 7's checks printed `dated 2026-11-25` and, against the split's
  tip (`4f55d6ed`'s tree), the eight dated lines alone; the mutants of
  the runner's form of then, without `R`, ran 14 of 14 as expected; and
  `specblocks.py` passed with that day on a re-dated copy of the tree.
- **Not run here, each with its reason** (decision 17): the Rust 1.100
  clippy pair, the mimicked Windows clippy and the Rust 1.78 check: no
  Rust file changes, and CI's clippy on three systems and its MSRV job
  passed on `4f55d6ed`, whose tree is the release commit's but for its
  date.
- **The GitHub tools Task 3 uses,** read on `v0.4.0` (reads only;
  nothing was started or written): `get_tag` gives the annotated tag
  object, its message, and its `object`, `type` `commit`, the release
  commit `eab7da8e`; `get_release_by_tag` gives the release's `name`,
  `draft`, `prerelease`, `body`, `html_url`, and each asset's `name` and
  `digest` (`sha256:` and the hex sum). `release-gate.yml` and
  `publish-crates.yml` each take one input, `version`, required.

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. No Rust file
  changes in 5.6b, so nothing new meets the MSRV; CI's MSRV job (`cargo
  check --locked --workspace` under 1.78, `RUSTFLAGS=-D warnings`) checks
  the bumped lock on the pull request, as it passed on `4f55d6ed` (run
  45316548924, the check 37.93 s; decision 17).
- **No new crate; `Cargo.lock` changes in its 23 workspace `version`
  lines alone,** 0.4.0 → 0.5.0, as `cargo update --workspace` moves them
  (decision 8). The four third-party packages at 0.4.0
  (`difflib`, `rand_xorshift`, `winapi-i686-pc-windows-gnu`,
  `winapi-x86_64-pc-windows-gnu`) stay. **Any other change to the lock
  stops the task:** ask the user; do not change the lock first. Its
  sha256 is `a3009b6850e56f7b…` at the base and `f47897873c81b571…`
  after `t1-versions`, to the end.
- **No code, test, workflow or manifest change but the versions.**
  `crates/nova-cli/Cargo.toml` and every member's `Cargo.toml` stay
  (`version.workspace = true`; decision 10); `selfhost/nova.toml`
  stays at 0.1.0, which nothing reads.
- **Tests never touch the internet,** and the shells set
  `CARGO_NET_OFFLINE=true` (Conventions), as the replay ran: no `cargo`
  command here needs the network. Task 3's checks read GitHub through the
  session's GitHub tools (decision 24); crates.io's index and the stores
  are read by the user, at their own prompt or in a browser. No test runs
  there. The agent runs no `gh` and no `curl` anywhere in this plan: the
  runbook's commands that use them are the user's, at their own prompt.
- **Temporary names are fixed.** 5.6b adds none: the gate's
  `nova-selfhost-bootstrap`, the golden walk's `nova-selfhost-home` and
  the suite's own stay as they are.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and the suite's runs leave `/tmp/nova-todo-*`
  (about 90 MiB a run, read in the replay).
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets and
  `S` sets here.
- **`release_golden_suite` runs only under `NOVA_REQUIRE_CLANG=1`,** with
  `NOVA_CLANG=clang-18`, as `G` and CI's Linux leg run it: 147 passed, 9
  excluded, unchanged.
- **`selfhost.rs` is not in `cargo test --workspace`:** its target says
  `test = false`. `S` runs the whole suite as the Test job does, `SH`
  `selfhost.rs` as the Selfhost job's main step does, and `C` the ignored
  tests it names, as the clang steps do.
- **`NOVA_EMBED_RUNTIME` and `CARGO_PROFILE_DEV_OPT_LEVEL` are set for one
  step only** (Task 2, Step 5), which ends with the plain build again.
  Never build every test target at `opt-level = 1`: only `selfhost.rs`'s,
  as the Selfhost job does.
- **`RUST_BACKTRACE` is `0`** in every shell (Conventions).
- **The process limit.** Every test, suite, gate and mutant runs under
  the limit of the Conventions.
- **Work in one tree.** Never build in another worktree, and never run two
  suites, or a suite and `SH`, at once: the tests' fixed directories are
  shared.
- **Keep `df -h /` above 6G (6,144 MiB), and check it before each build,
  not after.** Read the thresholds in MiB: `df -h` rounds up, so its
  `7.0G` can be 7,071 MiB, under 7 GiB (7,168 MiB). Every `df` this plan
  asks for is logged, by this block, its `echo` naming the task, the step
  and what comes next; the steps call it "the disk check":

  ```sh
  mkdir -p "$P/exec56b/logs"
  { echo "Task 2 Step 5, before the embedded build"; df -h /; df -m / | tail -1; } >> "$P/exec56b/logs/df.log"
  tail -n 1 "$P/exec56b/logs/df.log"    # its fourth field: the MiB free, read against the threshold
  ```

  - **Before each change of opt-level or runtime** (Task 2, Step 5, and
    back): at least 7,168 MiB.
  - **Before any other build or suite run:** at least 6,451 MiB (6.3G).
  - **No Rust 1.100 clippy pair is run** (decision 17), so its 6,656 MiB
    threshold does not arise.
  - **Both build sets stay in `target/`** (decision 18): the 0.4.0 set,
    which Task 0 uses, and the 0.5.0 set, which Task 1 and Task 2 use,
    the latter with its `opt-level = 1` embedded build. A move between
    `3f72bd38` and the release commit rebuilds nothing for the plain
    builds; the embedded build recompiles `nova-cli` alone, in place
    (decision 18). **No executable in `target/` is removed before the
    merge;** never `cargo clean`. Removing the 0.4.0 set's stale
    executables waits until after the merge (Task 2, Step 11), and only
    by literal paths or `"${VAR:?}"` forms.
  - **If a check reads under its threshold,** stop and report to the main
    session: nothing may be removed before the merge to make room.
  - **The replay's readings** (logged): 8,170 MiB at the base's start;
    8,081 after its suite; 8,095 at the tip's start; 8,007 after its
    suite, and before and after the embedded build and the gate; 7,989
    after the clippy commands, and 7,988 before the mutants, which wrote
    nothing to `target/`.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code
  in braces. None of 5.6b's patches or scripts holds one. Apply the
  patches with `patch_block`, from the committed plan.
- **No patch adds a CR, a NUL or trailing whitespace.** The one non-ASCII
  character the patches add is `§` (`t1-records`).
- **Nothing writes into the repository but the patches and `redate.sh`.**
  `git status --short` is empty after every run, the gate's and the
  mutants' included.
- **Push and open the pull request on the user's word only.** No push of
  this branch is allowed yet. The user's answer of 2026-11-24, "อนุญาต
  push -56b-ci + draft PR (แนะนำ)", was for the prototype on
  `claude/loving-archimedes-frtf5q-56b-ci` with draft Sakeerin/nova#150,
  measurement only, closed unmerged (decision 12); it does not cover this
  branch. Task 2, Step 10 stops before any push, and the main session asks
  the user. **After the push, nothing is amended or force-pushed without
  the user's word** (decision 20): a change CI or a review asks for then
  stops the work, and the main session asks the user.
- **Merge by rebase, on the user's word only,** pinned to the head.
  Never enable auto-merge.
- **The tag is the user's** (decision 7; spec §2): this
  session's credential was refused with HTTP 403 for `v0.3.0`. The agent
  never makes or pushes a tag, never makes a token or a secret, never
  approves a deployment and never uploads to a store; it starts the
  workflows, each on the user's word, and checks what each step made
  (Task 3; decision 24). In Task 3 the agent is the main session itself,
  not a subagent, and works through its GitHub tools; what they cannot do
  is the user's command, run at the user's own prompt, its output pasted
  back.
- **No secret is read, printed or stored by the agent.** The crates.io
  token and the store tokens are the user's, typed by the user into GitHub
  or their own terminal.
- **This machine is Linux.** macOS and Windows results come from CI.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Every shell exports it, `export P=…` with the
  directory's path, before a command that names `$P`. Exported, not
  merely set: the mutant runner reads it from the environment.
- **The executor's directory:** `$P/exec56b/`. Every script this plan
  writes (`redate.sh`, `specblocks.py`, `names.py`, `mutrun.py`), every
  output and every log goes there: the suite's logs, the disk checks'
  `logs/df.log`, the extracted patches, the commit message, `$NOVA_HOME`,
  and the mutants' `mut/`. Two files are kept outside it: the ledger,
  `$P/ledger-56b.md`, and the env file, `$P/env56b-exec.sh`.
- **The ledger:** `$P/ledger-56b.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures, and each
  of the user's words in Task 2 and Task 3, in their words.
- **The env file:** `$P/env56b-exec.sh`, sourced at the start of every
  tool call (`source "$P/env56b-exec.sh"`): each tool call is a fresh
  shell, so a variable a step sets is set again in each call (5.5b's
  ruling R5.3). Its text, with `P` the scratch directory's path:

  ```sh
  # Phase 5.6b execution shell setup: the plan's Conventions (root, cgroup v1), with PLAN the committed plan.
  P=/path/to/the/scratch/directory; export P; cd /path/to/the/repository
  CG=/sys/fs/cgroup/pids/nova-limit; mkdir -p "$CG" && echo 600 > "$CG/pids.max" && echo $$ > "$CG/cgroup.procs"
  grep -q nova-limit /proc/self/cgroup || { echo "NOT IN nova-limit: stop"; return 1 2>/dev/null || exit 1; }
  mkdir -p "$P/exec56b"
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 CARGO_NET_OFFLINE=true NOVA_HOME="$P/exec56b/home"
  unset NOVA_RUNTIME_LIB NOVA_REQUIRE_NODE NOVA_REQUIRE_CLANG NOVA_CLANG NOVA_EMBED_RUNTIME CARGO_PROFILE_DEV_OPT_LEVEL
  mkdir -p "$NOVA_HOME"
  SH() { cargo test --locked --workspace --test selfhost -- "$@"; }
  S() { NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast "$@"; }
  G() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test run_tests -- release_golden_suite --nocapture "$@"; }
  C() { NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --test selfhost -- --ignored --exact "$@"; }
  V() { cargo test --locked --workspace --no-fail-fast --test release_manifests --test vscode_extension "$@"; }
  PLAN=docs/superpowers/plans/2026-11-24-phase-5-6b-the-release-v0.5.0.md
  block() {
    awk -v name="$1" '
      $0 == "<!-- patch " name " -->" { grab = 1; next }
      grab && /^````diff$/ { inside = 1; next }
      inside && /^````$/ { exit }
      inside { print }
    ' "$PLAN" > "$P/exec56b/$1.patch"
  }
  patch_block() {
    block "$1" && git apply --check "$P/exec56b/$1.patch" && git apply "$P/exec56b/$1.patch"
  }
  ```

  So `SH` runs `selfhost.rs` as the Selfhost job's main step does, `S`
  the whole suite without `selfhost.rs`, as the Test job runs it, `G` the
  golden suite through LLVM as CI's Linux leg runs it, `C` the ignored
  tests it names, with `clang-18` required, and `V` the two tests that
  hold the versions, the runbook's check (`docs/releasing.md:143-145`) in
  the workspace's form, both binaries run whatever the first gives
  (decision 14). `NOVA_HOME` keeps a run by hand off `~/.nova`; every test
  that needs one sets its own. `CARGO_NET_OFFLINE=true` keeps every
  `cargo` command off the network, as the replay ran (Global
  Constraints).
- **The process limit.** Every shell that runs a test or a mutant first
  caps the tasks it may start at 600, threads as well as processes, and
  checks that the cap holds: the env file joins the pids control group
  `nova-limit` (root, cgroup v1, as the replay ran), and stops if the shell
  is not in it. The 5.6a plan's Conventions
  (`docs/superpowers/plans/2026-11-22-phase-5-6a-the-bootstrap-gate.md`)
  give the forms for cgroup v2 and for a user other than root. If a test
  fails to start a thread, read the group's `pids.peak`, raise `pids.max`,
  and record it.
- **Commits.** Task 1's message is written to `$P/exec56b/msg-t1.txt` by
  its here-document (Task 1, Step 5): the subject, a body, a blank line,
  and these two trailer lines, exactly:

  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
  ```

  Then `git add -A`; `git status --short` lists only the task's files;
  and `git commit -F "$P/exec56b/msg-t1.txt"`. Every commit of this plan
  ends with those two lines: the plan's own (Task 0) and the release
  commit (Task 1), into which the review's fixes are folded (decision 20).
  The pull request's body ends as Task 2, Step 10 gives it. No model ID (a
  `claude-…` model name) goes in any artifact: no commit, file, pull
  request, comment or ledger line names one.
- **Every `cargo` command selects the whole workspace,** `--workspace`
  with `--test NAME` or `--lib` to narrow it, never `-p`: each `-p`
  selection builds and links its own copy of every test executable it
  needs, which the disk does not have room for. The exceptions are the
  clippy commands and `cargo fmt --all`, written as CI writes them, and
  `cargo package --list -p nova-cli` (Task 0, Step 3 and Task 2, Step 6),
  which builds nothing.
- **Timeouts, and nothing in the background.**
  - **A tool's timeout is in milliseconds.** Give every command a tool
    timeout of at least its stated time plus half again, never under
    120000 (120 s) and at most 600000 (600 s), the most a foreground
    command may take: for example 400000 for `S`'s third part (211 s
    here), 300000 for `SH` (175 s).
  - Run every command in the foreground: never with `&`, `nohup` or a
    tool's background mode, and never let a tool send one to the
    background. A block of several commands runs one command at a time,
    each with its own timeout. Long runs write their output to a log under
    `$P/exec56b/`, and only its summary is read.
  - Each step that runs `S`, `SH` or `C` builds the tests first, `cargo
    test --locked --workspace --no-run` and `cargo test --locked
    --workspace --test selfhost --no-run`, each a command of its own.
  - **`S` in three runs,** everywhere, as the replay ran it:

    ```sh
    S --lib --bins > "$P/exec56b/s-a.txt" 2>&1                       # the libraries' and binaries' tests
    S --doc > "$P/exec56b/s-b.txt" 2>&1                              # the doc tests
    S --test '[!s]*' --test 's[!e]*' > "$P/exec56b/s-c.txt" 2>&1     # every integration test but selfhost.rs
    cat "$P/exec56b/s-a.txt" "$P/exec56b/s-b.txt" "$P/exec56b/s-c.txt" | grep '^test result' | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
    ```

    They gave `1300 1 8` (the IPv6 test the failure), `2 0 0` and `1535 0
    2`, `2837 1 10` summed, at the base and at the tip. The globs name
    every integration test but `selfhost`, and only integration tests: a
    `--test` pattern takes a `test = false` target, which `--tests`
    leaves out, and `--tests` would run the libraries' tests again. The
    replay checked it: the third run's log holds 67 `Running tests/`
    lines and no `Running unittests` line, the first 23 `Running
    unittests` lines, the second 21 `Doc-tests`: 111 binaries, CI's 111
    `test result:` lines. `grep -c 'Running tests/selfhost.rs'` on the
    third prints `0`. A difference from the sum is recorded in the ledger.
  - **A child killed at `CHILD_LIMIT`.** The harness kills a child process
    past 120 s. If a test fails with `… did not finish within 120s:
    killed`, run that test alone once, `SH --exact <test>` (or `C` for an
    ignored one). If it is killed again, stop and report.
  - If a command is killed by its timeout, or the shell restarts, restore
    what it left: after a cut-off mutant run, `git checkout -- <file>` of
    the mutant's file, then `git status --short` must print nothing, then
    `mut/` is moved aside and every mutant runs again from the first (Task
    2, Step 7). After a half-applied patch in Task 1: `git checkout -- .`
    back to HEAD, `git status --short` empty, and the task again from
    Step 1.
- **CI's runs: waiting for them and reading them** (Task 2, Steps 10
  and 11; Task 3). The main session never waits with a foreground
  `sleep`.
  - **Waiting.** For the pull request (Task 2, Step 10), the main
    session subscribes to the pull request's activity
    (`subscribe_pr_activity`) once it is open, and CI's results wake the
    session. For a workflow run in Task 3, it arms one scheduled
    check-in in this session (`send_later`) for about the run's expected
    time, which each step gives with its source; when it fires, it reads
    the run with `actions_get` (`method: get_workflow_run`) and, if the
    run's `status` is not `completed`, arms another. Or the user says
    when the run is done.
  - **Reading the logs.** The main session does not pull whole job logs
    into its own context. A read-only helper agent fetches each job log
    a check needs with `get_job_logs` into `$P/exec56b/ci/`, and writes
    a summary, `$P/exec56b/ci-<run>.txt`, `<run>` the run's id: each
    job's conclusion, the `test result:` counts summed for each Test
    job, the gate's `bootstrap:` lines, and the step lines the step's
    Expected names. It writes nothing to GitHub. A check that reads a
    log is made against that summary.
- **The build.** Each "Run" step starts with `cargo build --locked
  --workspace`, which ends with `Finished`, so that the `nova` the steps
  run by hand and the runtime library beside it are the tree's.
- **The patches.** Task 1's three blocks below are each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  prototype's diff, so apply them; do not retype them. `patch_block
  t1-versions` (the env file defines it, with `PLAN` the committed plan)
  writes the block to `$P/exec56b/t1-versions.patch` and applies it with
  plain `git apply`, which prints nothing: no 5.6b patch adds a line with
  trailing whitespace, a CR or a backslash-u escape. A block that does not
  apply means the tree differs from the plan's: stop, and record why in
  the ledger.
- **The scripts.** `redate.sh`, `specblocks.py`, `names.py` and
  `mutrun.py` are written by here-documents at column 0, each in its own
  fenced block, to `$P/exec56b/`: copy each block as it is, or run it as
  it stands; never retype it.
- **Each "checks" step** runs:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy,
    `cargo clippy --locked --all-targets --all-features -- -D warnings`, as
    CI runs it, which ends with `Finished` and no warning;
  - `selfhost.rs`'s clippy,
    `cargo clippy --locked --all-features --test selfhost -- -D warnings`,
    CI's second command, which ends with `Finished` and no warning;
  - `target/debug/nova fmt --check std examples selfhost`, CI's
    arguments, and `target/debug/nova fmt --check std examples tests/wasm
    tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr
    selfhost`, the earlier plans' superset, with the `nova` the step's
    build made, each printing nothing.

  Not the Rust 1.100 clippy pair, nor the mimicked Windows clippy
  (decision 17). A difference from an `Expected:` is fixed in the smallest
  way and recorded in the ledger as a ruling, with its cost if wrong.

## Review Focus

The inputs most likely to bite a user, most likely first. Each has its
check in the task that owns it:

1. **`Cargo.lock` changes in its 23 workspace `version` lines alone**
   (decision 8): → Task 1, Step 1 reads the lock's diff line
   by line and names its 23 packages against `cargo metadata`; Task 2,
   Step 2 again over the whole diff (mutant L1).
2. **The version is 0.5.0 in all four files** (`[workspace.package]`, the
   20 pins, `package.json`, the two lines of `package-lock.json`, the 23
   lock lines): → the red and the green of Task 1, Steps 2 and 4; `nova
   version`; "Packaged nova" on CI (mutants K1, K2, L1, E1 to E3).
3. **`release.yml` finds the notes** (`## [0.5.0]`, `release.yml:326`): →
   `release-notes.sh` in Task 1, Step 5 and Task 2, Step 3; "Prepare the
   release" on the pull request (mutant C1).
4. **The records say what the spec drafts** (the lead, 5.2b in the list,
   the fresh `[Unreleased]`, the extension's section, the README's
   subsection, the runbook's two notes): → `specblocks.py` and the
   `[Unreleased]` grep (Task 1, Step 5, and again after the fold, Task 2,
   Step 9), and the review (Task 2, Step 9). No test reads them (spec
   §6), but those two checks do: mutants C3, C4, X1, R1, D1 and D2 are
   caught by `specblocks.py`, and C2 by the grep (Task 2, Step 7's `R`).
5. **The commit's date is in exactly its four places and the message,**
   and no 2026-11-24 is left when it is another day: → `redate.sh` (Task
   1, Step 5), the commit's date checked after it (Step 7).
6. **The bootstrap holds on the new runtime** (its crc changes with the
   runtime crate's version): → the gate on the debug runtime and as CI's
   step runs it (Task 2, Steps 4 and 5); the gate on CI (Task 2, Step
   10). Sizes and the runtime's directory are recorded, not expected
   (decision 21).
7. **CI's counts do not move** (spec §4): → Task 2, Step 10's Expected,
   from run 45316548924 and `main`'s 45142347493.
8. **Each of the user's steps after the merge, and each write, waits
   for the user's word, and every step is checked** (decision 7): →
   Task 3's **[stop]** steps, and each step's checks by the runbook's
   lines.

## Decisions

The user's, from the spec's header and §12 (Phase 5.6's of 2026-11-22,
5.6b's of 2026-11-23, each the recommended option):

1. **5.6 is three pull requests** (5.6's decision 1, as given: 5.6a the
   gate in CI, the tolerances ADR and a `selfhost/` README; then the
   release `v0.5.0`, the version bump; then the records, the boundary ADR
   closing Phase 5, after the release). This plan is the second's.
2. **The release does not ship `selfhost/`** (5.6's decision 8, as given:
   the archive unchanged; the release notes and the README say N lives in
   `selfhost/` and the bootstrap passes the gate). The lead and the
   README's subsection (Task 1).
3. **The lead, "released" as `v0.3.0`'s** (5.6b's decision 1, as given:
   "Phase 5, Self-hosting, is released"; N in `selfhost/`, not shipped;
   the bootstrap passes the gate in CI (ADR 0060); the boundary will be
   recorded in `docs/adr/0061-phase-5-boundary.md`. The tag message says
   the same). The spec's §3.2 draft, copied (`specblocks.py`); the tag
   message of spec §3.7 (Task 3, Step 2).
4. **`main`'s run, no sizes** (5.6b's decision 2, as given: S1 == S2 ==
   S3 byte for byte in `main`'s CI run 45142347493, no sizes; the
   release's own runs go to the boundary ADR). The lead cites run
   45142347493 and no size; run 45316548924's figures stay out of the
   release commit.
5. **The README's `### Self-hosting`** (5.6b's decision 3, as given:
   after `### Web`: N in `selfhost/`, not shipped, S1 == S2 == S3 in CI,
   links to `selfhost/README.md` and ADR 0060).
6. **The extension 0.5.0, in both stores** (5.6b's decision 4, as given:
   its CHANGELOG "Released with Nova 0.5.0; no change to the extension.";
   both stores after the tag, the user uploads). Task 1 and Task 3, Step
   7.
7. **Every publish, as for `v0.4.0`** (5.6b's decision 5, as given: tag
   (the user pushes it) → `release.yml` → crates.io, all 20 crates (a
   publish-update token suffices: no new crate) → Marketplace and Open
   VSX (the user uploads) → the release gate, with step 6.2 replaced by
   the user trying the published `nova` (as ADR 0047 decision 6). Each
   step still on the user's word when it comes). Task 3.
8. **`Cargo.lock`: these 23 lines** (5.6b's decision 6, as given: exactly
   the 23 workspace version lines 0.4.0 → 0.5.0 from `cargo update
   --workspace`; anything else changing stops and asks). Global
   Constraints; Task 1, Step 1.
9. **The records pull request after the publishes and the try** (5.6b's
   decision 7, as given: like #127; ADR 0061 records every release run,
   the user's check, Phase 5's inventory and backlog, in one PR). Not in
   this plan; Task 3, Step 10 names what it takes.
10. **`nova-cli`'s crate as it is** (5.6b's decision 8, as given: no
    manifest change; `tests/selfhost*` stay in the crate). 1,759 files
    (Task 0, Step 3; Task 2, Step 6).

The user's answers of 2026-11-24:

11. **The spec, approved as drafted.**
    - **The user's answer, as given:** "อนุมัติตามร่าง (แนะนำ)", approved
      as drafted. Committed as `3f72bd38` on this branch.
12. **CI measured from the prototype.**
    - **The user's answer, as given:** asked after the prototype whether
      to push it to `claude/loving-archimedes-frtf5q-56b-ci` with a draft
      PR to measure CI, closed unmerged, the user answered "อนุญาต push
      -56b-ci + draft PR (แนะนำ)".
    - **What happened, the facts:** `4f55d6ed` went to that branch with
      draft Sakeerin/nova#150. CI run 45316548924 passed all 19 jobs, and
      Release run 45316548687 its seven, "Publish the release" skipped
      ("Checked on" gives the figures). #150 was closed unmerged.
    - **In this plan:** they are Task 2, Step 10's Expected (decision 21),
      and the spec's amendment holds them (decision 23). The release
      commit carries none of them (decision 4).

How this plan is cut and checked:

13. **Three patch blocks, one commit.** `t1-versions` (`Cargo.toml`,
    `Cargo.lock`), `t1-extension` (`package.json`, `package-lock.json`)
    and `t1-records` (the CHANGELOG, the README, the runbook, the
    extension's CHANGELOG) are each the prototype's diff over its files,
    applied in order, each whole, by plain `git apply`, then one commit,
    as `eab7da8e` (spec §8 and its decision 14). Chosen over one patch applied in
    parts, which would need `git apply --include=<pattern>` for each part:
    an `--include` whose pattern matches no file of the patch applies
    nothing and prints nothing (checked here: `git apply --check
    --include='Cargo.tom'` on `t1-versions` exits 0, silent), so a
    mistyped part would pass unseen, while a whole block that does not
    apply fails loudly. Each block is a
    step with its own check (the red, the green, the records), and the
    split's script checked each applied tree.
14. **The red and the green with the existing tests** **(differs)**. No
    test is added (spec §6 and its decision 19): `release_manifests.rs:301`
    (`the_extension_has_the_workspace_version_and_its_name`) and
    `vscode_extension.rs:76-85`
    (`the_extension_has_nova_clis_version_and_its_floor`) hold the
    extension to the workspace's version. With `t1-versions` alone
    applied, both fail, each for `package.json`'s 0.4.0 (Task 1, Step 2);
    with `t1-extension`, both pass, 8 and 3 (Step 4). The command is `V`
    (Conventions), `cargo test --locked --workspace --no-fail-fast --test
    release_manifests --test vscode_extension`, which differs from the
    runbook's check (`docs/releasing.md:144`) and spec §3.1 in two ways:
    `--workspace` for `-p nova-cli` (Conventions: a `-p` selection builds
    its own copy of every test executable), and `--no-fail-fast`, without
    which cargo stops after `release_manifests`' failure and never runs
    `vscode_extension` (the replay: `error: test failed, to rerun pass
    `-p nova-cli --test release_manifests``, and no `vscode_extension`
    line). On the green tree both forms pass alike.
15. **The date: the commit's day, by script.** The patches date the
    records 2026-11-24, the prototype's day, in exactly four places: the
    two `## [0.5.0] - 2026-11-24` headings (`CHANGELOG.md`,
    `tools/vscode-nova/CHANGELOG.md`) and the runbook's two `**Amended
    2026-11-24 (branch `claude/loving-archimedes-frtf5q-56b`):**` notes;
    and the message once (`[0.5.0] - 2026-11-24`). Spec §3.2, §3.3, §3.5
    and its decision 10 give them the release commit's date, as
    `eab7da8e`'s 2026-10-23. So Task 1, Step 5 runs `redate.sh "$(date -u
    +%F)" "$P/exec56b/msg-t1.txt"` before the commit, whatever the day:
    it rewrites those five places whatever date they hold, leaves the
    spec's file name (which holds 2026-11-24 too) alone, and counts: each
    place holds the day, and on another day no 2026-11-24 is left in
    those files or the message but in the spec's file name. The day is
    UTC's, as the commit's own date is (`eab7da8e`: 2026-10-23 15:14
    +0000). Step 7 checks the commit's date against it. **A commit made
    is re-dated one way,** whether the day turned between Step 5 and Step
    7 or the review's fixes are folded in on another day (decision 20):
    `redate.sh` with the new day, which rewrites the records and the
    message file; `specblocks.py` and the grep with that day; `git add
    -A`; then `git commit --amend --date=now -F "$P/exec56b/msg-t1.txt"`,
    so that the records, the message and the author date all carry the
    new day, and Step 7's date check passes. A fold on the commit's own
    day is `git commit --amend --no-edit`, which keeps the date and the
    message, both already that day. The replay ran `redate.sh` with
    2026-11-25 (4 lines and the message changed, `2026-11-24 outside the
    spec's file name: 0`), then with 2026-11-24 again (the tree the
    split's, the message the prototype's), and refused `26-11-25` and
    `2026-13-01`; `git commit --amend --date=now -F` was checked to give
    the commit that moment as its author date and the file's message.
16. **The cut** **(differs)**: spec §8's three tasks, as this plan makes
    them:
    - **§8's Task 1, the release commit,** is Task 1, with the red and
      the green inside it (decision 14).
    - **§8's Task 2, the measurements, is no task:** CI was measured from
      the prototype before this plan (decision 12), and the local figures
      are measured again at the release commit (Task 2).
    - **§8's Task 3, verification,** is Task 2, which also holds the
      mutants, the pull request, its CI and the merge, as 5.6a's last task
      did.
    - **Task 0,** the baseline at 0.4.0, and **Task 3,** spec §5's steps
      after the merge, are added: each of §5's steps that is the user's
      or writes to GitHub is a **[stop]** for the user's word (decision
      7), the main session's reads and checks need none, and the plan
      gives each step its checks from the runbook (decision 24).
17. **Not run: the Rust 1.100 clippy pair, the mimicked Windows clippy,
    and the Rust 1.78 check.** 5.6b changes no Rust file. The release
    commit changes 8 files: 4 that carry the version, the manifests
    `Cargo.toml` and `tools/vscode-nova/package.json` and the lockfiles
    `Cargo.lock` and `tools/vscode-nova/package-lock.json`, and 4 Markdown
    records, `CHANGELOG.md`, `README.md`, `docs/releasing.md` and
    `tools/vscode-nova/CHANGELOG.md` (`git diff --stat 3f72bd38 4f55d6ed`:
    8 files, 96 lines added, 49 removed). The pull request adds 2 more
    Markdown files, this plan and the spec with its amendment (`3f72bd38`
    and the plan's commit): 10 files in all. Each check lints or
    checks the same Rust as at the base, under a version string that no
    lint reads. CI's clippy, on its stable on Linux, macOS and Windows,
    and its MSRV job under 1.78 passed on `4f55d6ed` (run 45316548924),
    whose tree is the release commit's but for its date (Task 1, Step 7
    compares them),
    and the pull request runs them again; the local stable's pair runs in
    each checks step. Skipping the 1.100 pair also keeps its target
    directory, about 0.4G, off the disk. The spec gives Rust 1.78 to CI's
    MSRV job (§6).
18. **Both build sets stay, and are reused.** `target/` holds the 0.4.0
    build (Task 0) and the 0.5.0 build (Task 1 and Task 2), the latter
    plain and with its `opt-level = 1` embedded build, from the replay and
    the prototype. A version is in every crate's metadata, so the two sets
    have their own artifacts, side by side. The replay, after its first
    builds, moved from the tip to `3f72bd38` and back: `cargo build`, the
    tests' and `selfhost.rs`'s builds each compiled nothing either way.
    Two exceptions, both in place, neither costing disk:
    - the replay's first build at `3f72bd38` recompiled 16 crates
      (`nova-lexer` and those above it), because some of their sources'
      times were newer than the 0.4.0 artifacts (a checkout of 2026-11-23,
      after 5.6a's last build); once rebuilt, they stayed fresh;
    - the embedded build (Task 2, Step 5) recompiles `nova-cli` alone
      (23 s, then 16 s for `selfhost.rs`'s): under `NOVA_EMBED_RUNTIME`
      its build script watches `Cargo.lock`
      (`crates/nova-cli/build.rs:122`), whose time a checkout changes.

    Removing the 0.4.0 set's executables waits until after the merge
    (Task 2, Step 11): a step that needs room before then stops and asks
    (Global Constraints).
19. **The mutants** (Task 2, Step 7): 14 breakages of the release
    commit's files, each with the command that must catch it: `V`, the
    two version tests; `N`, `release-notes.sh 0.5.0 CHANGELOG.md`; and
    `R`, the records' check, `specblocks.py` with the commit's day, then
    `grep -m2 '^## \[' CHANGELOG.md`, which must print `## [Unreleased]`
    then `## [0.5.0] - D`. All 14 are caught: K1 (one pin left at
    `=0.4.0`) and K2 (`[workspace.package]`'s version left at 0.4.0) by
    cargo's resolution under `--locked`; L1 (one lock line left at 0.4.0)
    by `--locked`; E1 (`package.json`), E2 and E3 (`package-lock.json`'s
    two lines) by the version tests; C1 (`## [v0.5.0]`) by
    `release-notes.sh` and the grep; C3 (the lead left out), C4 (5.2b left
    out), X1 (the extension's section left out), R1 (the README's
    subsection left out), D1 and D2 (the runbook's notes left out) by
    `specblocks.py`; and C2 (the fresh `[Unreleased]` left out) by the
    grep. No test reads the records (spec §6), and none is added for them
    (decision 14): `R` is the plan's own check of the tree against the
    drafts of spec §3.2 to §3.5, run in Task 1, Step 5 and again after the
    fold (Task 2, Step 9), and the review reads them too. All 14 results
    come from one run at one tip.
20. **The review's fixes go into the release commit; after the push,
    nothing changes without the user's word.** The runbook's pull request
    has the release commit last (`docs/releasing.md:117`), and `main`'s
    tip after the merge is the commit the tag names (`:193-200`). Spec
    §3.7 prints that commit's subject (`git log -1 --format='%H %s'
    origin/main`); checking the subject before the tag is made is this
    plan's own addition, as in `v0.4.0`'s plan (Task 3, Step 2). So the
    one fix pass (Task 2, Step 9) folds its changes into the release
    commit before anything is pushed: on the commit's own day with `git
    commit --amend --no-edit`; on another day with `redate.sh` and `git
    commit --amend --date=now -F "$P/exec56b/msg-t1.txt"` (decision 15).
    The pull request keeps one release commit, as spec §8 has it. 5.6a's
    plan made a separate fix commit; this one cannot.
    - **After the push, nothing is amended or force-pushed without the
      user's word.** If CI or a review needs a change then, the executor
      stops, and the main session asks the user, offering two ways:
      - **amend the release commit and force-push with lease,** pinned to
        the pushed head (`--force-with-lease=<branch>:<pushed head>`): the
        release commit stays the pull request's last, as this decision
        wants;
      - **a new commit after it:** `main`'s tip is then not the release
        commit. The tag still names `main`'s release commit, `REL`, found
        by its subject and full hash (Task 2, Step 11; Task 3, Step 2).
        The release, its notes and the crates are built from the tag's
        tree, `REL`'s: `release.yml` runs on the tag, and its "Prepare
        the release" runs `release-notes.sh` in the tag's checkout
        (`release.yml:326`); "Publish to crates.io" runs on the tag
        (`publish-crates.yml:71` refuses any other ref); and "Release
        gate" checks out `v<version>` (`release-gate.yml:47,122`). So
        the later commit reaches none of them, and a fix to the records
        must go into the release commit, the first way, to reach the
        release. With this way, `release-notes.sh 0.5.0` is checked on
        `REL`'s tree, and CI's evidence for `REL`'s tree is the pull
        request's run on the first pushed head. The user hears this with
        the choice.
21. **CI's lines and counts for the pull request** (decision 12): every
    job green, CI's 19 and `release.yml`'s eight, with "Publish the
    release" skipped; the counts of run 45316548924 and `main`'s
    45142347493 (Task 2, Step 10). **Recorded, not expected:** the
    binaries' sizes, the runtime's directory (`0.5.0-<crc>`), the crates'
    bytes and every time: CI builds its runtime with its current stable,
    so a size or a directory can change without anything being wrong; the
    gate is S1 == S2 == S3 (ADR 0060 §2), which no size pins. Past a time
    line (80, 100 and 110 s for `selfhost.rs`, 75 s for the bootstrap
    step), the user is asked.
22. **The branch starts from `3f72bd38`:**
    `claude/loving-archimedes-frtf5q-56b` from `3f72bd38` (`main`
    `ed165975` and the approved spec), then this plan's commit, which adds
    this file and appends the spec's dated amendment (2026-11-24): the
    facts that proved wrong and CI's figures (decision 23), and §8's cut as
    this plan makes it (decision 16). Then Task 1's release commit. The
    patches apply to the plan's commit as to `3f72bd38`.
23. **The facts that proved wrong, and CI's figures** **(differs; the
    amendment records each)**:
    - **the prototype's base** (§7, item 1, "from `ed165975`"): it
      branched from `3f72bd38`, `ed165975` and the spec, which changes no
      file the release touches;
    - **5.2b in the list** (§3.2, "in the list's form"): the insertion
      re-wraps two lines into three, so the CHANGELOG's diff is 21 added
      and 2 removed, not 19 and 0;
    - **the release notes** (§10, risk 3, "about 41,000"): 40,612
      characters, 614 lines, 40,673 bytes;
    - **the gate after the bump** (§4, §10 risk 2): on CI, S1 == S2 == S3
      at 9,539,848 bytes each (+112 on 0.4.0's 9,539,736) on the runtime
      `0.5.0-4f8a6f1b`, N's `.ll` 12,760,111 as before, the step 26.66 s;
    - **"Packaged nova"** (§4): `nova-cli-0.5.0.crate` 1,759 files,
      584,057 bytes on Linux and macOS (+25 on 0.4.0), 586,939 on Windows;
      the 20 packages 2,119,078 bytes (+67), 2,130,553 on Windows, whose
      checkout has CRLF line endings;
    - **Windows' `selfhost.rs` time** (§4, §10 risk 6, 108.49 s on
      `main`'s run against its 110 s line): 73.06 s on run 45316548924;
      macOS 80.55 s (its line 100 s), Linux 53.90 s;
    - **the runbook's check** (§3.1): in the workspace's form, with
      `--no-fail-fast` (decision 14).
24. **Task 3, the steps after the merge,** in spec §5's order, each of
    the user's steps and each of the main session's writes a **[stop]**
    for the user's word (Steps 1, 3 and 10, the main session's reads and
    checks, need none), who does what as for `v0.4.0`: the user
    pushes the tag, makes the token and the secret, approves the
    deployment, uploads to the stores and tries the published `nova`; the
    agent starts the workflows and checks what each step made (spec §5,
    its decision 17).
    - **The agent is the main session itself, not a subagent, and works
      through its GitHub tools:** `actions_list` (`list_workflow_runs`,
      `list_workflow_jobs`), `actions_get` (`get_workflow_run`) and
      `get_job_logs` for the runs, each run waited for by a scheduled
      check-in (`send_later`) and its logs summarised by a read-only
      helper agent (Conventions); `get_tag` for the tag;
      `get_release_by_tag` and `get_latest_release` for the release; and
      `actions_run_trigger` to start a workflow (`run_workflow`), cancel a
      run (`cancel_workflow_run`) or re-run its failed jobs
      (`rerun_failed_jobs`). Each of these writes is made on the user's
      word for that step, and recorded in the ledger with the run's
      address.
    - **The session has no `gh` and runs no `curl`.** What its tools
      cannot do is the user's command, from the runbook, run at the
      user's own prompt with the output pasted back: the token and the
      secret, approving the deployment, deleting a tag or a release after
      a failure, crates.io's index, the uploads to the stores and their
      checks, and the try. If a dispatch is refused (HTTP 403 or the
      like), the user runs the runbook's `gh workflow run` line; a refusal
      is never worked around.
    - **Each step's checks are the runbook's,** cited by its lines **as
      `main` has it after the merge**, that is with this release's two
      notes in: the spec cites `ed165975`'s, which are 7 lower from
      `:335` on and 14 lower from `:525` on (the token's note adds lines
      335 to 341, its blank line first; step 6.2's, 532 to 538, its blank
      line last).
    - **The tag's commands are spec §3.7's,** with `main`'s subject
      checked before the tag is made, as `v0.4.0`'s plan gave them (Task
      3, Step 2; decision 20): the message and the steps are the spec's.
25. **The spec's recommendations, not asked, as taken** (spec §12, 9 to
    19): the branch names `-56b`, `-56b-ci`, `-56b-records`; both new
    headings dated with the release commit (decision 15); the runbook's
    two notes in this pull request; 5.2b in the list; the lead naming what
    the language and std gained; one commit; the README's two Markdown
    links; the tag message and the try as drafted (Task 3); the agent
    merging and starting the workflows on the user's word; the prototype
    measured on CI first (decision 12); no new test.
26. **The figures, this machine's** (the replay, 2026-11-24): 4 CPUs,
    Rust 1.97 (stable), Node v22.23.4, `clang-18` 18.1.3, `cc` 13.3.0, GNU
    ld 2.42, cgroup v1. The gate's sizes here, 14,287,432 bytes on the
    debug runtime and 9,354,688 on the embedded one (`0.5.0-04a13feb`),
    are this machine's, with Rust 1.97's runtimes: recorded, not expected.

### Task 0: The baseline at 0.4.0

The branch is `claude/loving-archimedes-frtf5q-56b`: from `3f72bd38`, then
this plan's commit, which adds this file and appends the spec's dated
amendment, so the patches apply to it as to `3f72bd38` (decision 22). The
0.4.0 build set in `target/` serves this task (decision 18).

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-56b`.
  - `git log --format='%h %s' -2` prints the plan's commit, then
    `3f72bd38 docs: the Phase 5.6b spec is approved`.
  - `git diff --name-only 3f72bd38 HEAD` lists only
    `docs/superpowers/plans/2026-11-24-phase-5-6b-the-release-v0.5.0.md`
    and
    `docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 2's review reads from it.
  - The disk check (Global Constraints), `Task 0 Step 1, the start`, shows
    at least 6,451 MiB. The replay began with 8,170 MiB.
- [ ] **Step 2: The tools and the process limit.**
  - `clang-18 --version | head -n 1` prints `Ubuntu clang version 18.1.3
    (1ubuntu1)`, CI's pin; `cc --version | head -n 1` and `ld --version |
    head -n 1` print `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0` and `GNU
    ld (GNU Binutils for Ubuntu) 2.42` here, as on CI; `node --version`
    prints `v22.23.4` here (any Node 22); `cargo --version` the local
    stable (1.97 here). Record them.
  - The process limit, as the env file sets it: `id -u` prints `0`; `grep
    nova-limit /proc/self/cgroup` prints `N:pids:/nova-limit`; `cat
    /sys/fs/cgroup/pids/nova-limit/pids.max` prints `600`. Record which
    way, and its output.
- [ ] **Step 3: The suite at 0.4.0.** In a shell set up as the
  conventions say, each command with its own timeout:

  ```sh
  mkdir -p "$P/exec56b"
  cargo build --locked --workspace
  target/debug/nova version
  cargo test --locked --workspace --no-run
  S --lib --bins > "$P/exec56b/base-a.txt" 2>&1
  S --doc > "$P/exec56b/base-b.txt" 2>&1
  S --test '[!s]*' --test 's[!e]*' > "$P/exec56b/base-c.txt" 2>&1
  cat "$P"/exec56b/base-[abc].txt > "$P/exec56b/base.txt"
  grep '^test result' "$P/exec56b/base.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec56b/base.txt"
  grep -c 'Running tests/selfhost.rs' "$P/exec56b/base-c.txt"
  for t in release_manifests vscode_extension project; do awk -v t="Running tests/$t.rs" 'index($0, t) { f = 1 } f && /^test result/ { print t ": " $0; f = 0 }' "$P/exec56b/base-c.txt"; done
  cargo test --locked --workspace -- --list > "$P/exec56b/base-list.txt" 2>&1
  grep -c ': test$' "$P/exec56b/base-list.txt"
  cargo test --locked --workspace --test selfhost -- --list > "$P/exec56b/base-list-sh.txt" 2>&1
  grep -c ': test$' "$P/exec56b/base-list-sh.txt"
  cargo test --locked --workspace --test selfhost --no-run
  time SH > "$P/exec56b/base-sh.txt" 2>&1; grep '^test result' "$P/exec56b/base-sh.txt"
  bash .github/scripts/release-notes.sh 0.5.0 CHANGELOG.md; echo "exit $?"
  cargo package --list --locked --offline -p nova-cli > "$P/exec56b/base-package-list.txt" 2> "$P/exec56b/base-package-list.err"; echo "exit $?"
  wc -l < "$P/exec56b/base-package-list.txt"; wc -c < "$P/exec56b/base-package-list.err"
  ls target/package
  sha256sum Cargo.lock
  ```

  Expected, from the replay on `3f72bd38`:
  - `cargo build` ends with `Finished`, and `nova version` prints `nova
    0.4.0`, `target: x86_64-unknown-linux-gnu` and `runtime: not
    embedded`. The build compiles nothing when the 0.4.0 set is fresh, as
    the replay's later builds found it; if it compiles crates, record
    which, the time and the disk (decision 18: the replay's first build
    recompiled 16 crates in place, 14.6 s, the disk unchanged);
  - the suite, in three runs (108, 1 and 182 s): `1300 1 8`, `2 0 0` and
    `1535 0 2`, `2837 1 10` summed; one failure, `test
    net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
    ... FAILED`, which needs IPv6, which this machine lacks, as in every
    earlier phase; `0` binaries named `tests/selfhost.rs`; in the third,
    `release_manifests` `8 passed`, `vscode_extension` `3 passed` and
    `project` `25 passed`: the versions agree at 0.4.0;
  - the list holds `2848` names (the run's 2837 + 1 + 10), and
    `selfhost.rs`'s `74`;
  - `SH`: `71 passed; 0 failed; 3 ignored`, in 171.23 s;
  - `release-notes.sh` prints `release-notes: CHANGELOG.md has no notes
    for 0.5.0` and `exit 1`: there is no `[0.5.0]` yet;
  - the package list as Task 2, Step 6 reads it: `exit 0`, `1759`, `0`
    (nothing on stderr), and `ls` failing (`No such file or
    directory`): no `target/package` made;
  - `Cargo.lock`'s sha256 begins `a3009b6850e56f7b`.
- [ ] **Step 4: The ledger.** Start `$P/ledger-56b.md`, headed with the
  base commit, the baseline's figures, `Cargo.lock`'s sha256, the tools'
  versions and the process limit's form; and `$P/exec56b`, the executor's
  directory (Conventions).


### Task 1: The release commit

The runbook's step 1 (`docs/releasing.md:115-189`) in one commit, `release:
v0.5.0, Phase 5 released`: spec §3.1 to §3.5, in three blocks (decision
13), the red and the green between them (decision 14). The 0.5.0 build set
in `target/` serves it (decision 18). Each step runs in a shell set up as
the conventions say.

- [ ] **Step 1: The versions, and the lock read line by line.**
  `patch_block t1-versions`, which prints nothing. Then:

  ```sh
  git diff --numstat
  git diff -U0 Cargo.lock | grep '^[-+][^-+]' | sort | uniq -c
  git diff -U1 Cargo.lock | grep -c '^ name = '
  comm -3 <(git diff -U1 Cargo.lock | sed -n 's/^ name = "\(.*\)"$/\1/p' | sort) <(cargo metadata --no-deps --offline --format-version 1 | python3 -c 'import json, sys; print("\n".join(sorted(p["name"] for p in json.load(sys.stdin)["packages"])))')
  git diff --word-diff=porcelain -U0 | grep -E '^[-+][^-+]' | sort | uniq -c
  sha256sum Cargo.lock
  ```

  Expected:
  - `23	23	Cargo.lock` and `21	21	Cargo.toml`: the runbook's 21 lines of
    `Cargo.toml` (`[workspace.package]`'s `version` and the 20 pins) and
    23 of `Cargo.lock` (`docs/releasing.md:137-140`), each changed in
    place;
  - `23 +version = "0.5.0"` and `23 -version = "0.4.0"`, nothing else:
    the lock's changed lines are its `version` lines alone;
  - `23`, and `comm` prints nothing: the 23 packages whose `version` moved
    are exactly the workspace's 23 (`cargo metadata --no-deps`);
  - every changed word is a version: `24 +"0.5.0"`, `20 +"=0.5.0"`, `24
    -"0.4.0"`, `20 -"=0.4.0"` (the lock's 23 and the workspace's version;
    the 20 pins);
  - the sha256 begins `f47897873c81b571`.

  **Anything else in the lock stops the task** (decision 8):
  `git checkout -- .`, record what differed, and ask the user.
- [ ] **Step 2: Run, and see it fail.** The disk check, `Task 1 Step 2,
  before the build`, at least 6,451 MiB. Then:

  ```sh
  cargo build --locked --workspace
  target/debug/nova version
  V > "$P/exec56b/t1-red.txt" 2>&1; echo "exit $?"
  grep -E '^test .* FAILED$|panicked at|^assertion|^  left|^ right|^test result|^error: [0-9]+ targets failed' "$P/exec56b/t1-red.txt"
  ```

  Expected: the build ends with `Finished` (the replay compiled nothing:
  the 0.5.0 set was fresh); `nova version` prints `nova 0.5.0`; `exit
  101`; and, in order (the thread numbers and the times vary):

  ```
  test the_extension_has_the_workspace_version_and_its_name ... FAILED
  thread 'the_extension_has_the_workspace_version_and_its_name' (…) panicked at crates/nova-cli/tests/release_manifests.rs:309:5:
  assertion `left == right` failed: package.json
    left: String("0.4.0")
   right: "0.5.0"
  test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.04s
  test the_extension_has_nova_clis_version_and_its_floor ... FAILED
  thread 'the_extension_has_nova_clis_version_and_its_floor' (…) panicked at crates/nova-cli/tests/vscode_extension.rs:79:5:
  assertion `left == right` failed
    left: String("0.4.0")
   right: "0.5.0"
  test result: FAILED. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
  error: 2 targets failed:
  ```

  Both fail for the extension's version alone: `package.json` still says
  0.4.0 where the workspace says 0.5.0. Every other test of the two
  passes, the pins and the members' versions among them. (Without
  `--no-fail-fast`, cargo stops after the first binary, and only the first
  six lines show: decision 14.)
- [ ] **Step 3: The extension.** `patch_block t1-extension`, which prints
  nothing. Then `git diff --numstat` adds `2	2
  tools/vscode-nova/package-lock.json` and `1	1
  tools/vscode-nova/package.json` to Step 1's two lines, and `git diff
  --word-diff=porcelain -U0 | grep -E '^[-+][^-+]' | sort | uniq -c`
  prints Step 1's four counts and `3 +"0.5.0",` and `3 -"0.4.0",`: the
  three version lines (`package.json:5`, `package-lock.json:3` and `:9`).
- [ ] **Step 4: Run.** `V > "$P/exec56b/t1-green.txt" 2>&1; echo "exit $?"`,
  then `grep '^test result' "$P/exec56b/t1-green.txt"`. Expected: `exit 0`;
  `8 passed; 0 failed` (`release_manifests`) and `3 passed; 0 failed`
  (`vscode_extension`), the runbook's check (`docs/releasing.md:143-147`).
- [ ] **Step 5: The records, and the date.** `patch_block t1-records`,
  which prints nothing; `git diff --numstat` lists the eight files:

  ```text
  21	2	CHANGELOG.md
  23	23	Cargo.lock
  21	21	Cargo.toml
  10	0	README.md
  14	0	docs/releasing.md
  4	0	tools/vscode-nova/CHANGELOG.md
  2	2	tools/vscode-nova/package-lock.json
  1	1	tools/vscode-nova/package.json
  ```

  Then the message, the date and the checks of the records. The message,
  the prototype's, written as it stands:

```sh
mkdir -p "$P/exec56b"
cat > "$P/exec56b/msg-t1.txt" <<'EOF'
release: v0.5.0, Phase 5 released

The workspace's version is 0.5.0 in four files: the root Cargo.toml
([workspace.package]'s version and the 20 exact pins), the extension's
package.json and package-lock.json, and Cargo.lock's 23 version lines,
the workspace's own packages by `cargo update --workspace`, nothing else
in it. CHANGELOG's [Unreleased] becomes [0.5.0] - 2026-11-24, under a
lead paragraph that calls Phase 5 released, says N lives in selfhost/
and is not shipped, cites main's CI run 45142347493 for S1 == S2 == S3
and names ADR 0061 for the boundary, with a fresh, empty [Unreleased]
above it; the list of sub-phases gains 5.2b. The extension's CHANGELOG
gets a [0.5.0] section: released with Nova 0.5.0, no change to the
extension. The README gains a Self-hosting subsection, and the runbook
two dated notes: v0.5.0's token takes publish-update alone, and the
maintainer's try of the published nova replaces step 6.2.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
EOF
```

  `redate.sh` (decision 15) and `specblocks.py`, written as they stand:

```sh
mkdir -p "$P/exec56b"
cat > "$P/exec56b/redate.sh" <<'EOF'
#!/usr/bin/env bash
# redate.sh DATE MSG: date the release commit DATE (YYYY-MM-DD, the day of
# the commit in UTC). The patches date it 2026-11-24 in exactly four places,
# and the commit message in one: the two `## [0.5.0] - ` headings
# (CHANGELOG.md, tools/vscode-nova/CHANGELOG.md), the runbook's two notes
# `**Amended … (branch `claude/loving-archimedes-frtf5q-56b`):**`
# (docs/releasing.md), and the message's `[0.5.0] - `. Each is rewritten
# whatever date it holds, so the script can run again; the spec's file name,
# which holds 2026-11-24 too, is left alone. Then it counts: each place holds
# DATE, and, when DATE is another day, those files and the message hold no
# 2026-11-24 but in the spec's file name. Run from the repository's root,
# after t1-records and before the commit.
set -euo pipefail
D=$1
MSG=$2
[[ $D =~ ^20[0-9][0-9]-[01][0-9]-[0-3][0-9]$ ]] && [ "$(date -u -d "$D" +%F 2>/dev/null)" = "$D" ] ||
  { echo "redate: not a date: $D" >&2; exit 1; }
export D
perl -pi -e 's/^## \[0\.5\.0\] - \d{4}-\d{2}-\d{2}$/## [0.5.0] - $ENV{D}/' CHANGELOG.md tools/vscode-nova/CHANGELOG.md
perl -pi -e 's/\*\*Amended \d{4}-\d{2}-\d{2} (\(branch `claude\/loving-archimedes-frtf5q-56b`\):\*\*)/**Amended $ENV{D} $1/' docs/releasing.md
perl -pi -e 's/\[0\.5\.0\] - \d{4}-\d{2}-\d{2}/[0.5.0] - $ENV{D}/' "$MSG"
n1=$(grep -c "^## \[0\.5\.0\] - $D\$" CHANGELOG.md || true)
n2=$(grep -c "^## \[0\.5\.0\] - $D\$" tools/vscode-nova/CHANGELOG.md || true)
n3=$(grep -c "\*\*Amended $D (branch \`claude/loving-archimedes-frtf5q-56b\`):\*\*" docs/releasing.md || true)
n4=$(grep -c "\[0\.5\.0\] - $D" "$MSG" || true)
all=$(grep -c "(branch \`claude/loving-archimedes-frtf5q-56b\`):\*\*" docs/releasing.md || true)
left=$(cat CHANGELOG.md tools/vscode-nova/CHANGELOG.md docs/releasing.md README.md "$MSG" | grep -v '2026-11-24-phase-5-6b' | grep -c '2026-11-24' || true)
echo "redate: $D in CHANGELOG.md $n1, tools/vscode-nova/CHANGELOG.md $n2, docs/releasing.md $n3 of $all notes, the message $n4"
echo "redate: 2026-11-24 outside the spec's file name: $left"
[ "$n1 $n2 $n3 $all $n4" = "1 1 2 2 1" ] || { echo "redate: FAILED: not every place holds $D" >&2; exit 1; }
if [ "$D" != 2026-11-24 ] && [ "$left" != 0 ]; then
  echo "redate: FAILED: 2026-11-24 is left" >&2
  exit 1
fi
echo "redate: ok"
EOF
```

```sh
mkdir -p "$P/exec56b"
cat > "$P/exec56b/specblocks.py" <<'EOF'
#!/usr/bin/env python3
"""specblocks.py DATE: check the release commit's records against the
spec's drafts. The spec's five ```markdown blocks (§3.2's lead, §3.3's
section, §3.4's README section, §3.5's two notes), each taken out of its
fence with the fence's own indentation removed and its date placeholder
(`<the release commit's date>`, `<date>`) given DATE, must each occur
verbatim in its file in the working tree; and §3.2's 5.2b sentence must
occur in CHANGELOG.md, compared with its line breaks as spaces. Run from
the repository's root."""
import re
import sys

D = sys.argv[1]
SPEC = "docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md"
TARGETS = ["CHANGELOG.md", "tools/vscode-nova/CHANGELOG.md", "README.md", "docs/releasing.md", "docs/releasing.md"]
lines = open(SPEC, encoding="utf-8").read().split("\n")
blocks, k = [], 0
while k < len(lines):
    m = re.match(r"^( *)```markdown$", lines[k])
    if m:
        pad, body, k = len(m.group(1)), [], k + 1
        while lines[k].strip() != "```":
            body.append(lines[k][pad:] if lines[k].startswith(" " * pad) else lines[k].lstrip())
            k += 1
        blocks.append("\n".join(body).replace("<the release commit's date>", D).replace("<date>", D) + "\n")
    k += 1
bad = 0
if len(blocks) != len(TARGETS):
    sys.exit(f"specblocks: {len(blocks)} blocks in the spec, not {len(TARGETS)}")
for n, (block, path) in enumerate(zip(blocks, TARGETS), 1):
    text = open(path, encoding="utf-8").read()
    found = text.count(block)
    print(f"block {n} ({block.count(chr(10))} lines) in {path}: {'found' if found == 1 else f'FOUND {found} TIMES'}")
    bad += found != 1
sentence = ("§3; ADR 0053), and 5.2b, packages, the WASM target and test mode in the "
            "resolution dump (§4; ADR 0054).")
flat = " ".join(open("CHANGELOG.md", encoding="utf-8").read().split())
print(f"the 5.2b sentence in CHANGELOG.md: {'found' if flat.count(sentence) == 1 else 'NOT FOUND'}")
bad += flat.count(sentence) != 1
print("specblocks: ok" if not bad else "specblocks: FAILED")
sys.exit(1 if bad else 0)
EOF
```

  Then:

  ```sh
  D=$(date -u +%F); echo "$D"
  bash "$P/exec56b/redate.sh" "$D" "$P/exec56b/msg-t1.txt"
  python3 "$P/exec56b/specblocks.py" "$D"
  grep -m2 '^## \[' CHANGELOG.md
  git diff --numstat
  bash .github/scripts/release-notes.sh 0.5.0 CHANGELOG.md > "$P/exec56b/notes-0.5.0.md"; echo "exit $?"
  wc -l < "$P/exec56b/notes-0.5.0.md"; wc -c < "$P/exec56b/notes-0.5.0.md"; LC_ALL=C.UTF-8 wc -m < "$P/exec56b/notes-0.5.0.md"
  sed -n 2p "$P/exec56b/notes-0.5.0.md"
  ```

  Expected:
  - `redate.sh` prints `redate: D in CHANGELOG.md 1,
    tools/vscode-nova/CHANGELOG.md 1, docs/releasing.md 2 of 2 notes, the
    message 1` (with the day for `D`), then `redate: 2026-11-24 outside the
    spec's file name: 0` on any other day (`5` on 2026-11-24 itself: the
    five places), then `redate: ok`;
  - `specblocks.py` prints `found` for each of its five blocks (the lead,
    15 lines, in `CHANGELOG.md`; the extension's section, 3 lines; the
    README's subsection, 9 lines; the runbook's two notes, 6 lines each),
    `the 5.2b sentence in CHANGELOG.md: found` and `specblocks: ok`: the
    records hold the spec's drafts word for word, dated `D`;
  - the grep prints `## [Unreleased]`, then `## [0.5.0] - D`: the fresh,
    empty `[Unreleased]` is above the release's section (spec §3.2;
    `specblocks.py` and this grep are Task 2, Step 7's `R`);
  - `git diff --numstat` as above: the date changes no line count;
  - `exit 0`; `614` lines, `40673` bytes, `40612` characters (spec §10,
    risk 3; GitHub's limit for a release's body is 125,000); and the
    second line is the lead's first, `**Phase 5, "Self-hosting", is
    released.** This is the `v0.{phase}.0``: `release.yml:326` will find
    `## [0.5.0]` and take its section (the first line is blank, as the
    section's first line).
- [ ] **Step 6: The checks,** as the conventions list them, after the
  disk check (`Task 1 Step 6, before the checks`, at least 6,451 MiB) and
  `cargo build --locked --workspace`. The replay: `cargo fmt` clean;
  clippy 11.87 s and 1.79 s, no warning; both `nova fmt --check` lines
  silent. No Rust changed, so the Rust 1.100 pair and the Windows mimic do
  not run (decision 17).
- [ ] **Step 7: Commit.**

  ```sh
  git add -A
  git status --short
  git commit -F "$P/exec56b/msg-t1.txt"
  D=$(sed -n 's/^## \[0\.5\.0\] - \([0-9-]*\)$/\1/p' CHANGELOG.md); echo "$D"
  test "$(TZ=UTC git log -1 --date=format-local:%Y-%m-%d --format=%ad)" = "$D" && echo "dated $D"
  printf '%s\n' "$(git log -1 --format=%B)" | tail -n 3
  git cat-file -e 4f55d6ed^{commit} || git fetch origin pull/150/head
  git diff 4f55d6ed HEAD -- . ':(exclude)docs/superpowers' | grep '^[-+][^-+]'
  sha256sum Cargo.lock
  ```

  Expected:
  - `git status --short` lists the eight files, each `M`, and nothing
    else;
  - the day the records carry, Step 5's, then `dated D`: the commit's own
    date, in UTC, is that day. If it prints nothing, the day turned since
    Step 5, and the commit is re-dated (decision 15), each command with
    its own timeout:

    ```sh
    D=$(date -u +%F); echo "$D"
    bash "$P/exec56b/redate.sh" "$D" "$P/exec56b/msg-t1.txt"
    python3 "$P/exec56b/specblocks.py" "$D"
    grep -m2 '^## \[' CHANGELOG.md
    git add -A
    git status --short
    git commit --amend --date=now -F "$P/exec56b/msg-t1.txt"
    ```

    each as Step 5 gives it (`git status --short` lists the three files
    the day is in, `CHANGELOG.md`, `docs/releasing.md` and
    `tools/vscode-nova/CHANGELOG.md`, their dated lines alone changed),
    then this step's checks again from its `D=` line;
  - the message ends with a blank line and the two trailer lines, the
    last three lines printed;
  - `git cat-file` prints nothing (the prototype, `4f55d6ed`, the head of
    draft Sakeerin/nova#150, is in the clone; if not, the fetch takes it
    from #150's ref); the diff against `4f55d6ed`, whose tree the replay's
    split has, prints nothing on 2026-11-24; on any other day it prints
    eight lines, the four dated lines old and new, and nothing else;
  - the sha256 begins `f47897873c81b571`.

  Record the commit, the day, and the comparison in the ledger.

<!-- patch t1-versions -->

````diff
diff --git a/Cargo.lock b/Cargo.lock
index b92048fb..05a4de3c 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -938,7 +938,7 @@ dependencies = [
 
 [[package]]
 name = "nova-ast"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "nova-diagnostics",
  "serde",
@@ -947,11 +947,11 @@ dependencies = [
 
 [[package]]
 name = "nova-bench-http"
-version = "0.4.0"
+version = "0.5.0"
 
 [[package]]
 name = "nova-bundler"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "nova-diagnostics",
@@ -960,7 +960,7 @@ dependencies = [
 
 [[package]]
 name = "nova-cli"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "assert_cmd",
@@ -989,7 +989,7 @@ dependencies = [
 
 [[package]]
 name = "nova-codegen-cranelift"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "cranelift",
@@ -1007,7 +1007,7 @@ dependencies = [
 
 [[package]]
 name = "nova-codegen-llvm"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "nova-diagnostics",
@@ -1022,7 +1022,7 @@ dependencies = [
 
 [[package]]
 name = "nova-codegen-wasm"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "nova-diagnostics",
@@ -1034,7 +1034,7 @@ dependencies = [
 
 [[package]]
 name = "nova-diagnostics"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "ariadne",
  "codespan-reporting",
@@ -1043,7 +1043,7 @@ dependencies = [
 
 [[package]]
 name = "nova-doc"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "nova-ast",
  "nova-diagnostics",
@@ -1061,7 +1061,7 @@ dependencies = [
 
 [[package]]
 name = "nova-driver"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "cc",
@@ -1085,7 +1085,7 @@ dependencies = [
 
 [[package]]
 name = "nova-fmt"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "nova-ast",
@@ -1097,7 +1097,7 @@ dependencies = [
 
 [[package]]
 name = "nova-hir"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "nova-diagnostics",
  "nova-resolver",
@@ -1105,7 +1105,7 @@ dependencies = [
 
 [[package]]
 name = "nova-index"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "base64",
  "flate2",
@@ -1122,7 +1122,7 @@ dependencies = [
 
 [[package]]
 name = "nova-lexer"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "criterion",
  "insta",
@@ -1134,7 +1134,7 @@ dependencies = [
 
 [[package]]
 name = "nova-lsp"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "crc32fast",
@@ -1157,7 +1157,7 @@ dependencies = [
 
 [[package]]
 name = "nova-mir"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "nova-diagnostics",
  "nova-hir",
@@ -1170,7 +1170,7 @@ dependencies = [
 
 [[package]]
 name = "nova-parser"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "insta",
  "nova-ast",
@@ -1182,7 +1182,7 @@ dependencies = [
 
 [[package]]
 name = "nova-pm"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "crc32fast",
  "nova-diagnostics",
@@ -1193,7 +1193,7 @@ dependencies = [
 
 [[package]]
 name = "nova-resolver"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "indexmap",
  "nova-ast",
@@ -1207,7 +1207,7 @@ dependencies = [
 
 [[package]]
 name = "nova-runtime"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "cc",
@@ -1221,11 +1221,11 @@ dependencies = [
 
 [[package]]
 name = "nova-std"
-version = "0.4.0"
+version = "0.5.0"
 
 [[package]]
 name = "nova-test"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "anyhow",
  "nova-diagnostics",
@@ -1234,7 +1234,7 @@ dependencies = [
 
 [[package]]
 name = "nova-typeck"
-version = "0.4.0"
+version = "0.5.0"
 dependencies = [
  "indexmap",
  "nova-ast",
diff --git a/Cargo.toml b/Cargo.toml
index 6a35cc87..d377cde5 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -8,7 +8,7 @@ resolver = "2"
 # and package-lock.json, and Cargo.lock (spec
 # docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
 # §3.2); nova-cli's tests/release_manifests.rs holds them to it.
-version = "0.4.0"
+version = "0.5.0"
 edition = "2021"
 rust-version = "1.78"
 license = "MIT OR Apache-2.0"
@@ -24,26 +24,26 @@ homepage = "https://github.com/Sakeerin/nova"
 # stable, so nova-cli is never built against another release's
 # nova-driver, even by a `cargo install` without `--locked`.
 # Dev-dependencies stay `{ path = ... }`, which `cargo package` strips.
-nova-ast = { path = "crates/nova-ast", version = "=0.4.0" }
-nova-cli = { path = "crates/nova-cli", version = "=0.4.0" }
-nova-codegen-cranelift = { path = "crates/nova-codegen-cranelift", version = "=0.4.0" }
-nova-codegen-llvm = { path = "crates/nova-codegen-llvm", version = "=0.4.0" }
-nova-codegen-wasm = { path = "crates/nova-codegen-wasm", version = "=0.4.0" }
-nova-diagnostics = { path = "crates/nova-diagnostics", version = "=0.4.0" }
-nova-doc = { path = "crates/nova-doc", version = "=0.4.0" }
-nova-driver = { path = "crates/nova-driver", version = "=0.4.0" }
-nova-fmt = { path = "crates/nova-fmt", version = "=0.4.0" }
-nova-hir = { path = "crates/nova-hir", version = "=0.4.0" }
-nova-index = { path = "crates/nova-index", version = "=0.4.0" }
-nova-lexer = { path = "crates/nova-lexer", version = "=0.4.0" }
-nova-lsp = { path = "crates/nova-lsp", version = "=0.4.0" }
-nova-mir = { path = "crates/nova-mir", version = "=0.4.0" }
-nova-parser = { path = "crates/nova-parser", version = "=0.4.0" }
-nova-pm = { path = "crates/nova-pm", version = "=0.4.0" }
-nova-resolver = { path = "crates/nova-resolver", version = "=0.4.0" }
-nova-runtime = { path = "crates/nova-runtime", version = "=0.4.0" }
-nova-std = { path = "std", version = "=0.4.0" }
-nova-typeck = { path = "crates/nova-typeck", version = "=0.4.0" }
+nova-ast = { path = "crates/nova-ast", version = "=0.5.0" }
+nova-cli = { path = "crates/nova-cli", version = "=0.5.0" }
+nova-codegen-cranelift = { path = "crates/nova-codegen-cranelift", version = "=0.5.0" }
+nova-codegen-llvm = { path = "crates/nova-codegen-llvm", version = "=0.5.0" }
+nova-codegen-wasm = { path = "crates/nova-codegen-wasm", version = "=0.5.0" }
+nova-diagnostics = { path = "crates/nova-diagnostics", version = "=0.5.0" }
+nova-doc = { path = "crates/nova-doc", version = "=0.5.0" }
+nova-driver = { path = "crates/nova-driver", version = "=0.5.0" }
+nova-fmt = { path = "crates/nova-fmt", version = "=0.5.0" }
+nova-hir = { path = "crates/nova-hir", version = "=0.5.0" }
+nova-index = { path = "crates/nova-index", version = "=0.5.0" }
+nova-lexer = { path = "crates/nova-lexer", version = "=0.5.0" }
+nova-lsp = { path = "crates/nova-lsp", version = "=0.5.0" }
+nova-mir = { path = "crates/nova-mir", version = "=0.5.0" }
+nova-parser = { path = "crates/nova-parser", version = "=0.5.0" }
+nova-pm = { path = "crates/nova-pm", version = "=0.5.0" }
+nova-resolver = { path = "crates/nova-resolver", version = "=0.5.0" }
+nova-runtime = { path = "crates/nova-runtime", version = "=0.5.0" }
+nova-std = { path = "std", version = "=0.5.0" }
+nova-typeck = { path = "crates/nova-typeck", version = "=0.5.0" }
 
 # Parsing & errors
 chumsky = "0.9"
````

<!-- patch t1-extension -->

````diff
diff --git a/tools/vscode-nova/package-lock.json b/tools/vscode-nova/package-lock.json
index a9317afb..f57627af 100644
--- a/tools/vscode-nova/package-lock.json
+++ b/tools/vscode-nova/package-lock.json
@@ -1,12 +1,12 @@
 {
   "name": "vscode-nova",
-  "version": "0.4.0",
+  "version": "0.5.0",
   "lockfileVersion": 3,
   "requires": true,
   "packages": {
     "": {
       "name": "vscode-nova",
-      "version": "0.4.0",
+      "version": "0.5.0",
       "license": "MIT OR Apache-2.0",
       "dependencies": {
         "vscode-languageclient": "^10.1.2"
diff --git a/tools/vscode-nova/package.json b/tools/vscode-nova/package.json
index f45677c8..9e9e7a4f 100644
--- a/tools/vscode-nova/package.json
+++ b/tools/vscode-nova/package.json
@@ -2,7 +2,7 @@
   "name": "vscode-nova",
   "displayName": "Nova Language Tools",
   "description": "The Nova language: colouring, and diagnostics, completion, formatting, hover, go to definition, find references, rename, quick fixes, organize imports and semantic highlighting from nova lsp.",
-  "version": "0.4.0",
+  "version": "0.5.0",
   "publisher": "sakeerin",
   "license": "MIT OR Apache-2.0",
   "repository": {
````

<!-- patch t1-records -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index d7bcd070..6f249f3a 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -9,6 +9,24 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
 
 ## [Unreleased]
 
+## [0.5.0] - 2026-11-24
+
+**Phase 5, "Self-hosting", is released.** This is the `v0.{phase}.0`
+milestone `nova-spec/00-MASTER-SPEC.md` §7 reserves for a phase. Nova
+gained what a compiler written in Nova needs: `?`, `+=` on fields and
+elements, tuples, nested patterns and or-patterns, `Char` literals and
+ranges, and in std `StringBuilder`, number parsing, `OrderedMap` and
+`std/process`. That compiler, N, lives in `selfhost/` and is not
+shipped: the release ships the Rust `nova`, as before. The bootstrap
+passes its gate in CI: `nova build --release` builds N into S1, S1
+builds N into S2, and S2 builds N into S3, and S1 == S2 == S3, byte for
+byte, in `main`'s CI run 45142347493
+(https://github.com/Sakeerin/nova/actions/runs/45142347493;
+`docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md`). The boundary
+Phase 5 closes within will be recorded in
+`docs/adr/0061-phase-5-boundary.md`. `nova --version` now reports
+0.5.0.
+
 Phase 5.0a, the small language and CI (spec
 `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
 §3; ADR 0048, Phase 5's scope and backend), 5.0b, tuples and
@@ -19,8 +37,9 @@ self-hosted lexer and the differential (spec
 depth limit (§4; ADR 0052). Phase 5.2a, the self-hosted loader and
 resolver for loose programs and the resolution dump (spec
 `docs/superpowers/specs/2026-11-02-phase-5-2-the-loader-and-the-resolver-design.md`
-§3; ADR 0053). Phase 5.3a, the self-hosted item collection and the HIR
-dump (spec
+§3; ADR 0053), and 5.2b, packages, the WASM target and test mode in the
+resolution dump (§4; ADR 0054). Phase 5.3a, the self-hosted item
+collection and the HIR dump (spec
 `docs/superpowers/specs/2026-11-06-phase-5-3-the-type-checker-and-hir-design.md`
 §3; ADR 0055), and 5.3b, the self-hosted type checker's bodies and the
 full HIR dump (§4; ADR 0056). Phase 5.4, the self-hosted MIR and the MIR
diff --git a/README.md b/README.md
index 082baada..d088fb50 100644
--- a/README.md
+++ b/README.md
@@ -119,6 +119,16 @@ as a component, runs with `nova dev` in its directory; and
 sessions, forms and an admin, builds with `nova build --ssr`, then sets its
 admin's password and starts its server, as its README shows.
 
+### Self-hosting
+
+`selfhost/` holds N, Nova's compiler written in Nova. It is not shipped:
+a release ships the Rust `nova`, and N stays in the repository. On every
+pull request, on Linux, CI builds N with `nova build --release` into S1,
+then N with S1 into S2 and with S2 into S3, and holds S1 == S2 == S3,
+byte for byte. [`selfhost/README.md`](selfhost/README.md) says how to
+build and run it; [ADR 0060](docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md)
+records the gate.
+
 ---
 
 ## How to use this bundle
diff --git a/docs/releasing.md b/docs/releasing.md
index ae76ad91..da5516b6 100644
--- a/docs/releasing.md
+++ b/docs/releasing.md
@@ -332,6 +332,13 @@ nothing published.
    new versions of crates that exist. The new crate falls within the
    burst of 5, so its upload takes minutes, as `0.3.1`'s and `0.3.2`'s
    did, not the 2 h 20 min of 19 new crates above.
+
+   **Amended 2026-11-24 (branch `claude/loving-archimedes-frtf5q-56b`):**
+   `v0.5.0` adds no crate: its 20 are new versions of crates on
+   crates.io, within the burst of 30, so its token takes
+   **publish-update** alone (spec
+   `docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`,
+   the user's decision 5).
 5. Check what crates.io serves:
 
    ```sh
@@ -522,6 +529,13 @@ by their READMEs, in place of this run (spec
 `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`,
 decision 6; `docs/adr/0047-phase-4-boundary.md`).
 
+**Amended 2026-11-24 (branch `claude/loving-archimedes-frtf5q-56b`):** for
+`v0.5.0`, by the user's decision of 2026-11-23, the maintainer installs
+the published `nova` and tries it in place of this run, as for
+`v0.4.0` (spec
+`docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`,
+decision 5). `docs/adr/0061-phase-5-boundary.md` will record the try.
+
 The commands, in order, each with what it should show:
 
 1. Install from crates.io; time it.
diff --git a/tools/vscode-nova/CHANGELOG.md b/tools/vscode-nova/CHANGELOG.md
index 67c33c25..af8994e6 100644
--- a/tools/vscode-nova/CHANGELOG.md
+++ b/tools/vscode-nova/CHANGELOG.md
@@ -6,6 +6,10 @@ Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
 The extension's version is always the version of Nova it is released
 with.
 
+## [0.5.0] - 2026-11-24
+
+Released with Nova 0.5.0; no change to the extension.
+
 ## [0.4.0] - 2026-10-23
 
 Released with Nova 0.4.0, whose language gained components, written
````

### Task 2: Verification, the mutants, the review, the PR, and the merge

Every step runs in a shell set up as the conventions say, at Task 1's
commit, committed, on the 0.5.0 build set (decision 18).

- [ ] **Step 1: The full checks.** First, before the build, the disk check
  (`Task 2 Step 1, before the build`) must show at least 6,451 MiB. Then
  `names.py`, written as it stands:

```sh
mkdir -p "$P/exec56b"
cat > "$P/exec56b/names.py" <<'EOF'
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

  It compares Task 0's list with this one binary by binary, by name: a
  binary is its `Running` or `Doc-tests` line, its hash stripped, an
  integration test's prefixed with the crate whose unit tests cargo listed
  just before it; a test is a line ending `: test`. Then, each command
  with its own timeout:

  ```sh
  cargo build --locked --workspace
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo clippy --locked --all-features --test selfhost -- -D warnings
  target/debug/nova fmt --check std examples selfhost
  target/debug/nova fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr selfhost
  cargo test --locked --workspace --no-run
  S --lib --bins > "$P/exec56b/full-a.txt" 2>&1
  S --doc > "$P/exec56b/full-b.txt" 2>&1
  S --test '[!s]*' --test 's[!e]*' > "$P/exec56b/full-c.txt" 2>&1
  cat "$P"/exec56b/full-[abc].txt > "$P/exec56b/full.txt"
  grep '^test result' "$P/exec56b/full.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/exec56b/full.txt"
  grep -c 'Running tests/selfhost.rs' "$P/exec56b/full-c.txt"
  for t in release_manifests vscode_extension project; do awk -v t="Running tests/$t.rs" 'index($0, t) { f = 1 } f && /^test result/ { print t ": " $0; f = 0 }' "$P/exec56b/full-c.txt"; done
  cargo test --locked --workspace -- --list > "$P/exec56b/final-list.txt" 2>&1
  cargo test --locked --workspace --test selfhost -- --list > "$P/exec56b/final-list-sh.txt" 2>&1
  python3 "$P/exec56b/names.py" "$P/exec56b/base-list.txt" "$P/exec56b/final-list.txt"
  comm -3 <(grep ': test$' "$P/exec56b/base-list-sh.txt" | sort) <(grep ': test$' "$P/exec56b/final-list-sh.txt" | sort)
  ```

  Expected:
  - the build ends with `Finished` (the replay: nothing compiled, 0.10 s);
    the formatters print nothing, and both clippy commands end with
    `Finished`, with no warning or error;
  - the suite, in three runs (118, 4 and 211 s here): `1300 1 8`, `2 0 0`
    and `1535 0 2`, `2837 1 10` summed; the one failure is Task 0's IPv6
    test; `0` binaries named `tests/selfhost.rs`; `release_manifests` `8
    passed`, `vscode_extension` `3 passed`, `project` `25 passed`;
  - `names.py` prints `binaries: base 111, now 111`, `tests: base 2848,
    now 2848` and `net added: 0`: the suite's tests are the base's;
  - `comm` prints nothing: `selfhost.rs`'s 74 tests are the base's.
- [ ] **Step 2: The diff, read whole.**

  ```sh
  git diff --numstat 3f72bd38 HEAD -- . ':(exclude)docs/superpowers'
  git diff --word-diff=porcelain -U0 3f72bd38 HEAD -- Cargo.toml Cargo.lock tools/vscode-nova/package.json tools/vscode-nova/package-lock.json | grep -E '^[-+][^-+]' | sort | uniq -c
  git diff -U0 3f72bd38 HEAD -- Cargo.lock | grep '^[-+][^-+]' | sort | uniq -c
  grep -n -B1 '^version = "0.4.0"' Cargo.lock | grep 'name = '
  sha256sum Cargo.lock
  git diff 3f72bd38 HEAD -- . ':(exclude)docs/superpowers' | grep '^+[^+]' | grep -n -i -E '/home/|/tmp/|/root|scratch|claude-|opus|sonnet|haiku|\bproto/|\bsplit/'
  git grep -n -I '0\.4\.0' HEAD -- . ':(exclude)CHANGELOG.md' ':(exclude)docs' ':(exclude)nova-spec' ':(exclude)Cargo.lock' ':(exclude)tools/vscode-nova/CHANGELOG.md' | grep -v 'phase-4-8-the-gate-and-v0\.4\.0-design\.md'
  git log --format='%h %s' 3f72bd38..HEAD
  ```

  Expected:
  - the eight files of Task 1, Step 5, as its numstat gives them: `96`
    lines added and `49` removed in all;
  - the four version files' changed words, all versions: `24 +"0.5.0"`,
    `3 +"0.5.0",`, `20 +"=0.5.0"`, `24 -"0.4.0"`, `3 -"0.4.0",`, `20
    -"=0.4.0"`;
  - the lock's `23 +version = "0.5.0"` and `23 -version = "0.4.0"`, and
    its four lines still at 0.4.0 are the third-party `difflib`,
    `rand_xorshift`, `winapi-i686-pc-windows-gnu` and
    `winapi-x86_64-pc-windows-gnu` (spec §3.1);
  - the sha256 begins `f47897873c81b571`;
  - the privacy grep prints nothing: no added line names a local path, a
    scratch directory, a model or a local branch (the added lines name
    `claude/loving-archimedes-frtf5q-56b`, a pushed branch's name, which
    the pattern does not match);
  - the stale search prints exactly the two comments that say "from
    v0.4.0 on", `.github/scripts/gate.sh:18` and
    `.github/workflows/release-gate.yml:75` (spec §3.6): no file the
    release reads names 0.4.0 as current, outside the records and the 4.8
    spec's file name;
  - the log: the release commit, then the plan's commit.
- [ ] **Step 3: The version and the notes.**

  ```sh
  target/debug/nova version
  bash .github/scripts/release-notes.sh 0.5.0 CHANGELOG.md > "$P/exec56b/notes-0.5.0.md"; echo "exit $?"
  git show 3f72bd38:CHANGELOG.md > "$P/exec56b/changelog-base.md"
  bash .github/scripts/release-notes.sh Unreleased "$P/exec56b/changelog-base.md" > "$P/exec56b/notes-base.md"
  diff "$P/exec56b/notes-base.md" "$P/exec56b/notes-0.5.0.md" | grep -v '^[<>-]'
  diff "$P/exec56b/notes-base.md" "$P/exec56b/notes-0.5.0.md" | grep -c '^[<>]'
  ```

  Expected: `nova 0.5.0`, `target: x86_64-unknown-linux-gnu`, `runtime:
  not embedded`; `exit 0`; the diff's hunks `1a2,17` and `12,13c28,30`,
  and `21` changed lines: the section is the base's `[Unreleased]` (597
  lines, 39,616 characters) with the lead and its blank line added (16
  lines) and the 5.2b sentence re-wrapping two lines into three (spec
  §3.2: nothing else changes).
- [ ] **Step 4: `selfhost.rs`, the gate on the debug runtime, and the
  golden suite.** The disk check (`Task 2 Step 4, before selfhost.rs`), at
  least 6,451 MiB. Then:

  ```sh
  cargo test --locked --workspace --test selfhost --no-run
  time SH > "$P/exec56b/full-sh.txt" 2>&1; grep '^test result' "$P/exec56b/full-sh.txt"
  time C the_bootstrap_holds_s1_s2_s3 --nocapture > "$P/exec56b/gate-debug.txt" 2>&1; grep '^bootstrap\|^test result' "$P/exec56b/gate-debug.txt"
  time G > "$P/exec56b/golden.txt" 2>&1; grep '^release golden\|^test result' "$P/exec56b/golden.txt"
  git status --short
  ```

  Expected:
  - `SH`: `71 passed; 0 failed; 3 ignored`, 174.58 s here (the base's
    171.23 s), no child past the 120 s `CHILD_LIMIT`;
  - the gate: `1 passed; 0 failed; 0 ignored; 0 measured; 73 filtered
    out`, in 51.52 s here, its report `bootstrap: S1 == S2 == S3, SIZE
    bytes each; N's .ll the same from S0, S1 and S2, 12760111 bytes`, the
    clang line `bootstrap: Ubuntu clang version 18.1.3 (1ubuntu1)`, the
    runtime line naming `target/debug/libnova_runtime.a` (a plain build
    carries no runtime), and the stages' times. SIZE was 14287432 here:
    this machine's figure with Rust 1.97's debug runtime, which holds its
    build paths, recorded, not expected (decision 26); what must hold is
    one size for the three;
  - the golden suite: `release golden: 147 passed, 9 excluded, Ubuntu clang
    version 18.1.3 (1ubuntu1)` and `1 passed`, 21 s here;
  - `git status --short` prints nothing.
- [ ] **Step 5: The gate as CI's step runs it.** **First the disk,** since
  this changes the opt-level and the runtime: the disk check, `Task 2 Step
  5, before the embedded build`, must show at least 7,168 MiB; if not,
  stop and report (Global Constraints: nothing is removed before the
  merge). Then, in every tool call of this step after the env file,
  `export NOVA_EMBED_RUNTIME=1 CARGO_PROFILE_DEV_OPT_LEVEL=1`, two of the
  Selfhost job's variables (the third, the workflow's `RUST_BACKTRACE=1`,
  is not set: the shell keeps `0`):

  ```sh
  time cargo build --locked --workspace
  target/debug/nova version
  time cargo test --locked --workspace --all-features --test selfhost --no-run
  cc --version | head -n 1; ld --version | head -n 1
  time NOVA_REQUIRE_CLANG=1 NOVA_CLANG=clang-18 cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3 > "$P/exec56b/gate-ci.txt" 2>&1
  grep '^bootstrap\|^test result' "$P/exec56b/gate-ci.txt"
  git status --short
  ```

  The fifth command is the bootstrap step's `cargo test` line as
  `ci.yml` writes it, with the step's variables. Expected:
  - the build ends with `Finished` (22.98 s here: it recompiles
    `nova-cli` alone, in place, decision 18); `nova version` prints `nova
    0.5.0` and `runtime: embedded`; the test build `Finished` (15.77 s
    here, `nova-cli` again);
  - `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0` and `GNU ld (GNU Binutils
    for Ubuntu) 2.42`;
  - `1 passed; 0 failed; 0 ignored; 0 measured; 73 filtered out`, the
    report `bootstrap: S1 == S2 == S3, SIZE bytes each; N's .ll the same
    from S0, S1 and S2, 12760111 bytes`, the runtime line
    `bootstrap: runtime /tmp/nova-selfhost-bootstrap/home/runtime/0.5.0-CRC/libnova_runtime.a`,
    under the gate's own home and named for 0.5.0, as the gate's runtime
    check requires (ADR 0060), and the stages' times. Here: SIZE 9354688,
    CRC `04a13feb`, `finished in 39.00s` (stages 11.43, 13.54 and 13.87
    s), this machine's figures with Rust 1.97's release runtime, recorded,
    not expected (decision 26); CI's are 9,539,848 and `4f8a6f1b`, the
    same `.ll`;
  - `git status --short` prints nothing.

  Then, in a fresh call without the exports: the disk check, `Task 2 Step
  5, before the plain build`, at least 7,168 MiB; `cargo build --locked
  --workspace`, which ends with `Finished` (0.12 s here, nothing
  compiled), and `target/debug/nova version`, which prints `runtime: not
  embedded`.
- [ ] **Step 6: The package list.** `cargo package --list --locked
  --offline -p nova-cli > "$P/exec56b/package-list.txt" 2>
  "$P/exec56b/package-list.err"; echo "exit $?"`, then `wc -l <
  "$P/exec56b/package-list.txt"`, `wc -c < "$P/exec56b/package-list.err"`
  and `ls target/package`. Expected: `exit 0`, `1759`, `0`, and `ls`
  failing (`No such file or directory`): the list builds nothing, and
  `nova-cli`'s files are 0.4.0's 1,759 (decision 10). CI's
  "Packaged nova" packs and installs them (Step 10).
- [ ] **Step 7: The mutants.** One at a time, each edit is made to the
  committed tree, three checks run, and the file restored with `git
  checkout -- <file>`, `git status --short` checked empty before and
  after. The checks: `V`; `N`, `bash .github/scripts/release-notes.sh
  0.5.0 CHANGELOG.md`; and `R`, the records' check, `python3
  "$P/exec56b/specblocks.py" D` (Task 1, Step 5), which must print
  `specblocks: ok`, then `grep -m2 '^## \[' CHANGELOG.md`, which must
  print `## [Unreleased]` then `## [0.5.0] - D`, `D` the day the tree's
  `## [0.5.0]` carries (decision 19). The replay's runner does it, and
  the executor uses it as given, writing no runner of its own: it holds
  the 14 mutants, refuses to start if `$P/exec56b/mut/` holds anything,
  runs `V`, `N` and `R` on the unbroken tree first (all must pass), and
  writes each result to `$P/exec56b/mut/<n>.json` with its log beside
  it. Write it, as it stands:

```sh
mkdir -p "$P/exec56b"
cat > "$P/exec56b/mutrun.py" <<'EOF'
#!/usr/bin/env python3
r"""mutrun.py: break the release commit's files one way at a time, on the
checked-out tree (the release commit, committed and clean), and see what
catches each. Run from the repository's root, under the process limit, with
$P exported. Every mutant, in order, in one run:
  1. `git status --short` must print nothing;
  2. its edit: its `old` text must occur exactly once in its file ({D} in a
     text is the date the tree's `## [0.5.0]` heading carries, the commit's);
  3. V: cargo test --locked --workspace --no-fail-fast --test release_manifests --test vscode_extension
     N: bash .github/scripts/release-notes.sh 0.5.0 CHANGELOG.md
     R: python3 $P/exec56b/specblocks.py {D}, which must print `specblocks: ok`,
        then grep -m2 '^## \[' CHANGELOG.md, which must print `## [Unreleased]`
        then `## [0.5.0] - {D}`
  4. the file restored with `git checkout -- FILE`, then `git status --short`
     must print nothing again.
V, N and R run on the unbroken tree first, and must pass. A mutant is caught
when V, N or R fails; it is as expected when what failed is its `expect` list
exactly. Results go to $P/exec56b/mut/<id>.json and the logs beside them;
the run refuses to start if mut/ holds anything (after a cut-off run:
restore the tree, move mut/ aside to a fixed name, run again from the first).
"""
import json
import os
import re
import subprocess
import sys
import time

P = os.environ["P"]
OUT = f"{P}/exec56b/mut"
REPO = os.getcwd()
if not os.path.isdir(os.path.join(REPO, ".git")):
    raise SystemExit("run mutrun.py from the repository's root")
ENV = dict(os.environ, CARGO_NET_OFFLINE="true")
V = ["cargo", "test", "--locked", "--workspace", "--no-fail-fast",
     "--test", "release_manifests", "--test", "vscode_extension"]
N = ["bash", ".github/scripts/release-notes.sh", "0.5.0", "CHANGELOG.md"]
R = ["python3", f"{P}/exec56b/specblocks.py"]
H = ["grep", "-m2", r"^## \[", "CHANGELOG.md"]
LEAD = ("**Phase 5, \"Self-hosting\", is released.** This is the `v0.{phase}.0`\n"
        "milestone `nova-spec/00-MASTER-SPEC.md` §7 reserves for a phase. Nova\n"
        "gained what a compiler written in Nova needs: `?`, `+=` on fields and\n"
        "elements, tuples, nested patterns and or-patterns, `Char` literals and\n"
        "ranges, and in std `StringBuilder`, number parsing, `OrderedMap` and\n"
        "`std/process`. That compiler, N, lives in `selfhost/` and is not\n"
        "shipped: the release ships the Rust `nova`, as before. The bootstrap\n"
        "passes its gate in CI: `nova build --release` builds N into S1, S1\n"
        "builds N into S2, and S2 builds N into S3, and S1 == S2 == S3, byte for\n"
        "byte, in `main`'s CI run 45142347493\n"
        "(https://github.com/Sakeerin/nova/actions/runs/45142347493;\n"
        "`docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md`). The boundary\n"
        "Phase 5 closes within will be recorded in\n"
        "`docs/adr/0061-phase-5-boundary.md`. `nova --version` now reports\n"
        "0.5.0.\n\n")
RECORDS = "a record: no test reads it (spec §6); R holds it to the spec's draft"
MUTANTS = [
    dict(key="K1", title="one `=0.4.0` pin left in `Cargo.toml`", file="Cargo.toml",
         old='nova-driver = { path = "crates/nova-driver", version = "=0.5.0" }',
         new='nova-driver = { path = "crates/nova-driver", version = "=0.4.0" }',
         expect=["V: error: failed to select a version for the requirement `nova-driver = \"=0.4.0\"`"]),
    dict(key="K2", title="`[workspace.package]`'s version left at 0.4.0", file="Cargo.toml",
         old='version = "0.5.0"\nedition = "2021"', new='version = "0.4.0"\nedition = "2021"',
         expect=["V: error: failed to select a version for the requirement `nova-diagnostics = \"=0.5.0\"`"]),
    dict(key="L1", title="`Cargo.lock` left at 0.4.0 for `nova-cli`", file="Cargo.lock",
         old='name = "nova-cli"\nversion = "0.5.0"', new='name = "nova-cli"\nversion = "0.4.0"',
         expect=["V: error: cannot update the lock file …/Cargo.lock because --locked was passed to prevent this"]),
    dict(key="E1", title="the extension's `package.json` left at 0.4.0", file="tools/vscode-nova/package.json",
         old='  "version": "0.5.0",', new='  "version": "0.4.0",',
         expect=["the_extension_has_the_workspace_version_and_its_name",
                 "the_extension_has_nova_clis_version_and_its_floor"]),
    dict(key="E2", title="`package-lock.json`'s version left at 0.4.0", file="tools/vscode-nova/package-lock.json",
         old='"name": "vscode-nova",\n  "version": "0.5.0",', new='"name": "vscode-nova",\n  "version": "0.4.0",',
         expect=["the_extension_has_the_workspace_version_and_its_name"]),
    dict(key="E3", title="`package-lock.json`'s `packages[\"\"]` version left at 0.4.0",
         file="tools/vscode-nova/package-lock.json",
         old='"name": "vscode-nova",\n      "version": "0.5.0",', new='"name": "vscode-nova",\n      "version": "0.4.0",',
         expect=["the_extension_has_the_workspace_version_and_its_name"]),
    dict(key="C1", title="the CHANGELOG heading misspelt", file="CHANGELOG.md",
         old="## [0.5.0] - {D}", new="## [v0.5.0] - {D}",
         expect=["N: exit 1: release-notes: CHANGELOG.md has no notes for 0.5.0",
                 "R grep: ## [Unreleased] / ## [v0.5.0] - {D}"]),
    dict(key="C2", title="the fresh, empty `[Unreleased]` left out", file="CHANGELOG.md",
         old="## [Unreleased]\n\n## [0.5.0]", new="## [0.5.0]",
         expect=["R grep: ## [0.5.0] - {D} / ## [0.4.0] - 2026-10-23"], note=RECORDS),
    dict(key="C3", title="the lead paragraph left out", file="CHANGELOG.md",
         old=LEAD, new="", expect=["R specblocks: block 1 (15 lines) in CHANGELOG.md: FOUND 0 TIMES"],
         note=RECORDS + "; the notes still hold the list of sub-phases, so release-notes.sh prints them"),
    dict(key="C4", title="5.2b left out of the list of sub-phases", file="CHANGELOG.md",
         old="§3; ADR 0053), and 5.2b, packages, the WASM target and test mode in the\n"
             "resolution dump (§4; ADR 0054). Phase 5.3a, the self-hosted item\n"
             "collection and the HIR dump (spec\n",
         new="§3; ADR 0053). Phase 5.3a, the self-hosted item collection and the HIR\ndump (spec\n",
         expect=["R specblocks: the 5.2b sentence in CHANGELOG.md: NOT FOUND"], note=RECORDS),
    dict(key="X1", title="the extension's CHANGELOG section left out", file="tools/vscode-nova/CHANGELOG.md",
         old="## [0.5.0] - {D}\n\nReleased with Nova 0.5.0; no change to the extension.\n\n", new="",
         expect=["R specblocks: block 2 (3 lines) in tools/vscode-nova/CHANGELOG.md: FOUND 0 TIMES"],
         note=RECORDS + "; the stores show it, and Task 3 reads both listings"),
    dict(key="R1", title="the README's `### Self-hosting` section left out", file="README.md",
         old="### Self-hosting\n\n`selfhost/` holds N, Nova's compiler written in Nova. It is not shipped:\n"
             "a release ships the Rust `nova`, and N stays in the repository. On every\n"
             "pull request, on Linux, CI builds N with `nova build --release` into S1,\n"
             "then N with S1 into S2 and with S2 into S3, and holds S1 == S2 == S3,\n"
             "byte for byte. [`selfhost/README.md`](selfhost/README.md) says how to\n"
             "build and run it; [ADR 0060](docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md)\n"
             "records the gate.\n\n",
         new="", expect=["R specblocks: block 3 (9 lines) in README.md: FOUND 0 TIMES"], note=RECORDS),
    dict(key="D1", title="the runbook's note on the token left out", file="docs/releasing.md",
         old="\n   **Amended {D} (branch `claude/loving-archimedes-frtf5q-56b`):**\n"
             "   `v0.5.0` adds no crate: its 20 are new versions of crates on\n"
             "   crates.io, within the burst of 30, so its token takes\n"
             "   **publish-update** alone (spec\n"
             "   `docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`,\n"
             "   the user's decision 5).\n",
         new="", expect=["R specblocks: block 4 (6 lines) in docs/releasing.md: FOUND 0 TIMES"],
         note=RECORDS + "; Task 3 gives the token's scope itself"),
    dict(key="D2", title="the runbook's note on step 6.2 left out", file="docs/releasing.md",
         old="**Amended {D} (branch `claude/loving-archimedes-frtf5q-56b`):** for\n"
             "`v0.5.0`, by the user's decision of 2026-11-23, the maintainer installs\n"
             "the published `nova` and tries it in place of this run, as for\n"
             "`v0.4.0` (spec\n"
             "`docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`,\n"
             "decision 5). `docs/adr/0061-phase-5-boundary.md` will record the try.\n\n",
         new="", expect=["R specblocks: block 5 (6 lines) in docs/releasing.md: FOUND 0 TIMES"],
         note=RECORDS + "; Task 3 gives the try itself"),
]


def sh(cmd, log):
    t = time.time()
    r = subprocess.run(cmd, cwd=REPO, env=ENV, capture_output=True, text=True, errors="replace", timeout=300)
    out = r.stdout + r.stderr
    log.write(f"$ {' '.join(cmd)}\n{out}\n[exit {r.returncode}, {time.time() - t:.1f} s]\n\n")
    return r.returncode, out


def clean():
    return subprocess.run(["git", "status", "--short"], cwd=REPO, capture_output=True,
                          text=True).stdout.strip() == ""


def observe(log, date):
    seen = []
    rc, out = sh(V, log)
    failed = re.findall(r"^test (\S+) \.\.\. FAILED", out, re.M)
    seen += failed
    if rc != 0 and not failed:
        err = next((l for l in out.splitlines() if l.startswith("error")), f"exit {rc}")
        seen.append("V: " + err.replace(REPO, "…"))
    rc, out = sh(N, log)
    if rc != 0:
        seen.append(f"N: exit {rc}: " + out.strip().splitlines()[-1])
    elif not out.strip():
        seen.append("N: no notes")
    rc, out = sh(R + [date], log)
    if rc != 0:
        bad = [l for l in out.splitlines() if l and not l.endswith(": found") and l != "specblocks: FAILED"]
        seen.append("R specblocks: " + "; ".join(bad or [f"exit {rc}"]))
    rc, out = sh(H, log)
    heads = out.strip().splitlines()
    if heads != ["## [Unreleased]", f"## [0.5.0] - {date}"]:
        seen.append("R grep: " + " / ".join(heads).replace(date, "{D}"))
    return seen


def main():
    os.makedirs(OUT, exist_ok=True)
    if os.listdir(OUT):
        raise SystemExit(f"{OUT} is not empty: move it aside to a fixed name first")
    if not clean():
        raise SystemExit("the tree is not clean")
    dates = re.findall(r"^## \[0\.5\.0\] - (\d{4}-\d{2}-\d{2})$", open(os.path.join(REPO, "CHANGELOG.md"),
                       encoding="utf-8").read(), re.M)
    if len(dates) != 1:
        raise SystemExit(f"CHANGELOG.md has {len(dates)} [0.5.0] headings")
    date = dates[0]
    print(f"the release commit's date: {date}", flush=True)
    with open(f"{OUT}/0-baseline.log", "w") as log:
        base = observe(log, date)
    print(f"baseline: {'V, N and R pass' if not base else 'FAILS: ' + str(base)}", flush=True)
    if base:
        raise SystemExit(1)
    total = caught = expected = 0
    for k, m in enumerate(MUTANTS, 1):
        if not clean():
            raise SystemExit("the tree is not clean")
        path = os.path.join(REPO, m["file"])
        text = open(path, encoding="utf-8").read()
        old, new = m["old"].replace("{D}", date), m["new"].replace("{D}", date)
        n = text.count(old)
        if n != 1:
            raise SystemExit(f"{k} {m['key']}: its text occurs {n} times in {m['file']}")
        t0 = time.time()
        log = open(f"{OUT}/{k}.log", "w")
        try:
            open(path, "w", encoding="utf-8").write(text.replace(old, new))
            seen = observe(log, date)
        finally:
            subprocess.run(["git", "checkout", "--", m["file"]], cwd=REPO, check=True)
            log.close()
        if not clean():
            raise SystemExit(f"{k} {m['key']}: the tree is not clean after it")
        res = dict(id=k, key=m["key"], title=m["title"], file=m["file"], observed=seen,
                   caught=bool(seen), as_expected=sorted(seen) == sorted(m["expect"]),
                   seconds=round(time.time() - t0, 1))
        json.dump(res, open(f"{OUT}/{k}.json", "w"), indent=1)
        total += 1
        caught += res["caught"]
        expected += res["as_expected"]
        print(f"{k} {m['key']}: {'CAUGHT' if res['caught'] else 'NOT CAUGHT'} "
              f"{'as expected' if res['as_expected'] else 'NOT AS EXPECTED'} {res['seconds']} s {seen}", flush=True)
    print(f"mutants: {total}, caught {caught}, not caught {total - caught}, as expected {expected} of {total}")
    sys.exit(0 if expected == total else 1)


main()
EOF
```

  Then the disk check (`Task 2 Step 7, before the mutants`, at least 6,451
  MiB), and from the repository's root, the first command with a tool
  timeout of 300000 (the run takes about 40 s; `timeout 280` stops it
  inside the tool's limit), its output in a log read after it:

  ```sh
  time timeout 280 python3 "$P/exec56b/mutrun.py" > "$P/exec56b/mutrun.txt" 2>&1; echo "exit $?"
  cat "$P/exec56b/mutrun.txt"
  git status --short
  sha256sum Cargo.lock
  ```

  Expected, in 38 s here (each mutant 2 to 3 s; C1 0.3 s): `exit 0`;
  `the release commit's date: D`, `baseline: V, N and R pass`, one line
  per mutant, each `CAUGHT as expected`, then `mutants: 14, caught 14,
  not caught 0, as expected 14 of 14`; `git status --short` empty; the
  sha256 `f47897873c81b571…`. A runner stopped by `timeout` exits 124,
  and one that finds a mutant not as expected exits 1: either stops the
  step. The mutants and what each must show:

  1. **K1: One `=0.4.0` pin left in `Cargo.toml`.** `Cargo.toml`: `nova-driver = { path = "crates/nova-driver", version = "=0.5.0" }` becomes `… version = "=0.4.0" }`.
     - Expected: caught: `V` stops before any test: `` error: failed to select a version for the requirement `nova-driver = "=0.4.0"` ``.
  2. **K2: `[workspace.package]`'s version left at 0.4.0.** `Cargo.toml`: `[workspace.package]`'s `version = "0.5.0"` becomes `version = "0.4.0"`.
     - Expected: caught: `V` stops before any test: `` error: failed to select a version for the requirement `nova-diagnostics = "=0.5.0"` ``.
  3. **L1: `Cargo.lock` left at 0.4.0 for `nova-cli`.** `Cargo.lock`: `nova-cli`'s `version = "0.5.0"` becomes `version = "0.4.0"`.
     - Expected: caught: `V` stops before any test: `error: cannot update the lock file …/Cargo.lock because --locked was passed to prevent this`.
  4. **E1: The extension's `package.json` left at 0.4.0.** `tools/vscode-nova/package.json`: `"version": "0.5.0",` (line 5) becomes `"version": "0.4.0",`.
     - Expected: caught: `the_extension_has_the_workspace_version_and_its_name` fails; `the_extension_has_nova_clis_version_and_its_floor` fails.
  5. **E2: `package-lock.json`'s version left at 0.4.0.** `tools/vscode-nova/package-lock.json`: the top `"version": "0.5.0",` (line 3) becomes `"0.4.0"`.
     - Expected: caught: `the_extension_has_the_workspace_version_and_its_name` fails.
  6. **E3: `package-lock.json`'s `packages[""]` version left at 0.4.0.** `tools/vscode-nova/package-lock.json`: `packages[""]`'s `"version": "0.5.0",` (line 9) becomes `"0.4.0"`.
     - Expected: caught: `the_extension_has_the_workspace_version_and_its_name` fails.
  7. **C1: The CHANGELOG heading misspelt.** `CHANGELOG.md`: `## [0.5.0] - D` becomes `## [v0.5.0] - D`, `D` the commit's day.
     - Expected: caught: `N` fails: `exit 1: release-notes: CHANGELOG.md has no notes for 0.5.0`; `R`'s grep prints `## [Unreleased]` then `## [v0.5.0] - D`.
  8. **C2: The fresh, empty `[Unreleased]` left out.** `CHANGELOG.md`: the fresh `## [Unreleased]` and the blank line after it go.
     - Expected: caught: `R`'s grep prints `## [0.5.0] - D` then `## [0.4.0] - 2026-10-23`. A record: no test reads it (spec §6); `R` holds it to the spec's draft.
  9. **C3: The lead paragraph left out.** `CHANGELOG.md`: the lead paragraph (15 lines) and the blank line after it go.
     - Expected: caught: `R`'s `specblocks.py` fails: `block 1 (15 lines) in CHANGELOG.md: FOUND 0 TIMES`. A record: no test reads it (spec §6); `R` holds it to the spec's draft; the notes still hold the list of sub-phases, so release-notes.sh prints them.
  10. **C4: 5.2b left out of the list of sub-phases.** `CHANGELOG.md`: the 5.2b sentence goes: its three lines become the base's two.
      - Expected: caught: `R`'s `specblocks.py` fails: `the 5.2b sentence in CHANGELOG.md: NOT FOUND`. A record: no test reads it (spec §6); `R` holds it to the spec's draft.
  11. **X1: The extension's CHANGELOG section left out.** `tools/vscode-nova/CHANGELOG.md`: the `## [0.5.0] - D` heading, its line and their blank lines go.
      - Expected: caught: `R`'s `specblocks.py` fails: `block 2 (3 lines) in tools/vscode-nova/CHANGELOG.md: FOUND 0 TIMES`. A record: no test reads it (spec §6); `R` holds it to the spec's draft; the stores show it, and Task 3 reads both listings.
  12. **R1: The README's `### Self-hosting` section left out.** `README.md`: the `### Self-hosting` subsection (9 lines) and its blank line go.
      - Expected: caught: `R`'s `specblocks.py` fails: `block 3 (9 lines) in README.md: FOUND 0 TIMES`. A record: no test reads it (spec §6); `R` holds it to the spec's draft.
  13. **D1: The runbook's note on the token left out.** `docs/releasing.md`: the note for `v0.5.0` in step 3's item 4 (6 lines) and the blank line before it go.
      - Expected: caught: `R`'s `specblocks.py` fails: `block 4 (6 lines) in docs/releasing.md: FOUND 0 TIMES`. A record: no test reads it (spec §6); `R` holds it to the spec's draft; Task 3 gives the token's scope itself.
  14. **D2: The runbook's note on step 6.2 left out.** `docs/releasing.md`: the note for `v0.5.0` in step 6.2 (6 lines) and the blank line after it go.
      - Expected: caught: `R`'s `specblocks.py` fails: `block 5 (6 lines) in docs/releasing.md: FOUND 0 TIMES`. A record: no test reads it (spec §6); `R` holds it to the spec's draft; Task 3 gives the try itself.

  If the run is cut off: `git checkout -- <the mutant's file>`, `git
  status --short` empty, `rm -rf "${P:?}/exec56b/mut-cut" && mv
  "$P/exec56b/mut" "$P/exec56b/mut-cut"`, then the run again from the
  first. All 14 results come from one run, at one tip.
- [ ] **Step 8: The review's inputs.** The full run's figures, the lists'
  comparison, Step 2's diff checks, the notes' size and diff, `SH`'s time,
  the gate's reports on both runtimes, the golden suite, the package list,
  and the mutants, each against its Expected, go in the ledger.
- [ ] **Step 9: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec
    and its amendment, the runbook and this plan, and is told to hunt for:
    - a change to `Cargo.lock` beyond its 23 workspace `version` lines, a
      version left at 0.4.0 in any of the four files, a manifest changed
      otherwise (Review Focus 1 and 2);
    - a CHANGELOG `[0.5.0]` that `release-notes.sh` cannot find, or whose
      section is more than the base's `[Unreleased]` with the lead and
      5.2b (Review Focus 3);
    - the records against the spec's drafts (§3.2 to §3.5): the lead's
      words and its claims (run 45142347493, ADR 0060's path, ADR 0061 to
      come, "is not shipped"), the fresh `[Unreleased]`, 5.2b's place,
      the extension's section above `[0.4.0]`, the README's subsection
      after `### Web` and its two links, each runbook note's place and
      indentation, and anything else the release leaves stale (§3.6)
      (Review Focus 4);
    - the date: every dated place the commit's day, and nothing else
      dated anew (Review Focus 5; decision 15);
    - the gate after the bump: its runtime under `0.5.0-<crc>`, sizes
      recorded and not expected (Review Focus 6; decision 21);
    - CI's Expected (Step 10) against run 45316548924 and `main`'s
      45142347493, and the steps after the merge (Task 3) against the
      runbook and spec §5: each of the user's steps and each write a
      stop, each check the runbook's (Review
      Focus 7 and 8);
    - the mutants (Step 7): whether each is caught by what should catch
      it, and any breakage of the records that `R` would not catch;
    - a local path, a person's name, a model ID or a scratch file in the
      diff or the plan.
  - Each finding is either fixed, or answered in the ledger with the
    reason it stands. The fixes are folded into the release commit
    before anything is pushed (decision 20), the files changed in the
    working tree first:
    - **on the commit's own day** (`TZ=UTC git log -1
      --date=format-local:%Y-%m-%d --format=%ad` prints today's `date -u
      +%F`): `git add -A`, `git status --short` listing only the changed
      files, then `git commit --amend --no-edit`, which keeps the date and
      the message;
    - **on another day:** the re-date of Task 1, Step 7 (decision 15),
      `redate.sh` with the new day, `specblocks.py`, the grep, `git add
      -A`, `git status --short` listing the changed files and the three
      dated ones, then `git commit --amend --date=now -F
      "$P/exec56b/msg-t1.txt"`;
    - then Task 1, Step 7's checks again from its `D=` line: `dated D`,
      the trailers, and the comparison with `4f55d6ed`, which now also
      shows the fixes' lines (and, on another day, the dated lines).
  - **After the fold, the records' check again,** whatever the fixes
    changed, as Task 1, Step 5 gives it, `D` the day the records carry:

    ```sh
    D=$(sed -n 's/^## \[0\.5\.0\] - \([0-9-]*\)$/\1/p' CHANGELOG.md); echo "$D"
    python3 "$P/exec56b/specblocks.py" "$D"
    grep -m2 '^## \[' CHANGELOG.md
    ```

    Expected: the day; `specblocks: ok`, each block `found`; `##
    [Unreleased]`, then `## [0.5.0] - D`. A fix that changes a record so
    that it no longer holds a spec draft stops the work: the draft or the
    fix is wrong, and the user is asked.
  - Then the conventions' checks again in full, then Steps 1 to 6.
  - **Then every mutant again, all 14, if the fold changed any of the
    eight files** of Task 1, Step 5 (each mutant's file is one of them; a
    fold on another day changes three): all mutant results come from one
    tip. Before the re-run, `rm -rf "${P:?}/exec56b/mut-before-fixes" &&
    mv "$P/exec56b/mut" "$P/exec56b/mut-before-fixes"` (the runner refuses
    a `mut/` that holds anything); then Step 7 again as it is written.
    Expected: 14, all caught, each as Step 7 gives it, unless a fix
    changed a mutant's text (its `old` must then be updated in the runner,
    and the change recorded). A mutant whose result changes is a finding
    for the ledger, with the fix that changed it or the reason it stands.
    If the fold changed none of the eight, Step 7's results stand, and the
    ledger says so.
- [ ] **Step 10: The PR.**
  - **Stop here, and have the main session ask the user** whether to push
    the branch and open the PR (Global Constraints). No push of this
    branch has been allowed: the user's answer of 2026-11-24 covered only
    the prototype's `…-56b-ci` and draft Sakeerin/nova#150. Nothing is
    pushed, and no PR opened, before their word; record their answer in
    the ledger.
  - Before asking, check that every commit the PR would carry ends with
    the two trailer lines of the Conventions: `git log --format=%B
    origin/main..HEAD`, its commits `3f72bd38` (the spec's), the plan's
    and the release commit, last.
  - On the user's word: push, `git push -u origin
    claude/loving-archimedes-frtf5q-56b`, then open the PR to `main`, not
    as a draft (the main session, with its GitHub tools), with the
    repository's template headings (`## Summary`,
    `## Changes`, `## Testing`, `## Phase Gate Criteria`). The body names
    no local path, no person and no model ID. It ends with this line, a
    blank line, and the session's address, exactly:

    ```
    🤖 Generated with [Claude Code](https://claude.com/claude-code)

    https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
    ```

    If the server replaces that ending with its own footer (it did on #149
    and #150), send the body once more with the ending, and check it.
  - The body holds:
    - **Summary:** the release `v0.5.0`, by `docs/releasing.md`; the
      release commit last; what a user sees: `nova --version` prints
      0.5.0, nothing else changes; the steps after the merge, each on
      the user's word (Task 3), and the records pull request with ADR 0061
      after them;
    - its commits: `3f72bd38`, the spec's; the plan's, with the spec's
      amendment (decision 22); the release commit;
    - **Changes:** the four version files and their counts (21, 23, 1, 2
      lines; the lock's 23 workspace `version` lines alone, read line by
      line); the CHANGELOG's `[0.5.0]`, its lead, 5.2b and the fresh
      `[Unreleased]`; the extension's section; the README's subsection;
      the runbook's two notes; the date (decision 15);
    - **Testing:** the template's boxes ticked for what ran (`cargo test
      --workspace`, `--test selfhost`, `cargo fmt`, both clippy
      commands), "New tests added" not ticked, with the reason (spec §6:
      the existing tests hold the versions; decision 14's red and green),
      and "CHANGELOG.md updated" ticked; then the figures: the suite
      `2837 1 10` here, the IPv6 test the one failure; `selfhost.rs` `71 0
      3`; the gate on both runtimes, sizes as this machine's (decision
      26); the golden suite `147 passed, 9 excluded`; the notes' 614 lines
      and 40,612 characters; the package list 1,759; the mutants, 14,
      all caught, the 7 records by the plan's check of them against the
      spec's drafts (decision 19); what CI measured on the
      prototype, run 45316548924 and Release run 45316548687 on
      `4f55d6ed`, the tree of this pull request's last commit but for its
      date (decision 12);
    - the user's decisions and this plan's, numbered as here, with
      decisions 14, 16 and 23 marked as differing, each recorded in the
      spec's amendment;
    - the review's findings and what came of each;
    - **Phase Gate Criteria:** spec §13's six items, and which this pull
      request meets on its merge (the first); the others are Task 3's.
  - **Wait for CI, then read it** (Conventions), with no foreground
    `sleep`: once the pull request is open, the main session subscribes
    to its activity (`subscribe_pr_activity`), and CI's results wake the
    session. When the CI run and the Release run have ended, a read-only
    helper agent fetches each needed job's log with `get_job_logs` into
    `$P/exec56b/ci/` and writes a summary of each run,
    `$P/exec56b/ci-<run>.txt`: each job's conclusion, the summed `test
    result:` counts of each Test job, the gate's `bootstrap:` lines, and
    the step lines named below. It writes nothing to GitHub. **Expected,**
    checked against those summaries, from run 45316548924 and `main`'s
    45142347493 (decision 21):
    - **every job green:** CI's 19 (Test, Selfhost, Packaged nova,
      Installed nova and Clippy, each on three systems; Browser
      (Chromium), VS Code extension, MSRV (1.78), Rustfmt) and Release's
      eight, which the pull request starts by changing `Cargo.toml`,
      `Cargo.lock` and `tools/vscode-nova/**` (`release.yml:13-26`): its
      four builds, "Write std's documentation site", "Package the VS Code
      extension", "Prepare the release" green, and "Publish the release"
      **skipped** (`release.yml:378`), as on #150;
    - **Release:** the extension packed as `nova-vscode-0.5.0.vsix`; the
      archives `nova-0.5.0-x86_64-unknown-linux-gnu.tar.gz`,
      `nova-0.5.0-x86_64-apple-darwin.tar.gz`,
      `nova-0.5.0-aarch64-apple-darwin.tar.gz` and
      `nova-0.5.0-x86_64-pc-windows-msvc.zip`; std's site
      `nova-std-docs-0.5.0.tar.gz`; "Prepare the release": the count check
      passed, `SHA256SUMS` over those six, `install.sh` and `install.ps1`
      (8 files), `## [0.5.0]` found (the step passes only so), and the
      line `publish runs: gh release create v0.5.0 --verify-tag --title
      "Nova 0.5.0" --notes-file notes.md dist/*`, with no `--prerelease`;
      the Linux archive installed with `install.sh` prints `nova 0.5.0`
      and `runtime: embedded`;
    - **Packaged nova,** on each system: `package-gate: 20 crates at
      0.5.0`, every crate `*-0.5.0.crate`, `nova-cli-0.5.0.crate` with
      1,759 files, "every packaged Cargo.lock agrees with the
      workspace's", the installed `nova` printing `nova 0.5.0` and
      `runtime: embedded`, and `package-gate: passed`. Recorded, not
      expected: `nova-cli`'s bytes (584,057 on Linux and macOS, 586,939 on
      Windows on run 45316548924) and the 20 packages' (2,119,078 and
      2,130,553);
    - **the Test job's main step,** `cargo test --locked --workspace
      --all-features --no-fail-fast`, its 111 `test result:` lines summed:
      `2838 passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9` on macOS,
      `2815, 0, 9` on Windows; in it `release_manifests` `8 passed`,
      `vscode_extension` `3 passed`, `project` `25 passed`; on Linux `test
      release_golden_suite ... ok` and no `tests/selfhost.rs`. The advisory
      `-- --ignored` step after it is not counted; what it shows is
      recorded (on the prototype's run 45316548924 and on #149's pull
      request run 45139701894 alike, Linux's `extern_ffi_run` and three of
      macOS's GC tests failed there);
    - **the Selfhost job:** `selfhost.rs` `71 passed; 0 failed; 3
      ignored` on Linux, `69`, `3` on macOS, `65`, `3` on Windows, under
      the 80, 100 and 110 s lines (run 45316548924: 53.90, 80.55 and 73.06
      s; `main`'s: 55.57, 31.18 and 108.49 s); on Linux alone the golden
      step `2 passed; 0 failed; 0 ignored; 0 measured; 72 filtered out`,
      the bootstrap step printing `cc`'s and `ld`'s versions,
      `bootstrap: S1 == S2 == S3, …`, its runtime line under the gate's
      `home/runtime/0.5.0-<crc>/`, and `1 passed; 0 failed; 0 ignored; 0
      measured; 73 filtered out` (a `0 passed` there means the step runs
      no test: stop and report), then `the_selfhost_tests_pass` `1
      passed`; on macOS and Windows the clang steps skipped. Recorded, not
      expected: S1's size (9,539,848 on run 45316548924), the runtime's
      directory (`0.5.0-4f8a6f1b` there) and the step's time (26.66 s
      there, under its 75 s line);
    - **MSRV (1.78):** `cargo check --locked --workspace` under `rustc
      1.78.0`, `Finished`;
    - Installed nova, Browser (Chromium), VS Code extension, Clippy and
      Rustfmt: green.

    Record each job's time, the run's address, the figures above and the
    Release run's in the ledger, and compare them with run 45316548924's
    in the PR's body. **If a count differs,** or a job fails here and did
    not on #150, record it, stop and ask the user; never lower a count.
    **If `selfhost.rs` takes more than 110 s on Windows, 100 s on macOS or
    80 s on Linux, or the bootstrap step more than 75 s,** ask the user
    (spec §10, risk 6).
  - **After the push, nothing is amended or force-pushed without the
    user's word** (decision 20). If CI, the review or anything else needs
    a change to the pushed branch, the executor stops, records why, and
    the main session asks the user, offering both of decision 20's ways:
    amend the release commit and force-push with lease, pinned to the
    pushed head, which keeps the release commit last; or a new commit
    after it, which reaches none of the release, its notes and the
    crates, all built from the tag's tree, `REL`'s: a fix to the records
    must take the first way to reach the release (decision 20). Each
    push after the first is on the user's word for it, and Steps 1 to 10
    run again on the new head as far as the change reaches.
- [ ] **Step 11: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit
    (`expectedHeadSha`). Never auto-merge. The runbook's **[stop]**
    (`docs/releasing.md:186-189`).
  - Then `git fetch origin main`, `git diff <head> origin/main --stat`,
    which prints nothing: the merged tree is the PR's; and `git log -3
    --format='%H %s' origin/main`, which prints, newest first, the
    release commit, the plan's and the spec's, each with a new hash: the
    rebase gives each commit one. **`main`'s release commit** is the
    commit on `main` whose subject is `release: v0.5.0, Phase 5
    released`, after the merge, taken by its full 40-digit hash: here the
    first line's. Record it in the ledger as `REL`, with `main`'s other
    hashes; the tag must name it (Task 3, Step 2). Had a commit followed
    the release commit (decision 20's second way), it would print first,
    and `REL` would still be the release commit's hash, not `main`'s tip.
  - Then CI's run on `main`, once it ends, accepted by its `conclusion`
    and its head commit's tree (Task 3, Step 1).
  - **Only now** may the 0.4.0 set's stale executables in `target/debug/`
    be removed, by literal paths or `"${VAR:?}"` forms, never `cargo
    clean` and never anything under `/tmp/nova-*`; each removal and the
    disk before and after go in the ledger.

---

## After the merge

### Task 3: The steps after the merge, by `docs/releasing.md`

Spec §5, in its order; decisions 7 and 24. The steps marked **[stop]**
wait for the user's explicit word in this conversation, and a word given
for one step does not cover the next; Steps 1, 3 and 10 are the main
session's reads and checks, which need no word. Between two stops
is preparation that can be redone. The runbook's lines are cited as `main`
has them after the merge (decision 24); read each section before its step.
**The user** pushes the tag, makes the token and the secret, approves the
deployment, uploads to the stores and tries the published `nova`; **the
agent** starts the workflows and checks what each step made.

**The user's commands** run at their own prompt from the root of an
up-to-date checkout of `main`, after `git fetch origin` and `git checkout
main && git pull --ff-only` (or as the runbook says), with
`version=0.5.0` set first: the runbook's commands read `$version`, and
the runbook sets it only once, at `docs/releasing.md:21`, to `0.3.0`, as
an example.

**The agent here is the main session itself, not a subagent,** and it
works through its GitHub tools, each call named below with its arguments;
every call takes owner `Sakeerin` and repo `nova`. The session has no
`gh` and runs no `curl`: what its tools cannot do is the user's command,
the runbook's, run at the user's own prompt, with the output pasted back
into the conversation. Every write the session makes (a dispatch, a
cancel, a re-run) is made on the user's word for that step, and the ledger
records it with the run's address; a refused write (HTTP 403 or the like)
goes back to the user, who runs the runbook's line for it, and is never
worked around. A run is waited for as the Conventions give it, never
with a foreground `sleep`: one scheduled check-in in this session
(`send_later`) for about the run's expected time, which each step gives
with its source; when it fires, `actions_get` (`method:
get_workflow_run`, `resource_id` the run's id), and another check-in if
its `status` is not `completed`; or the user says when the run is done.
A run's logs are fetched by a read-only helper agent with `get_job_logs`
into `$P/exec56b/ci/` and summarised in `$P/exec56b/ci-<run>.txt`
(Conventions); each check that reads a log is made against that
summary, and the main session reads no whole log. Every word of the
user's, every run's address and every check's output go in the ledger. A
failed step follows the runbook's §7 (`docs/releasing.md:670-748`); a fix
in the repository is `0.5.1` (spec §5).

- [ ] **Step 1: `main`'s CI on the merged commits.** Read CI's run on
  `main` once it ends (spec §5, row 1): `actions_list` (`method:
  list_workflow_runs`, `resource_id: ci.yml`, `workflow_runs_filter:
  {"branch": "main", "event": "push"}`) gives the run whose `head_sha` is
  `main`'s tip (Task 2, Step 11); one check-in about 13 minutes on
  (`main`'s run 45142347493, at 0.4.0, took 12 min 33 s; #150's CI run
  10 min 28 s), then `actions_get` (`get_workflow_run`) as above; and
  `actions_list` (`method: list_workflow_jobs`, `resource_id` the run's
  id) lists its jobs. **Accepted** by its `conclusion`, `success`, all
  19 jobs `success`, and by its head commit's tree being the pull
  request's last head's: after `git fetch origin main`, `git rev-parse
  <head_sha>^{tree}` and `git rev-parse <the last head>^{tree}` print
  the same hash, as 5.6a accepted `main`'s run after #149's merge. Its
  counts are read only if a job fails or the trees differ: then the
  helper agent summarises its logs, and the counts must be Task 2, Step
  10's (the Test job `2838 0 10`, `2838 0 9`, `2815 0 9`; `selfhost.rs`
  `71 0 3`, `69 0 3`, `65 0 3`; the bootstrap step `1 passed`). The tag
  waits for it
  (`docs/releasing.md:186-189`, and the note of 2026-10-14 at
  `:222-224`: `v0.3.2` was pushed before `main`'s CI had finished).
  Record the run's address (`html_url`).
- [ ] **Step 2: The tag.** **[stop]** The user makes the annotated tag on
  `main`'s release commit and pushes it (spec §3.7; the runbook's §2,
  `:191-221`): this session's credential cannot push a tag (spec §2). Give
  them these commands in full, from a checkout whose `origin` is
  `https://github.com/Sakeerin/nova.git`: spec §3.7's, with the subject
  checked before the tag is made, as `v0.4.0`'s plan gave them (decision
  20), and a last line that shows what `origin` holds:

  ```sh
  git fetch origin main
  git log -1 --format='%H %s' origin/main   # release: v0.5.0, Phase 5 released
  commit=$(git rev-parse origin/main)
  test "$(git log -1 --format=%s "$commit")" = "release: v0.5.0, Phase 5 released" &&
    git tag -a v0.5.0 -m "Nova v0.5.0 — Phase 5 (Self-hosting)" -m "The release of Phase 5. N, the Nova compiler written in Nova, lives in selfhost/ and is not shipped; the bootstrap passes its gate in CI (docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md). docs/adr/0061-phase-5-boundary.md will record the boundary." "$commit" &&
    git push origin v0.5.0
  git ls-remote origin 'refs/tags/v0.5.0*'
  ```

  The second command prints the release commit as rebased onto `main`,
  `REL` (Task 2, Step 11); the fourth makes and pushes the tag only when
  that commit's subject is the release commit's, and otherwise makes
  none. The last prints two lines once the tag is pushed, the tag object's
  hash with `refs/tags/v0.5.0` and `REL` with `refs/tags/v0.5.0^{}`, and
  nothing if no tag was pushed: then stop, and ask. Had a commit followed
  the release commit on `main` (decision 20's second way), the third line
  is `commit=REL`, the full hash, in its place. **Checked after, by the
  agent,** once the user says it is pushed:
  - `get_tag` (`tag: v0.5.0`) returns a tag object: an annotated tag,
    whose own `sha` is not `REL`; its `object.type` is `commit` and its
    `object.sha` equals `REL`, the full hash; its `message` is spec
    §3.7's two paragraphs, `Nova v0.5.0 — Phase 5 (Self-hosting)`, a blank
    line, then `The release of Phase 5. …`. A lightweight tag has no tag
    object: if the call fails, `list_tags` shows whether `v0.5.0` exists
    and the commit it names; either way, stop and ask;
  - `actions_list` (`method: list_workflow_runs`, `resource_id:
    release.yml`, `workflow_runs_filter: {"event": "push"}`) lists a
    Release run whose `head_branch` is `v0.5.0` and whose `head_sha` is
    `REL`.
- [ ] **Step 3: `release.yml` on the tag, and the GitHub release.** It
  starts by itself on the tag (`docs/releasing.md:226-232`); the agent
  waits for it with one check-in about 6 minutes on (`v0.4.0`'s Release
  run on its tag, 40063515984, took 5 min 23 s; #150's 5 min 50 s).
  **Checked after** (`:234-259`):
  - `actions_list` (`method: list_workflow_jobs`, the run's id): the
    eight jobs `success`, "Publish the release" among them this time;
  - the helper agent's summary of "Prepare the release"'s log
    (`get_job_logs`, its `job_id`): the eight `SHA256SUMS` lines it prints
    (`release.yml:325`, `cat SHA256SUMS`), each `<sum>  <file>` past the
    log's timestamp, one for each file below but `SHA256SUMS`;
  - `get_release_by_tag` (`tag: v0.5.0`): `name` `Nova 0.5.0`, `draft`
    `false`, `prerelease` `false` (the version has no `-`), and nine
    assets, the eight files and `SHA256SUMS` (`:242-255`):
    `nova-0.5.0-aarch64-apple-darwin.tar.gz`,
    `nova-0.5.0-x86_64-apple-darwin.tar.gz`,
    `nova-0.5.0-x86_64-pc-windows-msvc.zip`,
    `nova-0.5.0-x86_64-unknown-linux-gnu.tar.gz`,
    `nova-std-docs-0.5.0.tar.gz`, `nova-vscode-0.5.0.vsix`, `install.sh`,
    `install.ps1`, `SHA256SUMS`. **The sums** (spec §13's "every sum
    `OK`"): each of the eight files' `digest`, `sha256:` and its hex
    sum, equals that file's line of the sums "Prepare the release"
    printed. If the release gives no `digest`, the user runs the
    runbook's download and check (`:236-238`, the block at `:234-240`)
    at their own prompt and pastes the output: eight lines, each `OK`;
  - the release's `body`: its first line that is not blank is the
    lead's, `**Phase 5, "Self-hosting", is released.** This is the
    `v0.{phase}.0``: the notes are `[0.5.0]`'s section (`:258-259`);
  - `get_latest_release`, in place of the runbook's `releases/latest`
    check (`:239`, `:257-259`): `tag_name` `v0.5.0`, `html_url`
    `https://github.com/Sakeerin/nova/releases/tag/v0.5.0`.

  Record the run's address. If the run fails, the runbook's §7
  (`:672-688`): a passing failure is re-run, by `actions_run_trigger`
  (`method: rerun_failed_jobs`, `run_id`) on the user's word, in place of
  the runbook's `gh run rerun` (`:673-674`); a wrong archive is fixed
  before step 3, with the release and the tag deleted by the user, at
  their own prompt, by the runbook's commands (`:680-684`).
- [ ] **Step 4: The crates.io token and the secret.** **[stop]** The user
  (`docs/releasing.md:276-289`, and its note for `v0.5.0` at `:336-341`):
  - a token at crates.io, Account Settings → API Tokens → New Token: a
    name such as `nova v0.5.0`; expiring the next day; the scope
    **publish-update** only, since `v0.5.0` adds no crate (decision 7);
    the crates `nova-*` (`:276-281`);
  - the environment's secret `CARGO_REGISTRY_TOKEN`, in Settings →
    Environments → `crates-io`, or by the runbook's `gh secret set` line
    at their own prompt (`:282-289`).

  The agent reads, prints and stores no token. **Checked after:** the
  user's word that both are made; nothing else (the session could not read
  the environment's secrets, spec §10 risk 4).
- [ ] **Step 5: "Publish to crates.io".** **[stop]** On the user's word,
  the agent starts the workflow from the tag (`docs/releasing.md:291-296`)
  with `actions_run_trigger`: `method: run_workflow`, `workflow_id:
  publish-crates.yml`, `ref: v0.5.0`, `inputs: {"version": "0.5.0"}`
  (the workflow's one input, `version`, required,
  `publish-crates.yml:29-34`). Then `actions_list` (`method:
  list_workflow_runs`, `resource_id: publish-crates.yml`,
  `workflow_runs_filter: {"event": "workflow_dispatch"}`) gives the new
  run, whose `head_branch` is `v0.5.0`; record its address. If the
  dispatch is refused, the user runs the runbook's `gh workflow run` line
  (`:294-296`) at their own prompt.
  - **[stop]** The run waits for the environment's reviewer
    (`get_workflow_run`: `status` `waiting`): the user approves it under
    "Review deployments" (`:298-299`). If it starts without waiting, the
    environment has no required reviewer (`:301-308`): the agent tells
    the user at once and, on their word, cancels it with
    `actions_run_trigger` (`method: cancel_workflow_run`, `run_id`).
  - **Waiting:** once it is approved, one check-in about 12 minutes on
    (`v0.4.0`'s run 40074121811 took 12 min 49 s from its dispatch to its
    end, the wait for its approval included), and another, as the intro
    gives it, if it has not ended.
  - **Checked after** (`:309-318`), once the run ends, from the helper
    agent's summary of its one job's log, "Publish"
    (`list_workflow_jobs`, then `get_job_logs`): the package check
    passed, the 20 crates at 0.5.0; 20
    lines `<crate>: on crates.io, owned by <logins>`, each naming the
    repository's owner, and no `not on crates.io`: no crate is new
    (`:336-341`); the 20 uploads within the burst of 30, in minutes
    (`v0.4.0`'s run, above, took about 13 in all); the run ends with
    one line per crate, all 20 published, none skipped.
  - **crates.io's index** (`:342-348`, for all 20), the user's command at
    their own prompt, its output pasted back:

    ```sh
    for c in $(sed -n '/^crates=(/,/^)/s/^  \(nova-[a-z-]*\)$/\1/p' .github/scripts/publish-crates.sh); do printf '%s ' "$c"; curl -sS "https://index.crates.io/no/va/$c" | tail -n 1 | grep -o '"vers":"[^"]*"'; done
    ```

    which prints 20 lines, each `"vers":"0.5.0"`.

  Record the run's address and its time. If the token is refused at the
  first upload (`nova-diagnostics`), nothing is published: a new token,
  and the workflow again from the tag (spec §10 risk 4; `:699-706`).
- [ ] **Step 6: The token revoked, the secret deleted.** **[stop]**
  Whatever Step 5's result, the agent reminds the user, who revokes the
  token (crates.io, Account Settings → API Tokens → Revoke) and deletes
  the secret, in Settings or by the runbook's `gh secret delete` line at
  their own prompt (`docs/releasing.md:349-357`). **Checked after:** the
  user's word that both are done.
- [ ] **Step 7: The extension in both stores.** **[stop]** The user
  uploads `nova-vscode-0.5.0.vsix`, the release's own, to the
  Marketplace, then to Open VSX (`docs/releasing.md:359-422`), all on
  their own machine:
  - first the runbook's block (`:386-392`): the tag's checkout, `npm
    ci`, the `.vsix` and `SHA256SUMS` from the release, and the sum
    checked (Step 3 compared the same file's `digest`);
  - the Marketplace (`:394-406`), or the manage page's upload, which needs
    no token (the note of 2026-10-14, `:373-378`);
  - Open VSX (`:407-417`), with the user's token.

  A version published to a store cannot be published there again
  (`:735-738`). **Checked after** (`:418-422`), by the user, who says what
  each shows: both listings,
  <https://marketplace.visualstudio.com/items?itemName=sakeerin.vscode-nova>
  and <https://open-vsx.org/extension/sakeerin/vscode-nova>, show version
  0.5.0, the README, the CHANGELOG with its `[0.5.0]` section, and the
  repository, homepage and issues links. The user revokes any Azure DevOps
  token they made (`:422`).
- [ ] **Step 8: The Release gate.** **[stop]** On the user's word, the
  agent starts "Release gate" from `main`, version `0.5.0`
  (`docs/releasing.md:465-471`), with `actions_run_trigger`: `method:
  run_workflow`, `workflow_id: release-gate.yml`, `ref: main`, `inputs:
  {"version": "0.5.0"}`. The workflow's own `inputs:`
  (`release-gate.yml:12-17`) are one, `version`, required, the released
  version without the `v`; it reads no ref of its own. The tool requires
  a `ref`; the runbook's `gh` line (`:470`) leaves it out, which starts it
  on the default branch, `main`, the same. Then `list_workflow_runs`
  (`resource_id: release-gate.yml`) gives the run; record its address,
  and wait with one check-in about 5 minutes on (`v0.4.0`'s Release gate
  run 40107204392 took 4 min 33 s). If
  the dispatch is refused, the user runs the runbook's line (`:469-471`)
  at their own prompt. **Checked after** (`:473-477`), with
  `list_workflow_jobs` and the helper agent's summary of the six jobs'
  logs (`get_job_logs`): all six jobs `success`, "From
  crates.io" and "From the install script" on Linux, macOS and Windows:
  `nova` installed from crates.io (`cargo install nova-cli --version
  =0.5.0 --locked`) and the install gates run on it; `nova` installed
  with the release's `install.sh` or `install.ps1`, and `gate.sh --test`
  run on it; and, since the tag's `gate.sh` takes `--node`
  (`.github/workflows/release-gate.yml:75`), each `gate.sh` run printing
  `gate: the WASM build prints Hello, Nova! under Node` (read one log to
  see the flag taken). If the crates.io jobs fail, the user decides
  quickly (`:479-480`, `:744-748`).
- [ ] **Step 9: The user's try of the published `nova`.** **[stop]** In
  place of the runbook's step 6.2, by its note for `v0.5.0`
  (`docs/releasing.md:532-537`; decision 7): the user installs
  the published `nova` on a machine of their choosing and tries it, as
  for `v0.4.0` (ADR 0047 decision 6). As spec §5 drafts it: `cargo
  install nova-cli --locked`; `nova --version`, which prints `nova 0.5.0`
  (`nova version` prints it, the target and `runtime: embedded`,
  `:541-548`); `nova new hello`, `cd hello`, `nova run`, which prints
  `Hello, Nova!` (`:549-557`); and any Phase 5 program they choose. The
  runbook's step 5, publishing `shapes`, does not run again (`:522-525`).
  **Checked after:** their word, and `nova --version`'s output, recorded
  in their words.
- [ ] **Step 10: 5.6b's gate, then the records.** Check spec §13's six
  items against the ledger: the pull request merged by rebase, its CI and
  `main`'s green, the lock changed in its 23 lines only (Task 2, Steps 10
  and 11; Task 3, Step 1); the tag `v0.5.0` on the release commit, its
  release with the eight assets and `SHA256SUMS`, every sum `OK` (the
  digests against the printed sums, or the user's check), `releases/latest`
  naming it (Steps 2 and 3); all 20 crates on crates.io at 0.5.0 and the
  token revoked (Steps 5 and 6); the extension 0.5.0 in both stores (Step
  7); "Release gate" green, all six jobs (Step 8); the user's try, and
  their word (Step 9).
  **5.6b is done.** The records pull request follows (spec §9; decision
  9), on `claude/loving-archimedes-frtf5q-56b-records`, under its own
  plan: ADR 0061, `docs/adr/0061-phase-5-boundary.md`, with "The
  gate's record" holding `main`'s run 45142347493, the release pull
  request's CI run, the Release run on the tag, the publish run, both
  store listings, the Release gate's run and the user's try, each from
  this task's ledger; CHANGELOG `[0.5.0]`'s dated note with the run IDs;
  dated notes in the master spec and `docs/phase-5-plan.md`; it cites
  `main`'s hashes only.
