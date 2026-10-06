# Phase 2 close-out: design

Branch `phase-2-closeout`, from `main` at `800694c`. The design was approved
section by section in conversation on 2026-10-06; this file is the written spec.

## 1. What this is, and what it is not

**The goal.** Close Phase 2, "Standard Library Core", against
`nova-spec/00-MASTER-SPEC.md` §7's Definition of Done:

1. crates compile and pass CI;
2. gate criteria are met with reproducible commands;
3. documentation exists for new public surface;
4. CHANGELOG is updated;
5. an ADR exists for every decision that deviates from the spec;
6. a `v0.{phase}.0` tag.

The 2026-10-05 assessment found items 1, 2 and 4 met. Item 3 lacks 20-STDLIB
sections for `std/strings` and `std/bytes`. Item 5 lacks records for several
deviations. Item 6 is the tag. PR #99 (2026-10-06) closed the material gap
that assessment named: the collector now runs on Linux and macOS.

**The user's decisions, 2026-10-06:**
- **Record only.** The close-out builds nothing. Each gap is either covered by
  an existing ADR already or is recorded as deferred. `v0.2.0` ships what
  exists.
- **The deferred std pieces go to an unscheduled backlog,** with no phase and
  no date; each is built when a program needs it. Tooling-shaped items map
  where they fit: `salsa` to Phase 3, the LSP; fuzzing to Phase 6, the
  security audit.
- **Approach A:** one consolidated ADR, 0025, "Phase 2's boundary", built
  around an inventory table.
- **The release step bumps the workspace crates from 0.1.0 to 0.2.0,** so
  `nova --version` matches the tag.
- Design sections 1 to 4 were approved as presented: ADR 0025 (§3.1–§3.2),
  the other records (§3.3–§3.4), 20-STDLIB §18 and §19 (§3.5), and the
  release (§3.6).

**Not in scope:**
- building any deferred item;
- fixing the LLVM release test that passes when `clang` is missing (§3.2's
  LLVM row records it);
- Phase 1 drift beyond one line in ADR 0025 (§3.1);
- the 9 deferred minors from PR #99's final review;
- the pre-existing release-only test failure, a separate task.

## 2. Evidence

- **Tags:** `v0.1.0` (Phase 1), then four pre-releases:
  - `v0.2.0-alpha.1` (2026-08-17), "Phase 2 progress";
  - `alpha.2` (2026-09-09), "module inventory complete, gate still open";
  - `alpha.3` (2026-09-11), "gate example exists, ... measured and not met";
  - `alpha.4` (2026-09-12), "measured on both criteria, and met on neither".
- **Crate versions:** no tag ever bumped them; `crates/nova-cli/Cargo.toml:3`
  is still `version = "0.1.0"`.
- **`release.yml`:** on any `v*` tag it builds `nova-cli` in release for
  x86_64 Linux, x86_64 and aarch64 macOS, and x86_64 Windows, and uploads
  each as an artifact.
- **20-STDLIB** has sections §2–§17 but none for `std/strings` or
  `std/bytes`.
- **The parser:** no crate depends on chumsky (`Cargo.lock` and every
  `Cargo.toml` checked). `nova-spec/11-PARSER.md:11-12`, `:310` and `:316`
  describe chumsky.
- **The LLVM backend** (`crates/nova-codegen-llvm`) emits textual IR and
  calls `clang`/`llc`; `inkwell` is not a dependency.
  `tests/ir_tests.rs` checks the IR text in 16 tests. The only end-to-end
  release test, `release_builds_and_runs_when_clang_available`
  (`run_tests.rs:1751`), builds and runs hello world, and returns early,
  passing, when `clang` is not on `PATH`.

## 3. Design

### 3.1 ADR 0025, "Phase 2's boundary"

`docs/adr/0025-phase-2-boundary.md`. It has these parts:

- **Status:** accepted, dated, branch `phase-2-closeout`.
- **Context:**
  - §7's Definition of Done;
  - the three places Phase 2's promises live: master spec §3's Phase 2 list
    and gate, phase-2-plan's sub-phases 2.0–2.5 with its cross-cutting
    section, and 20-STDLIB §1's module index;
  - the 2026-10-05 assessment;
  - the user's decisions in §1.
