# Phase 6.0: the plan and ADR 0062

- **Date:** 2026-11-26. **Status:** approved by the user (2026-11-27), as
  drafted.
- **The user's decisions** of 2026-11-26 (P1 to P12, the plan, the local
  target, Q1 and Q2) are quoted in §11, with the spec's own.
- **Branch:** `claude/loving-archimedes-frtf5q-60`, from `main` at
  `4fe11d55` (Phase 5 closed by ADR 0061, #152; `v0.5.0` released).
  Nothing is pushed without the user's word.
- **The records' rule:** every record this pull request writes names
  `main`'s commits only, by 8 characters; pull request numbers are allowed;
  no `proto/`, `split/` or `-ci` branch, no pull request head or pre-rebase
  hash, no local or scratch path, no person's name or email, no model
  identifier. The spike is "a spike of 2026-11-26, its code not
  committed"; the user is "the user" (decision 16).
- **Inputs:** the approved plan; the research at `4fe11d55`; the spike's
  report, in scratch space; the 5.6c spec; ADRs 0026, 0038 and 0048 (the
  scope ADRs), and 0061; the master spec's Phase 6, §1.9 and §7.

## 1. What 6.0 delivers

Phase 6's first pull request, documentation only (Q2), as #128 carried
Phase 5's plan: **`docs/phase-6-plan.md`**, the approved plan, byte for
byte (§3.1); **ADR 0062**, Phase 6's scope, the playground as S0 in the
browser with the spike's figures, `1.0.0` as the phase's release, and
what Phase 6 leaves out (§3.2); **dated notes** under the master spec's
Phase 6, atop `[Unreleased]` and after the plan's 6.0 entry, and markers
at the mapping ADR 0062 amends in ADRs 0048 and 0061 (§3.3 to §3.6). No
code, test, workflow, manifest, `Cargo.lock`, version or release change;
the spike's patch and entry are 6.1's (§10).

## 2. The starting point

Read at `4fe11d55`; nothing built.

- **`main` is `4fe11d55`,** "docs: Phase 5's records, ADR 0061", #152,
  merged by rebase on 2026-11-26; `main`'s CI run 45834816043 passed, 19
  of 19, accepted on its conclusion and tree.
- **What 6.0 answers:** the master spec's Phase 6 is its list alone
  (`nova-spec/00-MASTER-SPEC.md:935-942`), with no goal, gate or note;
  §7's last item reads `v0.6.0` for it (`:1134`); ADR 0048 (`:44`) and ADR
  0061 (`:93`, `:148`, `:394-395`, `:462-463`) map N built for WASM to
  its playground. `docs/adr/` ends at 0061; `[Unreleased]` is empty
  (`CHANGELOG.md:10-11`); there is no `docs/phase-6-plan.md`.
- **Done outside the repository on 2026-11-26:** the research, P1 to
  P12, the plan approved, the target added locally (stable), the spike
  (its code in scratch space), Q1 and Q2. The repository was never
  edited.
- **The precedent:** #128 carried Phase 5's plan as `a9d876e3` (draft)
  and `4bae15da` (approved), then `d8eb9ac1`, "docs: ADR 0048, Phase 5's
  scope and backend, and the notes for 5.0a", which gave the plan's 5.0
  entry a dated note (`docs/phase-5-plan.md:308`). No phase plan had a
  CHANGELOG note; ADR 0026 had an Added line (`CHANGELOG.md:961`), ADR
  0048 the 5.0a line that opened `[Unreleased]`.

## 3. The changes, file by file

### 3.1 `docs/phase-6-plan.md`

The approved text, 750 lines, byte for byte; its status already reads
"approved by the user (2026-11-26)". **No line must change** (decision
7). Its `path:line`s are `4fe11d55`'s, as its status says; the records
commit moves five, which check 2 reads there: the master spec's `:996`,
`:1126-1134` and `:1134` (plan `:5`, `:32-33`, `:306`), by §3.3's note,
and `CHANGELOG.md:219` (plan `:185`), by §3.4's. Its 6.0 note is §3.5's.

### 3.2 ADR 0062, `docs/adr/0062-phase-6-scope.md`

The series' form (`0026-phase-3-scope.md`, `0038-phase-4-scope.md`,
`0048-phase-5-scope-and-backend.md`), ADR 0048's shape (decision 1).
Target 240 to 300 lines:

