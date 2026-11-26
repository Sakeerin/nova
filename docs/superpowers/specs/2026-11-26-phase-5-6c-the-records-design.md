# Phase 5.6c: the records, ADR 0061

- **Date:** 2026-11-26. **Status:** approved by the user (2026-11-26), as
  drafted, with R7: a dated note in the runbook's Open VSX step (§3.5).
- **The user's decisions:** 5.6's 1, 6 and 8 (2026-11-22), 5.6b's 5 and 7
  (2026-11-23), and the records' R1 to R7 (2026-11-26), quoted in §11 with
  the spec's own, each marked "(recommended, not asked)".
- **Branch:** `claude/loving-archimedes-frtf5q-56b-records`, from `main` at
  `2842d63c` (5.6b merged as #151; 5.6b spec decision 9). Nothing is pushed
  without the user's word.
- **The records' rule:** every record this pull request writes names
  `main`'s commits only, by 8 characters; pull request numbers are allowed;
  no `proto/`, `split/` or `-ci` branch, no pull request head or pre-rebase
  hash, no local or scratch path (decisions 7 and 8).
- **Inputs:** ADR 0047 (the shape); ADR 0037 (written after its release, as
  0061 is); `cc829f58`, `v0.4.0`'s records (#127); the 5.6b spec and plan;
  ADR 0060; the release's ledger; the research of 2026-11-26 at `2842d63c`.

## 1. What 5.6c delivers

The third of 5.6's three pull requests (5.6 decision 1), after the release,
like #127 (5.6b decision 7), documentation only: **ADR 0061**,
`docs/adr/0061-phase-5-boundary.md`, with Phase 5's inventory, deviations,
"The gate's record" (every run and check of the release, and the user's
try), backlog and what was found outside the boundary (§3.1); and **dated
notes pointing to it** in `CHANGELOG.md`'s `[0.5.0]`, with the run IDs, the
master spec's Phase 5 gate, `docs/phase-5-plan.md`'s 5.6,
`selfhost/README.md`'s records and the runbook's step 6.2 (§3.2 to §3.5).
No code, test, workflow, version or release changes.

## 2. The starting point

Read at `2842d63c`; nothing built.

- **`main` is `2842d63c`,** "release: v0.5.0, Phase 5 released", after
  `214aa18a` (the 5.6b spec) and `c424fb16` (its plan): #151, merged by
  rebase on 2026-11-24. `main`'s CI run 45352790982 passed, 19 of 19.
- **The release is done:** the 5.6b plan's Task 3, Steps 1 to 10, each met
  on the user's word, from `main`'s CI to the tag, the GitHub release, the
  20 crates, the token revoked, both stores, the Release gate and the
  user's try; 5.6b's gate, its six items met on 2026-11-26.
- **What promises ADR 0061:** `CHANGELOG.md:25-27`; `docs/releasing.md:537`;
  the tag's message; the master spec `:913-914`; the phase plan `:79`,
  `:239-244`, `:685-686`, `:704-705`, `:720-721`, `:731`, `:776` (the
  pinned `clang`); ADRs 0048 `:85-86`, 0059 `:739`, 0060 `:12-14`, `:217`,
  `:573-574`, `:642-643`. No ADR 0061 exists; `[Unreleased]` is empty;
  `selfhost/src` is `83ba8210`'s (5.5b's last commit).

## 3. The changes, file by file

### 3.1 ADR 0061, `docs/adr/0061-phase-5-boundary.md`

ADR 0047's shape (560 lines), with ADR 0037's closing line, since the tag
already exists. Target 560 to 620 lines:

| Part | Lines | What it holds |
|---|---:|---|
| Title | 1 | `# ADR 0061 — Phase 5's boundary` |
| `## Status` | 9 | drafted below |
| `## Context` | 55 | the §7 Definition of Done, as ADR 0047 `:12-18`; where Phase 5 promises: the master spec's list, "item N" (`00-MASTER-SPEC.md:746-751`), its gate (`:753`), §0's "Self-hosting target" (`:22`); the phase plan's decisions, "decision N" (`docs/phase-5-plan.md:197-294`), and sub-phases 5.0 to 5.6; ADRs 0048 to 0060, a line each; fifteen pull requests merged from 2026-10-27 to 2026-11-24 (the table below), 122 commits from `cc829f58` to `2842d63c`, nine measurement drafts closed unmerged (#134, #136, #137, #139, #142, #144, #146, #148, #150), by number; `v0.5.0` released, as ADR 0037 `:38-45` lists `v0.3.0`'s; the user's decisions, numbered: 5.6's 1, 6, 8; 5.6b's 5, 7; R1 to R6 |
| `### The inventory` | 70 | legend; about 44 rows; the status words (drafted below) |
| `### Deviations, each decided by its record` | 22 | "no deviation is decided here"; three short `####`, citing, not restating: item 5 replaced by textual `.ll` through `clang` (ADR 0048 decision 3; ADRs 0058, 0059); item 3's MIR native only (ADR 0057 §6); the gate on Ubuntu alone, no tolerance on the bytes, ADR 0060 §2's four conditions named in one line (ADR 0060 §2, §11; 5.6 decision 6) |
| `### Rows whose record already explains them` | 18 | S1 by S0 `--release`, not Cranelift (ADR 0060 §4); N not shipped (5.6 decision 8; decision 5); decision 3's `.ll` differential since 5.5a (ADR 0058; ADR 0060 §6); decision 7's job as a step (ADR 0060 §11); `build` refused on Windows (ADR 0059 §4); depth limit 128 (ADR 0052); N's speed (ADR 0052 `:1095-1097`; ADR 0057's lines); std shared (ADRs 0055, 0056) |
| `### The gate's record` | 125 | drafted in full below |
| `### The backlog` | 45 | "unscheduled … no date, no phase and no order" (ADR 0047 `:425-426`); the nine groups below |
| `### Found outside this boundary` | 30 | outlined below; "recorded, not decided" |
| `## Consequences` | 38 | §7 one by one: 1 #151's run 45345843046 and `main`'s 45352790982; 2 the bootstrap step and `selfhost/README.md`'s chain by hand; 3 `[0.5.0]`'s Added, std's site, `selfhost/README.md`, the root README's "Self-hosting", the notes; 4 `[0.5.0]` with its note of 2026-11-26; 5 ADRs 0048 to 0060 and this one; 6 `v0.5.0` on `2842d63c`, its publishes recorded. The closing line. A `v0.5.0` user relies on the "shipped" rows; N is "built"; the backlog promises nothing, Phase 6's playground its one mapping; notes point here from §3.2 to §3.5's five places, not the root README (R6); `[0.5.0]`'s lead keeps "will be recorded", and the published notes are not regenerated |
| `## References` | 13 | this spec and plan; the master spec §0, §3, §7; the phase plan; `docs/releasing.md`; ADRs 0025, 0037, 0047, 0048 to 0060; the fifteen pull requests, and #148 and #150 as ADR 0060 and the 5.6b spec cite them; CI 45142347493, 45345843046, 45352790982; Release 45345843088, 45373121044; Publish to crates.io 45548505763; Release gate 45596343684; `ci.yml`'s Selfhost job; `selfhost.rs` |

