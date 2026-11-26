# ADR 0061 — Phase 5's boundary

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

## Context

`nova-spec/00-MASTER-SPEC.md` §7 calls a phase done when:
1. its crates compile and pass CI;
2. its gate criteria are met with reproducible commands;
3. its new public surface is documented;
4. `CHANGELOG.md` is updated;
5. an ADR records each decision that deviates from the spec;
6. a `v0.{phase}.0` milestone is tagged.

Phase 5, "Self-hosting", makes its promises in three places:
- **the master spec's Phase 5:** its list of six numbered items
  (`nova-spec/00-MASTER-SPEC.md:746-751`), cited below as "item N", and
  its gate (`:753`): "CI builds stage2 == stage1 byte-for-byte (or close
  — may differ in non-deterministic codegen, document tolerances)";
- **the master spec's §0:** the "Self-hosting target", Phase 5 (`:22`);
- **`docs/phase-5-plan.md`:** its §3 decisions (`:197-294`), cited as
  "decision N", and sub-phases 5.0 to 5.6, each with its own gate.

Earlier ADRs already narrowed Phase 5, one for each sub-phase:
- 0048: the scope, the backend, and the list of what Phase 5 leaves out;
- 0049: tuples and patterns;
- 0050: the std a compiler needs;
- 0051: the self-hosted lexer and the differential;
- 0052: the parser, the AST dump and the depth limit;
- 0053: the loader, the resolver and the resolution dump;
- 0054: packages, the WASM target and test mode in the resolution dump;
- 0055: item collection and the HIR dump;
- 0056: the type checker and the full HIR dump;
- 0057: MIR and the MIR dump;
- 0058: the `.ll` emitter and the `.ll` dump;
- 0059: the driver, `build` and `check`;
- 0060: the bootstrap gate and its tolerances.

Phase 5 was built in fifteen pull requests, merged by rebase from
2026-10-27 to 2026-11-24: 122 commits on `main` after `cc829f58`, up to
`2842d63c`. Nine drafts that measured CI were closed unmerged: #134,
#136, #137, #139, #142, #144, #146, #148 and #150. Each merged pull
request, with its commits on `main`:

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

`v0.5.0` was released on 2026-11-24 and 2026-11-25:
- the annotated tag `v0.5.0`, on the release commit `2842d63c`;
- the GitHub release "Nova 0.5.0": four archives, the two install
  scripts, std's documentation site and the extension's `.vsix`, with
  `SHA256SUMS`;
- the 20 crates, on crates.io at 0.5.0;
- the VS Code extension 0.5.0, in the Marketplace and Open VSX.

The user decided, and approved with each spec:
1. **5.6 decision 1 (2026-11-22):** 5.6 is three pull requests: the
   gate, the release, then the records, with this ADR, after the release;
2. **5.6 decision 6:** the gate on Linux alone; macOS and Windows go to
   this ADR's backlog;
3. **5.6 decision 8:** `selfhost/` is not shipped, which the notes say; the
   archives are unchanged;
4. **5.6b decision 5 (2026-11-23):** every step of the release, as for
   `v0.4.0`, with the user's try of the published `nova` in place of the
   runbook's step 6.2;
5. **5.6b decision 7:** the records after every publish and the try, in one
   pull request, as #127 was;
6. **R1 (2026-11-26):** ADR 0055's thread and ADR 0053's E0093, not
   done, recorded with the depth N survives, and put on the backlog;
7. **R2:** the try, recorded as `cargo install nova-cli --locked` on
   Windows, with the user's words, glossed;
8. **R3:** Open VSX's token, shown on screen and revoked, and 0.5.0
   inactive for a while, recorded briefly, as lessons;
9. **R4:** a dated note under the master spec's Phase 5 gate;
10. **R5:** Phase 6 gets N built for WASM alone, as the master spec says;
11. **R6:** notes in `selfhost/README.md` and `docs/releasing.md`, with
    `CHANGELOG.md`'s and the phase plan's, and none in the root README;
