# Phase 6.0, "the plan and ADR 0062", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** Phase 6's first pull request, documentation only (Q2), as #128
carried Phase 5's plan. It commits `docs/phase-6-plan.md`, the Phase 6
plan the user approved, byte for byte, in a commit of its own, `docs: the
Phase 6 plan, approved` (as `4bae15da`); then the records commit, `docs:
ADR 0062, Phase 6's scope, and the notes for 6.0` (as `d8eb9ac1`), which
adds `docs/adr/0062-phase-6-scope.md` (Phase 6's scope: all seven items,
the playground as S0 in the browser with the spike's figures, `1.0.0` as
the phase's release, and what Phase 6 leaves out) and dated notes that
point to it: under the master spec's Phase 6, atop `CHANGELOG.md`'s
`[Unreleased]`, after the plan's 6.0 entry, and markers in ADRs 0048 and
0061 at the mapping ADR 0062 amends. No code, test, workflow, manifest,
`Cargo.lock`, version or release change. The records name `main`'s
commits only. After the merge, on the user's word: `main`'s CI, the
checks again on `main`, and spec §12's gate; then 6.1, and 6.2 beside it,
wait for the user's word.

**Architecture:**
- **Two commits** (Task 1; decisions 20 and 21), from three patch blocks:
  `t1-plan`, the approved plan, a new file (750 lines), committed alone;
  then `t1-adr`, the new ADR (256 lines), and `t1-notes`, the five edited
  files (43 lines added, none removed), applied in order, given the commit's
  day by `redate.sh` (decision 24), and committed once.
- **Verification** (Task 2): the records' checks, one script with a
  subcommand for each of spec §5's checks and three more (`planfile`,
  `guard`, `split`), and `redate.sh --check` (decision 25); 15 mutants of
  the records' files (decision 27); one fresh read-only review and one
  fix pass, folded into the records commit before any push (decision
  30); the pull request and its CI (decision 31); the merge.
- **After the merge** (Task 3): `main`'s CI, the checks again on `main`,
  and spec §12's gate (decision 32).

**Tech Stack:**
- Markdown alone. **No Rust, test, workflow, manifest or `Cargo.lock`
  change,** so no `cargo` command runs in this plan: no test reads these
  files (spec §5), and CI runs all 19 jobs on the pull request (spec §4,
  §6; decision 29).
- The checks are Python 3 and bash, run from the repository's root: git,
  and `.github/scripts/release-notes.sh`.

**Spec:**
`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`,
approved by the user on 2026-11-27 as drafted (`8709e8dc`), and its dated
amendment of 2026-11-27, which this plan's commit appends (decision 36).
- Read its header, §1 to §12, and the amendment, before Task 1.
- The user's decisions are decisions 1 to 17 here, the approval decision
  18.

**Base:** `8709e8dc` ("docs: the Phase 6.0 spec is approved"), `main`
`4fe11d55` ("docs: Phase 5's records, ADR 0061", #152) and the spec's
commit. The branch is `claude/loving-archimedes-frtf5q-60`, from
`8709e8dc`, then this plan's commit (decision 36). `origin/main` is
`4fe11d55`; the clone is shallow at `a71d27d9`.

**Checked on 2026-11-27, before execution:**
- **A prototype made the records whole,** on a local branch from `8709e8dc`,
  never pushed (spec §6 and its decision 14: no draft pull request): the
  approved plan copied, the spec's drafted blocks copied by script, the
  outline filled. Its checks, spec §5's as decision 25 makes them, all
  passed: one commit hash, `4fe11d55`, on `main` by `git merge-base`; ADR
  0062's 35 `path:line` citations resolving with their words (38 after
  decision 2's fix, below), the plan's 83 at `4fe11d55`, each to lines that
  exist and are not blank; the nine drafted blocks at their places; no
  private text; every figure of ADR 0062's table in the spike's report; the
  records commit's day only in its six places; `release-notes.sh 0.5.0`
  printing `4fe11d55`'s 627 lines, `Unreleased` exiting 0 with the note (1
  before); exactly six files. The spec's own 86 citations and five hashes
  were checked once, by hand, on 2026-11-27 (decision 25), and so were the
  amendment's own 29 `path:line` citations: all hold. Where the spec's
  draft proved wrong, and where it was filled, is decision 34 and the
  amendment.
- **Three prototype fixes** (decision 19): ADR 0062's References name this
  plan, `docs/superpowers/plans/2026-11-27-phase-6-0-the-plan-and-adr-0062.md`,
  in ADR 0061's form, three lines for one at the file's end, so no
  citation moves; decision 3's "No new crate or feature", after "a new
  crate or a split of `nova-driver`", says "No new dependency", the spike's
  meaning; and, by the user's ruling of 2026-11-27 (decision 18), decision
  2's last item is two: what other specs give Phase 6, and what the specs
  and earlier ADRs put elsewhere (examples 08 to 15 among it, as their
  table lists them), nine lines for six, so every line after it moves by
  three. ADR 0062 is 256 lines; the records commit adds 299 lines
  and removes none, the two commits 1,049.
- **The split** (decisions 20 to 23 and 28): a local branch from `8709e8dc`
  with two commits, the approved plan's and the records commit, with this
  plan's messages; its last tree is the fixed prototype's (`git diff`
  between the two prints nothing). Its three blocks are the prototype's diff
  over their files, `t1-plan`'s against `8709e8dc` with the approved text,
  and each applies with plain `git apply`, in order, printing nothing, each
  applied tree the expected one; no added line holds trailing whitespace, a
  CR or a backslash-u escape. **The split is a local reference, not a commit
  the executor makes** or pushes: this plan names it by its six files'
  blobs, which `checks.py split` compares (Task 1, Step 5), and the approved
  text by its blob, `d891448d`, which `checks.py planfile` compares.
- **The replay** (decision 26): a rehearsal of the execution branch on no
  branch, in a linked worktree, as an executor would make it: `8709e8dc`;
  this plan's commit (this plan as drafted, and the amendment); then Task 1
  from this plan's own blocks and here-documents, each copied out by script,
  and Task 2's checks at its tip, dated 2026-11-27:
  - `hashcheck`: 1,067 lines read (the added lines and the two messages), 1
    distinct word, `4fe11d55`, 6 times (five in the records, once in the
    approved plan's message), a commit on `main` by `git merge-base`;
  - `pathcheck`: 38 ADR citations, 83 of the plan's at `4fe11d55` (8 of them
    reading otherwise in the tree), 64 repository paths (this plan's among
    them), 21 names that are not paths, 0 Markdown links, 0 URLs;
  - `privacy`: 1,067 lines read (1,049 added lines and 18 message lines), 2
    names built; allowed, the owner's login once, in the plan's Pages
    address, and the records' branch 6 times;
  - `notes`: `0.5.0`'s 627 lines, the same as `4fe11d55`'s; `Unreleased`
    exits 0 with a blank line and the note, 10 lines (it exits 1 at
    `4fe11d55`);
  - `files`: 6 files, 1,049 lines added and 0 removed;
  - `place`: 11 checks, each `ok`: the title, the Status, the Decision, the
    figures, the Consequences, the parts' order, and the three notes and two
    markers, each once at its place; `redate.sh --check`: the six places
    dated, the history as written;
  - `planfile`: 763 lines; without lines 427 to 439, 44,866 bytes, the
    approved text's blob; the approved plan's commit adds it as that blob;
    `guard`: the twelve decisions (105 lines) and the figures table (15
    lines) the split's;
  - `split`: 6 of 6 files the split's; and the tree outside
    `docs/superpowers` the split's.
- **The mutants** (decision 27): 15 breakages at the rehearsal's tip, in one
  run of under two minutes: 14 caught, each by exactly the checks it should
  be, and 1, A1, accepted; 15 of 15 as expected (Task 2, Step 3).
- **Another day** (decision 24): on the rehearsal's tree, `redate.sh
  2026-11-28` changed 6 lines in 6 files, and every dated check passed with
  that day (`place`, `notes`, `split` and `redate.sh --check`); `redate.sh
  2026-11-27` then gave the tree back, `git status --short` empty.
  `redate.sh` refused `26-11-27` and `2026-02-30`.
- **The GitHub reads** (read only, 2026-11-27): `get_commit` of
  `4bae15da` and `d8eb9ac1`, #128's, for the two commits' form: "docs:
  the Phase 5 plan, approved", with a one-line body, and "docs: ADR 0048,
  Phase 5's scope and backend, and the notes for 5.0a"; and `list_commits`
  from `a71d27d9` on `main`, which lists `d8eb9ac1` (its 8th), `4bae15da`
  (its 19th) and `a9d876e3` (its 20th), so the three the spec names, older
  than the shallow boundary, are `main`'s (spec §5, check 3). The records
  name none of them, nor any commit older than the boundary.

## Global Constraints

- **Documentation only.** Task 1's two commits change exactly six files,
  `CHANGELOG.md`, `docs/adr/0048-phase-5-scope-and-backend.md`,
  `docs/adr/0061-phase-5-boundary.md`, `docs/adr/0062-phase-6-scope.md`
  (new), `docs/phase-6-plan.md` (new) and `nova-spec/00-MASTER-SPEC.md`,
  and the pull request adds the spec, this plan and the spec's amendment:
  eight files. Nothing under `crates`, `std`, `selfhost`, `tests`,
  `examples`, `.github` or `tools`, no manifest and no `Cargo.lock` (spec
  §5, check 8). **Any other change stops the task;** ask the user.
- **The approved plan is committed verbatim** (spec §3.1 and its decision
  7: "No line must change"). `docs/phase-6-plan.md` is the approved text,
  then the 6.0 note alone (spec §3.5), and nothing else: not a fix to one
  of its citations (decision 34), not a review's fix (decision 30).
  `checks.py planfile` holds it.
- **No `cargo` command runs here.** Nothing is built: no test reads these
  files (spec §5), and CI runs all 19 jobs on the pull request and on
  `main` after it (decision 29). So no build set in `target/` is used or
  changed, and the MSRV (1.78) check and the rest are CI's.
- **The records' rule** (spec header; its decision 16): every record names
  `main`'s commits only, by 8 characters; pull request numbers are allowed;
  no `proto/`, `split/` or `-ci` branch, no pull request head or pre-rebase
  hash, no local or scratch path, no person's name or email, no model
  identifier. The records name one commit, `4fe11d55`; the spec's own
  commit, `8709e8dc`, never reaches `main` by that hash (the rebase merge
  gives each commit a new one), and no record names it. `checks.py
  hashcheck` and `privacy` hold the rule (decision 25).
- **The shallow clone.** The clone is shallow at `a71d27d9`, so `git
  merge-base` cannot place a commit older than it. The records name none
  (Checked on): any such hash fails `hashcheck`, and the main session
  checks it with its GitHub MCP tools before it is kept. `git fetch
  --unshallow` needs the user's word (spec §9, risk 5). The executor runs
  no `git fetch`; the main session fetches `origin main`, with git, after
  the merge (Task 2, Step 8). If that fetch is refused, Task 2, Step 8's
  fallback reads `main` with the GitHub MCP tools; nothing else works
  around the refusal.
- **Never clean `/tmp/nova-*`.** Nothing in this plan writes there: every
  temporary file goes under `$P/exec60/tmp`, the env file's `TMPDIR`
  (Conventions).
- **Keep `df -m /` above 6,144 MiB.** Nothing here writes more than a few
  MiB, all of it under `$P/exec60/`. The disk check (Conventions) runs at
  Task 0 and before the mutants; if it reads under the threshold, stop and
  report: nothing is removed to make room.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code
  in braces. None of 6.0's blocks or scripts holds one. Apply the blocks
  with `patch_block`, from the committed plan.
- **No block adds a CR, a NUL or trailing whitespace.** The non-ASCII
  characters the blocks add are `§`, `—` and `×`. `t1-notes` holds 6 blank
  context lines, each a single space, which `git apply` needs: `git diff
  --check` on this plan's file reports them, and the records' own lines are
  clean (Task 2, Step 2).
- **Nothing writes into the repository but the blocks, `redate.sh` and the
  review's fixes.** `git status --short` is empty after every check and
  after the mutants.
- **Push and open the pull request on the user's word only.** No push of
  this branch is allowed yet; Task 2, Step 6 stops before any push, and
  the main session asks the user. **After the push, nothing is amended or
  force-pushed without the user's word** (decision 30): a change CI or a
  review asks for then stops the work, and the main session asks the
  user.
- **Merge by rebase, on the user's word only,** pinned to the head. Never
  enable auto-merge.
- **The main session, not a subagent, does every GitHub action.** It
  pushes with git, `git push -u origin
  claude/loving-archimedes-frtf5q-60`, retried with backoff on a network
  error, as the session's rules say, and fetches with git (`git fetch
  origin main`, Task 2, Step 8); no GitHub MCP tool pushes commits.
  Everything else goes through its GitHub MCP tools: the pull request
  (`create_pull_request`, `update_pull_request`), the subscription, the
  merge, every read of CI and of `main`. The session has no `gh` and runs
  no `curl`. A push refused by a permission or safety rule stops the
  work, and the user is asked; a refused fetch has the one fallback Task
  2, Step 8 gives. A helper agent that reads CI's logs is read-only and
  writes nothing to GitHub.
- **No secret is read, printed or stored,** and no token's value goes in a
  record.
- **This machine is Linux;** macOS and Windows results come from CI.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Every shell exports it, `export P=…` with the
  directory's path, before a command that names `$P`. Exported, not
  merely set: the mutant runner reads it from the environment.
- **The executor's directory:** `$P/exec60/`. Every script this plan
  writes (`redate.sh`, `checks.py`, `mutrun.py`), the two commits'
  messages, every output and every log goes there, the disk checks'
  `logs/df.log`, the mutants' `mut/` and the temporary files' `tmp/` among
  them. Two files are kept outside it: the ledger, `$P/ledger-60.md`, and
  the env file, `$P/env60-exec.sh`.
- **The ledger:** `$P/ledger-60.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures, and each
  of the user's words in Task 2 and Task 3, in their words.
- **The env file:** `$P/env60-exec.sh`, sourced at the start of every
  tool call (`source "$P/env60-exec.sh"`): each tool call is a fresh
  shell, so a variable a step sets is set again in each call. Its text,
  with `P` the scratch directory's path:

  ```sh
  # Phase 6.0 execution shell setup: the plan's Conventions (root, cgroup v1), with PLAN the committed plan.
  P=/path/to/the/scratch/directory; export P; cd /path/to/the/repository
  CG=/sys/fs/cgroup/pids/nova-limit; mkdir -p "$CG" && echo 600 > "$CG/pids.max" && echo $$ > "$CG/cgroup.procs"
  grep -q nova-limit /proc/self/cgroup || { echo "NOT IN nova-limit: stop"; return 1 2>/dev/null || exit 1; }
  mkdir -p "$P/exec60/logs" "$P/exec60/tmp"
  export NO_COLOR=1 PYTHONUTF8=1 GIT_PAGER=cat TMPDIR="$P/exec60/tmp"
  PLAN=docs/superpowers/plans/2026-11-27-phase-6-0-the-plan-and-adr-0062.md
  CHK() { python3 "$P/exec60/checks.py" "$@"; }
  MSGS() { echo --msg "$P/exec60/msg-t1-plan.txt" --msg "$P/exec60/msg-t1.txt"; }
  block() {
    awk -v name="$1" '
      $0 == "<!-- patch " name " -->" { grab = 1; next }
      grab && /^````diff$/ { inside = 1; next }
      inside && /^````$/ { exit }
      inside { print }
    ' "$PLAN" > "$P/exec60/$1.patch"
  }
  patch_block() {
    block "$1" && git apply --check "$P/exec60/$1.patch" && git apply "$P/exec60/$1.patch"
  }
  ```

  So `CHK NAME` runs one of the records' checks (Task 1, Step 1), `CHK
  NAME $(MSGS)` gives it the two commits' messages, and `patch_block NAME`
  writes a block of this plan to `$P/exec60/NAME.patch` and applies it
  with plain `git apply`. `PYTHONUTF8=1` keeps the scripts' output in
  UTF-8 whatever the locale; `TMPDIR` keeps `checks.py notes`' temporary
  copy, and every other temporary file, under `$P/exec60/` (5.6c's
  rulings R1.1 and R2.1).
- **The process limit.** Every shell first caps the tasks it may start at
  600, threads as well as processes, and checks that the cap holds: the
  env file joins the pids control group `nova-limit` (root, cgroup v1, as
  5.6c's did), and stops if the shell is not in it. The 5.6a plan's
  Conventions
  (`docs/superpowers/plans/2026-11-22-phase-5-6a-the-bootstrap-gate.md`)
  give the forms for cgroup v2 and for a user other than root.
- **The disk check,** logged, its `echo` naming the task, the step and
  what comes next:

  ```sh
  { echo "Task 2 Step 3, before the mutants"; df -m / | tail -1; } >> "$P/exec60/logs/df.log"
  tail -n 1 "$P/exec60/logs/df.log"    # its fourth field: the MiB free, at least 6,144
  ```

- **Commits.** The two commits' messages are written to
  `$P/exec60/msg-t1-plan.txt` and `$P/exec60/msg-t1.txt` by their
  here-documents (Task 1, Step 1): each a subject, a body, a blank line,
  and these two trailer lines, exactly:

  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
  ```

  Then `git add -A`; `git status --short` lists only the step's files;
  and `git commit -F` with the message's file. Every commit of this plan
  ends with those two lines: the plan's own, the approved plan's and the
  records commit, into which the review's fixes are folded (decision 30).
  The pull request's body ends as Task 2, Step 6 gives it. No model ID (a
  `claude-…` model name) goes in any artifact: no commit, file, pull
  request, comment or ledger line names one.
- **Timeouts, and nothing in the background.**
  - **A tool's timeout is in milliseconds:** at least the command's
    stated time plus half again, never under 120000 (120 s) and at most
    600000 (600 s); 300000 for the mutants (Task 2, Step 3).
  - Run every command in the foreground: never with `&`, `nohup` or a
    tool's background mode. A block of several commands runs one command
    at a time. Long runs write their output to a log under `$P/exec60/`,
    which is read after.
  - If a command is cut off, restore what it left: after a cut-off mutant
    run, `git checkout -- .`, then `git status --short` must print
    nothing, then `mut/` is moved aside and the run starts again from the
    first (Task 2, Step 3). `git apply` applies a block whole or not at
    all: if `t1-notes` does not apply after `t1-adr` did, `git apply -R
    "$P/exec60/t1-adr.patch"` takes the new file away, `git status
    --short` must print nothing, and the task starts again from Step 3; if
    `t1-plan` does not apply, nothing was written, and the step stops.