| Part | Lines | What it holds |
|---|---:|---|
| Title | 1 | `# ADR 0062 — Phase 6's scope, the playground and 1.0.0` |
| `## Status` | 10 | drafted below |
| `## Context` | 40 | §3's Phase 6, "1.0 Release", weeks 105 to 120, seven items (`:936-942`), no goal and no gate, unlike Phases 0 to 5 (`:935-943`); §1.9 (`:99-102`); §5.3's "Link to docs" and §7's `v0.{phase}.0`, by section, since §3.3's note moves their lines; ADR 0048 `:44` and ADR 0061 `:93` (R5), `:148`, `:394-395`, which give the playground N built for WASM, where `:940` names no stage; the research at `4fe11d55`, a line per item (`docs/phase-6-plan.md` §2); why not N: its MIR native only (ADR 0057 `:521-534`), its `async fn main` never collecting mid-compile (ADR 0040 `:42-43`); S0 and N named; the user's decisions, labelled P1 to P12, Q1 |
| `## Decision` | 125 | the twelve decisions, drafted below |
| `### The spike's figures` | 45 | drafted below |
| `## Consequences` | 28 | drafted below |
| `## References` | 14 | the master spec §1.9, §3, §5.3, §7; `docs/phase-6-plan.md` §1, §3, §4's 6.0 and 6.1; this spec and its plan, in ADR 0061's form; `docs/phase-5-plan.md` §2 item 9; ADRs 0026, 0037, 0038, 0039, 0040, 0047, 0048, 0052, 0057, 0061; `crates/nova-codegen-wasm/src/lib.rs:61-63`, `:198-208`; `crates/nova-driver/src/analyze.rs:25-42`, `lib.rs:865-892`; `ci.yml`'s browser job |