12. **R7:** R3's lessons as a dated note in the runbook's Open VSX step,
    not on the backlog.

## Decision

### The inventory

"Item N" is the master spec's Phase 5 list
(`nova-spec/00-MASTER-SPEC.md:746-751`). "Decision N" is one of
`docs/phase-5-plan.md`'s §3 decisions, and "5.Nx" one of its sub-phases,
with its pull request and its commits on `main`.

| Promise | Source | Status | Record |
|---|---|---|---|
| Port the lexer to Nova | item 1; 5.1a, #131, `25be9f3c..4f32f13d` | built: `selfhost/src/lexer.nova` and `token.nova`, held to S0's token dump by the differential and by a fuzzer on fixed seeds, on three systems | ADR 0051 |
| Port the parser to Nova | item 2; 5.1b | built: `selfhost/src/parser.nova` and its `parse_*.nova` modules, held to S0's AST dump by the differential and by fuzzed inputs, with one depth limit, 128, in both compilers | ADR 0052 |
| Port the AST, HIR and MIR to Nova | item 3; 5.1b to 5.4 | built differently: the AST (5.1b), resolution (5.2a, 5.2b), HIR (5.3a, 5.3b) and MIR (5.4), each held to S0's dump; the MIR is native only, WASM's and test mode's staying S0's | ADRs 0052 to 0057; ADR 0057 §6 |
| Port the type checker to Nova | item 4; 5.3a, 5.3b | built: item collection and the checking of bodies, held to S0's HIR dump by the differential, a body fuzzer and depth cases | ADRs 0055, 0056 |
| Backend bindings: "bind LLVM via `unsafe` extern from Nova" | item 5; decision 2 | replaced: N writes LLVM IR as text, as S0 does, and runs `clang` 18.1.3 and `cc` on it; no binding, no `unsafe` | ADRs 0048 (decision 3), 0058, 0059 |
| Bootstrap: stage0 (Rust) → stage1 → stage2, "assert stage1 == stage2" | item 6; 5.5b, 5.6a | built differently: stage1 by S0's `nova build --release`, stage2 by stage1's `build` and stage3 by stage2's, each running `clang` and `cc`; stage3 == stage2 is held too | ADRs 0059, 0060 |
| The gate: "CI builds stage2 == stage1 byte-for-byte (or close …, document tolerances)" | the master spec's Phase 5 gate; decision 3 | met on Ubuntu, with no tolerance: S1 == S2 == S3, byte for byte, in `main`'s CI run 45142347493 and in the release pull request's run 45345843046; macOS and Windows on the backlog | ADR 0060; "The gate's record" |
| The gate: "document tolerances" | the master spec's Phase 5 gate; decision 3 | met: there is none on the bytes, under four conditions: one `-o` basename, one `clang` 18.1.3, one runtime archive, one `cc` and `ld` within a job | ADR 0060 §2 |
| The "Self-hosting target": Phase 5 | the master spec's §0 | met, as `v0.5.0`: N builds itself, and S1 == S2 == S3 | this ADR |
| Scope: N covers the pipeline from source to `.ll`, the driver, and `build` and `check`; diagnostics are compared by code and span | decision 1 | built: N's `build` and `check`; what Phase 5 leaves out is ADR 0048's list, in the rows below | ADR 0048 |
| The IR differential: S0 and S1 emit identical `.ll` on `tests/runtime/`, `examples/` and N | decision 3 | met: on 5.5a's corpus, N's root among it, in the Selfhost job's main step on three systems, and on N from S0, S1 and S2 in the bootstrap step | ADR 0058; ADR 0060 §6 |
| N in `selfhost/`, a Nova package with flat modules; no new Rust crate | decision 4 | built: `selfhost/`, with 57 modules in `selfhost/src` and its own `nova.toml`; the workspace's crates are those of `v0.4.0` | ADRs 0051, 0054 |
| What ships: the Rust `nova`; N in the repository, not published | decision 5 | shipped: `v0.5.0` ships the Rust `nova`, and N is in no archive and no crate, as `[0.5.0]` says | ADR 0048; 5.6 decision 8 |
| The differential and the fuzz harness, in every sub-phase | decision 6 | shipped: S0's hidden dumps of tokens, the AST, resolution, HIR, MIR and `.ll`, hidden flags of the released `nova` with no stability promise; built: N's dumps and the harness in `selfhost.rs`, comparing the two on the corpus on three systems, and on a fuzzer's inputs from fixed seeds in each sub-phase from 5.1a to 5.4 (5.5a's `.ll` on the MIR fuzzer's); in 5.5b by probe tests, and in 5.6a by N's `.ll` from S0, S1 and S2 in the bootstrap step | ADRs 0051 to 0058; ADR 0059 §9; ADR 0060 §6 |
| The bootstrap on every pull request: "a CI job on `ubuntu-latest`" | decision 7 | done differently: a step of the Selfhost job's Linux leg, on `ubuntu-24.04`, on every pull request and push to `main`; 29.11 s in #151's run, under its 75 s line | ADR 0060 §11 |
| Speed: S1 and S2 link a release runtime; work for speed only if the bootstrap is too slow, and on the user's word | decision 8 | done: the chain links the release runtime S0 carries; interned string literals, unboxed `Option` and `Result`, and inline bounds checks are left out, on the backlog | ADR 0060; backlog |
| Determinism by construction, checked by a search of N for `Map` iteration | decision 9 | met: ADR 0060 §5's 17 `.keys()` lines, each sorted, given to `closest()` or put back into a `Map`; the search, repeated here on `2842d63c`, prints the same 17 | ADR 0060 §5; "The gate's record" |
| Rust changes build on 1.78 | decision 10 | met: CI's MSRV job checks the workspace with Rust 1.78, as in #151's run | `.github/workflows/ci.yml`'s MSRV job |
| 5.0a: the small language and CI | 5.0a, #128, `cc829f58..a3a95a2e` | shipped, in the released `nova`: `?`, `+=` on fields and elements, the newline rule, `String + String`, and the warnings E0102 and E0103; done: the golden suite through LLVM in CI | ADR 0048 |
| 5.0b: tuples and patterns | 5.0b, #129, `a3a95a2e..e13630f0` | shipped: tuples, nested patterns, or-patterns, guards, and `Char` and range patterns, lowered by test chains | ADR 0049 |
| 5.0c: std | 5.0c, #130, `e13630f0..25be9f3c` | shipped: `StringBuilder`, `OrderedMap`, number parsing and conversions, `Float.to_bits`, array methods, and `std/process`'s `run`, `status`, `env` and `current_exe` | ADR 0050 |
| 5.1a: the lexer and the differential | 5.1a, #131, `25be9f3c..4f32f13d` | built; its gate met: N's token dump equals S0's on the corpus and on a fuzzer's fixed seeds, on three systems | ADR 0051 |
| 5.1b: the parser, the AST dump and the depth limit | 5.1b, #132, `4f32f13d..1fa7cd3f` | built; its gate met: N's AST dump equals S0's, and both refuse a program nested deeper than the limit with P0002 | ADR 0052 |
| 5.2a: the loader, the resolver and the resolution dump | 5.2a, #133, `1fa7cd3f..2d38ad7a` | built; its gate met: N's resolution dump of loose programs equals S0's | ADR 0053 |
| 5.2b: packages, the WASM target and test mode in the resolution dump | 5.2b, #135, `2d38ad7a..2a67c6ce` | built; its gate met, with N's own reader of `nova.toml` | ADR 0054 |
| 5.3a: item collection and the HIR dump | 5.3a, #138, `2a67c6ce..e5c555d0` | built; its gate met: N's HIR dump of items equals S0's; `selfhost.rs` in a Selfhost job of its own | ADR 0055 |
| CI under Rust 1.100, between 5.3a and 5.3b | #140, `e5c555d0..f1f17328` | done: Rust 1.100's lints and test layout, `synchronization.lib` among Windows' link libraries, and the Browser job on `ubuntu-24.04`; not a sub-phase | — |
| 5.3b: the type checker and the full HIR dump | 5.3b, #141, `f1f17328..9389a84b` | built; its gate met: N's full HIR dump equals S0's, with a body fuzzer and depth cases | ADR 0056 |
| 5.4: MIR and the MIR dump | 5.4, #143, `9389a84b..1570ddbd` | built; its gate met, natively: N's MIR dump equals S0's, std spliced whole, with a MIR fuzzer | ADR 0057 |
| 5.5a: the `.ll` emitter and the `.ll` dump | 5.5a, #145, `1570ddbd..e71446df` | built; its gate met: N's `.ll` equals S0's on the corpus, N's root among it | ADR 0058 |
| 5.5b: the driver, `build` and `check` | 5.5b, #147, `e71446df..83ba8210` | built; its gate met: S1 leaves the same binary as S0's `nova build --release` on the 147 golden programs and on N itself, on Linux; `build` is refused on Windows | ADR 0059 |
| 5.6a: the bootstrap gate | 5.6a, #149, `83ba8210..ed165975` | built; its gate met: S1 == S2 == S3 in the Selfhost job's bootstrap step, in `main`'s run 45142347493; `selfhost/README.md` | ADR 0060 |
| 5.6b: the release `v0.5.0` | 5.6b, #151, `ed165975..2842d63c` | done: the release commit `2842d63c`, then the tag, the GitHub release, crates.io, both stores and the Release gate, each on the user's word, and the user's try of the published `nova`; recorded | 5.6b decisions 5 and 7; "The gate's record" |
| 5.6c: the records | 5.6c; 5.6 decision 1 | done: this ADR and the notes that point to it | this ADR |
| Pointer FFI, `unsafe` blocks, linking C libraries, and a binding of LLVM's C API | ADR 0048 `:38-39`; decision 2 | left out; backlog | ADR 0048 |
| Porting `run`, `test`, fmt, the language server, doc, the package manager, and the Cranelift and WASM backends | ADR 0048 `:40-41` | left out: each stays S0's; backlog | ADR 0048 |
| S1 as the shipped `nova` | ADR 0048 `:42`; decision 5 | left out: `v0.5.0` ships the Rust `nova`; backlog | ADR 0048; 5.6 decision 8 |
| The byte gate on macOS and Windows | ADR 0048 `:43`; decision 3 | left out: the gate runs on Ubuntu alone; backlog, with an unpinned `cc` and `ld` | ADRs 0048, 0060; 5.6 decision 6 |
| N built for WASM, which Phase 6's playground needs | ADR 0048 `:44` | Phase 6: its playground, item 5 | ADR 0048 |
| Trait objects, mutable globals and nested module directories | ADR 0048 `:45` | left out; backlog | ADR 0048 |
| N on a large-stack thread, run by 5.5's driver | ADR 0055 §12, the user's ruling of 2026-11-06 | not done: N runs on its main thread (1 MB on Windows); backlog | ADR 0055; "Found outside this boundary" |
| E0093's path on Windows, which "5.5's driver settles" | ADR 0053 `:773-775` | not done: N writes `/` where S0 writes `\`, which ADR 0059 §7 keeps as (differs); backlog | ADRs 0053, 0059; "Found outside this boundary" |

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

