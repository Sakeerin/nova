# Phase 3.6b, "The release": design

> Status: **draft for review** (2026-10-12). The second half of Phase 3.6,
> "The `v0.3.0` release" (`docs/phase-3-plan.md` §4, the 3.6 entry), and
> the end of Phase 3. Phase 3.6a is merged: PR #111, ADR 0036, the
> runbook `docs/releasing.md`. On 2026-10-12 the user made four decisions
> (§13, decisions 1 to 4):
> - two pull requests: the release, then, after the gate, the close-out;
> - the install texts show `cargo install nova-cli --locked` first;
> - the agent pushes the tag and starts each workflow, each on the
>   user's word;
> - of the accounts the release needs, the user has crates.io's; the
>   Marketplace publisher, the Open VSX namespace, the GitHub environment
>   `crates-io` and the index repository are still to be made.
>
> Branch `claude/loving-archimedes-frtf5q`.

3.6b publishes `v0.3.0`: the GitHub release with its archives and install
scripts, the 19 crates on crates.io, and the VS Code extension on the
Marketplace and Open VSX. Then it runs the Phase 3 gate on what was
published, on fresh CI runners and on a clean machine of the user's, and
records the result in the ADR that closes Phase 3.

3.6a built every piece; 3.6b only runs them, in the runbook's order. Most
of its steps publish something that stays, so almost every step is a stop
for the user's word.

## 1. What 3.6b delivers

1. **The release pull request:** the version bump to 0.3.0,
   CHANGELOG's `[0.3.0]`, the extension's CHANGELOG, and the texts that
   say how to install `nova` (§3).
2. **`v0.3.0`,** published: the tag and its GitHub release, the 19 crates,
   and the extension in both stores (§4).
3. **The package index,** `Sakeerin/nova-index`, with `shapes` as its
   first package (§4, step 5 and the gate).
4. **The Phase 3 gate,** run and recorded: "Release gate" on three
   systems, and the user's own run on a clean machine (§5).
5. **The close-out pull request:** ADR 0037, "Phase 3's boundary", with
   the gate's record; ADR 0031's measurements; the dated notes that say
   Phase 3 is complete (§6).

## 2. The starting point (read on 2026-10-12)

- `main` is `59b0d53`, PR #111's last commit. Every member of the
  workspace is at `0.2.0`, set once in `[workspace.package]`, with the 19
  published crates pinned at `=0.2.0` in `[workspace.dependencies]`
  (ADR 0036).
- `docs/releasing.md` holds every step of a release, each **[stop]**
  marked, and what to do when one fails (its §7).
- The workflows "Publish to crates.io" and "Release gate" are on `main`
  and have never run. `release.yml` has never run on a tag since 3.0
  rewrote it; its pull-request runs, the last on PR #111, were green with
  every new step.
- Tags on the repository end at `v0.2.0`, which has no GitHub release;
  three `v0.2.0-alpha` prereleases have one each, with no assets. GitHub's
  `releases/latest` skips prereleases, so it will name `v0.3.0`.
- All 19 crate names were free on crates.io on 2026-10-12. The publish
  script stops before its first upload if any is now someone else's.
- The install texts still say `cargo install --locked --git
  https://github.com/Sakeerin/nova nova-cli`: the root README (`:11`),
  the extension's README (`:12`) and the error the extension shows when it
  cannot start `nova` (`tools/vscode-nova/src/extension.ts:53`). nova-cli's
  own crates.io page, `crates/nova-cli/README.md`, already shows
  `cargo install nova-cli --locked` first, then the install scripts.
- The runbook's step 1 suggests, for `v0.3.0`, a CHANGELOG lead paragraph
  and a tag message that say Phase 3 is complete. The gate cannot run
  before the release exists, so 3.6b words both as a release whose gate
  follows (decision 6).

## 3. The release pull request

The runbook's step 1, with these contents:

