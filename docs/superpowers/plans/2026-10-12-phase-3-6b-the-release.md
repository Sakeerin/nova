# Phase 3.6b, "The release", Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `v0.3.0` is published and the Phase 3 gate is met on it. A
release pull request bumps the version and says how to install `nova`;
then the runbook's steps run, each on the user's word; then a close-out
pull request records the gate and closes Phase 3.

**Spec:** `docs/superpowers/specs/2026-10-12-phase-3-6b-the-release-design.md`,
approved by the user on 2026-10-12. Its §13 lists the decisions; its
decision 5 approves the release commit's change to `Cargo.lock` (the 23
workspace packages' versions only).

**Runbook:** `docs/releasing.md`. This plan does not repeat its steps; it
says which of them the agent runs, with what, and what it checks.

## Global Constraints

- **Every step marked [stop] in the runbook, and every row marked stop in
  the spec's §4, waits for the user's explicit word in this
  conversation.** A word given for one step does not cover the next.
- **`Cargo.lock` changes only in Task 1, by `cargo update --workspace`:**
  23 lines, the workspace's own packages. Any other change stops the
  task.
- **MSRV 1.78**; no new crate; no code change but the extension's error
  string (Task 3).
- **Tests never touch the internet.** No test is added.
- **No secret is read, printed or stored by the agent.** The crates.io
  token, the store tokens and the index token are the user's, typed by
  the user into GitHub or their own terminal.
- **Merge by rebase, on the user's word; never enable auto-merge.** PR
  bodies hold no local paths and no personal name (`Sakeerin` only in
  repository URLs, `sakeerin` only as the publisher ID).
- **This machine is Linux;** Windows and macOS results come from CI.

## Conventions