**The Status, drafted** (dated by the records commit's day, decision 15):

```markdown
## Status

Accepted (2026-11-26). Branch `claude/loving-archimedes-frtf5q-60`
(`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`
§3.2), with `docs/phase-6-plan.md`, which the user approved on
2026-11-26. The scope is the user's decisions of 2026-11-26, P1 to P12
(the plan's §1), with the `wasm32-unknown-unknown` target allowed
locally for the spike and, after it, the long-function limit caught on
the page (Q1). This ADR confirms the plan's §3 decisions 1 to 3; the
others are confirmed in their sub-phases' specs. It names `main`'s
commits only.
```

**The Decision, drafted:**

```markdown
## Decision

1. **Scope: all seven items** (P1). Each item's in-repository part is
   built and gated (`docs/phase-6-plan.md` §1). The external parts are
   the user's: an audit firm, the TechEmpower pull request, the launch
   posts, a domain and its DNS, switching GitHub Pages on, and any
   account or money. Phase 6's closing ADR records each, done or not,
   and none blocks `1.0.0`.
2. **What Phase 6 leaves out,** each to the backlog unless a sub-phase's
   spec pulls it back in with a ruling that names this ADR:
   - N built for WASM (decision 3);
   - a database driver (decision 9);
   - UDP and Unix sockets, AEAD, `Queue`, `RwLock` and atomics, of
     `nova-spec/20-STDLIB.md` §1's index (decision 10);
   - ADR 0061's backlog, the bootstrap on macOS and Windows, N on a
     large-stack thread, E0093's `\` on Windows and compiler speed among
     it, but for the two bugs decision 10 fixes;
   - what other specs give Phase 6 and its list does not name: multiple
     toolchains (`nova-spec/40-TOOLING.md:646`; ADR 0037 `:93`),
     `nova-embed` (`nova-spec/13-RUNTIME.md:480`), the `setup-nova`
     action (`40-TOOLING.md:583-598`), examples 08 to 13
     (`nova-spec/60-EXAMPLES.md:801-812`), and edition rules
     (`40-TOOLING.md:436-439`).
3. **The playground is S0 in the browser** (P2), as the spike below
   showed it can be. S0's front end, from the lexer to MIR, and its WASM
   backend, built for `wasm32-unknown-unknown`, compile one source text
   in the page, with std embedded, and return the diagnostics as text,
   or `app.wasm` and `app.js`. The program runs in a sandbox away from
   the site's origin, through hand-written JS glue, with no
   `wasm-bindgen`.
   - **It amends ADR 0048 (`:44`) and ADR 0061 (`:148`, `:394-395`),**
     which gave the playground N built for WASM. N built for WASM goes
     to the backlog. The master spec is not amended: its "WASM-compiled
     Nova compiler" (`:940`) names no stage.
   - **The limits,** which the spike met: the compiler under 5 MB
     gzipped; hello world compiled in under 2 s in Chromium; every depth
     case at the parser's limit, 128, compiled and run. 6.1's gate
     measures the size and hello world's time again, in CI.
   - **A long function body** overflows the stack in the browser where
     natively it builds: codegen's structurer recurses once per
     sequential join or loop, which is why S0 compiles on a 256 MB
     thread (`crates/nova-codegen-wasm/src/lib.rs:61-63`). The page
     catches the overflow and says so (Q1). S0's codegen does not change
     before the freeze; an iterative structurer goes to the backlog.
   - **What 6.1 builds,** asked as decision 12 says: the browser's entry,
     a new crate or a split of `nova-driver`, either a change to
     `Cargo.lock`; codegen's thread kept off `wasm32` by a `cfg`
     (`lib.rs:198-208`); a larger shadow stack, set at link time; a print
     hook in the shim; the target in CI. No new crate or feature.
4. **The version: `1.0.0`** (P3) is Phase 6's release, in place of the
   `v0.{phase}.0` of the master spec's §7, item 6, which reads `v0.6.0`
   for this phase. `docs/releasing.md` runs unchanged with
   `version=1.0.0`. A `1.0.0-rc.N` comes only if the audit, the freeze
   or the Release gate asks for one, on the user's word. Semver is strict
   after it (`:101`).
5. **The stability promise** (P6), written in 6.2 and in force from
   `1.0.0`, covers the language at edition `"2026"`, std's public API on
   both targets, the CLI's commands, documented flags and exit codes,
   `nova.toml`, `nova.lock` and the index line. It does not cover the
   hidden flags, the Rust crates, whose API is internal, N, diagnostics'
   wording, the generated JS's and WASM's internals, the playground, or
   performance; 6.2's spec confirms the details. **Breaking changes
   before `1.0.0`:** §1.9 asks an RFC for each (`:100`). As in Phases 1
   to 5, a sub-phase's approved spec and its ADR stand as that RFC, for
   6.3 to 6.5 too; the RFC process 6.2 writes governs every change after
   `1.0.0`.
6. **Documentation 100%** (P5), each part held by tests where it can be:
   std's documentation on the site, with `std/dom`, which this ADR takes
   off ADR 0047's backlog (`:180-184`); `missing_docs` on the Rust
   library crates; an error-code index, each rendered diagnostic linking
   to its entry (the master spec's §5.3); a language reference, the CLI and
   `nova.toml` references, and a book.
7. **The audit's in-repository gate is hardening** (P7): a threat model,
   the `unsafe` and dependency inventories, fuzz targets, a dependency
   check in CI and `SECURITY.md`. The external audit is the user's,
   recorded as not done unless booked; if the user books a firm in time,
   `1.0.0` may wait for its findings, on the user's word.
8. **The site is a project site on GitHub Pages** (P4), under `/nova/`,
   which the user turns on, built by `nova-doc` as the plan recommends
   and 6.8's spec confirms. There is no `novalang.dev`
   (`nova-spec/40-TOOLING.md:626`); a custom domain may follow, if the
   user buys one.
9. **TechEmpower** (P8): the test types that need no database, JSON and
   plaintext, as an app in the repository, with how to submit it. The
   pull request to TechEmpower is the user's.
10. **Before the freeze** (P9): an `@deprecated` attribute; fixes to the
    two S0 bugs ADR 0061 keeps (`:391-393`), std's bodies seeing a
    program's impls and float `%` failing `build --release`; issue #3,
    E0902 on `sqrt` under `nova run` on Linux; `std/regex`; and the HTTP
    client, with handlers that can await. **Out** (P10): the rest of ADR
    0061's backlog, and of `20-STDLIB.md` §1's index, as decision 2
    lists.
11. **The MSRV stays 1.78** (P11), with a policy written in 6.2: a raise
    is the user's decision, in a minor release, with an ADR; an advisory
    in a pin held back for 1.78 may force one.
12. **No new crate, rustup target or CI tool without the user's word,**
    asked in its sub-phase's spec once the spec shows the change
    (`docs/phase-6-plan.md` §3 decision 12): the target in CI and the
    entry in 6.1; an oracle crate in 6.4; `rustls` and `webpki-roots` in
    6.5; the dependency check's tool in 6.6; the Pages actions in 6.8.
    The user allowed the target locally, for the spike, on 2026-11-26.
```

**The spike's figures, drafted** (decision 6; every figure the report's):