1. **The version, 0.3.0,** by the runbook's commands: the workspace's
   `version` and the 19 pins in the root `Cargo.toml`, the extension's
   `package.json` and `package-lock.json`, and `cargo update --workspace`.
   **This changes `Cargo.lock`:** the 23 workspace packages' own
   versions, and nothing else (the runbook's diff: 23 lines). Approving
   this spec approves that change (decision 5).
2. **`CHANGELOG.md`:** `[Unreleased]` becomes `[0.3.0] - <the day it
   merges>`, with a fresh, empty `[Unreleased]` above it, as Phase 2's
   `[0.2.0]` did. Its lead paragraph says: Phase 3, "Tooling", is
   released; this is the milestone master spec §7 reserves; the gate runs
   on this release, and ADR 0037 records its result and the phase's
   boundary; `nova --version` reports 0.3.0. `release.yml` takes the
   release notes from this section.
3. **`tools/vscode-nova/CHANGELOG.md`:** `[Unreleased]` becomes
   `[0.3.0] - <the day>`, with "The first published version."
4. **The install texts** (3.6a's decision 21), `cargo install` first
   (decision 2):
   - the root README's "Install": `cargo install nova-cli --locked`, then
     the two install-script lines for a machine without Rust, then the
     release archives; and its "Editor support": the extension is
     `sakeerin.nova` on the Marketplace and Open VSX, and each release
     still attaches the `.vsix`;
   - the extension's README: the same order, shorter;
   - the error in `src/extension.ts`: "Set nova.server.path, or install
     nova: cargo install nova-cli --locked (or see
     https://github.com/Sakeerin/nova#install)".
   These name what steps 2 to 4 publish within hours of the tag; the
   window is the release's own.
5. **The runbook,** a dated note in its step 1 and step 2: a phase's
   release names its gate as following, and the phase is called complete
   by the close-out's ADR (decision 6).

**Checks before it merges:** the manifests and extension tests
(`release_manifests`, `vscode_extension`), the full suite, and CI, whose
"Packaged nova" packages the 19 at 0.3.0 and installs `nova` from them,
and whose `release.yml` run builds the 0.3.0 archives and takes its notes
from the new section. It merges by rebase, on the user's word.

## 4. The release, step by step

Each row is the runbook's step; **stop** means the user's explicit word
comes first. "Agent" is the agent working this spec.

| # | Step | Who | Stop | What is checked after |
|---|---|---|---|---|
| 0 | **Preparation:** the Marketplace publisher `sakeerin`; an open-vsx.org account linked to Eclipse, the agreement signed, the namespace `sakeerin`; the environment `crates-io` (required reviewer: the user; tag rule `v*`) | user | yes | the user says each exists |
| 1 | The release PR (§3) merged by rebase | agent | yes | the merged tree equals the PR's head; CI on `main` green |
| 2 | The annotated tag `v0.3.0` on the release commit, pushed | agent | yes | the Release run green; the release has its eight assets and `SHA256SUMS`; `releases/latest` names `v0.3.0` |
| 3a | A crates.io token (publish-new, publish-update, `nova-*`, expiring the next day), added as `crates-io`'s secret `CARGO_REGISTRY_TOKEN` | user | yes | — |
| 3b | "Publish to crates.io" started on the tag, version `0.3.0` | agent | yes | the user approves the deployment; the run's owner check passes; the 19 publish (about 2 h 20 min); the index serves `nova-cli` 0.3.0 |
| 3c | The token revoked, the secret deleted | user | — | the agent reminds, whatever the result |
| 4 | The extension published from the release's `.vsix`, to the Marketplace and Open VSX | user | yes | both listings show 0.3.0 |
| 5 | `Sakeerin/nova-index` created with its `config.json`; a fine-grained token for it | user | yes | `config.json` is served |
| 6a | "Release gate" started, version `0.3.0` | agent | yes | all six jobs green |
| 6b | The clean-machine run, recorded, ending with `nova publish` of `shapes` | user | yes (the publish) | the transcript, as the runbook's §6.3 template |
| 7 | The close-out PR (§6) | agent | merge: yes | — |

- **Preparation comes before the release PR merges** (decision 7). The
  publisher is inside the `.vsix`, so if the Marketplace or Open VSX
  refuses `sakeerin`, the extension's manifest changes before anything is
  published, not after the crates are up.
- **The gate runs as soon as the crates are up** (decision 8). A
  crates.io install that fails is simplest to undo within 72 hours of the
  first publish, by deleting the 19 in reverse order (the runbook's §7).
- **When a step fails,** the runbook's §7 says what to do. A failure that
  needs a change to the repository is a new patch version, `0.3.1`, from
  step 1; nothing published is replaced.
- **How the agent pushes and starts:** the tag with `git push origin
  v0.3.0`; the workflows through GitHub's API (a `workflow_dispatch` with
  `ref: v0.3.0` for the publish, `main` for the gate). If this session
  cannot push a tag or start a workflow, the agent says so and the user
  runs the runbook's own command for that step.
- **What the agent can and cannot see:** it reads runs, jobs, logs and
  releases through GitHub's API. It may not reach crates.io's API, the
  stores or `raw.githubusercontent.com` from its sandbox; where it cannot
  check, the user does, with the runbook's command, and says what it
  printed.

## 5. The gate

The Phase 3 gate (`nova-spec/00-MASTER-SPEC.md:653`): "External user can
`cargo install nova-cli`, init a project, write code with autocomplete,
format, and publish a package." 3.6b's gate is that gate met on what
`v0.3.0` published:

1. **"Release gate"** on `0.3.0`, all six jobs green: `cargo install
   nova-cli --version =0.3.0 --locked` from crates.io on Linux, macOS and
   Windows with `gate.sh --test`, `packages-gate.sh`, `doc-gate.sh` and
   `nova fmt --check`; and the release's install script with `gate.sh
   --test` on each system.
2. **The user's clean-machine run,** the runbook's §6.2, in its §6.3
   template: `cargo install nova-cli --locked` and `nova version`;
   `nova new` and `nova run`; completion after `p.` in VS Code with the
   extension from a store (with a screenshot); `nova fmt --check`,
   `nova fmt`, `nova fmt --check`; `nova publish` of `shapes` to the real
   index, then `nova add shapes` in a second project and `nova run`.

The transcript reaches the agent with home directories, user and machine
names, addresses and tokens removed. Both together are the gate's record.

## 6. The close-out pull request

After the gate passes:

1. **ADR 0037, "Phase 3's boundary",** in ADR 0025's form:
   - **the inventory:** the master spec's Phase 3 list, positions 1 to 8,
     and the plan's sub-phases 3.0 to 3.6, each with what was built and
     its ADR or spec;
   - **the deviations** no earlier record decides, and those whose record
     does (ADR 0026's scope: no REPL, no debugger, no registry server;
     ADR 0028 to 0036);
   - **the backlog:** the deferred minors the Phase 3 pull requests left,
     gathered from their bodies and the plan;
   - **the gate's record:** the Release gate's run and its six jobs; the
     user's transcript, with the screenshot described in words;
   - **the consequences:** Phase 3 is complete within this boundary.
2. **ADR 0031's measurements,** which wait for the first real publish:
   the API requests `nova publish` made and the index's size, from the
   transcript.
3. **Dated notes** that say Phase 3 is complete: the master spec's Phase 3
   section (as Phase 2's note at `:559`), `docs/phase-3-plan.md` (the 3.6
   entry and §7), ADR 0027 (its package route ran for real at the
   crates.io install), ADR 0036 (what the release showed), and the
   runbook if a step went otherwise than it says.
4. **`CHANGELOG.md` `[Unreleased]`:** nothing a user would notice, so no
   entry unless the close-out changes behaviour.

It merges by rebase, on the user's word. Phase 4 starts after it.

## 7. Testing

3.6b adds no code. What it changes is checked by what exists:
- the release PR: `release_manifests` and `vscode_extension` (the
  version everywhere), the full suite, `nova fmt --check std examples`,
  and CI on three systems, "Packaged nova" and `release.yml` included;
- the release itself: the Release run, the publish run, and the gate
  (§5).

## 8. Records

- In the release PR: CHANGELOG `[0.3.0]`, the extension's CHANGELOG, the
  install texts, the runbook's note (§3).
- In the close-out PR: ADR 0037, ADR 0031's measurements, the dated notes
  (§6).

## 9. Risks

1. **A crate name taken before the publish.** The publish script stops
   before its first upload. *Then:* a rename is a change for `0.3.1`; the
   GitHub release stands.
2. **A store refuses the publisher or the name.** *Mitigation:* the
   accounts and namespace exist before the release PR merges (decision
   7), so a change to `publisher` comes before the tag.
3. **The real crates.io install fails where the emulation passed**
   (ADR 0036's limit: the nested runtime build resolves offline from
   what the install fetched). *Mitigation:* the gate runs right after the
   publish, inside the 72 hours in which the 19 can be deleted in reverse
   order.
4. **This session cannot push a tag or start a workflow.** *Then:* the
   user runs the runbook's command for that step; nothing else changes.
5. **A long publish.** The 19 new crates take about 2 h 20 min at
   crates.io's rate limit; the job's limit is 345 minutes. *Then:* a run
   that stops resumes from the same tag (the runbook's §7).
6. **The README names routes that are hours from working** (the stores,
   crates.io) when the tag is pushed. Accepted: the release day's window.

## 10. Not in 3.6b

- Any code change; anything 3.6a deferred (its PR's "Deferred minors"),
  unless the release needs it, in which case it is `0.3.1`.
- Signing or notarizing binaries; more targets; Homebrew, winget or other
  package managers.
- Automating the extension's publish, or keeping any store token in
  GitHub.
- Phase 4.

## 11. Ordering

1. This spec, approved.
2. The release PR (§3): written, checked, CI green, merged on the user's
   word.
3. §4's steps 0 to 6, each on the user's word, in the table's order
   (step 0 before step 1's merge).
4. The close-out PR (§6), after the gate passes.

## 12. Open for the plan

- The exact wording of CHANGELOG's lead paragraph, the README's install
  section and the extension's texts.
- The ADR's inventory, from the Phase 3 specs, ADRs 0026 to 0036 and the
  pull requests #96 to #111.

## 13. Decisions

By the user, 2026-10-12:

1. Two pull requests: the release (§3), then the close-out after the gate
   (§6).
2. The install texts show `cargo install nova-cli --locked` first, then
   the install scripts, then the archives.
3. The agent pushes the tag and starts "Publish to crates.io" and
   "Release gate", each on the user's word; the user adds and removes the
   crates.io token, publishes the extension, creates the index, and does
   the clean-machine run.
4. The user has a crates.io account; the Marketplace publisher, the Open
   VSX namespace, the environment `crates-io` and the index repository
   are made in step 0 and step 5.

Made in this spec, for review:

5. The release commit changes `Cargo.lock`: the 23 workspace packages'
   versions, by `cargo update --workspace`, and nothing else.
6. CHANGELOG's `[0.3.0]` and the tag's message call `v0.3.0` Phase 3's
   release, with its gate to follow; ADR 0037 calls Phase 3 complete.
   Phase 2's tag came after its gate, but Phase 3's gate needs the
   published release. The runbook gets a dated note saying so.
7. The store accounts and the environment are made before the release PR
   merges.
8. The gate runs as soon as the crates are up, within the 72 hours.
9. A failure that needs a change to the repository is released as
   `0.3.1`; nothing published is replaced.
10. The extension's error names `cargo install nova-cli --locked` and the
    README's install section.
11. CHANGELOG `[Unreleased]` gets no entry for the close-out unless it
    changes behaviour.