**The Status, drafted:**

```markdown
## Status

Accepted (2026-11-26). Branch `claude/loving-archimedes-frtf5q-56b-records`
(`docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md`
§3.1). Written after the release, as ADR 0037 was: `v0.5.0` is tagged on
`2842d63c` and published. The three pull requests of 5.6, the release's
steps and the user's try in place of the runbook's step 6.2 are the
user's decisions of 2026-11-22 and 2026-11-23; what this ADR records of
two rulings not carried out, of the try and of Open VSX, what it maps to
Phase 6, and where it is pointed to, the user's of 2026-11-26. All are
listed below. It names `main`'s commits only.
```

**The inventory's legend, header and three sample rows:**

```markdown
"Item N" is the master spec's Phase 5 list
(`nova-spec/00-MASTER-SPEC.md:746-751`). "Decision N" is one of
`docs/phase-5-plan.md`'s §3 decisions, and "5.Nx" one of its sub-phases,
with its pull request and its commits on `main`.

| Promise | Source | Status | Record |
|---|---|---|---|
| Port the lexer to Nova | item 1; 5.1a, #131, `25be9f3c..4f32f13d` | built: `selfhost/src/lexer.nova` and `token.nova`, held to S0's token dump by the differential and by a fuzzer on fixed seeds, on three systems | ADR 0051 |
| Backend bindings: "bind LLVM via `unsafe` extern from Nova" | item 5; decision 2 | replaced: N writes LLVM IR as text, as S0 does, and runs `clang` 18.1.3 and `cc` on it; no binding, no `unsafe` | ADRs 0048 (decision 3), 0058, 0059 |
| The gate: "CI builds stage2 == stage1 byte-for-byte (or close …, document tolerances)" | the master spec's Phase 5 gate; decision 3 | met on Ubuntu, with no tolerance: S1 == S2 == S3, byte for byte, in `main`'s CI run 45142347493 and in the release pull request's run 45345843046; macOS and Windows on the backlog | ADR 0060; "The gate's record" |
```