```markdown
### The spike's figures

Measured on 2026-11-26 in a spike, its code not committed, as
`docs/phase-5-plan.md` §2 item 9 records Phase 5's lexer spike. It built
the front-end crates and `nova-codegen-wasm` of `4fe11d55` for
`wasm32-unknown-unknown` with Rust 1.97, offline, from the crates
`Cargo.lock` holds, with a throwaway entry over the C ABI that copies
about 60 lines of `nova-driver`'s private front end. Its one change to
S0, in its own copy, calls `compile` directly on `wasm32`
(`crates/nova-codegen-wasm/src/lib.rs:198`); without it every compile
fails at run time, starting the thread. The programs: hello world;
`traits`, `json_round_trip`, `collections` (18 KB) and `iterator`
(22 KB) of the golden suite; `examples/03-producer-consumer`; one with a
type error. Node 22 and Chromium 153 (Playwright's headless shell) on
one machine; times are medians of five compiles in one instance.

| Measure | Figure | Limit |
|---|---|---|
| Size, `gzip -9`, the release profile (opt-level 3) | 830,149 bytes (2,937,593 raw) | under 5 MB |
| Size gzipped, opt-level `z`, LTO, one codegen unit, stripped | 519,967 bytes (1,534,479 raw) | |
| Hello world, the first compile in Chromium | 197 ms (126 ms at `z` with LTO) | under 2 s |
| Hello world, later compiles | 34.2 ms in Chromium, 28 ms in a worker, 40.3 ms in Node | |
| The seven programs, later compiles | 26.9 to 40.8 ms in Chromium, about 30 ms of each std's | |
| The compiler ready after navigation | 119 ms (60 ms at `z` with LTO) | |
| The depth limit, 128 | 15 kinds build and run at 128 in Node and in Chromium, on the main thread and in a worker, the 16th (nested `fn` items) E0900 as natively; 206 probes in Node and 100 a thread in Chromium, no trap; past 128, P0002 | 128 survived |
| V8's stack at 128 | 350 to 450 KB of Node's 984 KB; a worker's stack about 40% of the main thread's | |
| Rust's shadow stack, 1 MiB by default | 768 KiB passes at 128; 512 KiB traps on 20 of 42 probes | |
| Memory | 2.25 MB fresh; about 10 MB after one compile; 41 MB for a 140 KB function; flat over 50 compiles | |
| Output | the six that run print what `nova run` prints; the type error's text is `nova run`'s but for the file name; `app.wasm` and `app.js` equal `nova build --target wasm`'s, byte for byte | |
| A long body, the main thread | 3,000 `if`s in a row compile, 4,000 overflow; 1,200 `while` loops compile, 1,500 overflow | caught (decision 3) |
| A long body, a worker | 1,500 `if`s compile, 2,000 overflow; 600 loops compile, 800 overflow | caught |

Not measured: a build on Rust 1.78; the size after `wasm-opt`; Chromium
in CI, on phones and on slow machines; a real network's transfer time; a
worker's exact headroom; `std/dom` programs. 6.1 measures the first
three it can.
```

