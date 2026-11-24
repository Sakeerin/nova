# Phase 5.6b, "the release `v0.5.0`": design

- **Date:** 2026-11-24.
- **Status:** approved by the user (2026-11-24), as drafted.
- **The user's decisions:** Phase 5.6's decisions 1 and 8 (2026-11-22)
  and 5.6b's decisions 1 to 8 (2026-11-23), each the recommended option,
  quoted in §12. The spec's own recommendations follow them there, each
  marked "(recommended, not asked)".
- **Branch:** `claude/loving-archimedes-frtf5q-56b`, from `main` at
  `ed165975` (5.6a merged as #149). Nothing is pushed without the user's
  word.
- **Inputs:** `docs/releasing.md`; the 3.6b, 0.3.1, 4.8 and 5.6a specs;
  ADR 0047 (decisions 5 and 6, "The gate's record"); ADR 0060; `v0.4.0`'s
  release commit `eab7da8e` (#126) and records commit `cc829f58` (#127);
  the research of 2026-11-23 at `ed165975`.

## 1. What 5.6b delivers

The second of Phase 5.6's three pull requests (5.6 decision 1),
published by the runbook as `v0.4.0` was:

1. **One release pull request,** whose last commit is the release
   commit (§3); its CI is §4.
2. **The steps after its merge** (§5), each on the user's word: the tag,
   the GitHub release, the 20 crates, the extension in both stores, the
   Release gate and the user's try of the published `nova`. The records
   pull request, with ADR 0061, follows them (§9).

## 2. The starting point

Read at `ed165975` and re-checked for this spec; no build run.

- **`main`'s CI run 45142347493** passed, 19 of 19 jobs. The last
  release is `v0.4.0`, on `eab7da8e`; the next ADR is 0061.
- **Since `v0.4.0`,** `Cargo.toml`, `Cargo.lock` and `tools/vscode-nova/`
  have not changed; of `.github/`, only `ci.yml` and the pull-request
  template have.
- **Nothing pins 0.4.0** but the four version files (§3.1): no test,
  golden file, script or workflow. The tests read
  `env!("CARGO_PKG_VERSION")` or the root `Cargo.toml`.
- **The tag is the user's to push:** this session's credential was
  refused with HTTP 403 for `v0.3.0` (4.8 plan `:119-123`), and the user
  pushed `v0.4.0` (ADR 0047 `:364-365`).

## 3. The release commit

The runbook's step 1 (`docs/releasing.md:115-189`), in one commit,
`release: v0.5.0, Phase 5 released`.

### 3.1 The versions, 0.4.0 → 0.5.0, in four files

By the runbook's commands (`:126-135`), with `version=0.5.0`, and
`cargo update --workspace --offline` without a network (`:138`):

| File | Lines | What changes |
|---|---:|---|
| `Cargo.toml` | 21 | `[workspace.package]`'s `version` (`:11`), the 20 `=` pins (`:27-46`) |
| `Cargo.lock` | 23 | the `version` of the workspace's own 23 packages |
| `tools/vscode-nova/package.json` | 1 | `version` (`:5`) |
| `tools/vscode-nova/package-lock.json` | 2 | the two `version` lines under `"name": "vscode-nova"` (`:3`, `:9`) |

- These are the runbook's counts (`:137-140`) and `eab7da8e`'s. On a
  scratch copy of `ed165975` the commands gave them, offline ("Locking
  23 packages"), and the lock's only changes were the 23 `version` lines.
- **`Cargo.lock` changes in those 23 lines only** (5.6b decision 6); four
  third-party packages at 0.4.0 (`:544`, `:1473`, `:2215`, `:2230`) stay.
  **Any other change to the lock stops the work** and goes to the user.
- `selfhost/nova.toml:3` stays at 0.1.0; nothing reads it.
- Then the runbook's check (`:144`): `cargo test --locked -p nova-cli
  --test release_manifests --test vscode_extension`, 8 and 3 tests.

### 3.2 `CHANGELOG.md`

- `## [Unreleased]` (`:10`) becomes `## [0.5.0] - <the release commit's
  date>`, with a fresh, empty `## [Unreleased]` above it.
- **The lead follows the `[0.3.0]` model** (`:886-895`; 5.6b decision
  1): the phase "is released", and an ADR named before it exists "will"
  record its boundary. The `[0.4.0]` model, "is complete, within the
  boundary … records" (`:611-625`), needs that ADR first. Drafted, with
  no sizes (5.6b decision 2) and N not shipped (5.6 decision 8):

  ```markdown
  **Phase 5, "Self-hosting", is released.** This is the `v0.{phase}.0`
  milestone `nova-spec/00-MASTER-SPEC.md` §7 reserves for a phase. Nova
  gained what a compiler written in Nova needs: `?`, `+=` on fields and
  elements, tuples, nested patterns and or-patterns, `Char` literals and
  ranges, and in std `StringBuilder`, number parsing, `OrderedMap` and
  `std/process`. That compiler, N, lives in `selfhost/` and is not
  shipped: the release ships the Rust `nova`, as before. The bootstrap
  passes its gate in CI: `nova build --release` builds N into S1, S1
  builds N into S2, and S2 builds N into S3, and S1 == S2 == S3, byte for
  byte, in `main`'s CI run 45142347493
  (https://github.com/Sakeerin/nova/actions/runs/45142347493;
  `docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md`). The boundary
  Phase 5 closes within will be recorded in
  `docs/adr/0061-phase-5-boundary.md`. `nova --version` now reports
  0.5.0.
  ```

- **The list of sub-phases** (`:12-37`) stays under the lead. It leaves
  out 5.2b, which the body covers; 5.2b goes after 5.2a, in the list's
  form: "… §3; ADR 0053), and 5.2b, packages, the WASM target and test
  mode in the resolution dump (§4; ADR 0054)."
- Nothing else changes. `release.yml` takes the release notes from this
  section (`.github/workflows/release.yml:326`).

### 3.3 `tools/vscode-nova/CHANGELOG.md`

Two tests hold the extension's version to Nova's
(`release_manifests.rs:301`, `vscode_extension.rs:76-85`). The file has
no `[Unreleased]`; each release adds a section above the last
(`docs/releasing.md:172-177`). The extension has not changed since 0.4.0,
so the section is the runbook's line (`:167-169`), version changed (5.6b
decision 4), above `## [0.4.0] - 2026-10-23` (`:9`), in its form:

```markdown
## [0.5.0] - <the release commit's date>

Released with Nova 0.5.0; no change to the extension.
```

### 3.4 `README.md`

A `### Self-hosting` subsection after `### Web` (`:80-120`), before the
`---` at `:122` (5.6b decision 3; 5.6 decision 8). Nothing in the README
mentions `selfhost/` today. Drafted:

```markdown
### Self-hosting

`selfhost/` holds N, Nova's compiler written in Nova. It is not shipped:
a release ships the Rust `nova`, and N stays in the repository. On every
pull request, on Linux, CI builds N with `nova build --release` into S1,
then N with S1 into S2 and with S2 into S3, and holds S1 == S2 == S3,
byte for byte. [`selfhost/README.md`](selfhost/README.md) says how to
build and run it; [ADR 0060](docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md)
records the gate.
```

These are the README's first Markdown links. `crates/nova-cli/README.md`,
crates.io's page, describes the shipped tool and does not change.

### 3.5 The runbook, `docs/releasing.md`: two dated notes

Both are needed: the runbook's latest notes speak of `v0.4.0` only.
Each is in the runbook's form, dated with the release commit's date.

1. **The token,** after the notes that close step 3's item 4, and
   indented as they are (`:320-334`, the last saying `v0.4.0` added a
   crate). Item 1 (`:276-281`) asks for publish-new and publish-update:

   ```markdown
      **Amended <date> (branch `claude/loving-archimedes-frtf5q-56b`):**
      `v0.5.0` adds no crate: its 20 are new versions of crates on
      crates.io, within the burst of 30, so its token takes
      **publish-update** alone (spec
      `docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`,
      the user's decision 5).
   ```

2. **Step 6.2,** after the note of 2026-10-22 (`:515-523`), which
   replaces the run "for `v0.4.0`":

   ```markdown
   **Amended <date> (branch `claude/loving-archimedes-frtf5q-56b`):** for
   `v0.5.0`, by the user's decision of 2026-11-23, the maintainer installs
   the published `nova` and tries it in place of this run, as for
   `v0.4.0` (spec
   `docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`,
   decision 5). `docs/adr/0061-phase-5-boundary.md` will record the try.
   ```

### 3.6 What else the release makes stale

Searched at `ed165975` for `0.4.0`, "unreleased" and the release in the
future tense. Nothing else names 0.4.0 as current; the other hits are
records, and two comments that say "from v0.4.0 on"
(`.github/scripts/gate.sh:18`, `.github/workflows/release-gate.yml:75`).

| Place | What | Where it changes |
|---|---|---|
| `CHANGELOG.md:12-37` | lacks 5.2b | here (§3.2) |
| `docs/releasing.md:320-334`, `:515-523` | the token and step 6.2, for `v0.4.0` | here (§3.5) |
| `nova-spec/00-MASTER-SPEC.md:913-914` | "The release `v0.5.0` and the boundary ADR … follow" | the records pull request; `eab7da8e` left the master spec alone |
| `docs/phase-5-plan.md:700-705` | 5.6's three pull requests, in the future | the records pull request |
| ADR 0060 `:428-432`, `:516-522`; ADR 0059 `:561` | the runtime's `0.4.0-…` directories | never: records of their tree, which ADR 0060 says will change at the bump |
| `README.md`, `selfhost/README.md`, `crates/nova-cli/README.md` | no version, no "unreleased" | nothing |

### 3.7 The tag message

Short, saying what the lead says (5.6b decision 1), in `v0.3.0`'s form
(`docs/releasing.md:209-212`). The user makes and pushes the tag (5.6b
decision 5) with these commands, once CI on `main` has passed on the
merged commit (`:222-224`):

```sh
git fetch origin main
git log -1 --format='%H %s' origin/main   # release: v0.5.0, Phase 5 released
commit=$(git rev-parse origin/main)
git tag -a v0.5.0 -m "Nova v0.5.0 — Phase 5 (Self-hosting)" -m "The release of Phase 5. N, the Nova compiler written in Nova, lives in selfhost/ and is not shipped; the bootstrap passes its gate in CI (docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md). docs/adr/0061-phase-5-boundary.md will record the boundary." "$commit"
git push origin v0.5.0
```

## 4. What CI must show on the release pull request

Every job green: CI's 19, and `release.yml`'s eight, which the commit
starts by touching `Cargo.toml`, `Cargo.lock` and `tools/vscode-nova/**`
(`release.yml:13-26`). The bump adds no test and no file a count reads,
so no count should move from `main`'s run 45142347493:

| Job, step | Run 45142347493 | Expected |
|---|---|---|
| `release.yml` (on a pull request) | — | four builds, the `.vsix` `nova-vscode-0.5.0.vsix`, std's site; "Prepare the release" finds `## [0.5.0]` (`:326`) and prints `publish runs: gh release create v0.5.0 …` (`:338`); "Publish the release" skipped (`:378`), as on `v0.4.0`'s release pull request (run 39976759655) |
| Packaged nova, three systems (`ci.yml:537-555`) | `nova-cli-0.4.0.crate`, 1759 files; 20 packages; `package-gate: passed` | the same at 0.5.0 |
| Test, `cargo test` (passed, failed, ignored) | Linux 2838 0 10; macOS 2838 0 9; Windows 2815 0 9 | the same |
| in it, `release_manifests`, `vscode_extension`, `project` | 8, 3 and 25 passed | the same |
| Selfhost, `selfhost.rs`'s main step | Linux 71 0 3 (55.57 s); macOS 69 0 3 (31.18 s); Windows 65 0 3 (108.49 s) | the same counts |
| Selfhost, S1 builds the golden suite | 2 passed (30.46 s) | the same |
| Selfhost, the bootstrap gate | 1 passed; S1 == S2 == S3 | the same |

- **Recorded, not expected** (ADR 0060 §10): the binaries' size, the
  runtime's directory, now `home/runtime/0.5.0-<crc>/`
  (`crates/nova-driver/src/runtime_cache.rs:77`), and the crates' bytes
  (584,032 and 2,119,011 at 0.4.0). The runtime's bytes most likely
  change with its crate's version, and S1's size (9,539,736 at
  `ed165975`) with them; N's `.ll` (12,760,111) should not, since no
  emitter reads the version. **S1 == S2 == S3 is the gate** (ADR 0060
  §2), not a size; no test pins one.
- **Windows' `selfhost.rs` time,** 108.49 s against its 110 s line (ADR
  0057 §19), is recorded too. CI does not enforce the line, and the
  release changes nothing that step runs.

## 5. After the merge, in order

Each step is a stop for the user's word (5.6b decision 5). Who does what
is as for `v0.4.0` (ADR 0047 `:364-421`) and as 0.3.1 set it (its spec
`:167-170`): **the user** pushes the tag, makes the token and the
secret, approves the deployment, uploads to the stores and tries the
published `nova`; **the agent** merges, starts the workflows, and checks
what each step produced. If the session cannot start a workflow (not yet
tried) or reach crates.io's index or the stores, the user runs the
runbook's command and says what it printed.

| # | Step | Runbook | Who | Checked after |
|---|---|---|---|---|
| 1 | Merge by rebase | `:186-189` | agent | CI on `main` green |
| 2 | The annotated tag `v0.5.0` on the release commit, pushed (§3.7) | `:191-221` | user | — |
| 3 | `release.yml` on the tag, and the GitHub release | `:226-259` | automatic; agent checks | the run green; "Nova 0.5.0", not a draft or prerelease, notes from `[0.5.0]`; eight assets and `SHA256SUMS`, every sum `OK`; `releases/latest` → `v0.5.0` |
| 4a | A crates.io token, **publish-update** only, `nova-*`, expiring the next day; `crates-io`'s secret `CARGO_REGISTRY_TOKEN` | `:276-289` | user | — |
| 4b | "Publish to crates.io" from the tag, version `0.5.0`; the deployment approved | `:291-341` | agent starts; user approves | the package check; all 20 owned by the repository's owner; all 20 published at 0.5.0, none skipped; each index line `"vers":"0.5.0"` |
| 4c | The token revoked, the secret deleted | `:342-350` | user | the agent reminds |
| 5 | `nova-vscode-0.5.0.vsix`, its sum checked, to the Marketplace, then Open VSX | `:352-415` | user | both listings show 0.5.0 |
| 6a | "Release gate" from `main`, version `0.5.0` | `:456-473` | agent starts | six jobs green, crates.io and install script, on three systems |
| 6b | Step 6.2, replaced (§3.5): the published `nova`, tried | `:515-523` | user | their word, and `nova --version` |
| 7 | The records pull request (§9) | — | agent; merge on the user's word | — |

- **Step 4b** adds 20 versions within the burst of 30 (`:315-317`):
  minutes, as `v0.4.0`'s run 40074121811 (about 13 in all). The index,
  the runbook's step 5, is for the first release only.
- **The try (6b)** is the user's to shape; as for `v0.4.0` (ADR 0047
  `:410-421`): `cargo install nova-cli --locked`, `nova --version`
  printing `nova 0.5.0`, `nova new`, `nova run`, and any Phase 5 program
  they choose. `shapes` is not published again (`:516-517`).