**The other rows, outlined:** items 2, 3 (built differently: MIR native
only), 4 and 6 (built differently: stage1 by S0 `--release`, and stage3);
the gate's "document tolerances" (met: none, under four conditions); §0's
"Self-hosting target" (met, as `v0.5.0`); decisions 1 and 4 to 10 as they
bear; a row per sub-phase from this table (5.0a to 5.0c "shipped", being in
the released `nova`; 5.1a to 5.6a "built; its gate met"; 5.6b "done …
recorded"; 5.6c "done: this ADR and the notes"); ADR 0048's left-out list
(`0048:36-45`), a row each, "left out; backlog", N for WASM "Phase 6"; and
two rows "not done" (R1): N on a large-stack thread (ADR 0055 §12), and
E0093's `\` on Windows (ADR 0053).

| Sub-phase | PR | `main` | ADR | | Sub-phase | PR | `main` | ADR |
|---|---|---|---|---|---|---|---|---|
| 5.0a | #128 | `cc829f58..a3a95a2e` | 0048 | | 5.3b | #141 | `f1f17328..9389a84b` | 0056 |
| 5.0b | #129 | `a3a95a2e..e13630f0` | 0049 | | 5.4 | #143 | `9389a84b..1570ddbd` | 0057 |
| 5.0c | #130 | `e13630f0..25be9f3c` | 0050 | | 5.5a | #145 | `1570ddbd..e71446df` | 0058 |
| 5.1a | #131 | `25be9f3c..4f32f13d` | 0051 | | 5.5b | #147 | `e71446df..83ba8210` | 0059 |
| 5.1b | #132 | `4f32f13d..1fa7cd3f` | 0052 | | 5.6a | #149 | `83ba8210..ed165975` | 0060 |
| 5.2a | #133 | `1fa7cd3f..2d38ad7a` | 0053 | | 5.6b | #151 | `ed165975..2842d63c` | — |
| 5.2b | #135 | `2d38ad7a..2a67c6ce` | 0054 | | CI, Rust 1.100 | #140 | `e5c555d0..f1f17328` | — |
| 5.3a | #138 | `2a67c6ce..e5c555d0` | 0055 | | | | | |

**The status words, drafted,** new where "shipped" does not fit N, which is
in the repository and not in the release:

```markdown
**Status words:**
- *Built:* in the repository, in `selfhost/`, and held by CI on every pull
  request, within the limits its record states; not in the release, which
  ships the Rust `nova` (5.6 decision 8). *Built differently* when its
  record changed how.
- *Shipped:* a `v0.5.0` user can rely on it in the released `nova`, within
  the limits its record states.
- *Replaced:* something else does its job, as its record says.
- *Left out:* nothing implements it. *Not done:* an earlier ruling nothing
  carried out, recorded in "Found outside this boundary".
- *Met:* a gate's checks passed, as the row and its record state them.
  *Done* says the same of a step that is not a feature.
- *Backlog*, *Phase 6:* where a deferred item goes.
```

**"The gate's record," drafted in full,** from the ledger; #151's runs by
number and pull request, never by its head:

````markdown
### The gate's record

**What runs.** On every pull request and push to `main`, the Selfhost
job's Linux leg installs `clang-18` 18.1.3, has S1 build the golden suite,
then runs "The bootstrap, S1 == S2 == S3 (Linux)"
(`.github/workflows/ci.yml:200-233`): `cc`'s and `ld`'s versions, then
`the_bootstrap_holds_s1_s2_s3` (`crates/nova-cli/tests/selfhost.rs:8591`),
which builds S1 by S0's `nova build --release`, S2 by S1 and S3 by S2, in
order, and holds N's `.ll` equal from S0, S1 and S2, then S1 == S2 == S3,
byte for byte. There is no tolerance; the four conditions (one `-o`
basename, one `clang` 18.1.3, one runtime archive, one `cc` and `ld` in a
job) are ADR 0060 §2's. `selfhost/README.md`'s "S1, S2 and S3 by hand"
gives the chain as commands.

**`main`'s CI run 45142347493,**
https://github.com/Sakeerin/nova/actions/runs/45142347493, on 5.6a's last
commit `ed165975`, on 2026-11-23, which `[0.5.0]`'s lead cites: all 19 jobs
passed. 5.6a accepted it on that conclusion, its tree being #149's, whose
run 45139701894 had checked every count; the 5.6b spec read its counts
(`docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md:211-219`):
the Test job 2838 passed, 0 failed, 10 ignored on Linux, 2838, 0, 9 on
macOS, 2815, 0, 9 on Windows; `selfhost.rs` 71, 69 and 65 passed, 3 ignored
each; the golden step 2 passed. Its bootstrap step printed `bootstrap: S1 ==
S2 == S3, 9539736 bytes each`, N's `.ll` 12,760,111 bytes from S0, S1 and
S2, on the runtime `0.4.0-5549317a`, and `1 passed`.

**The release pull request, #151,** whose head has the release commit
`2842d63c`'s tree (it merged by rebase on 2026-11-24, pinned to that head,
and `main`'s tree after it is the head's):
- **CI run 45345843046,**
  https://github.com/Sakeerin/nova/actions/runs/45345843046, on
  2026-11-24: all 19 jobs passed, with every count of run 45142347493, and
  in the Test job `release_manifests` 8, `vscode_extension` 3 and `project`
  25 passed. `selfhost.rs` took 58.66, 92.34 and 108.43 s, under the job's
  lines of 80, 100 and 110 s. "Packaged nova" packed the 20 crates at
  0.5.0, `nova-cli-0.5.0.crate` with 1,759 files, installed `nova` from
  them (`nova 0.5.0`, `runtime: embedded`) and passed its gate on three
  systems; the MSRV job checked the workspace with Rust 1.78. The bootstrap
  step took 29.11 s, under its 75 s line, and printed, its stage times left
  out:

  ```text
  cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0
  GNU ld (GNU Binutils for Ubuntu) 2.42
  bootstrap: S1 == S2 == S3, 9539848 bytes each; N's .ll the same from S0, S1 and S2, 12760111 bytes
  bootstrap: Ubuntu clang version 18.1.3 (1ubuntu1)
  bootstrap: runtime …/runtime/0.5.0-4f8a6f1b/libnova_runtime.a
  test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 73 filtered out; finished in 28.98s
  ```

  S1 is 112 bytes larger than at `ed165975`, on the 0.5.0 runtime; sizes
  and times are recorded, not expected (ADR 0060 §10). The advisory step
  failed as on #149 and #150, not counted: Linux's `extern_ffi_run`, issue
  #3's E0902 on `sqrt`, and three macOS collector tests. Windows'
  `selfhost.rs`, 1.57 s under its line, is the runner's time (108.49 s in
  run 45142347493).
- **Release run 45345843088:** seven jobs passed; "Publish the release" was
  skipped, as on a pull request. "Prepare the release" found `## [0.5.0]`,
  wrote `SHA256SUMS` over 8 files and printed `publish runs: gh release
  create v0.5.0 --verify-tag --title "Nova 0.5.0" --notes-file notes.md
  dist/*`, with no `--prerelease`.

**`main`'s CI run 45352790982,**
https://github.com/Sakeerin/nova/actions/runs/45352790982, on `2842d63c`,
on 2026-11-24: all 19 jobs passed, the bootstrap step among them. It was
accepted on that conclusion and on its tree, the same as run 45345843046's;
its counts were not read. The Benchmarks run 45352790526 passed. The
release's local figures are the 5.6b spec's amendment, item 7.

**The search of N for `Map` iteration, repeated** (ADR 0060 §5, `:217`), on
`2842d63c`: `git grep -n '\.keys()' -- selfhost/` prints ADR 0060 §5's 17
lines: `resolution_dump.nova` 54, 316, 378, 381, 384; `typeck_body.nova`
125; `fixes.nova` 521, 527, 528, 530, 538, 545; `typeck_pattern.nova` 722;
`typeck_expr.nova` 281; `typeck_ty.nova` 184; `typeck.nova` 879, 1027.
`selfhost/src` is `83ba8210`'s, the tree ADR 0060 searched.

**The publishes,** from the tag `v0.5.0`, which the user made, annotated,
on `2842d63c` and pushed on 2026-11-24, its message naming this ADR; each
step on the user's word:
- **The Release workflow,**
  https://github.com/Sakeerin/nova/actions/runs/45373121044, on the tag,
  passed all eight jobs on 2026-11-24, "Publish the release" among them.
  Its GitHub release, "Nova 0.5.0", is neither a draft nor a prerelease;
  its notes are `[0.5.0]` as the release commit has it, 614 lines. Its
  eight assets, each named for `0.5.0`, are the two install scripts, the
  four archives, std's site and the `.vsix`, with `SHA256SUMS`; each
  asset's digest, as GitHub's API gives it, equals its line of `SHA256SUMS`
  as "Prepare the release" printed it. It is the latest release.