As 3.6a's: one commit per task, the message ending with the session's
two attribution lines; `cargo fmt --all -- --check` and the named tests
before each commit; a difference from an `Expected:` is fixed in the
smallest way and recorded in the ledger
(`$P/ledger-36b.md`, `$P` the session's scratch directory) as a ruling
with its cost if wrong.

## File map

| File | Task |
|---|---|
| `Cargo.toml`, `Cargo.lock`, `tools/vscode-nova/package.json`, `tools/vscode-nova/package-lock.json` | 1 |
| `CHANGELOG.md`, `tools/vscode-nova/CHANGELOG.md` | 2 |
| `README.md`, `tools/vscode-nova/README.md`, `tools/vscode-nova/src/extension.ts` | 3 |
| `docs/releasing.md` | 4 |
| `docs/adr/0037-phase-3-boundary.md`, `docs/adr/0031-package-index-and-publishing.md`, dated notes | 7 |

---

### Task 0: Baseline

`main` at `59b0d53`, the tree PR #111 merged, gave on Linux 2022 passed,
1 failed (`nova-runtime net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`,
IPv6 missing here), 10 ignored (3.6a's fix pass, `$P/fix36a/`). Nothing to
run.

### Task 1: The version

**Files:** `Cargo.toml`, `Cargo.lock`, `tools/vscode-nova/package.json`,
`tools/vscode-nova/package-lock.json`.

- [ ] **Step 1: The runbook's commands** (its step 1, item 1), from the
  repository's root, with `version=0.3.0` and `old=0.2.0`:

  ```sh
  perl -pi -e "s/^version = \"\Q$old\E\"\$/version = \"$version\"/; s/version = \"=\Q$old\E\"/version = \"=$version\"/" Cargo.toml
  perl -pi -e "s/^  \"version\": \"\Q$old\E\",\$/  \"version\": \"$version\",/" tools/vscode-nova/package.json
  perl -0pi -e "s/(\"name\": \"nova\",\n\s*\"version\": )\"\Q$old\E\"/\$1\"$version\"/g" tools/vscode-nova/package-lock.json
  cargo update --workspace --offline
  git diff --stat
  ```

  Expected: `Cargo.toml` 20 lines changed (the workspace's `version` and
  19 pins), `Cargo.lock` 23, `package.json` 1, `package-lock.json` 2;
  nothing else. `git diff Cargo.lock` shows only `version = "0.2.0"` →
  `"0.3.0"` lines under `name = "nova-…"` entries.

- [ ] **Step 2: Run**

  `cargo test --locked -p nova-cli --test release_manifests --test vscode_extension --test project`

  Expected: 7, 2 and 25 passed. (`project.rs` reads `nova --version`
  through `CARGO_PKG_VERSION`, so it follows.)

- [ ] **Step 3: Commit** `release: the workspace's version is 0.3.0`.

### Task 2: The CHANGELOGs

**Files:** `CHANGELOG.md`, `tools/vscode-nova/CHANGELOG.md`.

- [ ] **Step 1:** In `CHANGELOG.md`, `## [Unreleased]` becomes
  `## [0.3.0] - 2026-10-12` (the day the PR merges; if it merges later,
  that day), and above it a fresh, empty `## [Unreleased]` followed by a
  blank line (the file has no separator between version sections; its
  one `---` is under the header). Under the new heading,
  before `### Added`, the lead paragraph (spec §3 item 2, decision 6), in
  the voice of `[0.2.0]`'s:

  ```markdown
  **Phase 3, "Tooling", is released.** This is the `v0.{phase}.0`
  milestone `nova-spec/00-MASTER-SPEC.md` §7 reserves for a phase:
  `nova fmt`, `nova lsp` and the VS Code extension, packages with
  `nova.toml`, `nova add` and a git-backed package index, `nova doc` and
  doc tests, and `nova` itself on crates.io, with install scripts for
  Linux, macOS and Windows. The Phase 3 gate runs on this release, as it
  must: it installs what the release published. Its result, and the
  boundary Phase 3 closes within, are recorded in
  `docs/adr/0037-phase-3-boundary.md`. `nova --version` now reports
  0.3.0.
  ```

- [ ] **Step 2:** In `tools/vscode-nova/CHANGELOG.md`, `## [Unreleased]`
  becomes `## [0.3.0] - 2026-10-12`, with the line `The first published
  version.` under it, before `### Added`.

- [ ] **Step 3: Run**

  `bash .github/scripts/release-notes.sh 0.3.0 CHANGELOG.md | head -n 3`

  Expected: the lead paragraph's first lines (the script `release.yml`
  takes the notes with). And `bash .github/scripts/release-notes.sh 0.3.0 CHANGELOG.md | wc -l`
  prints about the size `[Unreleased]` was (about 250 lines).

- [ ] **Step 4: Commit** `docs: CHANGELOG [0.3.0], the release of Phase 3`.

### Task 3: The install texts

**Files:** `README.md`, `tools/vscode-nova/README.md`,
`tools/vscode-nova/src/extension.ts`.

- [ ] **Step 1: The root README's "Install"** (spec §3 item 4, decision 2):

  ````markdown
  ## Install

  With Rust installed:

  ```bash
  cargo install nova-cli --locked
  nova new hello
  cd hello
  nova run
  ```

  `--locked` builds with the dependency versions the release was tested
  with. The install also builds Nova's runtime library and embeds it in
  `nova`, so the installed `nova` needs nothing beside it. It unpacks the
  library into `~/.nova/runtime/` the first time it links a program, and
  `NOVA_HOME` moves that directory.

  Without Rust, a ready-built `nova` from the latest GitHub release,
  checked against the release's `SHA256SUMS`: on Linux and macOS, into
  `~/.nova/bin`,

  ```bash
  curl -sSfL https://github.com/Sakeerin/nova/releases/latest/download/install.sh | sh
  ```

  and on Windows, into `%USERPROFILE%\.nova\bin`, which the script adds
  to your PATH:

  ```powershell
  irm https://github.com/Sakeerin/nova/releases/latest/download/install.ps1 | iex
  ```

  Each release also attaches the archives the scripts install from. `nova
  build` and `nova test` need a C linker: `cc` on Linux and macOS, the
  Visual Studio Build Tools on Windows. `nova run` needs nothing else.
  ````

  (The outer fence here is for this plan; the README gets the inner text
  between `## Install` and `### Editor support`.)

- [ ] **Step 2: The root README's "Editor support",** its sentence about
  the `.vsix`: "For VS Code, install **Nova** by `sakeerin` from the
  Marketplace or Open VSX (`code --install-extension sakeerin.nova`).
  Each GitHub release also attaches it as `nova-vscode-<version>.vsix`,
  which installs with **Extensions: Install from VSIX…**." The rest of
  the section stays.

- [ ] **Step 3: The extension's README,** its install line:

  ```markdown
  The extension runs the `nova` you have installed: the `nova.server.path`
  setting, or else `nova` from your PATH. Install it with

      cargo install nova-cli --locked

  or, without Rust, with the install scripts in Nova's README
  (https://github.com/Sakeerin/nova#install).
  ```

- [ ] **Step 4: The extension's error** (`src/extension.ts`, the
  `showErrorMessage` call): its last string becomes
  `"cargo install nova-cli --locked (or see https://github.com/Sakeerin/nova#install)"`.

- [ ] **Step 5: Run**

  - `grep -rn "cargo install --locked --git" README.md tools/vscode-nova crates/nova-cli/README.md`
    Expected: nothing.
  - In `tools/vscode-nova`: `npm ci && npm run compile`, then remove
    `node_modules/` and `out/`. Expected: `tsc` exits 0.
  - `cargo test --locked -p nova-cli --test vscode_extension`: 2 passed.

- [ ] **Step 6: Commit** `docs: install nova from crates.io, or with the install scripts`.

### Task 4: The runbook's note

**Files:** `docs/releasing.md`.