### Deviations, each decided by its record

No deviation is decided here. Each was decided in its sub-phase, by its
ADR and the user's rulings, and the rows cite them; this section names
them.

#### Item 5: textual LLVM IR through `clang`, not a binding

The master spec's "bind LLVM via `unsafe` extern from Nova" is replaced:
N writes LLVM IR as text, as S0's `--release` does, and runs `clang` and
`cc` on it (ADR 0048, decision 3; the emitter, ADR 0058; the driver, ADR
0059). Pointer FFI and `unsafe` are on the backlog.

#### Item 3: MIR, native only

N's MIR is the native one; WASM's and test mode's stay S0's (ADR 0057
§6).

#### The gate: on Ubuntu alone, with no tolerance on the bytes

The gate runs on `ubuntu-24.04` alone, macOS and Windows on the backlog
(5.6 decision 6; ADR 0060 §11), and holds S1 == S2 == S3 with nothing
tolerated, under ADR 0060 §2's four conditions: one `-o` basename, one
`clang` 18.1.3, one runtime archive, one `cc` and `ld` within a job.

### Rows whose record already explains them

These rows have no section above:
- **S1 by S0's `--release`, not Cranelift:** the Cranelift S1 is another
  binary, so the chain starts from the `--release` one (ADR 0060 §4).