- **"Publish to crates.io",**
  https://github.com/Sakeerin/nova/actions/runs/45548505763, from the tag,
  version `0.5.0`, started on 2026-11-25 and approved by the user under
  "Review deployments", passed. Its token, the user's, was
  **publish-update** alone, for `nova-*`; the agent never saw it. It printed
  `Publishing 0.5.0 from refs/tags/v0.5.0 (2842d63c…)`, `package-gate: 20
  crates at 0.5.0`, `nova-cli-0.5.0.crate` with 1,759 files, `nova 0.5.0`,
  `runtime: embedded`, `gate: the WASM build prints Hello, Nova! under
  Node`, `gate: passed`, `package-gate: passed in 224 s`, and for each of
  the 20 `on crates.io, owned by` the repository's owner: no crate was new.
  Each was published at its first attempt, with no rate-limit wait; the
  summary lists all 20 `published`, none skipped. The index, by the user's
  command, has a line `"vers":"0.5.0"` for each. The user then revoked the
  token and deleted the secret.
- **The extension,** 0.5.0, in both stores on 2026-11-25: the user uploaded
  the release's `nova-vscode-0.5.0.vsix`. Their screenshots: the
  Marketplace's listing, version 0.5.0, last updated 11/25/2026,
  `sakeerin.vscode-nova`, the repository's link; Open VSX's, Nova Language
  Tools, version 0.5.0 with the alias `latest`, the README, the namespace
  still unverified. The user confirmed `[0.5.0]` on both and the tokens
  removed.
- **"Release gate",**
  https://github.com/Sakeerin/nova/actions/runs/45596343684, version
  `0.5.0`, from `main` at `2842d63c`, on 2026-11-25: all six jobs passed,
  each on the tag. "From crates.io", on Linux, macOS and Windows, ran
  `cargo install nova-cli --version "=0.5.0" --locked` and printed
  `Installed package nova-cli v0.5.0`, `nova 0.5.0`, `runtime: embedded`,
  `packages-gate: passed` and `doc-gate: passed`. "From the install script"
  printed `nova: checked nova-0.5.0-<target> … against SHA256SUMS` on each,
  Windows' under PowerShell 7 and Windows PowerShell 5.1. All six ran
  `gate.sh --test --node`, the flag found by the greps of
  `.github/workflows/release-gate.yml:83` and `:169` in the tag's `gate.sh`
  (`:45`), and printed `gate: the WASM build prints Hello, Nova! under
  Node` and `gate: passed`.
- **The user's try of the published `nova`,** on 2026-11-26, in place of
  the runbook's step 6.2 (its note of 2026-11-24), on Windows: `cargo
  install nova-cli --locked`; `nova --version` printed `nova 0.5.0`; `nova
  new hello` printed "created `hello`: nova.toml, .gitignore, README.md,
  src/main.nova", and `nova run`, in `hello`, `Hello, Nova!`. They
  reported it in their words "ลองแล้ว nova --version ได้ nova 0.5.0 และ
  Hello, Nova!" ("tried it: nova --version gives nova 0.5.0, and Hello,
  Nova!"), with two screenshots.

**What went otherwise than the runbook says,** recorded as lessons for its
next use, not as blame (the user's decision 3):
- **Open VSX's token, first try:** the runbook reads it with bash's `read
  -rs` (step 4, item 2); in Windows PowerShell 5.1, whose `Read-Host` has
  no `-MaskInput`, it was shown on screen. Nothing was published by that
  try; the token was revoked, and a new one made and read masked.
- **Open VSX held 0.5.0 inactive** for a few minutes after the upload,
  until its check passed, so a second `ovsx publish` answered "Extension
  sakeerin.vscode-nova 0.5.0 is already published, but currently isn't
  active". It was not retried (`docs/releasing.md:735-738`), and the
  listing then showed 0.5.0 as `latest`.
````

**The backlog's nine groups,** each item with its record, as ADR 0047
`:428-466`:
1. left out by ADR 0048 (`0048:36-45`);
2. the bootstrap on macOS and Windows, with an unpinned `cc` and `ld` (ADR
   0060 `:642-643`; ADR 0059 §4; `selfhost/README.md:66`; ADR 0060 `:219`);
3. N's coverage of S0 (ADR 0057 §6; ADR 0059 `:740-745` and §7, among it
   **E0093's `\` on Windows**, R1; ADR 0054 `:516-521`);
4. the language and std (the 5.0 spec's §11; ADRs 0049, 0050);
5. speed and robustness (decision 8; the lazy splice, ADR 0057; **N on a
   large-stack thread**, R1);
6. S0's bugs kept (ADR 0056 `:1220-1221`; ADR 0058 `:885-886`);
7. mapped elsewhere, Phase 6 alone (R5): N for WASM, for the playground
   (`00-MASTER-SPEC.md:923`);
8. taken from earlier backlogs (`0025:327`, `:337`, built; `0047:443-445`);
9. the process: the README's relative links in the archives. (R3's
   lessons go into the runbook's Open VSX step, R7, not the backlog.)

**"Found outside this boundary," outlined** (recorded, not decided):
- **ADR 0055's ruling of 2026-11-06** (`0055:524-527`), N on a large-stack
  thread, was not carried out: N runs on its 1 MB main thread
  (`selfhost/src` starts none). It survives 64 linked associated types
  each nesting 124 arrays (7,936 levels), overflows at 125 (8,000), and
  nothing in the corpus, cases or fuzzer comes near (`0055:518-524`).
- **ADR 0053's "5.5's driver settles it"** (`0053:773-775`), E0093's `/`
  for `\` on Windows: ADR 0059 §7 keeps it as (differs) (`0059:265`).
- **R9.1:** the 5.6b plan (`:501-507`, `:706-724`) and spec (`:523-531`)
  say `R` holds "the fresh `[Unreleased]`"; it holds each record's text and
  the headings' order, not each place, nor that `[Unreleased]` is empty,
  both checked once on the release commit's tree.
- **R3.1:** the 5.6b plan's `release-gate.yml:75` (`:1803`, `:2681`) is a
  comment; the greps are `:83` and `:169`.
- **No 5.6a note in `14-CODEGEN.md`** (the plan `:748-751`; as ADR 0047
  `:491-497`); **the plan's status** still "approved" (`:3`; as ADR 0037
  `:463-464`), and its `:4` says "§1" for §0 (`00-MASTER-SPEC.md:22`).
- **ADR 0060 names two commits not on `main`** (`:180`, `:467`,
  `:487-489`, `:544`); #148's and #149's refs keep them.

**The closing line,** ADR 0037's form (`0037:478`), under the Definition of
Done in Consequences: ``**Phase 5 is complete as `v0.5.0`, within this
boundary.**``

### 3.2 `CHANGELOG.md`: the dated note in `[0.5.0]`

`cc829f58` put `[0.4.0]`'s note after its lead, before `### Added`
(`CHANGELOG.md:646-656`; lead `:630-644`; `### Added` `:658`). `[0.5.0]` has
a sub-phase list between its lead (`:14-28`) and `### Added` (`:58`): the
note takes `:58-69`, after the list and its blank line, and `### Added`
moves to `:71` (decision 9). In `cc829f58`'s form:

```markdown
**Recorded 2026-11-26 (branch
`claude/loving-archimedes-frtf5q-56b-records`):** `v0.5.0` is published:
- the tag `v0.5.0` and its GitHub release (run 45373121044);
- the 20 crates at 0.5.0 on crates.io (run 45548505763);
- the VS Code extension 0.5.0, in the Marketplace and Open VSX;
- "Release gate", which passed all six jobs (run 45596343684).

The maintainer installed the published `nova` with `cargo install
nova-cli --locked` and ran `nova new` and `nova run` with it. The
boundary Phase 5 closes within is recorded in
`docs/adr/0061-phase-5-boundary.md`, whose "The gate's record" has each
run and the try.
```

The lead's "will be recorded" stays as published (decision 18);
`[Unreleased]` stays empty, as the records change no behaviour.

### 3.3 The master spec: R4's note

After `nova-spec/00-MASTER-SPEC.md:914`, before the `---` at `:916`, as
Phase 4's note closes its gate (`:726-739`); nothing before Phase 0
(`:560-589`) or in §0 (`:22`):

```markdown
**Recorded 2026-11-26 (branch `claude/loving-archimedes-frtf5q-56b-records`):
Phase 5 is complete as `v0.5.0`, within
`docs/adr/0061-phase-5-boundary.md`'s boundary.** Its inventory takes
Phase 5's promises from the list above and `docs/phase-5-plan.md`, and
says whether each was built, shipped, replaced or left out, and where
each is recorded. N, the compiler written in Nova, is built in
`selfhost/` and held by CI on every pull request; it is not shipped, and
the release ships the Rust `nova`. Item 5 is replaced: N writes LLVM IR
as text and runs `clang`, with no binding (ADR 0048); item 3's MIR is
native only (ADR 0057). The gate is met on Ubuntu with no tolerance, as
ADR 0060 records: stage1 == stage2 == stage3, byte for byte, in `main`'s
CI run 45142347493 and in the release pull request's run 45345843046.
macOS and Windows, and the rest of what Phase 5 leaves out, are on that
ADR's backlog; building the compiler for WASM is mapped to Phase 6's
playground (Phase 6, item 5). Every item of the §7 Definition of Done is
met.
```

### 3.4 `docs/phase-5-plan.md`: the note on 5.6

After the 5.6a note (`:732`), before `## 5. Cross-cutting` (`:734`),
answering `:704-705`'s future tense; the status line (`:3-8`) stays
(decision 13):

```markdown
**Recorded 2026-11-26 (5.6c, branch
`claude/loving-archimedes-frtf5q-56b-records`, ADR 0061; spec
`docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md`):**
5.6's three pull requests are in, and Phase 5 is complete as `v0.5.0`,
within `docs/adr/0061-phase-5-boundary.md`'s boundary:
- 5.6a, the gate, is #149;
- 5.6b, the release, is #151 (spec
  `docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md`):
  the commit `2842d63c`, which the tag `v0.5.0` names, then the GitHub
  release, the 20 crates, the extension in both stores and the Release
  gate, each on the user's word, and the user's try of the published
  `nova`;
- 5.6c, the records: ADR 0061, with the inventory, the gate's record and
  the backlog, macOS and Windows on it, and the dated notes.

ADR 0061 cites ADR 0060 for decision 3's tolerances, and repeats decision
9's search of N for `Map` iteration: the same 17 lines. Two rulings were
not carried out and are on its backlog: N on a large-stack thread (ADR
0055 §12), and E0093's `\` on Windows (ADR 0053).
```

### 3.5 `selfhost/README.md` and `docs/releasing.md` (R6)

- **`selfhost/README.md`,** after `:205`, in the list's form:
  `- ADR 0061: Phase 5's boundary.`
- **`docs/releasing.md:537`,** in place: the 2026-11-24 note's last
  sentence, "`docs/adr/0061-phase-5-boundary.md` will record the try.",
  says what ADR 0061 records, with the try's own date, so the note's date
  stays true (decision 15):

  ```markdown
  decision 5). `docs/adr/0061-phase-5-boundary.md` records the try, of
  2026-11-26: on Windows, `cargo install nova-cli --locked`, then `nova
  --version` printing `nova 0.5.0`, and `nova new hello` and `nova run`
  printing `Hello, Nova!`.
  ```