- **Decision: the inventory table** (§3.2). Each row gives a promise, its
  source with file and line, its status, and its record: an existing ADR, a
  20-STDLIB section, "backlog", "Phase 3/6", or "decided here".
- **One short section for each deviation without a prior record:**
  - what was promised and where;
  - what exists;
  - why it waits, or what was decided.

  The sections:
  - UDP (and Unix sockets);
  - the HTTP client;
  - a router beyond `Server`'s exact paths and GET only;
  - atomics and RwLock;
  - Queue (and Deque);
  - AEAD;
  - BLAKE3, refused by the `ring` backing, so it needs another backing;
  - LLVM parity, naming the release test's blind spot;
  - the collections benchmark;
  - the fixture migration;
  - `salsa`;
  - fuzz targets;
  - chumsky, **decided:** the hand-written parser stays;
  - per-sub-phase tags, **decided:** the four alpha tags stand in for them;
  - "benchmark hardware", **decided:** for Phase 2 it means this development
    host, where ADR 0021's procedure was run.
- **The backlog:** the unscheduled items, each "built when a program needs
  it". `salsa` is listed under Phase 3, fuzz targets under Phase 6.
- **Found outside this boundary,** one line each, with no decision:
  - ADR 0001's and master spec §1.1's "no JIT", while `nova run` JIT-compiles
    with Cranelift;
  - 14-CODEGEN §2.2's `inkwell`.
