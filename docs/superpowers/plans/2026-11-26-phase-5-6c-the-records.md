# Phase 5.6c, "the records, ADR 0061", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
superpowers:subagent-driven-development (recommended) or
superpowers:executing-plans to implement this plan task-by-task. Steps use
checkbox (`- [ ]`) syntax for tracking.

**Goal:** Phase 5 is closed as `v0.5.0`, within a recorded boundary. One
pull request, the third of 5.6's three (5.6 decision 1), documentation
only, as #127 was for `v0.4.0`. Its last commit is the records commit,
`docs: Phase 5's records, ADR 0061`, which adds
`docs/adr/0061-phase-5-boundary.md` (Phase 5's inventory, its deviations
as their records decided them, "The gate's record" with every run of the
release and the user's try, the backlog, and what was found outside the
boundary) and five dated notes that point to it: `CHANGELOG.md`'s
`[0.5.0]`, with the publishes' run IDs; the master spec's Phase 5 gate;
`docs/phase-5-plan.md`'s 5.6; `selfhost/README.md`'s list of records; and
`docs/releasing.md`, step 6.2's last sentence changed in place and a note
in its Open VSX step (R7). No code, test, workflow, version or release
change. The records name `main`'s commits only. After the merge, on the
user's word: `main`'s CI, the checks again on `main`, and spec §12's gate;
then Phase 5 is closed, and Phase 6 waits for the user's word.

**Architecture:**
- **The records commit** (Task 1), in two patch blocks applied in order and
  committed once (decision 15): `t1-adr`, the new ADR (492 lines), then
  `t1-notes`, the five other files (67 lines added, 1 removed); then
  `redate.sh` gives the records the commit's day (decision 17).
- **Verification** (Task 2): the records' checks, one script with a
  subcommand for each of spec §5's checks, and `redate.sh --check`
  (decision 18); 16 mutants of the commit's files (decision 20); one fresh
  read-only review and one fix pass, folded into the records commit
  before any push (decision 23); the pull request and its CI (decision
  24); the merge.
- **After the merge** (Task 3): `main`'s CI, the checks again on `main`,
  and spec §12's gate (decision 25).

**Tech Stack:**
- Markdown alone. **No Rust, test, workflow, manifest or `Cargo.lock`
  change,** so no `cargo` command runs in this plan: no test reads these
  files (spec §5), and CI runs all 19 jobs on the pull request (spec §4,
  §6; decision 22).
- The checks are Python 3 and bash, run from the repository's root: git,
  and `.github/scripts/release-notes.sh`.

**Spec:**
`docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md`,
approved by the user on 2026-11-26 as drafted, with R7 (`5d57733d`), and
its dated amendment of 2026-11-26, which this plan's commit appends
(decision 29).
- Read its header, §1 to §12, and the amendment, before Task 1.
- The user's decisions are decisions 1 to 12 here, the approval decision
  13.

**Base:** `5d57733d` ("docs: the Phase 5.6c spec is approved"), `main`
`2842d63c` (the release commit, #151) and the spec's commit. The branch is
`claude/loving-archimedes-frtf5q-56b-records`, from `5d57733d`, then this
plan's commit (decision 29). `origin/main` is `2842d63c`; the clone is
shallow at `a71d27d9`.

**Checked on 2026-11-26, before execution:**
- **A prototype made the records commit whole,** on a local branch from
  `5d57733d`, never pushed (spec §6 and its decision 20: no draft
  pull request): the spec's drafted blocks copied by script, the outline
  filled. Its checks, spec §5's, all passed: 16 commit hashes, all on
  `main` (14 by `git merge-base`, `cc829f58` and `a3a95a2e` by GitHub's
  `list_commits`); 55 `path:line` citations, each resolving, the words on
  it; the six notes' blocks verbatim at their places; no private text;
  `release-notes.sh 0.5.0` 627 lines, the released notes' 614 with the
  dated note; exactly six files, 558 lines added and 1 removed. Where the
  spec's draft proved wrong, and where it was filled, is decision 27 and
  the amendment.
- **Two prototype fixes** (decision 14). First, ADR 0061's References name
  this plan, `docs/superpowers/plans/2026-11-26-phase-5-6c-the-records.md`,
  in ADR 0037's form, where the prototype named no path (the plan did not
  exist at its tip): three lines for two, at the file's end, so no citation
  moves. Then, after this plan's first fact-check, ADR 0061 and the
  runbook's Open VSX note say only what the release's ledger shows of Open
  VSX, and N's main thread is "1 MB on Windows", as ADR 0055 §12 says (the
  amendment, item 6): the ADR stays 492 lines and the note 11, so no
  citation of the runbook moves, and 18 of `pathcheck`'s keys move up one
  line, rewritten by `checks.py renumber`. The commit adds 559 lines and
  removes 1.
- **The split** (decisions 15, 16 and 21): a local branch from `5d57733d`
  with one commit, the records commit with this plan's message; its tree
  is the fixed prototype's (`git diff` between the two prints nothing).
  Its two blocks are the prototype's diff over their files, and each
  applies with plain `git apply`, in order, printing nothing, each applied
  tree the expected one; no added line holds trailing whitespace, a CR or
  a backslash-u escape. **The split is a local reference, not a commit
  the executor makes** or pushes: this plan names it by its six files'
  blobs, which `checks.py split` compares (Task 1, Step 4).