- **`docs/releasing.md`, the Open VSX step (R7),** after item 2's
  parenthesis (`:416-417`), indented as the item, a dated note dated by
  the records commit's day:

  ```markdown
     **Amended 2026-11-26 (branch
     `claude/loving-archimedes-frtf5q-56b-records`):** in Windows
     PowerShell 5.1, whose `Read-Host` has no `-MaskInput`, read the token
     masked with `$s = Read-Host -AsSecureString "token"` and
     `$env:OVSX_PAT = [System.Net.NetworkCredential]::new('', $s).Password`,
     run `npx ovsx publish` in the `.vsix`'s folder, then
     `Remove-Item Env:OVSX_PAT; Remove-Variable s`. Open VSX may hold a
     new version inactive for a few minutes after its upload, while it
     checks it; `ovsx publish` then answers "already published, but
     currently isn't active": wait for the listing, and do not publish
     again (`docs/adr/0061-phase-5-boundary.md`).
  ```

### 3.6 What else is stale

| Place | What | Recommendation |
|---|---|---|
| `CHANGELOG.md:25-27`; the tag's message; ADRs 0048, 0059, 0060 (§2); the 5.6b plan and spec | "will record"; R9.1's wording; R3.1's line | stay: published, or records of their day; ADR 0061 answers each |
| `nova-spec/14-CODEGEN.md`; `docs/phase-5-plan.md:3-4` | no 5.6a note; "approved", "§1" | recorded, not edited (decisions 13, 16) |
| `docs/releasing.md:407-417` | Open VSX's step, in bash only | R7: a dated note after it (§3.5) |
| `selfhost/README.md:66`; `README.md:122-130` | macOS linking not run; no ADR 0061 | stay: true; R6 |

## 4. CI

- **All 19 CI jobs** run on the pull request (`ci.yml:3-6`, no path filter),
  the bootstrap step among them, and again on `main` after the merge, with
  a Benchmarks run. **No Release run:** `release.yml`'s `pull_request`
  paths (`.github/workflows/release.yml:15-27`: the workflow, the scripts,
  build files, manifests, the lock, `tools/vscode-nova/**`,
  `tools/install/**`) hold none of §3's files; the publish and gate
  workflows run only when started.
- **Expected, as #151's run 45345843046:** Test 2838 0 10, 2838 0 9, 2815 0
  9, with `release_manifests` 8, `vscode_extension` 3, `project` 25;
  `selfhost.rs` 71 0 3, 69 0 3, 65 0 3; golden 2 passed; bootstrap 1
  passed, S1 == S2 == S3; "Packaged nova" 20 crates at 0.5.0, `nova-cli`
  1,759 files; MSRV passed. Nothing a count reads changes. Recorded, not
  expected: sizes, the runtime directory, crate bytes, times, the advisory
  step's known failures.

## 5. Testing

No test reads these files: `git grep` over `crates/` finds `docs/adr`,
`CHANGELOG`, `00-MASTER-SPEC`, `phase-5-plan` and `releasing.md` in
comments only; the one `.md` a test reads is the root `README.md`
(`crates/nova-cli/tests/install_scripts.rs:157`), unchanged; `selfhost.rs`
walks `.nova` files only (`:434`); no crate holds `selfhost/README.md`. The
plan checks the records commit instead:

1. **Text and place** (R9.1's lesson): each block §3 drafts is in its file
   verbatim, between the lines §3.2 to §3.5 name.
2. **Every link and `path:line`** resolves at the tip: the path exists, the
   line is in the file, the quoted words are on it.
3. **Every commit hash** is on `main` (`git merge-base --is-ancestor`); the
   clone is shallow at `a71d27d9`, so older ones (`cc829f58`, `a3a95a2e`)
   are checked with GitHub's `list_commits` on `main`, read only.
4. **No private text:** no local or scratch path, person's name, email,
   model identifier, `proto/`, `split/` or `-ci` branch, or token; the
   repository's URLs and the publisher `sakeerin` are the records' own.
5. **The notes:** `release-notes.sh 0.5.0 CHANGELOG.md` exits 0, printing
   the base's 614 lines plus §3.2's; `Unreleased` still exits 1. Nothing
   published changes: `release.yml` writes the notes (`:326`) and the
   release (`:375-395`) only on a `v*` tag push (`:378`), so `[0.5.0]`
   differs from the published body by the note, as `[0.4.0]` has since
   `cc829f58`.
6. **The diff:** §3's six files, the spec and the plan; nothing under
   `crates`, `std`, `selfhost/src`, `tests`, `.github`, `tools` or the
   manifests.

## 6. The prototype

On a local branch from `2842d63c`, never pushed under its own name, checked
by §5. **No CI measurement through a draft pull request** (decision 20): no
file CI reads changes, the pull request's own CI runs all 19 jobs before
the merge, and a draft would add a closed pull request and a run to the
records for nothing. The user decides at that step.

## 7. The cut

Three commits, as 5.6b's: `docs: the Phase 5.6c spec is approved` (as
`docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md`);
`Plan Phase 5.6c: the records, ADR 0061`, any dated amendment appended;
`docs: Phase 5's records, ADR 0061`, the ADR and the five notes in one
commit, as `cc829f58` (decision 19). Then the pull request, "Phase 5.6c:
the records, ADR 0061", by the template as #127's body; merged by rebase on
the user's word.

## 8. Records

This spec and its plan, which ADR 0061 cites; §3's files; nothing in
`[Unreleased]`. The 5.6b spec and plan stay; ADR 0061 records R9.1, R3.1.

## 9. Risks

1. **Citations that move** in the files the commit edits: §5's check 2
   runs at the tip.
2. **The shallow clone:** `git merge-base` cannot place commits before
   `a71d27d9`; the GitHub read covers them (`git fetch --unshallow` would
   need the user's word; the research's one fetch was refused).
3. **Saying more than was checked:** run 45352790982's counts were not
   read; the listings and the index are the user's screenshots; the digests
   were compared through the API. The text says each as it was.