- **CI's runs: waiting for them and reading them** (Task 2, Step 7; Task
  3). The main session never waits with a foreground `sleep`.
  - **Waiting.** For the pull request, the main session subscribes to its
    activity (`subscribe_pr_activity`) once it is open, and CI's results
    wake the session. For `main`'s run after the merge, it arms one
    scheduled check-in in this session (`send_later`) for about the run's
    time (#152's took 10 min, `main`'s 13 min), reads the run with
    `actions_get` (`method: get_workflow_run`) when it fires, and arms
    another if its `status` is not `completed`; or the user says when it
    is done.
  - **Reading the logs.** The main session does not pull whole job logs
    into its own context. A read-only helper agent fetches each job log a
    check needs with `get_job_logs`, whole (`return_content: true`,
    `tail_lines` 100000), into `$P/exec60/ci/`, and writes a summary,
    `$P/exec60/ci-<run>.txt`, `<run>` the run's id: each job's
    conclusion, the `test result:` counts summed for each Test job, the
    Selfhost job's lines, the gate's `bootstrap:` lines, and the step
    lines Task 2, Step 7's Expected names. It writes nothing to GitHub. A
    check that reads a log is made against that summary.
- **The patches.** Task 1's three blocks below are each marked `<!-- patch
  NAME -->` and fenced with four backticks. They are the prototype's diff,
  so apply them; do not retype them. `patch_block t1-adr` (the env file
  defines it, with `PLAN` the committed plan) writes the block to
  `$P/exec60/t1-adr.patch` and applies it with plain `git apply`, which
  prints nothing. A block that does not apply means the tree differs from
  the plan's: stop, and record why in the ledger.
- **The scripts.** `redate.sh`, `checks.py` and `mutrun.py`, and the
  commits' messages, are written by here-documents at column 0, each in
  its own fenced block, to `$P/exec60/`: copy each block out by script,
  its line range checked at both ends (5.6b's ruling R1.1), or run it as
  it stands; never retype it.

## Review Focus

The inputs most likely to bite a reader of the records, most likely first.
Each has its check in the task that owns it:

1. **A record that names a commit not on `main`, or a `path:line` that
   moved** (spec §9, risk 1; the records' rule): ADR 0062's citations at
   the tip, the plan's at `4fe11d55` (its 6.0 note says so), and none in
   the notes: → `CHK hashcheck` and `CHK pathcheck` (Task 1, Step 4; Task
   2, Step 1; again on `main`, Task 3, Step 2); mutants H1, P1 and P2.
2. **Saying more than was measured** (spec §9, risk 2): Chromium was
   local; no 1.78 build; no `wasm-opt`; a worker's headroom an estimate.
   The decisions and the figures table are the spec's drafts, held by
   `CHK place` with the amendment's fixes, and byte for byte by `CHK
   guard` after any fold (mutant G1); the review (Task 2, Step 5) reads
   them, and ADR 0062's Context, which is filled, not drafted (mutant A1),
   against the spike's figures as spec §3.2 drafts them and the plan's
   §2. No script can tell a true claim from a false one.
3. **The approved plan changed** (spec decision 7): → `CHK planfile`,
   the approved text's blob without the 6.0 note; mutant L1.
4. **Private text** (a scratch path, a login, an email, a prototype's
   branch): → `CHK privacy`, its names built from git and never written
   (decision 25), and the review; mutants S1 to S3.
5. **The date** (spec §5, check 6): the commit's day in exactly its six
   places, the history's days untouched: → `redate.sh` and its `--check`
   (decision 24); mutants D1 and D2.
6. **`release-notes.sh`:** `0.5.0`'s notes unchanged, and `Unreleased`
   printing the note: → `CHK notes`; mutant N1.
7. **Each note at its place** (spec §5, check 1): → `CHK place` and `CHK
   files`; mutants M1, K1 and F1.
8. **CI green with §4's counts:** → Task 2, Step 7's Expected, from #152's
   run 45829766657; macOS's `selfhost.rs` near or over its 100 s line
   (104.82 s on #152) goes to the user, as the time is the runner's.
9. **The merge, and 6.0's gate, each on the user's word:** → Task 2,
   Steps 6 and 8, and Task 3.

## Decisions

The user's, from the spec's header and §11, each option's name as given,
in Thai, ending "(แนะนำ)", "recommended"; then what was chosen, and where
the records carry it.

Phase 6's (2026-11-26):

1. **The start,** "เริ่ม Phase 6 ได้เลย": Phase 6 begins as Phase 5 did,
   with research, a spike, the phase plan for approval, then sub-phases.
   This pull request is the first sub-phase's, 6.0.
2. **P1,** "ครบ 7 ข้อ, งานภายนอกเป็นของคุณ (แนะนำ)": all seven items, the
   external parts the user's, not blocking 1.0. ADR 0062's decision 1 and
   the master spec's note (`t1-adr`, `t1-notes`).
3. **P2,** "A สำหรับ 1.0, B ไป backlog (แนะนำ)": S0 in the browser after a
   spike, an ADR, N for WASM on the backlog. ADR 0062's decision 3, its
   figures, and the markers in ADRs 0048 and 0061.
4. **P3,** "1.0.0 เป็น release ของ Phase (แนะนำ)": ADR 0062's decision 4;
   the master spec's and the CHANGELOG's notes.
5. **P4,** "ยังไม่มี ใช้ GitHub Pages ไปก่อน (แนะนำ)": ADR 0062's decision
   8; the master spec's note.
6. **P5,** "ทั้งหมด (แนะนำ)": std with `std/dom`, `missing_docs`, a
   reference, a book, an error index. ADR 0062's decision 6.
7. **P6,** "ภาษา, std, CLI, nova.toml, nova.lock, index format (แนะนำ)":
   ADR 0062's decision 5.
8. **P7,** "ใช้ hardening ใน repo เป็น gate (แนะนำ)": ADR 0062's decision
   7.
9. **P8,** "เฉพาะ test ที่ไม่ใช้ database (แนะนำ)": ADR 0062's decision 9.
10. **P9,** "รวม std ที่ยังขาดด้วย", then "std/regex, HTTP client": those
    two, with the break-later items. ADR 0062's decision 10.
11. **P10,** "ไม่เอาเข้าเลย (แนะนำ)", the third and final answer: none of
    ADR 0061's backlog. ADR 0062's decisions 2 and 10.
12. **P11,** "คง 1.78 + เขียน policy (แนะนำ)": ADR 0062's decision 11.
13. **P12,** "Playground spike ก่อน (แนะนำ)": the spike, whose figures ADR
    0062 holds.
14. **The plan,** "อนุมัติตามร่าง (แนะนำ)": the Phase 6 plan approved as
    drafted, committed verbatim by this pull request (`t1-plan`; spec
    §3.1).
15. **The target,** "อนุญาตติดตั้งในเครื่องนี้ (แนะนำ)": the
    `wasm32-unknown-unknown` target locally, for the spike. ADR 0062's
    Status and decision 12; CI's target is 6.1's to ask.
16. **Q1,** "จับและแจ้งบนหน้า playground (แนะนำ)": 6.1 catches a long
    function's overflow and says so on the page; S0's codegen untouched
    before the freeze; the rewrite on the backlog. ADR 0062's decision 3
    and the plan's 6.0 note.
17. **Q2,** "Workflow เต็ม (แนะนำ)": as 5.6c, with the plan, ADR 0062 and
    notes in the master spec and `[Unreleased]`. This plan's workflow,
    and the CHANGELOG's note.

The user's answers of 2026-11-27, on the spec and on ADR 0062's
decision 2:

18. **The spec, approved as drafted** ("อนุมัติตามร่าง (แนะนำ)"), the
    playground's limits ruled with it: under 5 MB gzipped, a small program
    in under 2 s, the depth limit of 128 survived; the 1.78 build and the
    `wasm-opt` size to 6.1. Committed as `8709e8dc` on this branch, from
    `4fe11d55`. **Then ADR 0062's decision 2 corrected,**
    "แก้ใน ADR 0062 ให้ถูก (แนะนำ)" ("fix it in ADR 0062 so it is correct",
    recommended): its last item, as drafted, said other specs give Phase 6
    five things its list does not name, and the sources give it two; the
    user ruled that the item be split in two, the two Phase 6 is given and
    the three left where the specs and earlier ADRs put them, all five out
    of Phase 6 as before (decisions 19 and 34; the amendment).

How this plan is cut and checked:

19. **Three prototype fixes, then the split made again.**
    - **References name this plan.** Spec §3.2's outline gives References
      "this spec and its plan, in ADR 0061's form". The prototype named
      the spec's path alone, since the plan did not exist at its tip. The
      real cut has this plan in the commit before Task 1's, so the fixed
      prototype names it in ADR 0061's form ("and its plan," then the
      path), three lines for one. They are the file's last section, so
      no line anything cites moves; `pathcheck` holds the path (mutant
      P2).
    - **No new dependency.** Decision 3's last item, "What 6.1 builds",
      lists "the browser's entry, a new crate or a split of
      `nova-driver`", and the draft ended it "No new crate or feature.",
      the spike's words, which beside that list read as a contradiction:
      the spike meant that 6.1 needs no crate `Cargo.lock` does not
      already hold. It now ends "No new dependency." The line stays one
      line, so no citation moves; `place` holds it as the amendment
      records it (`FIXES`).
    - **Decision 2's last item, as its sources say** (the user's ruling,
      decision 18). As drafted, it named five things "other specs give Phase
      6": multiple toolchains, `nova-embed`, the `setup-nova` action,
      examples 08 to 13 and edition rules. The sources give Phase 6 the
      first two (`nova-spec/40-TOOLING.md:646`, ADR 0037 `:93`;
      `nova-spec/13-RUNTIME.md:480`). The other three stay where they were
      put: the `setup-nova` action and edition rules on Phase 3's backlog
      (ADR 0026 `:50`; ADR 0037 `:100` and `:97`), and examples 08 to 15
      "post-Phase 6", after the 1.0 release
      (`nova-spec/60-EXAMPLES.md:801-803`). So the item is two, nine lines
      for six, and all five stay out of Phase 6. The draft's "08 to 13", the
      research's too, is "08 to 15" by the same ruling, a fix made after the
      first: the section's table (`nova-spec/60-EXAMPLES.md:807-814`) lists
      eight examples, 08 to 15; the line keeps its length. Every line after
      the item moves by three: `pathcheck`'s table was renumbered
      (`renumber`), with three rows added for the new citations and one
      changed (`:801-812` is `:801-803`, the heading and its "after 1.0
      release"); `place` holds the two items as the amendment records them
      (`FIXES`), and `guard` the decisions as fixed.
    - Before them, the prototype fixed three of the spec's drafts against
      their sources (decision 34).
20. **The cut: spec §7's four commits, the approved plan's third**
    **(differs).** Spec §7 (its decision 13) has four commits, "the
    plan's first, as in #128": `docs: the Phase 6 plan, approved`, `docs:
    the Phase 6.0 spec is approved`, `Plan Phase 6.0: the plan and ADR
    0062`, and the records commit, `docs: ADR 0062, Phase 6's scope, and
    the notes for 6.0`. The spec's commit, `8709e8dc`, was made on
    `4fe11d55` when the user approved the spec, and is this branch's base.
    Task 1 makes the approved plan's commit from this plan's `t1-plan`
    block, copied out of the committed plan, so this plan's commit must come
    before it; and `planfile` needs the first commit after `4fe11d55` that
    touches `docs/phase-6-plan.md` to be the approved one, so the records
    commit, which adds the 6.0 note to that file, comes after it. So the
    branch carries the four commits in this order: the spec's, this plan's,
    the approved plan's, and the records commit, last. #128 does not give
    this order: there the approved plan, `4bae15da`, was the 2nd of 14
    commits, before the 5.0 spec and its plan (§7's order), so #128 shows
    only that the approved plan came before the records, `d8eb9ac1`. Each
    commit holds what spec §7 gives it; only the approved plan's place in
    the order differs, and the amendment records it.
21. **Three patch blocks, two commits** (spec §7: the records in one
    commit, §3.2 to §3.6, as `d8eb9ac1`).
    - `t1-plan` is the approved text as a new file (756 lines), committed
      alone; `t1-adr` the new ADR (262 lines); `t1-notes` the five edited
      files (98 lines): the master spec's note, the CHANGELOG's note, the
      plan's 6.0 note on the file `t1-plan` added, and the two markers. Each
      is the prototype's diff over its files, `t1-plan`'s against `8709e8dc`
      with the approved text, applied in order, each whole, by plain `git
      apply`.
    - **`t1-plan` is a patch, not a copy-out,** as 5.6c's 498-line ADR
      block was: `git apply` writes it whole or not at all and refuses a
      tree where the file exists, `checks.py planfile` then compares its
      bytes with the approved text's blob, and nothing in the repository
      holds that text for a copy-out to name (it is in no commit before
      this pull request, and a scratch path is not a record's).
    - **The plan's note is in `t1-notes`,** not in `t1-plan`: the
      approved plan's commit is the approved text alone, so that commit's
      blob is the one the user approved, and the note arrives with the
      records it points to, as `d8eb9ac1` gave the Phase 5 plan's 5.0
      entry its note after `4bae15da`.
    - Not one block for the records in parts by `git apply --include`: an
      `--include` whose pattern matches no file applies nothing and prints
      nothing (5.6b's decision 13), while a whole block that does not
      apply fails loudly. Not six blocks: the notes and markers are one
      change, pointing to one ADR, and each block has its own check
      (Task 1: the new file's length and blob, the new ADR's length, the
      five files' numstat).
22. **The commits' subjects are spec §7's;** the bodies are this plan's.
    `docs: the Phase 6 plan, approved` says, as `4bae15da` did, when the
    user approved it ("on 2026-11-26", history, which `redate.sh` does
    not touch), and that its citations are `4fe11d55`'s. The records
    commit's body is the prototype's, with §3.2 to §3.6 for §3. Neither
    message carries the records' day, so the commit's day is in the
    records alone (decision 24); `hashcheck` and `privacy` read both
    messages.
23. **The approved plan named by its blob.** The approved text is in no
    commit before this pull request, so `checks.py planfile` holds its
    blob, `d891448dd75bf0e84ee55af6ddf95a56aed716d4` (44,866 bytes, 750
    lines): `docs/phase-6-plan.md` without its lines 427 to 439 (the 6.0
    note and the blank after it) must hash to it, and the first commit
    after `4fe11d55` that touches the file must add it as that blob,
    with spec §7's subject. Lines 426 and 439 blank and line 440 `### 6.1
    — The playground` place the note.
24. **The date: the commit's day, by script.** The blocks date the
    records 2026-11-27, the prototype's day, in exactly six places: ADR
    0062's Status ("Accepted (2026-11-27)."), the master spec's note
    ("**Amended 2026-11-27 (branch"), the CHANGELOG's note ("**Recorded
    2026-11-27 (branch"), the plan's 6.0 note ("**Amended 2026-11-27
    (6.0, branch") and the two markers ("**Amended 2026-11-27 (branch",
    each at its indentation). Spec §3.2 dates the Status by the records
    commit's day (its decision 15), and §5's check 6 allows that day in
    the notes' places. Every other date in the added lines is history,
    and stays as written whatever day the commit lands: the user's
    decisions, the plan's approval and the spike, all of 2026-11-26 (six
    times in ADR 0062, seven in the plan, twice in the master spec's note,
    once in ADR 0061's marker), and issue #3's 2026-08-12 in the plan. The
    spec's approval (2026-11-27) and the release's days are in no line
    the records add. So Task 1, Step 4 runs `redate.sh "$(date -u +%F)"`
    before the records commit, whatever the day: it finds each place by
    the words around it and rewrites it whatever date it holds, then
    checks, against `4fe11d55`, that each place holds the day, that no
    removed line holds a date, and that the history's dates in the added
    lines are exactly as written (file names' dates are names).
    `redate.sh --check DAY` checks without writing; the mutants and Task
    2 use it. The day is UTC's, as the commit's own date is. **A commit
    made is re-dated one way,** if the day turned between Step 4 and the
    commit or the review's fixes are folded on another day: `redate.sh`
    with the new day, the checks with it, `git add -A`, then `git commit
    --amend --date=now --no-edit` (the message has no date to change). A
    fold on the commit's own day is `git commit --amend --no-edit`. The
    approved plan's commit holds no records' day, and is not re-dated.
25. **The checks, spec §5's, as one script** **(differs: checks 2 and 3 on
    the spec's own text are not scripted, below)** (`checks.py`, Task 1,
    Step 1), a subcommand for each, each printing what it found and a last
    line `NAME: ok` or `NAME: FAILED`, and exiting 0 or 1; and `renumber`,
    for `pathcheck`'s table (below):
    - **They read the working tree against `4fe11d55`,** `main`, whose
      files the records change: on a clean tree at the records commit
      that is the two commits, and an edit is seen before it is
      committed, which is what lets the mutants (decision 27) and a check
      before the commit (Task 1, Step 4) work. "Added lines" are those
      `git diff 4fe11d55` adds outside `docs/superpowers` (the spec and
      this plan), and the lines of a file not yet added: the whole of the
      approved plan among them.
    - **`hashcheck`** (spec §5, check 3): every word of 7 to 40 hex digits
      in the added lines and the two messages is a commit on `main` by 8
      characters (spec decision 16), by `git merge-base --is-ancestor`
      against `origin/main`; anything else fails, a decimal word too, so a
      new figure is classified before it passes. The records hold no
      commit older than the shallow boundary, and the script holds none
      by name.
    - **`pathcheck`** (check 2): ADR 0062's 38 backticked `path:line`
      citations, each in its table with the ADR's line, the cited file (an
      ADR by its number) and the words its lines must hold, at the tip; a
      citation the table lacks fails, as does one off its line. The plan's
      83, read at `4fe11d55`, as its 6.0 note says and spec §3.1 rules: each
      resolving there to lines that exist and are not blank (by line alone:
      their words are not checked, below), a short `:N` by the plan's line
      and the file its sentence names; the eight that read otherwise at the
      tip are listed, not failed. No `path:line` in the notes or markers.
      Then every backticked repository path in the added lines exists, or is
      a name the table lists as not a path (a file named as missing, 6.2's
      `docs/stability.md`, the records' branch, the plan's Pages address,
      which the script matches by form and never writes); no Markdown link
      fails; no URL. **The ADR's table, `CITES`, is keyed by ADR 0062's
      line,** so an edit that adds or removes a line above a citation moves
      every key after it: `renumber` rewrites the keys in the script from
      the ADR in the tree, in the citations' order, and refuses, writing
      nothing, if a citation was added, removed or changed (that row is
      changed by hand); then `pathcheck` checks each row's words again (Task
      2, Step 5).
    - **`privacy`** (check 4): no local or scratch path, branch but
      `claude/loving-archimedes-frtf5q-60`, person's name or email, model
      identifier, session link or token, in the added lines or the
      messages, their two trailer lines excepted. The names are built when
      it runs, from the remote's owner, the authors and committers on
      `main` and the git config, and never written in this plan; the
      owner's login is allowed only in the repository's address and the
      plan's Pages address (`docs/phase-6-plan.md:96`), the records' own
      (spec §5, check 4). It counts the lines it read with and without
      the messages, so a run without `--msg` (Task 3) is told apart.
    - **`notes D`** (check 7): `release-notes.sh 0.5.0 CHANGELOG.md`
      exits 0 and prints `4fe11d55`'s notes, the same lines (the note is
      in `[Unreleased]`); `release-notes.sh Unreleased CHANGELOG.md`
      exits 0 and prints a blank line and spec §3.4's note, dated `D`,
      nothing else (it exits 1 at `4fe11d55`).
    - **`files`** (check 8): exactly the six files differ from `4fe11d55`
      outside `docs/superpowers`, and no other file is untracked.
    - **`place D`** (check 1): each block spec §3.2 to §3.6 drafts (ADR
      0062's Status, Decision, figures and Consequences; the master spec's,
      the CHANGELOG's and the plan's notes; the two markers), dated `D`
      where the commit's day goes, with the amendment's fixes (`FIXES`: five
      lines, and decision 2's last item, six lines for nine), verbatim,
      once, at the place §3.3 to §3.6 name, by the lines around it; ADR
      0062's title as §3.2's table gives it, and its parts in order.
    - **`planfile`** (check 1's first half): decision 23.
    - **`guard`:** ADR 0062's twelve decisions (`## Decision` up to `###
      The spike's figures`, 105 lines) and its figures table (15 lines)
      hash to the split's, by SHA-256, whatever `place`'s fixes say: the
      parts the user ruled on, byte for byte after any fold (decision
      30).
    - **`split D`:** the six files, `D` put back to 2026-11-27, hash to
      the split's blobs (decision 28).
    - Check 5 (the figures equal the spike's report) and check 6 (the
      dates) are not here as such: the report is in scratch space, which
      no record or script of this plan names, so the prototype compared
      every figure of the table with it, and `guard` and `place` hold the
      table as compared; `redate.sh` is check 6.
    - **What no script holds (differs).** Spec §5's checks 2 and 3 name the
      spec's own text too: its citations, at `4fe11d55`, and its hashes. The
      spec's 86 `path:line` citations (29 with a path, 57 short) and its
      five hashes were checked once, by hand, on 2026-11-27: every citation
      holds at `4fe11d55` but spec `:156`'s `:101`, which amendment item 2
      records; `4fe11d55` and `a71d27d9` are on `main` by `git merge-base`,
      and `a9d876e3`, `4bae15da` and `d8eb9ac1`, older than the shallow
      boundary, by `list_commits` (Checked on). No script holds them: the
      spec is not among the lines `checks.py` reads. And the plan's 83
      citations are held by line alone, each resolving at `4fe11d55` to
      lines that exist and are not blank: their words are not checked
      (`:243`'s words, which ADR 0009 holds at `:105`, not `:57`, are
      amendment item 4). The amendment's own 29 `path:line` citations were
      checked by hand on 2026-11-27, and all hold; no script reads them
      either.
26. **The replay is a rehearsal of the execution branch, on no branch.**
    `pathcheck` needs this plan's path, which only the plan's commit adds,
    so the checks were replayed where an executor runs them: a linked
    worktree detached at `8709e8dc`, this plan's commit made there (the
    plan as drafted, the amendment appended), then Task 1 from this plan's
    blocks and here-documents, copied out by script, then Task 2's checks
    and the mutants at its tip; then the worktree removed. No branch was
    made or moved by it, and the scripts ran in a linked worktree, whose
    `.git` is a file (`os.path.exists(".git")`, 5.6c's fact-check F9).
27. **The mutants** (Task 2, Step 3): 15 breakages of the records' files,
    each with the checks that must catch it, all run in one run at one
    tip by the runner, which restores each file and checks `git status
    --short` before and after. Fourteen are caught, each by exactly the
    checks it should be; one, A1 (a figure in ADR 0062's Context changed),
    is accepted: the Context is filled, not drafted, so no check holds its
    words, only its citations, hashes and private text, and its figures
    are the review's (Review Focus 2). `split` is not among the mutants'
    checks: any edit of the six files fails it, by design, so it tells
    that the tree is the split's, not which part broke. Among the caught:
    a figure in the table changed (G1), by `place` and `guard`; a line of
    the plan changed (L1), by `planfile` alone; the CHANGELOG's note moved
    into `[0.5.0]` (N1), by `notes` and `place`; the master spec's note or
    ADR 0061's marker left out (M1, K1), by `files`, `place` and
    `redate.sh --check`.
28. **The split named by its blobs, not by a local commit.** The prototype
    and the split are local branches, never pushed (spec decision 14), so
    a commit hash of theirs would resolve for no one but this clone, and
    the records may name none (spec decision 16). `checks.py split D`
    hashes each of the six files, `D` put back to 2026-11-27, and
    compares the split's blobs, which it holds: no other text in those
    files holds a day after 2026-11-27. After the review's fixes, the
    files they change differ, and the ledger says which.
29. **No `cargo` command, and the PR template's boxes.** No Rust, test or
    workflow file changes, and no test reads these files (spec §5), so
    nothing local is built or run; CI's run on the pull request runs the
    suite, `selfhost.rs` and clippy on three systems, `cargo fmt` and the
    MSRV check (Task 2, Step 7). **The body ticks only what has run:** at
    the opening, "CHANGELOG.md updated"; the five `cargo` boxes after CI's
    run is green, each "CI's run <id> ran it"; "New tests
    added" not ticked, with the reason (no test reads these files; this
    plan's checks hold them).
30. **The review's fixes go into the records commit; after the push,
    nothing changes without the user's word.** Spec §7 has one records
    commit, last. So the one fix pass (Task 2, Step 5) folds its changes
    into the records commit before anything is pushed (decision 24's
    amend). The approved plan's commit is never changed: a finding against
    the approved text is recorded, as the amendment records the plan's
    `:243` (decision 34), and answered in the ledger. **A fix may not
    touch the parts the user ruled on:** the drafted blocks (`place`), the
    twelve decisions and the figures table (`guard`, byte for byte), and
    the plan (`planfile`); a finding against one of them stops the work,
    and the user is asked. **After the push, nothing is amended or
    force-pushed without the user's word.** If CI or a review needs a
    change then, the executor stops, and the main session asks the user,
    offering two ways: amend the records commit and force-push with lease,
    pinned to the pushed head, with git (`git push
    --force-with-lease=<branch>:<pushed head> origin <branch>`, `<branch>`
    this plan's branch), which keeps one records commit, as spec §7 has
    it; or a new commit after it, which the records' history then shows.
    No tag or release reads this branch's tree, so either way reaches
    everything.
31. **CI's lines and counts for the pull request** (spec §4): every one of
    CI's 19 jobs green, with the counts of #152's run 45829766657 (Task 2,
    Step 7); no Release run, since `release.yml` runs on a pull request
    only for the paths its `pull_request` trigger names
    (`.github/workflows/release.yml:15-27`), which hold none of the eight
    files; no Benchmarks run before the merge (`benchmarks.yml` runs on
    pushes to `main`, `.github/workflows/benchmarks.yml:3-5`). **Recorded,
    not expected:** sizes, the runtime's directory, the crates' bytes,
    every time, and the advisory step's known failures. Past a time line
    (80, 100 and 110 s for `selfhost.rs`, 75 s for the bootstrap step) the
    user is asked; on #152, macOS's 104.82 s was over its line, and the
    user ruled it the runner's (spec §4).
32. **The merge and Task 3.** By rebase, on the user's word, pinned to the
    head (Task 2, Step 8). Then `main`'s CI run is accepted by its
    `conclusion` and its head commit's tree being the pull request's last
    head's, as 5.6c accepted its own; its counts are read only if a job
    fails or the trees differ. Then the checks again on `main`'s tree,
    `hashcheck` against the fetched `origin/main` (spec §12 asks that §5's
    checks 2, 3 and 7 hold there: as decision 25 makes them, the records'
    hashes, ADR 0062's citations with their words, the plan's by line, and
    the notes); then spec §12's gate, and 6.0 done.
33. **The cut, as this plan makes it.** Task 0 is the baseline; Task 1 the
    two commits; Task 2 their verification, the review, the pull request,
    CI and the merge; Task 3 spec §12's gate after the merge. Spec §6's
    prototype was made before this plan (Checked on), and its measurement
    on CI is none (spec decision 14).
34. **The facts that proved wrong, and the outline as filled** **(differs;
    the amendment records each):**
    - **the records' day:** the drafts date the six places 2026-11-26;
      the records commit's day is 2026-11-27 at the earliest (the spec was
      approved that day), so the prototype dated them 2026-11-27, as
      §3.2's Status note and §5's check 6 allow, and `redate.sh` gives
      them the commit's day (decision 24); the history's 2026-11-26 stays;
    - **three drafts against the spike's report and the files:** decision
      3's limits, "every depth case at the parser's limit, 128, compiled
      and run", is "survived with no trap" (at 128, 15 kinds build and
      run, and the 16th, nested `fn` items, is E0900 natively too, as the
      figures table says); the plan's 6.0 note's "every depth case at 128
      run" is "survived", two lines rewrapped; decision 4's "Semver is
      strict after it (`:101`)" followed `docs/releasing.md`, whose `:101`
      is blank, and is "(the master spec's §1.9, `:101`)";
    - **decision 3's "No new crate or feature"** is "No new dependency"
      (decision 19);
    - **decision 2's last item,** by the user's ruling of 2026-11-27
      (decision 18): the draft's "what other specs give Phase 6" said more
      than its sources, and is two items, the two other specs give Phase 6
      and the three the specs and earlier ADRs put elsewhere, among them
      examples 08 to 15, where the draft and the research said 08 to 13
      (decision 19); spec `:358`, §3.7's row of the four places, calls them
      "Phase 6 items ADR 0062 leaves out", and two are; the approved plan's
      §3 decision 1 (`docs/phase-6-plan.md:279-283`) says what the draft
      said, 08 to 13 among it, and stays verbatim;
    - **two more of the plan's citations the records commit moves** (spec
      §3.1 lists five): `docs/phase-6-plan.md:341`'s `:985` (the master
      spec's §5.2, after §3.3's note) and `docs/phase-6-plan.md:703`'s ADR
      0061 `:430-436` (after §3.6's marker), `:703` as committed (the
      approved text's `:690`); the 6.0 note's "stay `4fe11d55`'s" covers
      them, and `pathcheck` reads all the plan's at `4fe11d55`;
    - **the approved plan's `:243`** cites ADR 0009 `:57` for "Nova gets
      concurrency, not multi-core execution", which is at `:105` (`:57` is
      the single-threaded executor, which `docs/phase-6-plan.md:753`, as
      committed, cites rightly: the approved text's `:740`): recorded in the
      amendment; the plan stays verbatim (spec §3.1 and its decision 7);
    - **spec §5's checks 2 and 3 on the spec's own text:** not scripted; its
      citations and hashes were checked once, by hand, and the plan's
      citations are held by line alone (decision 25);
    - **the outline, filled:** Context is 68 lines, against 40 (a line for
      each of the research's eleven items, and P1 to P12 labelled); the
      Decision 104 lines (125), the figures 36 (45), Consequences 18 (28),
      References 12 (14): ADR 0062 is 256 lines, within §3.2's 240 to 300;
      References name this plan (decision 19);
    - **the cut:** the approved plan's commit third (decision 20).
35. **The spec's recommendations, not asked, as taken** (spec §11, 1 to
    16): the ADR's file name; the limits, met locally, CI's figures
    6.1's; std's checked items not kept across compiles; Chromium
    measured locally; the 1.78 build and the `wasm-opt` size left to 6.1;
    the figures cited as a spike's; the plan verbatim, its citations
    `4fe11d55`'s; a dated note in the plan's 6.0 entry; the master spec's
    note "Amended", §7 untouched; the CHANGELOG note dated, holding 5.0a's
    sub-phase line; dated markers adding lines only; pre-1.0 RFCs as the
    plan's §3 decision 1 words them; four commits (decision 20); no draft
    pull request; ADR 0062 dated by its commit's day, citing no pull
    request of its own; `main`'s commits only.
36. **The branch starts from `8709e8dc`:**
    `claude/loving-archimedes-frtf5q-60` from `8709e8dc` (`main`
    `4fe11d55` and the approved spec), then this plan's commit, `Plan
    Phase 6.0: the plan and ADR 0062`, which adds this file and appends
    the spec's dated amendment (2026-11-27): the facts that proved wrong
    and the outline as filled (decision 34), and §7's cut as this plan
    makes it (decisions 20 and 33). Then Task 1's two commits. The blocks
    apply to the plan's commit as to `8709e8dc`.

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-60`: from `8709e8dc`, then
this plan's commit, which adds this file and appends the spec's dated
amendment, so the blocks apply to it as to `8709e8dc` (decision 36). Each
step runs in a shell set up as the conventions say.

- [ ] **Step 1: The tree.**

  ```sh
  git status --short
  git branch --show-current
  git log --format='%h %s' -2
  git diff --name-only 8709e8dc HEAD
  git diff --name-only 4fe11d55 HEAD
  git rev-parse origin/main
  git rev-parse --is-shallow-repository; cat "$(git rev-parse --git-path shallow)"
  git merge-base --is-ancestor a71d27d9 origin/main && echo "a71d27d9 on origin/main"
  git rev-parse --short HEAD
  ```

  Expected:
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-60`;
  - the log: this plan's commit (`Plan Phase 6.0: the plan and ADR 0062`,
    spec §7's subject; a subject that adds ", and the spec's dated
    amendment" is the same commit, as 5.6c's ruling R0.1 found), then
    `8709e8dc docs: the Phase 6.0 spec is approved`;
  - the names against `8709e8dc`:
    `docs/superpowers/plans/2026-11-27-phase-6-0-the-plan-and-adr-0062.md`
    and `docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`;
    against `4fe11d55`, the same two: the spec's commit added the spec
    alone;
  - `4fe11d558b5d48c9c34fef79823045b6907825fe`, Phase 5's records commit
    (#152);
  - `true`, and the shallow file's one line,
    `a71d27d9167198b0f05baa29b2e475fa359a456d`; then `a71d27d9 on
    origin/main`;
  - the plan's commit, recorded in the ledger as `BASE`. Task 2's review
    reads from it.

  Then the disk check (Conventions), `Task 0 Step 1, the start`: at least
  6,144 MiB.
- [ ] **Step 2: The tools and the process limit.** `python3 --version`,
  `git --version` and `bash --version | head -n 1` print Python 3.13.18,
  git 2.43.0 and GNU bash 5.2.21 here (any Python 3.8 or later does).
  The process limit, as the env file sets it: `id -u` prints `0`; `grep
  nova-limit /proc/self/cgroup` prints `N:pids:/nova-limit`; `cat
  /sys/fs/cgroup/pids/nova-limit/pids.max` prints `600`; `echo "$TMPDIR"`
  prints `$P/exec60/tmp`, written out. Record them.
- [ ] **Step 3: The base's records.**

  ```sh
  ls docs/adr/0062-phase-6-scope.md docs/phase-6-plan.md
  bash .github/scripts/release-notes.sh 0.5.0 CHANGELOG.md > "$P/exec60/notes-base.md"; echo "exit $?"
  wc -l < "$P/exec60/notes-base.md"
  bash .github/scripts/release-notes.sh Unreleased CHANGELOG.md; echo "exit $?"
  grep -c 'claude/loving-archimedes-frtf5q-60' CHANGELOG.md docs/adr/0048-phase-5-scope-and-backend.md docs/adr/0061-phase-5-boundary.md nova-spec/00-MASTER-SPEC.md
  sed -n '940p;942,944p' nova-spec/00-MASTER-SPEC.md
  ```

  Expected: `ls` fails twice (`No such file or directory`): no ADR 0062
  and no phase plan yet; `exit 0` and `627` lines, `[0.5.0]`'s notes as
  `4fe11d55` holds them; `exit 1`, `release-notes: CHANGELOG.md has no
  notes for Unreleased` (spec §2: `[Unreleased]` is empty); `0` for each
  of the four files: no note of this branch yet; and the master spec's
  item 5 (`5. Interactive playground (compile in browser via WASM-compiled
  Nova compiler)`), item 7, a blank line and `---`, where §3.3's note
  goes.
- [ ] **Step 4: The ledger.** Start `$P/ledger-60.md`, headed with
  `BASE`, `origin/main`, the shallow boundary, the tools' versions, the
  process limit's form, `TMPDIR` and the disk; and `$P/exec60`, the
  executor's directory (Conventions).

### Task 1: The approved plan, and the records commit

Spec §3.1, then §3.2 to §3.6, in two commits (decision 20): `docs: the
Phase 6 plan, approved`, from `t1-plan`; then `docs: ADR 0062, Phase 6's
scope, and the notes for 6.0`, from `t1-adr` and `t1-notes` (decision
21), dated by `redate.sh` (decision 24). Each step runs in a shell set up
as the conventions say.

- [ ] **Step 1: The messages and the scripts.** The two messages, written
  as they stand:

```sh
mkdir -p "$P/exec60"
cat > "$P/exec60/msg-t1-plan.txt" <<'EOF'
docs: the Phase 6 plan, approved

The user approved the Phase 6 plan on 2026-11-26. Its text, byte for
byte, as docs/phase-6-plan.md: Phase 6, "1.0 Release", in sub-phases
6.0 to 6.11. Its path:line citations are 4fe11d55's, as its status
says; the draft was never committed.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
EOF
```

```sh
mkdir -p "$P/exec60"
cat > "$P/exec60/msg-t1.txt" <<'EOF'
docs: ADR 0062, Phase 6's scope, and the notes for 6.0

ADR 0062, Phase 6's scope: all seven items, the playground as S0 in the
browser with the spike's figures, and 1.0.0 as the phase's release;
dated notes under the master spec's Phase 6, atop [Unreleased] and after
the plan's 6.0 entry; and markers in ADRs 0048 and 0061 at the mapping
ADR 0062 amends. Records only, as the 6.0 spec's §3.2 to §3.6 draft
them, with its amendment's fixes.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
EOF
```

  `redate.sh` (decision 24), written as it stands:

```sh
mkdir -p "$P/exec60"
cat > "$P/exec60/redate.sh" <<'EOF'
#!/usr/bin/env bash
# redate.sh [--check] DATE: date the records commit DATE (YYYY-MM-DD, the
# commit's day in UTC). The blocks date the records 2026-11-27 in exactly six
# places, each found by the words around it: ADR 0062's Status, "Accepted
# (DATE). Branch"; the master spec's note, "**Amended DATE (branch"; the
# CHANGELOG's note, "**Recorded DATE (branch"; the plan's 6.0 note,
# "**Amended DATE (6.0, branch"; and the markers in ADRs 0048 and 0061,
# "**Amended DATE (branch", each at its indentation. Each is rewritten
# whatever date it holds, so the script can run again; with --check nothing
# is written. Then it checks, against 4fe11d55 (on main, whose files the
# records change): each place holds DATE; no line the records remove holds a
# date; and every other date in the lines they add is history, exactly as
# written, whatever DATE is: the user's decisions, the plan's approval and
# the spike, all of 2026-11-26 (six times in ADR 0062, seven in the plan,
# twice in the master spec's note, once in ADR 0061's marker), and issue
# #3's 2026-08-12 in the plan. The spec's approval (2026-11-27) and the
# release's days are in no line the records add. Dates in file names
# (DATE-phase-...) are names, not dates. The messages carry no date of the
# records. Run from the repository's root, after the blocks are applied,
# before or after the commit.
set -euo pipefail
MODE=write
if [ "${1:-}" = --check ]; then MODE=check; shift; fi
D=${1:?usage: redate.sh [--check] DATE}
[[ $D =~ ^20[0-9][0-9]-[01][0-9]-[0-3][0-9]$ ]] && [ "$(date -u -d "$D" +%F 2>/dev/null)" = "$D" ] ||
  { echo "redate: not a date: $D" >&2; exit 1; }
MODE=$MODE D=$D python3 - <<'PY'
import collections, os, re, subprocess, sys
D, MODE = os.environ["D"], os.environ["MODE"]
DATE = r"\d{4}-\d{2}-\d{2}"
B = re.escape("`claude/loving-archimedes-frtf5q-60`")
PLACES = [
    ("ADR 0062's Status", "docs/adr/0062-phase-6-scope.md",
     r"(?m)^(Accepted \()(%s)(\)\. Branch %s)$" % (DATE, B)),
    ("the master spec's note", "nova-spec/00-MASTER-SPEC.md",
     r"(?m)^(\*\*Amended )(%s)( \(branch %s\):\*\*\nPhase 6 follows )" % (DATE, B)),
    ("the CHANGELOG's note", "CHANGELOG.md",
     r"(?m)^(\*\*Recorded )(%s)( \(branch %s\):\*\*\nPhase 6\.0, )" % (DATE, B)),
    ("the plan's 6.0 note", "docs/phase-6-plan.md",
     r"(?m)^(\*\*Amended )(%s)( \(6\.0, branch %s,)$" % (DATE, B)),
    ("ADR 0048's marker", "docs/adr/0048-phase-5-scope-and-backend.md",
     r"(?m)^(     \*\*Amended )(%s)( \(branch %s\):\*\*)$" % (DATE, B)),
    ("ADR 0061's marker", "docs/adr/0061-phase-5-boundary.md",
     r"(?m)^(  \*\*Amended )(%s)( \(branch %s\):\*\*)$" % (DATE, B)),
]
HISTORY = {
    "docs/adr/0061-phase-5-boundary.md": {"2026-11-26": 1},
    "docs/adr/0062-phase-6-scope.md": {"2026-11-26": 6},
    "docs/phase-6-plan.md": {"2026-08-12": 1, "2026-11-26": 7},
    "nova-spec/00-MASTER-SPEC.md": {"2026-11-26": 2},
}
bad = 0
for name, path, rx in PLACES:
    text = open(path, encoding="utf-8").read()
    n = len(re.findall(rx, text))
    if MODE == "write" and n == 1:
        text = re.sub(rx, lambda m: m.group(1) + D + m.group(3), text)
        open(path, "w", encoding="utf-8").write(text)
    got = [m.group(2) for m in re.finditer(rx, text)]
    ok = got == [D]
    bad += not ok
    print(f"redate: {name} ({path}): {', '.join(got) or 'NOT FOUND'}{'' if ok else '  <- FAILED'}")
def lines(sign):
    d = subprocess.run(["git", "-c", "core.quotepath=off", "diff", "--no-color", "--no-ext-diff",
                        "--src-prefix=a/", "--dst-prefix=b/", "-U0", "4fe11d55", "--", ".",
                        ":(exclude)docs/superpowers"], capture_output=True, text=True, encoding="utf-8", check=True).stdout
    out, f, hunk = [], None, False
    for l in d.split("\n"):
        if l.startswith("diff --git "):
            f, hunk = None, False
        elif not hunk and l.startswith("+++ "):
            f = l[6:] if l.startswith("+++ b/") else None
        elif l.startswith("@@"):
            hunk = True
        elif hunk and l.startswith(sign):
            out.append((f, l[1:]))
    if sign == "+":
        u = subprocess.run(["git", "ls-files", "--others", "--exclude-standard"], capture_output=True,
                           text=True, encoding="utf-8", check=True).stdout.split()
        for p in u:
            if not p.startswith("docs/superpowers/"):
                out += [(p, l) for l in open(p, encoding="utf-8").read().splitlines()]
    return out
texts = collections.defaultdict(str)
for f, l in lines("+"):
    texts[f] += l + "\n"
for name, path, rx in PLACES:
    texts[path] = re.sub(rx, lambda m: m.group(1) + "PLACE" + m.group(3), texts[path], count=1)
found = {f: dict(collections.Counter(re.findall(r"(%s)(?!-phase)" % DATE, t))) for f, t in texts.items()}
found = {f: c for f, c in found.items() if c}
for f in sorted(set(found) | set(HISTORY)):
    ok = found.get(f, {}) == HISTORY.get(f, {})
    bad += not ok
    print(f"redate: history in {f}: {', '.join(f'{k} {v}' for k, v in sorted(found.get(f, {}).items()))}"
          + ("" if ok else f"  <- FAILED, expected {HISTORY.get(f, {})}"))
gone = [(f, l) for f, l in lines("-") if re.search(DATE, l)]
for f, l in gone:
    print(f"redate: FAILED: a removed line holds a date: {f}: {l.strip()[:80]}")
bad += len(gone)
print("redate: ok" if not bad else "redate: FAILED")
sys.exit(1 if bad else 0)
PY
EOF
```

  `checks.py` (decision 25), written as it stands: the records' checks,
  each `CHK NAME …` (the env file's `CHK`), each from the repository's
  root, each ending `NAME: ok` and exit 0, or `NAME: FAILED` and exit 1:

```sh
mkdir -p "$P/exec60"
cat > "$P/exec60/checks.py" <<'EOF'
#!/usr/bin/env python3
"""checks.py CHECK [ARGS]: the records' checks (spec §5), run from the
repository's root. Each reads the working tree against BASE, `4fe11d55` on
`main`, whose six files Task 1 changes: an edit is seen before it is
committed (the mutants are such edits), and a clean tree at the records
commit is that commit. The "added lines" are the lines `git diff BASE` adds
outside docs/superpowers, and every line of an untracked file there: the
whole of docs/phase-6-plan.md, a new file, among them. Each check prints
what it found and a last line `CHECK: ok` or `CHECK: FAILED`, and exits 0
or 1.

  hashcheck [--msg FILE]...  every word of 7 to 40 hex digits in the added
      lines and the messages is a commit on `main`, by 8 characters
      (`git merge-base --is-ancestor` against origin/main; the clone is
      shallow at `a71d27d9`, and the records name no commit older than it),
      or a word this table classifies (none yet). Anything else fails.
  pathcheck   ADR 0062's backticked `path:line` citations: each in CITES,
      on its line, resolving in the tree, its lines holding the words; the
      plan's, read at BASE (the plan is committed verbatim, and its 6.0
      note says its citations stay `4fe11d55`'s): each resolving there to
      lines that exist and are not blank, its short form `:N` by PLAN_SHORT;
      no `path:line` in the notes or markers; every backticked repository
      path in the added lines in the tree, or a name NOT_PATHS lists; every
      Markdown link resolving; no URL. CITES is keyed by ADR 0062's line,
      so an edit that adds or removes a line above a citation moves every
      key after it: run renumber, then pathcheck.
  renumber    rewrite CITES' keys in this script from ADR 0062 in the tree,
      by the citations' order: the k-th backticked `path:line` in the ADR
      takes the table's k-th row. It writes nothing, and fails, if the
      ADR's citations differ from the table's in number or order (a
      citation added, removed or changed is a row added, removed or
      changed by hand). pathcheck then checks each row's words again.
  privacy [--msg FILE]...  no local or scratch path, branch but the
      records' own, person's name or email (built here from the remote, the
      committers on `main` and the git config, never written), model
      identifier, session link or token, in the added lines or the messages
      (their two trailer lines excepted); the owner's login only in the
      repository's address and the plan's Pages address.
  notes D     release-notes.sh 0.5.0 exits 0 and prints BASE's notes, the
      same lines; release-notes.sh Unreleased exits 0 and prints a blank
      line and spec §3.4's note, dated D, nothing else.
  files       exactly the six files differ from BASE outside
      docs/superpowers, and no other file is untracked.
  place D     spec §3.2 to §3.6's drafted blocks, dated D where the
      commit's day goes, with FIXES (the amendment's: five lines, and
      decision 2's last item, six lines for nine, by the user's ruling),
      each once, at its place; ADR 0062's title as spec §3.2's table
      gives it.
  planfile    docs/phase-6-plan.md without its lines 427 to 439 (the 6.0
      note and the blank after it) is the approved text, byte for byte (its
      blob, APPROVED); the first commit after BASE that touches it adds it
      as that blob, `docs: the Phase 6 plan, approved`.
  guard       ADR 0062's twelve decisions and its figures table hash to the
      split's (GUARD): byte for byte, after any fold.
  split D     each of the six files, with D put back to 2026-11-27 (the
      day the split's records carry; no other text of theirs holds a later
      day), hashes to the split's blob: the records are the split's.
"""
import collections
import hashlib
import os
import re
import subprocess
import sys
import tempfile

BASE = "4fe11d55"
MAIN = "origin/main"
BRANCH = "claude/loving-archimedes-frtf5q-60"
SPEC = "docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md"
ADR = "docs/adr/0062-phase-6-scope.md"
PLAN = "docs/phase-6-plan.md"
MS = "nova-spec/00-MASTER-SPEC.md"
A48 = "docs/adr/0048-phase-5-scope-and-backend.md"
A61 = "docs/adr/0061-phase-5-boundary.md"
SIX = ["CHANGELOG.md", A48, A61, ADR, PLAN, MS]
TRAILERS = ["Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
            "Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN"]
CLASSIFIED = {}  # hex words that are not commits: none in these records
APPROVED = "d891448dd75bf0e84ee55af6ddf95a56aed716d4"  # the approved plan's blob, 44,866 bytes, 750 lines
SPLIT = {  # the six files' blobs at the split's tip, whose records are dated 2026-11-27
    "CHANGELOG.md": "fd1e988efbcb79be43e6a1c86c2c4b19bd85c84e",
    A48: "b30a650942b3f37692e68314f5f1d1d29915d9a9",
    A61: "dbba0e40757b9211fba87581d45e60f4f1ba1a6f",
    ADR: "417606d560ea3f07634af907b81a89775098cdd8",
    PLAN: "ab378414e0a872983d76dc9cec2ccfc8b7fd3b65",
    MS: "74608f99c79951b77a28cbb52c4b394ecfcb81fd",
}
GUARD = {  # sha256 of ADR 0062's parts at the split's tip
    "the twelve decisions": "5129d6be65048c41c93beb8aced193b0498960d155dc8552ed5e3f483f2d41be",
    "the figures table": "4f637b047de32625a1bf346b6410ea4d2edc475e6ad1edd1bd9ffd8d10f89acb",
}
# The spec's drafted lines that the records hold otherwise, each as the
# spec's amendment records it: (spec line, the draft's line, the records'),
# or, for a fix of several lines, (its first spec line, the draft's lines,
# the records' lines).
FIXES = [
    (119, ["   - what other specs give Phase 6 and its list does not name: multiple",
           "     toolchains (`nova-spec/40-TOOLING.md:646`; ADR 0037 `:93`),",
           "     `nova-embed` (`nova-spec/13-RUNTIME.md:480`), the `setup-nova`",
           "     action (`40-TOOLING.md:583-598`), examples 08 to 13",
           "     (`nova-spec/60-EXAMPLES.md:801-812`), and edition rules",
           "     (`40-TOOLING.md:436-439`)."],
          ["   - what other specs give Phase 6 and its list does not name: multiple",
           "     toolchains (`nova-spec/40-TOOLING.md:646`; ADR 0037 `:93`) and",
           "     `nova-embed` (`nova-spec/13-RUNTIME.md:480`);",
           "   - what the specs and earlier ADRs put elsewhere, not in Phase 6, left",
           "     there: the `setup-nova` action (`40-TOOLING.md:583-598`), on Phase",
           "     3's backlog (ADR 0026 `:50`; ADR 0037 `:100`); examples 08 to 15",
           "     (`nova-spec/60-EXAMPLES.md:801-803`), \"post-Phase 6\", after the 1.0",
           "     release; and edition rules (`40-TOOLING.md:436-439`), on Phase 3's",
           "     backlog (ADR 0037 `:97`)."]),
    (138, "     case at the parser's limit, 128, compiled and run. 6.1's gate",
          "     case at the parser's limit, 128, survived with no trap. 6.1's gate"),
    (150, "     hook in the shim; the target in CI. No new crate or feature.",
          "     hook in the shim; the target in CI. No new dependency."),
    (156, "   after it (`:101`).",
          "   after it (the master spec's §1.9, `:101`)."),
    (321, "about 34 ms after; every depth case at 128 run, in a worker too. It built",
          "about 34 ms after; every depth case at 128 survived, in a worker too. It"),
    (322, "on stable alone; the build on 1.78, the size after `wasm-opt` and",
          "built on stable alone; the build on 1.78, the size after `wasm-opt` and"),
]
CITES = [
    (18, ':936-942', 'nova-spec/00-MASTER-SPEC.md', ['1. Security audit (external)', '7. Launch posts']),
    (22, ':940', 'nova-spec/00-MASTER-SPEC.md', ['WASM-compiled Nova compiler']),
    (23, ':935-943', 'nova-spec/00-MASTER-SPEC.md', ['### Phase 6', '1.0 Release (week 105']),
    (24, ':99-102', 'nova-spec/00-MASTER-SPEC.md', ['1.9 Versioning & Stability', 'Pre-1.0:** breaking changes allowed in minor versions, RFC required', 'Post-1.0:** semver strict', 'Deprecation:** minimum 2 minor versions before removal']),
    (31, ':44', '0048', ["N built for WASM, which Phase 6's playground needs"]),
    (31, ':93', '0061', ['R5:** Phase 6 gets N built for WASM alone']),
    (32, ':148', '0061', ["N built for WASM, which Phase 6's playground needs", 'Phase 6: its playground, item 5']),
    (32, ':394-395', '0061', ['Mapped elsewhere, to Phase 6 alone', 'N built for WASM, for the playground']),
    (33, ':940', 'nova-spec/00-MASTER-SPEC.md', ['WASM-compiled Nova compiler']),
    (48, 'crates/nova-driver/src/analyze.rs:25-42', None, ["an editor's buffer if one is open, else the file", 'impl Sources for DiskSources']),
    (49, 'lib.rs:865-892', 'crates/nova-driver/src/lib.rs', ['pub fn build_program_wasm', 'compile_module', 'std::fs::write']),
    (64, ':521-534', '0057', ['Native only', "WASM's MIR differs"]),
    (65, ':42-43', '0040', ['A long computation within one turn does not collect', 'out of memory']),
    (102, 'nova-spec/40-TOOLING.md:646', None, ['Multiple toolchains', '(Phase 6)']),
    (102, ':93', '0037', ['Multiple toolchains', 'Phase 6']),
    (103, 'nova-spec/13-RUNTIME.md:480', None, ['`nova-embed` (Phase 6)']),
    (105, '40-TOOLING.md:583-598', 'nova-spec/40-TOOLING.md', ['novalang/setup-nova@v1']),
    (106, ':50', '0026', ["§9's CI template and its `setup-nova` action"]),
    (106, ':100', '0037', ['The CI template and its `setup-nova` action', 'ADR 0026; backlog']),
    (107, 'nova-spec/60-EXAMPLES.md:801-803', None, ['Additional Examples (post-Phase 6)', 'after 1.0 release']),
    (108, '40-TOOLING.md:436-439', 'nova-spec/40-TOOLING.md', ['Edition rules are out of Phase 3']),
    (109, ':97', '0037', ['edition rules', 'ADRs 0026, 0031; backlog']),
    (117, ':44', '0048', ["N built for WASM, which Phase 6's playground needs"]),
    (117, ':148', '0061', ["N built for WASM, which Phase 6's playground needs", 'Phase 6: its playground, item 5']),
    (117, ':394-395', '0061', ['Mapped elsewhere, to Phase 6 alone', 'N built for WASM, for the playground']),
    (120, ':940', 'nova-spec/00-MASTER-SPEC.md', ['WASM-compiled Nova compiler']),
    (128, 'crates/nova-codegen-wasm/src/lib.rs:61-63', None, ['structuring recurses once per nested join', 'const STACK: usize = 256 << 20;']),
    (134, 'lib.rs:198-208', 'crates/nova-codegen-wasm/src/lib.rs', ['pub fn compile_module', 'std::thread::scope', 'stack_size(STACK)']),
    (141, ':101', 'nova-spec/00-MASTER-SPEC.md', ['Post-1.0:** semver strict']),
    (149, ':100', 'nova-spec/00-MASTER-SPEC.md', ['RFC required']),
    (155, ':180-184', '0047', ['`std/dom` on the site goes to the backlog']),
    (167, 'nova-spec/40-TOOLING.md:626', None, ['there is no `novalang.dev`']),
    (173, ':391-393', '0061', ["S0's bugs, kept:** std's bodies see a program's impls", 'float `%` type-checks and fails in `build --release`']),
    (198, 'crates/nova-codegen-wasm/src/lib.rs:198', None, ['pub fn compile_module']),
    (254, 'crates/nova-codegen-wasm/src/lib.rs:61-63', None, ['structuring recurses once per nested join', 'const STACK: usize = 256 << 20;']),
    (254, ':198-208', 'crates/nova-codegen-wasm/src/lib.rs', ['pub fn compile_module', 'std::thread::scope', 'stack_size(STACK)']),
    (255, 'crates/nova-driver/src/analyze.rs:25-42', None, ["an editor's buffer if one is open, else the file", 'impl Sources for DiskSources']),
    (255, 'lib.rs:865-892', 'crates/nova-driver/src/lib.rs', ['pub fn build_program_wasm', 'compile_module', 'std::fs::write']),
]
# The plan's short citations `:N`, by the plan's line at the tip: the file
# each names, read off its sentence (an ADR by its number).
PLAN_SHORT = {
    (4, ":935-942"): MS, (5, ":99-102"): MS, (5, ":996"): MS, (5, ":1126-1134"): MS, (18, ":936-942"): MS,
    (31, ":100-102"): MS, (32, ":996"): MS, (33, ":1134"): MS, (158, ":48-53"): "0031", (181, ":20-23"): "0031",
    (188, ":1220-1221"): "0056", (189, ":885-886"): "0058", (199, ":170-185"): "0047", (203, ":182-185"): MS,
    (204, ":186-188"): MS, (209, ":626"): "nova-spec/40-TOOLING.md", (218, ":38-41"): "0039", (220, ":70"): "0039",
    (221, ":105-107"): "0039", (233, ":198-208"): "crates/nova-codegen-wasm/src/lib.rs", (238, ":521-534"): "0057",
    (239, ":42-43"): "0040", (243, ":57"): "0009", (252, ":226-231"): "docs/releasing.md",
    (256, ":25"): "nova-spec/20-STDLIB.md", (261, ":1205-1211"): "0056", (262, ":205"): "0061", (275, ":44"): "0048",
    (276, ":394-395"): "0061", (280, ":93"): "0037", (287, ":100"): MS, (297, ":940"): MS, (306, ":1134"): MS,
    (332, ":180-185"): "0047", (341, ":985"): MS, (361, ":1205-1211"): "0056", (693, ":205"): "0061",
    (699, ":340-353"): "0061", (703, ":430-436"): "0061", (747, ":377-386"): "0061", (753, ":57"): "0009",
}
NOT_PATHS = {  # backticked names in the added lines that are not repository paths, or name what is not there yet
    "nova.lock": "a project's lock file", "app.js": "a build's output", "/nova/": "the Pages site's base",
    "SECURITY.md": "named as missing, 6.2's", "deny.toml": "named as missing", "fuzz/": "named as missing",
    "docs/spec/": "named as missing (master spec §2)", "docs/book/": "named as missing (master spec §2)",
    "docs/book/src/SUMMARY.md": "6.8's, to come", "docs/stability.md": "6.2's, to come",
    "docs/rfcs/README.md": "6.2's, to come", "docs/security/": "6.6's, to come", "docs/errors/": "6.7's, to come",
    "docs/reference/": "6.7's, to come", "docs/launch/": "6.10's, to come", "std/regex": "a std module 6.4 adds",
    "actions/configure-pages": "a GitHub Action 6.8 may ask for",
    "actions/upload-pages-artifact": "a GitHub Action 6.8 may ask for",
    "actions/deploy-pages": "a GitHub Action 6.8 may ask for",
    BRANCH: "the records' branch, not a path",
}
CITE = re.compile(r"`([^`\s]*):(\d+)(?:-(\d+))?`")


def git(*args, check=True):
    r = subprocess.run(["git", *args], capture_output=True, text=True, encoding="utf-8",
                       errors="replace")
    if check and r.returncode != 0:
        sys.exit(f"git {' '.join(args)}: {r.stderr.strip()}")
    return r


def untracked():
    return [f for f in git("ls-files", "--others", "--exclude-standard").stdout.split("\n") if f]


def diff_lines():
    """(added, removed): lists of (file, line) against BASE, outside docs/superpowers."""
    d = git("-c", "core.quotepath=off", "diff", "--no-color", "--no-ext-diff", "--no-renames",
            "--src-prefix=a/", "--dst-prefix=b/", "-U0", BASE, "--", ".",
            ":(exclude)docs/superpowers").stdout
    added, removed, f, hunk = [], [], None, False
    for l in d.split("\n"):
        if l.startswith("diff --git "):
            f, hunk = None, False
        elif not hunk and l.startswith("+++ "):
            f = l[6:] if l.startswith("+++ b/") else None
        elif l.startswith("@@"):
            hunk = True
        elif hunk and l.startswith("+"):
            added.append((f, l[1:]))
        elif hunk and l.startswith("-"):
            removed.append((f, l[1:]))
    for u in untracked():
        if not u.startswith("docs/superpowers/"):
            added += [(u, l) for l in open(u, encoding="utf-8", errors="replace").read().splitlines()]
    return added, removed


def message(argv):
    out = []
    for k, a in enumerate(argv):
        if a == "--msg":
            out += [l for l in open(argv[k + 1], encoding="utf-8").read().split("\n") if l not in TRAILERS]
    return out


def done(name, bad):
    print(f"{name}: {'ok' if not bad else 'FAILED'}")
    sys.exit(1 if bad else 0)


def read(path):
    lines = open(path, encoding="utf-8").read().split("\n")
    if lines and lines[-1] == "":
        lines.pop()
    return lines


def flat(lines):
    return re.sub(r"\s+", " ", " ".join(x.strip() for x in lines))


def hashcheck(argv):
    lines = diff_lines()[0] + [("the message", l) for l in message(argv)]
    words = collections.OrderedDict()
    for f, l in lines:
        for m in re.finditer(r"(?<![0-9A-Za-z_])[0-9a-f]{7,40}(?![0-9A-Za-z_])", l):
            words.setdefault(m.group(0), []).append(f)
    bad, kinds = 0, collections.Counter()
    for w, fs in words.items():
        if w in CLASSIFIED:
            kind, verdict = "other words", CLASSIFIED[w]
        else:
            full = git("rev-parse", "--verify", "--quiet", w + "^{commit}", check=False).stdout.strip()
            if not full:
                kind, verdict = "BAD", "FAIL: neither a commit here nor a word this table classifies"
            elif git("merge-base", "--is-ancestor", full, MAIN, check=False).returncode == 0:
                kind, verdict = "on main", f"commit on {MAIN}: {git('log', '-1', '--format=%s', full).stdout.strip()[:60]}"
            else:
                kind, verdict = "BAD", (f"FAIL: commit {full[:12]} is not on {MAIN} (if it is older than "
                                        "the shallow boundary a71d27d9, the main session checks it with GitHub)")
        if kind == "on main" and len(w) != 8:
            kind, verdict = "BAD", f"FAIL: a commit named by {len(w)} characters, not 8 (spec decision 16)"
        bad += kind == "BAD"
        kinds[kind] += len(fs)
        print(f"{w}\t{len(fs)}x, {', '.join(sorted(set(fs)))}\t{verdict}")
    print(f"hashcheck: {len(lines)} lines read, {len(words)} distinct words; occurrences: "
          + (", ".join(f"{k} {v}" for k, v in sorted(kinds.items())) or "none"))
    done("hashcheck", bad)


def target(path, token):
    if path is None:
        return token.rsplit(":", 1)[0]
    if re.fullmatch(r"\d{4}", path):
        g = [f for f in os.listdir("docs/adr") if f.startswith(path + "-")]
        return f"docs/adr/{g[0]}" if len(g) == 1 else f"docs/adr/{path}-?.md"
    return path


def base_files():
    return set(git("ls-tree", "-r", "--name-only", BASE).stdout.split("\n")) - {""}


def plan_rows(plan):
    """The plan's `path:line` citations: (line, token, file at BASE or None)."""
    tree0 = base_files()
    rows = []
    for i, l in enumerate(plan, 1):
        for m in CITE.finditer(l):
            p, r = m.group(1), m.group(0)[1:-1].rsplit(":", 1)[1]
            if p:
                f = p if p in tree0 else None
                if f is None:
                    hits = sorted(x for x in tree0 if x.endswith("/" + p))
                    f = hits[0] if len(hits) == 1 else None
            else:
                k = PLAN_SHORT.get((i, ":" + r))
                f = None if k is None else (next((x for x in tree0 if x.startswith(f"docs/adr/{k}-")), None)
                                            if re.fullmatch(r"\d{4}", k) else k)
            rows.append((i, m.group(0)[1:-1], f, r))
    return rows


def pathcheck(argv):
    bad = 0
    adr = read(ADR)
    for n, token, path, words in CITES:
        p = target(path, token)
        if n > len(adr) or f"`{token}`" not in adr[n - 1]:
            print(f"FAIL ADR:{n} `{token}`: not on that line")
            bad += 1
            continue
        m = CITE.fullmatch(f"`{token}`")
        a, b = int(m.group(2)), int(m.group(3) or m.group(2))
        if not os.path.isfile(p):
            print(f"FAIL ADR:{n} `{token}`: {p} is not in the tree")
            bad += 1
            continue
        lines = read(p)
        seg = lines[a - 1:b]
        missing = [w for w in words if w not in flat(seg)] if b <= len(lines) else words
        print(f"{'FAIL' if missing else 'ok'} ADR:{n} `{token}` -> {p}:{a}-{b}"
              + (f" missing {missing!r}" if missing else ""))
        bad += bool(missing)
    held = collections.Counter((n, t) for n, t, _, _ in CITES)
    found = collections.Counter((n, m.group(0)[1:-1]) for n, l in enumerate(adr, 1) for m in CITE.finditer(l))
    for (n, t), k in sorted((found - held).items()):
        print(f"FAIL ADR:{n} `{t}`: a citation the table does not hold")
        bad += 1
    plan = read(PLAN)
    rows = plan_rows(plan)
    moved = 0
    for i, t, f, r in rows:
        if f is None:
            print(f"FAIL plan:{i} `{t}`: not resolved at {BASE}")
            bad += 1
            continue
        old = git("show", f"{BASE}:{f}").stdout.split("\n")[:-1]
        a, _, b = r.partition("-")
        a, b = int(a), int(b or a)
        seg = old[a - 1:b]
        good = 1 <= a <= b <= len(old) and any(x.strip() for x in seg)
        now = read(f) if os.path.isfile(f) else []
        same = now[a - 1:b] == seg
        moved += not same
        first = next((x.strip() for x in seg if x.strip()), "")[:60]
        print(f"{'ok' if good else 'FAIL'} plan:{i} `{t}` -> {f}:{r} at {BASE} [{first}]"
              + ("" if same else "  (reads otherwise in the tree; the plan's 6.0 note keeps 4fe11d55's)"))
        bad += not good
    used = {(i, ":" + r) for i, t, f, r in rows if t.startswith(":")}
    for k in sorted(set(PLAN_SHORT) - used):
        print(f"FAIL PLAN_SHORT {k}: no such citation in the plan")
        bad += 1
    added = diff_lines()[0]
    for f, l in added:
        if f not in (ADR, PLAN) and CITE.search(l):
            print(f"FAIL {f}: a path:line in a note or marker: {l.strip()[:80]}")
            bad += 1
    text = "\n".join(l for _, l in added)
    tracked = set(git("ls-files").stdout.split("\n")) | set(untracked())
    dirs = {"/".join(p.split("/")[:k]) + "/" for p in tracked for k in range(1, p.count("/") + 1)}
    paths, kinds = collections.OrderedDict(), collections.Counter()
    for s in re.findall(r"`([^`\s]+)`", text):
        tok = s.split(":")[0] if re.search(r":\d", s) else s
        if "://" in tok or tok.startswith(("-", "/", "~", "$")) and tok != "/nova/" or \
                tok.startswith(".") and not tok.startswith(".github"):
            continue
        if "/" in tok or re.search(r"\.(md|rs|toml|lock|ya?ml|js|nova|c|sh|ps1|json|mjs|cjs)$", tok):
            paths[tok] = paths.get(tok, 0) + 1
    for tok, k in paths.items():
        t2 = tok.rstrip("/")
        hits = sorted(x for x in tracked if x.endswith("/" + t2))
        if os.path.exists(tok) and (tok in tracked or tok in dirs or t2 + "/" in dirs):
            kind, note = "ok", ""
        elif len(hits) == 1:
            kind, note = "ok", f" = {hits[0]} (a suffix)"
        elif "/" not in tok and len(hits) > 1:
            kind, note = "ok", f": the name of {len(hits)} files, each use resolved by its sentence"
        elif tok in NOT_PATHS:
            kind, note = "--", f": {NOT_PATHS[tok]}"
        elif re.fullmatch(r"[\w-]+\.github\.io/nova", tok):
            kind, note = "--", ": the plan's Pages address, the records' own (spec §5, check 4)"
        else:
            kind, note = "FAIL", ": not in the tree"
        kinds[kind] += 1
        bad += kind == "FAIL"
        print(f"{kind} path {tok}{note} ({k}x)")
    links = re.findall(r"\]\(([^)]+)\)", text)
    for u in links:
        ok = u.startswith("https://") or os.path.exists(u.split("#")[0])
        bad += not ok
        print(f"{'ok' if ok else 'FAIL'} link {u}")
    urls = re.findall(r"https?://[^\s)>`\"]+", text)
    for u in urls:
        print(f"FAIL url {u}: the records hold no URL")
        bad += 1
    print(f"pathcheck: {len(CITES)} ADR citations, {len(rows)} plan citations at {BASE} ({moved} read "
          f"otherwise in the tree), {kinds['ok']} repository paths, {kinds['--']} names that are not paths, "
          f"{len(links)} Markdown links, {len(urls)} URLs")
    done("pathcheck", bad)


def renumber(argv):
    adr = read(ADR)
    found = [(n, m.group(0)[1:-1]) for n, l in enumerate(adr, 1) for m in CITE.finditer(l)]
    if [t for _, t in found] != [t for _, t, _, _ in CITES]:
        print(f"renumber: the ADR's {len(found)} citations are not the table's {len(CITES)}, in the same "
              "order: add, remove or change the row by hand")
        done("renumber", 1)
    me = os.path.abspath(__file__)
    src = open(me, encoding="utf-8").read().split("\n")
    a = src.index("CITES = [")
    b = src.index("]", a)
    rows = [i for i in range(a + 1, b) if re.match(r"    \((\d+), ['\"]", src[i])]
    assert len(rows) == len(CITES), (len(rows), len(CITES))
    moved = 0
    for i, (n, t), (old, _, _, _) in zip(rows, found, CITES):
        src[i] = re.sub(r"^    \(\d+,", f"    ({n},", src[i], count=1)
        if n != old:
            moved += 1
            print(f"ADR:{old} -> ADR:{n} `{t}`")
    open(me, "w", encoding="utf-8").write("\n".join(src))
    print(f"renumber: {len(CITES)} rows, {moved} keys moved, in {me}")
    done("renumber", 0)


def private_names():
    url = git("remote", "get-url", "origin", check=False).stdout.strip()
    m = re.search(r"github\.com[:/]([^/]+)/", url)
    owner = m.group(1) if m else None
    names = {owner or ""}
    names |= set(git("log", "-200", "--format=%an%n%ae%n%cn%n%ce", MAIN, check=False).stdout.split("\n"))
    names |= {git("config", k, check=False).stdout.strip() for k in ("user.name", "user.email")}
    generic = {"", "Claude", "noreply@anthropic.com", "GitHub", "noreply@github.com"}
    return owner, sorted(n.strip() for n in names if n.strip() not in generic and len(n.strip()) >= 3)


def privacy(argv):
    added = diff_lines()[0]
    msg = message(argv)
    lines = added + [("the message", l) for l in msg]
    owner, names = private_names()
    fam = "|".join(["op" + "us", "son" + "net", "hai" + "ku", "fa" + "ble"])
    scratch = os.environ.get("P", "")
    rules = [
        ("a local or scratch path", r"/tmp/|/home/|/root\b|/Users/|[A-Za-z]:\\Users|~/"
         r"|(?<![\w$])\$P\b|\b(?:exec|plan|proto|split|spike|env|ledger-)60\b|" + "scratch" + "pad"),
        ("a branch not the records'", r"(?<![\w/.-])(?:proto|split)/|\b[\w./-]+-ci\b"),
        ("an email", r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+"),
        ("a model identifier or session link", r"\bclaude-(?:%s|instant|\d)|\b(?:%s)\b|claude\.ai/" % (fam, fam)),
        ("a token", r"\b(?:ghp|gho|ghs|ghu|ghr|github_pat)_\w{8,}|\bcio[A-Za-z0-9]{16,}"
         r"|(?<![A-Za-z0-9])(?=[A-Za-z0-9]*[A-Z])(?=[A-Za-z0-9]*[a-z])(?=[A-Za-z0-9]*[0-9])[A-Za-z0-9]{20,}(?![A-Za-z0-9])"),
    ]
    bad, allowed = 0, collections.Counter()
    for f, l in lines:
        for b in re.findall(r"\bclaude/[\w./-]+", l):
            if b != BRANCH:
                print(f"FAIL {f}: a branch not the records': {b}")
                bad += 1
            else:
                allowed["the records' branch"] += 1
        for what, rx in rules:
            for m in re.finditer(rx, l, re.I):
                print(f"FAIL {f}: {what}: {l.strip()[:90]}")
                bad += 1
        if scratch and scratch in l:
            print(f"FAIL {f}: the scratch directory: {l.strip()[:90]}")
            bad += 1
        rest = l
        if owner:
            for form, what in ((f"https://github.com/{owner}/nova", "the repository's address"),
                               (f"{owner}.github.io/nova", "the plan's Pages address")):
                k = len(re.findall(re.escape(form), rest, re.I))
                if k:
                    allowed[f"the owner's login in {what}"] += k
                    rest = re.sub(re.escape(form), "", rest, flags=re.I)
        for n in names:
            if re.search(re.escape(n), rest, re.I):
                print(f"FAIL {f}: a person's name or address (built from git, not written): "
                      f"{re.sub(re.escape(n), '<name>', l.strip(), flags=re.I)[:90]}")
                bad += 1
    print(f"privacy: {len(lines)} lines read ({len(added)} added lines, {len(msg)} message lines), "
          f"{len(names)} names built; allowed: "
          + (", ".join(f"{k} {v}" for k, v in sorted(allowed.items())) or "none"))
    done("privacy", bad)


def notes_of(version, path):
    r = subprocess.run(["bash", ".github/scripts/release-notes.sh", version, path],
                       capture_output=True, text=True, encoding="utf-8")
    return r.returncode, r.stdout, r.stderr.strip()


def spec_lines():
    return read(SPEC)


def dated(lines, D, dedent=0):
    return [l[dedent:].replace("Accepted (2026-11-26)", f"Accepted ({D})")
            .replace("**Recorded 2026-11-26 (", f"**Recorded {D} (")
            .replace("**Amended 2026-11-26 (", f"**Amended {D} (") for l in lines]


def drafted(spec, a, b, first, last, D, dedent=0):
    """Spec lines a to b, checked at both ends, FIXES applied (the last
    first, so a fix of several lines moves none above it), dated D."""
    assert spec[a - 1].strip() == first and spec[b - 1].strip() == last, (a, b, spec[a - 1], spec[b - 1])
    out = list(spec[a - 1:b])
    for n, old, new in sorted(FIXES, key=lambda f: -f[0]):
        old, new = ([old], [new]) if isinstance(old, str) else (old, new)
        if a <= n <= b:
            assert n - 1 + len(old) <= b and out[n - a:n - a + len(old)] == old, (n, out[n - a])
            out[n - a:n - a + len(old)] = new
    return dated(out, D, dedent)


def notes(argv):
    D = argv[0]
    bad = 0
    rc, now, err = notes_of("0.5.0", "CHANGELOG.md")
    print(f"release-notes.sh 0.5.0 CHANGELOG.md: exit {rc} {err}")
    with tempfile.TemporaryDirectory() as t:
        old = os.path.join(t, "CHANGELOG.md")
        open(old, "w", encoding="utf-8").write(git("show", f"{BASE}:CHANGELOG.md").stdout)
        rc0, was, _ = notes_of("0.5.0", old)
        rcu0, _, erru0 = notes_of("Unreleased", old)
        erru0 = erru0.replace(old, f"{BASE}'s CHANGELOG.md")
    for name, out in ((f"{BASE}'s", was), ("now", now)):
        print(f"0.5.0, {name}: {len(out.split(chr(10))) - 1} lines, {len(out.encode())} bytes")
    same = rc == 0 and rc0 == 0 and now == was
    print(f"0.5.0: {'the same lines as ' + BASE + chr(39) + 's' if same else 'NOT the same as ' + BASE + chr(39) + 's'}")
    bad += not same
    print(f"release-notes.sh Unreleased, {BASE}'s CHANGELOG.md: exit {rcu0} {erru0}")
    rcu, nowu, erru = notes_of("Unreleased", "CHANGELOG.md")
    want = [""] + drafted(spec_lines(), 298, 306, "**Recorded 2026-11-26 (branch `claude/loving-archimedes-frtf5q-60`):**",
                          "changes yet.", D)
    got = nowu.split("\n")[:-1]
    ok = rcu == 0 and got == want
    print(f"release-notes.sh Unreleased CHANGELOG.md: exit {rcu} {erru}; {len(got)} lines; "
          f"{'a blank line and spec §3.4 note, dated ' + D if ok else 'NOT the note alone'}")
    bad += not ok
    done("notes", bad)


def files(argv):
    tracked = {f for f in git("diff", "--name-only", "--no-renames", BASE, "--", ".",
                               ":(exclude)docs/superpowers").stdout.split("\n") if f}
    untr = {f for f in untracked() if not f.startswith("docs/superpowers/")}
    for f in sorted(untr):
        print(f"untracked: {f}")
    got = tracked | untr
    print(git("diff", "--numstat", BASE, "--", ".", ":(exclude)docs/superpowers").stdout.rstrip())
    for f in sorted(untr & set(SIX)):
        print(f"{len(read(f))}\t0\t{f} (untracked)")
    for f in sorted(got - set(SIX)):
        print(f"FAIL: {f} changed or untracked, not one of the six")
    for f in sorted(set(SIX) - got):
        print(f"FAIL: {f} unchanged")
    print(f"files: {len(got)} changed against {BASE} outside docs/superpowers, {len(untr)} untracked")
    done("files", got != set(SIX))


def place(argv):
    D = argv[0]
    spec = spec_lines()

    def at(lines, block):
        return [i + 1 for i in range(len(lines)) if lines[i:i + len(block)] == block]

    checks = []
    ad = read(ADR)
    title = spec[74].split("`")[1]
    checks.append(("ADR 0062's title (spec 75)", [1] if ad[0] == title else [], ad[0] == title))
    for name, a, b, first, last in (
            ("ADR 0062's Status (spec 86-96)", 86, 96, "## Status", "commits only."),
            ("ADR 0062's Decision (spec 102-202)", 102, 202, "## Decision",
             "The user allowed the target locally, for the spike, on 2026-11-26."),
            ("ADR 0062's figures (spec 208-243)", 208, 243, "### The spike's figures", "three it can."),
            ("ADR 0062's Consequences (spec 249-266)", 249, 266, "## Consequences",
             "edition or a recorded exception.")):
        h = at(ad, drafted(spec, a, b, first, last, D))
        checks.append((name, h, h == [3] if a == 86 else len(h) == 1))
    k = ad.index("## Decision") if "## Decision" in ad else -1
    checks.append(("ADR 0062's parts in order", [k + 1], k > 0 and ad[k - 2] != "" and ad.index("## Context") < k
                   < ad.index("### The spike's figures") < ad.index("## Consequences") < ad.index("## References")
                   if all(x in ad for x in ("## Context", "### The spike's figures", "## Consequences",
                                            "## References")) else False))
    ms = read(MS)
    h = at(ms, drafted(spec, 275, 287, "**Amended 2026-11-26 (branch `claude/loving-archimedes-frtf5q-60`):**",
                       "`1.0.0`, in place of the `v0.6.0` that §7 would give it.", D))
    checks.append(("the master spec note (spec 275-287)", h, len(h) == 1 and ms[h[0] - 2] == ""
                   and ms[h[0] - 3].startswith("7. Launch posts: HN, Lobsters") and ms[h[0] + 12] == ""
                   and ms[h[0] + 13] == "---"))
    cl = read("CHANGELOG.md")
    h = at(cl, drafted(spec, 298, 306, "**Recorded 2026-11-26 (branch `claude/loving-archimedes-frtf5q-60`):**",
                       "changes yet.", D))
    checks.append(("the CHANGELOG note (spec 298-306)", h, h == [12] and cl[9] == "## [Unreleased]"
                   and cl[10] == "" and cl[20] == "" and cl[21].startswith("## [0.5.0] - ")))
    pl = read(PLAN)
    h = at(pl, drafted(spec, 315, 326, "**Amended 2026-11-26 (6.0, branch `claude/loving-archimedes-frtf5q-60`,",
                       "This plan's `path:line`s stay `4fe11d55`'s.", D))
    checks.append(("the plan's 6.0 note (spec 315-326)", h, h == [427] and pl[424].startswith("  Chromium locally")
                   and pl[425] == "" and pl[438] == "" and pl[439] == "### 6.1 — The playground"))
    a48 = read(A48)
    h = at(a48, drafted(spec, 338, 340, "**Amended 2026-11-26 (branch `claude/loving-archimedes-frtf5q-60`):**",
                        "on the backlog (ADR 0062);", D, 2))
    checks.append(("ADR 0048's marker (spec 338-340)", h, len(h) == 1
                   and a48[h[0] - 2] == "   - N built for WASM, which Phase 6's playground needs;"
                   and a48[h[0] + 2].startswith("   - trait objects, mutable globals")))
    a61 = read(A61)
    h = at(a61, drafted(spec, 347, 349, "**Amended 2026-11-26 (branch `claude/loving-archimedes-frtf5q-60`):**",
                        "decision of 2026-11-26, and puts N built for WASM on this backlog.", D, 2))
    checks.append(("ADR 0061's marker (spec 347-349)", h, len(h) == 1
                   and a61[h[0] - 2] == "  playground (`nova-spec/00-MASTER-SPEC.md:940`)."
                   and a61[h[0] + 2].startswith("- **Taken from earlier backlogs:**")))
    bad = 0
    for name, h, good in checks:
        bad += not good
        print(f"{'ok' if good else 'FAIL'} {name}: at {h}")
    done("place", bad)


def blob(data):
    return subprocess.run(["git", "hash-object", "--stdin"], input=data, capture_output=True,
                          check=True).stdout.decode().strip()


def planfile(argv):
    bad = 0
    data = open(PLAN, "rb").read()
    t = data.split(b"\n")
    rest = b"\n".join(t[:426] + t[439:])
    got = blob(rest)
    ok = got == APPROVED and len(t) > 440 and t[425] == b"" and t[438] == b"" \
        and t[439].decode() == "### 6.1 — The playground"
    print(f"{'the approved text' if ok else 'NOT the approved text'}: {PLAN} without lines 427-439, "
          f"{len(rest)} bytes, blob {got[:12]} (the approved {APPROVED[:12]}); the file {len(t) - 1} lines, "
          f"{len(data)} bytes")
    bad += not ok
    log = git("log", "--reverse", "--format=%H %s", f"{BASE}..HEAD", "--", PLAN).stdout.strip().split("\n")
    log = [x for x in log if x]
    if not log:
        print(f"no commit after {BASE} touches {PLAN} yet")
    else:
        h, subj = log[0].split(" ", 1)
        b0 = git("rev-parse", f"{h}:{PLAN}", check=False).stdout.strip()
        par = git("cat-file", "-e", f"{h}^:{PLAN}", check=False).returncode
        ok = b0 == APPROVED and subj == "docs: the Phase 6 plan, approved" and par != 0
        print(f"the first commit after {BASE} touching it: \"{subj}\", adding it as {b0[:12]}: "
              f"{'the approved text' if ok else 'NOT as expected'}; commits touching it: {len(log)}")
        bad += not ok
    done("planfile", bad)


def guard_parts():
    ad = read(ADR)
    a, f = ad.index("## Decision"), ad.index("### The spike's figures")
    c = ad.index("## Consequences")
    table = [l for l in ad[f:c] if l.startswith("|")]
    return {"the twelve decisions": "\n".join(ad[a:f]) + "\n", "the figures table": "\n".join(table) + "\n"}


def guard(argv):
    bad = 0
    for name, text in guard_parts().items():
        got = hashlib.sha256(text.encode("utf-8")).hexdigest()
        ok = got == GUARD[name]
        bad += not ok
        print(f"{'same' if ok else 'DIFFERS'} {name}: {len(text.split(chr(10))) - 1} lines, sha256 {got[:16]}, "
              f"the split's {GUARD[name][:16]}")
    done("guard", bad)


def split(argv):
    D = argv[0]
    bad = 0
    for f, want in SPLIT.items():
        got = blob(open(f, "rb").read().replace(D.encode(), b"2026-11-27"))
        bad += got != want
        print(f"{'same' if got == want else 'DIFFERS'} {f}: {got[:12]}, the split's {want[:12]}")
    done("split", bad)


if __name__ == "__main__":
    if not os.path.exists(".git") or len(sys.argv) < 2:
        sys.exit(__doc__)
    {"hashcheck": hashcheck, "pathcheck": pathcheck, "renumber": renumber, "privacy": privacy,
     "notes": notes, "files": files, "place": place, "planfile": planfile, "guard": guard,
     "split": split}[sys.argv[1]](sys.argv[2:])
EOF
```

  Then:

  ```sh
  bash -n "$P/exec60/redate.sh" && echo "redate.sh parses"
  python3 -c 'import ast, sys; ast.parse(open(sys.argv[1], encoding="utf-8").read())' "$P/exec60/checks.py" && echo "checks.py parses"
  git status --short
  ```

  Expected: `redate.sh parses`, `checks.py parses`, and `git status
  --short` prints nothing: the scripts and messages are outside the
  repository.
- [ ] **Step 2: The approved plan, and its commit.** `patch_block
  t1-plan`, printing nothing. Then:

  ```sh
  git status --short
  wc -l < docs/phase-6-plan.md
  git hash-object docs/phase-6-plan.md
  git add -A
  git status --short
  git commit -F "$P/exec60/msg-t1-plan.txt"
  git rev-parse HEAD:docs/phase-6-plan.md
  printf '%s\n' "$(git log -1 --format=%B)" | diff - "$P/exec60/msg-t1-plan.txt" && echo "the message is the file's"
  git show --stat --format='%h %s' HEAD | tail -n 1
  git status --short
  ```

  Expected: `?? docs/phase-6-plan.md` and nothing else; `750`;
  `d891448dd75bf0e84ee55af6ddf95a56aed716d4`, the approved text's blob
  (decision 23); `A  docs/phase-6-plan.md`; the commit, `docs: the Phase
  6 plan, approved`; the same blob, now the commit's; `the message is the
  file's`; `1 file changed, 750 insertions(+)`; and nothing. Record the
  commit in the ledger. **A blob that differs stops the task:** the plan
  is the approved text, byte for byte, or nothing (spec decision 7).
- [ ] **Step 3: The records' blocks.** `patch_block t1-adr`, then
  `patch_block t1-notes`, each printing nothing. Then:

  ```sh
  git status --short
  wc -l < docs/adr/0062-phase-6-scope.md
  git diff --numstat
  ```

  Expected: `?? docs/adr/0062-phase-6-scope.md` and ` M` for
  `CHANGELOG.md`, `docs/adr/0048-phase-5-scope-and-backend.md`,
  `docs/adr/0061-phase-5-boundary.md`, `docs/phase-6-plan.md` and
  `nova-spec/00-MASTER-SPEC.md`, and nothing else; `256`; and the five
  files' numstat:

  ```text
  10	0	CHANGELOG.md
  3	0	docs/adr/0048-phase-5-scope-and-backend.md
  3	0	docs/adr/0061-phase-5-boundary.md
  13	0	docs/phase-6-plan.md
  14	0	nova-spec/00-MASTER-SPEC.md
  ```

- [ ] **Step 4: The day, and the checks before the commit.**

  ```sh
  D=$(date -u +%F); echo "$D"
  bash "$P/exec60/redate.sh" "$D"; echo "exit $?"
  CHK place "$D"; echo "exit $?"
  CHK notes "$D"; echo "exit $?"
  CHK files; echo "exit $?"
  CHK planfile; echo "exit $?"
  CHK guard; echo "exit $?"
  CHK split "$D"; echo "exit $?"
  git diff --numstat
  ```

  Expected:
  - the day, in UTC (2026-11-27 or later);
  - `redate.sh`: each of the six places holding the day; the history:
    `docs/adr/0061-phase-5-boundary.md: 2026-11-26 1`,
    `docs/adr/0062-phase-6-scope.md: 2026-11-26 6`,
    `docs/phase-6-plan.md: 2026-08-12 1, 2026-11-26 7` and
    `nova-spec/00-MASTER-SPEC.md: 2026-11-26 2`; `redate: ok`, `exit 0`.
    On 2026-11-27 it writes the dates the blocks already hold; on a later
    day it rewrites the six lines, and the numstat below is the same;
  - `place`: eleven lines `ok`, the title at `[1]`, the Status at `[3]`,
    the Decision, the figures and the Consequences once each, the parts
    in order, the master spec's note at `[944]`, the CHANGELOG's at
    `[12]`, the plan's at `[427]`, ADR 0048's marker at `[45]` and ADR
    0061's at `[396]`; `place: ok`;
  - `notes`: `0.5.0`'s notes 627 lines, `the same lines as 4fe11d55's`;
    `Unreleased` at `4fe11d55` `exit 1`; now `exit 0`, `10 lines; a blank
    line and spec §3.4 note, dated D`; `notes: ok`;
  - `files`: the six files' numstat, the plan `763 0`, the ADR `256 0`
    (untracked), `files: 6 changed against 4fe11d55 outside
    docs/superpowers, 1 untracked`, `files: ok`;
  - `planfile`: `the approved text`: the plan without lines 427-439,
    44,866 bytes, blob `d891448dd75b`, the file 763 lines, 45,642 bytes;
    the first commit touching it, `"docs: the Phase 6 plan, approved"`,
    adding it as `d891448dd75b`, `commits touching it: 1`; `planfile:
    ok`;
  - `guard`: `same` for the twelve decisions (105 lines) and the figures
    table (15 lines); `guard: ok`;
  - `split`: `same` for each of the six; `split: ok`;
  - `exit 0` after each; the numstat as Step 3's.

  A check that fails stops the task: record its output in the ledger,
  and do not commit.
- [ ] **Step 5: The records commit.**

  ```sh
  git add -A
  git status --short
  git commit -F "$P/exec60/msg-t1.txt"
  D=$(sed -n 's/^Accepted (\([0-9-]*\))\. Branch .*/\1/p' docs/adr/0062-phase-6-scope.md); echo "$D"
  test "$(TZ=UTC git log -1 --date=format-local:%Y-%m-%d --format=%ad)" = "$D" && echo "dated $D"
  printf '%s\n' "$(git log -1 --format=%B)" | tail -n 3
  git show --stat --format='%h %s' HEAD | tail -n 1
  CHK split "$D"
  git status --short
  ```

  Expected:
  - `git status --short` lists the six files, `A` for the ADR and `M` for
    the five, and nothing else;
  - the day the records carry, Step 4's, then `dated D`: the commit's own
    date, in UTC, is that day. If it prints nothing, the day turned since
    Step 4, and the commit is re-dated (decision 24), each command with
    its own timeout:

    ```sh
    D=$(date -u +%F); echo "$D"
    bash "$P/exec60/redate.sh" "$D"
    CHK place "$D"
    CHK notes "$D"
    git add -A
    git status --short
    git commit --amend --date=now --no-edit
    ```

    each as Step 4 gives it (`git status --short` lists the six files the
    day is in, their six dated lines alone changed), then this step's
    checks again from its `D=` line;
  - the message ends with a blank line and the two trailer lines, the
    last three lines printed;
  - `6 files changed, 299 insertions(+)`;
  - `split`: `same` for each of the six, `split: ok`: the records commit
    holds the split's records, on any day (decision 28);
  - `git status --short` prints nothing.

  Record the commit, the day and the comparison in the ledger.

<!-- patch t1-plan -->

````diff
diff --git a/docs/phase-6-plan.md b/docs/phase-6-plan.md
new file mode 100644
index 00000000..d891448d
--- /dev/null
+++ b/docs/phase-6-plan.md
@@ -0,0 +1,750 @@
+# Phase 6 Plan — 1.0 Release
+
+> Status: **approved** by the user (2026-11-26). Derived from
+> `nova-spec/00-MASTER-SPEC.md` §3 (Phase 6, `:935-942`), §1.9
+> (`:99-102`), §5.3 (`:996`) and §7 (`:1126-1134`),
+> `nova-spec/20-STDLIB.md` §1, `docs/releasing.md`, and ADRs 0047, 0048
+> and 0061, with the research of 2026-11-26 on `main` at `4fe11d55`
+> (Phase 5 closed by ADR 0061; `v0.5.0` released). The user's decisions
+> of 2026-11-26 are in §1. Each sub-phase below gets its own spec, plan
+> and review before it is built.
+
+## 1. Goal (from the spec)
+
+Phase 6 = "1.0 Release", weeks 105 to 120: 16 weeks, the last of the
+master spec's 30 months (`nova-spec/README.md:93`). Unlike Phases 0 to
+5, it has no goal line and no gate (`nova-spec/00-MASTER-SPEC.md:935-943`);
+this plan proposes the gate below. The master spec lists seven items
+(`:936-942`):
+
+1. Security audit (external)
+2. Stability freeze — RFC for any breaking change
+3. Documentation 100% (every public API has rustdoc-equivalent)
+4. Tutorial site at `novalang.dev`
+5. Interactive playground (compile in browser via WASM-compiled Nova compiler)
+6. Submit to TechEmpower benchmarks
+7. Launch posts: HN, Lobsters, Reddit r/programming, dev.to, Twitter
+
+Around them: §1.9's "Pre-1.0: breaking changes allowed in minor
+versions, RFC required", "Post-1.0: semver strict, edition system
+(`edition = "2026"`) for opt-in breaking" and "Deprecation: minimum 2
+minor versions before removal" (`:100-102`); §5.3's "Link to docs" in
+every user-facing error (`:996`); and §7's "Tag a milestone release:
+`v0.{phase}.0`" (`:1134`), which for Phase 6 reads `v0.6.0`.
+
+Names, as in Phase 5's plan: **S0** is the Rust `nova`; **N** is the
+Nova compiler in `selfhost/`.
+
+**The gate (proposed; the master spec gives none):** a part for each
+item in order (items 6 and 7 together), then the release; on `main`
+after 6.10, with reproducible commands, in CI wherever it can run:
+1. **The audit, as in-repo hardening:** `SECURITY.md`, a threat model
+   and the `unsafe` and dependency inventories merged, the inventories
+   held by tests; fuzz targets for the lexer, the parser, JSON, regex
+   and the HTTP request parser pass in CI on fixed seeds; a dependency
+   check (advisories, licences, sources) passes in CI; the external
+   audit recorded as done, with its findings' disposition, or not done.
+2. **The freeze:** the stability, RFC and MSRV policies merged; std's
+   public signatures and each command's `--help` pinned by snapshot
+   tests (recommended, §3 decision 4); `@deprecated` warning at a use
+   site and shown on std's site; the two S0 bugs ADR 0061 keeps fixed in
+   S0 and N with cases, and issue #3 closed by a test; `std/regex`, and
+   the HTTP client with handlers that await, documented and tested on
+   their targets, the client against local servers only.
+3. **Documentation 100%:** std's documentation tests cover every public
+   module, `std/dom` included; `missing_docs` denied on every library
+   crate; every code a diagnostic can carry has an error-index entry,
+   and each rendered diagnostic its link, both held by tests; the
+   reference's and the book's code blocks run as doc tests.
+4. **The site:** one command builds it (the book, the reference, std's
+   documentation with `std/dom`, the error index, the playground) in CI
+   with no broken internal link; a workflow deploys it to GitHub Pages
+   from the release's tag.
+5. **The playground:** CI builds the compiler for
+   `wasm32-unknown-unknown`; Node and Chromium tests compile and run
+   hello world and a fixed list of programs through it against their
+   expected output, and show a failing program's diagnostics; its size
+   and hello world's compile time recorded, within 6.0's limits or as
+   ruled.
+6. **TechEmpower and the launch:** an app in the repository answers the
+   JSON and plaintext test types as the rules require, held by a CI
+   test, with how to submit; the announcement and draft posts in the
+   repository; the TechEmpower pull request and the posts recorded as
+   the user's, done or not.
+7. **The release:** `1.0.0` published by `docs/releasing.md`, each
+   [stop] on the user's word, the Release gate green on three systems;
+   every item of §7's Definition of Done met, as the boundary ADR
+   records.
+
+**The user's decisions, 2026-11-26:**
+- **P1, scope: all seven items.** The in-repo work is gated. The
+  external items (an external audit firm, the TechEmpower pull request,
+  the launch posts, the domain and DNS, switching GitHub Pages on, any
+  account or money) are the user's, recorded, and do not block 1.0.
+- **P2, the playground: option A for 1.0, after a spike.** S0's front
+  end and WASM backend built for `wasm32-unknown-unknown`, running in
+  the browser (in-memory sources through the `Sources` trait,
+  `build_program_wasm`, the JS layer's `init({ wasm })` with a print
+  hook). The spike needs the user's word to add the `wasm32` rustup
+  target, locally and in CI. An ADR records that the playground is S0
+  in the browser, not N built for WASM, as ADRs 0048 and 0061 had it;
+  option B, N built for WASM, stays on the backlog.
+- **P3, the version:** `1.0.0` is the phase's release, an ADR recording
+  it in place of §7's `v0.{phase}.0`; strict semver after it (§1.9);
+  release candidates only if needed.
+- **P4, the site:** no `novalang.dev` yet. GitHub Pages
+  (`sakeerin.github.io/nova`) hosts the tutorial, the documentation and
+  the playground; the user switches Pages on; a custom domain later, if
+  bought.
+- **P5, documentation 100%: all of it.** std, `std/dom` included, on the
+  site; a `missing_docs` lint for the Rust crates; a language reference;
+  a book or tutorial; an error-code index, errors linking to it (§5.3);
+  the measurable parts held by tests.
+- **P6, stability:** the promise covers the language, std, the CLI,
+  `nova.toml`, `nova.lock` and the index format; not the hidden flags,
+  the Rust crates (internal), N, or diagnostic wording. A written
+  policy, an RFC process and `SECURITY.md`.
+- **P7, the audit:** in-repo hardening is the gate (a threat model, the
+  `unsafe` and dependency inventories, fuzz targets, `cargo-deny` or
+  `cargo-audit` or an equivalent that needs no new crate in the
+  workspace, `SECURITY.md`); the external audit is the user's, recorded
+  as not yet done unless booked.
+- **P8, TechEmpower:** the test types that need no database (JSON,
+  plaintext), as an app in the repository with how to submit; a
+  database driver on the backlog; the pull request is the user's.
+- **P9, before the freeze:** what would break later (an `@deprecated`
+  attribute; fixes to the two S0 bugs ADR 0061 keeps, std's bodies
+  seeing a program's impls and float `%` failing `build --release`;
+  issue #3, E0902 on `sqrt`), and `std/regex` and the HTTP client, with
+  handlers that can await. UDP and Unix sockets, AEAD, `Queue`,
+  `RwLock` and atomics stay post-1.0.
+- **P10:** none of ADR 0061's backlog is in Phase 6 (the bootstrap on
+  macOS and Windows, N on a large-stack thread and E0093, compiler speed
+  among it), but for the two kept bugs P9 names.
+- **P11, MSRV:** keep 1.78, with a written policy; a raise only on the
+  user's word (an advisory may force one).
+- **P12, the order:** the playground spike first; the rest ordered by
+  this plan, for the user's approval.
+
+**Not asked; recommended here:** the launch materials in the repository
+(an announcement, the `[1.0.0]` changelog entry, draft posts, and a
+README for users in place of the "Specification Bundle" opening,
+`README.md:1-4`), the posts themselves the user's; the site built by
+`nova-doc`, extended to render a book (§3 decision 8); and the gate
+above, the research's question 12 made testable item by item.
+
+## 2. Reality check — what 1.0 needs that the repo lacks
+
+Measured on `main` at `4fe11d55`, 2026-11-26, with `nova` 0.5.0.
+"Heuristic" counts come from searches, not tests.
+
+1. **The audit (item 1): nothing an auditor is handed exists.**
+   - No `SECURITY.md`, `deny.toml`, `cargo audit` or `cargo deny` step,
+     or `fuzz/`; `CODE_OF_CONDUCT.md:7` says "Report issues to the
+     maintainers", with no contact.
+   - `Cargo.lock` holds 254 packages, 231 from crates.io. Two pins are
+     held back by MSRV 1.78: `wasm-encoder` and `wasmparser` at
+     `=0.243.0` (`Cargo.toml:69-73`), `ureq` at `=3.2.1`
+     (`crates/nova-index/Cargo.toml:33-37`). Advisories were not checked
+     (no network).
+   - `unsafe` occurs 715 times in `crates/nova-runtime/src` (tests
+     included), beside 119 `#[no_mangle]` exports and `gc_stack.c`;
+     outside it, `crates/nova-cli/src/cmd/run.rs:146` and the JIT
+     entry's `transmute` (`crates/nova-codegen-cranelift/src/lib.rs:62`).
+     No crate forbids `unsafe_code`. Fuzzing is the self-hosting
+     differential (`crates/nova-cli/tests/selfhost/fuzz.rs`) and
+     `proptest` in the lexer's and parser's tests.
+   - Defended already: a package checked against the lock's SHA-256,
+     bounded and confined (ADR 0031 `:48-53`); install scripts checking
+     `SHA256SUMS` from the same release
+     (`tools/install/install.sh:78-94`): integrity, not authenticity.
+     `ci.yml` and `benchmarks.yml` set no `permissions`; no branch on
+     GitHub is protected, `main` included.
+   - An audit's surfaces: `std/http`'s parsing, `std/json`'s depth,
+     `std/crypto` on `ring`, the blog's sessions (ADR 0046), the
+     collector and executor, `nova dev`, the WASM shim (ADR 0041), the
+     index and downloads, and this phase's playground and client.
+     External: the firm and its cost; GitHub's settings.
+2. **The freeze (item 2): no policy, no process, nothing pins std's
+   API.**
+   - `docs/rfcs/` holds 0000 (informational) and 0001 (the component
+     syntax), with no README, template or states; RFCs "propose new
+     features that the spec doesn't yet cover"
+     (`nova-spec/README.md:117-119`). `CONTRIBUTING.md` is 32 lines,
+     with no governance.
+   - The golden suite (176 programs), `tests/wasm/`, `tests/dom/`, 16
+     insta snapshots and the CLI's tests pin behaviour; std's
+     documentation test checks documentation, not signatures
+     (`crates/nova-doc/tests/std_docs.rs:8-27`).
+   - Versioned: `nova.lock`'s `version = 1`
+     (`nova-spec/40-TOOLING.md:424-429`), the index line's `v` (ADR 0031
+     `:20-23`); one edition, `"2026"`
+     (`crates/nova-pm/src/manifest.rs:13-14`), edition rules unbuilt
+     (`40-TOOLING.md:436-439`). Outside any promise already: the 20
+     published crates (`Cargo.toml:20-26`) and the hidden `--dump-*`
+     flags (`CHANGELOG.md:219`). No deprecation: `test` is the only
+     attribute, any other E0082 (`crates/nova-resolver/src/lib.rs:3336`).
+   - What a freeze would lock in: std's bodies see a program's impls,
+     worked around by a fallback (ADR 0056 `:1220-1221`); float `%`
+     type-checks and fails in `build --release` (ADR 0058 `:885-886`);
+     issue #3, `nova run` failing to resolve `sqrt` through the JIT on
+     Linux (E0902), open since 2026-08-12.
+3. **Documentation (item 3): std is at 100% by test; the rest does not
+   exist.**
+   - `std_is_documented` and `every_std_module_has_an_example` cover
+     the 21 modules of `nova_std::ALL` (`std_docs.rs:8-47`); heuristic:
+     621 public items in all 25 std sources, none without `///`. `ALL`
+     leaves out `RT`, `DOM` and `std/ui`'s backend (`std/lib.rs:76-78`);
+     `std/dom`'s 67 items are off the site by ADR 0047's decision
+     (`:170-185`).
+   - Rust: no `missing_docs` lint; heuristic: 113 of 832 item-level
+     `pub` declarations lack `///` (44 in `nova-cli`, a binary), fields
+     and variants not counted.
+   - Missing: a language reference (§2's `docs/spec/`, `:182-185`), a
+     book (`docs/book/`, `:186-188`), an error-code index, CLI and
+     `nova.toml` references. §5.3's link is built and recorded nowhere.
+     About 75 codes are in use (55 E, 17 M, 2 P, 1 W; heuristic).
+   - The README opens as "Nova Language — Specification Bundle".
+4. **The site (item 4): nothing exists, and `novalang.dev` "does not
+   exist"** (`40-TOOLING.md:450`, `:626`; `50-TESTING.md:243`). No
+   site sources, Pages workflow or `gh-pages` branch; a Pages deploy
+   needs `pages: write` and `id-token: write`, which no workflow grants.
+   At hand: `nova-doc` renders Markdown with `pulldown-cmark`, with
+   search and themes, and runs code blocks as doc tests (ADRs 0034,
+   0035); `release.yml`'s `std-docs` job builds std's site
+   (`.github/workflows/release.yml:264-291`). mdBook, which §2 names, is
+   not in the workspace. External: the Pages switch; a domain and DNS.
+5. **The playground (item 5): option A's path exists in pieces.**
+   - S0's WASM backend needs no other toolchain (ADR 0039 `:38-41`);
+     its runtime is `std/rt`, in Nova; one memory of at most 1 GB
+     (`:70`); V8's stack, about 1 MB, bounds a program's recursion
+     (`:105-107`). WASM's std is 16 modules, without `io`, `fs`, `net`,
+     `http`, `crypto`, `process`, `test` and `ui.ssr`
+     (`crates/nova-resolver/src/lib.rs:2892-2909`).
+   - In memory: `build_program_wasm` lowers, compiles and makes the shim
+     before its writes (`crates/nova-driver/src/lib.rs:865-892`); the
+     front end reads through `Sources`
+     (`crates/nova-driver/src/analyze.rs:25-42`); the shim's `init`
+     takes `options.wasm` (`crates/nova-codegen-wasm/src/shim/run.js:2-10`)
+     and prints to `console.log` (`shim/core.js:34-45`).
+   - The probe, without a build: the front-end crates and
+     `nova-codegen-wasm` use no OS API outside tests but one thread with
+     a 256 MB stack (`crates/nova-codegen-wasm/src/lib.rs:63`,
+     `:198-208`). `nova-driver` runs `clang` and depends, through
+     Cranelift, on `nova-runtime` (`ring`, `libc`, C), so it cannot be
+     the browser's entry. The chain is 55,749 Rust lines with tests, and
+     std's sources 742,977 bytes; whether it builds for `wasm32`, its
+     size and its speed are unknown until built.
+   - Option B: N's MIR is native only (ADR 0057 `:521-534`), and its
+     `async fn main` would never collect mid-compile (ADR 0040 `:42-43`).
+6. **TechEmpower (item 6): a server, no database driver, no client.**
+   `std/http` serves HTTP/1.1 with a router, pipelined requests in
+   order; a handler "cannot await anything" (`std/http/lib.nova:8-9`);
+   "Nova gets concurrency, not multi-core execution" (ADR 0009 `:57`).
+   Measured once: 11,940.0 req/s on the read-and-parse path
+   (`docs/benchmarks/http-fixed-response.md`). The rules were not read
+   (no network). External: the pull request and its review.
+7. **The launch (item 7): the user's.** The repository can give
+   `[1.0.0]`'s notes, which `release.yml` takes from `CHANGELOG.md`
+   (`docs/releasing.md:148-154`), an announcement, and drafts.
+8. **The release: the runbook needs no change for `1.0.0`** (from
+   `v0.3.0` on, `docs/releasing.md:3`; a prerelease only "if the version
+   has a `-`", `:226-231`). CI's MSRV job checks 1.78 (`ci.yml:399-476`);
+   no policy says when it may rise. The next ADR is 0062.
+9. **std's v1.0 index** (`20-STDLIB.md:8-29`): not built, `Queue`,
+   atomics and `RwLock`, UDP and Unix sockets, the HTTP client, AEAD,
+   and `std/regex`, "PCRE-compatible" (`:25`). On WASM, `std/dom` has a
+   `fetch` (`README.md:111-114`).
+10. **Two compilers.** CI holds N equal to S0 on every pull request: the
+    dump differential on three systems, S1 == S2 == S3 on Ubuntu (ADR
+    0060). A change to a ported part changes both in one pull request
+    (ADR 0056 `:1205-1211`). The Selfhost job's lines are 80, 100 and
+    110 s (ADR 0061 `:205`).
+11. **Tools here:** Node 22; Rust 1.97 with the 1.78 and 1.100
+    toolchains; no `wasm32-unknown-unknown` target. CI's browser job
+    installs Chromium, Lighthouse and binaryen (`ci.yml:281-356`).
+
+## 3. Key decisions (recommended now, confirmed before building)
+
+Each is confirmed in its sub-phase's spec, and recorded in an ADR where
+it departs from the master spec or an earlier ADR.
+
+1. **Scope, and what is external (P1, P9, P10).** ADR 0062 records the
+   seven items, the in-repo part of each gated and the user's parts
+   named, and what Phase 6 leaves out, to the backlog:
+   - N built for WASM (option B), amending ADR 0048 (`:44`) and ADR
+     0061's mapping (`:394-395`); a database driver; UDP and Unix
+     sockets, AEAD, `Queue`, `RwLock` and atomics; ADR 0061's backlog,
+     but for its two kept bugs;
+   - what other specs give Phase 6 that the seven items do not name:
+     multiple toolchains (`40-TOOLING.md:646`; ADR 0037 `:93`),
+     `nova-embed` (`13-RUNTIME.md:480`), the `setup-nova` action
+     (`40-TOOLING.md:583-598`), examples 08 to 13
+     (`60-EXAMPLES.md:801-812`), and edition rules.
+
+   A sub-phase's spec may pull one back in, with a ruling that names
+   ADR 0062. **Pre-1.0 RFCs:** §1.9 asks one for a breaking change
+   before 1.0 (`:100`); as in Phases 1 to 5, a sub-phase's approved
+   spec and its ADR stand as that RFC for 6.3 to 6.5, and the RFC
+   process (6.2) governs every change after `1.0.0`.
+2. **The playground: S0 in the browser, after a spike (P2).** 6.0's
+   spike builds the front-end chain and `nova-codegen-wasm` for
+   `wasm32-unknown-unknown` with a throwaway entry, in scratch space, as
+   Phase 5's lexer spike was, and measures the compiler's size, hello
+   world's compile time, memory, and the deepest input that survives.
+   ADR 0062 records the approach with those figures; it amends ADRs
+   0048 and 0061, not the master spec, whose "WASM-compiled Nova
+   compiler" (`:940`) names no stage. The page compiles one source text
+   with std embedded, returns diagnostics or `app.wasm` and `app.js`,
+   and runs the program in a sandbox away from the site's origin (§6
+   risk 1), through hand-written JS glue (no `wasm-bindgen`). **If the
+   spike misses its limits,** the user rules before 6.1: make it smaller
+   (std's checking kept across compiles, `wasm-opt`), compile on a
+   server (an external service, P1), or leave the playground for after
+   1.0; an ADR records the choice.
+3. **The version: `1.0.0` (P3).** ADR 0062 records it in place of §7's
+   `v0.{phase}.0` (`:1134`). The runbook runs unchanged with
+   `version=1.0.0`. A `1.0.0-rc.N` comes only if the audit, the freeze
+   or the Release gate asks for one, on the user's word, since each
+   release repeats the user's publishing steps. Strict semver after.
+4. **The stability promise (P6), written in 6.2, in force from
+   `1.0.0`.** **Covered:** the language at edition `"2026"`; std's
+   public API on both targets; the CLI's commands, documented flags and
+   exit codes; `nova.toml`; `nova.lock` (`version = 1`); the index line
+   (`v`). **Not covered:** hidden flags; the Rust crates, published so
+   that `cargo install` works, their API internal as their README says;
+   N; diagnostics' wording; the generated JS and WASM's internals; the
+   playground; performance. **Rules:** semver; a breaking language
+   change only in a new edition; deprecation at least two minor versions
+   before a removal, through `@deprecated`; soundness and security fixes
+   as recorded exceptions. **The RFC process:** `docs/rfcs/README.md`
+   and a template (0000 is the overview's): when an RFC is needed, its
+   states, who decides (the maintainer). **Held by tests (recommended,
+   not asked):** snapshots of std's public signatures, from `nova-doc`'s
+   item tables, and of each command's `--help`, which a pull request
+   changing either updates in review.
+5. **Before the freeze (P9).** What would break, or come harder, after
+   `1.0.0` lands before hardening: `@deprecated` and the three fixes
+   (6.3), `std/regex` (6.4), the client and handlers that await (6.5).
+   The handler change breaks today's `fn(Request) -> Response`
+   handlers; `ui::ssr`, the examples and the blog move with it.
+6. **Documentation 100% (P5), each part held:** std with `std/dom`
+   (amending ADR 0047, `:180-185`; `rt` and the ui backends internal),
+   by the extended documentation tests; Rust, by `missing_docs`, denied
+   through clippy's `-D warnings`; the error index, by tests that every
+   emitted code has an entry whose example gives it, and that each
+   rendered diagnostic and the language server's `codeDescription` link
+   to it; the references, by doc tests and tests that every command,
+   documented flag and manifest key appears; the book, by doc tests.
+7. **Hardening, the audit's in-repo gate (P7),** as 6.6, with
+   `SECURITY.md` from 6.2 and fuzz targets for §5.2's lexer, parser,
+   JSON and regex (`:985`) and the HTTP request parser. If the user
+   books a firm in time, `1.0.0` may wait for its findings, on the
+   user's word.
+8. **The site: GitHub Pages, built by `nova-doc` (P4; recommended).**
+   A project site under `/nova/`, so every link is relative to that
+   base; the user turns Pages on (source: GitHub Actions). `nova-doc`,
+   extended to render a book, over mdBook, which §2 names: no new tool
+   in CI, one look and one search across std's documentation and the
+   book, and its code blocks tested by `nova`'s own doc tests. The book
+   keeps mdBook's layout (`docs/book/src/SUMMARY.md`), so mdBook could
+   render it later; the site's command is hidden, outside the promise.
+   It deploys from the release's tag, the playground built from the
+   same commit.
+9. **TechEmpower: JSON and plaintext (P8),** as 6.9; the rules read in
+   its spec, by the user or with the user's leave.
+10. **MSRV: 1.78, with a policy (P11),** written in 6.2: a raise is the
+    user's decision, in a minor release, with an ADR; an advisory in a
+    pin held back for 1.78 may force one, and the dependency check finds
+    it.
+11. **Two compilers in step.** A change to a ported part (6.3) changes
+    S0 and N in one pull request (ADR 0056 `:1205-1211`); std grown in
+    Nova is type-checked by both; the playground, being S0, needs no
+    port.
+12. **New crates, rustup targets and CI tools.** None is added before
+    the user's word, after its spec shows the change:
+
+    | Sub-phase | What may be asked | For |
+    |---|---|---|
+    | 6.0 | the rustup target `wasm32-unknown-unknown`, locally (stable and 1.78) | the spike, built offline from the crates `Cargo.lock` already holds |
+    | 6.1 | the same target in CI; a new unpublished workspace crate for the browser's entry, or a feature split of `nova-driver` (either changes `Cargo.lock`) | the playground |
+    | 6.4 | nothing, if `std/regex` is in Nova; `regex` 1.13.1 (in `Cargo.lock` through `criterion`) as a test-only oracle, or as the engine if the spec chooses the runtime | the engine and its check |
+    | 6.5 | `rustls` 0.23.45 and `webpki-roots` 1.0.9 for `nova-runtime` (in `Cargo.lock` through `ureq`; a lock change); test certificates checked in | HTTPS |
+    | 6.6 | `cargo-deny` or `cargo-audit`, installed by its CI job at a pinned version, with the advisory database that job fetches; optionally `cargo-fuzz`, `libfuzzer-sys` and a nightly toolchain, outside the workspace; optionally attestations in `release.yml` | the checks |
+    | 6.8 | `actions/configure-pages`, `actions/upload-pages-artifact`, `actions/deploy-pages`, in a job with `pages: write` and `id-token: write` | the deploy |
+    | 6.9 | Docker, for the user's local check only | TechEmpower |
+
+    A JS library for the playground's editor, or any npm package, is
+    asked the same way; the default is none.
+
+## 4. Sub-phases (each gated, reviewed, and merged on the user's word)
+
+Each sub-phase runs the established loop: research where needed, the
+user's decisions, a spec, a prototype, a draft pull request measuring
+CI on the user's word and closed unmerged, a plan, the implementation,
+one fresh review and one fix pass, a pull request, and a merge on the
+user's word.
+
+**The order.** Each sub-phase needs the one before it, but where this
+paragraph says otherwise. The spike comes first (P12), after the user
+approves this plan, and the playground follows while its figures are
+fresh. The policies (6.2) move ahead of the changes that affect the
+freeze, so that the promise decides what must change first and
+`@deprecated` (6.3) builds the rule they write; 6.1 and 6.2 may run
+side by side. `std/regex` (6.4) and the client (6.5) precede hardening
+(6.6), whose fuzz targets and threat model cover them. Documentation
+(6.7) follows the last change to the language, std and the codes, so
+the index and the reference are written once; the site (6.8) needs it
+and the playground. TechEmpower (6.9) needs only 6.5 and may run beside
+6.7 or 6.8. The release (6.10) needs 6.8 and 6.9; the records (6.11)
+follow its publishes, as 5.6's did.
+
+**Sizes, in Phase 5's terms.** Phase 5's sub-phase pull requests each
+added about 1,100 (5.5b) to 4,100 lines (5.0b) outside `docs/`,
+`nova-spec/`, the CHANGELOG and the test directories, beside a spec and
+a plan; 5.6's release and records added almost none.
+
+### 6.0 — The scope ADR and the playground spike
+- **Goal:** know whether option A is viable, and record the scope.
+- **Scope:** the spike, in scratch space, offline; only its figures are
+  merged. The chain and `nova-codegen-wasm` (the thread cut by a `cfg`)
+  built for `wasm32-unknown-unknown` on stable and 1.78, with a
+  throwaway entry; measured: the size raw, gzipped and after
+  `wasm-opt`; hello world's and two examples' compile times under Node,
+  and Chromium where it can run; peak memory; the deepest of 5.1b's
+  depth cases that survives V8's stack; hello world's output.
+- **Deliverables:** ADR 0062 (§3 decisions 1 to 3, with the spike's
+  figures); this plan, approved; a dated note in the master spec.
+- **Gate:** the figures are within the spec's limits, or the user has
+  ruled; ADR 0062 merged.
+- **The user's part:** the word for the local target; the limits.
+- **Size:** records only, as 5.6c; the spike's code stays in scratch.
+- **Questions for its spec:** the limits (suggested: under 5 MB
+  gzipped, hello world in under 2 s in Chromium on CI, the depth limit
+  of 128 survived); keeping std's checked items across compiles;
+  Chromium locally or in a draft pull request.
+
+### 6.1 — The playground
+- **Goal:** compile and run Nova in the browser, with S0's own parts.
+- **Scope:** the browser's entry, apart from `nova-driver`'s native
+  backends, over in-memory `Sources` (one file, std embedded); the
+  codegen thread behind `cfg(not(target_arch = "wasm32"))`, the stack
+  set at link time; a print hook in the shim; a page (an editor, Run,
+  the output, examples, diagnostics), the compiler in a worker, the
+  program in a sandboxed iframe with a time limit and Stop; CI builds
+  the compiler, shrinks it with `wasm-opt` and prints its size.
+- **Deliverables:** the code; §1's gate item 5's Node and Chromium
+  tests, over a fixed list of programs that fit WASM's std; an ADR; the
+  page as a build output, deployed by 6.8.
+- **Gate:** both tests pass in CI; size and compile time recorded,
+  within 6.0's limits or as ruled; the WASM suite and the Selfhost job
+  unchanged.
+- **The user's part:** the word for the CI target and the entry.
+- **Size:** about 800 to 1,500 lines of Rust, JS and HTML with CI; one
+  pull request, two if the entry's split goes first; about 5.5a's.
+- **Questions for its spec:** a crate or a feature; the fixed list;
+  `std/dom` programs in the iframe, or console ones only; share links
+  (recommended: none that run on open); the editor (a textarea, or a
+  library, asked); the MSRV job and `wasm32`; a release asset or not.
+
+### 6.2 — The stability policy, the RFC process, the MSRV policy and `SECURITY.md`
+- **Goal:** what `1.0.0` promises, and how it changes, written first.
+- **Deliverables:** `docs/stability.md` (name per spec: coverage,
+  semver, editions, deprecation, exceptions, the MSRV policy);
+  `docs/rfcs/README.md` and a template, the issue template pointing at
+  them; `SECURITY.md` (supported versions, the reporting channel, what
+  a reporter can expect, disclosure); `CONTRIBUTING.md` (who decides,
+  the RFC process, the MSRV, the two-compilers rule); the snapshot
+  tests (recommended, §3 decision 4); an ADR.
+- **Gate:** the documents merged and linked from the README; the
+  snapshot tests pass, and fail on a changed signature or flag.
+- **The user's part:** the policy's text; the reporting channel and
+  its GitHub setting (private vulnerability reporting).
+- **Size:** mostly prose, 600 to 1,000 lines, with 200 to 400 of tests;
+  one pull request.
+- **Questions for its spec:** whether a code keeps its meaning; the
+  support window; what needs an RFC after 1.0 (breaking changes only, or
+  new features and std modules too); the snapshot's format; the conduct
+  contact `CODE_OF_CONDUCT.md:7` lacks.
+
+### 6.3 — Before the freeze: `@deprecated`, the two kept bugs and issue #3
+- **Goal:** a way to deprecate, and fixes to what 1.0 would promise.
+- **Scope:** `@deprecated` with a note on public items, warning at each
+  use outside the item's module on every backend, shown by `nova doc`
+  and tagged by the language server, in S0 and N (the resolver's
+  attribute list, the type checker's warning) with differential cases;
+  std's bodies no longer seeing a program's impls, in both type
+  checkers, the fallback gone; float `%` under `--release`, in both
+  `.ll` emitters; the JIT resolving `sqrt` and its kin on Linux.
+- **Deliverables:** the code, cases and golden programs; an ADR; issue
+  #3 closed by the pull request.
+- **Gate:** the cases pass under `run`, `build` and `--release`; the
+  differential on three systems and the bootstrap hold; a test runs
+  `sqrt` under `nova run` on Linux.
+- **The user's part:** rulings where a fix changes a program.
+- **Size:** about 1,000 to 2,000 lines across both compilers; one pull
+  request; about 5.0a's.
+- **Questions for its spec:** the attribute's form and targets; the
+  warning's code; whether std deprecates anything now; what the impls
+  fix changes; which libm symbols the JIT lacks.
+
+### 6.4 — `std/regex`
+- **Goal:** the std module P9 adds beside the client.
+- **Scope (recommended):** written in Nova, so it runs on both targets
+  and in the playground with no new crate; a linear-time engine (a Pike
+  VM), so no pattern makes a match exponential; a documented syntax, a
+  subset recorded as a departure from "PCRE-compatible", without
+  backreferences or lookaround; imported (`import regex`), not
+  glob-imported, so a program that does not use it pays nothing.
+- **Deliverables:** the module, documented, with an example; tests on
+  both targets; a check against an oracle (§3 decision 12) or left to
+  6.6; a section in `20-STDLIB.md`; an ADR.
+- **Gate:** the tests pass natively and on WASM; std's documentation
+  tests cover it; its matches equal the oracle's on a fixed-seed corpus.
+- **The user's part:** the oracle's crate, if chosen.
+- **Size:** about 1,500 to 3,000 lines of Nova with tests; one pull
+  request; about 5.0c's.
+- **Questions for its spec:** Nova, or the runtime over the `regex`
+  crate (native only); the syntax; Unicode classes; the API
+  (`Regex::new`, `is_match`, `find`, `captures`, `replace`, `split`).
+
+### 6.5 — The HTTP client, and handlers that await
+May split into 6.5a (the client over plain HTTP, and the handlers) and
+6.5b (HTTPS), as its spec decides.
+- **Goal:** the client half of `std/http`, and handlers that await.
+- **Scope:** the client, native only (WASM keeps `std/dom`'s `fetch`):
+  `get`, `post` and a request with a method, headers and a body;
+  responses framed by `Content-Length` or chunked; redirects, timeouts
+  and size limits; HTTPS through `rustls` on `ring` with
+  `webpki-roots`, on the user's word, over the runtime's poller;
+  handlers returning a future that the server awaits, with `ui::ssr`,
+  the examples, the blog and their tests moved to them.
+- **Deliverables:** the code; tests against servers the tests start,
+  plain and with a checked-in test certificate; `20-STDLIB.md` §6 up to
+  date; an ADR.
+- **Gate:** the tests pass on three systems with no internet; in a
+  test, a handler that awaits the client serves a request; the golden
+  suite, the WASM suite and the browser job still pass.
+- **The user's part:** the word for `rustls` and `webpki-roots`, or
+  HTTPS left for after 1.0.
+- **Size:** about 1,500 to 3,000 lines of Nova and Rust; one or two
+  pull requests; about 5.0b's in all.
+- **Questions for its spec:** the API, and a name for the client's JSON
+  decoding, `Response::json` being the server's constructor; `async fn`
+  handlers only, or both forms; whether a closure returning a future
+  serves, there being no `async` closures; how much `rustls` grows each
+  native program; bundled roots or the system's.
+
+### 6.6 — Hardening
+- **Goal:** what an auditor is handed, and checks that keep it true.
+- **Deliverables:** a threat model (`docs/security/`, name per spec):
+  §2 item 1's surfaces with the playground and the client, each
+  defence and what is left; the `unsafe` inventory, held by a test of
+  each file's count, and `#![forbid(unsafe_code)]` where there is none;
+  the dependency inventory; fuzz targets in `cargo test` on fixed
+  seeds: the lexer and the parser directly, and JSON, regex and the
+  HTTP request parser through small Nova programs fed random input, as
+  the selfhost fuzzer drives N; the dependency check in CI (licences,
+  sources and bans on every pull request; advisories on a schedule, on
+  a change to `Cargo.lock` and before a release, so an upstream
+  advisory fails no unrelated pull request); `permissions:` in every
+  workflow; an ADR.
+- **Gate:** the documents merged; the inventory tests and fuzz targets
+  pass in CI; the dependency check passes, each exception reasoned.
+- **The user's part:** the word for the check's tool and its install,
+  and any optional tool (§3 decision 12); private vulnerability
+  reporting, Dependabot alerts and branch protection on `main`,
+  recommended before the launch; whether a firm is booked.
+- **Size:** about 1,000 to 2,000 lines of tests and CI, with the
+  documents; one pull request; about 5.1a's.
+- **Questions for its spec:** `cargo-deny` or `cargo-audit`;
+  `cargo-fuzz` too, or not; the fuzz targets' time in CI; attestations
+  for the release's assets; actions pinned by commit.
+
+### 6.7 — Documentation 100%
+Two pull requests, in order (§3 decision 6).
+- **6.7a, the measurable parts:** `missing_docs` on every library crate,
+  its gaps filled; `std/dom` in the documentation tests and on std's
+  site, its examples built for WASM, not run; the error index
+  (`docs/errors/`, name per spec), an entry per code with its title,
+  explanation, example and fix; the link in each rendered diagnostic
+  and the language server's, from one base address; the tests of §3
+  decision 6.
+- **6.7b, the references:** the language reference (`docs/reference/`,
+  recommended over §2's `docs/spec/`): lexical structure, items, types
+  and inference, traits and generics, patterns, modules and packages,
+  async, attributes, the WASM target's differences, editions, the
+  grammar; the CLI and `nova.toml` references; their doc tests and
+  coverage tests.
+- **Gate:** §1's gate item 3, but for the book.
+- **The user's part:** approval only.
+- **Size:** 6.7a about 1,000 to 2,000 lines of code and comments, with
+  about 75 index entries; 6.7b thousands of lines of Markdown, little
+  code.
+- **Questions for its spec:** the link's form; `nova explain`
+  (recommended: not in 1.0, as it would join the promised CLI); the link
+  added by the renderer from the code, not as a note in each diagnostic,
+  so the dumps N is compared on stay as they are; `nova-cli` and the
+  lint.
+
+### 6.8 — The book, and the site on GitHub Pages
+- **Goal:** the tutorial site, hosting the docs and the playground.
+- **Deliverables:** `nova-doc` renders a book from
+  `docs/book/src/SUMMARY.md` and builds the site (§1's gate item 4,
+  with the CLI and manifest references, a home page and one search);
+  the book (installing, the basics, errors and `?`, collections, traits
+  and generics, async, packages, testing, a web server, a front end in
+  WASM, shipping), each code block a doc test; a workflow deploying the
+  site to Pages from a `v*` tag or by hand; the root README for users
+  (what Nova is, install, hello world, the site, the playground);
+  an ADR.
+- **Gate:** the site builds in CI with no broken internal link under the
+  `/nova/` base; the book's doc tests pass; the deploy job runs green
+  once, by hand on the user's word, and the user checks the hosted page.
+- **The user's part:** turn Pages on; the word for the Pages actions;
+  check the page.
+- **Size:** about 800 to 1,500 lines of Rust and workflow, with 3,000 to
+  5,000 lines of book; one or two pull requests.
+- **Questions for its spec:** a hidden subcommand or flag; the deploy's
+  trigger (the tag only, or `main` as a preview); whether std's site
+  stays a release asset; the README's two relative links in
+  "Self-hosting", which the rewrite replaces.
+
+### 6.9 — The TechEmpower app
+- **Goal:** what a submission needs, in the repository (P8).
+- **Deliverables:** the app for the JSON and plaintext test types, with
+  the files TechEmpower's layout asks for; a CI test building it with
+  the checkout's `nova` and checking each response's status, headers and
+  body; a `Dockerfile` installing the released `nova`, its version in
+  the release's bump and in `release_manifests.rs`; how to submit and to
+  run it locally.
+- **Gate:** the test passes in CI; beside it, the rules each response
+  meets, as read.
+- **The user's part:** reading the rules, or leave for the session to
+  read them; the pull request to TechEmpower, after `1.0.0`.
+- **Size:** about 200 to 500 lines; one pull request; smaller than 5.6a.
+- **Questions for its spec:** the rules (headers such as `Date` and
+  `Server`, plaintext's pipelining); a process per core, the executor
+  using one (`std/net` has no `SO_REUSEPORT`); the app's directory.
+
+### 6.10 — The release `1.0.0`, and the launch materials
+Two pull requests, in order, then the publishes.
+- **6.10a, the launch materials:** an announcement, on the site and in
+  `docs/launch/` (name per spec); drafts of the posts for each site item
+  7 names, for the user to edit and post; `[Unreleased]` read through as
+  `1.0.0`'s notes.
+- **6.10b, the release:** the bump by `docs/releasing.md` with
+  `version=1.0.0` and `CHANGELOG.md`'s `[1.0.0]`; then, each on the
+  user's word, the tag, the GitHub release, the 20 crates, the
+  extension in both stores, the Release gate on three systems, the
+  user's try, and the site's deploy from the tag.
+- **Gate:** §1's gate item 7; the hosted site and playground at
+  `1.0.0`.
+- **The user's part:** every [stop] of the runbook; the posts, when the
+  user chooses.
+- **Size:** prose for 6.10a; about 5.6b's for 6.10b.
+- **Questions for its spec:** a release candidate first, or not; the
+  runbook's new steps (the deploy, the app's version); whether the
+  user's try replaces step 6.2 again, as for `v0.5.0`.
+
+### 6.11 — The records, and the boundary ADR
+- **Deliverables:** the boundary ADR, as ADR 0061 closed Phase 5: the
+  inventory of the seven items and of this plan's decisions; the gate's
+  record with its runs; each external item, done or not (the audit, the
+  TechEmpower pull request, the posts, the domain); the backlog (§3
+  decision 1's list, ADR 0061's carried); dated notes in the master
+  spec's Phase 6, `20-STDLIB.md`, `40-TOOLING.md`, `50-TESTING.md`,
+  `docs/releasing.md` and this plan; `[1.0.0]`'s note. Phase 6 is the
+  master spec's last, so the ADR says later work goes through the RFC
+  process.
+- **Gate:** every item of §7's Definition of Done met, as recorded.
+- **The user's part:** the external items' status; the merge.
+- **Size:** records only, as 5.6c.
+
+## 5. Cross-cutting
+
+- **The workflow** of §4, in every sub-phase. Nothing is pushed,
+  merged, tagged or published without the user's word; no auto-merge;
+  merges by rebase.
+- **Testing:** tests never touch the internet (servers and certificates
+  are the test's own); tests run with a fresh `NOVA_HOME`; fuzz targets
+  run on fixed seeds in CI and longer locally; the playground is tested
+  in Node and in the browser job's Chromium.
+- **Dependencies and tools** (§3 decision 12): no new crate, no change
+  to `Cargo.lock`, no rustup target, CI tool or third-party action
+  without the user's word, after the spec shows it. Rust builds on
+  1.78, which CI's MSRV job checks.
+- **CI time:** every program type-checks the prelude, so std's growth in
+  6.5 (and 6.4, if regex joins the prelude) slows every run, N's
+  differential among them, against the Selfhost job's lines of 80, 100
+  and 110 s (ADR 0061 `:205`); a line crossed is the user's ruling, as
+  in Phase 5. The playground's build and tests, the fuzz targets and the
+  dependency check add their own time; each spec measures it.
+- **The release runbook for `1.0.0`:** `docs/releasing.md` as for
+  `v0.5.0`, with two steps added, the site's deploy from the tag (a
+  [stop]) and the app's `Dockerfile` in the bump; `v0.5.0`'s lessons
+  stand (ADR 0061 `:340-353`).
+- **Records:** the master spec's Phase 6 and the companion specs get
+  dated notes as each sub-phase lands, each departure an ADR, and this
+  plan dated amendments, as Phase 5's did. ADR 0061 found notes missing
+  in Phase 5 (`:430-436`); 6.11 checks every one.
+- **Pull requests** carry no local path and no personal detail.
+- **GitHub's settings** are the user's: Pages, private vulnerability
+  reporting, Dependabot alerts, branch protection, environments.
+
+## 6. Top risks
+
+1. **The playground in a browser** (§2 item 5).
+   - Size and speed: 55,749 lines of Rust and 743 KB of std in one
+     module, which loads slowly on a phone, and each Run type-checks the
+     whole prelude std (6.0 asks whether std's checked items can be
+     kept).
+   - Stack and dependencies: natively, the backend compiles on a 256 MB
+     thread (`crates/nova-codegen-wasm/src/lib.rs:63`); in V8 a deep
+     input may overflow, and the page must say so. The chain's `wasm32`
+     build is unverified; a dependency that fails needs a `cfg` or a
+     replacement, asked.
+   - The origin: Pages serves an account's project sites from one
+     origin, so a program run there could read what the account's other
+     sites keep; programs run in a sandboxed iframe with an opaque
+     origin, and no share link runs on open.
+   - The remedy: the spike first, with limits and the user's ruling.
+2. **Regex's size.** "PCRE-compatible" is open-ended; a backtracking
+   engine is exponential on some patterns, a denial of service for a
+   server matching untrusted input; Unicode classes need tables; Nova
+   code runs about 5× slower than Rust (`docs/phase-5-plan.md:172-174`).
+   The remedy: a subset, a linear engine, an import, an oracle.
+3. **The client's TLS.** HTTPS puts `rustls` in the runtime, over the
+   poller, on three systems; roots go stale; test certificates expire;
+   every native program's runtime archive may grow. The remedy: HTTPS
+   last (6.5b); if the user declines the crates, HTTP only in 1.0.
+4. **The audit may not happen.** 1.0 then ships with in-repo hardening
+   alone (P7), as the boundary ADR and `SECURITY.md` say plainly; the
+   threat model and inventories make a later audit cheaper, and a
+   finding after 1.0 is a patch release.
+5. **The phase's length.** Twelve sub-phases, about sixteen pull
+   requests, much of it prose (the error index, the reference, the
+   book), against 16 weeks. The remedy: the measurable parts first; a
+   prose sub-phase may split; nothing external blocks 1.0 (P1).
+6. **Two compilers, and CI time.** 6.3 costs twice; std's growth slows
+   N's differential toward the Selfhost job's lines; a change missed in
+   N fails the differential, which is the guard.
+7. **The freeze locks in what exists:** the prelude's names, the
+   handler type, `nova.toml`'s keys; the language's gaps (ADR 0061
+   `:377-386`) are additive. The policy's exclusions, the snapshots and
+   `@deprecated`'s two minor versions' notice are the remedy.
+8. **A pin held back for 1.78 gets an advisory.** `ureq` 3.2.1 and
+   `wasm-encoder` 0.243.0 cannot move without a newer Rust; the
+   dependency check finds it, and the raise is the user's (P11).
+9. **TechEmpower's showing.** One executor uses one core (ADR 0009
+   `:57`), so a many-core run shows one core's throughput unless the
+   app runs a process per core; submitting is the user's decision.
+10. **Links that move.** Diagnostics link to the Pages site, which a
+    later domain moves; one base address, wording outside the promise,
+    and Pages' redirect of the old address (6.8's spec) contain it.
+
+## 7. Suggested first step
+
+6.0: ask the user's word for the local `wasm32-unknown-unknown` target,
+run the spike in scratch space, and write 6.0's spec with its figures
+and limits, then ADR 0062. Nothing comes before it (P12).
````

<!-- patch t1-adr -->

````diff
diff --git a/docs/adr/0062-phase-6-scope.md b/docs/adr/0062-phase-6-scope.md
new file mode 100644
index 00000000..417606d5
--- /dev/null
+++ b/docs/adr/0062-phase-6-scope.md
@@ -0,0 +1,256 @@
+# ADR 0062 — Phase 6's scope, the playground and 1.0.0
+
+## Status
+
+Accepted (2026-11-27). Branch `claude/loving-archimedes-frtf5q-60`
+(`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`
+§3.2), with `docs/phase-6-plan.md`, which the user approved on
+2026-11-26. The scope is the user's decisions of 2026-11-26, P1 to P12
+(the plan's §1), with the `wasm32-unknown-unknown` target allowed
+locally for the spike and, after it, the long-function limit caught on
+the page (Q1). This ADR confirms the plan's §3 decisions 1 to 3; the
+others are confirmed in their sub-phases' specs. It names `main`'s
+commits only.
+
+## Context
+
+`nova-spec/00-MASTER-SPEC.md` §3 makes Phase 6 "1.0 Release", weeks 105
+to 120, the last of its phases, with seven items (`:936-942`): an
+external security audit; a stability freeze, "RFC for any breaking
+change"; documentation 100%; a tutorial site at `novalang.dev`; an
+interactive playground, to "compile in browser via WASM-compiled Nova
+compiler" (`:940`); a TechEmpower submission; and launch posts. Unlike
+Phases 0 to 5, it has no goal and no gate (`:935-943`). Around it, §1.9
+(`:99-102`) allows breaking changes before 1.0 with an RFC, then asks
+strict semver, editions for opt-in breaks and two minor versions' notice
+before a removal; §5.3 asks every user-facing error for a "Link to
+docs"; and §7's Definition of Done tags each phase `v0.{phase}.0`, which
+reads `v0.6.0` for this one.
+
+ADR 0048 left N built for WASM out of Phase 5 as what "Phase 6's
+playground needs" (`:44`), and ADR 0061 kept that mapping: R5 (`:93`),
+its inventory's row (`:148`) and its backlog (`:394-395`). The master
+spec's `:940` names no stage.
+
+The research of 2026-11-26 on `main` at `4fe11d55`
+(`docs/phase-6-plan.md` §2), a line for each item:
+- **the audit:** no `SECURITY.md`, threat model, fuzz targets or
+  dependency check; 254 packages in `Cargo.lock`, two pins held back by
+  MSRV 1.78;
+- **the freeze:** no stability policy or RFC process, no test of std's
+  signatures, no way to deprecate; two S0 bugs ADR 0061 keeps; issue #3;
+- **documentation:** std at 100% by test, but for `std/dom`, which ADR
+  0047 left off the site; no `missing_docs`, reference, book or error
+  index;
+- **the site:** none, no Pages workflow, and no `novalang.dev`;
+- **the playground:** S0's WASM backend needs no other toolchain (ADR
+  0039); the front end reads through `Sources`
+  (`crates/nova-driver/src/analyze.rs:25-42`); `build_program_wasm`
+  builds in memory before it writes (`lib.rs:865-892`); the front-end
+  crates and `nova-codegen-wasm` use no OS API but codegen's 256 MB
+  thread; `nova-driver`, which runs `clang` and depends on the native
+  runtime, cannot be the browser's entry;
+- **TechEmpower:** a server, with no database driver and no client;
+- **the launch:** the user's; the repository can give notes and drafts;
+- **the release:** `docs/releasing.md` runs unchanged for `1.0.0`;
+- **std's v1.0 index:** unbuilt, `Queue`, atomics and `RwLock`, UDP and
+  Unix sockets, the HTTP client, AEAD and `std/regex`;
+- **two compilers:** CI holds N equal to S0, so a change to a ported
+  part changes both;
+- **the tools:** Node 22, Rust 1.97 with the 1.78 toolchain, and no
+  `wasm32-unknown-unknown` target.
+
+N built for WASM is further off: its MIR is native only (ADR 0057
+`:521-534`), and under its `async fn main` a compile, one long turn,
+would never collect (ADR 0040 `:42-43`). Names, as in Phase 5's plan:
+**S0** is the Rust `nova`; **N** is the Nova compiler in `selfhost/`.
+
+The user's decisions of 2026-11-26, labelled as in the plan's §1:
+**P1,** all seven items, the external parts the user's; **P2,** the
+playground S0 in the browser after a spike, N for WASM to the backlog;
+**P3,** `1.0.0` as the phase's release; **P4,** GitHub Pages until there
+is a domain; **P5,** documentation, all of it; **P6,** what the
+stability promise covers; **P7,** in-repository hardening as the audit's
+gate; **P8,** TechEmpower's test types that need no database; **P9,**
+`@deprecated`, the two kept bugs, issue #3, `std/regex` and the HTTP
+client before the freeze; **P10,** none of ADR 0061's backlog; **P11,**
+MSRV 1.78, with a policy; **P12,** the spike first. The spike, on the
+target the user allowed locally, measured S0's front end and WASM
+backend built for `wasm32-unknown-unknown`: their size, speed, memory
+and output, and the depth cases at the parser's limit of 128 levels (ADR
+0052), as below. After it, **Q1:** a long function's overflow is caught
+on the page.
+
+## Decision
+
+1. **Scope: all seven items** (P1). Each item's in-repository part is
+   built and gated (`docs/phase-6-plan.md` §1). The external parts are
+   the user's: an audit firm, the TechEmpower pull request, the launch
+   posts, a domain and its DNS, switching GitHub Pages on, and any
+   account or money. Phase 6's closing ADR records each, done or not,
+   and none blocks `1.0.0`.
+2. **What Phase 6 leaves out,** each to the backlog unless a sub-phase's
+   spec pulls it back in with a ruling that names this ADR:
+   - N built for WASM (decision 3);
+   - a database driver (decision 9);
+   - UDP and Unix sockets, AEAD, `Queue`, `RwLock` and atomics, of
+     `nova-spec/20-STDLIB.md` §1's index (decision 10);
+   - ADR 0061's backlog, the bootstrap on macOS and Windows, N on a
+     large-stack thread, E0093's `\` on Windows and compiler speed among
+     it, but for the two bugs decision 10 fixes;
+   - what other specs give Phase 6 and its list does not name: multiple
+     toolchains (`nova-spec/40-TOOLING.md:646`; ADR 0037 `:93`) and
+     `nova-embed` (`nova-spec/13-RUNTIME.md:480`);
+   - what the specs and earlier ADRs put elsewhere, not in Phase 6, left
+     there: the `setup-nova` action (`40-TOOLING.md:583-598`), on Phase
+     3's backlog (ADR 0026 `:50`; ADR 0037 `:100`); examples 08 to 15
+     (`nova-spec/60-EXAMPLES.md:801-803`), "post-Phase 6", after the 1.0
+     release; and edition rules (`40-TOOLING.md:436-439`), on Phase 3's
+     backlog (ADR 0037 `:97`).
+3. **The playground is S0 in the browser** (P2), as the spike below
+   showed it can be. S0's front end, from the lexer to MIR, and its WASM
+   backend, built for `wasm32-unknown-unknown`, compile one source text
+   in the page, with std embedded, and return the diagnostics as text,
+   or `app.wasm` and `app.js`. The program runs in a sandbox away from
+   the site's origin, through hand-written JS glue, with no
+   `wasm-bindgen`.
+   - **It amends ADR 0048 (`:44`) and ADR 0061 (`:148`, `:394-395`),**
+     which gave the playground N built for WASM. N built for WASM goes
+     to the backlog. The master spec is not amended: its "WASM-compiled
+     Nova compiler" (`:940`) names no stage.
+   - **The limits,** which the spike met: the compiler under 5 MB
+     gzipped; hello world compiled in under 2 s in Chromium; every depth
+     case at the parser's limit, 128, survived with no trap. 6.1's gate
+     measures the size and hello world's time again, in CI.
+   - **A long function body** overflows the stack in the browser where
+     natively it builds: codegen's structurer recurses once per
+     sequential join or loop, which is why S0 compiles on a 256 MB
+     thread (`crates/nova-codegen-wasm/src/lib.rs:61-63`). The page
+     catches the overflow and says so (Q1). S0's codegen does not change
+     before the freeze; an iterative structurer goes to the backlog.
+   - **What 6.1 builds,** asked as decision 12 says: the browser's entry,
+     a new crate or a split of `nova-driver`, either a change to
+     `Cargo.lock`; codegen's thread kept off `wasm32` by a `cfg`
+     (`lib.rs:198-208`); a larger shadow stack, set at link time; a print
+     hook in the shim; the target in CI. No new dependency.
+4. **The version: `1.0.0`** (P3) is Phase 6's release, in place of the
+   `v0.{phase}.0` of the master spec's §7, item 6, which reads `v0.6.0`
+   for this phase. `docs/releasing.md` runs unchanged with
+   `version=1.0.0`. A `1.0.0-rc.N` comes only if the audit, the freeze
+   or the Release gate asks for one, on the user's word. Semver is strict
+   after it (the master spec's §1.9, `:101`).
+5. **The stability promise** (P6), written in 6.2 and in force from
+   `1.0.0`, covers the language at edition `"2026"`, std's public API on
+   both targets, the CLI's commands, documented flags and exit codes,
+   `nova.toml`, `nova.lock` and the index line. It does not cover the
+   hidden flags, the Rust crates, whose API is internal, N, diagnostics'
+   wording, the generated JS's and WASM's internals, the playground, or
+   performance; 6.2's spec confirms the details. **Breaking changes
+   before `1.0.0`:** §1.9 asks an RFC for each (`:100`). As in Phases 1
+   to 5, a sub-phase's approved spec and its ADR stand as that RFC, for
+   6.3 to 6.5 too; the RFC process 6.2 writes governs every change after
+   `1.0.0`.
+6. **Documentation 100%** (P5), each part held by tests where it can be:
+   std's documentation on the site, with `std/dom`, which this ADR takes
+   off ADR 0047's backlog (`:180-184`); `missing_docs` on the Rust
+   library crates; an error-code index, each rendered diagnostic linking
+   to its entry (the master spec's §5.3); a language reference, the CLI and
+   `nova.toml` references, and a book.
+7. **The audit's in-repository gate is hardening** (P7): a threat model,
+   the `unsafe` and dependency inventories, fuzz targets, a dependency
+   check in CI and `SECURITY.md`. The external audit is the user's,
+   recorded as not done unless booked; if the user books a firm in time,
+   `1.0.0` may wait for its findings, on the user's word.
+8. **The site is a project site on GitHub Pages** (P4), under `/nova/`,
+   which the user turns on, built by `nova-doc` as the plan recommends
+   and 6.8's spec confirms. There is no `novalang.dev`
+   (`nova-spec/40-TOOLING.md:626`); a custom domain may follow, if the
+   user buys one.
+9. **TechEmpower** (P8): the test types that need no database, JSON and
+   plaintext, as an app in the repository, with how to submit it. The
+   pull request to TechEmpower is the user's.
+10. **Before the freeze** (P9): an `@deprecated` attribute; fixes to the
+    two S0 bugs ADR 0061 keeps (`:391-393`), std's bodies seeing a
+    program's impls and float `%` failing `build --release`; issue #3,
+    E0902 on `sqrt` under `nova run` on Linux; `std/regex`; and the HTTP
+    client, with handlers that can await. **Out** (P10): the rest of ADR
+    0061's backlog, and of `20-STDLIB.md` §1's index, as decision 2
+    lists.
+11. **The MSRV stays 1.78** (P11), with a policy written in 6.2: a raise
+    is the user's decision, in a minor release, with an ADR; an advisory
+    in a pin held back for 1.78 may force one.
+12. **No new crate, rustup target or CI tool without the user's word,**
+    asked in its sub-phase's spec once the spec shows the change
+    (`docs/phase-6-plan.md` §3 decision 12): the target in CI and the
+    entry in 6.1; an oracle crate in 6.4; `rustls` and `webpki-roots` in
+    6.5; the dependency check's tool in 6.6; the Pages actions in 6.8.
+    The user allowed the target locally, for the spike, on 2026-11-26.
+
+### The spike's figures
+
+Measured on 2026-11-26 in a spike, its code not committed, as
+`docs/phase-5-plan.md` §2 item 9 records Phase 5's lexer spike. It built
+the front-end crates and `nova-codegen-wasm` of `4fe11d55` for
+`wasm32-unknown-unknown` with Rust 1.97, offline, from the crates
+`Cargo.lock` holds, with a throwaway entry over the C ABI that copies
+about 60 lines of `nova-driver`'s private front end. Its one change to
+S0, in its own copy, calls `compile` directly on `wasm32`
+(`crates/nova-codegen-wasm/src/lib.rs:198`); without it every compile
+fails at run time, starting the thread. The programs: hello world;
+`traits`, `json_round_trip`, `collections` (18 KB) and `iterator`
+(22 KB) of the golden suite; `examples/03-producer-consumer`; one with a
+type error. Node 22 and Chromium 153 (Playwright's headless shell) on
+one machine; times are medians of five compiles in one instance.
+
+| Measure | Figure | Limit |
+|---|---|---|
+| Size, `gzip -9`, the release profile (opt-level 3) | 830,149 bytes (2,937,593 raw) | under 5 MB |
+| Size gzipped, opt-level `z`, LTO, one codegen unit, stripped | 519,967 bytes (1,534,479 raw) | |
+| Hello world, the first compile in Chromium | 197 ms (126 ms at `z` with LTO) | under 2 s |
+| Hello world, later compiles | 34.2 ms in Chromium, 28 ms in a worker, 40.3 ms in Node | |
+| The seven programs, later compiles | 26.9 to 40.8 ms in Chromium, about 30 ms of each std's | |
+| The compiler ready after navigation | 119 ms (60 ms at `z` with LTO) | |
+| The depth limit, 128 | 15 kinds build and run at 128 in Node and in Chromium, on the main thread and in a worker, the 16th (nested `fn` items) E0900 as natively; 206 probes in Node and 100 a thread in Chromium, no trap; past 128, P0002 | 128 survived |
+| V8's stack at 128 | 350 to 450 KB of Node's 984 KB; a worker's stack about 40% of the main thread's | |
+| Rust's shadow stack, 1 MiB by default | 768 KiB passes at 128; 512 KiB traps on 20 of 42 probes | |
+| Memory | 2.25 MB fresh; about 10 MB after one compile; 41 MB for a 140 KB function; flat over 50 compiles | |
+| Output | the six that run print what `nova run` prints; the type error's text is `nova run`'s but for the file name; `app.wasm` and `app.js` equal `nova build --target wasm`'s, byte for byte | |
+| A long body, the main thread | 3,000 `if`s in a row compile, 4,000 overflow; 1,200 `while` loops compile, 1,500 overflow | caught (decision 3) |
+| A long body, a worker | 1,500 `if`s compile, 2,000 overflow; 600 loops compile, 800 overflow | caught |
+
+Not measured: a build on Rust 1.78; the size after `wasm-opt`; Chromium
+in CI, on phones and on slow machines; a real network's transfer time; a
+worker's exact headroom; `std/dom` programs. 6.1 measures the first
+three it can.
+
+## Consequences
+
+- **The master spec's Phase 6 list is not amended;** this ADR records the
+  difference, as ADRs 0026, 0038 and 0048 did for Phases 3 to 5, and a
+  dated note under the list points here. §7 keeps `v0.{phase}.0` for the
+  phases before; Phase 6's milestone is `1.0.0`.
+- **ADRs 0048 and 0061 keep their text,** each with a dated note at the
+  mapping decision 3 changes. Phase 6 builds nothing in N for the
+  playground, which, being S0, needs no port.
+- **6.1 inherits the spike's caveats** (a long body; thin headroom in a
+  worker and in the shadow stack) and unknowns (1.78; `wasm-opt`, which
+  CI's browser job already installs; Chromium in CI), and changes
+  `Cargo.lock` and CI's targets on the user's word.
+- **The backlog grows** by decision 2's list, which Phase 6's closing ADR
+  keeps, with each external item, done or not. Phase 6 is the master
+  spec's last; later work goes through 6.2's RFC process.
+- **From `1.0.0` on, decision 5's promise holds:** a break needs a new
+  edition or a recorded exception.
+
+## References
+
+- Spec: `docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`,
+  and its plan,
+  `docs/superpowers/plans/2026-11-27-phase-6-0-the-plan-and-adr-0062.md`
+- `nova-spec/00-MASTER-SPEC.md` §1.9, §3, §5.3 and §7
+- `docs/phase-6-plan.md` §1, §3, and §4's 6.0 and 6.1
+- `docs/phase-5-plan.md` §2 item 9, Phase 5's lexer spike
+- ADRs 0026, 0037, 0038, 0039, 0040, 0047, 0048, 0052, 0057 and 0061
+- `crates/nova-codegen-wasm/src/lib.rs:61-63` and `:198-208`;
+  `crates/nova-driver/src/analyze.rs:25-42` and `lib.rs:865-892`
+- `.github/workflows/ci.yml`'s browser job
````

<!-- patch t1-notes -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index ea19a2bc..fd1e988e 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -9,6 +9,16 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
 
 ## [Unreleased]
 
+**Recorded 2026-11-27 (branch `claude/loving-archimedes-frtf5q-60`):**
+Phase 6.0, the plan and Phase 6's scope (spec
+`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`;
+ADR 0062, Phase 6's scope; `docs/phase-6-plan.md`). Phase 6, "1.0
+Release", will release `1.0.0`, in place of the `v0.{phase}.0` milestone
+`nova-spec/00-MASTER-SPEC.md` §7 reserves for a phase, and its
+playground will compile Nova in the browser with the Rust `nova`'s own
+front end and WASM backend, built for WebAssembly. Nothing in `nova`
+changes yet.
+
 ## [0.5.0] - 2026-11-24
 
 **Phase 5, "Self-hosting", is released.** This is the `v0.{phase}.0`
diff --git a/docs/adr/0048-phase-5-scope-and-backend.md b/docs/adr/0048-phase-5-scope-and-backend.md
index 5bb1f868..b30a6509 100644
--- a/docs/adr/0048-phase-5-scope-and-backend.md
+++ b/docs/adr/0048-phase-5-scope-and-backend.md
@@ -42,6 +42,9 @@ server with the runner's.
    - S1 as the shipped `nova`: `v0.5.0` ships the Rust `nova`;
    - the byte gate on macOS and Windows;
    - N built for WASM, which Phase 6's playground needs;
+     **Amended 2026-11-27 (branch `claude/loving-archimedes-frtf5q-60`):**
+     Phase 6's playground is S0 built for WASM, and N built for WASM is
+     on the backlog (ADR 0062);
    - trait objects, mutable globals and nested module directories.
 3. **The backend.** N writes textual `.ll` and runs `clang` and `cc`, as
    S0's `--release` does, through 5.0c's `std/process`. This departs from
diff --git a/docs/adr/0061-phase-5-boundary.md b/docs/adr/0061-phase-5-boundary.md
index 844c759f..dbba0e40 100644
--- a/docs/adr/0061-phase-5-boundary.md
+++ b/docs/adr/0061-phase-5-boundary.md
@@ -393,6 +393,9 @@ needs it, and the list promises no date, no phase and no order.
   fails in `build --release` (ADR 0058 `:885-886`).
 - **Mapped elsewhere, to Phase 6 alone** (R5): N built for WASM, for the
   playground (`nova-spec/00-MASTER-SPEC.md:940`).
+  **Amended 2026-11-27 (branch `claude/loving-archimedes-frtf5q-60`):**
+  ADR 0062 makes Phase 6's playground S0 built for WASM, by the user's
+  decision of 2026-11-26, and puts N built for WASM on this backlog.
 - **Taken from earlier backlogs:** ADR 0025's `?` operator (`:327`) and
   `std/process`'s `spawn` and `env` (`:337`) are shipped, by 5.0a and
   5.0c, with a blocking `run` in place of `spawn`; ADR 0047's nested
diff --git a/docs/phase-6-plan.md b/docs/phase-6-plan.md
index d891448d..ab378414 100644
--- a/docs/phase-6-plan.md
+++ b/docs/phase-6-plan.md
@@ -424,6 +424,19 @@ a plan; 5.6's release and records added almost none.
   of 128 survived); keeping std's checked items across compiles;
   Chromium locally or in a draft pull request.
 
+**Amended 2026-11-27 (6.0, branch `claude/loving-archimedes-frtf5q-60`,
+ADR 0062; spec
+`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`):**
+the spike ran on 2026-11-26, its code not committed. Option A is within
+its spec's limits, measured in Node and a local Chromium: 519,967 to
+830,149 bytes gzipped; hello world compiled in 197 ms the first time,
+about 34 ms after; every depth case at 128 survived, in a worker too. It
+built on stable alone; the build on 1.78, the size after `wasm-opt` and
+Chromium in CI are 6.1's. A long function body overflows codegen's
+recursion in the browser: 6.1 catches it and says so, by the user's
+decision of 2026-11-26, and an iterative structurer is on the backlog.
+This plan's `path:line`s stay `4fe11d55`'s.
+
 ### 6.1 — The playground
 - **Goal:** compile and run Nova in the browser, with S0's own parts.
 - **Scope:** the browser's entry, apart from `nova-driver`'s native
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 9cd4899b..74608f99 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -941,6 +941,20 @@ met.
 6. Submit to TechEmpower benchmarks
 7. Launch posts: HN, Lobsters, Reddit r/programming, dev.to, Twitter
 
+**Amended 2026-11-27 (branch `claude/loving-archimedes-frtf5q-60`):**
+Phase 6 follows `docs/phase-6-plan.md`, which the user approved on
+2026-11-26 and whose §1 gives the gate this list lacks, within the scope
+`docs/adr/0062-phase-6-scope.md` records. All seven items are in it: the
+in-repository part of each is built and gated, and the external parts
+(an audit firm, the TechEmpower pull request, the launch posts, a
+domain, switching GitHub Pages on) are the user's, recorded, and do not
+block 1.0. Item 5's playground is the Rust `nova`'s front end and WASM
+backend, built for WebAssembly and run in the browser, as a spike of
+2026-11-26 measured it; building the Nova compiler in `selfhost/` for
+WASM, which ADRs 0048 and 0061 mapped here, is on the backlog. Item 4's
+site is on GitHub Pages until there is a domain. The phase's release is
+`1.0.0`, in place of the `v0.6.0` that §7 would give it.
+
 ---
 
 ## 4. Files in This Spec Bundle
````

### Task 2: Verification, the mutants, the review, the PR, and the merge

Every step runs in a shell set up as the conventions say, at Task 1's
records commit, committed, `D` the day ADR 0062's Status carries:
`D=$(sed -n 's/^Accepted (\([0-9-]*\))\. Branch .*/\1/p'
docs/adr/0062-phase-6-scope.md)`.

- [ ] **Step 1: The records' checks, all of them** (decision 25), each
  command's output kept in a log, each read whole:

  ```sh
  CHK hashcheck $(MSGS) > "$P/exec60/c-hash.txt" 2>&1; echo "exit $?"
  CHK pathcheck > "$P/exec60/c-path.txt" 2>&1; echo "exit $?"
  CHK privacy $(MSGS) > "$P/exec60/c-privacy.txt" 2>&1; echo "exit $?"
  CHK notes "$D" > "$P/exec60/c-notes.txt" 2>&1; echo "exit $?"
  CHK files > "$P/exec60/c-files.txt" 2>&1; echo "exit $?"
  CHK place "$D" > "$P/exec60/c-place.txt" 2>&1; echo "exit $?"
  CHK planfile > "$P/exec60/c-planfile.txt" 2>&1; echo "exit $?"
  CHK guard > "$P/exec60/c-guard.txt" 2>&1; echo "exit $?"
  CHK split "$D" > "$P/exec60/c-split.txt" 2>&1; echo "exit $?"
  bash "$P/exec60/redate.sh" --check "$D" > "$P/exec60/c-redate.txt" 2>&1; echo "exit $?"
  printf '%s\n' "$(git log -1 --format=%B HEAD~1)" | diff - "$P/exec60/msg-t1-plan.txt" && echo "the plan's message is the file's"
  printf '%s\n' "$(git log -1 --format=%B HEAD)" | diff - "$P/exec60/msg-t1.txt" && echo "the records' message is the file's"
  tail -n 2 "$P"/exec60/c-*.txt
  git status --short
  ```

  Expected, `exit 0` each, and:
  - `hashcheck`: 1,067 lines read (the added lines and the two messages), 1
    distinct word, `4fe11d55`, 6 times (five in the records, once in the
    approved plan's message), a commit on `main` by `git merge-base`; no
    pull request head, no pre-rebase hash, no commit of this branch;
  - `pathcheck`: 38 ADR citations, 83 of the plan's at `4fe11d55` (8 of them
    reading otherwise in the tree), 64 repository paths (this plan's among
    them), 21 names that are not paths, 0 Markdown links, 0 URLs; the eight
    are the plan's `:5` (`:996`, `:1126-1134`), `:32`, `:33`, `:185`,
    `:306`, `:341` and `:703` (decision 34);
  - `privacy`: 1,067 lines read (1,049 added lines and 18 message lines), 2
    names built; allowed, the owner's login once, in the plan's Pages
    address, and the records' branch 6 times;
  - `notes`, `place`, `planfile` (now `commits touching it: 2`), `guard`
    and `split` as Task 1, Steps 4 and 5 give them; `files`, now `0
    untracked`;
  - `redate.sh --check`: the six places holding `D`, the history as Task
    1, Step 4 gives it, `redate: ok`;
  - `the plan's message is the file's`, `the records' message is the
    file's`; `git status --short` prints nothing.
- [ ] **Step 2: The diff, read whole.**

  ```sh
  git diff --stat origin/main HEAD | tail -n 1
  git diff --name-only origin/main HEAD
  git log --format='%h %s' origin/main..HEAD
  git diff --check origin/main HEAD -- . ':(exclude)docs/superpowers'; echo "exit $?"
  git diff --name-only origin/main HEAD -- crates std selfhost tests examples .github tools Cargo.toml Cargo.lock
  git diff origin/main HEAD -- . ':(exclude)docs/superpowers' ':(exclude)docs/phase-6-plan.md' > "$P/exec60/records.diff"
  git diff HEAD~1 HEAD -- docs/phase-6-plan.md > "$P/exec60/plan-note.diff"
  ```

  Expected: 8 files, the six, the spec and this plan; the log, newest
  first, the records commit, the approved plan's, this plan's and
  `8709e8dc`; `exit 0` and nothing printed by `--check`: no added line
  of the records holds trailing whitespace; nothing under the code's
  paths. Then read `records.diff` (the ADR, the master spec, the
  CHANGELOG and the markers) and `plan-note.diff` (the 6.0 note, 13 lines
  added, nothing else) whole, against spec §3 and its amendment: each of
  §3.2's parts, each note at its place, nothing the spec does not hold.
  The approved plan itself is `planfile`'s: byte for byte, not read again
  here.
- [ ] **Step 3: The mutants** (decision 27). One at a time, each edit is
  made to the committed tree, every check runs, and the files are
  restored with `git checkout --`, `git status --short` checked empty
  before and after. The runner does it, and the executor uses it as
  given, writing no runner of its own: it holds the 15 mutants, refuses to
  start if `$P/exec60/mut/` holds anything, runs every check on the
  unbroken tree first (all must pass), and writes each result to
  `$P/exec60/mut/<n>.json` with its log beside it. Write it, as it
  stands:

```sh
mkdir -p "$P/exec60"
cat > "$P/exec60/mutrun.py" <<'EOF'
#!/usr/bin/env python3
r"""mutrun.py: break the records' files one way at a time, on the
checked-out tree (the records commit, committed and clean), and see which
of the records' checks catch each. Run from the repository's root with $P
exported (and TMPDIR under $P/exec60/); it writes nothing but
$P/exec60/mut/. Every mutant, in order, in one run:
  1. `git status --short` must print nothing;
  2. its steps: `redate` runs `redate.sh N` (N the day after the tree's),
     then each edit: its `old` text must occur exactly once in its file
     ({D} in a text is the tree's day, from ADR 0062's Status; {N} the day
     after it; {P} the scratch directory);
  3. the checks, with D the day ADR 0062's Status then carries: checks.py
     hashcheck and privacy (each with the two messages), pathcheck,
     notes D, files, place D, planfile and guard, and redate.sh --check D
     (not split, which any edit of the six files fails by design);
  4. the files restored with `git checkout --`, then `git status --short`
     must print nothing again.
The checks run on the unbroken tree first, and must all pass. A mutant is
caught when a check fails; it is as expected when the checks that failed
are its `expect` list exactly (an empty list: accepted, for the reason
given). Results go to $P/exec60/mut/<n>.json, each with its log; the run
refuses to start if mut/ holds anything (after a cut-off run: restore the
tree, move mut/ aside to a fixed name, and run again from the first).
"""
import datetime
import json
import os
import re
import subprocess
import sys
import time

P = os.environ["P"]
X = f"{P}/exec60"
OUT = f"{X}/mut"
REPO = os.getcwd()
if not os.path.exists(os.path.join(REPO, ".git")):
    raise SystemExit("run mutrun.py from the repository's root")
ADR = "docs/adr/0062-phase-6-scope.md"
PLAN = "docs/phase-6-plan.md"
MS = "nova-spec/00-MASTER-SPEC.md"
A61 = "docs/adr/0061-phase-5-boundary.md"
TOUCH = ["CHANGELOG.md", "docs/adr/0048-phase-5-scope-and-backend.md", A61, ADR, PLAN, MS, "README.md"]
MSGS = ["--msg", f"{X}/msg-t1-plan.txt", "--msg", f"{X}/msg-t1.txt"]
CHECKS = [
    ("hashcheck", ["python3", f"{X}/checks.py", "hashcheck", *MSGS]),
    ("pathcheck", ["python3", f"{X}/checks.py", "pathcheck"]),
    ("privacy", ["python3", f"{X}/checks.py", "privacy", *MSGS]),
    ("notes", ["python3", f"{X}/checks.py", "notes", "{D}"]),
    ("files", ["python3", f"{X}/checks.py", "files"]),
    ("place", ["python3", f"{X}/checks.py", "place", "{D}"]),
    ("planfile", ["python3", f"{X}/checks.py", "planfile"]),
    ("guard", ["python3", f"{X}/checks.py", "guard"]),
    ("redate", ["bash", f"{X}/redate.sh", "--check", "{D}"]),
]
B = "`claude/loving-archimedes-frtf5q-60`"
END = "- `.github/workflows/ci.yml`'s browser job\n"
HEXWORD = r"(?<![0-9A-Za-z_])[0-9a-f]{7,40}(?![0-9A-Za-z_])"
CL_NOTE = ("**Recorded {D} (branch " + B + "):**\n"
           "Phase 6.0, the plan and Phase 6's scope (spec\n"
           "`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`;\n"
           "ADR 0062, Phase 6's scope; `docs/phase-6-plan.md`). Phase 6, \"1.0\n"
           "Release\", will release `1.0.0`, in place of the `v0.{phase}.0` milestone\n"
           "`nova-spec/00-MASTER-SPEC.md` §7 reserves for a phase, and its\n"
           "playground will compile Nova in the browser with the Rust `nova`'s own\n"
           "front end and WASM backend, built for WebAssembly. Nothing in `nova`\n"
           "changes yet.\n\n")
MS_NOTE = ("**Amended {D} (branch " + B + "):**\n"
           "Phase 6 follows `docs/phase-6-plan.md`, which the user approved on\n"
           "2026-11-26 and whose §1 gives the gate this list lacks, within the scope\n"
           "`docs/adr/0062-phase-6-scope.md` records. All seven items are in it: the\n"
           "in-repository part of each is built and gated, and the external parts\n"
           "(an audit firm, the TechEmpower pull request, the launch posts, a\n"
           "domain, switching GitHub Pages on) are the user's, recorded, and do not\n"
           "block 1.0. Item 5's playground is the Rust `nova`'s front end and WASM\n"
           "backend, built for WebAssembly and run in the browser, as a spike of\n"
           "2026-11-26 measured it; building the Nova compiler in `selfhost/` for\n"
           "WASM, which ADRs 0048 and 0061 mapped here, is on the backlog. Item 4's\n"
           "site is on GitHub Pages until there is a domain. The phase's release is\n"
           "`1.0.0`, in place of the `v0.6.0` that §7 would give it.\n\n")
A61_MARKER = ("  **Amended {D} (branch " + B + "):**\n"
              "  ADR 0062 makes Phase 6's playground S0 built for WASM, by the user's\n"
              "  decision of 2026-11-26, and puts N built for WASM on this backlog.\n")
REVIEW = ("prose the review holds: ADR 0062's Context is filled, not drafted (spec §3.2's outline), so no "
          "check holds its words, only its citations, hashes and private text; its research figures are the "
          "review's, against the plan's §2 (Review Focus 2)")
MUTANTS = [
    dict(key="H1", title="a commit not on `main`, #152's head before the rebase, named in ADR 0062", steps=[
        (ADR, "on `main` at `4fe11d55`", "on `main` at `574af54a`")], expect=["hashcheck"]),
    dict(key="P1", title="a `path:line` one line off", steps=[
        (ADR, "(`:99-102`) allows", "(`:100-102`) allows")], expect=["pathcheck"]),
    dict(key="P2", title="the plan's path in References misspelt", steps=[
        (ADR, "`docs/superpowers/plans/2026-11-27-phase-6-0-the-plan-and-adr-0062.md`",
         "`docs/superpowers/plans/2026-11-27-phase-6-0-the-plan-and-adr-62.md`")], expect=["pathcheck"]),
    dict(key="S1", title="a scratch path added to References", steps=[
        (ADR, END, END + "- The executor's notes: `{P}/exec60/notes.md`\n")],
         expect=["privacy"] + (["hashcheck"] if re.search(HEXWORD, P) else []),
         note="its hex parts, if the scratch directory's name has any, are hashcheck's too"),
    dict(key="S2", title="an email added to References", steps=[
        (ADR, END, END + "- Asked by: " + "release" + "@" + "example.org" + "\n")], expect=["privacy"]),
    dict(key="S3", title="a prototype branch named in References", steps=[
        (ADR, END, END + "- The prototype: `" + "pro" + "to/6-0`\n")], expect=["pathcheck", "privacy"],
         note="a branch's name is not a path in the tree either"),
    dict(key="N1", title="the CHANGELOG's note moved into `[0.5.0]`", steps=[
        ("CHANGELOG.md", "## [Unreleased]\n\n" + CL_NOTE, "## [Unreleased]\n\n"),
        ("CHANGELOG.md", "## [0.5.0] - 2026-11-24\n\n", "## [0.5.0] - 2026-11-24\n\n" + CL_NOTE)],
         expect=["notes", "place"]),
    dict(key="F1", title="a seventh file: the root README pointing to ADR 0062", steps=[
        ("README.md", "`selfhost/` holds N, Nova's compiler written in Nova. It is not shipped:\n",
         "ADR 0062 records Phase 6's scope.\n\n"
         "`selfhost/` holds N, Nova's compiler written in Nova. It is not shipped:\n")], expect=["files"]),
    dict(key="M1", title="the master spec's note left out", steps=[(MS, MS_NOTE, "")],
         expect=["files", "place", "redate"], note="the note was the file's only change, so files finds it unchanged"),
    dict(key="K1", title="ADR 0061's marker left out", steps=[(A61, A61_MARKER, "")],
         expect=["files", "place", "redate"], note="the marker was the file's only change"),
    dict(key="D1", title="a stale date: redate.sh to the next day, the master spec's note left at the old",
         steps=["redate", (MS, "**Amended {N} (branch " + B + "):**\nPhase 6 follows",
                           "**Amended {D} (branch " + B + "):**\nPhase 6 follows")], expect=["place", "redate"]),
    dict(key="D2", title="a history date moved: redate.sh to the next day, the user's decisions' day in "
         "ADR 0062's Context with it",
         steps=["redate", (ADR, "The user's decisions of 2026-11-26, labelled",
                           "The user's decisions of {N}, labelled")], expect=["redate"]),
    dict(key="L1", title="a line of the plan changed outside its 6.0 note", steps=[
        (PLAN, "# Phase 6 Plan — 1.0 Release\n", "# Phase 6 Plan — 1.0 release\n")], expect=["planfile"]),
    dict(key="G1", title="a figure in ADR 0062's table changed", steps=[
        (ADR, "| 830,149 bytes (2,937,593 raw) |", "| 830,194 bytes (2,937,593 raw) |")],
         expect=["place", "guard"]),
    dict(key="A1", title="a figure in ADR 0062's Context changed: 245 packages for 254", steps=[
        (ADR, "254 packages in `Cargo.lock`", "245 packages in `Cargo.lock`")], expect=[], accept=REVIEW),
]


def sh(cmd, log):
    t = time.time()
    r = subprocess.run(cmd, cwd=REPO, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=120)
    log.write(f"$ {' '.join(cmd)}\n{r.stdout}{r.stderr}\n[exit {r.returncode}, {time.time() - t:.1f} s]\n\n")
    return r.returncode


def clean():
    return subprocess.run(["git", "status", "--short"], cwd=REPO, capture_output=True,
                          text=True).stdout.strip() == ""


def day():
    m = re.search(r"^Accepted \((\d{4}-\d{2}-\d{2})\)\. Branch", open(os.path.join(REPO, ADR), encoding="utf-8").read(), re.M)
    return m.group(1) if m else "0000-00-00"


def observe(log):
    D = day()
    log.write(f"[D = {D}]\n")
    return [name for name, cmd in CHECKS if sh([c.replace("{D}", D) for c in cmd], log) != 0]


def main():
    os.makedirs(OUT, exist_ok=True)
    if os.listdir(OUT):
        raise SystemExit(f"{OUT} is not empty: move it aside to a fixed name first")
    if not clean():
        raise SystemExit("the tree is not clean")
    D = day()
    N = (datetime.date.fromisoformat(D) + datetime.timedelta(days=1)).isoformat()
    print(f"the records commit's day: {D}; the next day: {N}", flush=True)
    with open(f"{OUT}/0-baseline.log", "w", encoding="utf-8") as log:
        base = observe(log)
    print(f"baseline: {'every check passes' if not base else 'FAILS: ' + str(base)}", flush=True)
    if base:
        raise SystemExit(1)
    total = caught = expected = 0
    for k, m in enumerate(MUTANTS, 1):
        if not clean():
            raise SystemExit("the tree is not clean")
        t0 = time.time()
        log = open(f"{OUT}/{k}.log", "w", encoding="utf-8")
        try:
            for step in m["steps"]:
                if step == "redate":
                    sh(["bash", f"{X}/redate.sh", N], log)
                    continue
                f, old, new = step
                path = os.path.join(REPO, f)
                old = old.replace("{D}", D).replace("{N}", N).replace("{P}", P)
                new = new.replace("{D}", D).replace("{N}", N).replace("{P}", P)
                text = open(path, encoding="utf-8").read()
                if text.count(old) != 1:
                    raise SystemExit(f"{k} {m['key']}: its text occurs {text.count(old)} times in {f}")
                open(path, "w", encoding="utf-8").write(text.replace(old, new))
            seen = observe(log)
        finally:
            subprocess.run(["git", "checkout", "--", *TOUCH], cwd=REPO, check=True)
            log.close()
        if not clean():
            raise SystemExit(f"{k} {m['key']}: the tree is not clean after it")
        res = dict(id=k, key=m["key"], title=m["title"], observed=seen, expect=m["expect"],
                   caught=bool(seen), as_expected=sorted(seen) == sorted(m["expect"]),
                   accepted=m.get("accept", ""), seconds=round(time.time() - t0, 1))
        json.dump(res, open(f"{OUT}/{k}.json", "w"), indent=1)
        total += 1
        caught += res["caught"]
        expected += res["as_expected"]
        print(f"{k} {m['key']}: {'CAUGHT' if seen else 'NOT CAUGHT'} by {', '.join(seen) or 'nothing'}, "
              f"{'as expected' if res['as_expected'] else 'NOT AS EXPECTED'}, {res['seconds']} s", flush=True)
    print(f"mutants: {total}, caught {caught}, not caught {total - caught} (accepted), as expected {expected} of {total}")
    sys.exit(0 if expected == total else 1)


main()
EOF
```

  Then the disk check (Conventions), `Task 2 Step 3, before the mutants`,
  and the run, in the foreground, under `timeout` (the run takes under two
  minutes here; `timeout 280` stops it well inside the tool's 300000 ms):

  ```sh
  timeout 280 python3 "$P/exec60/mutrun.py" > "$P/exec60/mutrun.txt" 2>&1; echo "exit $?"
  cat "$P/exec60/mutrun.txt"
  git status --short
  ```

  Expected: `exit 0`; `baseline: every check passes`; one line per mutant,
  each `as expected`; the last line `mutants: 15, caught 14, not caught 1
  (accepted), as expected 15 of 15`; `git status --short` empty. A runner
  stopped by `timeout` exits 124, and one that finds a mutant not as
  expected exits 1: either stops the step. The mutants and what each must
  show, with the first line each failing check printed in the replay, where
  `D` was 2026-11-27 and `N` the day after:

  1. **H1: A commit not on `main`, #152's head before the rebase, named in ADR 0062.** ADR 0062's Context: `` on `main` at `4fe11d55` `` becomes `` on `main` at `574af54a` ``, `574af54a` being #152's head before the rebase, on no branch of `main`.
     - Expected: caught by `hashcheck` (`574af54a 1x, docs/adr/0062-phase-6-scope.md FAIL: commit 574af54adc70 is not on origin/main (if it …`).
  2. **P1: A `path:line` one line off.** ADR 0062's Context: `` (`:99-102`) allows `` becomes `` (`:100-102`) allows ``.
     - Expected: caught by `pathcheck` (`` FAIL ADR:24 `:99-102`: not on that line ``).
  3. **P2: The plan's path in References misspelt.** ADR 0062's References: this plan's path, `…-the-plan-and-adr-0062.md`, becomes `…-the-plan-and-adr-62.md`.
     - Expected: caught by `pathcheck` (`FAIL path docs/superpowers/plans/2026-11-27-phase-6-0-the-plan-and-adr-62.md: not in the tree (1x)`).
  4. **S1: A scratch path added to References.** ADR 0062's References gain a line naming `$P/exec60/notes.md`, with `$P` written out.
     - Expected: caught by `privacy` (`FAIL docs/adr/0062-phase-6-scope.md: a local or scratch path`); `hashcheck` (`<a word of $P> 1x, docs/adr/0062-phase-6-scope.md FAIL: neither a commit here nor a word this table …`). Its hex parts, if the scratch directory's name has any, are hashcheck's too.
  5. **S2: An email added to References.** ADR 0062's References gain a line holding an email address, which the runner builds.
     - Expected: caught by `privacy` (`FAIL docs/adr/0062-phase-6-scope.md: an email`).
  6. **S3: A prototype branch named in References.** ADR 0062's References gain a line naming a prototype's branch, which the runner builds.
     - Expected: caught by `pathcheck` (`FAIL path <the branch>: not in the tree (1x)`); `privacy` (`FAIL docs/adr/0062-phase-6-scope.md: a branch not the records'`). A branch's name is not a path in the tree either.
  7. **N1: The CHANGELOG's note moved into `[0.5.0]`.** `CHANGELOG.md`: the note (9 lines) and the blank line after it move from `[Unreleased]` to under `## [0.5.0] - 2026-11-24`.
     - Expected: caught by `notes` (`0.5.0: NOT the same as 4fe11d55's`); `place` (`FAIL the CHANGELOG note (spec 298-306): at [14]`).
  8. **F1: A seventh file: the root README pointing to ADR 0062.** The root `README.md` gains "ADR 0062 records Phase 6's scope." above `selfhost/`'s paragraph.
     - Expected: caught by `files` (`FAIL: README.md changed or untracked, not one of the six`).
  9. **M1: The master spec's note left out.** `nova-spec/00-MASTER-SPEC.md`: the note (13 lines) and the blank line after it go.
     - Expected: caught by `files` (`FAIL: nova-spec/00-MASTER-SPEC.md unchanged`); `place` (`FAIL the master spec note (spec 275-287): at []`); `redate` (`redate: the master spec's note (nova-spec/00-MASTER-SPEC.md): NOT FOUND`). The note was the file's only change, so files finds it unchanged.
  10. **K1: ADR 0061's marker left out.** `docs/adr/0061-phase-5-boundary.md`: the marker (3 lines) goes.
      - Expected: caught by `files` (`FAIL: docs/adr/0061-phase-5-boundary.md unchanged`); `place` (`FAIL ADR 0061's marker (spec 347-349): at []`); `redate` (`redate: ADR 0061's marker (docs/adr/0061-phase-5-boundary.md): NOT FOUND`). The marker was the file's only change.
  11. **D1: A stale date: redate.sh to the next day, the master spec's note left at the old.** `redate.sh N`, `N` the day after `D`, then the master spec's `**Amended N (branch` set back to `**Amended D (branch`.
      - Expected: caught by `place` (`FAIL the master spec note (spec 275-287): at []`); `redate` (`redate: the master spec's note (nova-spec/00-MASTER-SPEC.md): 2026-11-27`).
  12. **D2: A history date moved: redate.sh to the next day, the user's decisions' day in ADR 0062's Context with it.** `redate.sh N`, then ADR 0062's `The user's decisions of 2026-11-26, labelled` made `The user's decisions of N, labelled`.
      - Expected: caught by `redate` (`redate: history in docs/adr/0062-phase-6-scope.md: 2026-11-26 5, 2026-11-28 1, expected …`).
  13. **L1: A line of the plan changed outside its 6.0 note.** `docs/phase-6-plan.md`'s first line, `# Phase 6 Plan — 1.0 Release`, becomes `# Phase 6 Plan — 1.0 release`.
      - Expected: caught by `planfile` (`NOT the approved text: docs/phase-6-plan.md without lines 427-439, 44866 bytes, blob 56d5e2448e1e …`).
  14. **G1: A figure in ADR 0062's table changed.** ADR 0062's table: the release profile's `830,149 bytes` becomes `830,194 bytes`.
      - Expected: caught by `place` (`FAIL ADR 0062's figures (spec 208-243): at []`); `guard` (`DIFFERS the figures table: 15 lines, sha256 a5b93d0cee236e39, the split's 4f637b047de32625`).
  15. **A1: A figure in ADR 0062's Context changed: 245 packages for 254.** ADR 0062's Context: `` 254 packages in `Cargo.lock` `` becomes `` 245 packages in `Cargo.lock` ``.
      - Expected: not caught, every check passing; accepted as prose the review holds: ADR 0062's Context is filled, not drafted (spec §3.2's outline), so no check holds its words, only its citations, hashes and private text; its research figures are the review's, against the plan's §2 (Review Focus 2).

  If the run is cut off: `git checkout -- .`, `git status --short` empty,
  `rm -rf "${P:?}/exec60/mut-cut" && mv "$P/exec60/mut"
  "$P/exec60/mut-cut"`, then the run again from the first. All 15
  results come from one run, at one tip.
- [ ] **Step 4: The review's inputs.** Step 1's logs, Step 2's diffs and
  its checks, and the mutants, each against its Expected, go in the
  ledger.
- [ ] **Step 5: One fresh read-only review, then one fix pass** (the
  user's workflow, Q2).
  - The reviewer, a fresh read-only agent, reads `git diff $BASE..HEAD`
    (Task 0) and this plan's commit against the spec and its amendment,
    the spike's report and the user's decisions, which the main session
    hands it, and is told to hunt for:
    - a claim in ADR 0062 or a note that says more than the spike
      measured, or other than its report says (Review Focus 2): each
      figure against the report and the table spec §3.2 drafts; Chromium
      local, no 1.78 build, no `wasm-opt`, a worker's headroom estimated;
      ADR 0062's Context, filled, not drafted, against the plan's §2 and
      the master spec;
    - a hash not on `main`, a `path:line` that does not hold what the text
      says of it, a record of a pull request head or a pre-rebase commit
      (Review Focus 1);
    - private text: a token, a login, a local time or folder, a person's
      name, a model ID or a scratch path (Review Focus 4);
    - the date: the six places the commit's day, the history's days as
      they were (Review Focus 5);
    - `[Unreleased]` and `[0.5.0]` (Review Focus 6); each note at its
      place and in its form (Review Focus 7); the plan, the approved text
      and its note alone (Review Focus 3);
    - ADR 0062's decisions against the user's decisions (P1 to P12, the
      target, Q1, Q2, the ruling on decision 2) and the plan's §1, §3 and
      §4; its decision 2 against ADR 0061's backlog,
      `nova-spec/20-STDLIB.md` §1 and the lines its last two items cite; its
      Consequences against ADRs 0026, 0038 and 0048;
    - the mutants (Step 3): whether each is caught by what should catch
      it, and any breakage of the records the checks would not catch;
    - the plan's remaining steps: the PR, CI's Expected, the merge and
      Task 3.
  - Each finding is either fixed, or answered in the ledger with the
    reason it stands. **A finding against a part the user ruled on is not
    fixed here** (decision 30): the drafted blocks (`place`), the twelve
    decisions and the figures table (`guard`), the approved plan
    (`planfile`); the work stops, and the main session asks the user. The
    fixes are folded into the records commit before anything is pushed
    (decision 30), the files changed in the working tree first:
    - **on the commit's own day** (`TZ=UTC git log -1
      --date=format-local:%Y-%m-%d --format=%ad` prints today's `date -u
      +%F`): `git add -A`, `git status --short` listing only the changed
      files, then `git commit --amend --no-edit`, which keeps the date and
      the message;
    - **on another day:** the re-date of Task 1, Step 5 (decision 24),
      `redate.sh` with the new day, `git add -A`, `git status --short`
      listing the changed files and the dated ones, then `git commit
      --amend --date=now --no-edit`;
    - then Task 1, Step 5's checks again from its `D=` line: `dated D`,
      the trailers, and `CHK split "$D"`, which now prints `DIFFERS` for
      each file a fix changed, and only for those: the ledger names them;
    - then **the guard,** `CHK guard` (`same` twice), and `git diff
      <the records commit before the fold> HEAD` read whole: the fix and
      nothing else.
  - **A fix that adds or removes a line of ADR 0062 moves `pathcheck`'s
    keys.** Its table, `CITES`, is keyed by the ADR's line, so every
    citation after the fix's line gets a new key. Then, from the
    repository's root:

    ```sh
    CHK renumber
    CHK pathcheck > "$P/exec60/c-path.txt" 2>&1; echo "exit $?"
    ```

    `renumber` rewrites the keys in `$P/exec60/checks.py` from the ADR
    in the tree, in the citations' order, printing each key it moves, and
    `pathcheck` checks again that each row's lines still hold its words.
    If the fix added, removed or changed a citation, `renumber` refuses
    (`renumber: FAILED`), writing nothing: add, remove or change that row
    by hand, from the citation's file at the tip, then run both again.
  - **A fix that changes a date `redate.sh` counts, or a text a mutant
    edits,** is followed in the script by the edit it needs: the
    history's count, the mutant's `old` text. Each change to a script is
    recorded in the ledger as a ruling with its cost if wrong.
  - Then Steps 1 to 3 again, all of them: every check and every mutant,
    after `rm -rf "${P:?}/exec60/mut-before-fixes" && mv "$P/exec60/mut"
    "$P/exec60/mut-before-fixes"`. Expected: as written, but for what the
    ledger records. If no fix changed a file, Steps 1 to 3's results
    stand, and the ledger says so.
- [ ] **Step 6: The PR.**
  - **Stop here, and have the main session ask the user** whether to push
    the branch and open the PR (Global Constraints). No push of this
    branch has been allowed. Nothing is pushed, and no PR opened, before
    their word; record their answer in the ledger, in their words.
  - Before asking, check that every commit the PR would carry ends with
    the two trailer lines of the Conventions: `git log --format=%B
    origin/main..HEAD`, its commits `8709e8dc` (the spec's), this plan's,
    the approved plan's and the records commit, last.
  - On the user's word, the main session pushes the branch with git, `git
    push -u origin claude/loving-archimedes-frtf5q-60`, retried with
    backoff on a network error, as the session's rules say (no GitHub MCP
    tool pushes commits; a push refused by a permission or safety rule
    stops the work, and the user is asked), and records the pushed head.
    Then it opens the PR with the GitHub MCP tool `create_pull_request`, to
    `main`, not as a draft, titled **"Phase 6.0: the plan and ADR 0062"**,
    with the repository's template headings
    (`.github/PULL_REQUEST_TEMPLATE.md`: `## Summary`, `## Changes`, `##
    Testing`, `## Phase Gate Criteria`), and subscribes to its activity
    (`subscribe_pr_activity`). The body names no local path, no person
    and no model ID. It ends with this line, a blank line, and the
    session's address, exactly:

    ```
    🤖 Generated with [Claude Code](https://claude.com/claude-code)

    https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
    ```

    If the server replaces that ending with its own footer (it did on
    #149 to #152), send the body once more with the ending
    (`update_pull_request`), and check it.
  - The body holds:
    - **Summary:** Phase 6's first pull request, documentation only, as
      #128 carried Phase 5's plan: the Phase 6 plan, approved, byte for
      byte; ADR 0062, Phase 6's scope; dated notes under the master spec's
      Phase 6, atop `[Unreleased]` and after the plan's 6.0 entry, and
      markers in ADRs 0048 and 0061. No code, test, workflow, manifest,
      `Cargo.lock`, version or release change. Its commits: `8709e8dc`,
      the spec's; this plan's, with the spec's amendment; the approved
      plan's; the records commit, last (decision 20).
    - **Changes:** `docs/phase-6-plan.md` (the approved text's 750 lines,
      then the 6.0 note's 13); ADR 0062's parts (the Status; Context; twelve
      decisions, the playground as S0 in the browser and `1.0.0` the phase's
      release among them; the spike's figures, each within the limits the
      user ruled; Consequences; References); the master spec's note under
      Phase 6; the CHANGELOG's note in `[Unreleased]`; the markers in ADRs
      0048 and 0061; the date (decision 24); the records name `main`'s
      commits only.
    - **Testing,** the template's boxes, **ticking only what has run**
      (decision 29): at the opening, "CHANGELOG.md updated" ticked, and
      the five `cargo` boxes not, each "CI's run on this pull request
      runs it: no Rust, test or workflow file changes"; "New tests added"
      not ticked: none, as no test reads these files (spec §5), and the
      plan's checks hold them. After CI's run is green (Step 7), the five
      `cargo` boxes are ticked, each "CI's run <id> ran it", with the
      ending sent again. Then the plan's checks and their figures (Step
      1): `release-notes.sh 0.5.0`'s 627 lines unchanged and `Unreleased`
      printing the note; the approved plan's blob; the mutants, 15, 14
      caught and one accepted as prose the review holds; the review's
      findings and what came of each; the user's decisions and this
      plan's, numbered as here, decisions 20, 25 and 34 marked as
      differing and recorded in the spec's amendment.
    - **Phase Gate Criteria:** spec §12's gate, item by item, and which
      this pull request meets on its merge; `main`'s CI and the checks on
      `main` are Task 3's.
- [ ] **Step 7: CI.** **Wait for CI, then read it** (Conventions), with no
  foreground `sleep`: CI's results wake the session through the PR's
  subscription. When the CI run has ended, a read-only helper agent
  fetches each job's log whole with `get_job_logs` into `$P/exec60/ci/`
  and writes `$P/exec60/ci-<run>.txt` (Conventions). It writes nothing to
  GitHub. **Expected,** checked against that summary, from #152's run
  45829766657 (spec §4; decision 31):
  - **which workflows run:** CI alone, on the pull request
    (`.github/workflows/ci.yml:3-6`: `pull_request`, no path filter).
    **No Release run:** `release.yml`'s `pull_request` trigger names its
    paths (`.github/workflows/release.yml:15-27`: the workflow, the
    scripts, the build files, the manifests, the lock,
    `tools/vscode-nova/**`, `tools/install/**`), and none of the eight
    files is among them; a Release run on this pull request is a finding.
    No Benchmarks run (`.github/workflows/benchmarks.yml:3-5`: pushes to
    `main`); "Publish to crates.io" and "Release gate" run only when
    started (`workflow_dispatch`).
  - **every job green, CI's 19:** Test, Selfhost, Packaged nova,
    Installed nova and Clippy, each on three systems; Browser (Chromium),
    VS Code extension, MSRV (1.78) and Rustfmt;
  - **the Test job's main step,** its 111 `test result:` lines summed:
    `2838 passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9` on macOS,
    `2815, 0, 9` on Windows; in it `release_manifests` `8 passed`,
    `vscode_extension` `3 passed`, `project` `25 passed`; on Linux `test
    release_golden_suite ... ok` and no `tests/selfhost.rs`. The advisory
    `-- --ignored` step after it is not counted; what it shows is recorded
    (on #152's run, Linux's `extern_ffi_run`, issue #3's E0902 on `sqrt`,
    and three of macOS's collector tests failed there, as on #149 to
    #151);
  - **the Selfhost job:** `selfhost.rs` `71 passed; 0 failed; 3 ignored`
    on Linux, `69`, `3` on macOS, `65`, `3` on Windows, under the 80, 100
    and 110 s lines (#152's run: 55.92, 104.82 and 94.23 s, macOS's over
    its line, the runner's by the user's word); on Linux alone the golden
    step `2 passed; 0 failed; 0 ignored; 0 measured; 72 filtered out`,
    the bootstrap step (`.github/workflows/ci.yml:225-233`) printing
    `cc`'s and `ld`'s versions, `bootstrap: S1 == S2 == S3, …`, its
    runtime line under the gate's `home/runtime/0.5.0-<crc>/`, and `1
    passed; 0 failed; 0 ignored; 0 measured; 73 filtered out` (a `0
    passed` there means the step runs no test: stop and report), then
    `the_selfhost_tests_pass` `1 passed`; on macOS and Windows the clang
    steps skipped. Recorded, not expected: S1's size (9,539,848 bytes on
    #152's run, S1 == S2 == S3), the runtime's directory (`0.5.0-4f8a6f1b`
    there) and the step's time (27.37 s there, under its 75 s line);
  - **Packaged nova,** on each system: `package-gate: 20 crates at
    0.5.0`, `nova-cli-0.5.0.crate` with 1,759 files, the installed `nova`
    printing `nova 0.5.0` and `runtime: embedded`, and `package-gate:
    passed`. Recorded, not expected: the crates' bytes;
  - **MSRV (1.78):** `cargo check --locked --workspace` under `rustc
    1.78.0`, `Finished`;
  - Installed nova, Browser (Chromium), VS Code extension, Clippy and
    Rustfmt: green.

  Record each job's time, the run's address and the figures above in the
  ledger. **If a count differs,** or a job fails, record it, stop and ask
  the user; never lower a count. **If `selfhost.rs` takes more than 110 s
  on Windows, 100 s on macOS or 80 s on Linux, or the bootstrap step more
  than 75 s,** ask the user, with the merge question (spec §4: macOS's
  104.82 s on #152 was ruled the runner's): CI does not enforce the
  lines.
  - **After the push, nothing is amended or force-pushed without the
    user's word** (decision 30). If CI, the review or anything else needs
    a change to the pushed branch, the executor stops, records why, and
    the main session asks the user, offering decision 30's two ways. Each
    push after the first is on the user's word for it, and Steps 1 to 7
    run again on the new head as far as the change reaches.
- [ ] **Step 8: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit: `merge_pull_request` with
    `merge_method: rebase` and `expectedHeadSha` the head's full SHA, its 40
    hex digits, the last head pushed (Steps 6 and 7). Never auto-merge. Once
    it is merged, `unsubscribe_pr_activity` for the PR: its events no longer
    wake the session.
  - Then the main session runs `git fetch origin main`, `git diff <head>
    origin/main --stat`, which prints nothing: the merged tree is the
    PR's; and `git log -4 --format='%H %s' origin/main`, which prints,
    newest first, the records commit, the approved plan's, this plan's
    and the spec's, each with a new hash: the rebase gives each commit
    one. Record `main`'s four hashes in the ledger. None of them is in the
    records, which cite no commit of their own pull request (spec decision
    15).
  - **If `git fetch origin main` is refused** (by a permission or safety
    rule, or a network error after the retries), nothing works around it
    but this, read only, with the GitHub MCP tools: `list_commits`
    (`sha: main`, `perPage: 4`) gives `main`'s four new commits, newest
    first, the records commit's `sha` the tip; `get_commit` (`sha: main`)
    gives no tree (its output has the commit's `sha`, message, author,
    committer, stats and files, but no `tree`: 5.6c's fix pass checked a
    real call), so the tree is compared by the root's entries:
    `get_file_contents` (`path: /`, `sha:` the tip, `fields: [name, type,
    sha]`) lists each top-level entry with its hash, and each must equal
    `git ls-tree <the pushed head>`'s line for that name, the same names
    on both sides. Equal names, types and hashes are equal trees but for
    a top-level file's mode, which the API does not give. Record the
    refusal and the comparison in the ledger; `origin/main` stays
    `4fe11d55`, which Task 3 allows for.

---

## After the merge

### Task 3: `main`'s CI, the checks on `main`, and 6.0's gate

Spec §12; decision 32. The main session does each step itself, through its
GitHub MCP tools and git (no `gh`, no `curl`), and needs no word for its
reads and checks; what it records goes in the ledger. A run is waited for as
the Conventions give it, and its logs, if they are read, by the read-only
helper's summary.

- [ ] **Step 1: `main`'s CI on the merged commits.** `actions_list`
  (`method: list_workflow_runs`, `resource_id: ci.yml`,
  `workflow_runs_filter: {branch: main, event: push}`) finds the run whose
  `head_sha` is `main`'s records commit (Task 2, Step 8); one scheduled
  check-in (`send_later`) about 13 minutes on, then `actions_get` (`method:
  get_workflow_run`). **Accepted** when its `status` is `completed` and
  its `conclusion` `success`, all 19 jobs `success` (`list_workflow_jobs`),
  and its head commit's tree is the pull request's last head's: `git
  rev-parse <head_sha>^{tree}` and `git rev-parse <PR head>^{tree}` print
  the same (after Task 2, Step 8's fetch), or, if that fetch was refused,
  Task 2, Step 8's comparison of the root's entries, with `sha:` the
  run's `head_sha`, holds. Its counts are read, by the helper's summary,
  only if a job fails or the trees differ, and then the user is asked.
  The Benchmarks run on the same push is recorded with its conclusion.
  Record both runs' addresses.
- [ ] **Step 2: The checks on `main`'s tree.** On the branch, whose tree is
  `main`'s (Step 1), `origin/main` now the merged records commit, in a
  shell set up as the conventions say, `D` as Task 2 gives it. (If Task 2,
  Step 8's fetch was refused, `origin/main` is still `4fe11d55`: the first
  line below then prints nothing, and Step 1's comparison of the root's
  entries stands in for it; `hashcheck` still places `4fe11d55`, the one
  commit the records name.)

  ```sh
  git diff --quiet HEAD origin/main && echo "the same tree as origin/main"
  CHK hashcheck > "$P/exec60/m-hash.txt" 2>&1; echo "exit $?"
  CHK pathcheck > "$P/exec60/m-path.txt" 2>&1; echo "exit $?"
  CHK privacy > "$P/exec60/m-privacy.txt" 2>&1; echo "exit $?"
  CHK notes "$D" > "$P/exec60/m-notes.txt" 2>&1; echo "exit $?"
  CHK files > "$P/exec60/m-files.txt" 2>&1; echo "exit $?"
  CHK place "$D" > "$P/exec60/m-place.txt" 2>&1; echo "exit $?"
  CHK planfile > "$P/exec60/m-planfile.txt" 2>&1; echo "exit $?"
  CHK guard > "$P/exec60/m-guard.txt" 2>&1; echo "exit $?"
  tail -n 2 "$P"/exec60/m-*.txt
  git status --short
  ```

  Expected: `the same tree as origin/main` (the rebase gave the commits new
  hashes, and kept the tree). Then `exit 0` for each check, with Task 2,
  Step 1's figures, but `hashcheck` and `privacy` without the messages:
  `hashcheck` 1,049 lines read, `4fe11d55` 5 times, and `privacy` 1,049
  lines read (1,049 added lines, 0 message lines), with the same names built
  and the same allowed counts; `planfile` as Task 2, Step 1 gives it (it
  reads the branch's two commits; `main`'s are their rebased copies, the
  trees the same, Task 2, Step 8); `git status --short` empty. So every hash
  the records name is on `main`, ADR 0062's `path:line`s resolve with their
  words at the merged commit and the plan's to lines that exist and are not
  blank at `4fe11d55`, and `release-notes.sh` exits 0 there for `0.5.0` and
  for `Unreleased`. The spec's own citations and hashes are not checked
  again (decision 25).
- [ ] **Step 3: Spec §12's gate,** item by item against the ledger:
  1. the pull request merged by rebase on the user's word (Task 2, Step
     8);
  2. its CI green with §4's counts (Task 2, Step 7);
  3. `main`'s CI green on the merged commit (Step 1);
  4. the plan, ADR 0062 and §3.3 to §3.6's notes on `main` (Step 2's
     `files`, `place` and `planfile`);
  5. §5's checks 2, 3 and 7 holding there, as decision 25 makes them: the
     records' hashes on `main` (Step 2's `hashcheck`), ADR 0062's citations
     with their words and the plan's by line at `4fe11d55` (`pathcheck`),
     and the notes (`notes`); the spec's own citations and hashes, and the
     amendment's own 29 `path:line` citations (all holding), checked once
     by hand on 2026-11-27, are not checked again;
  6. the figures within the limits the user ruled (the plan's 6.0 gate;
     decision 18), as ADR 0062's table holds them (Step 2's `guard`):
     830,149 bytes gzipped at the release profile, under 5 MB; hello world
     compiled in 197 ms the first time in Chromium, under 2 s; every depth
     case at 128 survived.

  Each met, **6.0 is done.** The main session tells the user, and records
  it in the ledger with the date. 6.1, and 6.2 beside it, wait for the
  user's word; 6.1's spec asks for the CI target and the entry (ADR 0062's
  decision 12). Deleting branches (`-60` and the earlier ones) is the
  user's (spec §10).