**Consequences, drafted:**

```markdown
## Consequences

- **The master spec's Phase 6 list is not amended;** this ADR records the
  difference, as ADRs 0026, 0038 and 0048 did for Phases 3 to 5, and a
  dated note under the list points here. §7 keeps `v0.{phase}.0` for the
  phases before; Phase 6's milestone is `1.0.0`.
- **ADRs 0048 and 0061 keep their text,** each with a dated note at the
  mapping decision 3 changes. Phase 6 builds nothing in N for the
  playground, which, being S0, needs no port.
- **6.1 inherits the spike's caveats** (a long body; thin headroom in a
  worker and in the shadow stack) and unknowns (1.78; `wasm-opt`, which
  CI's browser job already installs; Chromium in CI), and changes
  `Cargo.lock` and CI's targets on the user's word.
- **The backlog grows** by decision 2's list, which Phase 6's closing ADR
  keeps, with each external item, done or not. Phase 6 is the master
  spec's last; later work goes through 6.2's RFC process.
- **From `1.0.0` on, decision 5's promise holds:** a break needs a new
  edition or a recorded exception.
```

### 3.3 The master spec: the note under Phase 6

After `nova-spec/00-MASTER-SPEC.md:942` and its blank `:943`, before the
`---` at `:944`, so `:935-943` stay as cited (decision 9):

```markdown
**Amended 2026-11-26 (branch `claude/loving-archimedes-frtf5q-60`):**
Phase 6 follows `docs/phase-6-plan.md`, which the user approved on
2026-11-26 and whose §1 gives the gate this list lacks, within the scope
`docs/adr/0062-phase-6-scope.md` records. All seven items are in it: the
in-repository part of each is built and gated, and the external parts
(an audit firm, the TechEmpower pull request, the launch posts, a
domain, switching GitHub Pages on) are the user's, recorded, and do not
block 1.0. Item 5's playground is the Rust `nova`'s front end and WASM
backend, built for WebAssembly and run in the browser, as a spike of
2026-11-26 measured it; building the Nova compiler in `selfhost/` for
WASM, which ADRs 0048 and 0061 mapped here, is on the backlog. Item 4's
site is on GitHub Pages until there is a domain. The phase's release is
`1.0.0`, in place of the `v0.6.0` that §7 would give it.
```

### 3.4 `CHANGELOG.md`: the note in `[Unreleased]`

Phase plans never had one (§2); the user asked for it (Q2). After
`## [Unreleased]` (`:10`) and its blank, before `## [0.5.0]` (`:12`),
with 5.0a's sub-phase line inside it, so 6.10a reads it through as
`[1.0.0]`'s first sub-phase (decision 10):

```markdown
**Recorded 2026-11-26 (branch `claude/loving-archimedes-frtf5q-60`):**
Phase 6.0, the plan and Phase 6's scope (spec
`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`;
ADR 0062, Phase 6's scope; `docs/phase-6-plan.md`). Phase 6, "1.0
Release", will release `1.0.0`, in place of the `v0.{phase}.0` milestone
`nova-spec/00-MASTER-SPEC.md` §7 reserves for a phase, and its
playground will compile Nova in the browser with the Rust `nova`'s own
front end and WASM backend, built for WebAssembly. Nothing in `nova`
changes yet.
```