4. **Private text from the ledger** (a token, a login, the user's local
   time or folders): check 4 and the review.
5. **Windows' `selfhost.rs`** near its 110 s line (108.43 s on #151): CI
   does not enforce it; a run over goes to the user.
6. **The date:** a later commit dates ADR 0061 and the notes by its day.

## 10. Not in 5.6c

Any code, test, workflow or version change; any release, tag or publish;
the published notes; the root README (R6); the runbook beyond `:537`;
`14-CODEGEN.md`; edits of the 5.6b spec, plan or earlier ADRs; N's thread
and E0093's `\` (R1). Deleting branches (`-56b`, `-56b-ci`, the earlier
Phase 5 ones, `-56b-records` after the merge) is the user's.

## 11. Decisions

The user's are quoted as given: each option's name, in Thai, as asked, ends
"(แนะนำ)", "recommended"; the text after it says what was chosen.

**5.6's (2026-11-22).** **1.** "5.6a gate → release → บันทึก (แนะนำ)": PR 3
= the boundary ADR and the records closing Phase 5, after the release.
**6.** "Linux อย่างเดียว (แนะนำ)": macOS and Windows in the boundary ADR's
backlog. **8.** "ไม่ส่ง selfhost/ บอกใน notes (แนะนำ)": the archive unchanged.

**5.6b's (2026-11-23), and its spec's.** **5.** "ครบทุกขั้นแบบ v0.4.0
(แนะนำ)": step 6.2 replaced by the user trying the published nova. **7.**
"หลัง publish ครบและคุณลองแล้ว (แนะนำ)": like #127; ADR 0061 records every
release run, the user's check, Phase 5's inventory and backlog, in one PR.
**9.** The branch `-56b-records` (the 5.6b spec's own; its §9 lists the
records).

**The records' (2026-11-26).**
- **R1.** "บันทึกใน ADR 0061 + backlog (แนะนำ)": ADR 0055's thread and ADR
  0053's E0093 `/`, not done, recorded with the depth N survives and put
  on the backlog; documentation only.
- **R2.** "cargo install nova-cli --locked": on Windows; `nova 0.5.0`;
  `Hello, Nova!`; their words in Thai, glossed.
- **R3.** "บันทึกสั้น ๆ ทั้งสองเรื่อง (แนะนำ)": the shown, revoked token and
  0.5.0 inactive until Open VSX's check; no token value; lessons.
- **R4.** "Dated note ใต้ gate ของ Phase 5 (แนะนำ)": at `:913-914`.
- **R5.** "เฉพาะที่ master spec ระบุ (แนะนำ)": Phase 6 gets N for WASM alone.
- **R6.** "selfhost/README + releasing.md (แนะนำ)": not the root README; plus
  the CHANGELOG note and a phase-plan note.
- **R7.** "เพิ่ม note ใน runbook เลย (แนะนำ)": R3's two lessons as a dated
  note in the runbook's Open VSX step (§3.5), not on the backlog.

**Made in this spec, each (recommended, not asked):**

7. `main`'s commits only; the tag by name; #151's runs by number and PR.
8. 8-character hashes, as ADRs 0053 to 0060 (ADR 0047 used 7).
9. The CHANGELOG note after `[0.5.0]`'s sub-phase list: the lead and its
   list stay one block, the note before `### Added`, as `[0.4.0]`'s.
10. "Built" and "not done" beside "shipped": N is not in the release.
11. ADR 0060's tolerances cited, the four conditions named in one line
    (the plan `:720-721`; ADR 0060 `:573-574`).
12. The `Map` search repeated, as ADR 0060 `:217` promises.
13. The plan's status line unchanged: "approved" stays true, and Phase 4's
    plan closed by a note on its last sub-phase alone.
14. The master spec's note "Recorded", as Phase 3's completion (`:581`).
15. `docs/releasing.md:537` changed in place, as R6 says, with the date.
16. `14-CODEGEN.md`'s gap recorded, not filled, as ADR 0047 did.
17. Superseded by R7, the user's: R3's lessons go into the runbook.
18. Run 45352790982 recorded as accepted, counts not read now; `[0.5.0]`'s
    lead kept.
19. One records commit: the ADR and the notes cite each other.
20. No draft pull request to measure CI (§6).
21. ADR 0061 dated by its commit's day, not citing its own PR (as 0047).

## 12. Gate

5.6c is done when the records pull request is merged by rebase on the
user's word, its CI green with §4's counts and `main`'s CI green on the
merged commit; ADR 0061 and the notes of §3.2 to §3.5 are on `main`; every
hash the new text names is on `main`, and every `path:line` resolves at
the merged commit; and `release-notes.sh 0.5.0 CHANGELOG.md` exits 0 there.
Then Phase 5 is closed; Phase 6 waits for the user's word.

## Amendment (2026-11-26)

Dated, with the plan
(`docs/superpowers/plans/2026-11-26-phase-5-6c-the-records.md`), after the
prototype, its checks, the split, the replay, the mutants and the plan's
fact-check. Where this section and the text above differ, this section
holds. It names `main`'s commits only.

1. **The prototype's base** (§6). The prototype branched from this spec's
   commit, `2842d63c` with this spec, not from `2842d63c`; the spec's
   commit changes no file the records touch. It was a local branch, never
   pushed (decision 20).
2. **The release's assets** (§3.1, "The gate's record", the Release
   workflow). "Its eight assets, each named for `0.5.0`, are the two
   install scripts, the four archives, std's site and the `.vsix`" is
   wrong of the install scripts, `install.sh` and `install.ps1`, which are
   not named for a version. ADR 0061 says: "Its eight assets are the two
   install scripts, and the four archives, std's site and the `.vsix`,
   each named for `0.5.0`, with `SHA256SUMS`".
3. **The bootstrap step's lines** (§3.1, "The gate's record", run
   45345843046). The step printed more lines than the block quotes
   (cargo's own, "running 1 test", the test's line), and the block
   shortens the runtime's path to `…/runtime/0.5.0-4f8a6f1b/`. ADR 0061
   says the step "printed these lines among others, its stage times left
   out and the runtime's path shortened:", then the block as drafted.