- **N not shipped:** `v0.5.0` ships the Rust `nova` (decision 5; 5.6
  decision 8).
- **Decision 3's `.ll` differential:** it has run since 5.5a in the
  Selfhost job's main step, on three systems (ADR 0058; ADR 0060 §6).
- **Decision 7's job:** a step of the Selfhost job (ADR 0060 §11).
- **`build` refused on Windows:** ADR 0059 §4.
- **The depth limit, 128,** in both compilers: ADR 0052.
- **N's speed:** about 6× slower than release S0 (ADR 0052 `:1095-1097`),
  under the Selfhost job's lines of 80, 100 and 110 s (ADR 0057 §19).
- **std in S0's HIR dump** (5.3a, 5.3b): its item tables and bodies are
  checked once per process and mode, for the dump alone, falling back to
  the full check where sharing does not apply (ADR 0055 §4; ADR 0056 §4).

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
  step took 29.11 s, under its 75 s line, and printed these lines among
  others, its stage times left out and the runtime's path shortened:

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
  eight assets are the two install scripts, and the four archives, std's
  site and the `.vsix`, each named for `0.5.0`, with `SHA256SUMS`; each
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
  `nova-vscode-0.5.0.vsix`. Their screenshots: the Marketplace's listing,
  version 0.5.0, last updated 11/25/2026, `sakeerin.vscode-nova`, the
  repository's link; Open VSX's, Nova Language Tools, version 0.5.0 with
  the alias `latest`, the README, the namespace still unverified. The user
  confirmed `[0.5.0]` on both and the tokens removed.
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
next use, not as blame (R3):
- **Open VSX's token, first try:** the runbook reads it with bash's `read
  -rs` (step 4, item 2); in Windows PowerShell 5.1, whose `Read-Host` has
  no `-MaskInput`, it was shown on screen. That try uploaded nothing; the
  token was revoked, and a new one made and read masked.