- **A failed step** follows the runbook's §7 (`:656-734`); a fix in the
  repository is `0.5.1`.

## 6. Testing

5.6b adds no code. What holds what:

- **The version everywhere:** `release_manifests.rs`'s 8 tests (members
  and pins `:176`, `:184`, `:221`; the extension's two files `:301`) and
  `vscode_extension.rs:76-85`; `nova version` in `project.rs:18-30`.
- **The 20 crates and the notes:** "Packaged nova", and `release.yml`'s
  "Prepare the release". No test reads either CHANGELOG or the README;
  the review does.
- **The bootstrap on the new runtime:** `the_bootstrap_holds_s1_s2_s3`
  (`crates/nova-cli/tests/selfhost.rs:8591`) finds the runtime by
  listing its fresh home (`runtime_in`, `:6899`), with no version.
- **The full suite, as the Test job runs it** (`ci.yml:73-78`): `cargo
  test --locked --workspace --all-features --no-fail-fast`, with
  `NOVA_REQUIRE_NODE=1`, and on Linux `NOVA_REQUIRE_CLANG=1` and
  `clang-18`: `2838 passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9`
  on macOS, `2815, 0, 9` on Windows, as in run 45142347493.
- **The golden suite is unchanged:** no file under `tests/runtime/`
  changes; `release_golden_suite` (`run_tests.rs:1798`) runs in that
  step on Linux. **Rust 1.78:** CI's MSRV job.