### 3.5 `docs/phase-6-plan.md`: the note on 6.0

After the 6.0 entry (`:407-425`) and its blank, before `### 6.1` (`:427`),
as `d8eb9ac1` noted 5.0 (decision 8):

```markdown
**Amended 2026-11-26 (6.0, branch `claude/loving-archimedes-frtf5q-60`,
ADR 0062; spec
`docs/superpowers/specs/2026-11-26-phase-6-0-the-plan-and-adr-0062-design.md`):**
the spike ran on 2026-11-26, its code not committed. Option A is within
its spec's limits, measured in Node and a local Chromium: 519,967 to
830,149 bytes gzipped; hello world compiled in 197 ms the first time,
about 34 ms after; every depth case at 128 run, in a worker too. It built
on stable alone; the build on 1.78, the size after `wasm-opt` and
Chromium in CI are 6.1's. A long function body overflows codegen's
recursion in the browser: 6.1 catches it and says so, by the user's
decision of 2026-11-26, and an iterative structurer is on the backlog.
This plan's `path:line`s stay `4fe11d55`'s.
```

### 3.6 ADRs 0048 and 0061: dated markers

Accepted ADRs are not rewritten; a later decision adds a dated note in
place (ADR 0026 `:69`, 0038 `:62`, 0039 `:72`, 0053 `:84`). Each marker
only adds lines (decision 11):
- **`docs/adr/0048-phase-5-scope-and-backend.md`,** after `:44`, inside
  its list item:

  ```markdown
       **Amended 2026-11-26 (branch `claude/loving-archimedes-frtf5q-60`):**
       Phase 6's playground is S0 built for WASM, and N built for WASM is
       on the backlog (ADR 0062);
  ```

- **`docs/adr/0061-phase-5-boundary.md`,** after `:395`, inside the
  backlog's item; `:148` and the Consequences stay Phase 5's record:

  ```markdown
    **Amended 2026-11-26 (branch `claude/loving-archimedes-frtf5q-60`):**
    ADR 0062 makes Phase 6's playground S0 built for WASM, by the user's
    decision of 2026-11-26, and puts N built for WASM on this backlog.
  ```

### 3.7 What else points to Phase 6

| Place | What | Recommendation |
|---|---|---|
| ADR 0061 `:93`, `:148`, `:462-463` | R5; the inventory row; "its one mapping" | stay: Phase 5's record; ADR 0062 and §3.6's marker answer them |
| master spec §7 `:1134` | `v0.{phase}.0` | stays: §3.3's note says it, as §0's row stayed in 5.6c |
| `40-TOOLING.md:646`, `:583-598`; `13-RUNTIME.md:480`; `60-EXAMPLES.md:801-812` | Phase 6 items ADR 0062 leaves out | 6.11's notes, as the plan's 6.11 lists |
| ADR 0047 `:180-184` | `std/dom` on the site, on its backlog | stays; ADR 0062 decision 6 takes it; 6.7a's ADR records it |
| `README.md:1-4`; `selfhost/README.md:208`; `nova-spec/README.md:117-119` | the bundle's opening; Phase 5's plan; RFCs | stay: 6.8 and 6.2 |

## 4. CI

All 19 CI jobs run on the pull request (`ci.yml:3-6`, no path filter)
and on `main` after the merge, with Benchmarks (`benchmarks.yml:3-5`); no
Release run, as `release.yml`'s paths (`:15-27`) hold none of §3's files.
**Expected, as #152's run 45829766657:** Test 2838 0 10, 2838 0 9, 2815 0
9 (`release_manifests` 8, `vscode_extension` 3, `project` 25);
`selfhost.rs` 71 0 3, 69 0 3, 65 0 3; golden 2 passed; bootstrap 1
passed, S1 == S2 == S3, 9539848 bytes each; "Packaged nova" 20 crates,
`nova-cli` 1,759 files; MSRV passed. Recorded, not expected: sizes,
times (macOS `selfhost.rs` 104.82 s on #152, over its 100 s line, the
runner's by the user's word), the advisory step's known failures.