- **The second try** answered "Extension sakeerin.vscode-nova 0.5.0 is
  already published, but currently isn't active and therefore not
  visible"; when 0.5.0 reached Open VSX, and why it was not yet active,
  are not known. It was not retried (`docs/releasing.md:750-753`), and
  the listing then showed 0.5.0 as `latest`.

### The backlog

The backlog is unscheduled. Each item is built when a program or a user
needs it, and the list promises no date, no phase and no order.

- **Left out of Phase 5 by ADR 0048** (`:36-45`): pointer FFI, `unsafe`
  blocks, linking C libraries, and a binding of LLVM's C API; porting
  `run`, `test`, fmt, the language server, doc, the package manager, and
  the Cranelift and WASM backends; S1 as the shipped `nova`; the byte
  gate on macOS and Windows (the next group); trait objects, mutable
  globals and nested module directories. N built for WASM is mapped to
  Phase 6 (below).
- **The bootstrap on macOS and Windows,** with an unpinned `cc` and `ld`
  (5.6 decision 6; ADR 0060 `:642-643`): N's `build`, refused on Windows
  (ADR 0059 §4), and N's linking on macOS, which has not run
  (`selfhost/README.md:66`); a `Map` lint, which ADR 0060 does not add
  (`:219`).
- **N's coverage of S0,** the parts that stay S0's alone: WASM's and
  test mode's MIR (ADR 0057 §6) and code generation; Cranelift in N; an
  embedded runtime in N; project mode, the library refusal, the registry
  sync, `--target`, `--ssr`, `--ssg` and `llc`; rendered diagnostics, and
  `check`'s code and span on every input (ADR 0059 `:740-745`); ADR 0059
  §7's (differs) list, among it **E0093's `\` on Windows**, which N
  writes as `/` (R1); the M-codes, registry packages, `nova.lock`, doc
  tests and `nova test`'s whole-package program (ADR 0054 `:516-521`).
- **The language and std** (the 5.0 spec's §11; ADRs 0049, 0050): `is`,
  `if let`, `while let`; record, array, `@` and `Float` patterns; impls
  on tuples, tuple `Map` keys and structural `==`; a conversion for `?`,
  a `From` trait and generic traits; `Map::entries` and `channel`
  returning a tuple; a decision tree for `match`, a full-range match
  without `_`, a lazy expansion of or-patterns, and flattened tuples
  (0049); a builder over a runtime byte buffer, and `OrderedSet` (0050);
  `as` casts, turbofish and `Float.from_bits`; a `Command` builder,
  asynchronous processes and processes on WASM (0050); `loop`, `while
  true` known to diverge, `[String; 32]` and `mut` fields.
- **Speed and robustness** (decision 8): interned string literals,
  unboxed `Option` and `Result`, and inline bounds checks, each only on
  the user's word; the lazy splice (ADR 0057); **N on a large-stack
  thread** (ADR 0055 §12; R1).
- **S0's bugs, kept:** std's bodies see a program's impls, worked around
  by the fallback (ADR 0056 `:1220-1221`); float `%` type-checks and
  fails in `build --release` (ADR 0058 `:885-886`).
- **Mapped elsewhere, to Phase 6 alone** (R5): N built for WASM, for the
  playground (`nova-spec/00-MASTER-SPEC.md:940`).
- **Taken from earlier backlogs:** ADR 0025's `?` operator (`:327`) and
  `std/process`'s `spawn` and `env` (`:337`) are shipped, by 5.0a and
  5.0c, with a blocking `run` in place of `spawn`; ADR 0047's nested
  module directories (`:443-445`) stay out, on ADR 0048's list too.
- **The process:** the root README's two relative links in
  "Self-hosting", which ship in the release's archives without what they
  point to. (R3's lessons are a dated note in the runbook's Open VSX
  step, by R7, not on the backlog.)

### Found outside this boundary

Met while closing Phase 5; recorded, not decided:
- **ADR 0055's ruling of 2026-11-06** (`:524-527`), that 5.5's driver
  runs N on a large-stack thread, was not carried out: N runs on its main
  thread (1 MB on Windows), and `selfhost/src` starts no thread. As ADR
  0055 measured it on Linux under `ulimit -s 1024` (`:518-524`), N
  survives 64 linked associated types each nesting 124 arrays (7,936
  levels), and overflows at 125 (8,000 levels) on the debug runtime, at
  125 or 126 on the release one; nothing in the corpus, the cases or the
  fuzzer comes near. It is on the backlog (R1).
- **ADR 0053's "5.5's driver settles it"** (`:773-775`): N writes `/` in
  E0093's path on Windows, where S0 writes `\`, and ADR 0059 §7 keeps it
  as (differs) (`:265`). It is on the backlog (R1).
- **The 5.6b plan's check of the records:** the plan
  (`docs/superpowers/plans/2026-11-24-phase-5-6b-the-release-v0.5.0.md:501-507`,
  `:706-724`) and its spec
  (`docs/superpowers/specs/2026-11-24-phase-5-6b-the-release-v0.5.0-design.md:523-531`)
  say the check, `specblocks.py` and a grep of the headings, holds "the
  fresh `[Unreleased]`". It holds each record's text and the headings'
  order, not each record's place, nor that `[Unreleased]` is empty; both
  were checked once, on the release commit's tree.
- **The 5.6b plan cites `.github/workflows/release-gate.yml:75`** for
  `--node` (its `:1803` and `:2681`): that line is a comment, and the
  greps that find the flag are `:83` and `:169`.
- **No note in `nova-spec/14-CODEGEN.md` for the front end's 5.1a to 5.3b,
  nor for 5.6a,** which the phase plan's §5 asks for as each sub-phase
  lands (`docs/phase-5-plan.md:768-771`), as ADR 0047 found for Phase 4
  (`:491-497`). **The phase plan's status** still says "approved" (`:3`),
  as ADR 0037 found of Phase 3's (`:463-464`), and its `:4` says "§1" for
  the master spec's §0 (`nova-spec/00-MASTER-SPEC.md:22`).
- **ADR 0060 names two commits not on `main`** (`:180`, `:467`,
  `:487-489`, `:544`), from #148's and #149's branches before the
  rebase; those pull requests' refs keep them.

## Consequences

- Every item of the master spec's §7 Definition of Done is met:
  1. the crates compile and pass CI: the release pull request's run
     45345843046, and `main`'s run 45352790982 on `2842d63c`, recorded in
     "The gate's record";
  2. the gate is met with reproducible commands: the Selfhost job's
     bootstrap step, on every pull request, and `selfhost/README.md`'s
     "S1, S2 and S3 by hand", which gives the chain as commands;
  3. the new public surface is documented: `[0.5.0]`'s Added, std's
     documentation site, `selfhost/README.md`, the root README's
     "Self-hosting" and the dated notes;
  4. `CHANGELOG.md`'s `[0.5.0]` holds Phase 5, with its note of
     2026-11-26;
  5. ADRs 0048 to 0060 and this one record the deviations;
  6. the tag `v0.5.0`, on `2842d63c`, is the release's, and its
     publishes are recorded in "The gate's record".

  **Phase 5 is complete as `v0.5.0`, within this boundary.**
- What a `v0.5.0` user can rely on is the "shipped" rows, within the
  limits their records state. N is "built": in the repository and held
  by CI on every pull request, not in the release.
- The backlog promises nothing. Phase 6's playground is its one mapping,
  for N built for WASM.
- Dated notes point here from:
  - `CHANGELOG.md`'s `[0.5.0]`, with the publishes' runs;
  - the master spec's Phase 5 gate;
  - `docs/phase-5-plan.md`'s 5.6;
  - `selfhost/README.md`'s records;
  - `docs/releasing.md`'s step 6.2, which `v0.5.0` replaces, and its
    Open VSX step, with the lessons above.

  The root README gets none (R6).
- `[0.5.0]`'s lead keeps "will be recorded", as published, and the
  published notes are not regenerated: `release.yml` writes them only on
  a `v*` tag's push, so `[0.5.0]` now differs from the GitHub release's
  notes by its dated note, as `[0.4.0]` has since `cc829f58`.

## References

- Spec: `docs/superpowers/specs/2026-11-26-phase-5-6c-the-records-design.md`,
  and its plan,
  `docs/superpowers/plans/2026-11-26-phase-5-6c-the-records.md`
- `nova-spec/00-MASTER-SPEC.md` §0, §3 and §7; `docs/phase-5-plan.md`
- `docs/releasing.md`
- ADRs 0025, 0037, 0047, and 0048 to 0060
- Pull requests #128 to #133, #135, #138, #140, #141, #143, #145, #147,
  #149 and #151; #148 and #150, as ADR 0060 and the 5.6b spec cite them
- The workflow runs: CI 45142347493, 45345843046 and 45352790982; Release
  45345843088 and 45373121044; Publish to crates.io 45548505763; Release
  gate 45596343684
- `.github/workflows/ci.yml`'s Selfhost job, and
  `crates/nova-cli/tests/selfhost.rs`
