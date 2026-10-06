# Phase 2 close-out: design

Branch `phase-2-closeout`, from `main` at `800694c`. The design was approved
section by section in conversation on 2026-10-06; this file is the written spec.
A fresh agent then checked its claims against the repository, and §8 lists
what that changed.

Line numbers are as of `800694c`. The master spec is cited by section and list
position instead. Its own §3 note of 2026-09-03 (`00-MASTER-SPEC.md:208-241`)
says its line numbers move, translates the old ones, and asks to be cited by
heading.

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
that assessment named: the collector now runs on glibc Linux and macOS
(ADR 0024).

**The user's decisions, 2026-10-06:**
- **Record only.** The close-out builds nothing. Each gap is either covered by
  an existing record already or is recorded as deferred. `v0.2.0` ships what
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
  LLVM rows record it);
- Phase 1 drift beyond the one-line mentions in ADR 0025 (§3.1);
- any manifest change on this branch: the root `Cargo.toml`'s two unused
  workspace dependencies are recorded, not removed (§3.3);
- the 9 deferred minors from PR #99's final review;
- the pre-existing release-only test failure, a separate task.

## 2. Evidence

- **Tags:** `v0.0.0` (2026-05-10, Phase 0), `v0.1.0` (2026-07-23, Phase 1),
  then four pre-releases:
  - `v0.2.0-alpha.1` (2026-08-17), "Phase 2 progress";
  - `alpha.2` (2026-09-09), "module inventory complete, gate still open";
  - `alpha.3` (2026-09-11), "gate example exists, ... measured and not met";
  - `alpha.4` (2026-09-12), "measured on both criteria, and met on neither".

  Every release tag points at a "docs: finalize CHANGELOG for the ..." commit;
  the repository has no merge commits. Each CHANGELOG heading carries its
  finalize commit's date, which can precede the tag: alpha.1's commit and
  heading say 2026-08-16, its tag 2026-08-17.
- **Crate versions:** all 21 crate manifests say `version = "0.1.0"`, as they
  did at `v0.1.0` and at every alpha tag, and `Cargo.lock` has the 21 matching
  `nova*` entries.
- **`release.yml`:** on any `v*` tag it builds `nova-cli` in release for
  x86_64 Linux, x86_64 and aarch64 macOS, and x86_64 Windows, and uploads
  each as an artifact.
- **20-STDLIB** has sections §2–§17 but none for `std/strings` or
  `std/bytes`. Its §1 is headed "Module Index (v1.0)": it names v1.0's
  surface, which is wider than Phase 2's. Three dated amendments under it
  (`:31-44`) track which modules still lack a section.
- **The parser** is hand-written recursive descent
  (`crates/nova-parser/src/grammar.rs:1-2`). No crate depends on chumsky, and
  `Cargo.lock` has none. The root `Cargo.toml:13` still declares
  `chumsky = "0.9"` as a workspace dependency that no member uses. Chumsky is
  named as the parser's in:
  - the parser's own rustdoc (`crates/nova-parser/src/lib.rs:4`, `:31`);
  - `nova-spec/11-PARSER.md:11-12`, `:310`, `:316`;
  - `nova-spec/10-LEXER.md:15-17`, whose decision reads "Chumsky for parser
    only";
  - master spec Phase 0, position 6;
  - three Phase 0 guides: `ARCHITECTURE.md:41`, `agent.md:109`, and
    `skill.md:47`, `:51`, `:247`.
- **Master spec §6, "Bootstrap Dependencies (Rust crates, FINAL list)",**
  names eight crates that are not in `Cargo.lock`: chumsky, `salsa`,
  `inkwell`, `wasm-encoder`, `walrus`, `tokio`, `tower-lsp` and `rustyline`.
  The root `Cargo.toml:35` also declares `rustyline = "14"`, unused.