## 5. Testing

No test reads §3's files (`docs/adr`, `CHANGELOG` and the master spec
appear in code comments only; the one `.md` a test reads is the root
`README.md`, `crates/nova-cli/tests/install_scripts.rs:157`). The plan's
scripts check the tree against `4fe11d55` instead:

1. **Files and place:** the plan equals the approved text, byte for
   byte; each block §3 drafts is in its file verbatim, where §3.3 to
   §3.6 put it.
2. **Every `path:line`** resolves (path, line, quoted words): ADR 0062's
   and the notes' at the tip; the plan's and this spec's at `4fe11d55`.
3. **Every hash** is on `main`: `git merge-base --is-ancestor`, or, before
   the shallow boundary `a71d27d9` (`a9d876e3`, `4bae15da`, `d8eb9ac1`),
   GitHub's `list_commits` on `main`, read only.
4. **No private text** in the added lines: no local or scratch path,
   name, email, model identifier, `proto/`, `split/` or `-ci` branch, or
   token; the repository's URLs and the plan's Pages address (`:96`) are
   the records' own.
5. **The figures** equal the spike's report, read locally, not cited.
6. **Dates:** 2026-11-26, or the records commit's day in its places; the
   user's decisions stay 2026-11-26 whatever the day.
7. **The notes:** `release-notes.sh 0.5.0 CHANGELOG.md` exits 0 with
   `4fe11d55`'s lines (the note is in `[Unreleased]`); `Unreleased` now
   exits 0 with the note (1 before); no workflow reads it before a tag.
8. **The diff:** §3's six files, the spec and the plan; nothing under
   `crates`, `std`, `selfhost`, `tests`, `examples`, `.github`, `tools`,
   the manifests or `Cargo.lock`.

## 6. The prototype

A local branch from the spec's commit, never pushed, checked by §5. No
draft pull request measures CI (decision 14): no file CI reads changes.

## 7. The cut

Four commits (decision 13): `docs: the Phase 6 plan, approved` (as
`4bae15da`; the draft was never committed); `docs: the Phase 6.0 spec is
approved`; `Plan Phase 6.0: the plan and ADR 0062`, with any dated
amendment to this spec; `docs: ADR 0062, Phase 6's scope, and the notes
for 6.0` (§3.2 to §3.6, as `d8eb9ac1`). Then the pull request, "Phase
6.0: the plan and ADR 0062", by the template, merged by rebase on the
user's word.

## 8. Records

This spec and its plan, which ADR 0062 cites; §3's files. The spike's
report stays in scratch; ADR 0062 holds its figures.

## 9. Risks

1. **Citations that move:** the plan's five (§3.1), the master spec's
   §5.3 and §7 (ADR 0062 cites them by section), ADR 0061 after `:395`.
2. **Saying more than was measured:** Chromium was local; no 1.78 build;
   no `wasm-opt`; a worker's headroom is an estimate. ADR 0062 says each
   as it was, and the spike's code is not committed: 6.1's CI measures
   again, for good.
3. **The limits** are the user's (the plan's 6.0); approving this spec
   rules them (decision 2).
4. **Editing accepted ADRs:** the markers only add lines; without them,
   ADR 0062 alone records the change, as 5.6c did for ADR 0060's slips.
5. **The shallow clone, the date, `selfhost.rs`'s lines** (macOS over
   100 s on #152): as 5.6c's.

## 10. Not in 6.0