- **The replay** (decision 19): a rehearsal of the execution branch on no
  branch, as an executor would make it: `5d57733d`; this plan's commit (this
  plan as drafted, and the amendment); then Task 1 from this plan's own
  blocks and here-documents, each copied out by script, and Task 2's checks
  at its tip, dated 2026-11-26:
  - `hashcheck`: 30 words: 16 commits on `main` (14 by `git merge-base`, 2
    by GitHub's read), 9 run IDs and 5 other words (three sizes, two CRCs);
  - `pathcheck`: 55 `path:line` citations, 18 repository paths (this plan's
    among them), 0 Markdown links, 6 run URLs;
  - `privacy`: 573 lines read (the added lines and the message), 2 names
    built; allowed, the owner's login 8 times, in the repository's run URLs
    and the publisher id, and the records' branch 5 times;
  - `notes`: the released notes' 614 lines, 627 now: the note and a blank
    line inserted after line 45; `Unreleased` exits 1;
  - `files`: 6 files, 559 lines added and 1 removed;
  - `place`: 12 blocks, each once at its place; `redate.sh --check`: the six
    places dated, the history as written;
  - `split`: 6 of 6 files the split's; and the tree outside
    `docs/superpowers` the split's.
- **The mutants** (decision 20): 16 breakages at the rehearsal's tip, in one
  run of about 100 s: 15 caught, each by exactly the checks it should be,
  and 1, A1, accepted; 16 of 16 as expected (Task 2, Step 3).
- **Another day** (decision 17): on the rehearsal's tree, `redate.sh
  2026-11-27` changed 6 lines in 5 files, and every dated check passed with
  that day (`place`, `notes`, `split` and `redate.sh --check`); `redate.sh
  2026-11-26` then gave the tree back, `git status --short` empty.
  `redate.sh` refused `26-11-27` and `2026-02-30`.
- **The GitHub reads** (read only, 2026-11-26): `list_commits` from
  `a71d27d9` on `main` lists `a3a95a2e` (its 7th) and `cc829f58` (its
  21st), so both are on `main` (decision 18); #127's body, for the records
  pull request's form.

## Global Constraints

- **Documentation only.** The records commit changes exactly six files,
  `CHANGELOG.md`, `docs/adr/0061-phase-5-boundary.md` (new),
  `docs/phase-5-plan.md`, `docs/releasing.md`,
  `nova-spec/00-MASTER-SPEC.md` and `selfhost/README.md`, and the pull
  request adds this plan and the spec's amendment: eight files. Nothing
  under `crates`, `std`, `selfhost/src`, `tests`, `.github` or `tools`,
  no manifest and no `Cargo.lock` (spec §5, check 6). **Any other change
  stops the task;** ask the user.
- **No `cargo` command runs here.** Nothing is built: no test reads these
  files (spec §5), and CI runs all 19 jobs on the pull request and on
  `main` after it (decision 22). So no build set in `target/` is used or
  changed, and the Rust 1.78 and 1.100 checks are CI's.
- **The records' rule** (spec header; its decisions 7 and 8): every record
  names `main`'s commits only, by 8 characters; pull request numbers are
  allowed; no `proto/`, `split/` or `-ci` branch, no pull request head or
  pre-rebase hash, no local or scratch path. `checks.py hashcheck` and
  `privacy` hold it (decision 18).
- **The shallow clone.** The clone is shallow at `a71d27d9`, so `git
  merge-base` cannot place a commit older than it. The two the records
  name, `cc829f58` and `a3a95a2e`, are on `main` by GitHub's
  `list_commits` (Checked on), and `hashcheck` holds them by name; any
  other such hash fails it, and the main session checks it with its GitHub
  MCP tools before it is kept. `git fetch --unshallow` needs the user's word
  (spec §9, risk 2). The executor runs no `git fetch`; the main session
  fetches `origin main`, with git, after the merge (Task 2, Step 8). If
  that fetch is refused, Task 2, Step 8's fallback reads `main` with the
  GitHub MCP tools; nothing else works around the refusal.
- **Never clean `/tmp/nova-*`.** Nothing in this plan writes there.
- **Keep `df -m /` above 6,144 MiB.** Nothing here writes more than a few
  MiB, all of it under `$P/exec56c/`. The disk check (Conventions) runs at
  Task 0 and before the mutants; if it reads under the threshold, stop and
  report: nothing is removed to make room.
- **Never use the Write or Edit tool for text that holds a backslash-u
  escape,** a backslash and `u` followed by four hex digits or by a code
  in braces. None of 5.6c's blocks or scripts holds one. Apply the blocks
  with `patch_block`, from the committed plan.
- **No block adds a CR, a NUL or trailing whitespace.** The non-ASCII
  characters the blocks add are `§`, `—`, `…`, `×` and `→`, and the user's
  words in Thai, quoted in "The gate's record" (13 characters). `t1-notes`
  holds 10 blank context lines, each a single space, which `git apply`
  needs: `git diff --check` on this plan's file reports them, and the
  records' own lines are clean (Task 2, Step 2).
- **Nothing writes into the repository but the blocks, `redate.sh` and the
  review's fixes.** `git status --short` is empty after every check and
  after the mutants.
- **Push and open the pull request on the user's word only.** No push of
  this branch is allowed yet; Task 2, Step 6 stops before any push, and
  the main session asks the user. **After the push, nothing is amended or
  force-pushed without the user's word** (decision 23): a change CI or a
  review asks for then stops the work, and the main session asks the
  user.
- **Merge by rebase, on the user's word only,** pinned to the head. Never
  enable auto-merge.
- **The main session, not a subagent, does every GitHub action.** It
  pushes with git, `git push -u origin
  claude/loving-archimedes-frtf5q-56b-records`, retried with backoff on a
  network error, as the session's rules say, and fetches with git (`git
  fetch origin main`, Task 2, Step 8); no GitHub MCP tool pushes commits.
  Everything else goes through its GitHub MCP tools: the pull request
  (`create_pull_request`), the merge, every read of CI and of `main`. The
  session has no `gh` and runs no `curl`. A push refused by a permission
  or safety rule stops the work, and the user is asked; a refused fetch
  has the one fallback Task 2, Step 8 gives. A helper agent that reads
  CI's logs is read-only and writes nothing to GitHub.
- **No secret is read, printed or stored,** and no token's value goes in a
  record (R3).
- **This machine is Linux;** macOS and Windows results come from CI.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Every shell exports it, `export P=…` with the
  directory's path, before a command that names `$P`. Exported, not
  merely set: the mutant runner reads it from the environment.
- **The executor's directory:** `$P/exec56c/`. Every script this plan
  writes (`redate.sh`, `checks.py`, `mutrun.py`), the commit's message,
  every output and every log goes there, the disk checks' `logs/df.log`
  and the mutants' `mut/` among them. Two files are kept outside it: the
  ledger, `$P/ledger-56c.md`, and the env file, `$P/env56c-exec.sh`.
- **The ledger:** `$P/ledger-56c.md`. Every ruling made while executing
  goes there, with its cost if wrong; so do each task's figures, and each
  of the user's words in Task 2 and Task 3, in their words.
- **The env file:** `$P/env56c-exec.sh`, sourced at the start of every
  tool call (`source "$P/env56c-exec.sh"`): each tool call is a fresh
  shell, so a variable a step sets is set again in each call. Its text,
  with `P` the scratch directory's path:

  ```sh
  # Phase 5.6c execution shell setup: the plan's Conventions (root, cgroup v1), with PLAN the committed plan.
  P=/path/to/the/scratch/directory; export P; cd /path/to/the/repository
  CG=/sys/fs/cgroup/pids/nova-limit; mkdir -p "$CG" && echo 600 > "$CG/pids.max" && echo $$ > "$CG/cgroup.procs"
  grep -q nova-limit /proc/self/cgroup || { echo "NOT IN nova-limit: stop"; return 1 2>/dev/null || exit 1; }
  mkdir -p "$P/exec56c/logs"
  export NO_COLOR=1 PYTHONUTF8=1 GIT_PAGER=cat
  PLAN=docs/superpowers/plans/2026-11-26-phase-5-6c-the-records.md
  CHK() { python3 "$P/exec56c/checks.py" "$@"; }
  block() {
    awk -v name="$1" '
      $0 == "<!-- patch " name " -->" { grab = 1; next }
      grab && /^````diff$/ { inside = 1; next }
      inside && /^````$/ { exit }
      inside { print }
    ' "$PLAN" > "$P/exec56c/$1.patch"
  }
  patch_block() {
    block "$1" && git apply --check "$P/exec56c/$1.patch" && git apply "$P/exec56c/$1.patch"
  }
  ```

  So `CHK NAME` runs one of the records' checks (Task 1, Step 2), and
  `patch_block NAME` writes a block of this plan to
  `$P/exec56c/NAME.patch` and applies it with plain `git apply`.
  `PYTHONUTF8=1` keeps the scripts' output in UTF-8 whatever the locale.
- **The process limit.** Every shell first caps the tasks it may start at
  600, threads as well as processes, and checks that the cap holds: the
  env file joins the pids control group `nova-limit` (root, cgroup v1, as
  5.6b's did), and stops if the shell is not in it. The 5.6a plan's
  Conventions
  (`docs/superpowers/plans/2026-11-22-phase-5-6a-the-bootstrap-gate.md`)
  give the forms for cgroup v2 and for a user other than root.
- **The disk check,** logged, its `echo` naming the task, the step and
  what comes next:

  ```sh
  { echo "Task 2 Step 3, before the mutants"; df -m / | tail -1; } >> "$P/exec56c/logs/df.log"
  tail -n 1 "$P/exec56c/logs/df.log"    # its fourth field: the MiB free, at least 6,144
  ```

- **Commits.** The records commit's message is written to
  `$P/exec56c/msg-t1.txt` by its here-document (Task 1, Step 2): the
  subject, a body, a blank line, and these two trailer lines, exactly:

  ```
  Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
  Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
  ```

  Then `git add -A`; `git status --short` lists only the task's files;
  and `git commit -F "$P/exec56c/msg-t1.txt"`. Every commit of this plan
  ends with those two lines: the plan's own and the records commit, into
  which the review's fixes are folded (decision 23). The pull request's
  body ends as Task 2, Step 6 gives it. No model ID (a `claude-…` model
  name) goes in any artifact: no commit, file, pull request, comment or
  ledger line names one.
- **Timeouts, and nothing in the background.**
  - **A tool's timeout is in milliseconds:** at least the command's
    stated time plus half again, never under 120000 (120 s) and at most
    600000 (600 s); 300000 for the mutants (Task 2, Step 3).
  - Run every command in the foreground: never with `&`, `nohup` or a
    tool's background mode. A block of several commands runs one command
    at a time. Long runs write their output to a log under `$P/exec56c/`,
    which is read after.
  - If a command is cut off, restore what it left: after a cut-off mutant
    run, `git checkout -- .`, then `git status --short` must print
    nothing, then `mut/` is moved aside and the run starts again from the
    first (Task 2, Step 3). `git apply` applies a block whole or not at
    all: if `t1-notes` does not apply after `t1-adr` did, `git apply -R
    "$P/exec56c/t1-adr.patch"` takes the new file away, `git status
    --short` must print nothing, and the task starts again from Step 1.
- **CI's runs: waiting for them and reading them** (Task 2, Step 7; Task
  3). The main session never waits with a foreground `sleep`.
  - **Waiting.** For the pull request, the main session subscribes to its
    activity (`subscribe_pr_activity`) once it is open, and CI's results
    wake the session. For `main`'s run after the merge, it arms one
    scheduled check-in in this session (`send_later`) for about the run's
    time (#151's took 12 min, `main`'s 11 min), reads the run with
    `actions_get` (`method: get_workflow_run`) when it fires, and arms
    another if its `status` is not `completed`; or the user says when it
    is done.
  - **Reading the logs.** The main session does not pull whole job logs
    into its own context. A read-only helper agent fetches each job log a
    check needs with `get_job_logs`, whole (`return_content: true`,
    `tail_lines` 100000; 5.6b's ruling R9.2), into `$P/exec56c/ci/`, and
    writes a summary, `$P/exec56c/ci-<run>.txt`, `<run>` the run's id:
    each job's conclusion, the `test result:` counts summed for each Test
    job, the Selfhost job's lines, the gate's `bootstrap:` lines, and the
    step lines Task 2, Step 7's Expected names. It writes nothing to
    GitHub. A check that reads a log is made against that summary.
- **The patches.** Task 1's two blocks below are each marked `<!-- patch
  NAME -->` and fenced with four backticks. They are the prototype's diff,
  so apply them; do not retype them. `patch_block t1-adr` (the env file
  defines it, with `PLAN` the committed plan) writes the block to
  `$P/exec56c/t1-adr.patch` and applies it with plain `git apply`, which
  prints nothing. A block that does not apply means the tree differs from
  the plan's: stop, and record why in the ledger.
- **The scripts.** `redate.sh`, `checks.py` and `mutrun.py`, and the
  commit's message, are written by here-documents at column 0, each in
  its own fenced block, to `$P/exec56c/`: copy each block out by script,
  its line range checked at both ends (5.6b's ruling R1.1), or run it as
  it stands; never retype it.

## Review Focus

The inputs most likely to bite a reader of the records, most likely first.
Each has its check in the task that owns it:

1. **A record that names a commit not on `main`, or a `path:line` that
   moved** (spec §9, risks 1 and 2; the records' rule): → `CHK hashcheck`
   and `CHK pathcheck` (Task 1, Step 3; Task 2, Step 1; again on `main`,
   Task 3, Step 2); mutants H1, P1 and P2.
2. **Saying more than was checked** (spec §9, risk 3): run 45352790982's
   counts were not read; the listings and the index are the user's
   screenshots; the digests were compared through the API: → the review
   (Task 2, Step 5) reads "The gate's record" against the release's ledger
   and spec §3.1. No script can tell a true claim from a false one
   (mutant A1).
3. **Private text** (spec §9, risk 4: a token, a login, the user's local
   time or folders): → `CHK privacy`, its names built from git and never
   written (decision 18), and the review; mutants S1 to S3.
4. **The date** (spec §9, risk 6): the commit's day in exactly its six
   places, the history's days untouched: → `redate.sh` and its `--check`
   (decision 17); mutants N3, D1 and D2.
5. **`release-notes.sh` still finds `[0.5.0]`, and the published notes are
   not changed:** `[0.5.0]` differs from them by the dated note alone: →
   `CHK notes`; mutants N1 and N2.
6. **Each note at its place** (R9.1's lesson, spec §5 check 1): → `CHK
   place`; mutants R7, M1 and M2.
7. **CI green with §4's counts:** → Task 2, Step 7's Expected, from #151's
   run 45345843046; Windows' `selfhost.rs` near its 110 s line (spec §9,
   risk 5) goes to the user if over.
8. **The merge, and Phase 5 closed, each on the user's word:** → Task 2,
   Steps 6 and 8, and Task 3.

## Decisions

The user's, from the spec's header and §11, each option's name as given,
in Thai, ending "(แนะนำ)", "recommended"; then what was chosen.

Phase 5.6's (2026-11-22):

1. **5.6 is three pull requests** (5.6's decision 1, "5.6a gate → release
   → บันทึก (แนะนำ)": the third is the boundary ADR and the records closing
   Phase 5, after the release). This plan is the third's.
2. **The gate on Linux alone** (5.6's decision 6, "Linux อย่างเดียว
   (แนะนำ)": macOS and Windows in the boundary ADR's backlog). ADR 0061's
   deviations and backlog (`t1-adr`).
3. **`selfhost/` not shipped** (5.6's decision 8, "ไม่ส่ง selfhost/ บอกใน
   notes (แนะนำ)": the archive unchanged). The inventory's "built" rows,
   not "shipped" (`t1-adr`; spec decision 10).

5.6b's (2026-11-23):

4. **Every publish, as for `v0.4.0`** (5.6b's decision 5, "ครบทุกขั้นแบบ
   v0.4.0 (แนะนำ)": step 6.2 replaced by the user trying the published
   `nova`). "The gate's record" records each publish and the try.
5. **The records after every publish and the try, in one pull request,
   as #127** (5.6b's decision 7, "หลัง publish ครบและคุณลองแล้ว (แนะนำ)":
   ADR 0061 records every release run, the user's check, Phase 5's
   inventory and backlog). This pull request.

The records' (2026-11-26):

6. **R1,** "บันทึกใน ADR 0061 + backlog (แนะนำ)": ADR 0055's thread and ADR
   0053's E0093 `/`, not done, recorded with the depth N survives, and put
   on the backlog; documentation only. The inventory's two "not done"
   rows, the backlog and "Found outside this boundary".
7. **R2,** "cargo install nova-cli --locked": the try, on Windows; `nova
   0.5.0`; `Hello, Nova!`; the user's words in Thai, glossed. "The gate's
   record" and the runbook's `:537` sentence.
8. **R3,** "บันทึกสั้น ๆ ทั้งสองเรื่อง (แนะนำ)": Open VSX's token, shown and
   revoked, and 0.5.0 inactive for a while, as lessons; no token value.
   "What went otherwise than the runbook says", which says only what the
   release's ledger shows: not when 0.5.0 reached Open VSX, nor why it was
   not yet active (the spec's "until Open VSX's check" said more: the
   amendment, item 6).
9. **R4,** "Dated note ใต้ gate ของ Phase 5 (แนะนำ)": the master spec's
   note, after its Phase 5 gate (`t1-notes`).
10. **R5,** "เฉพาะที่ master spec ระบุ (แนะนำ)": Phase 6 gets N for WASM
    alone; the rest is a backlog that promises nothing.
11. **R6,** "selfhost/README + releasing.md (แนะนำ)": notes there, with
    the CHANGELOG's and the phase plan's; none in the root README (mutant
    F1 shows `files` refusing one).
12. **R7,** "เพิ่ม note ใน runbook เลย (แนะนำ)": R3's two lessons as a
    dated note in the runbook's Open VSX step, not on the backlog
    (`t1-notes`; mutant R7), worded as the amendment's item 6 gives it.

The user's answer of 2026-11-26 on the spec:

13. **The spec, approved as drafted** ("อนุมัติตามร่าง (แนะนำ)"), then R7
    drafted into §3.5, and its status line saying so. Committed as
    `5d57733d` on this branch, from `2842d63c`.

How this plan is cut and checked:

14. **Two prototype fixes, each followed by the split made again.**
    - **References name this plan.** Spec §3.1 gives References "this
      spec and plan". The prototype named the spec's path and "its plan"
      without a path, since the plan did not exist at its tip. The real
      cut has this plan in the commit before the records commit, so the
      fixed prototype names it in ADR 0037's form
      (`docs/adr/0037-phase-3-boundary.md`'s References: "and its plan,"
      then the path), three lines for two. They are the file's last
      section, so no line the ADR or anything else cites moves;
      `pathcheck` holds the path (mutant P2).
    - **Only what the sources show** (this plan's first fact-check). The
      release's ledger shows Open VSX's first try uploading nothing, the
      second answering "… is already published, but currently isn't active
      and therefore not visible", no retry, and the listing then showing
      0.5.0 as `latest`; it does not show when 0.5.0 reached Open VSX, why
      it was not yet active, nor which copy of the `.vsix` the user
      uploaded. So ADR 0061's lesson, its R3 ("0.5.0 inactive for a while",
      the user's words) and its extension bullet say only that, and the
      runbook's note reads: "Open VSX may show a new version as already
      published but not yet active, for a while after its upload; `ovsx
      publish` then answers 'already published, but currently isn't active':
      wait for the listing, and do not publish again", with the file named,
      `npx ovsx publish .\nova-vscode-<version>.vsix` in the `.vsix`'s
      folder. And N runs on its main thread, "1 MB on Windows", as ADR 0055
      §12 (`:495`) says, not on "its 1 MB main thread"; ADR 0055 measured
      the depth on Linux under `ulimit -s 1024`. The ADR stays 492 lines and
      the note 11, so no citation of the runbook moves; 18 of `pathcheck`'s
      keys, ADR lines 329 to 412, move up one (`checks.py renumber`).
      `place` holds the note as the amendment's item 6 gives it.
15. **Two patch blocks, one commit** (spec §7 and its decision 19: the ADR
    and the five notes in one commit, as `cc829f58`). `t1-adr` is the new
    file alone (498 lines); `t1-notes` the five edited files (130 lines),
    each the prototype's diff over its files, applied in order, each whole,
    by plain `git apply`, then one commit. Two blocks, not one: a new file
    applies anywhere, while the notes' hunks need the base's lines around
    them, so a tree that differs from the base fails on `t1-notes` alone and
    names where; and each has its own check (Task 1, Step 1: the new file's
    length, the five files' numstat). Not one block in parts by `git apply
    --include`: an `--include` whose pattern matches no file applies nothing
    and prints nothing (5.6b's decision 13), while a whole block that does
    not apply fails loudly. Not six blocks: the notes are one change,
    pointing to one ADR.
16. **The commit's subject is spec §7's,** `docs: Phase 5's records, ADR
    0061`, in `cc829f58`'s form ("docs: v0.4.0's records"); the body is the
    prototype's, which names the release's runs, as `cc829f58`'s did. The
    prototype's own subject was a prototype's. The message carries no
    date, so the commit's day is in the records alone (decision 17).
17. **The date: the commit's day, by script.** The blocks date the records
    2026-11-26, the prototype's day, in exactly six places: ADR 0061's
    Status ("Accepted (2026-11-26).") and its Consequences ("with its note
    of 2026-11-26;"), the three notes "**Recorded 2026-11-26 (" (the
    CHANGELOG, the master spec, the phase plan) and the runbook's Open VSX
    note "**Amended 2026-11-26 (". Spec §3.5 (the note "dated by the records
    commit's day"), its §9, risk 6, and its decision 21 give them the
    records commit's day. Every other date in the added lines is history,
    and stays as written, whatever day the commit lands: the earlier
    rulings' and pull requests' days, the release's 2026-11-24 and
    2026-11-25, and the user's decisions of 2026-11-26 and the try,
    2026-11-26 three times in the ADR (the Status's "the user's of
    2026-11-26", R1's day and the try) and once in the runbook's `:537`
    sentence, the try's (spec §3.5: "with the try's own date, so the note's
    date stays true"). So Task 1, Step 3 runs `redate.sh "$(date -u +%F)"`
    before the commit, whatever the day: it finds each place by the words
    around it and rewrites it whatever date it holds, then checks, against
    `2842d63c`, that each place holds the day, that no removed line holds a
    date, and that the history's dates in the added lines are exactly as
    written (file names' dates are names). `redate.sh --check DAY` checks
    without writing; the mutants and Task 2 use it. The day is UTC's, as the
    commit's own date is. **A commit made is re-dated one way,** if the day
    turned between Step 3 and the commit or the review's fixes are folded on
    another day: `redate.sh` with the new day, the checks with it, `git add
    -A`, then `git commit --amend --date=now --no-edit` (the message has no
    date to change). A fold on the commit's own day is `git commit --amend
    --no-edit`.
18. **The checks, spec §5's six, as one script** (`checks.py`, Task 1,
    Step 2), a subcommand for each, each printing what it found and a last
    line `NAME: ok` or `NAME: FAILED`, and exiting 0 or 1; and `renumber`,
    for `pathcheck`'s table (below):
    - **They read the working tree against `2842d63c`,** the release
      commit on `main`, whose six files the records change: on a clean
      tree at the records commit that is the commit, and an edit is seen
      before it is committed, which is what lets the mutants (decision 20)
      and a check before the commit (Task 1, Step 3) work. "Added lines"
      are those `git diff 2842d63c` adds outside `docs/superpowers`, and
      the lines of a file not yet added.
    - **`hashcheck`** (spec §5, check 3): every word of 7 to 40 hex digits
      in the added lines and the message is a commit on `main` by 8
      characters (spec decision 8), by `git merge-base --is-ancestor`
      against `origin/main`, or `cc829f58` or `a3a95a2e`, held by name,
      older than the shallow boundary and on `main` by GitHub's
      `list_commits` (Checked on); or a word its table classifies: the
      nine run IDs, three sizes and two runtime directories' CRCs the
      records name. Anything else fails, a decimal word too, so a new
      figure is classified before it passes.
    - **`pathcheck`** (check 2): the 55 backticked `path:line` citations
      of ADR 0061, each in its table with the ADR's line, the cited file
      (an ADR by its number) and the words its lines must hold; a citation
      the table lacks fails, as does one off its line. Then every
      backticked relative repository path in the added lines exists, every
      Markdown link resolves (there are none), and every URL is a run of
      this repository's that `hashcheck`'s table names. **The table,
      `CITES`, is keyed by ADR 0061's line,** so an edit that adds or
      removes a line above a citation moves every key after it:
      `renumber` rewrites the keys in the script from the ADR in the tree,
      in the citations' order, and refuses, writing nothing, if a citation
      was added, removed or changed (that row is changed by hand); then
      `pathcheck` checks each row's words again (Task 2, Step 5).
    - **`privacy`** (check 4): no local or scratch path, branch but
      `claude/loving-archimedes-frtf5q-56b-records`, person's name or
      email, model identifier, session link or token, in the added lines
      or the message, its two trailer lines excepted. The names are built
      when it runs, from the remote's owner, the authors and committers on
      `main` and the git config, and never written in this plan; the
      owner's login is allowed only in the repository's run URLs and the
      publisher id `….vscode-nova`, the records' own (spec §5, check 4).
    - **`notes D`** (check 5): `release-notes.sh 0.5.0 CHANGELOG.md` exits
      0 and prints the released notes, `2842d63c`'s, with spec §3.2's
      note, dated `D`, and one blank line inserted, nothing else;
      `release-notes.sh Unreleased CHANGELOG.md` exits 1.
    - **`files`** (check 6): exactly the six files differ from `2842d63c`
      outside `docs/superpowers`, and no other file is untracked.
    - **`place D`** (check 1, R9.1's lesson): each block spec §3 drafts,
      dated `D` where the commit's day goes, verbatim, once, at the place
      §3.2 to §3.5 name, by the lines around it, and ADR 0061's Status,
      legend, sample rows and status words as §3.1 drafts them; the
      runbook's Open VSX note as the amendment's item 6 gives it, which
      the script holds.
    - **`split D`:** the six files, `D` put back to 2026-11-26, hash to
      the split's blobs (decision 21).
19. **The replay is a rehearsal of the execution branch, on no branch.**
    `pathcheck` needs this plan's path, which only the plan's commit adds,
    so the checks were replayed where an executor runs them: a detached
    checkout of `5d57733d`, this plan's commit made there (the plan as
    drafted, the amendment appended), then Task 1 from this plan's blocks
    and here-documents, copied out by script, then Task 2's checks and the
    mutants at its tip; then back to the split. No branch was made or
    moved by it, as 5.6b's "another day" commit was made (5.6b's plan,
    Checked on).
20. **The mutants** (Task 2, Step 3): 16 breakages of the records
    commit's files, each with the checks that must catch it, all run in
    one run at one tip by the runner, which restores each file and checks
    `git status --short` before and after. 15 are caught, each by exactly
    the checks it should be; one, A1 (the parser's inventory row's status
    changed), is accepted. `place` holds three of the 42 inventory rows
    verbatim, spec §3.1's sample rows (the lexer's, the backend bindings'
    and the gate's), so a status changed in one of them is caught; the
    other 39 rows' statuses, the parser's among them, are held only by
    the review (Review Focus 2). Among the caught: the R7 note left out,
    caught by `place`, by `pathcheck` (ADR 0061's `docs/releasing.md:750-753`
    moves) and by `redate.sh --check` (one place gone); the master spec's
    note left out, by the same three and by `files`.
21. **The split named by its blobs, not by a local commit.** The prototype
    and the split are local branches, never pushed (spec decision 20), so
    a commit hash of theirs would resolve for no one but this clone, and
    the records may name none (decision 7 of the spec). `checks.py split
    D` hashes each of the six files, `D` put back to 2026-11-26, and
    compares the split's blobs, which it holds: no other text in those
    files holds a day after 2026-11-26. After the review's fixes, the
    files they change differ, and the ledger says which.
22. **No `cargo` command, and the PR template's boxes.** No Rust, test or
    workflow file changes, and no test reads these files (spec §5), so
    nothing local is built or run; CI's run on the pull request runs the
    suite, `selfhost.rs` and clippy on three systems, `cargo fmt` and the
    MSRV check (Task 2, Step 7). The body ticks each box with that said, as
    #127's body did for `v0.4.0`'s records, and "New tests added" is not
    ticked, with the reason.
23. **The review's fixes go into the records commit; after the push,
    nothing changes without the user's word.** Spec §7 has one records
    commit, last. So the one fix pass (Task 2, Step 5) folds its changes
    into the records commit before anything is pushed (decision 17's
    amend). **After the push, nothing is amended or force-pushed without
    the user's word.** If CI or a review needs a change then, the
    executor stops, and the main session asks the user, offering two
    ways: amend the records commit and force-push with lease, pinned to
    the pushed head, with git (`git push
    --force-with-lease=<branch>:<pushed head> origin <branch>`, `<branch>`
    this plan's branch), which keeps one records commit, as spec §7 has
    it; or a new commit after it,
    which the records' history then shows. Unlike 5.6b's, no tag or
    release reads this branch's tree, so either way reaches everything.
24. **CI's lines and counts for the pull request** (spec §4): every one
    of CI's 19 jobs green, with the counts of #151's run 45345843046 (Task
    2, Step 7); no Release run, since `release.yml` runs on a pull request
    only for the paths its `pull_request` trigger names
    (`.github/workflows/release.yml:15-27`), which hold none of the eight
    files; no Benchmarks run before the merge (`benchmarks.yml` runs on
    pushes to `main`, `.github/workflows/benchmarks.yml:3-5`). **Recorded,
    not expected:** sizes, the runtime's directory, the crates' bytes,
    every time, and the advisory step's known failures. Past a time line
    (80, 100 and 110 s for `selfhost.rs`, 75 s for the bootstrap step) the
    user is asked.
25. **The merge and Task 3.** By rebase, on the user's word, pinned to the
    head (Task 2, Step 8). Then `main`'s CI run is accepted by its
    `conclusion` and its head commit's tree being the pull request's last
    head's, as 5.6a and 5.6b accepted theirs; its counts are read only if a
    job fails or the trees differ. Then the checks again on `main`'s
    tree, `hashcheck` against the fetched `origin/main` (spec §12: every
    hash on `main`, every `path:line` resolving, `release-notes.sh 0.5.0`
    exiting 0); then spec §12's gate, and Phase 5 closed.
26. **The cut, as this plan makes it.** Spec §7's three commits: the
    spec's (`5d57733d`), this plan's (`Plan Phase 5.6c: the records, ADR
    0061`, with the spec's amendment), then the records commit. Task 0 is
    the baseline; Task 1 the records commit; Task 2 its verification, the
    review, the pull request, CI and the merge; Task 3 spec §12's gate
    after the merge. Spec §6's prototype was made before this plan
    (Checked on), and its measurement on CI is none (spec decision 20).
27. **The facts that proved wrong, and the outline as filled** **(differs;
    the amendment records each):**
    - **the prototype's base** (spec §6, "from `2842d63c`"): it branched
      from `5d57733d`, `2842d63c` with the spec, which changes no file the
      records touch;
    - **the release's assets** (§3.1, the gate's record: "Its eight
      assets, each named for `0.5.0`, are the two install scripts, …"):
      `install.sh` and `install.ps1` are not named for 0.5.0; the ADR says
      "the two install scripts, and the four archives, std's site and the
      `.vsix`, each named for `0.5.0`, with `SHA256SUMS`";
    - **the bootstrap step's lines** (§3.1, "and printed, its stage times
      left out:"): the step printed more lines than the block quotes
      (cargo's, the test's own line) and the block shortens the runtime's
      path; the ADR says "printed these lines among others, its stage times
      left out and the runtime's path shortened:";
    - **"(the user's decision 3)"** (§3.1, "What went otherwise"): in
      ADR 0061's own legend "decision 3" is the phase plan's; the lessons
      are R3's, and the ADR says "(R3)";
    - **three citations the commit moves** (§3.1; spec §9, risk 1), cited
      at the records commit: `docs/releasing.md:735-738` is `:750-753`
      (the R7 note adds 12 lines above it, the `:537` sentence 3);
      `nova-spec/00-MASTER-SPEC.md:923` is `:940` (R4's note, 17);
      `docs/phase-5-plan.md:748-751` is `:768-771` (the 5.6 note, 20);
    - **what the records say of Open VSX** (§3.1's gate's record, "held
      0.5.0 inactive for a few minutes after the upload, until its check
      passed" and "the release's `nova-vscode-0.5.0.vsix`"; §3.5's note,
      "for a few minutes after its upload, while it checks it"; §11's R3,
      "until Open VSX's check"): more than the release's ledger shows; the
      ADR and the note say only what was seen, and the note names the file
      to publish (decision 14);
    - **N's stack** (§3.1, "Found outside this boundary": "its 1 MB main
      thread"): ADR 0055 §12 says "N runs on its main thread, 1 MB on
      Windows", and the ADR says so (decision 14);
    - **the outline, filled:** Context lists R7 with R1 to R6 (the
      backlog cites it, and the Status's last sentence covers it); the
      decisions are labelled "5.6 decision N", "5.6b decision N" and "R1"
      to "R7", the status words' form; R9.1 and R3.1, the release
      ledger's labels, are described, not labelled, since the ledger is
      not in the repository; the inventory has 42 rows, against "about
      44"; ADR 0055's overflow is given as ADR 0055 measured it, "at 125
      (8,000 levels) on the debug runtime, at 125 or 126 on the release
      one"; References name this plan (decision 14);
    - **ADR 0061's length** (§3.1, "Target 560 to 620 lines"): 492. The
      parts' own targets sum to about 426, and the ADR keeps to them but
      where a part holds more than the table guessed: Context 84 lines
      (55: the pull requests' table and the user's twelve decisions), the
      gate's record 142 (125, drafted in full by the spec), the backlog
      53 (45).
28. **The spec's recommendations, not asked, as taken** (spec §11, 7 to
    21): `main`'s commits only, by 8 characters; the CHANGELOG note after
    the sub-phase list; "built" and "not done"; the tolerances cited; the
    `Map` search repeated; the plan's status line and `14-CODEGEN.md`
    unchanged; the master spec's note "Recorded"; `:537` changed in place;
    run 45352790982 accepted, its counts not read; one records commit; no
    draft pull request; ADR 0061 dated by its commit's day, citing no pull
    request of its own.
29. **The branch starts from `5d57733d`:**
    `claude/loving-archimedes-frtf5q-56b-records` from `5d57733d` (`main`
    `2842d63c` and the approved spec), then this plan's commit, which adds
    this file and appends the spec's dated amendment (2026-11-26): the
    facts that proved wrong and the outline as filled (decision 27), and
    §7's cut as this plan makes it (decision 26). Then Task 1's records
    commit. The blocks apply to the plan's commit as to `5d57733d`.

### Task 0: The baseline

The branch is `claude/loving-archimedes-frtf5q-56b-records`: from
`5d57733d`, then this plan's commit, which adds this file and appends the
spec's dated amendment, so the blocks apply to it as to `5d57733d`
(decision 29). Each step runs in a shell set up as the conventions say.

- [ ] **Step 1: The tree.**

  ```sh
  git status --short
  git branch --show-current
  git log --format='%h %s' -2
  git diff --name-only 5d57733d HEAD
  git rev-parse origin/main
  git rev-parse --is-shallow-repository; cat "$(git rev-parse --git-path shallow)"
  git merge-base --is-ancestor a71d27d9 origin/main && echo "a71d27d9 on origin/main"
  git rev-parse --short HEAD
  ```

  Expected:
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-56b-records`;
  - the log: this plan's commit (`Plan Phase 5.6c: the records, ADR
    0061`, spec §7's subject), then `5d57733d docs: the Phase 5.6c spec
    is approved`;
  - the names: `docs/superpowers/plans/2026-11-26-phase-5-6c-the-records.md`
    and `docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md`;
  - `2842d63c4e9eb2349d168d381221a1b3319dd422`, the release commit;
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
  /sys/fs/cgroup/pids/nova-limit/pids.max` prints `600`. Record them.
- [ ] **Step 3: The base's records.**

  ```sh
  ls docs/adr/0061-phase-5-boundary.md
  bash .github/scripts/release-notes.sh 0.5.0 CHANGELOG.md > "$P/exec56c/notes-base.md"; echo "exit $?"
  wc -l < "$P/exec56c/notes-base.md"
  bash .github/scripts/release-notes.sh Unreleased CHANGELOG.md; echo "exit $?"
  grep -n 'will record the try' docs/releasing.md
  grep -c 'claude/loving-archimedes-frtf5q-56b-records' CHANGELOG.md docs/phase-5-plan.md docs/releasing.md nova-spec/00-MASTER-SPEC.md
  ```

  Expected: `ls` fails (`No such file or directory`): no ADR 0061 yet;
  `exit 0` and `614` lines, the released notes (5.6b's ledger: the
  release's body is the release commit's `[0.5.0]`, 614 lines); `exit 1`,
  `release-notes: CHANGELOG.md has no notes for Unreleased`; the runbook's
  `537:decision 5). `docs/adr/0061-phase-5-boundary.md` will record the
  try.`; and `0` for each of the four files: no note of this branch yet.
- [ ] **Step 4: The ledger.** Start `$P/ledger-56c.md`, headed with
  `BASE`, `origin/main`, the shallow boundary, the tools' versions, the
  process limit's form and the disk; and `$P/exec56c`, the executor's
  directory (Conventions).

### Task 1: The records commit

Spec §3.1 to §3.5 in one commit, `docs: Phase 5's records, ADR 0061`, in
two blocks (decision 15), dated by `redate.sh` (decision 17). Each step
runs in a shell set up as the conventions say.

- [ ] **Step 1: The blocks.** `patch_block t1-adr`, then `patch_block
  t1-notes`, each printing nothing. Then:

  ```sh
  git status --short
  wc -l < docs/adr/0061-phase-5-boundary.md
  git diff --numstat
  ```

  Expected: `?? docs/adr/0061-phase-5-boundary.md` and ` M` for
  `CHANGELOG.md`, `docs/phase-5-plan.md`, `docs/releasing.md`,
  `nova-spec/00-MASTER-SPEC.md` and `selfhost/README.md`, and nothing
  else; `492`; and the five files' numstat:

  ```text
  13	0	CHANGELOG.md
  20	0	docs/phase-5-plan.md
  16	1	docs/releasing.md
  17	0	nova-spec/00-MASTER-SPEC.md
  1	0	selfhost/README.md
  ```

- [ ] **Step 2: The message and the scripts.** The message, written as it
  stands:

```sh
mkdir -p "$P/exec56c"
cat > "$P/exec56c/msg-t1.txt" <<'EOF'
docs: Phase 5's records, ADR 0061

ADR 0061 closes Phase 5 as v0.5.0: its inventory, the deviations as
their records decided them, "The gate's record" with every run of the
release (CI 45345843046 and 45352790982, Release 45373121044, "Publish
to crates.io" 45548505763, Release gate 45596343684), both store
listings and the user's try of the published nova, the backlog with
macOS and Windows on it, and what was found outside it. CHANGELOG's
[0.5.0] gains a dated note with the same run IDs; the master spec's
Phase 5 gate, the phase plan's 5.6 and selfhost/README.md point to it;
the runbook's step 6.2 says what it records, and its Open VSX step
gains the release's two lessons.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
EOF
```

  `redate.sh` (decision 17), written as it stands:

```sh
mkdir -p "$P/exec56c"
cat > "$P/exec56c/redate.sh" <<'EOF'
#!/usr/bin/env bash
# redate.sh [--check] DATE: date the records commit DATE (YYYY-MM-DD, the
# commit's day in UTC). The blocks date it 2026-11-26 in exactly six places,
# each found by the words around it: ADR 0061's Status, "Accepted (DATE).";
# its Consequences, "with its note of DATE;"; the three notes "**Recorded
# DATE (" in CHANGELOG.md, the master spec and the phase plan; and the
# runbook's Open VSX note, "**Amended DATE (". Each is rewritten whatever
# date it holds, so the script can run again; with --check nothing is
# written. Then it checks, against 2842d63c (the release commit, whose files
# the records change): each place holds DATE; no line the records remove
# holds a date; and every other date in the lines they add is history,
# exactly as written: the earlier rulings' and pull requests' days, the
# release's 2026-11-24 and 2026-11-25, and the user's decisions of
# 2026-11-26 and the try (2026-11-26 three times in the ADR: the Status's
# "the user's of 2026-11-26", R1's day and the try; once in the runbook,
# the try's), whatever DATE is. Dates in file names (DATE-phase-…) are
# names, not dates. The message carries no date. Run from the repository's
# root, after the blocks are applied, before or after the commit.
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
B = "`claude/loving-archimedes-frtf5q-56b-records`"
PLACES = [
    ("ADR 0061's Status", "docs/adr/0061-phase-5-boundary.md",
     r"(?m)^(Accepted \()(%s)(\)\. Branch %s)$" % (DATE, re.escape(B))),
    ("ADR 0061's Consequences", "docs/adr/0061-phase-5-boundary.md",
     r"(with its note of\n {5})(%s)(;\n)" % DATE),
    ("the CHANGELOG note", "CHANGELOG.md",
     r"(?m)^(\*\*Recorded )(%s)( \(branch\n%s\):\*\* `v0\.5\.0` is published:)$" % (DATE, re.escape(B))),
    ("the master spec note", "nova-spec/00-MASTER-SPEC.md",
     r"(?m)^(\*\*Recorded )(%s)( \(branch %s\):)$" % (DATE, re.escape(B))),
    ("the phase plan note", "docs/phase-5-plan.md",
     r"(?m)^(\*\*Recorded )(%s)( \(5\.6c, branch\n%s, ADR 0061; spec)$" % (DATE, re.escape(B))),
    ("the runbook's Open VSX note", "docs/releasing.md",
     r"(?m)^(   \*\*Amended )(%s)( \(branch\n   %s\):\*\* in Windows)$" % (DATE, re.escape(B))),
]
HISTORY = {
    "docs/adr/0061-phase-5-boundary.md": {"2026-10-27": 1, "2026-11-06": 2, "2026-11-22": 2, "2026-11-23": 3,
                                          "2026-11-24": 8, "2026-11-25": 4, "2026-11-26": 3},
    "docs/releasing.md": {"2026-11-26": 1},
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
                        "--src-prefix=a/", "--dst-prefix=b/", "-U0", "2842d63c", "--", ".",
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

  `checks.py` (decision 18), written as it stands: the records' checks,
  each `CHK NAME …` (the env file's `CHK`), each from the repository's
  root, each ending `NAME: ok` and exit 0, or `NAME: FAILED` and exit 1:

```sh
mkdir -p "$P/exec56c"
cat > "$P/exec56c/checks.py" <<'EOF'
#!/usr/bin/env python3
"""checks.py CHECK [ARGS]: the records commit's checks (spec §5), run from
the repository's root. Each reads the working tree against BASE, the
release commit `2842d63c` on `main`, whose six files the records change:
an edit is seen before it is committed (the mutants are such edits), and a
clean tree at the records commit is that commit. The "added lines" are the
lines `git diff BASE` adds outside docs/superpowers, and every line of an
untracked file. Each check prints what it found and a last line
`CHECK: ok` or `CHECK: FAILED`, and exits 0 or 1.

  hashcheck [--msg FILE]  every word of 7 to 40 hex digits in the added
      lines and the message is a commit on `main`, by 8 characters
      (`git merge-base --is-ancestor` against origin/main, or one of the
      two older than the clone's shallow boundary `a71d27d9`, which
      GitHub's list_commits places on `main`), or a word this table
      classifies: a run ID, a size or a runtime directory's CRC.
  pathcheck   every backticked `path:line` in ADR 0061 is in this table,
      resolves in the tree, and its lines hold the words; every backticked
      relative repository path in the added lines exists; every Markdown
      link resolves; every URL is a run of this repository's that the
      table holds. The table, CITES, is keyed by ADR 0061's line, so an
      edit that adds or removes a line above a citation moves every key
      after it: run renumber, then pathcheck.
  renumber    rewrite CITES' keys in this script from ADR 0061 in the tree,
      by the citations' order: the k-th backticked `path:line` in the ADR
      takes the table's k-th row. It writes nothing, and fails, if the
      ADR's citations differ from the table's in number or order (a
      citation added, removed or changed is a row added, removed or
      changed by hand). pathcheck then checks each row's words again.
  privacy [--msg FILE]  no local or scratch path, branch but the records'
      own, person's name or email (built here from the remote, the
      committers on `main` and the git config, never written), model
      identifier, session link or token, in the added lines or the
      message (its two trailer lines excepted).
  notes D     release-notes.sh 0.5.0 exits 0, and prints the released notes
      (BASE's) with spec §3.2's note, dated D, and one blank line inserted,
      nothing else; release-notes.sh Unreleased exits 1.
  files       exactly the six files differ from BASE outside
      docs/superpowers, and no other file is untracked.
  place D     spec §3's drafted blocks, dated D where the commit's day
      goes, each verbatim, once, at the place §3 names (§5 check 1); the
      runbook's Open VSX note as the spec's amendment, item 6, gives it.
  split D     each of the six files, with D put back to 2026-11-26 (the
      day the split's records carry; no other text of theirs holds a later
      day), hashes to the split's blob: the records are the split's.
"""
import collections
import glob
import os
import re
import subprocess
import sys
import tempfile

BASE = "2842d63c"
MAIN = "origin/main"
BRANCH = "claude/loving-archimedes-frtf5q-56b-records"
SPEC = "docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md"
ADR = "docs/adr/0061-phase-5-boundary.md"
SIX = ["CHANGELOG.md", ADR, "docs/phase-5-plan.md", "docs/releasing.md",
       "nova-spec/00-MASTER-SPEC.md", "selfhost/README.md"]
TRAILERS = ["Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>",
            "Claude-Session: https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN"]
RUNS = {
    "45142347493": "CI, main at ed165975", "45139701894": "CI, #149",
    "45345843046": "CI, #151", "45345843088": "Release, #151",
    "45352790982": "CI, main at 2842d63c", "45352790526": "Benchmarks, main at 2842d63c",
    "45373121044": "Release, the tag v0.5.0", "45548505763": "Publish to crates.io",
    "45596343684": "Release gate",
}
OTHER = {
    "9539736": "a size: S1 at ed165975", "9539848": "a size: S1 at 2842d63c",
    "12760111": "a size: N's .ll", "5549317a": "a CRC: the runtime 0.4.0-5549317a",
    "4f8a6f1b": "a CRC: the runtime 0.5.0-4f8a6f1b",
}
# Older than the shallow boundary a71d27d9, so git here cannot place them;
# GitHub's list_commits from a71d27d9 on main lists both (2026-11-26).
GITHUB = {"cc829f58": "v0.4.0's records, #127", "a3a95a2e": "5.0a's last commit, #128"}
SPLIT = {  # the six files' blobs at the split's tip, whose records are dated 2026-11-26
    "CHANGELOG.md": "ea19a2bcef223ad41981fbec96d167017106ee57",
    "docs/adr/0061-phase-5-boundary.md": "660424bfd8297439f14a64655cf00309c566980e",
    "docs/phase-5-plan.md": "a5e73f959067f7c391dfba8b53285db1199e9187",
    "docs/releasing.md": "6e747bd8e64d89c1bae1fa0fad96cf6e09ea97c0",
    "nova-spec/00-MASTER-SPEC.md": "9cd4899b3328a8d1ad4e16135ad7f929420803e4",
    "selfhost/README.md": "5e518f23cc54ac3e82b7801178cddd1b560afa0b",
}
# The runbook's Open VSX note (R7) as the spec's amendment, item 6, gives it:
# §3.5's draft said more than the release's ledger shows.
R7_NOTE = r"""   **Amended 2026-11-26 (branch
   `claude/loving-archimedes-frtf5q-56b-records`):** in Windows
   PowerShell 5.1, whose `Read-Host` has no `-MaskInput`, read the token
   masked with `$s = Read-Host -AsSecureString "token"` and
   `$env:OVSX_PAT = [System.Net.NetworkCredential]::new('', $s).Password`,
   run `npx ovsx publish .\nova-vscode-<version>.vsix` in the `.vsix`'s
   folder, then `Remove-Item Env:OVSX_PAT; Remove-Variable s`. Open VSX
   may show a new version as already published but not yet active, for a
   while after its upload; `ovsx publish` then answers "already published,
   but currently isn't active": wait for the listing, and do not publish
   again (`docs/adr/0061-phase-5-boundary.md`).""".split("\n")
CITES = [
    (27, 'nova-spec/00-MASTER-SPEC.md:746-751', None, [
        '1. Port lexer to Nova',
        '6. Bootstrap: stage0 (Rust)',
    ]),
    (28, ':753', 'nova-spec/00-MASTER-SPEC.md', [
        '**Gate:** CI builds stage2 == stage1 byte-for-byte (or close — may differ in non-deterministic codegen, document tolerances).',
    ]),
    (30, ':22', 'nova-spec/00-MASTER-SPEC.md', ['Self-hosting target', 'Phase 5']),
    (31, ':197-294', 'docs/phase-5-plan.md', [
        '## 3. Key decisions',
        '10. **Rust, and tests.**',
        'Tests never touch the internet.',
    ]),
    (104, 'nova-spec/00-MASTER-SPEC.md:746-751', None, [
        '5. Backend bindings: bind LLVM via `unsafe` extern from Nova',
    ]),
    (144, ':38-39', '0048', [
        'pointer FFI, `unsafe` blocks, linking C libraries, and a binding of',
        "LLVM's C API",
    ]),
    (145, ':40-41', '0048', [
        'porting `run`, `test`, fmt, the language server, doc, the package',
        'manager, and the Cranelift and WASM backends',
    ]),
    (146, ':42', '0048', ['S1 as the shipped `nova`']),
    (147, ':43', '0048', ['the byte gate on macOS and Windows']),
    (148, ':44', '0048', ["N built for WASM, which Phase 6's playground needs"]),
    (149, ':45', '0048', ['trait objects, mutable globals and nested module directories']),
    (151, ':773-775', '0053', [
        "writes `/` in E0093's path on Windows, where S0",
        "5.5's driver settles it",
    ]),
    (204, ':1095-1097', '0052', ['N is about 6× slower than release S0']),
    (215, '.github/workflows/ci.yml:200-233', None, [
        '- name: Install clang 18.1.3 (Linux)',
        '- name: S1 builds the golden suite (Linux)',
        '- name: The bootstrap, S1 == S2 == S3 (Linux)',
        'cc --version',
        'ld --version',
        'the_bootstrap_holds_s1_s2_s3',
    ]),
    (216, 'crates/nova-cli/tests/selfhost.rs:8591', None, ['fn the_bootstrap_holds_s1_s2_s3()']),
    (229, 'docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md:211-219', None, [
        'Linux 2838 0 10; macOS 2838 0 9; Windows 2815 0 9',
        '8, 3 and 25 passed',
        'Linux 71 0 3 (55.57 s); macOS 69 0 3 (31.18 s); Windows 65 0 3 (108.49 s)',
        '2 passed (30.46 s)',
        '1 passed; S1 == S2 == S3',
    ]),
    (279, ':217', '0060', ['this search, which the boundary ADR repeats.']),
    (328, '.github/workflows/release-gate.yml:83', None, [
        "if grep -qF -- '--node) with_node=--node' .github/scripts/gate.sh; then flags+=(--node); fi",
    ]),
    (328, ':169', '.github/workflows/release-gate.yml', [
        "if grep -qF -- '--node) with_node=--node' .github/scripts/gate.sh; then flags+=(--node); fi",
    ]),
    (329, ':45', '.github/scripts/gate.sh', ['--node) with_node=--node ;;']),
    (349, 'docs/releasing.md:750-753', None, [
        '**The extension** (step 4).',
        'A version once published to a store cannot be published',
        'there again.',
    ]),
    (357, ':36-45', '0048', [
        '2. **What Phase 5 leaves out,** each to the backlog',
        'trait objects, mutable globals and nested module directories.',
    ]),
    (365, ':642-643', '0060', [
        'macOS and Windows stay outside the gate',
        "go to the boundary ADR's backlog with an unpinned `cc` and `ld`.",
    ]),
    (367, 'selfhost/README.md:66', None, [
        'It refuses Windows. Its linking on macOS has not yet run.',
    ]),
    (368, ':219', '0060', ['No lint is added']),
    (373, ':740-745', '0059', [
        'project mode,',
        "`--ssg`; `llc`; WASM and test mode's code generation; Cranelift in N;",
        'an embedded runtime in N; Windows linking; rendered diagnostics;',
        "`check`'s code and span on every input",
    ]),
    (376, ':516-521', '0054', [
        '**Not ported**',
        'the M-codes',
        'registry packages, `nova.lock`',
        'doc tests',
        'whole-package program',
    ]),
    (392, ':1220-1221', '0056', [
        "The bug of §4 (std's bodies see a program's impls) stays in S0, worked",
        'around by the fallback',
    ]),
    (393, ':885-886', '0058', ['Float `%` still type-checks and fails in `build --release`']),
    (395, 'nova-spec/00-MASTER-SPEC.md:940', None, [
        '5. Interactive playground (compile in browser via WASM-compiled Nova compiler)',
    ]),
    (396, ':327', '0025', ['the `?` operator.']),
    (397, ':337', '0025', ['- `std/process`: `spawn`; `env`.']),
    (399, ':443-445', '0047', ['nested module directories']),
    (408, ':524-527', '0055', [
        'The user ruled',
        '(2026-11-06)',
        "5.5's driver runs N on a large-stack thread",
    ]),
    (411, ':518-524', '0055', [
        'survives 64 linked associated',
        'types each nesting 124 arrays (7,936 levels)',
        'on the',
        'debug runtime it overflows at 125 (8,000 levels), and on the release',
        'runtime at 125 or at 126 (8,064 levels)',
        'nothing',
        'in the corpus, the cases or the fuzzer comes near',
    ]),
    (416, ':773-775', '0053', ["5.5's driver settles it"]),
    (418, ':265', '0059', ["Windows: `build` refused (§4), and a locus's `\\` read as `/`;"]),
    (420, 'docs/superpowers/plans/2026-11-24-phase-5-6b-the-release-v0.5.0.md:501-507', None, [
        'the fresh `[Unreleased]`',
        '`specblocks.py`',
        '`[Unreleased]` grep',
    ]),
    (421, ':706-724', 'docs/superpowers/plans/2026-11-24-phase-5-6b-the-release-v0.5.0.md', [
        "`specblocks.py` with the commit's day",
        "grep -m2 '^## \\[' CHANGELOG.md",
        'C2 (the fresh `[Unreleased]` left out)',
    ]),
    (422, 'docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md:523-531', None, [
        "the plan's check of them against §3.2 to §3.5's drafts and of the",
        'fresh `[Unreleased]`',
    ]),
    (427, '.github/workflows/release-gate.yml:75', None, ["# the tag's gate.sh takes it"]),
    (428, ':1803', 'docs/superpowers/plans/2026-11-24-phase-5-6b-the-release-v0.5.0.md', [
        '`.github/workflows/release-gate.yml:75`',
    ]),
    (428, ':2681', 'docs/superpowers/plans/2026-11-24-phase-5-6b-the-release-v0.5.0.md', [
        '`.github/workflows/release-gate.yml:75`',
    ]),
    (429, ':83', '.github/workflows/release-gate.yml', ['--node) with_node=--node']),
    (429, ':169', '.github/workflows/release-gate.yml', ['--node) with_node=--node']),
    (431, 'docs/phase-5-plan.md:768-771', None, [
        '- **Records.**',
        "The master spec's Phase 5 and `14-CODEGEN.md` get dated notes as each",
        'sub-phase lands.',
    ]),
    (432, ':491-497', '0047', [
        "The phase plan's §5 asks for dated notes in `30-FRONTEND.md`,",
        '`14-CODEGEN.md`',
    ]),
    (433, ':3', 'docs/phase-5-plan.md', ['> Status: **approved** by the user (2026-10-26).']),
    (434, ':463-464', '0037', ['`docs/phase-3-plan.md`\'s status line still says "draft"']),
    (434, ':4', 'docs/phase-5-plan.md', ['§1 ("Self-hosting target")']),
    (435, 'nova-spec/00-MASTER-SPEC.md:22', None, ['Self-hosting target', 'Phase 5 (~month 42)']),
    (436, ':180', '0060', ['tree as of `']),
    (436, ':467', '0060', ["**`selfhost.rs`'s main step as CI runs it,** before (`"]),
    (437, ':487-489', '0060', ['run 45003631229 on `', 'measured, 19 of 19 jobs green']),
    (437, ':544', '0060', ['`the_bootstrap_reports_name_the_stage`, as `']),
]


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
    if "--msg" not in argv:
        return []
    path = argv[argv.index("--msg") + 1]
    return [l for l in open(path, encoding="utf-8").read().split("\n") if l not in TRAILERS]


def done(name, bad):
    print(f"{name}: {'ok' if not bad else 'FAILED'}")
    sys.exit(1 if bad else 0)


def hashcheck(argv):
    lines = diff_lines()[0] + [("the message", l) for l in message(argv)]
    words = collections.OrderedDict()
    for f, l in lines:
        for m in re.finditer(r"(?<![0-9A-Za-z_])[0-9a-f]{7,40}(?![0-9A-Za-z_])", l):
            words.setdefault(m.group(0), f)
    bad, kinds = 0, collections.Counter()
    for w, f in words.items():
        if w in RUNS:
            kind, verdict = "run IDs", f"run {RUNS[w]}"
        elif w in OTHER:
            kind, verdict = "other words", OTHER[w]
        elif w in GITHUB:
            kind, verdict = "on main, by GitHub", f"commit, on main by GitHub's list_commits ({GITHUB[w]})"
        else:
            full = git("rev-parse", "--verify", "--quiet", w + "^{commit}", check=False).stdout.strip()
            if not full:
                kind, verdict = "BAD", "FAIL: neither a commit here nor a word this table classifies"
            elif git("merge-base", "--is-ancestor", full, MAIN, check=False).returncode == 0:
                kind, verdict = "on main, by git", f"commit on {MAIN}: {git('log', '-1', '--format=%s', full).stdout.strip()[:60]}"
            else:
                kind, verdict = "BAD", (f"FAIL: commit {full[:12]} is not on {MAIN} (if it is older than "
                                        "the shallow boundary a71d27d9, the main session checks it with GitHub)")
        if kind.startswith("on main") and len(w) != 8:
            kind, verdict = "BAD", f"FAIL: a commit named by {len(w)} characters, not 8 (decision 8)"
        bad += kind == "BAD"
        kinds[kind] += 1
        print(f"{w}\t{f}\t{verdict}")
    print("hashcheck: " + ", ".join(f"{k} {v}" for k, v in sorted(kinds.items())))
    done("hashcheck", bad)


def target(path, token):
    if path is None:
        return token.rsplit(":", 1)[0]
    if re.fullmatch(r"\d{4}", path):
        g = glob.glob(f"docs/adr/{path}-*.md")
        return g[0] if len(g) == 1 else f"docs/adr/{path}-?.md"
    return path


def pathcheck(argv):
    bad = 0
    adr = open(ADR, encoding="utf-8").read().split("\n")
    cite = re.compile(r"`([^`\s]*):(\d+)(?:-(\d+))?`")
    for n, token, path, words in CITES:
        p = target(path, token)
        if n > len(adr) or f"`{token}`" not in adr[n - 1]:
            print(f"FAIL ADR:{n} `{token}`: not on that line")
            bad += 1
            continue
        m = cite.fullmatch(f"`{token}`")
        a, b = int(m.group(2)), int(m.group(3) or m.group(2))
        if not os.path.isfile(p):
            print(f"FAIL ADR:{n} `{token}`: {p} is not in the tree")
            bad += 1
            continue
        lines = open(p, encoding="utf-8").read().split("\n")
        if lines and lines[-1] == "":
            lines.pop()
        seg = lines[a - 1:b]
        text, flat = "\n".join(seg), " ".join(x.strip() for x in seg)
        missing = [w for w in words if w not in text and w not in flat] if b <= len(lines) else words
        print(f"{'FAIL' if missing else 'ok'} ADR:{n} `{token}` -> {p}:{a}-{b}"
              + (f" missing {missing!r}" if missing else ""))
        bad += bool(missing)
    held = {(n, t) for n, t, _, _ in CITES}
    for n, l in enumerate(adr, 1):
        for m in cite.finditer(l):
            if (n, m.group(0)[1:-1]) not in held:
                print(f"FAIL ADR:{n} {m.group(0)}: a citation the table does not hold")
                bad += 1
    added = diff_lines()[0]
    for f, l in added:
        if f != ADR and cite.search(l):
            print(f"FAIL {f}: a path:line outside ADR 0061: {l.strip()[:80]}")
            bad += 1
    text = "\n".join(l for _, l in added)
    paths = set()
    for s in re.findall(r"`([^`\s]+)`", text):
        base = re.sub(r":\d[\d-]*$", "", s)
        if ("/" in base and re.search(r"(\.[a-z]{1,5}|/)$", base) and "*" not in base
                and "<" not in base and not base.startswith(("http", "refs/", "…", "/", "~", "$"))):
            paths.add(base)
    for p in sorted(paths):
        ok = os.path.exists(p)
        bad += not ok
        print(f"{'ok' if ok else 'FAIL'} path {p}{'' if ok else ': not in the tree'}")
    links = re.findall(r"\]\(([^)]+)\)", text)
    for u in links:
        ok = u.startswith("https://") or os.path.exists(u.split("#")[0])
        bad += not ok
        print(f"{'ok' if ok else 'FAIL'} link {u}")
    urls = [u.rstrip(",.;") for u in re.findall(r"https?://[^\s)>`\"]+", text)]
    for u in urls:
        m = re.fullmatch(r"https://github\.com/[A-Za-z0-9-]+/nova/actions/runs/(\d+)", u)
        ok = bool(m) and m.group(1) in RUNS
        bad += not ok
        print(f"{'ok' if ok else 'FAIL'} url {u.split('/actions/')[-1]}")
    print(f"pathcheck: {len(CITES)} path:line citations, {len(paths)} repository paths, "
          f"{len(links)} Markdown links, {len(urls)} run URLs")
    done("pathcheck", bad)


def renumber(argv):
    adr = open(ADR, encoding="utf-8").read().split("\n")
    cite = re.compile(r"`([^`\s]*):(\d+)(?:-(\d+))?`")
    found = [(n, m.group(0)[1:-1]) for n, l in enumerate(adr, 1) for m in cite.finditer(l)]
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
    lines = added + [("the message", l) for l in message(argv)]
    owner, names = private_names()
    fam = "|".join(["op" + "us", "son" + "net", "hai" + "ku", "fa" + "ble"])
    scratch = os.environ.get("P", "")
    rules = [
        ("a local or scratch path", r"/tmp/|/home/|/root\b|/Users/|[A-Za-z]:\\Users|~/"
         r"|(?<![\w$])\$P\b|\b(?:exec|plan|proto|env|ledger-)56[a-z]\b|" + "scratch" + "pad"),
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
            for form in (f"https://github.com/{owner}/nova", f"{owner}.vscode-nova"):
                k = len(re.findall(re.escape(form), rest, re.I))
                if k:
                    allowed["the owner, in the repository's URLs and the publisher id"] += k
                    rest = re.sub(re.escape(form), "", rest, flags=re.I)
        for n in names:
            if re.search(re.escape(n), rest, re.I):
                print(f"FAIL {f}: a person's name or address (built from git, not written): "
                      f"{re.sub(re.escape(n), '<name>', l.strip(), flags=re.I)[:90]}")
                bad += 1
    print(f"privacy: {len(lines)} lines read, {len(names)} names built; allowed: "
          + (", ".join(f"{k} {v}" for k, v in sorted(allowed.items())) or "none"))
    done("privacy", bad)


def notes_of(version, path):
    r = subprocess.run(["bash", ".github/scripts/release-notes.sh", version, path],
                       capture_output=True, text=True, encoding="utf-8")
    return r.returncode, r.stdout, r.stderr.strip()


def spec_block(heading, nth=1):
    lines = open(SPEC, encoding="utf-8").read().split("\n")
    k = lines.index(heading)
    for _ in range(nth):
        k = next(i for i in range(k + 1, len(lines)) if lines[i].strip() == "```markdown")
    pad = len(lines[k]) - len(lines[k].lstrip())
    end = next(i for i in range(k + 1, len(lines)) if lines[i].strip() == "```")
    return [l[pad:] for l in lines[k + 1:end]]


def notes(argv):
    D = argv[0]
    bad = 0
    rc, now, err = notes_of("0.5.0", "CHANGELOG.md")
    print(f"release-notes.sh 0.5.0 CHANGELOG.md: exit {rc} {err}")
    bad += rc != 0
    with tempfile.TemporaryDirectory() as t:
        old = os.path.join(t, "CHANGELOG.md")
        open(old, "w", encoding="utf-8").write(git("show", f"{BASE}:CHANGELOG.md").stdout)
        rc0, was, _ = notes_of("0.5.0", old)
    block = [l.replace("**Recorded 2026-11-26 (", f"**Recorded {D} (")
             for l in spec_block("### 3.2 `CHANGELOG.md`: the dated note in `[0.5.0]`")]
    A, R, ins = now.split("\n")[:-1], was.split("\n")[:-1], block + [""]
    k = next((i for i in range(min(len(A), len(R))) if A[i] != R[i]), min(len(A), len(R)))
    same = rc == 0 and rc0 == 0 and A == R[:k] + ins + R[k:]
    for name, out in (("released (BASE's)", was), ("now", now)):
        print(f"{name}: {len(out.split(chr(10))) - 1} lines, {len(out.encode())} bytes, {len(out)} characters")
    print(f"the difference: {'spec §3.2 note, dated ' + D + ', and a blank line, inserted after line ' + str(k) if same else 'NOT the note alone'}")
    bad += not same
    rcu, _, erru = notes_of("Unreleased", "CHANGELOG.md")
    print(f"release-notes.sh Unreleased CHANGELOG.md: exit {rcu} {erru}")
    bad += rcu != 1
    done("notes", bad)


def files(argv):
    tracked = {f for f in git("diff", "--name-only", "--no-renames", BASE, "--", ".",
                               ":(exclude)docs/superpowers").stdout.split("\n") if f}
    untr = set(untracked())
    for f in sorted(untr):
        print(f"untracked: {f}")
    got = tracked | untr
    print(git("diff", "--numstat", BASE, "--", ".", ":(exclude)docs/superpowers").stdout.rstrip())
    for f in sorted(got - set(SIX)):
        print(f"FAIL: {f} changed or untracked, not one of the six")
    for f in sorted(set(SIX) - got):
        print(f"FAIL: {f} unchanged")
    print(f"files: {len(got)} changed against {BASE} outside docs/superpowers, {len(untr)} untracked")
    done("files", got != set(SIX) or bool(untr - set(SIX)))


def place(argv):
    D = argv[0]
    spec = open(SPEC, encoding="utf-8").read().split("\n")

    def blk(a, b, first, last, dedent=0):
        assert spec[a - 1].strip() == first and spec[b - 1].strip() == last, (a, b)
        return [l[dedent:].replace("Accepted (2026-11-26)", f"Accepted ({D})")
                .replace("**Recorded 2026-11-26 (", f"**Recorded {D} (")
                .replace("**Amended 2026-11-26 (", f"**Amended {D} (") for l in spec[a - 1:b]]

    def show(p):
        return open(p, encoding="utf-8").read().split("\n")

    def at(lines, block):
        return [i + 1 for i in range(len(lines)) if lines[i:i + len(block)] == block]

    checks = []
    ad = show(ADR)
    h = at(ad, blk(73, 83, "## Status", "listed below. It names `main`'s commits only."))
    checks.append(("ADR 0061's Status (spec 73-83)", h, h == [3]))
    h = at(ad, blk(89, 95, '"Item N" is the master spec\'s Phase 5 list', "|---|---|---|---|"))
    checks.append(("the inventory's legend and header (spec 89-95)", h, len(h) == 1))
    for k in (96, 97, 98):
        h = at(ad, [spec[k - 1]])
        checks.append((f"the inventory's sample row (spec {k})", h, len(h) == 1))
    h = at(ad, blk(127, 139, "**Status words:**", "- *Backlog*, *Phase 6:* where a deferred item goes."))
    checks.append(("the status words (spec 127-139)", h, len(h) == 1))
    cl = show("CHANGELOG.md")
    h = at(cl, blk(340, 351, "**Recorded 2026-11-26 (branch", "run and the try."))
    checks.append(("the CHANGELOG note (spec 340-351)", h, h == [58] and cl[55] == "ADR 0060)." and cl[56] == ""
                   and cl[69] == "" and cl[70] == "### Added" and cl[11].startswith("## [0.5.0] - ")
                   and cl[9] == "## [Unreleased]" and cl[10] == ""))
    ms = show("nova-spec/00-MASTER-SPEC.md")
    h = at(ms, blk(364, 379, "**Recorded 2026-11-26 (branch `claude/loving-archimedes-frtf5q-56b-records`):", "met."))
    checks.append(("the master spec note (spec 364-379)", h, len(h) == 1 and ms[h[0] - 3] == "pull requests of their own."
                   and ms[h[0] - 2] == "" and ms[h[0] + 15] == "" and ms[h[0] + 16] == "---"))
    pl = show("docs/phase-5-plan.md")
    h = at(pl, blk(389, 407, "**Recorded 2026-11-26 (5.6c, branch", "0055 §12), and E0093's `\\` on Windows (ADR 0053)."))
    checks.append(("the phase plan note (spec 389-407)", h, len(h) == 1
                   and pl[h[0] - 3].startswith("  - `selfhost/` is not shipped") and pl[h[0] - 2] == ""
                   and pl[h[0] + 18] == "" and pl[h[0] + 19] == "## 5. Cross-cutting"))
    sh = show("selfhost/README.md")
    line = spec[412].strip().strip("`")
    assert line == "- ADR 0061: Phase 5's boundary."
    h = at(sh, [line])
    checks.append(("the selfhost/README.md line (spec 413)", h, len(h) == 1
                   and sh[h[0] - 2] == "- ADR 0060: the bootstrap gate and its tolerances."))
    rl = show("docs/releasing.md")
    h = at(rl, blk(420, 423, "decision 5). `docs/adr/0061-phase-5-boundary.md` records the try, of",
                   "printing `Hello, Nova!`.", 2))
    checks.append(("the runbook's :537 sentence (spec 420-423)", h, len(h) == 1
                   and rl[h[0] - 2].startswith("`docs/superpowers/specs/2026-11-24-phase-5-6b")
                   and rl[h[0] - 6].startswith("**Amended 2026-11-24 (branch")
                   and "will record the try" not in "\n".join(rl)))
    h = at(rl, [l.replace("**Amended 2026-11-26 (", f"**Amended {D} (") for l in R7_NOTE])
    checks.append(("the runbook's Open VSX note (spec 431-441, as amended)", h, len(h) == 1
                   and rl[h[0] - 3] == "   token in the shell's history.)" and rl[h[0] - 2] == ""
                   and rl[h[0] + 10].startswith("3. Check the listings")
                   and rl[h[0] - 13].startswith("2. **[stop]** Open VSX")))
    bad = 0
    for name, h, good in checks:
        bad += not good
        print(f"{'ok' if good else 'FAIL'} {name}: at {h}")
    done("place", bad)


def split(argv):
    D = argv[0]
    bad = 0
    for f, blob in SPLIT.items():
        data = open(f, "rb").read().replace(D.encode(), b"2026-11-26")
        got = subprocess.run(["git", "hash-object", "--stdin"], input=data, capture_output=True,
                             check=True).stdout.decode().strip()
        bad += got != blob
        print(f"{'same' if got == blob else 'DIFFERS'} {f}: {got[:12]}, the split's {blob[:12]}")
    done("split", bad)


if __name__ == "__main__":
    if not os.path.exists(".git") or len(sys.argv) < 2:
        sys.exit(__doc__)
    {"hashcheck": hashcheck, "pathcheck": pathcheck, "renumber": renumber, "privacy": privacy,
     "notes": notes, "files": files, "place": place, "split": split}[sys.argv[1]](sys.argv[2:])
EOF
```

- [ ] **Step 3: The date, and the checks before the commit.**

  ```sh
  D=$(date -u +%F); echo "$D"
  bash "$P/exec56c/redate.sh" "$D"
  CHK place "$D"
  CHK notes "$D"
  CHK files
  CHK split "$D"
  git diff --numstat
  ```

  Expected:
  - the day, then `redate.sh` printing each of its six places holding
    it (ADR 0061's Status and Consequences, the CHANGELOG note, the master
    spec note, the phase plan note, the runbook's Open VSX note), then
    the history in the added lines, exactly
    `docs/adr/0061-phase-5-boundary.md: 2026-10-27 1, 2026-11-06 2,
    2026-11-22 2, 2026-11-23 3, 2026-11-24 8, 2026-11-25 4, 2026-11-26 3`
    and `docs/releasing.md: 2026-11-26 1`, whatever the day, then
    `redate: ok`;
  - `place`: `ok` for each of its twelve blocks, ADR 0061's Status at
    `[3]`, the legend at `[103]`, the sample rows at `[110]`, `[114]` and
    `[116]`, the status words at `[153]`, the CHANGELOG note at `[58]`,
    the master spec note at `[916]`, the phase plan note at `[734]`, the
    `selfhost/README.md` line at `[206]`, the runbook's `:537` sentence
    at `[549]` and its Open VSX note at `[419]`; `place: ok`;
  - `notes`: `exit 0`; the released notes 614 lines, 40673 bytes, 40612
    characters; now 627 lines, 41290 bytes, 41229 characters; the
    difference `spec §3.2 note, dated D, and a blank line, inserted after
    line 45`; `Unreleased` `exit 1`; `notes: ok`;
  - `files`: `untracked: docs/adr/0061-phase-5-boundary.md`, the five
    numstat lines, `files: 6 changed against 2842d63c outside
    docs/superpowers, 1 untracked`, `files: ok`;
  - `split`: `same` for each of the six files, `split: ok`;
  - the numstat as in Step 1: the date changes no line count.
- [ ] **Step 4: Commit.**

  ```sh
  git add -A
  git status --short
  git commit -F "$P/exec56c/msg-t1.txt"
  D=$(sed -n 's/^Accepted (\([0-9-]*\))\. Branch .*/\1/p' docs/adr/0061-phase-5-boundary.md); echo "$D"
  test "$(TZ=UTC git log -1 --date=format-local:%Y-%m-%d --format=%ad)" = "$D" && echo "dated $D"
  printf '%s\n' "$(git log -1 --format=%B)" | tail -n 3
  git show --stat --format='%h %s' HEAD | tail -n 1
  CHK split "$D"
  git status --short
  ```

  Expected:
  - `git status --short` lists the six files, `A` for the ADR and `M` for
    the five, and nothing else;
  - the day the records carry, Step 3's, then `dated D`: the commit's own
    date, in UTC, is that day. If it prints nothing, the day turned since
    Step 3, and the commit is re-dated (decision 17), each command with
    its own timeout:

    ```sh
    D=$(date -u +%F); echo "$D"
    bash "$P/exec56c/redate.sh" "$D"
    CHK place "$D"
    CHK notes "$D"
    git add -A
    git status --short
    git commit --amend --date=now --no-edit
    ```

    each as Step 3 gives it (`git status --short` lists the five files the
    day is in, all but `selfhost/README.md`, their six dated lines alone
    changed), then this step's checks again from its `D=` line;
  - the message ends with a blank line and the two trailer lines, the
    last three lines printed;
  - `6 files changed, 559 insertions(+), 1 deletion(-)`;
  - `split`: `same` for each of the six, `split: ok`: the records commit
    holds the split's records, on any day (decision 21);
  - `git status --short` prints nothing.

  Record the commit, the day and the comparison in the ledger.

<!-- patch t1-adr -->

````diff
diff --git a/docs/adr/0061-phase-5-boundary.md b/docs/adr/0061-phase-5-boundary.md
new file mode 100644
index 00000000..660424bf
--- /dev/null
+++ b/docs/adr/0061-phase-5-boundary.md
@@ -0,0 +1,492 @@
+# ADR 0061 — Phase 5's boundary
+
+## Status
+
+Accepted (2026-11-26). Branch `claude/loving-archimedes-frtf5q-56b-records`
+(`docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md`
+§3.1). Written after the release, as ADR 0037 was: `v0.5.0` is tagged on
+`2842d63c` and published. The three pull requests of 5.6, the release's
+steps and the user's try in place of the runbook's step 6.2 are the
+user's decisions of 2026-11-22 and 2026-11-23; what this ADR records of
+two rulings not carried out, of the try and of Open VSX, what it maps to
+Phase 6, and where it is pointed to, the user's of 2026-11-26. All are
+listed below. It names `main`'s commits only.
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
+Phase 5, "Self-hosting", makes its promises in three places:
+- **the master spec's Phase 5:** its list of six numbered items
+  (`nova-spec/00-MASTER-SPEC.md:746-751`), cited below as "item N", and
+  its gate (`:753`): "CI builds stage2 == stage1 byte-for-byte (or close
+  — may differ in non-deterministic codegen, document tolerances)";
+- **the master spec's §0:** the "Self-hosting target", Phase 5 (`:22`);
+- **`docs/phase-5-plan.md`:** its §3 decisions (`:197-294`), cited as
+  "decision N", and sub-phases 5.0 to 5.6, each with its own gate.
+
+Earlier ADRs already narrowed Phase 5, one for each sub-phase:
+- 0048: the scope, the backend, and the list of what Phase 5 leaves out;
+- 0049: tuples and patterns;
+- 0050: the std a compiler needs;
+- 0051: the self-hosted lexer and the differential;
+- 0052: the parser, the AST dump and the depth limit;
+- 0053: the loader, the resolver and the resolution dump;
+- 0054: packages, the WASM target and test mode in the resolution dump;
+- 0055: item collection and the HIR dump;
+- 0056: the type checker and the full HIR dump;
+- 0057: MIR and the MIR dump;
+- 0058: the `.ll` emitter and the `.ll` dump;
+- 0059: the driver, `build` and `check`;
+- 0060: the bootstrap gate and its tolerances.
+
+Phase 5 was built in fifteen pull requests, merged by rebase from
+2026-10-27 to 2026-11-24: 122 commits on `main` after `cc829f58`, up to
+`2842d63c`. Nine drafts that measured CI were closed unmerged: #134,
+#136, #137, #139, #142, #144, #146, #148 and #150. Each merged pull
+request, with its commits on `main`:
+
+| Sub-phase | PR | `main` | ADR | | Sub-phase | PR | `main` | ADR |
+|---|---|---|---|---|---|---|---|---|
+| 5.0a | #128 | `cc829f58..a3a95a2e` | 0048 | | 5.3b | #141 | `f1f17328..9389a84b` | 0056 |
+| 5.0b | #129 | `a3a95a2e..e13630f0` | 0049 | | 5.4 | #143 | `9389a84b..1570ddbd` | 0057 |
+| 5.0c | #130 | `e13630f0..25be9f3c` | 0050 | | 5.5a | #145 | `1570ddbd..e71446df` | 0058 |
+| 5.1a | #131 | `25be9f3c..4f32f13d` | 0051 | | 5.5b | #147 | `e71446df..83ba8210` | 0059 |
+| 5.1b | #132 | `4f32f13d..1fa7cd3f` | 0052 | | 5.6a | #149 | `83ba8210..ed165975` | 0060 |
+| 5.2a | #133 | `1fa7cd3f..2d38ad7a` | 0053 | | 5.6b | #151 | `ed165975..2842d63c` | — |
+| 5.2b | #135 | `2d38ad7a..2a67c6ce` | 0054 | | CI, Rust 1.100 | #140 | `e5c555d0..f1f17328` | — |
+| 5.3a | #138 | `2a67c6ce..e5c555d0` | 0055 | | | | | |
+
+`v0.5.0` was released on 2026-11-24 and 2026-11-25:
+- the annotated tag `v0.5.0`, on the release commit `2842d63c`;
+- the GitHub release "Nova 0.5.0": four archives, the two install
+  scripts, std's documentation site and the extension's `.vsix`, with
+  `SHA256SUMS`;
+- the 20 crates, on crates.io at 0.5.0;
+- the VS Code extension 0.5.0, in the Marketplace and Open VSX.
+
+The user decided, and approved with each spec:
+1. **5.6 decision 1 (2026-11-22):** 5.6 is three pull requests: the
+   gate, the release, then the records, with this ADR, after the release;
+2. **5.6 decision 6:** the gate on Linux alone; macOS and Windows go to
+   this ADR's backlog;
+3. **5.6 decision 8:** `selfhost/` is not shipped, which the notes say; the
+   archives are unchanged;
+4. **5.6b decision 5 (2026-11-23):** every step of the release, as for
+   `v0.4.0`, with the user's try of the published `nova` in place of the
+   runbook's step 6.2;
+5. **5.6b decision 7:** the records after every publish and the try, in one
+   pull request, as #127 was;
+6. **R1 (2026-11-26):** ADR 0055's thread and ADR 0053's E0093, not
+   done, recorded with the depth N survives, and put on the backlog;
+7. **R2:** the try, recorded as `cargo install nova-cli --locked` on
+   Windows, with the user's words, glossed;
+8. **R3:** Open VSX's token, shown on screen and revoked, and 0.5.0
+   inactive for a while, recorded briefly, as lessons;
+9. **R4:** a dated note under the master spec's Phase 5 gate;
+10. **R5:** Phase 6 gets N built for WASM alone, as the master spec says;
+11. **R6:** notes in `selfhost/README.md` and `docs/releasing.md`, with
+    `CHANGELOG.md`'s and the phase plan's, and none in the root README;
+12. **R7:** R3's lessons as a dated note in the runbook's Open VSX step,
+    not on the backlog.
+
+## Decision
+
+### The inventory
+
+"Item N" is the master spec's Phase 5 list
+(`nova-spec/00-MASTER-SPEC.md:746-751`). "Decision N" is one of
+`docs/phase-5-plan.md`'s §3 decisions, and "5.Nx" one of its sub-phases,
+with its pull request and its commits on `main`.
+
+| Promise | Source | Status | Record |
+|---|---|---|---|
+| Port the lexer to Nova | item 1; 5.1a, #131, `25be9f3c..4f32f13d` | built: `selfhost/src/lexer.nova` and `token.nova`, held to S0's token dump by the differential and by a fuzzer on fixed seeds, on three systems | ADR 0051 |
+| Port the parser to Nova | item 2; 5.1b | built: `selfhost/src/parser.nova` and its `parse_*.nova` modules, held to S0's AST dump by the differential and by fuzzed inputs, with one depth limit, 128, in both compilers | ADR 0052 |
+| Port the AST, HIR and MIR to Nova | item 3; 5.1b to 5.4 | built differently: the AST (5.1b), resolution (5.2a, 5.2b), HIR (5.3a, 5.3b) and MIR (5.4), each held to S0's dump; the MIR is native only, WASM's and test mode's staying S0's | ADRs 0052 to 0057; ADR 0057 §6 |
+| Port the type checker to Nova | item 4; 5.3a, 5.3b | built: item collection and the checking of bodies, held to S0's HIR dump by the differential, a body fuzzer and depth cases | ADRs 0055, 0056 |
+| Backend bindings: "bind LLVM via `unsafe` extern from Nova" | item 5; decision 2 | replaced: N writes LLVM IR as text, as S0 does, and runs `clang` 18.1.3 and `cc` on it; no binding, no `unsafe` | ADRs 0048 (decision 3), 0058, 0059 |
+| Bootstrap: stage0 (Rust) → stage1 → stage2, "assert stage1 == stage2" | item 6; 5.5b, 5.6a | built differently: stage1 by S0's `nova build --release`, stage2 by stage1's `build` and stage3 by stage2's, each running `clang` and `cc`; stage3 == stage2 is held too | ADRs 0059, 0060 |
+| The gate: "CI builds stage2 == stage1 byte-for-byte (or close …, document tolerances)" | the master spec's Phase 5 gate; decision 3 | met on Ubuntu, with no tolerance: S1 == S2 == S3, byte for byte, in `main`'s CI run 45142347493 and in the release pull request's run 45345843046; macOS and Windows on the backlog | ADR 0060; "The gate's record" |
+| The gate: "document tolerances" | the master spec's Phase 5 gate; decision 3 | met: there is none on the bytes, under four conditions: one `-o` basename, one `clang` 18.1.3, one runtime archive, one `cc` and `ld` within a job | ADR 0060 §2 |
+| The "Self-hosting target": Phase 5 | the master spec's §0 | met, as `v0.5.0`: N builds itself, and S1 == S2 == S3 | this ADR |
+| Scope: N covers the pipeline from source to `.ll`, the driver, and `build` and `check`; diagnostics are compared by code and span | decision 1 | built: N's `build` and `check`; what Phase 5 leaves out is ADR 0048's list, in the rows below | ADR 0048 |
+| The IR differential: S0 and S1 emit identical `.ll` on `tests/runtime/`, `examples/` and N | decision 3 | met: on 5.5a's corpus, N's root among it, in the Selfhost job's main step on three systems, and on N from S0, S1 and S2 in the bootstrap step | ADR 0058; ADR 0060 §6 |
+| N in `selfhost/`, a Nova package with flat modules; no new Rust crate | decision 4 | built: `selfhost/`, with 57 modules in `selfhost/src` and its own `nova.toml`; the workspace's crates are those of `v0.4.0` | ADRs 0051, 0054 |
+| What ships: the Rust `nova`; N in the repository, not published | decision 5 | shipped: `v0.5.0` ships the Rust `nova`, and N is in no archive and no crate, as `[0.5.0]` says | ADR 0048; 5.6 decision 8 |
+| The differential and the fuzz harness, in every sub-phase | decision 6 | built: S0's hidden dumps of tokens, the AST, resolution, HIR, MIR and `.ll`, and N's, compared on the corpus and on fuzzed inputs from fixed seeds, in `selfhost.rs` on three systems | ADRs 0051 to 0058 |
+| The bootstrap on every pull request: "a CI job on `ubuntu-latest`" | decision 7 | built differently: a step of the Selfhost job's Linux leg, on `ubuntu-24.04`, on every pull request and push to `main`; 29.11 s in #151's run, under its 75 s line | ADR 0060 §11 |
+| Speed: S1 and S2 link a release runtime; work for speed only if the bootstrap is too slow, and on the user's word | decision 8 | built: the chain links the release runtime S0 carries; interned string literals, unboxed `Option` and `Result`, and inline bounds checks are not built | ADR 0060; backlog |
+| Determinism by construction, checked by a search of N for `Map` iteration | decision 9 | met: ADR 0060 §5's 17 `.keys()` lines, each sorted, given to `closest()` or put back into a `Map`; the search, repeated here on `2842d63c`, prints the same 17 | ADR 0060 §5; "The gate's record" |
+| Rust changes build on 1.78 | decision 10 | met: CI's MSRV job checks the workspace with Rust 1.78, as in #151's run | `.github/workflows/ci.yml`'s MSRV job |
+| 5.0a: the small language and CI | 5.0a, #128, `cc829f58..a3a95a2e` | shipped, in the released `nova`: `?`, `+=` on fields and elements, the newline rule, `String + String`, the warnings E0102 and E0103, and the golden suite through LLVM in CI | ADR 0048 |
+| 5.0b: tuples and patterns | 5.0b, #129, `a3a95a2e..e13630f0` | shipped: tuples, nested patterns, or-patterns, guards, and `Char` and range patterns, lowered by test chains | ADR 0049 |
+| 5.0c: std | 5.0c, #130, `e13630f0..25be9f3c` | shipped: `StringBuilder`, `OrderedMap`, number parsing and conversions, `Float.to_bits`, array methods, and `std/process`'s `run`, `status`, `env` and `current_exe` | ADR 0050 |
+| 5.1a: the lexer and the differential | 5.1a, #131, `25be9f3c..4f32f13d` | built; its gate met: N's token dump equals S0's on the corpus and on a fuzzer's fixed seeds, on three systems | ADR 0051 |
+| 5.1b: the parser, the AST dump and the depth limit | 5.1b, #132, `4f32f13d..1fa7cd3f` | built; its gate met: N's AST dump equals S0's, and both refuse a program nested deeper than the limit with P0002 | ADR 0052 |
+| 5.2a: the loader, the resolver and the resolution dump | 5.2a, #133, `1fa7cd3f..2d38ad7a` | built; its gate met: N's resolution dump of loose programs equals S0's | ADR 0053 |
+| 5.2b: packages, the WASM target and test mode in the resolution dump | 5.2b, #135, `2d38ad7a..2a67c6ce` | built; its gate met, with N's own reader of `nova.toml` | ADR 0054 |
+| 5.3a: item collection and the HIR dump | 5.3a, #138, `2a67c6ce..e5c555d0` | built; its gate met: N's HIR dump of items equals S0's; `selfhost.rs` in a Selfhost job of its own | ADR 0055 |
+| CI under Rust 1.100, between 5.3a and 5.3b | #140, `e5c555d0..f1f17328` | done: Rust 1.100's lints and test layout, `synchronization.lib` among Windows' link libraries, and the Browser job on `ubuntu-24.04`; not a sub-phase | — |
+| 5.3b: the type checker and the full HIR dump | 5.3b, #141, `f1f17328..9389a84b` | built; its gate met: N's full HIR dump equals S0's, with a body fuzzer and depth cases | ADR 0056 |
+| 5.4: MIR and the MIR dump | 5.4, #143, `9389a84b..1570ddbd` | built; its gate met, natively: N's MIR dump equals S0's, std spliced whole, with a MIR fuzzer | ADR 0057 |
+| 5.5a: the `.ll` emitter and the `.ll` dump | 5.5a, #145, `1570ddbd..e71446df` | built; its gate met: N's `.ll` equals S0's on the corpus, N's root among it | ADR 0058 |
+| 5.5b: the driver, `build` and `check` | 5.5b, #147, `e71446df..83ba8210` | built; its gate met: S1 leaves the same binary as S0's `nova build --release` on the 147 golden programs and on N itself, on Linux; `build` is refused on Windows | ADR 0059 |
+| 5.6a: the bootstrap gate | 5.6a, #149, `83ba8210..ed165975` | built; its gate met: S1 == S2 == S3 in the Selfhost job's bootstrap step, in `main`'s run 45142347493; `selfhost/README.md` | ADR 0060 |
+| 5.6b: the release `v0.5.0` | 5.6b, #151, `ed165975..2842d63c` | done: the release commit `2842d63c`, then the tag, the GitHub release, crates.io, both stores and the Release gate, each on the user's word, and the user's try of the published `nova`; recorded | 5.6b decisions 5 and 7; "The gate's record" |
+| 5.6c: the records | 5.6c; 5.6 decision 1 | done: this ADR and the notes that point to it | this ADR |
+| Pointer FFI, `unsafe` blocks, linking C libraries, and a binding of LLVM's C API | ADR 0048 `:38-39`; decision 2 | left out; backlog | ADR 0048 |
+| Porting `run`, `test`, fmt, the language server, doc, the package manager, and the Cranelift and WASM backends | ADR 0048 `:40-41` | left out: each stays S0's; backlog | ADR 0048 |
+| S1 as the shipped `nova` | ADR 0048 `:42`; decision 5 | left out: `v0.5.0` ships the Rust `nova`; backlog | ADR 0048; 5.6 decision 8 |
+| The byte gate on macOS and Windows | ADR 0048 `:43`; decision 3 | left out: the gate runs on Ubuntu alone; backlog, with an unpinned `cc` and `ld` | ADRs 0048, 0060; 5.6 decision 6 |
+| N built for WASM, which Phase 6's playground needs | ADR 0048 `:44` | Phase 6: its playground, item 5 | ADR 0048 |
+| Trait objects, mutable globals and nested module directories | ADR 0048 `:45` | left out; backlog | ADR 0048 |
+| N on a large-stack thread, run by 5.5's driver | ADR 0055 §12, the user's ruling of 2026-11-06 | not done: N runs on its main thread (1 MB on Windows); backlog | ADR 0055; "Found outside this boundary" |
+| E0093's path on Windows, which "5.5's driver settles" | ADR 0053 `:773-775` | not done: N writes `/` where S0 writes `\`, which ADR 0059 §7 keeps as (differs); backlog | ADRs 0053, 0059; "Found outside this boundary" |
+
+**Status words:**
+- *Built:* in the repository, in `selfhost/`, and held by CI on every pull
+  request, within the limits its record states; not in the release, which
+  ships the Rust `nova` (5.6 decision 8). *Built differently* when its
+  record changed how.
+- *Shipped:* a `v0.5.0` user can rely on it in the released `nova`, within
+  the limits its record states.
+- *Replaced:* something else does its job, as its record says.
+- *Left out:* nothing implements it. *Not done:* an earlier ruling nothing
+  carried out, recorded in "Found outside this boundary".
+- *Met:* a gate's checks passed, as the row and its record state them.
+  *Done* says the same of a step that is not a feature.
+- *Backlog*, *Phase 6:* where a deferred item goes.
+
+### Deviations, each decided by its record
+
+No deviation is decided here. Each was decided in its sub-phase, by its
+ADR and the user's rulings, and the rows cite them; this section names
+them.
+
+#### Item 5: textual LLVM IR through `clang`, not a binding
+
+The master spec's "bind LLVM via `unsafe` extern from Nova" is replaced:
+N writes LLVM IR as text, as S0's `--release` does, and runs `clang` and
+`cc` on it (ADR 0048, decision 3; the emitter, ADR 0058; the driver, ADR
+0059). Pointer FFI and `unsafe` are on the backlog.
+
+#### Item 3: MIR, native only
+
+N's MIR is the native one; WASM's and test mode's stay S0's (ADR 0057
+§6).
+
+#### The gate: on Ubuntu alone, with no tolerance on the bytes
+
+The gate runs on `ubuntu-24.04` alone, macOS and Windows on the backlog
+(5.6 decision 6; ADR 0060 §11), and holds S1 == S2 == S3 with nothing
+tolerated, under ADR 0060 §2's four conditions: one `-o` basename, one
+`clang` 18.1.3, one runtime archive, one `cc` and `ld` within a job.
+
+### Rows whose record already explains them
+
+These rows have no section above:
+- **S1 by S0's `--release`, not Cranelift:** the Cranelift S1 is another
+  binary, so the chain starts from the `--release` one (ADR 0060 §4).
+- **N not shipped:** `v0.5.0` ships the Rust `nova` (decision 5; 5.6
+  decision 8).
+- **Decision 3's `.ll` differential:** it has run since 5.5a in the
+  Selfhost job's main step, on three systems (ADR 0058; ADR 0060 §6).
+- **Decision 7's job:** a step of the Selfhost job (ADR 0060 §11).
+- **`build` refused on Windows:** ADR 0059 §4.
+- **The depth limit, 128,** in both compilers: ADR 0052.
+- **N's speed:** about 6× slower than release S0 (ADR 0052 `:1095-1097`),
+  under the Selfhost job's lines of 80, 100 and 110 s (ADR 0057 §19).
+- **std's item tables and bodies,** checked once per process by S0's
+  shared check, which falls back to the full check where sharing does not
+  apply (ADR 0055 §4; ADR 0056 §4).
+
+### The gate's record
+
+**What runs.** On every pull request and push to `main`, the Selfhost
+job's Linux leg installs `clang-18` 18.1.3, has S1 build the golden suite,
+then runs "The bootstrap, S1 == S2 == S3 (Linux)"
+(`.github/workflows/ci.yml:200-233`): `cc`'s and `ld`'s versions, then
+`the_bootstrap_holds_s1_s2_s3` (`crates/nova-cli/tests/selfhost.rs:8591`),
+which builds S1 by S0's `nova build --release`, S2 by S1 and S3 by S2, in
+order, and holds N's `.ll` equal from S0, S1 and S2, then S1 == S2 == S3,
+byte for byte. There is no tolerance; the four conditions (one `-o`
+basename, one `clang` 18.1.3, one runtime archive, one `cc` and `ld` in a
+job) are ADR 0060 §2's. `selfhost/README.md`'s "S1, S2 and S3 by hand"
+gives the chain as commands.
+
+**`main`'s CI run 45142347493,**
+https://github.com/Sakeerin/nova/actions/runs/45142347493, on 5.6a's last
+commit `ed165975`, on 2026-11-23, which `[0.5.0]`'s lead cites: all 19 jobs
+passed. 5.6a accepted it on that conclusion, its tree being #149's, whose
+run 45139701894 had checked every count; the 5.6b spec read its counts
+(`docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md:211-219`):
+the Test job 2838 passed, 0 failed, 10 ignored on Linux, 2838, 0, 9 on
+macOS, 2815, 0, 9 on Windows; `selfhost.rs` 71, 69 and 65 passed, 3 ignored
+each; the golden step 2 passed. Its bootstrap step printed `bootstrap: S1 ==
+S2 == S3, 9539736 bytes each`, N's `.ll` 12,760,111 bytes from S0, S1 and
+S2, on the runtime `0.4.0-5549317a`, and `1 passed`.
+
+**The release pull request, #151,** whose head has the release commit
+`2842d63c`'s tree (it merged by rebase on 2026-11-24, pinned to that head,
+and `main`'s tree after it is the head's):
+- **CI run 45345843046,**
+  https://github.com/Sakeerin/nova/actions/runs/45345843046, on
+  2026-11-24: all 19 jobs passed, with every count of run 45142347493, and
+  in the Test job `release_manifests` 8, `vscode_extension` 3 and `project`
+  25 passed. `selfhost.rs` took 58.66, 92.34 and 108.43 s, under the job's
+  lines of 80, 100 and 110 s. "Packaged nova" packed the 20 crates at
+  0.5.0, `nova-cli-0.5.0.crate` with 1,759 files, installed `nova` from
+  them (`nova 0.5.0`, `runtime: embedded`) and passed its gate on three
+  systems; the MSRV job checked the workspace with Rust 1.78. The bootstrap
+  step took 29.11 s, under its 75 s line, and printed these lines among
+  others, its stage times left out and the runtime's path shortened:
+
+  ```text
+  cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
+  GNU ld (GNU Binutils for Ubuntu) 2.42
+  bootstrap: S1 == S2 == S3, 9539848 bytes each; N's .ll the same from S0, S1 and S2, 12760111 bytes
+  bootstrap: Ubuntu clang version 18.1.3 (1ubuntu1)
+  bootstrap: runtime …/runtime/0.5.0-4f8a6f1b/libnova_runtime.a
+  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 73 filtered out; finished in 28.98s
+  ```
+
+  S1 is 112 bytes larger than at `ed165975`, on the 0.5.0 runtime; sizes
+  and times are recorded, not expected (ADR 0060 §10). The advisory step
+  failed as on #149 and #150, not counted: Linux's `extern_ffi_run`, issue
+  #3's E0902 on `sqrt`, and three macOS collector tests. Windows'
+  `selfhost.rs`, 1.57 s under its line, is the runner's time (108.49 s in
+  run 45142347493).
+- **Release run 45345843088:** seven jobs passed; "Publish the release" was
+  skipped, as on a pull request. "Prepare the release" found `## [0.5.0]`,
+  wrote `SHA256SUMS` over 8 files and printed `publish runs: gh release
+  create v0.5.0 --verify-tag --title "Nova 0.5.0" --notes-file notes.md
+  dist/*`, with no `--prerelease`.
+
+**`main`'s CI run 45352790982,**
+https://github.com/Sakeerin/nova/actions/runs/45352790982, on `2842d63c`,
+on 2026-11-24: all 19 jobs passed, the bootstrap step among them. It was
+accepted on that conclusion and on its tree, the same as run 45345843046's;
+its counts were not read. The Benchmarks run 45352790526 passed. The
+release's local figures are the 5.6b spec's amendment, item 7.
+
+**The search of N for `Map` iteration, repeated** (ADR 0060 §5, `:217`), on
+`2842d63c`: `git grep -n '\.keys()' -- selfhost/` prints ADR 0060 §5's 17
+lines: `resolution_dump.nova` 54, 316, 378, 381, 384; `typeck_body.nova`
+125; `fixes.nova` 521, 527, 528, 530, 538, 545; `typeck_pattern.nova` 722;
+`typeck_expr.nova` 281; `typeck_ty.nova` 184; `typeck.nova` 879, 1027.
+`selfhost/src` is `83ba8210`'s, the tree ADR 0060 searched.
+
+**The publishes,** from the tag `v0.5.0`, which the user made, annotated,
+on `2842d63c` and pushed on 2026-11-24, its message naming this ADR; each
+step on the user's word:
+- **The Release workflow,**
+  https://github.com/Sakeerin/nova/actions/runs/45373121044, on the tag,
+  passed all eight jobs on 2026-11-24, "Publish the release" among them.
+  Its GitHub release, "Nova 0.5.0", is neither a draft nor a prerelease;
+  its notes are `[0.5.0]` as the release commit has it, 614 lines. Its
+  eight assets are the two install scripts, and the four archives, std's
+  site and the `.vsix`, each named for `0.5.0`, with `SHA256SUMS`; each
+  asset's digest, as GitHub's API gives it, equals its line of `SHA256SUMS`
+  as "Prepare the release" printed it. It is the latest release.
+- **"Publish to crates.io",**
+  https://github.com/Sakeerin/nova/actions/runs/45548505763, from the tag,
+  version `0.5.0`, started on 2026-11-25 and approved by the user under
+  "Review deployments", passed. Its token, the user's, was
+  **publish-update** alone, for `nova-*`; the agent never saw it. It printed
+  `Publishing 0.5.0 from refs/tags/v0.5.0 (2842d63c…)`, `package-gate: 20
+  crates at 0.5.0`, `nova-cli-0.5.0.crate` with 1,759 files, `nova 0.5.0`,
+  `runtime: embedded`, `gate: the WASM build prints Hello, Nova! under
+  Node`, `gate: passed`, `package-gate: passed in 224 s`, and for each of
+  the 20 `on crates.io, owned by` the repository's owner: no crate was new.
+  Each was published at its first attempt, with no rate-limit wait; the
+  summary lists all 20 `published`, none skipped. The index, by the user's
+  command, has a line `"vers":"0.5.0"` for each. The user then revoked the
+  token and deleted the secret.
+- **The extension,** 0.5.0, in both stores on 2026-11-25: the user uploaded
+  `nova-vscode-0.5.0.vsix`. Their screenshots: the Marketplace's listing,
+  version 0.5.0, last updated 11/25/2026, `sakeerin.vscode-nova`, the
+  repository's link; Open VSX's, Nova Language Tools, version 0.5.0 with
+  the alias `latest`, the README, the namespace still unverified. The user
+  confirmed `[0.5.0]` on both and the tokens removed.
+- **"Release gate",**
+  https://github.com/Sakeerin/nova/actions/runs/45596343684, version
+  `0.5.0`, from `main` at `2842d63c`, on 2026-11-25: all six jobs passed,
+  each on the tag. "From crates.io", on Linux, macOS and Windows, ran
+  `cargo install nova-cli --version "=0.5.0" --locked` and printed
+  `Installed package nova-cli v0.5.0`, `nova 0.5.0`, `runtime: embedded`,
+  `packages-gate: passed` and `doc-gate: passed`. "From the install script"
+  printed `nova: checked nova-0.5.0-<target> … against SHA256SUMS` on each,
+  Windows' under PowerShell 7 and Windows PowerShell 5.1. All six ran
+  `gate.sh --test --node`, the flag found by the greps of
+  `.github/workflows/release-gate.yml:83` and `:169` in the tag's `gate.sh`
+  (`:45`), and printed `gate: the WASM build prints Hello, Nova! under
+  Node` and `gate: passed`.
+- **The user's try of the published `nova`,** on 2026-11-26, in place of
+  the runbook's step 6.2 (its note of 2026-11-24), on Windows: `cargo
+  install nova-cli --locked`; `nova --version` printed `nova 0.5.0`; `nova
+  new hello` printed "created `hello`: nova.toml, .gitignore, README.md,
+  src/main.nova", and `nova run`, in `hello`, `Hello, Nova!`. They
+  reported it in their words "ลองแล้ว nova --version ได้ nova 0.5.0 และ
+  Hello, Nova!" ("tried it: nova --version gives nova 0.5.0, and Hello,
+  Nova!"), with two screenshots.
+
+**What went otherwise than the runbook says,** recorded as lessons for its
+next use, not as blame (R3):
+- **Open VSX's token, first try:** the runbook reads it with bash's `read
+  -rs` (step 4, item 2); in Windows PowerShell 5.1, whose `Read-Host` has
+  no `-MaskInput`, it was shown on screen. That try uploaded nothing; the
+  token was revoked, and a new one made and read masked.
+- **The second try** answered "Extension sakeerin.vscode-nova 0.5.0 is
+  already published, but currently isn't active and therefore not
+  visible"; when 0.5.0 reached Open VSX, and why it was not yet active,
+  are not known. It was not retried (`docs/releasing.md:750-753`), and
+  the listing then showed 0.5.0 as `latest`.
+
+### The backlog
+
+The backlog is unscheduled. Each item is built when a program or a user
+needs it, and the list promises no date, no phase and no order.
+
+- **Left out of Phase 5 by ADR 0048** (`:36-45`): pointer FFI, `unsafe`
+  blocks, linking C libraries, and a binding of LLVM's C API; porting
+  `run`, `test`, fmt, the language server, doc, the package manager, and
+  the Cranelift and WASM backends; S1 as the shipped `nova`; the byte
+  gate on macOS and Windows (the next group); trait objects, mutable
+  globals and nested module directories. N built for WASM is mapped to
+  Phase 6 (below).
+- **The bootstrap on macOS and Windows,** with an unpinned `cc` and `ld`
+  (5.6 decision 6; ADR 0060 `:642-643`): N's `build`, refused on Windows
+  (ADR 0059 §4), and N's linking on macOS, which has not run
+  (`selfhost/README.md:66`); a `Map` lint, which ADR 0060 does not add
+  (`:219`).
+- **N's coverage of S0,** the parts that stay S0's alone: WASM's and
+  test mode's MIR (ADR 0057 §6) and code generation; Cranelift in N; an
+  embedded runtime in N; project mode, the library refusal, the registry
+  sync, `--target`, `--ssr`, `--ssg` and `llc`; rendered diagnostics, and
+  `check`'s code and span on every input (ADR 0059 `:740-745`); ADR 0059
+  §7's (differs) list, among it **E0093's `\` on Windows**, which N
+  writes as `/` (R1); the M-codes, registry packages, `nova.lock`, doc
+  tests and `nova test`'s whole-package program (ADR 0054 `:516-521`).
+- **The language and std** (the 5.0 spec's §11; ADRs 0049, 0050): `is`,
+  `if let`, `while let`; record, array, `@` and `Float` patterns; impls
+  on tuples, tuple `Map` keys and structural `==`; a conversion for `?`,
+  a `From` trait and generic traits; `Map::entries` and `channel`
+  returning a tuple; a decision tree for `match`, a full-range match
+  without `_`, a lazy expansion of or-patterns, and flattened tuples
+  (0049); a builder over a runtime byte buffer, and `OrderedSet` (0050);
+  `as` casts, turbofish and `Float.from_bits`; a `Command` builder,
+  asynchronous processes and processes on WASM (0050); `loop`, `while
+  true` known to diverge, `[String; 32]` and `mut` fields.
+- **Speed and robustness** (decision 8): interned string literals,
+  unboxed `Option` and `Result`, and inline bounds checks, each only on
+  the user's word; the lazy splice (ADR 0057); **N on a large-stack
+  thread** (ADR 0055 §12; R1).
+- **S0's bugs, kept:** std's bodies see a program's impls, worked around
+  by the fallback (ADR 0056 `:1220-1221`); float `%` type-checks and
+  fails in `build --release` (ADR 0058 `:885-886`).
+- **Mapped elsewhere, to Phase 6 alone** (R5): N built for WASM, for the
+  playground (`nova-spec/00-MASTER-SPEC.md:940`).
+- **Taken from earlier backlogs:** ADR 0025's `?` operator (`:327`) and
+  `std/process`'s `spawn` and `env` (`:337`) are built, by 5.0a and 5.0c,
+  with a blocking `run` in place of `spawn`; ADR 0047's nested module
+  directories (`:443-445`) stay out, on ADR 0048's list too.
+- **The process:** the root README's two relative links in
+  "Self-hosting", which ship in the release's archives without what they
+  point to. (R3's lessons are a dated note in the runbook's Open VSX
+  step, by R7, not on the backlog.)
+
+### Found outside this boundary
+
+Met while closing Phase 5; recorded, not decided:
+- **ADR 0055's ruling of 2026-11-06** (`:524-527`), that 5.5's driver
+  runs N on a large-stack thread, was not carried out: N runs on its main
+  thread (1 MB on Windows), and `selfhost/src` starts no thread. As ADR
+  0055 measured it on Linux under `ulimit -s 1024` (`:518-524`), N
+  survives 64 linked associated types each nesting 124 arrays (7,936
+  levels), and overflows at 125 (8,000 levels) on the debug runtime, at
+  125 or 126 on the release one; nothing in the corpus, the cases or the
+  fuzzer comes near. It is on the backlog (R1).
+- **ADR 0053's "5.5's driver settles it"** (`:773-775`): N writes `/` in
+  E0093's path on Windows, where S0 writes `\`, and ADR 0059 §7 keeps it
+  as (differs) (`:265`). It is on the backlog (R1).
+- **The 5.6b plan's check of the records:** the plan
+  (`docs/superpowers/plans/2026-11-24-phase-5-6b-the-release-v0.5.0.md:501-507`,
+  `:706-724`) and its spec
+  (`docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md:523-531`)
+  say the check, `specblocks.py` and a grep of the headings, holds "the
+  fresh `[Unreleased]`". It holds each record's text and the headings'
+  order, not each record's place, nor that `[Unreleased]` is empty; both
+  were checked once, on the release commit's tree.
+- **The 5.6b plan cites `.github/workflows/release-gate.yml:75`** for
+  `--node` (its `:1803` and `:2681`): that line is a comment, and the
+  greps that find the flag are `:83` and `:169`.
+- **No 5.6a note in `nova-spec/14-CODEGEN.md`,** which the phase plan's
+  §5 asks for as each sub-phase lands (`docs/phase-5-plan.md:768-771`),
+  as ADR 0047 found for Phase 4 (`:491-497`). **The phase plan's status**
+  still says "approved" (`:3`), as ADR 0037 found of Phase 3's
+  (`:463-464`), and its `:4` says "§1" for the master spec's §0
+  (`nova-spec/00-MASTER-SPEC.md:22`).
+- **ADR 0060 names two commits not on `main`** (`:180`, `:467`,
+  `:487-489`, `:544`), from #148's and #149's branches before the
+  rebase; those pull requests' refs keep them.
+
+## Consequences
+
+- Every item of the master spec's §7 Definition of Done is met:
+  1. the crates compile and pass CI: the release pull request's run
+     45345843046, and `main`'s run 45352790982 on `2842d63c`, recorded in
+     "The gate's record";
+  2. the gate is met with reproducible commands: the Selfhost job's
+     bootstrap step, on every pull request, and `selfhost/README.md`'s
+     "S1, S2 and S3 by hand", which gives the chain as commands;
+  3. the new public surface is documented: `[0.5.0]`'s Added, std's
+     documentation site, `selfhost/README.md`, the root README's
+     "Self-hosting" and the dated notes;
+  4. `CHANGELOG.md`'s `[0.5.0]` holds Phase 5, with its note of
+     2026-11-26;
+  5. ADRs 0048 to 0060 and this one record the deviations;
+  6. the tag `v0.5.0`, on `2842d63c`, is the release's, and its
+     publishes are recorded in "The gate's record".
+
+  **Phase 5 is complete as `v0.5.0`, within this boundary.**
+- What a `v0.5.0` user can rely on is the "shipped" rows, within the
+  limits their records state. N is "built": in the repository and held
+  by CI on every pull request, not in the release.
+- The backlog promises nothing. Phase 6's playground is its one mapping,
+  for N built for WASM.
+- Dated notes point here from:
+  - `CHANGELOG.md`'s `[0.5.0]`, with the publishes' runs;
+  - the master spec's Phase 5 gate;
+  - `docs/phase-5-plan.md`'s 5.6;
+  - `selfhost/README.md`'s records;
+  - `docs/releasing.md`'s step 6.2, which `v0.5.0` replaces, and its
+    Open VSX step, with the lessons above.
+
+  The root README gets none (R6).
+- `[0.5.0]`'s lead keeps "will be recorded", as published, and the
+  published notes are not regenerated: `release.yml` writes them only on
+  a `v*` tag's push, so `[0.5.0]` now differs from the GitHub release's
+  notes by its dated note, as `[0.4.0]` has since `cc829f58`.
+
+## References
+
+- Spec: `docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md`,
+  and its plan,
+  `docs/superpowers/plans/2026-11-26-phase-5-6c-the-records.md`
+- `nova-spec/00-MASTER-SPEC.md` §0, §3 and §7; `docs/phase-5-plan.md`
+- `docs/releasing.md`
+- ADRs 0025, 0037, 0047, and 0048 to 0060
+- Pull requests #128 to #133, #135, #138, #140, #141, #143, #145, #147,
+  #149 and #151; #148 and #150, as ADR 0060 and the 5.6b spec cite them
+- The workflow runs: CI 45142347493, 45345843046 and 45352790982; Release
+  45345843088 and 45373121044; Publish to crates.io 45548505763; Release
+  gate 45596343684
+- `.github/workflows/ci.yml`'s Selfhost job, and
+  `crates/nova-cli/tests/selfhost.rs`
````

<!-- patch t1-notes -->

````diff
diff --git a/CHANGELOG.md b/CHANGELOG.md
index 6f249f3a..ea19a2bc 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -55,6 +55,19 @@ Phase 5.6a, the bootstrap gate (spec
 `docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`;
 ADR 0060).
 
+**Recorded 2026-11-26 (branch
+`claude/loving-archimedes-frtf5q-56b-records`):** `v0.5.0` is published:
+- the tag `v0.5.0` and its GitHub release (run 45373121044);
+- the 20 crates at 0.5.0 on crates.io (run 45548505763);
+- the VS Code extension 0.5.0, in the Marketplace and Open VSX;
+- "Release gate", which passed all six jobs (run 45596343684).
+
+The maintainer installed the published `nova` with `cargo install
+nova-cli --locked` and ran `nova new` and `nova run` with it. The
+boundary Phase 5 closes within is recorded in
+`docs/adr/0061-phase-5-boundary.md`, whose "The gate's record" has each
+run and the try.
+
 ### Added
 
 - **`?` on `Option` and `Result`.** `e?` gives the value inside `Some` or
diff --git a/docs/phase-5-plan.md b/docs/phase-5-plan.md
index acbd376c..a5e73f95 100644
--- a/docs/phase-5-plan.md
+++ b/docs/phase-5-plan.md
@@ -731,6 +731,26 @@ ADR 0060; spec
   - macOS and Windows go to the boundary ADR's backlog;
   - `selfhost/` is not shipped (decision 5).
 
+**Recorded 2026-11-26 (5.6c, branch
+`claude/loving-archimedes-frtf5q-56b-records`, ADR 0061; spec
+`docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md`):**
+5.6's three pull requests are in, and Phase 5 is complete as `v0.5.0`,
+within `docs/adr/0061-phase-5-boundary.md`'s boundary:
+- 5.6a, the gate, is #149;
+- 5.6b, the release, is #151 (spec
+  `docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`):
+  the commit `2842d63c`, which the tag `v0.5.0` names, then the GitHub
+  release, the 20 crates, the extension in both stores and the Release
+  gate, each on the user's word, and the user's try of the published
+  `nova`;
+- 5.6c, the records: ADR 0061, with the inventory, the gate's record and
+  the backlog, macOS and Windows on it, and the dated notes.
+
+ADR 0061 cites ADR 0060 for decision 3's tolerances, and repeats decision
+9's search of N for `Map` iteration: the same 17 lines. Two rulings were
+not carried out and are on its backlog: N on a large-stack thread (ADR
+0055 §12), and E0093's `\` on Windows (ADR 0053).
+
 ## 5. Cross-cutting
 
 - **Testing.**
diff --git a/docs/releasing.md b/docs/releasing.md
index da5516b6..6e747bd8 100644
--- a/docs/releasing.md
+++ b/docs/releasing.md
@@ -415,6 +415,18 @@ grep " nova-vscode-$version.vsix\$" SHA256SUMS | sha256sum --check   # macOS: sh
 
    (`-p <token>` in place of `OVSX_PAT` does the same, and leaves the
    token in the shell's history.)
+
+   **Amended 2026-11-26 (branch
+   `claude/loving-archimedes-frtf5q-56b-records`):** in Windows
+   PowerShell 5.1, whose `Read-Host` has no `-MaskInput`, read the token
+   masked with `$s = Read-Host -AsSecureString "token"` and
+   `$env:OVSX_PAT = [System.Net.NetworkCredential]::new('', $s).Password`,
+   run `npx ovsx publish .\nova-vscode-<version>.vsix` in the `.vsix`'s
+   folder, then `Remove-Item Env:OVSX_PAT; Remove-Variable s`. Open VSX
+   may show a new version as already published but not yet active, for a
+   while after its upload; `ovsx publish` then answers "already published,
+   but currently isn't active": wait for the listing, and do not publish
+   again (`docs/adr/0061-phase-5-boundary.md`).
 3. Check the listings, which can take a few minutes to appear:
    <https://marketplace.visualstudio.com/items?itemName=sakeerin.vscode-nova>
    and <https://open-vsx.org/extension/sakeerin/vscode-nova>. Each shows
@@ -534,7 +546,10 @@ decision 6; `docs/adr/0047-phase-4-boundary.md`).
 the published `nova` and tries it in place of this run, as for
 `v0.4.0` (spec
 `docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`,
-decision 5). `docs/adr/0061-phase-5-boundary.md` will record the try.
+decision 5). `docs/adr/0061-phase-5-boundary.md` records the try, of
+2026-11-26: on Windows, `cargo install nova-cli --locked`, then `nova
+--version` printing `nova 0.5.0`, and `nova new hello` and `nova run`
+printing `Hello, Nova!`.
 
 The commands, in order, each with what it should show:
 
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 710ddc18..9cd4899b 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -913,6 +913,23 @@ does not:
 The release `v0.5.0` and the boundary ADR closing Phase 5 follow in
 pull requests of their own.
 
+**Recorded 2026-11-26 (branch `claude/loving-archimedes-frtf5q-56b-records`):
+Phase 5 is complete as `v0.5.0`, within
+`docs/adr/0061-phase-5-boundary.md`'s boundary.** Its inventory takes
+Phase 5's promises from the list above and `docs/phase-5-plan.md`, and
+says whether each was built, shipped, replaced or left out, and where
+each is recorded. N, the compiler written in Nova, is built in
+`selfhost/` and held by CI on every pull request; it is not shipped, and
+the release ships the Rust `nova`. Item 5 is replaced: N writes LLVM IR
+as text and runs `clang`, with no binding (ADR 0048); item 3's MIR is
+native only (ADR 0057). The gate is met on Ubuntu with no tolerance, as
+ADR 0060 records: stage1 == stage2 == stage3, byte for byte, in `main`'s
+CI run 45142347493 and in the release pull request's run 45345843046.
+macOS and Windows, and the rest of what Phase 5 leaves out, are on that
+ADR's backlog; building the compiler for WASM is mapped to Phase 6's
+playground (Phase 6, item 5). Every item of the §7 Definition of Done is
+met.
+
 ---
 
 ### Phase 6 — 1.0 Release (week 105–120)
diff --git a/selfhost/README.md b/selfhost/README.md
index dd6fd761..5e518f23 100644
--- a/selfhost/README.md
+++ b/selfhost/README.md
@@ -203,6 +203,7 @@ test-mode code generation) changes in Rust alone.
 - ADR 0058: the `.ll` emitter and the `.ll` dump.
 - ADR 0059: the driver, `build` and `check`.
 - ADR 0060: the bootstrap gate and its tolerances.
+- ADR 0061: Phase 5's boundary.
 
 The ADRs are in `docs/adr/`. Phase 5's plan is `docs/phase-5-plan.md`,
 and each sub-phase's spec and plan are under `docs/superpowers/`.
````

### Task 2: Verification, the mutants, the review, the PR, and the merge

Every step runs in a shell set up as the conventions say, at Task 1's
commit, committed, `D` the day ADR 0061's Status carries:
`D=$(sed -n 's/^Accepted (\([0-9-]*\))\. Branch .*/\1/p'
docs/adr/0061-phase-5-boundary.md)`.

- [ ] **Step 1: The records' checks, all of them** (decision 18), each
  command's output kept in a log, each read whole:

  ```sh
  CHK hashcheck --msg "$P/exec56c/msg-t1.txt" > "$P/exec56c/c-hash.txt" 2>&1; echo "exit $?"
  CHK pathcheck > "$P/exec56c/c-path.txt" 2>&1; echo "exit $?"
  CHK privacy --msg "$P/exec56c/msg-t1.txt" > "$P/exec56c/c-privacy.txt" 2>&1; echo "exit $?"
  CHK notes "$D" > "$P/exec56c/c-notes.txt" 2>&1; echo "exit $?"
  CHK files > "$P/exec56c/c-files.txt" 2>&1; echo "exit $?"
  CHK place "$D" > "$P/exec56c/c-place.txt" 2>&1; echo "exit $?"
  CHK split "$D" > "$P/exec56c/c-split.txt" 2>&1; echo "exit $?"
  bash "$P/exec56c/redate.sh" --check "$D" > "$P/exec56c/c-redate.txt" 2>&1; echo "exit $?"
  printf '%s\n' "$(git log -1 --format=%B)" | diff - "$P/exec56c/msg-t1.txt" && echo "the message is the file's"
  tail -n 2 "$P"/exec56c/c-*.txt
  git status --short
  ```

  Expected, `exit 0` each, and:
  - `hashcheck`: one line per word, 30 words: 16 commits on `main` (14 by
    `git merge-base`, 2 by GitHub's read), 9 run IDs and 5 other words
    (three sizes, two CRCs): the 16 commits are `2842d63c`, `cc829f58`,
    `a3a95a2e` (the two by GitHub), `f1f17328`, `9389a84b`, `e13630f0`,
    `1570ddbd`, `25be9f3c`, `e71446df`, `4f32f13d`, `83ba8210`, `1fa7cd3f`,
    `ed165975`, `2d38ad7a`, `e5c555d0` and `2a67c6ce`; no pull request head,
    no pre-rebase hash;
  - `pathcheck`: 55 `path:line` citations, 18 repository paths (this plan's
    among them), 0 Markdown links, 6 run URLs;
  - `privacy`: 573 lines read (the added lines and the message), 2 names
    built; allowed, the owner's login 8 times, in the repository's run URLs
    and the publisher id, and the records' branch 5 times;
  - `notes`, `files` (now `0 untracked`), `place` and `split` as Task 1,
    Steps 3 and 4 give them;
  - `redate.sh --check`: the six places holding `D`, the history as Task
    1, Step 3 gives it, `redate: ok`;
  - `the message is the file's`; `git status --short` prints nothing.
- [ ] **Step 2: The diff, read whole.**

  ```sh
  git diff --stat origin/main HEAD | tail -n 1
  git diff --name-only origin/main HEAD
  git log --format='%h %s' origin/main..HEAD
  git diff --check origin/main HEAD -- . ':(exclude)docs/superpowers'; echo "exit $?"
  git diff --name-only origin/main HEAD -- crates std selfhost/src tests .github tools Cargo.toml Cargo.lock
  git diff origin/main HEAD -- . ':(exclude)docs/superpowers' > "$P/exec56c/records.diff"
  ```

  Expected: 8 files, the six and this plan and the spec; the log, newest
  first, the records commit, the plan's commit and `5d57733d`; `exit 0`
  and nothing printed by `--check`: no added line of the records holds
  trailing whitespace; nothing under the code's paths. Then read
  `records.diff` whole, against spec §3 and its amendment: each of §3.1's
  parts, each note at its place, nothing the spec does not hold.
- [ ] **Step 3: The mutants** (decision 20). One at a time, each edit is
  made to the committed tree, every check runs, and the files are
  restored with `git checkout --`, `git status --short` checked empty
  before and after. The runner does it, and the executor uses it as
  given, writing no runner of its own: it holds the 16 mutants, refuses to
  start if `$P/exec56c/mut/` holds anything, runs every check on the
  unbroken tree first (all must pass), and writes each result to
  `$P/exec56c/mut/<n>.json` with its log beside it. Write it, as it
  stands:

```sh
mkdir -p "$P/exec56c"
cat > "$P/exec56c/mutrun.py" <<'EOF'
#!/usr/bin/env python3
r"""mutrun.py: break the records commit's files one way at a time, on the
checked-out tree (the records commit, committed and clean), and see which
of the records' checks catch each. Run from the repository's root with $P
exported; it writes nothing but $P/exec56c/mut/. Every mutant, in order, in
one run:
  1. `git status --short` must print nothing;
  2. its steps: `redate N` runs `redate.sh N` (N the day after the tree's),
     then each edit: its `old` text must occur exactly once in its file
     ({D} in a text is the tree's day, from ADR 0061's Status; {N} the day
     after it; {P} the scratch directory);
  3. the checks, with D the day ADR 0061's Status then carries: checks.py
     hashcheck and privacy (each with --msg, the commit's message file),
     pathcheck, notes D, files, place D, and redate.sh --check D;
  4. the files restored with `git checkout --`, then `git status --short`
     must print nothing again.
The checks run on the unbroken tree first, and must all pass. A mutant is
caught when a check fails; it is as expected when the checks that failed
are its `expect` list exactly (an empty list: accepted, for the reason
given). Results go to $P/exec56c/mut/<n>.json, each with its log; the run
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
X = f"{P}/exec56c"
OUT = f"{X}/mut"
REPO = os.getcwd()
if not os.path.exists(os.path.join(REPO, ".git")):
    raise SystemExit("run mutrun.py from the repository's root")
ADR = "docs/adr/0061-phase-5-boundary.md"
TOUCH = ["CHANGELOG.md", ADR, "docs/phase-5-plan.md", "docs/releasing.md",
         "nova-spec/00-MASTER-SPEC.md", "selfhost/README.md", "README.md"]
MSG = f"{X}/msg-t1.txt"
CHECKS = [
    ("hashcheck", ["python3", f"{X}/checks.py", "hashcheck", "--msg", MSG]),
    ("pathcheck", ["python3", f"{X}/checks.py", "pathcheck"]),
    ("privacy", ["python3", f"{X}/checks.py", "privacy", "--msg", MSG]),
    ("notes", ["python3", f"{X}/checks.py", "notes", "{D}"]),
    ("files", ["python3", f"{X}/checks.py", "files"]),
    ("place", ["python3", f"{X}/checks.py", "place", "{D}"]),
    ("redate", ["bash", f"{X}/redate.sh", "--check", "{D}"]),
]
END = "  `crates/nova-cli/tests/selfhost.rs`\n"
HEXWORD = r"(?<![0-9A-Za-z_])[0-9a-f]{7,40}(?![0-9A-Za-z_])"
R7 = ("\n   **Amended {D} (branch\n"
      "   `claude/loving-archimedes-frtf5q-56b-records`):** in Windows\n"
      "   PowerShell 5.1, whose `Read-Host` has no `-MaskInput`, read the token\n"
      "   masked with `$s = Read-Host -AsSecureString \"token\"` and\n"
      "   `$env:OVSX_PAT = [System.Net.NetworkCredential]::new('', $s).Password`,\n"
      "   run `npx ovsx publish .\\nova-vscode-<version>.vsix` in the `.vsix`'s\n"
      "   folder, then `Remove-Item Env:OVSX_PAT; Remove-Variable s`. Open VSX\n"
      "   may show a new version as already published but not yet active, for a\n"
      "   while after its upload; `ovsx publish` then answers \"already published,\n"
      "   but currently isn't active\": wait for the listing, and do not publish\n"
      "   again (`docs/adr/0061-phase-5-boundary.md`).\n")
MS = ("**Recorded {D} (branch `claude/loving-archimedes-frtf5q-56b-records`):\n"
      "Phase 5 is complete as `v0.5.0`, within\n"
      "`docs/adr/0061-phase-5-boundary.md`'s boundary.** Its inventory takes\n"
      "Phase 5's promises from the list above and `docs/phase-5-plan.md`, and\n"
      "says whether each was built, shipped, replaced or left out, and where\n"
      "each is recorded. N, the compiler written in Nova, is built in\n"
      "`selfhost/` and held by CI on every pull request; it is not shipped, and\n"
      "the release ships the Rust `nova`. Item 5 is replaced: N writes LLVM IR\n"
      "as text and runs `clang`, with no binding (ADR 0048); item 3's MIR is\n"
      "native only (ADR 0057). The gate is met on Ubuntu with no tolerance, as\n"
      "ADR 0060 records: stage1 == stage2 == stage3, byte for byte, in `main`'s\n"
      "CI run 45142347493 and in the release pull request's run 45345843046.\n"
      "macOS and Windows, and the rest of what Phase 5 leaves out, are on that\n"
      "ADR's backlog; building the compiler for WASM is mapped to Phase 6's\n"
      "playground (Phase 6, item 5). Every item of the §7 Definition of Done is\n"
      "met.\n\n")
REVIEW = ("records the review holds: `place` holds only spec §3.1's three sample rows verbatim, so a status "
          "changed in one of them is caught, and the other 39 rows' statuses, the parser's among them, are the "
          "review's (spec §5)")
MUTANTS = [
    dict(key="H1", title="a commit not on `main`, #151's head before the rebase, named in ADR 0061",
         steps=[(ADR, "`2842d63c`'s tree (it merged by rebase", "`5b4440ad`'s tree (it merged by rebase")],
         expect=["hashcheck"]),
    dict(key="P1", title="a `path:line` one line off", steps=[
        (ADR, "`crates/nova-cli/tests/selfhost.rs:8591`", "`crates/nova-cli/tests/selfhost.rs:8592`")],
         expect=["pathcheck"]),
    dict(key="P2", title="the plan's path in References misspelt", steps=[
        (ADR, "`docs/superpowers/plans/2026-11-26-phase-5-6c-the-records.md`",
         "`docs/superpowers/plans/2026-11-26-phase-5-6c-the-record.md`")], expect=["pathcheck"]),
    dict(key="S1", title="a scratch path added to References", steps=[
        (ADR, END, END + "- The executor's notes: `{P}/exec56c/notes.md`\n")],
         expect=["privacy"] + (["hashcheck"] if re.search(HEXWORD, P) else []),
         note="its hex parts, if the scratch directory's name has any, are hashcheck's too"),
    dict(key="S2", title="an email added to References", steps=[
        (ADR, END, END + "- Asked by: " + "release" + "@" + "example.org" + "\n")], expect=["privacy"]),
    dict(key="S3", title="a prototype branch named in References", steps=[
        (ADR, END, END + "- The prototype: `" + "pro" + "to/5-6c`\n")], expect=["privacy"]),
    dict(key="N1", title="`[0.5.0]`'s lead changed: \"will be recorded\" made \"is recorded\"", steps=[
        ("CHANGELOG.md", "Phase 5 closes within will be recorded in\n", "Phase 5 closes within is recorded in\n")],
         expect=["notes"]),
    dict(key="N2", title="the `## [0.5.0]` heading's brackets lost", steps=[
        ("CHANGELOG.md", "## [0.5.0] - 2026-11-24\n", "## 0.5.0 - 2026-11-24\n")],
         expect=["notes", "place", "redate"]),
    dict(key="N3", title="the `## [0.5.0]` heading re-dated to the commit's day", steps=[
        ("CHANGELOG.md", "## [0.5.0] - 2026-11-24\n", "## [0.5.0] - {D}\n")],
         expect=["redate"], note="the notes are unchanged: release-notes.sh prints no heading"),
    dict(key="F1", title="a seventh file: the root README pointing to ADR 0061", steps=[
        ("README.md", "`selfhost/` holds N, Nova's compiler written in Nova. It is not shipped:\n",
         "ADR 0061 records Phase 5's boundary.\n\n"
         "`selfhost/` holds N, Nova's compiler written in Nova. It is not shipped:\n")], expect=["files"]),
    dict(key="R7", title="the runbook's Open VSX note (R7) left out", steps=[("docs/releasing.md", R7, "")],
         expect=["pathcheck", "place", "redate"]),
    dict(key="M1", title="the master spec's note left out", steps=[("nova-spec/00-MASTER-SPEC.md", MS, "")],
         expect=["pathcheck", "files", "place", "redate"],
         note="the note was the file's only change, so files finds it unchanged"),
    dict(key="M2", title="`selfhost/README.md`'s line left out", steps=[
        ("selfhost/README.md", "- ADR 0061: Phase 5's boundary.\n", "")], expect=["files", "place"]),
    dict(key="D1", title="a stale date: redate.sh to the next day, the master spec's note left at the old",
         steps=["redate", ("nova-spec/00-MASTER-SPEC.md", "**Recorded {N} (branch", "**Recorded {D} (branch")],
         expect=["place", "redate"]),
    dict(key="D2", title="a history date moved: redate.sh to the next day, the try's date in ADR 0061 with it",
         steps=["redate", (ADR, "of the published `nova`,** on 2026-11-26, in place of",
                           "of the published `nova`,** on {N}, in place of")], expect=["redate"]),
    dict(key="A1", title="an inventory row's status changed: the parser \"shipped\", not \"built\"", steps=[
        (ADR, "| Port the parser to Nova | item 2; 5.1b | built: ", "| Port the parser to Nova | item 2; 5.1b | shipped: ")],
         expect=[], accept=REVIEW),
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

  Then the disk check (`Task 2 Step 3, before the mutants`, at least 6,144
  MiB), and from the repository's root, with a tool timeout of 300000 (the
  run takes about 100 s here; `timeout 280` stops it inside the tool's
  limit), its output in a log read after it:

  ```sh
  time timeout 280 python3 "$P/exec56c/mutrun.py" > "$P/exec56c/mutrun.txt" 2>&1; echo "exit $?"
  cat "$P/exec56c/mutrun.txt"
  git status --short
  ```

  Expected: `exit 0`; `the records commit's day: D; the next day: …`,
  `baseline: every check passes`, one line per mutant, as below, then
  `mutants: 16, caught 15, not caught 1 (accepted), as expected 16 of
  16`; `git status --short` empty. A runner stopped by `timeout` exits
  124, and one that finds a mutant not as expected exits 1: either stops
  the step. The mutants and what each must show, with the first line each
  failing check printed in the replay, where `D` was 2026-11-26 and `N`
  2026-11-27:

  1. **H1: A commit not on `main`, #151's head before the rebase, named in ADR 0061.** ADR 0061, "The gate's record": `` `2842d63c`'s tree `` becomes `` `5b4440ad`'s tree ``, `5b4440ad` being #151's head before the rebase, on no branch of `main`.
     - Expected: caught by `hashcheck` (`5b4440ad docs/adr/0061-phase-5-boundary.md FAIL: commit 5b4440ad2edf is not on origin/main (if it …`).
  2. **P1: A `path:line` one line off.** ADR 0061, "The gate's record": `crates/nova-cli/tests/selfhost.rs:8591` becomes `:8592`.
     - Expected: caught by `pathcheck` (`` FAIL ADR:216 `crates/nova-cli/tests/selfhost.rs:8591`: not on that line ``).
  3. **P2: The plan's path in References misspelt.** ADR 0061's References: the plan's path, `…-the-records.md`, becomes `…-the-record.md`.
     - Expected: caught by `pathcheck` (`FAIL path docs/superpowers/plans/2026-11-26-phase-5-6c-the-record.md: not in the tree`).
  4. **S1: A scratch path added to References.** ADR 0061's References gain a line naming `$P/exec56c/notes.md`, with `$P` written out.
     - Expected: caught by `privacy` (`FAIL docs/adr/0061-phase-5-boundary.md: a local or scratch path`); `hashcheck` (`<a word of $P> docs/adr/0061-phase-5-boundary.md FAIL: neither a commit here nor a word this table …`). Its hex parts, if the scratch directory's name has any, are hashcheck's too.
  5. **S2: An email added to References.** ADR 0061's References gain a line holding an email address, which the runner builds.
     - Expected: caught by `privacy` (`FAIL docs/adr/0061-phase-5-boundary.md: an email`).
  6. **S3: A prototype branch named in References.** ADR 0061's References gain a line naming a prototype's branch, which the runner builds.
     - Expected: caught by `privacy` (`FAIL docs/adr/0061-phase-5-boundary.md: a branch not the records'`).
  7. **N1: `[0.5.0]`'s lead changed: "will be recorded" made "is recorded".** `CHANGELOG.md`'s lead: "Phase 5 closes within will be recorded in" becomes "… is recorded in" (spec decision 18 keeps it).
     - Expected: caught by `notes` (`the difference: NOT the note alone`).
  8. **N2: The `## [0.5.0]` heading's brackets lost.** `CHANGELOG.md`: `## [0.5.0] - 2026-11-24` becomes `## 0.5.0 - 2026-11-24`.
     - Expected: caught by `notes` (`release-notes.sh 0.5.0 CHANGELOG.md: exit 1 release-notes: CHANGELOG.md has no notes for 0.5.0`); `place` (`FAIL the CHANGELOG note (spec 340-351): at [58]`); `redate` (`redate: history in CHANGELOG.md: 2026-11-24 1, expected {}`).
  9. **N3: The `## [0.5.0]` heading re-dated to the commit's day.** `CHANGELOG.md`: `## [0.5.0] - 2026-11-24` becomes `## [0.5.0] - D`.
     - Expected: caught by `redate` (`redate: history in CHANGELOG.md: 2026-11-26 1, expected {}`). The notes are unchanged: release-notes.sh prints no heading.
  10. **F1: A seventh file: the root README pointing to ADR 0061.** The root `README.md` gains "ADR 0061 records Phase 5's boundary." above `selfhost/`'s paragraph (R6: none there).
      - Expected: caught by `files` (`FAIL: README.md changed or untracked, not one of the six`).
  11. **R7: The runbook's Open VSX note (R7) left out.** `docs/releasing.md`: the Open VSX note (11 lines) and the blank line before it go.
      - Expected: caught by `pathcheck` (`` FAIL ADR:349 `docs/releasing.md:750-753` -> docs/releasing.md:750-753 missing ['**The extension** … ``); `place` (`FAIL the runbook's Open VSX note (spec 431-441, as amended): at []`); `redate` (`redate: the runbook's Open VSX note (docs/releasing.md): NOT FOUND`).
  12. **M1: The master spec's note left out.** `nova-spec/00-MASTER-SPEC.md`: the note (16 lines) and the blank line after it go.
      - Expected: caught by `pathcheck` (`` FAIL ADR:395 `nova-spec/00-MASTER-SPEC.md:940` -> nova-spec/00-MASTER-SPEC.md:940-940 missing ['5. … ``); `files` (`FAIL: nova-spec/00-MASTER-SPEC.md unchanged`); `place` (`FAIL the master spec note (spec 364-379): at []`); `redate` (`redate: the master spec note (nova-spec/00-MASTER-SPEC.md): NOT FOUND`). The note was the file's only change, so files finds it unchanged.
  13. **M2: `selfhost/README.md`'s line left out.** `selfhost/README.md`: `- ADR 0061: Phase 5's boundary.` goes.
      - Expected: caught by `files` (`FAIL: selfhost/README.md unchanged`); `place` (`FAIL the selfhost/README.md line (spec 413): at []`).
  14. **D1: A stale date: redate.sh to the next day, the master spec's note left at the old.** `redate.sh N`, `N` the day after `D`, then the master spec's `**Recorded N (branch` set back to `**Recorded D (branch`.
      - Expected: caught by `place` (`FAIL the master spec note (spec 364-379): at []`); `redate` (`redate: the master spec note (nova-spec/00-MASTER-SPEC.md): 2026-11-26`).
  15. **D2: A history date moved: redate.sh to the next day, the try's date in ADR 0061 with it.** `redate.sh N`, then the try's day in ADR 0061, `on 2026-11-26, in place of`, made `on N, in place of`.
      - Expected: caught by `redate` (`redate: history in docs/adr/0061-phase-5-boundary.md: 2026-10-27 1, 2026-11-06 2, 2026-11-22 2, …`).
  16. **A1: An inventory row's status changed: the parser "shipped", not "built".** ADR 0061's inventory: the parser's row's `| built: …` becomes `| shipped: …`.
      - Expected: not caught, every check passing; accepted as records the review holds: `place` holds only spec §3.1's three sample rows verbatim, so a status changed in one of them is caught, and the other 39 rows' statuses, the parser's among them, are the review's (spec §5).

  If the run is cut off: `git checkout -- .`, `git status --short` empty,
  `rm -rf "${P:?}/exec56c/mut-cut" && mv "$P/exec56c/mut"
  "$P/exec56c/mut-cut"`, then the run again from the first. All 16
  results come from one run, at one tip.
- [ ] **Step 4: The review's inputs.** Step 1's logs, Step 2's diff and
  its checks, and the mutants, each against its Expected, go in the
  ledger.
- [ ] **Step 5: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer, a fresh read-only agent, reads `git diff $BASE..HEAD`
    (Task 0) and this plan's commit against the spec and its amendment,
    the release's ledger and CI's summaries of the release's runs, which
    the main session hands it, and the user's decisions, and is told to
    hunt for:
    - a claim in ADR 0061 or a note that says more than was checked, or
      other than the ledger and the runs say (Review Focus 2): each run's
      number, date, jobs and lines; what the user did and said; what was
      accepted on a conclusion and not read;
    - a hash not on `main`, a `path:line` that does not hold what the text
      says of it, a record of a pull request head or a pre-rebase commit
      (Review Focus 1);
    - private text: a token, a login, a local time or folder, a person's
      name, a model ID or a scratch path (Review Focus 3);
    - the date: the six places the commit's day, the history's days as
      they were (Review Focus 4);
    - `[0.5.0]` against the published notes: the dated note alone
      (Review Focus 5); each note at its place and in its form (Review
      Focus 6);
    - the inventory against the master spec's list, the phase plan's
      decisions and sub-phases, and ADR 0048's list; the backlog against
      each record it cites; "Found outside this boundary" against the
      5.6b plan and spec and ADRs 0053, 0055, 0059 and 0060;
    - the mutants (Step 3): whether each is caught by what should catch
      it, and any breakage of the records the checks would not catch;
    - the plan's remaining steps: the PR, CI's Expected, the merge and
      Task 3.
  - Each finding is either fixed, or answered in the ledger with the
    reason it stands. The fixes are folded into the records commit before
    anything is pushed (decision 23), the files changed in the working
    tree first:
    - **on the commit's own day** (`TZ=UTC git log -1
      --date=format-local:%Y-%m-%d --format=%ad` prints today's `date -u
      +%F`): `git add -A`, `git status --short` listing only the changed
      files, then `git commit --amend --no-edit`, which keeps the date and
      the message;
    - **on another day:** the re-date of Task 1, Step 4 (decision 17),
      `redate.sh` with the new day, `git add -A`, `git status --short`
      listing the changed files and the dated ones, then `git commit
      --amend --date=now --no-edit`;
    - then Task 1, Step 4's checks again from its `D=` line: `dated D`,
      the trailers, and `CHK split "$D"`, which now prints `DIFFERS` for
      each file a fix changed, and only for those: the ledger names them.
  - **A fix that adds or removes a line of ADR 0061 moves `pathcheck`'s
    keys.** Its table, `CITES`, is keyed by the ADR's line, so every
    citation after the fix's line gets a new key. Then, from the
    repository's root:

    ```sh
    CHK renumber
    CHK pathcheck > "$P/exec56c/c-path.txt" 2>&1; echo "exit $?"
    ```

    `renumber` rewrites the keys in `$P/exec56c/checks.py` from the ADR
    in the tree, in the citations' order, printing each key it moves, and
    `pathcheck` checks again that each row's lines still hold its words.
    If the fix added, removed or changed a citation, `renumber` refuses
    (`renumber: FAILED`), writing nothing: add, remove or change that row
    by hand, from the citation's file at the tip, then run both again.
  - **A fix that changes a date `redate.sh` counts, or a text a mutant
    edits,** is followed in the script by the edit it needs: the
    history's count, the mutant's `old` text. Each change to a script is
    recorded in the ledger as a ruling with its cost if wrong. A fix that
    makes a note no longer hold its draft (spec §3; for the runbook's
    Open VSX note, the amendment's item 6) stops the work: the draft or
    the fix is wrong, and the user is asked.
  - Then Steps 1 to 3 again, all of them: every check and every mutant,
    after `rm -rf "${P:?}/exec56c/mut-before-fixes" && mv "$P/exec56c/mut"
    "$P/exec56c/mut-before-fixes"`. Expected: as written, but for what the
    ledger records. If no fix changed a file, Steps 1 to 3's results
    stand, and the ledger says so.
- [ ] **Step 6: The PR.**
  - **Stop here, and have the main session ask the user** whether to push
    the branch and open the PR (Global Constraints). No push of this
    branch has been allowed. Nothing is pushed, and no PR opened, before
    their word; record their answer in the ledger, in their words.
  - Before asking, check that every commit the PR would carry ends with
    the two trailer lines of the Conventions: `git log --format=%B
    origin/main..HEAD`, its commits `5d57733d` (the spec's), the plan's
    and the records commit, last.
  - On the user's word, the main session pushes the branch with git, `git
    push -u origin claude/loving-archimedes-frtf5q-56b-records`, retried
    with backoff on a network error, as the session's rules say (no GitHub
    MCP tool pushes commits; a push refused by a permission or safety rule
    stops the work, and the user is asked), and records the pushed head.
    Then it opens the PR with the GitHub MCP tool `create_pull_request`, to
    `main`, not as a draft, titled **"Phase 5.6c: the records, ADR 0061"**,
    with the repository's template headings
    (`.github/PULL_REQUEST_TEMPLATE.md`: `## Summary`, `## Changes`, `##
    Testing`, `## Phase Gate Criteria`). The body names no local path, no
    person and no model ID. It ends with this line, a blank line, and the
    session's address, exactly:

    ```
    🤖 Generated with [Claude Code](https://claude.com/claude-code)

    https://claude.ai/code/session_01VzjWoMjudJFFEx1oAg4fTN
    ```

    If the server replaces that ending with its own footer (it did on
    #149, #150 and #151), send the body once more with the ending, and
    check it.
  - The body holds:
    - **Summary:** Phase 5's records, the third of 5.6's pull requests,
      after the release and the user's try: ADR 0061, Phase 5's boundary,
      and five dated notes pointing to it; documentation only, no code,
      test, workflow or version change; on its merge, Phase 5 is closed
      within the boundary (spec §12). Its commits: `5d57733d`, the spec's;
      the plan's, with the spec's amendment; the records commit, last.
    - **Changes:** ADR 0061's parts (the inventory, 42 rows; the
      deviations, each decided by its record; "The gate's record", every
      run of the release by number, the publishes and the user's try; the
      backlog in nine groups, macOS and Windows on it; what was found
      outside the boundary); the CHANGELOG's dated note, with the
      publishes' run IDs; the master spec's note under the Phase 5 gate;
      the phase plan's note on 5.6; `selfhost/README.md`'s line; the
      runbook's step 6.2 sentence and its Open VSX note (R7); the date
      (decision 17); the records name `main`'s commits only.
    - **Testing,** the template's boxes: `cargo test --workspace`, `--test
      selfhost`, `cargo fmt` and both clippy commands ticked, each "no
      Rust, test or workflow file changes; CI's run on this pull request
      runs it" (decision 22); "New tests added" not ticked: none, as no
      test reads these files (spec §5), and the plan's checks hold them;
      "CHANGELOG.md updated" ticked. Then the plan's checks and their
      figures (Step 1), `release-notes.sh 0.5.0`'s 627 lines, the released
      notes' 614 with the note, and the mutants, 16, 15 caught and one
      accepted as records the review holds; the review's findings and what
      came of each; the user's decisions and this plan's, numbered as
      here, decision 27 marked as differing and recorded in the spec's
      amendment.
    - **Phase Gate Criteria:** spec §12's gate, item by item, and which
      this pull request meets on its merge; `main`'s CI and the checks on
      `main` are Task 3's.
- [ ] **Step 7: CI.** **Wait for CI, then read it** (Conventions), with no
  foreground `sleep`: once the pull request is open, the main session
  subscribes to its activity (`subscribe_pr_activity`), and CI's results
  wake the session. When the CI run has ended, a read-only helper agent
  fetches each job's log whole with `get_job_logs` into
  `$P/exec56c/ci/` and writes `$P/exec56c/ci-<run>.txt` (Conventions). It
  writes nothing to GitHub. **Expected,** checked against that summary,
  from #151's run 45345843046 (spec §4; decision 24):
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
    (on #151's run, Linux's `extern_ffi_run`, issue #3's E0902 on `sqrt`,
    and three of macOS's collector tests failed there, as on #149 and
    #150);
  - **the Selfhost job:** `selfhost.rs` `71 passed; 0 failed; 3 ignored`
    on Linux, `69`, `3` on macOS, `65`, `3` on Windows, under the 80, 100
    and 110 s lines (#151's run: 58.66, 92.34 and 108.43 s); on Linux
    alone the golden step `2 passed; 0 failed; 0 ignored; 0 measured; 72
    filtered out`, the bootstrap step (`.github/workflows/ci.yml:225-233`)
    printing `cc`'s and `ld`'s versions, `bootstrap: S1 == S2 == S3, …`,
    its runtime line under the gate's `home/runtime/0.5.0-<crc>/`, and `1
    passed; 0 failed; 0 ignored; 0 measured; 73 filtered out` (a `0
    passed` there means the step runs no test: stop and report), then
    `the_selfhost_tests_pass` `1 passed`; on macOS and Windows the clang
    steps skipped. Recorded, not expected: S1's size (9,539,848 on #151's
    run), the runtime's directory (`0.5.0-4f8a6f1b` there) and the step's
    time (29.11 s there, the test 28.98 s, under its 75 s line);
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
  than 75 s,** ask the user (spec §9, risk 5): CI does not enforce the
  lines.
  - **After the push, nothing is amended or force-pushed without the
    user's word** (decision 23). If CI, the review or anything else needs
    a change to the pushed branch, the executor stops, records why, and
    the main session asks the user, offering decision 23's two ways. Each
    push after the first is on the user's word for it, and Steps 1 to 7
    run again on the new head as far as the change reaches.
- [ ] **Step 8: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit (`expectedHeadSha`). Never
    auto-merge.
  - Then the main session runs `git fetch origin main`, `git diff <head>
    origin/main --stat`, which prints nothing: the merged tree is the
    PR's; and `git log -3 --format='%H %s' origin/main`, which prints,
    newest first, the records commit, the plan's and the spec's, each
    with a new hash: the rebase gives each commit one. Record `main`'s
    three hashes in the ledger. None of them is in the records, which cite
    no commit of their own pull request (spec decision 21).
  - **If `git fetch origin main` is refused** (by a permission or safety
    rule, or a network error after the retries), nothing works around it
    but this, read only, with the GitHub MCP tools: `list_commits`
    (`sha: main`, `perPage: 3`) gives `main`'s three new commits, newest
    first, the records commit's `sha` the tip; `get_commit` (`sha: main`)
    gives no tree (its output has the commit's `sha`, message, author,
    committer, stats and files, but no `tree`: checked on a real call,
    2026-11-26), so the tree is compared by the root's entries:
    `get_file_contents` (`path: /`, `sha:` the tip, `fields: [name, type,
    sha]`) lists each top-level entry with its hash, and each must equal
    `git ls-tree <the pushed head>`'s line for that name, the same names
    on both sides (on 2026-11-26, `main`'s 25 entries equalled `git
    ls-tree 2842d63c`'s). Equal names, types and hashes are equal trees
    but for a top-level file's mode, which the API does not give. Record the
    refusal and the comparison in the ledger; `origin/main` stays
    `2842d63c`, which Task 3 allows for.

---

## After the merge

### Task 3: `main`'s CI, the checks on `main`, and Phase 5 closed

Spec §12; decision 25. The main session does each step itself, through its
GitHub MCP tools and git (no `gh`, no `curl`), and needs no word for its
reads and checks; what it records goes in the ledger. A run is waited for as
the Conventions give it, and its logs, if they are read, by the read-only
helper's summary.

- [ ] **Step 1: `main`'s CI on the merged commits.** `actions_list`
  (`method: list_workflow_runs`, `resource_id: ci.yml`,
  `workflow_runs_filter: {branch: main, event: push}`) finds the run whose
  `head_sha` is `main`'s records commit (Task 2, Step 8); one scheduled
  check-in (`send_later`) about 12 minutes on, then `actions_get` (`method:
  get_workflow_run`). **Accepted** when its `status` is `completed` and
  its `conclusion` `success`, all 19 jobs `success` (`list_workflow_jobs`),
  and its head commit's tree is the pull request's last head's: `git
  rev-parse <head_sha>^{tree}` and `git rev-parse <PR head>^{tree}` print
  the same (after Task 2, Step 8's fetch), or, if that fetch was refused,
  Task 2, Step 8's comparison of the root's entries, with `sha:` the
  run's `head_sha`, holds. Its counts are read, by the
  helper's summary, only if a job fails or the trees differ, and then the
  user is asked. The Benchmarks run on the same push is recorded with its
  conclusion. Record both runs' addresses.
- [ ] **Step 2: The checks on `main`'s tree.** On the branch, whose tree is
  `main`'s (Step 1), `origin/main` now the merged records commit, in a
  shell set up as the conventions say, `D` as Task 2 gives it. (If Task 2,
  Step 8's fetch was refused, `origin/main` is still `2842d63c`: the first
  line below then prints nothing, and Step 1's comparison of the root's
  entries stands in for it; `hashcheck` still places every hash the
  records name, all of them on `2842d63c`'s history.)

  ```sh
  git diff --quiet HEAD origin/main && echo "the same tree as origin/main"
  CHK hashcheck > "$P/exec56c/m-hash.txt" 2>&1; echo "exit $?"
  CHK pathcheck > "$P/exec56c/m-path.txt" 2>&1; echo "exit $?"
  CHK privacy > "$P/exec56c/m-privacy.txt" 2>&1; echo "exit $?"
  CHK notes "$D" > "$P/exec56c/m-notes.txt" 2>&1; echo "exit $?"
  CHK files > "$P/exec56c/m-files.txt" 2>&1; echo "exit $?"
  CHK place "$D" > "$P/exec56c/m-place.txt" 2>&1; echo "exit $?"
  tail -n 2 "$P"/exec56c/m-*.txt
  git status --short
  ```

  Expected: `the same tree as origin/main` (the rebase gave the commits
  new hashes, and kept the tree). Then `exit 0` for each check, with Task
  2, Step 1's figures (`hashcheck` without the message's run IDs, which
  the records also name; `privacy` 559 lines read, the added lines alone,
  without the message's 14, with the same names built and the same
  allowed counts); `git status --short` empty. Then the two older
  commits once more, read only: `list_commits` (`sha` `a71d27d9…`, the
  shallow boundary, `perPage` 22) lists `a3a95a2e` and `cc829f58`, and
  `a71d27d9` is on `origin/main` (Task 0, Step 1's check). So every hash
  the new text names is on `main`, every `path:line` resolves at the
  merged commit, and `release-notes.sh 0.5.0 CHANGELOG.md` exits 0 there.
- [ ] **Step 3: Spec §12's gate,** item by item against the ledger:
  1. the records pull request merged by rebase on the user's word (Task
     2, Step 8);
  2. its CI green with §4's counts (Task 2, Step 7);
  3. `main`'s CI green on the merged commit (Step 1);
  4. ADR 0061 and the notes of §3.2 to §3.5 on `main` (Step 2's `files`
     and `place`);
  5. every hash the new text names on `main`, and every `path:line`
     resolving at the merged commit (Step 2's `hashcheck` and
     `pathcheck`);
  6. `release-notes.sh 0.5.0 CHANGELOG.md` exiting 0 there (Step 2's
     `notes`).

  Each met, **Phase 5 is closed.** The main session tells the user, and
  records it in the ledger with the date. Phase 6 waits for the user's
  word. Deleting branches (`-56b`, `-56b-ci`, the earlier Phase 5 ones,
  `-56b-records`) is the user's (spec §10).