- [ ] **Step 1:** After step 1's item 2 and after step 2's tag-message
  paragraph, a dated note each:

  ```markdown
  **Amended 2026-10-12 (branch `claude/loving-archimedes-frtf5q`):** a
  phase's gate installs what its release published, so it runs after the
  tag. The lead paragraph and the tag message call the release the
  phase's, with its gate to follow; the ADR for the phase's boundary,
  written after the gate, calls the phase complete (spec
  `docs/superpowers/specs/2026-10-12-phase-3-6b-the-release-design.md`,
  decision 6).
  ```

- [ ] **Step 2: Commit** `docs: the runbook, a phase's gate follows its release`.

### Task 5: Checks, the review, the PR

- [ ] **Step 1:** `cargo fmt --all -- --check`; `cargo clippy --locked --all-targets --all-features -- -D warnings`;
  `cargo run -q -p nova-cli -- fmt --check std examples` (a fresh
  `NOVA_HOME`); `cargo build --locked --workspace` then the full suite,
  with the counting one-liner. Expected: 2022 passed, 1 failed (the IPv6
  test), 10 ignored, as Task 0.
- [ ] **Step 2:** `bash .github/scripts/package-gate.sh $P/pg36b` with
  `HOME` a scratch directory (as 3.6a's Task 2). Expected: `passed`, the
  packages named `<crate>-0.3.0.crate`, `nova version` saying `nova 0.3.0`.
- [ ] **Step 3:** One fresh read-only review of the branch, then one fix
  pass (the user's workflow).
- [ ] **Step 4:** The PR, under the repository's template headings, with
  a "Release steps" section quoting the spec's §4 table; read CI once.
  Merge waits for the user's word (spec §4, step 1).

### Task 6: The release (spec §4, steps 0 to 6)

The agent's part, step by step; each **[stop]** first gets the user's
word:

- **Step 0.** Ask the user to make the Marketplace publisher, the Open
  VSX namespace and the `crates-io` environment (runbook, "Before the
  first release"), and to say when each exists. **This comes before the
  merge of Task 5's PR** (decision 7).
- **Step 1.** On the word: merge Task 5's PR by rebase, pinned to its
  head; check the merged tree equals the head's; read CI on `main`.
- **Step 2.** On the word: tag and push.

  ```sh
  git fetch origin main
  git log -1 --format='%H %s' origin/main
  git tag -a v0.3.0 -m "Nova v0.3.0 — Phase 3 (Tooling)" -m "The release of Phase 3. Its gate runs on this release; docs/adr/0037-phase-3-boundary.md records the result and the boundary." "$(git rev-parse origin/main)"
  git push origin v0.3.0
  ```

  If the push is refused, say so; the user pushes it with the runbook's
  command. Then read the Release run's jobs, and the release's assets
  through GitHub's API (`get_release_by_tag`): eight assets and
  `SHA256SUMS`, not a prerelease.
- **Step 3.** The user makes the token and adds the secret, and says so.
  On the word: start "Publish to crates.io" through GitHub's API
  (`actions_run_trigger`, workflow `publish-crates.yml`, ref `v0.3.0`,
  input `version: 0.3.0`). The user approves the deployment. Read the
  run as it goes; report the owner check's lines and each crate. When it
  ends, remind the user to revoke the token and delete the secret.
- **Step 4.** The user publishes the extension and says what the
  listings show.
- **Step 5.** The user creates the index and its token, and says what
  `config.json` serves.
- **Step 6.** On the word: start "Release gate" (ref `main`, `version:
  0.3.0`) and read its six jobs. Then the user's clean-machine run; the
  user sends the transcript (runbook §6.3), with names and paths removed.
- **If a step fails:** the runbook's §7; tell the user what failed, what
  §7 says, and wait for their word.

Ledger every step: when, what ran, its result, its run's address.

### Task 7: The close-out PR (spec §6), after the gate passes

Written from the spec's §6 and the real records, as 3.5b's and 3.6a's
records tasks were:
- **Research first:** the Phase 3 specs and plans in
  `docs/superpowers/`, ADRs 0026 to 0036, `docs/phase-3-plan.md`, the
  master spec's Phase 3 list, and the Phase 3 PRs' bodies (their
  "Deferred minors"), through GitHub's API.
- **ADR 0037, "Phase 3's boundary,"** in ADR 0025's form: the inventory
  (master spec Phase 3 positions 1 to 8; sub-phases 3.0 to 3.6), the
  deviations, the backlog, the gate's record (the Release gate run and
  the transcript), and the consequences.
- **ADR 0031:** the measurements from the transcript.
- **Dated notes:** the master spec's Phase 3 section (as Phase 2's at
  `:559`), `docs/phase-3-plan.md` (3.6 entry, §7), ADR 0027, ADR 0036,
  and the runbook where a step went otherwise.
- Checks: every cited path exists; no local paths or names; `nova fmt
  --check std examples`.
- The PR, one review and fix pass as the workflow says, CI read once;
  merge on the user's word.