4. **"(the user's decision 3)"** (§3.1, "What went otherwise than the
   runbook says"). In ADR 0061's own legend, "decision 3" is the phase
   plan's §3 decision 3, the gate; the lessons are the records' R3
   (§11). ADR 0061 says "(R3)".
5. **Citations the records commit moves** (§3.1; §9, risk 1). Cited as
   the records commit leaves the files: `docs/releasing.md:735-738` is
   `:750-753` (the Open VSX note adds 12 lines above it, the `:537`
   sentence 3); `nova-spec/00-MASTER-SPEC.md:923` is `:940` (R4's note
   adds 17); `docs/phase-5-plan.md:748-751` is `:768-771` (the 5.6 note
   adds 20).
6. **What the records say of Open VSX, and of N's stack.** These drafts
   said more than the sources: §3.1's "The gate's record", "Open VSX held
   0.5.0 inactive for a few minutes after the upload, until its check
   passed" and "the user uploaded the release's `nova-vscode-0.5.0.vsix`";
   §3.5's Open VSX note, "Open VSX may hold a new version inactive for a
   few minutes after its upload, while it checks it"; §11's R3, "0.5.0
   inactive until Open VSX's check"; and §3.1's "Found outside this
   boundary", "N runs on its 1 MB main thread". The release's ledger
   shows Open VSX's first try uploading nothing; the second, with a new
   token read masked, answering "Extension sakeerin.vscode-nova 0.5.0 is
   already published, but currently isn't active and therefore not
   visible"; no retry; and the listing then showing 0.5.0 as `latest`.
   When 0.5.0 reached Open VSX, why it was not yet active, and which copy
   of the `.vsix` the user uploaded are not recorded. ADR 0055 §12
   (`0055:495`) says "N runs on its main thread, 1 MB on Windows". The
   records say only what was seen:
   - ADR 0061: the second try's answer, then "when 0.5.0 reached Open
     VSX, and why it was not yet active, are not known"; R3 as "0.5.0
     inactive for a while", the user's words; "the user uploaded
     `nova-vscode-0.5.0.vsix`"; and N "runs on its main thread (1 MB on
     Windows)", ADR 0055 having measured the depth on Linux under
     `ulimit -s 1024`;
   - the runbook's Open VSX note, as §3.5 drafts it but for two
     sentences: "run `npx ovsx publish .\nova-vscode-<version>.vsix` in
     the `.vsix`'s folder", which names the file, and "Open VSX may show
     a new version as already published but not yet active, for a while
     after its upload; `ovsx publish` then answers "already published,
     but currently isn't active": wait for the listing, and do not
     publish again". It is still 11 lines, so item 5's citation stands.
   - R7's note also makes two of the spec's lines stale: §1's, which
     names the runbook's step 6.2 as its only note there, and §10's "the
     runbook beyond `:537`" as not in 5.6c. The runbook carries both
     notes, `:537` and R7's.
7. **The outline, as ADR 0061 fills it** (§3.1).
   - Context lists the user's decisions R1 to R7, not R1 to R6: the
     backlog cites R7, and the Status's last sentence covers it.
   - The decisions are labelled "5.6 decision N", "5.6b decision N" and
     "R1" to "R7", the form of the drafted status words.
   - R9.1 and R3.1, the release ledger's labels, are described, not
     labelled, in "Found outside this boundary": the ledger is not in the
     repository.
   - The inventory has 42 rows, against "about 44": items 1 to 6; the
     gate twice; §0's target; decisions 1 and 3 to 10; a row for each of
     the 15 pull requests and for 5.6c; ADR 0048's six left-out bullets;
     and the two "not done".
   - ADR 0055's overflow is given as ADR 0055 measured it
     (`0055:518-524`): "at 125 (8,000 levels) on the debug runtime, at 125
     or 126 on the release one".
   - References name the plan's path, in ADR 0037's form ("and its
     plan," then the path), where the outline says "this spec and plan".
8. **ADR 0061's length** (§3.1, "Target 560 to 620 lines"). It is 492
   lines. The parts' own targets in the table sum to about 426, and the
   ADR keeps to them but where a part holds more than the table guessed:
   Context 84 lines (55: the pull requests' table and the user's twelve
   decisions), "The gate's record" 142 (125, drafted in full above), the
   backlog 53 (45). The records commit adds 559 lines and removes 1.
9. **§5's checks, as the plan makes them.** One script with a subcommand
   for each of §5's six checks, each reading the working tree against
   `2842d63c`, so that an edit is seen before it is committed: check 3
   holds `cc829f58` and `a3a95a2e`, older than the shallow boundary
   `a71d27d9`, by name, as GitHub's `list_commits` placed them on `main`
   on 2026-11-26; check 4 builds the names it looks for from git, and
   writes none; check 1 also holds ADR 0061's drafted Status, legend,
   sample rows and status words, and the Open VSX note as item 6 gives
   it; check 2's table is keyed by ADR 0061's line, and a subcommand
   rewrites its keys after an edit that moves them. The commit's day is
   put in its six places by a script, which checks that the history's
   days, the user's decisions of 2026-11-26 and the try among them, stay
   as written whatever the day.
10. **§7's cut, as the plan makes it.** Task 0, the baseline; Task 1, the
    records commit, two patch blocks (the new ADR, then the five notes)
    applied in order and committed once, with §7's subject; Task 2, the
    checks, 16 mutants of the commit's files (15 caught; one, the
    parser's inventory row's status changed, accepted: check 1 holds
    three of the 42 rows verbatim, and the other 39 rows' statuses are
    the review's), the review and one fix pass folded into the records
    commit before the push, the pull request and its CI, and the merge;
    Task 3, §12's gate after the merge. No `cargo` command runs locally:
    CI runs all 19 jobs on the pull request (§4).