- **No new test is needed:** each file the bump touches is held by a
  test or a workflow.

## 7. A prototype, and its measurement

1. **Locally:** §3 on a local prototype branch from `ed165975`, never
   pushed under its own name; the runbook's commands, `git diff
   --numstat` (21, 23, 1, 2), `git diff -U0 Cargo.lock` (the 23 lines
   only), the runbook's check, and `bash .github/scripts/release-notes.sh
   0.5.0 CHANGELOG.md | wc -m` (§10, risk 3).
2. **On CI,** with the user's word asked at that step: the prototype on
   `claude/loving-archimedes-frtf5q-56b-ci`, a draft pull request,
   measurement only, closed unmerged. Three measurements: **`release.yml`
   on a pull request** (eight jobs, the notes found, the `.vsix`'s
   name); **the gate after the bump** (S1's size, the runtime's
   `0.5.0-<crc>`, each stage's time); **"Packaged nova"** (`nova-cli`'s
   files and bytes, the 20 packages' bytes, three systems). The rest of
   CI is checked against §4; past an expected count, the user is asked.
3. **Nothing is published from it:** `release.yml` publishes only on a
   tag (`:378`); the other release workflows run only when started.

## 8. The cut, for the plan

One pull request, one commit, as `eab7da8e`:

1. **Task 1, the release commit:** §3.1 to §3.5, the runbook's check,
   the lock's diff read line by line.
2. **Task 2, the measurements:** §7's draft pull request.
3. **Task 3, verification:** the diff's stat; §4's counts; a search of
   the diff for local paths and private names; the review, and one fix
   pass, before the pull request opens.

## 9. Records

- **In the release pull request:** §3.2 to §3.5.
- **In the records pull request** (5.6b decision 7), after step 6b, on
  `claude/loving-archimedes-frtf5q-56b-records`: **ADR 0061,
  `docs/adr/0061-phase-5-boundary.md`,** in ADR 0047's shape, with the
  inventory (the master spec's Phase 5 list,
  `nova-spec/00-MASTER-SPEC.md:743-753`, and sub-phases 5.0a to 5.6b),
  the deviations, the backlog (macOS and Windows for the bootstrap among
  it, ADR 0060's Consequences), and "The gate's record": run
  45142347493, the release pull request's CI, every publish's run, the
  store listings, the Release gate and the user's try; CHANGELOG
  `[0.5.0]`'s dated note with the run IDs, as `cc829f58`; dated notes in
  the master spec (`:913-914`) and `docs/phase-5-plan.md`. It cites
  `main`'s commit hashes only.

## 10. Risks

1. **The tag push refused:** planned for, the user pushes it (§3.7).
2. **The runtime's crc changing,** and S1's size with it: expected. The
   gate is equality under one archive; no test pins a size or a
   directory; each home in `selfhost.rs` is fresh (`fresh_dir`, `:294`).
3. **The release notes' size:** `[0.5.0]` is 39,616 characters before
   the lead (597 lines; `release-notes.sh` on a scratch copy), about
   41,000 with it, against GitHub's limit of 125,000 characters on a
   release's body (recalled, not checked here). `v0.4.0`'s body has
   14,026. The prototype measures it.
4. **A token's scope:** publish-update alone. Were it refused at the
   first upload, `nova-diagnostics`, nothing would be published: a new
   token, and the workflow again from the tag (`:688-692`). The
   environment's secrets could not be read here (the proxy refused); the
   user sets the secret fresh anyway.
5. **The stores:** a version published to a store cannot be published
   there again (`:721-724`); each upload needs the user's own token or
   sign-in. The `.vsix` differs from 0.4.0's in its version alone.
6. **Windows' `selfhost.rs` time** at its line (§4): a pass goes to the
   user, not into this pull request.

## 11. Not in 5.6b

- ADR 0061 and the records (§9).
- Shipping `selfhost/`, N or an S1 binary (5.6 decision 8); the
  archives are unchanged.
- A change to `nova-cli`'s crate: `tests/selfhost*` stay in it (5.6b
  decision 8), 1,759 files and 584,032 bytes at 0.4.0, far under
  crates.io's 10 MB.
- Any code change; the bootstrap on macOS or Windows; the index.

## 12. Decisions

The user's are quoted as given: each option's name, in Thai, as it was
asked, ends "(แนะนำ)", "recommended", and the text after it says what
was chosen.

**Phase 5.6's (2026-11-22).** Decisions 2 to 7 are 5.6a's, recorded in
ADR 0060; 5.6b changes none of them.

- **1.** Split: "5.6a gate → release → บันทึก (แนะนำ)": PR 1 = the gate in
  CI + the tolerances ADR + selfhost/ README; PR 2 = the release v0.5.0
  (version bump); PR 3 = the boundary ADR and the records closing Phase
  5, after the release.
- **8.** Release and selfhost/: "ไม่ส่ง selfhost/ บอกใน notes (แนะนำ)": the
  archive unchanged; release notes and README say N lives in selfhost/
  and the bootstrap passes the gate.

**5.6b's (2026-11-23), each the recommended option.**

1. The lead: '"released" แบบ v0.3.0 (แนะนำ)': "Phase 5, Self-hosting, is
   released"; N in selfhost/, not shipped; the bootstrap passes the gate
   in CI (ADR 0060); the boundary will be recorded in
   docs/adr/0061-phase-5-boundary.md. The tag message says the same.
2. Figures: "อ้าง run ของ main ไม่ใส่ขนาด (แนะนำ)": S1 == S2 == S3 byte for
   byte in main's CI run 45142347493, no sizes; the release's own runs go
   to the boundary ADR.
3. README: "เพิ่มหัวข้อ Self-hosting ~5 บรรทัด (แนะนำ)": ### Self-hosting
   after ### Web: N in selfhost/, not shipped, S1 == S2 == S3 in CI,
   links to selfhost/README.md and ADR 0060.
4. Extension: "0.5.0 + publish ทั้งสอง store (แนะนำ)": version 0.5.0; its
   CHANGELOG "Released with Nova 0.5.0; no change to the extension.";
   both stores after the tag (the user uploads).
5. Publishes: "ครบทุกขั้นแบบ v0.4.0 (แนะนำ)": tag (the user pushes it) ->
   release.yml -> crates.io, all 20 crates (a publish-update token
   suffices: no new crate) -> Marketplace + Open VSX (the user uploads)
   -> the release gate, with step 6.2 replaced by the user trying the
   published nova (as ADR 0047 decision 6). Each step still on the
   user's word when it comes.
6. Cargo.lock: "อนุญาต 23 บรรทัดนี้ (แนะนำ)": exactly the 23 workspace
   version lines 0.4.0 -> 0.5.0 from cargo update --workspace; anything
   else changing stops and asks.
7. Records PR: "หลัง publish ครบและคุณลองแล้ว (แนะนำ)": like #127; ADR 0061
   records every release run, the user's check, Phase 5's inventory and
   backlog, in one PR.
8. nova-cli crate: "ปล่อยตามเดิม (แนะนำ)": no manifest change;
   tests/selfhost* stay in the crate.

**Made in this spec:**

9. The branch `-56b`, as 5.6a used `-56`; `-56b-ci` to measure,
   `-56b-records` for the records. (recommended, not asked)
10. Both new CHANGELOG headings take the release commit's date, as
    `eab7da8e`'s 2026-10-23 (merged and tagged 2026-10-24).
    (recommended, not asked)
11. The runbook's two notes go in the release pull request, so the
    runbook is right when followed, as 4.8 put step 6.2's in before
    `v0.4.0`. (recommended, not asked)
12. 5.2b (ADR 0054) joins the list of sub-phases. (recommended, not
    asked)
13. The lead names, in one sentence, what the language and std gained,
    as `[0.3.0]`'s names what shipped. (recommended, not asked)
14. One commit, README and runbook included, as `eab7da8e`, titled
    `release: v0.5.0, Phase 5 released`. (recommended, not asked)
15. The README's two pointers are Markdown links, its first.
    (recommended, not asked)
16. The tag message (§3.7) and the try (§5) as drafted. (recommended,
    not asked)
17. The agent merges and starts the workflows, each on the user's word.
    (recommended, not asked)
18. The prototype is measured on CI before the plan (§7), as for 5.5a,
    5.5b and 5.6a. (recommended, not asked)
19. No new test (§6). (recommended, not asked)

## 13. Gate

5.6b is done when:

- the release pull request is merged by rebase, its CI and `main`'s
  green as §4 lists, the lock changed in its 23 lines only;
- the tag `v0.5.0` is on the release commit, and its GitHub release has
  the eight assets and `SHA256SUMS`, every sum `OK`, with
  `releases/latest` naming it;
- all 20 crates are on crates.io at 0.5.0, and the token is revoked;
- the extension 0.5.0 is in the Marketplace and in Open VSX;
- "Release gate" passed all six jobs on 0.5.0;
- the user has tried the published `nova`, and said so.

Then the records pull request opens.