Any code (the `cfg` patch, the entry, the stack size, the print hook, the
page, the CI target: 6.1's); `Cargo.lock`; 1.78 or `wasm-opt` figures;
S0's structurer (backlog); the policies (6.2); the companions' notes
(6.11); ADR 0047; §7; the README; deleting branches.

## 11. Decisions

The user's, quoted as given: each option's name in Thai, as asked, ends
"(แนะนำ)", "recommended"; the text after it says what was chosen.

**2026-11-26.** The start: "เริ่ม Phase 6 ได้เลย".
- **P1** "ครบ 7 ข้อ, งานภายนอกเป็นของคุณ (แนะนำ)": all seven, the
  external parts the user's, not blocking 1.0. **P2** "A สำหรับ 1.0, B ไป
  backlog (แนะนำ)": S0 in the browser after a spike, an ADR, N for WASM on
  the backlog. **P3** "1.0.0 เป็น release ของ Phase (แนะนำ)". **P4**
  "ยังไม่มี ใช้ GitHub Pages ไปก่อน (แนะนำ)". **P5** "ทั้งหมด (แนะนำ)":
  std with `std/dom`, `missing_docs`, a reference, a book, an error
  index. **P6** "ภาษา, std, CLI, nova.toml, nova.lock, index format
  (แนะนำ)". **P7** "ใช้ hardening ใน repo เป็น gate (แนะนำ)". **P8**
  "เฉพาะ test ที่ไม่ใช้ database (แนะนำ)". **P9** "รวม std ที่ยังขาดด้วย",
  then "std/regex, HTTP client": those two, with the break-later items.
  **P10** "ไม่เอาเข้าเลย (แนะนำ)", the third and final answer: none of ADR
  0061's backlog. **P11** "คง 1.78 + เขียน policy (แนะนำ)". **P12**
  "Playground spike ก่อน (แนะนำ)".
- **The plan:** "อนุมัติตามร่าง (แนะนำ)". **The target:**
  "อนุญาตติดตั้งในเครื่องนี้ (แนะนำ)": locally, for the spike.
- **Q1** "จับและแจ้งบนหน้า playground (แนะนำ)": 6.1 catches the overflow
  and says so; S0's codegen untouched before the freeze; the rewrite on
  the backlog. **Q2** "Workflow เต็ม (แนะนำ)": as 5.6c, with the plan,
  ADR 0062 and notes in the master spec and `[Unreleased]`.

**Made in this spec, each (recommended, not asked):**

1. `docs/adr/0062-phase-6-scope.md`, titled for its three main decisions.
2. The limits, the plan's suggestion, met locally; CI's figures are 6.1's
   gate (the plan's §1 item 5). They are the user's: approval rules them.
3. std's checked items not kept across compiles (30 of a 34 ms compile).
4. Chromium measured locally, not in a draft pull request.
5. The 1.78 build and the `wasm-opt` size left to 6.1, whose CI already
   pins binaryen (`ci.yml:306-309`).
6. The figures cited as measured in a spike of 2026-11-26, as Phase 5's
   plan cites its lexer spike (`docs/phase-5-plan.md:184-193`).
7. The plan committed verbatim, its citations `4fe11d55`'s.
8. A dated note in the plan's 6.0 entry, as 5.0a's (`:308`).
9. The master spec's note "Amended", after the list; §7 untouched.
10. The CHANGELOG note dated, holding 5.0a's sub-phase line.
11. Dated markers in ADRs 0048 and 0061, adding lines only.
12. Pre-1.0 RFCs as the plan's §3 decision 1 words them.
13. Four commits, the plan's first, as in #128.
14. No draft pull request to measure CI.
15. ADR 0062 dated by its commit's day, not citing its own pull request.
16. `main`'s commits only, by 8 characters; no account named in ADR 0062.

## 12. Gate

6.0 is done when its pull request is merged by rebase on the user's word,
its CI green with §4's counts and `main`'s green on the merged commit;
the plan, ADR 0062 and §3.3 to §3.6's notes are on `main`; §5's checks 2,
3 and 7 hold there; and the figures are within the limits the user
ruled (the plan's 6.0 gate). Then 6.1, and 6.2 beside it, wait for the
user's word; 6.1's spec asks for the CI target and the entry.