- **Consequences:**
  - Phase 2 is complete, as of `v0.2.0`, within this boundary;
  - the shipped surface (§3.2's "shipped" rows) is what a `v0.2.0` user can
    rely on;
  - the backlog promises nothing.

### 3.2 The inventory table (draft; the implementer re-checks every row)

| Promise | Source | Status | Record |
|---|---|---|---|
| `std/core` | master spec Phase 2 item 1 | shipped | 20-STDLIB §2 |
| `std/fmt`, `std/io` | item 2 | shipped | §3, §4; ADR 0015 |
| `std/collections`: `Vec`, `Map`, `Set` | item 3 | shipped | §12 |
| `Queue` (and `Deque`) | 20-STDLIB §1 (`:23`), §12 (`:1965`); phase-2-plan 2.2 | not built | backlog |
| `std/strings` | item 4 | shipped | §18 (new) |
| `std/fs` | item 5 | shipped | §5; ADR 0012 |
| `std/time`, `std/log` | item 6 | shipped | §9, §10 |
| `std/task` ("wrap Tokio") | item 7 | shipped on a single-threaded executor, not Tokio | ADR 0009 |
| `std/sync`: `Mutex`, channel | item 8 | shipped | ADR 0016, ADR 0017 |
| `std/sync`: atomic; `RwLock` (index and plan only) | item 8; 20-STDLIB `:27`; phase-2-plan 2.3 | not built | ADR 0016 notes atomic untouched; backlog |
| `std/net`: TCP | item 9 | shipped | §16; ADR 0013 |
| `std/net`: UDP; Unix sockets (index only) | item 9; 20-STDLIB `:16` | not built | §16 (`:2513-2527`); backlog |
| `std/http`: server, over `httparse` | item 10 | shipped | §6; ADR 0019 |
| `std/http`: client | item 10 | not built | §6 ("Not in v1: the client"); backlog |
| `std/http`: router beyond exact paths and GET | phase-2-plan 2.4 (`:330`) | not built | backlog |
| `std/json` | item 11 | shipped | §7; ADR 0018 |
| `std/crypto`: hashes, HMAC, random | item 12 | shipped | §8 |
| `std/crypto`: AEAD | 20-STDLIB `:19` | not built | §8 (`:1314`); ADR 0018 `:966`; backlog |
| `std/crypto`: BLAKE3 | 20-STDLIB §8 | refused by the `ring` backing | §8 (`:1558`); backlog (another backing) |
| `std/test` (`nova test`) | item 13 | shipped | §11 |
| `std/bytes`, `std/process` | not on the Phase 2 list | shipped | §19 (new), §17 |
| Gate: `examples/05-json-api` 10k+ req/sec on benchmark hardware; methodology in `docs/benchmarks/` | §3 Phase 2 gate | met (ADR 0021), on this development host; `docs/benchmarks/README.md` | ADR 0021; "benchmark hardware" decided here |
| 2.0 gate: both backends | phase-2-plan 2.0 | met under Cranelift; LLVM: IR-text tests plus hello world in release | backlog: LLVM parity |
| 2.1 gate | phase-2-plan 2.1 | met | — |
| 2.2 gate: GC stress; "benchmark basic ops" | phase-2-plan 2.2 | stress met; no collections benchmark | backlog |
| 2.3 gate: producer/consumer | phase-2-plan 2.3 | met (`examples/03-producer-consumer`) | — |
| 2.4 gate | phase-2-plan 2.4 | met (`examples/05-json-api`) | ADR 0021 |
| 2.5: `std/test` | phase-2-plan 2.5 | shipped | §11 |
| 2.5: fixture migration to `nova test` | phase-2-plan 2.5 | not done | backlog |
| 2.5: chumsky 0.10 | phase-2-plan 2.5; master spec Phase 0 item 6 (`:553`), dependency list (`:747`) | not adopted; the parser is hand-written | decided here |
| 2.5: `salsa` | phase-2-plan 2.5; master spec dependency list (`:755`) | not built | Phase 3 |
| 2.5: fuzz targets | phase-2-plan 2.5; master spec §5.2 (`:696`); 50-TESTING §1.7 | not built | Phase 6 |
| 2.5: non-Windows GC stack bounds | phase-2-plan 2.5 | done | ADR 0024 |
| Cross-cutting: both backends in lockstep | phase-2-plan §5 | not in lockstep | backlog: LLVM parity |
| Sub-phases "each independently gated, reviewed, and tagged" | phase-2-plan §4 | gated and reviewed; tagged as four alphas | decided here |
| Definition of Done 1–6 | master spec §7 | 1, 2, 4 met; 3 by §18–§19; 5 by this ADR; 6 by the release | — |

### 3.3 Dated notes in other records (2026-10-06, branch `phase-2-closeout`)

The bodies stay as they are; each note points at ADR 0025.

- **`nova-spec/00-MASTER-SPEC.md`:**
  - §3 Phase 2: its boundary is ADR 0025; every Definition of Done item but
    the tag is met.
  - Phase 0 item 6 (`:553`, chumsky): the parser is hand-written.
  - The dependency list (`:747-755`): chumsky, logos and `salsa` are not
    used.
- **`nova-spec/11-PARSER.md`** (`:11-12`, `:310`, `:316`): the parser is
  hand-written; chumsky is not used.
- **`nova-spec/14-CODEGEN.md` §2.2:**
  - the LLVM backend emits textual IR and calls `clang`/`llc`; there is no
    `inkwell`;
  - end to end, only hello world is built and run in release;
  - parity is deferred.
- **`nova-spec/50-TESTING.md` §1.7:** no fuzz targets exist; they are mapped
  to Phase 6.
- **`docs/phase-2-plan.md`:**
  - its "draft" status line: closed by ADR 0025;
  - notes at 2.2, 2.3, 2.4 and 2.5, and at §5's backends item.
- **`nova-spec/20-STDLIB.md` §1** (the module index): how much of each module
  shipped is ADR 0025's table. Line 19's AEAD, for example, is not built.
- **Unchanged:** ADRs 0001, 0016, 0018 and 0019. Their statements stay
  true, or are outside this boundary, and ADR 0025 cites them.

### 3.4 CHANGELOG

`[Unreleased]` gains one "Added" bullet: ADR 0025, Phase 2's boundary, and
20-STDLIB §18 and §19.

### 3.5 20-STDLIB §18 `std/strings` and §19 `std/bytes`

Each follows §17's layout:
- an "Added 2026-10-06 (branch `phase-2-closeout`), numbered out of the
  module-index order" note, for §16's reason;
- a Nova block listing every public signature, each with a one-line comment;
- bullets on behaviour.

**§18 documents `impl String`** (`std/strings/lib.nova`):
- **The surface:** `is_empty`, `len`, `chars`, `char_at`, `slice`,
  `reverse`, `starts_with`, `ends_with`, `index_of`, `contains`, `split`,
  `join`, `trim`, `trim_start`, `trim_end`, `repeat`, `to_upper`,
  `to_lower`.
- **Behaviour pinned from the code and its tests:**
  - character-based, not byte-based, counting in `len`, `char_at`, `slice`
    and `index_of`;
  - out-of-range indexes;
  - `split` with an empty separator;
  - what `trim` treats as whitespace;
  - how far `to_upper` and `to_lower` are Unicode-aware;
  - `join` being called on the separator.

**§19 documents `impl Bytes`** (`std/bytes/lib.nova`):
- **The surface:** `len`, `to_string` (returns `Option`, `None` for invalid
  UTF-8), `byte_at`, `slice`, `concat`, `to_ints`, `index_of`, `contains`;
  the free functions `bytes_from_string` and `bytes_from_ints`; and
  `impl Eq`.
- **Behaviour:** the index's "immutable byte buffers" is confirmed against the
  code.

**Rules for both sections:**
- "Not built" bullets appear only where the module index promises more.
- Each section names the builtins behind it, as §17 does.
- Every behaviour bullet is checked against the code, and against a test where
  one exists.

### 3.6 The `v0.2.0` release, a separate step after this branch merges

1. **A branch `release-0.2.0`:**
   - CHANGELOG: `[Unreleased]` becomes `[0.2.0]`, dated the day the release
     branch is cut (the earlier releases' convention), with a lead
     paragraph saying Phase 2 is complete within ADR 0025's boundary; a fresh,
     empty `[Unreleased]` goes above it.
   - A dated note in master spec §3: "Phase 2 is complete: v0.2.0".
   - The workspace crates go from `0.1.0` to `0.2.0`, and `Cargo.lock` is
     updated.
   - The full suite runs on Windows. Anything that pins the version string
     would show up there.
2. **A PR,** CI green, then a merge on the user's word.
3. **An annotated tag `v0.2.0`** on the merge commit: "Nova v0.2.0 — Phase 2
   (Standard Library Core)".
   - It is pushed only on the user's explicit word: pushing it starts
     `release.yml`.
   - Each artifact appears in that workflow run.

## 4. Verification

- **A fresh read-only agent checks every factual claim** in ADR 0025 and
  §18–§19 against the repository: every file:line, every signature, every
  behaviour bullet. The same was done for the GC spec.
- **The sweep at the end of the records task:**
  - **phrases:** `chumsky`, `inkwell`, `not yet built`, `unbuilt`,
    `still not complete`, `Phase 2 is (still )?not`, `Queue`, `RwLock`,
    `UDP`, `client`, `AEAD`, `BLAKE3`;
  - **set difference:** the files still matched, minus the files touched;
  - every leftover is fixed or explained, with a multi-line search alongside,
    because grep is line-based.
- **This branch is docs-only.** CI runs as usual and its counts must equal
  PR #99's. The version bump in §3.6 changes `Cargo.lock`, so the release
  branch runs the full suite.

## 5. Risks

- **A wrong row in the inventory table** would misstate what `v0.2.0` ships.
  Each row is re-checked by the implementer, then by the fresh agent.
- **The bump to 0.2.0** could break a test or doc that pins `0.1.0`. The
  release branch's full suite and a `git grep '0\.1\.0'` sweep catch it.
- **The tag is public, and `release.yml` runs on it.** It is pushed only on
  the user's word.

## 6. What is not covered

- None of the backlog is built: that is the decision.
- The LLVM release test still passes when `clang` is missing; it is recorded,
  not fixed.
- The Phase 1 drift beyond ADR 0025's one-line mention.

## 7. Success criteria

1. ADR 0025 exists, and every row of its table is confirmed by the fresh agent.
2. 20-STDLIB §18 and §19 exist, and every signature and behaviour bullet is
   confirmed.
3. Every note in §3.3 is in place, and the sweep leaves no unexplained file.
4. The CHANGELOG bullet is in place.
5. CI is green with PR #99's counts.
6. With the user's approval, `v0.2.0` is tagged per §3.6.