- **The LLVM backend** (`crates/nova-codegen-llvm`) emits textual IR, which
  `crates/nova-driver/src/link.rs:24-42` passes to `clang`, or to `llc` as a
  fallback. `inkwell` is not a dependency. It is named in:
  - 14-CODEGEN §2.2 (`:39`), its §7 sketch (`:155-157`) and its §9.1
    (`:226`);
  - master spec Phase 1, position 9;
  - `agent.md:115` and `skill.md:103`.

  `tests/ir_tests.rs` checks the IR text in 16 tests. The only end-to-end
  release test, `release_builds_and_runs_when_clang_available`
  (`crates/nova-cli/tests/run_tests.rs:1752`), builds and runs hello world.
  It returns early, passing, when `clang` is not on `PATH`. The other two
  `--release` tests (`:1691`, `:1725`) hide the toolchain on purpose and check
  the IR left behind, also for hello world.
- **"Both backends" in the sub-phase records means two Cranelift paths.**
  `nova run` (JIT) and `nova build` (object file) are both Cranelift
  (`crates/nova-driver/src/lib.rs:61`, `:95`); only `nova build --release`
  reaches LLVM (`:131-136`). The module-system commit `8c37c79` and the 2.1
  design's gate
  (`docs/superpowers/specs/2026-07-25-phase-2-1-std-core-design.md:15-17`)
  both say "both backends" and name `nova run` and `nova build`.

## 3. Design

### 3.1 ADR 0025, "Phase 2's boundary"

`docs/adr/0025-phase-2-boundary.md`. It has these parts:

- **Status:** accepted, dated, branch `phase-2-closeout`.
- **Context:**
  - §7's Definition of Done;
  - where Phase 2's promises live: master spec §3's Phase 2 list and gate,
    and phase-2-plan's sub-phases 2.0–2.5 with its §4 and §5. 20-STDLIB §1's
    index is v1.0's, so the table lists its items that neither of those names
    as "v1.0 index only", to keep `v0.2.0`'s boundary explicit;
  - the earlier ADRs that already narrowed Phase 2: 0009 (no Tokio), 0014
    (build order), 0015 (`std/fmt`'s scope), 0016 and 0017 (`std/sync`), 0018
    and 0019 (2.4's split; `std/http` over `httparse`);
  - the 2026-10-05 assessment;
  - the user's decisions in §1.
- **Decision: the inventory table** (§3.2). Each row gives a promise, its
  source, its status, and its record: an existing ADR, a 20-STDLIB passage,
  "backlog", "Phase 3/6", or "decided here".
- **One short section for each deviation that no record decides.** Several
  already have a passage saying they are unbuilt; the section cites it and
  adds where the item goes:
  - what was promised and where;
  - what exists;
  - why it waits, or what was decided.

  The sections:
  - UDP and Unix sockets;
  - the HTTP client;
  - a router beyond `Server`'s exact paths and GET only;
  - atomics and `RwLock`;
  - `Queue`, `Deque` and `Vec::with_capacity`;
  - AEAD;
  - BLAKE3, refused by the `ring` backing, so it needs another backing;
  - LLVM parity. It names the release test's blind spot, and that the
    sub-phase records' "both backends" were Cranelift's two paths. No test
    runs the 2.0 gate, 2.1's gate or §5's testing item under LLVM;
  - the collections benchmark;
  - the fixture migration;
  - `salsa`;
  - fuzz targets;
  - chumsky, **decided:** the hand-written parser stays. The section records
    the root `Cargo.toml`'s unused `chumsky = "0.9"`, and that the version
    differs from the 0.10 the specs name;
  - per-sub-phase tags, **decided:** the four alpha tags stand in for them;
  - "benchmark hardware", **decided:** for Phase 2 it means this development
    host, where ADR 0021's procedure was run.

  Some rows get no section, because a passage already explains them, the
  plan made them optional, or the row says all there is:
  - `spawn_blocking` and `JoinHandle::cancel` (§13 says why);
  - HTTPS, HTTP/2 and chunked transfer-encoding (§6: "Not in v1");
  - `std/process`'s `spawn` and `env`, and `std/regex` (optional in plan 2.5);
  - the oneshot channel.
- **The backlog:** the unscheduled items, each "built when a program needs
  it". `salsa` is listed under Phase 3, fuzz targets under Phase 6.
- **Found outside this boundary,** one line each, with no decision:
  - ADR 0001's and master spec §1.1's "no JIT", while `nova run` JIT-compiles
    with Cranelift;
  - Phase 1's LLVM backend emits textual IR rather than using `inkwell`
    (master spec Phase 1, position 9; 14-CODEGEN §2.2).
- **Consequences:**
  - Phase 2 is complete, as of `v0.2.0`, within this boundary;
  - the shipped surface (§3.2's "shipped" rows) is what a `v0.2.0` user can
    rely on;
  - the backlog promises nothing.

### 3.2 The inventory table (draft; the implementer re-checks every row)

"Position N" is the master spec's Phase 2 list. "Index" is 20-STDLIB §1.

| Promise | Source | Status | Record |
|---|---|---|---|
| `std/core` | position 1 | shipped | 20-STDLIB §2 |
| `std/fmt`, `std/io` | position 2 | shipped | §3, §4; ADR 0015 |
| `std/collections`: `Vec`, `Map`, `Set` | position 3 | shipped | §12 |
| `Queue`, `Deque`; `Vec::with_capacity` | index (`:23`, `Queue` only); §12 (`:1937`, `:1965-1966`); phase-2-plan 2.2 (`:112`, `Queue` only) | not built | §12 (`:2084-2087`); backlog |
| `std/strings` | position 4 | shipped | §18 (new) |
| `std/fs` | position 5 | shipped | §5; ADR 0012 |
| `std/time`, `std/log` | position 6 | shipped | §9, §10 |
| `std/task` ("wrap Tokio") | position 7 | shipped, on a single-threaded executor rather than Tokio | ADR 0009 |
| `std/task`: `spawn_blocking`, `JoinHandle::cancel` | §13 (`:2108`, `:2113`) | not built | §13 (`:2211-2228`); backlog |
| `std/sync`: `Mutex`, a bounded `channel` | position 8 | shipped | ADR 0016, ADR 0017 |
| `std/sync`: atomics; `RwLock` | position 8 (atomics only); index (`:27`); phase-2-plan 2.3 (`:119`) | not built | §13 (`:2407-2419`); ADR 0016 (`:198-200`); backlog |
| `std/sync`: oneshot channel | phase-2-plan 2.3 (`:119`) | not built; a bounded `channel` ships (ADR 0017) | backlog |
| `std/net`: TCP | position 9 | shipped | §16; ADR 0013 |
| `std/net`: UDP; Unix sockets | position 9 (UDP only); index (`:16`) | not built | §16 (`:2527-2528`, `:2591-2592`); backlog |
| `std/http`: server, over `httparse` | position 10 | shipped | §6; ADR 0019 |
| `std/http`: client | position 10; index (`:17`) | not built | §6 (`:449-450`, `:500-501`); backlog |
| `std/http`: router beyond `Server`'s exact paths and GET | §6's block (`:531-556`); phase-2-plan 2.4 (`:329-334`) | not built | §6 (`:500-501`); backlog |
| `std/http`: HTTPS, HTTP/2, chunked transfer-encoding | §6 (`:449-452`) | not in v1, by §6's own statement | §6 |
| `std/json` | position 11 | shipped | §7; ADR 0018 |
| `std/crypto`: hashes, HMAC, random | position 12 | shipped | §8 |
| `std/crypto`: AEAD | index (`:19`); §8's block | not built | §8 (`:1599-1602`); ADR 0018 (`:966`); backlog |
| `std/crypto`: BLAKE3 | §8's block | refused by the `ring` backing | §8 (`:1556-1561`); backlog, with another backing |
| `std/test` (`nova test`) | position 13 | shipped | §11 |
| `std/bytes` | index (`:15`); not on the Phase 2 list | shipped | §19 (new) |
| `std/process`: `args`, `exit` | index (`:26`); phase-2-plan 2.5, optional (`:342-343`) | shipped | §17; ADR 0023 |
| `std/process`: `spawn`, `env` | index (`:26`); phase-2-plan 2.5, optional | not built | §17 (`:2598-2601`); backlog |
| `std/regex` | index (`:25`); phase-2-plan 2.5, optional | not built | backlog |
| Gate: `examples/05-json-api` 10k+ req/sec on benchmark hardware; methodology in `docs/benchmarks/` | master spec Phase 2 gate | met (ADR 0021), on this development host; `docs/benchmarks/README.md` | ADR 0021; "benchmark hardware" decided here |
| 2.0 gate: one multi-file program with `import`, a generic method, a `where` bound and an `extern` call, under both backends | phase-2-plan 2.0 (`:98-99`) | met in parts, under Cranelift only. `tests/runtime/modules/` is multi-file, with `import` and a generic function; `where` and `extern` (`tests/runtime/extern_ffi.nova`) are tested in other programs | backlog: LLVM parity |
| 2.1 gate | phase-2-plan 2.1 (`:107-108`) | met as the 2.1 design narrowed it (`:15-17`), under `nova run` and `nova build`. The plan's "rewrite the Phase-1 examples" is not in that gate | the 2.1 design's §2 |
| 2.2 gate: GC stress; "benchmark basic ops" | phase-2-plan 2.2 (`:114-115`) | stress met; no collections benchmark | backlog |
| 2.3 gate: producer/consumer | phase-2-plan 2.3 (`:121-122`) | met (`examples/03-producer-consumer`) | — |
| 2.4 gate: 05 functional, then 10k+ | phase-2-plan 2.4 (`:136-137`) | met (`examples/05-json-api`) | ADR 0021 |
| 2.5: `std/test` | phase-2-plan 2.5 (`:337`) | shipped | §11 |
| 2.5: fixture migration to `nova test` | phase-2-plan 2.5 (`:337`) | not done | backlog |
| 2.5: chumsky 0.10 | phase-2-plan 2.5 (`:338`); master spec Phase 0, position 6, and §6 | not adopted; the parser is hand-written | decided here |
| 2.5: `salsa` | phase-2-plan 2.5 (`:338`); master spec §6 | not built | Phase 3 |
| 2.5: fuzz targets | phase-2-plan 2.5 (`:338-339`); master spec §5.2; 50-TESTING §1.7 | not built; no `fuzz/` directory | Phase 6 |
| 2.5: non-Windows GC stack bounds | phase-2-plan 2.5 (`:339`) | done for glibc Linux and macOS | ADR 0024 |
| Cross-cutting: every module's programs under both backends and `NOVA_GC_STRESS` | phase-2-plan §5 (`:347-349`) | under LLVM, no module's programs run. Per-module `NOVA_GC_STRESS` coverage is not assessed here | backlog: LLVM parity |
| Cross-cutting: both backends in lockstep | phase-2-plan §5 (`:350-351`) | parity unverified: LLVM is tested on IR text and on hello world | backlog: LLVM parity |
| Sub-phases "each independently gated, reviewed, and tagged" | phase-2-plan §4 (`:81`) | gated and reviewed; tagged as four alphas | decided here |
| Definition of Done 1–6 | master spec §7 | 1, 2, 4 met; 3 by §18–§19; 5 by this ADR; 6 by the release | — |

### 3.3 Dated notes and corrections in other records (2026-10-06, branch `phase-2-closeout`)

Dated notes leave the bodies as they are, and each points at ADR 0025. The
Phase 0 guides and the parser's rustdoc are corrected in place instead,
because they describe the code as it is now rather than recording a history.

- **`nova-spec/00-MASTER-SPEC.md`:**
  - §3, after its last dated note (`:538-542`): Phase 2's boundary is
    ADR 0025; every Definition of Done item but the tag is met.
  - Phase 0, position 6 (chumsky): the parser is hand-written.
  - §5.2 (fuzz targets in `fuzz/`): none exist; ADR 0025 maps them to Phase 6.
  - §6 (the "FINAL list"): eight of its crates are not in `Cargo.lock`:
    - chumsky, decided in ADR 0025;
    - `salsa`, mapped to Phase 3 by ADR 0025;
    - `inkwell`: the LLVM backend emits textual IR;
    - `tokio`: ADR 0009's single-threaded executor;
    - `wasm-encoder` and `walrus`, for Phase 4, position 1;
    - `tower-lsp` and `rustyline`, for Phase 3, positions 4 and 7.
- **`nova-spec/11-PARSER.md`** (`:11-12`, `:310`, `:316`): the parser is
  hand-written recursive descent; chumsky is not used.
- **`nova-spec/10-LEXER.md`** (`:15-17`): the lexer uses `logos` as decided;
  the parser does not use chumsky either.
- **`nova-spec/14-CODEGEN.md` §2.2,** covering §7's sketch and §9.1 as well:
  - the LLVM backend emits textual IR, and `nova-driver`'s `link.rs` calls
    `clang` or `llc`; there is no `inkwell`;
  - end to end, only hello world is built and run in release;
  - parity is deferred.
- **`nova-spec/50-TESTING.md` §1.7:** no fuzz targets exist; they are mapped
  to Phase 6.
- **`docs/phase-2-plan.md`:**
  - its "draft" status line: closed by ADR 0025;
  - notes at 2.0 and 2.1 ("both backends" there meant `nova run` and
    `nova build`; 2.1's gate is the 2.1 design's narrowed one), 2.2, 2.3,
    2.4 and 2.5;
  - a note at §4's heading: the four alpha tags stand in for per-sub-phase
    tags;
  - notes at §5's testing and backends items.
- **`nova-spec/20-STDLIB.md` §1,** after the 2026-10-05 amendment (`:42-44`):
  - §18 and §19 now cover `std/strings` and `std/bytes`;
  - `std/regex` alone has no section, and is not built;
  - ADR 0025's table records how much of each module shipped.
- **Corrected in place:**
  - **The Phase 0 guides:** the cells that name chumsky or `inkwell` as an
    existing crate's dependency. These are `ARCHITECTURE.md:41`,
    `agent.md:109` and `:115`, and `skill.md:47`, `:51`, `:103` and `:247`.
    Rows about crates of later phases stay.
  - **The parser's rustdoc** (`crates/nova-parser/src/lib.rs:4` and `:31`),
    comment-only. `grammar.rs:1-2` already says hand-written; its
    "chumsky-style" names a style and stays.
- **Unchanged:**
  - ADRs 0001, 0016, 0017, 0018 and 0019. Their statements stay true, or are
    outside this boundary, and ADR 0025 cites them. The `00-MASTER-SPEC.md:238`
    that ADRs 0016 and 0017 cite is translated to position 8 by the master
    spec's own note (`:208-241`).
  - The root `Cargo.toml`'s `chumsky = "0.9"` (`:13`) and `rustyline = "14"`
    (`:35`). ADR 0025 and the §6 note record them.

### 3.4 CHANGELOG

`[Unreleased]` gains one "Added" bullet: ADR 0025, Phase 2's boundary, and
20-STDLIB §18 and §19.

### 3.5 20-STDLIB §18 `std/strings` and §19 `std/bytes`

Each follows §17's layout:
- an "Added 2026-10-06 (branch `phase-2-closeout`), numbered out of the
  module-index order" note, for §16's reason;
- a Nova block listing every public signature, each with a comment above it,
  of one or more lines as §17's are;
- bullets on behaviour, naming the builtins behind the module, as §17 does.

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
- Every behaviour bullet is checked against the code, and against a test where
  one exists.

### 3.6 The `v0.2.0` release, a separate step after this branch merges

1. **A branch `release-0.2.0`,** with the finalize commit last:
   - The workspace crates go from `0.1.0` to `0.2.0` in all 21 manifests, and
     `Cargo.lock`'s 21 entries follow.
   - A dated note in master spec §3: "Phase 2 is complete: v0.2.0".
   - "docs: finalize CHANGELOG for the v0.2.0 release": `[Unreleased]`
     becomes `[0.2.0]`, dated the day of that commit, as every earlier
     heading is. A lead paragraph says Phase 2 is complete within ADR 0025's
     boundary, and a fresh, empty `[Unreleased]` goes above it.
   - The full suite runs on Windows. Anything that pins the version string
     would show up there.
2. **A PR,** CI green, then a rebase merge on the user's word.
3. **An annotated tag `v0.2.0`** on the finalize commit as `main` has it after
   the merge, so the tagged tree carries the bump: "Nova v0.2.0 — Phase 2
   (Standard Library Core)".
   - It is pushed only on the user's explicit word: pushing it starts
     `release.yml`.
   - Each artifact appears in that workflow run.

## 4. Verification

- **A fresh read-only agent checks every factual claim** in ADR 0025, the
  notes and §18–§19 against the repository: every file:line, every signature,
  every behaviour bullet. The same was done for this spec (§8).
- **The sweep at the end of the records task:**
  - **phrases:** `chumsky`, `inkwell`, `salsa`, `fuzz`, `both backends`,
    `not yet built`, `unbuilt`, `still not complete`,
    `Phase 2 is (still )?not`, `Queue`, `RwLock`, `oneshot`, `UDP`, `client`,
    `AEAD`, `BLAKE3`, `std/regex`;
  - **set difference:** the files still matched, minus the files touched;
  - every leftover is fixed or explained, with a multi-line search alongside,
    because grep is line-based.
- **This branch changes no behaviour.** Its only `.rs` edits are two rustdoc
  sentences. CI runs as usual, and its counts must equal PR #99's. The version
  bump in §3.6 changes `Cargo.lock`, so the release branch runs the full suite.

## 5. Risks

- **A wrong row in the inventory table** would misstate what `v0.2.0` ships.
  Each row is re-checked by the implementer, then by the fresh agent.
- **The rustdoc edit** sits in `nova-parser`'s crate docs, above a doctest.
  CI's doctest run catches a broken one.
- **The bump to 0.2.0** could break a test or doc that pins `0.1.0`. The
  release branch's full suite and a `git grep '0\.1\.0'` sweep catch it.
- **The tag is public, and `release.yml` runs on it.** It is pushed only on
  the user's word.

## 6. What is not covered

- None of the backlog is built: that is the decision.
- The LLVM release test still passes when `clang` is missing; it is recorded,
  not fixed.
- The Phase 1 drift beyond ADR 0025's one-line mentions, including master
  spec Phase 1's position 9.
- The root `Cargo.toml`'s unused workspace dependencies stay declared.

## 7. Success criteria

1. ADR 0025 exists, and every row of its table is confirmed by the fresh agent.
2. 20-STDLIB §18 and §19 exist, and every signature and behaviour bullet is
   confirmed.
3. Every note and correction in §3.3 is in place, and the sweep leaves no
   unexplained file.
4. The CHANGELOG bullet is in place.
5. CI is green with PR #99's counts.
6. With the user's approval, `v0.2.0` is tagged per §3.6.

## 8. What the verification changed (2026-10-06)

A fresh read-only agent checked this spec's claims at `d240002`. It found 2
wrong and 16 partly right. Each was re-read at its source before it was
corrected here.

- **Wrong, corrected:**
  - `logos` is used: `crates/nova-lexer/Cargo.toml:18`. The §6 note now
    names the eight crates that are unused.
  - The AEAD row cited `:1314`, which is in §7. §8's statement is at
    `:1599-1602`.
- **Partly right, corrected:** the UDP, router, release-test and BLAKE3 line
  numbers; the `clang`/`llc` call site, which is in `nova-driver`, not the
  backend crate; `v0.0.0`, which was missing; the tag target, which is the
  finalize commit, never a merge commit; "glibc Linux and macOS"; the 2.0 and
  2.1 gate rows; "parity unverified"; §17's multi-line comments; and the
  index's v1.0 framing.
- **Added rows:** `Vec::with_capacity`, `spawn_blocking` and
  `JoinHandle::cancel`, the oneshot channel, HTTPS, HTTP/2 and chunked
  transfer-encoding, `std/process`'s `spawn` and `env`, `std/regex`, and
  §5's testing item.
- **Added to the design, for review:**
  - the "both backends" finding (§2), carried into the LLVM parity section;
  - the notes on master spec §5.2 and §6, on 10-LEXER, and on 14-CODEGEN §7
    and §9.1;
  - correcting the Phase 0 guides' cells and the parser's rustdoc in place.
    These are the only `.rs` edits, and they are comment-only;
  - recording, without removing, the root `Cargo.toml`'s unused `chumsky` and
    `rustyline`.
- **Unchanged:** every decision in §1.
