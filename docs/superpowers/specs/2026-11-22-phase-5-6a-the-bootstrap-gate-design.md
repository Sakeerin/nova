# Phase 5.6a, "the bootstrap gate: S1 == S2 == S3 in CI": design

- **Date:** 2026-11-22.
- **Status:** approved by the user (2026-11-22), with one change: the
  literal chain, built in order (decision 2).
- **The user's decisions, 2026-11-22:**
  1. **5.6 is three pull requests:** 5.6a, the gate in CI, the
     tolerances ADR and a `selfhost/` README (this spec); then the
     release `v0.5.0`, the version bump by `docs/releasing.md`; then the
     records, the boundary ADR closing Phase 5, after the release.
  2. **The chain runs in a step of the Selfhost job's Linux leg,** on
     every pull request, after 5.5b's golden step, with the `clang-18`
     installed there. The user approved the literal chain in order on
     2026-11-22: S1, then S2 by S1, then S3 by S2; only the `.ll` runs
     go side by side.
  3. **S1 is S0 `--release`'s,** so S1 == S2 == S3 byte for byte; the
     ADR records why the Cranelift S1 differs.
  4. **The gate is an ignored test in `selfhost.rs`,** named by
     `--exact` on its step's line, the existing CI test holding the
     names; N's `.ll` per stage first, then the binaries, reported by
     `define` and block or first differing offset, as today.
  5. **One chain, the conditions recorded:** S2 and S3 built in
     different directories; the ADR records one `-o` basename, clang
     18.1.3, one runtime archive, one `cc`/`ld`, and the search of N for
     `Map` iteration; no extra rebuild, no lint.
  6. **Linux only** (phase plan decision 3); macOS and Windows go to the
     boundary ADR's backlog.
  7. **`cc` and `ld` printed, not pinned:** the step prints their
     versions; the ADR records one `cc`/`ld` within a job as a condition.
  8. **Settled for the release PR:** `selfhost/` is not shipped and the
     archive is unchanged; the release notes and the README say N lives
     in `selfhost/` and that the bootstrap passes the gate.
- **Not asked; the recommendation taken, which the user can overturn:**
  a new step, not a name on 5.5b's step (§6.1); the runtime taken from
  S0's own unpacking (§3.1); each `.ll` from a front-end-only run
  (§3.3); the CI step test extended to two steps and a `run: |` block
  (§6.2); `--nocapture` on the step (§6.1); `same_binaries` given the
  stages' names, and the directory kept on a failure (§4); a main-step
  test of both reports (§7); the prototype measured on CI first (§8);
  one pull request in tasks (§9).
- **Branch:** `claude/loving-archimedes-frtf5q-56`, from `main` at
  `83ba8210` (5.5b merged), for this spec; the plan and the work follow
  on it. Nothing is pushed without the user's word.
- **Inputs:** `docs/phase-5-plan.md`; ADRs 0047, 0048 and 0057 to
  0059; the 5.5b spec; the research of 2026-11-22 (§2).

## 1. What 5.6a delivers

A new CI step on Ubuntu builds the bootstrap chain and holds it fixed:
S0 `--release` builds N into S1, then S1 builds N into S2, then S2
builds N into S3, strictly in order, one `-o` basename throughout; N's
`.ll` agrees across the stages, and **S1 == S2 == S3, byte for byte**:
phase plan decision 3's second and third items; its first, the `.ll`
differential, already runs (§5). ADR 0060 records the tolerances, none
on the bytes and four conditions. `selfhost/` gains a README.

## 2. The starting point

Measured on `83ba8210`, 2026-11-22 (the research: 4 cores, `clang-18`
18.1.3, `cc` gcc 13.3, GNU ld 2.42); line numbers re-checked for this
spec, no build run.

1. **S1 == S2 == S3 holds today.** One basename `selfhost`, the release
   runtime through `NOVA_RUNTIME_LIB`: one SHA-256 (`877a8ee8…`,
   9,354,680 bytes) over 12 builds, among them repeats, other
   directories, `TMPDIR`, cwd and std paths, a minimal environment, and
   each stage's split rebuild. N's `.ll` from S0, S1 and S2 is one file,
   12,760,111 bytes, 406,253 lines.
2. **The Cranelift S1 is another binary** (9,376,184 bytes): its code is
   Cranelift's. Its `.ll` is S0's, so its S2 is S1 `--release`.
3. **A stage takes 14 to 15 s, `clang` 85% of it:** S0's front end
   1.47 s, N's 2.03 to 2.06 s, `clang -O2 -c` 12.2 to 12.9 s, `cc` 0.15
   to 0.20 s. In order (the research's `chain-1.txt`): S1 14.01 to
   14.73 s, S2 by S1 14.88 to 14.92 s, S3 by S2 14.46 to 15.07 s; 43.4
   to 44.7 s in all. The Cranelift S1, not used here, builds in 9.38 s.
4. **What varies the binary:** the `.ll`'s basename (`STT_FILE`; a
   same-length name differs in 1 byte, a shorter one in 367,460 and the
   build-id); the runtime archive (the debug one: another binary); the
   link driver (`CC=clang-18`: 5.59 M bytes). **What does not:**
   timestamps; the runtime's path; `CC=gcc`; `NOVA_CLANG=clang` (one
   18.1.3); `env -i`, `LC_ALL=C`, odd variables; the paths of N, std,
   cwd and `TMPDIR`. GNU ld's build-id is a content hash; N's object has
   no debug section and no path.
5. **N reads from outside** only `NOVA_CLANG`, `CC`, `NOVA_RUNTIME_LIB`
   and `current_exe` (`selfhost/src/driver.nova:533`, `:537`, `:639`);
   std's modules come from a fixed list (`builtins.nova:444`).
6. **N iterates no seeded `Map` into output.** The research counted 14
   `.keys()` sites, each sorted, fed to `closest()` (a total order,
   `diagnostic.nova:159`), or re-inserted into a `Map`. A grep for this
   spec finds **17 lines** with `.keys()`, six of them in `fixes.nova`'s
   `did_you_mean` (`:521-545`); each falls in one of the same three
   classes (read for this spec; `closest()`'s order not re-checked). N
   uses no `OrderedMap`.
7. **The `.ll` differential runs on every pull request,** on Linux,
   macOS and Windows, in the Selfhost job's main step (`ci.yml:189-190`):
   S0's `check --dump-mir --ll` against N's `mir --ll`, 778 `.ll`s, N's
   root among them (ADR 0058 §11), N being the Cranelift S1.
8. **The harness** (`crates/nova-cli/tests/selfhost.rs`): `selfhost()`
   (`:464`) builds the Cranelift S1 once per process (`OnceLock`,
   `build_selfhost` `:482`) into `nova-selfhost-build/`, `NOVA_HOME`
   `nova-selfhost-home/`, where S0 unpacks its embedded runtime
   (`nova-driver/src/runtime_cache.rs:187`); `golden_runtime()`
   (`:6879`) gives `NOVA_RUNTIME_LIB`, else the runtime unpacked there,
   else the one beside S0; `same_binaries` (`:6917`) names both sizes
   and the first differing offset, in words fixed to S0 and S1;
   `compare::first_difference` (`selfhost/compare.rs:52`) names a
   `.ll`'s line, `define` and block (`:1903`); `fresh_dir` (`:286`).
9. **The clang tests** (`:6952`, `:8094`) are `#[ignore = "needs clang;
   the Selfhost job's Linux step runs it"]`, passing without `clang`
   unless `NOVA_REQUIRE_CLANG=1`; `the_ci_step_names_every_clang_test`
   (`:7091`) holds the step "S1 builds the golden suite (Linux)"
   (`ci.yml:210-215`) to exactly them, after `--ignored`.
10. **A successful build removes its `.ll`:** S0's
    (`nova-driver/src/lib.rs:943`) and N's (`driver.nova:217`). With
    `NOVA_CLANG` (and S0's `NOVA_LLC`) naming a missing file, each
    exits 1 and leaves it (`selfhost.rs:6805`).
11. **CI today** (runs 44961817202 and 44962982900): the Selfhost job's
    Linux leg 3:34 and 3:19, its golden step 31 s; the Test job 9:48 to
    12:28; `selfhost.rs`'s lines 80, 100 and 110 s (ADR 0057 §19).

## 3. The test, `the_bootstrap_holds_s1_s2_s3` (decisions 3, 4)

### 3.1 Its inputs

- **`#[ignore = "needs clang; the Selfhost job's Linux step runs it"]`,**
  the other clang tests' reason exactly, so §6.2 requires its name.
- **`clang`:** `NOVA_CLANG` or `clang`, probed by `--version`. If it does
  not spawn: under `NOVA_REQUIRE_CLANG=1` fail; else print `bootstrap:
  not run (no clang)` and pass. `cc` is not probed: a build without it
  fails and says why.
- **No Cranelift S1:** the test never calls `selfhost()`. Of what
  `build_selfhost` gives, it needs only a runtime, and gets it from S0
  itself. S1's build runs with `NOVA_HOME` at the fresh `home/`, and
  with `NOVA_RUNTIME_LIB` only if the environment sets it. S0 then
  links, as `link.rs:94` does, `NOVA_RUNTIME_LIB`, else its embedded
  runtime, unpacked into `home/runtime/<version>-<crc32>/`
  (`runtime_cache.rs:77`, `:187`), else the one beside it. After S1's
  build, the test takes the same file by the same order:
  `golden_runtime()`'s body, given the unpack directory as a parameter
  (`runtime_in(home)`; `golden_runtime()` calls it with
  `nova-selfhost-home`, unchanged). That file goes to S2's and S3's
  builds as `NOVA_RUNTIME_LIB`: one archive for all three.
- **N** is `<repo>/selfhost/src/main.nova`, in place, std `<repo>/std`.
  Every build has `NO_COLOR=1`, `RUST_BACKTRACE=0`,
  `RUST_LIB_BACKTRACE=0` and `CHILD_LIMIT` (120 s).
- **New fixed temporary names,** under `nova-selfhost-bootstrap/`
  (`fresh_dir("bootstrap")`): `s1/`, `s2/`, `s3/`, each stage's build,
  binary `selfhost`; `ll-s0/`, `ll-s1/`, `ll-s2/`, each front-end run's
  `selfhost.nova.ll`; `home/`, S1's build's `NOVA_HOME`, holding the
  unpacked runtime; `home-ll/`, the `NOVA_HOME` of S0's front-end run;
  `no-such-tool`, never created, for `NOVA_CLANG` and `NOVA_LLC`.

### 3.2 The steps, strictly in order

The literal chain, as the user approved on 2026-11-22. Each build
needs the one before it; only a front-end run goes beside a build.

1. **S1,** beside **S0's `.ll`:** S0, `nova build --release N -o
   s1/selfhost`; beside it, S0 again with `-o ll-s0/selfhost` and
   `NOVA_CLANG` and `NOVA_LLC` at `no-such-tool`: exit 1. Then the
   runtime (§3.1).
2. **S2,** beside **S1's `.ll`:** S1, `selfhost build N -o s2/selfhost
   --release --std STD`; beside it, S1 with `-o ll-s1/selfhost` and
   `NOVA_CLANG` at `no-such-tool`: exit 1.
3. **S3,** beside **S2's `.ll`:** S2, `-o s3/selfhost`; beside it, S2
   with `-o ll-s2/selfhost`, `NOVA_CLANG` at `no-such-tool`: exit 1.
4. **The comparisons** (§3.4).

Each pair runs in two threads (`std::thread::scope`), both awaited
before the next step. A stage that fails stops the chain: the stages
after it are not built. The critical path is the three builds, as the
research's `chain.sh` ran them (14.01 to 14.73, 14.88 to 14.92, 14.46
to 15.07 s); the front ends (1.5 to 2.1 s) run on a second core.

### 3.3 The `.ll`s

A successful build removes its `.ll` (§2 item 10), so each stage's is
made by a front-end-only run: the same command and basename in its own
directory, stopped at the `clang` probe, as the research, ADR 0059 §13
and `the_ll_dump_is_what_build_release_writes` made them. Each run must
exit 1 and leave the `.ll`. Both drivers look for the runtime only
after `clang`, so a front-end run never unpacks one. No wrapper around
`clang`, no flag on either driver, no emitter change.

### 3.4 The comparisons, in order

1. **N's `.ll`:** S1's against S0's; S2's against S0's.
2. **The binaries:** S1 against S2; S2 against S3.

All byte for byte. Every comparison runs; the failures are collected in
this order, and the test fails with all of them. Equal `.ll`s and
differing binaries point at the toolchain or the link; differing `.ll`s
at the front end.

### 3.5 Its report

On success, on stdout (shown by `--nocapture`, §6.1): the binaries'
and the `.ll`'s sizes, as read; `clang`'s first `--version` line; the
runtime's path; each stage's wall time. No digest: no hash crate.

### 3.6 Locally

`cargo test --test selfhost` skips it; it runs as the step runs it,
and without `clang` prints `bootstrap: not run (no clang)`. A
checkout's S0 carries no runtime, so without `NOVA_RUNTIME_LIB` S0
links the debug one beside it, and the test passes that file on: the
binaries are then about 14.3 MB. **S1 == S2 == S3 on the debug runtime
is not measured;** one archive for all three is the condition, so it
should hold.

## 4. Errors and the report on a mismatch

No new code in S0 or N; their messages do not change.

- **A build fails** (exit not 0, or no `built OUT` from N), or a
  front-end run exits other than 1 or leaves no `.ll`: the stage, the
  command, the status and the stderr (`quoted_stderr`); the later
  stages are not built.
- **The `.ll`:** a new helper in `selfhost.rs`, `ll_difference(a, b)`,
  wraps each `.ll` as one root's MIR part (`== selfhost/src/main.nova`,
  `(Mir bootstrap)`, `(Ll ok)`, the `.ll`) and calls
  `compare::first_difference`, whose `MirWalk::place` gives L, F and B:

  ```
  N's .ll from S2 differs from S0's at line L of its .ll, define F, block B
    S0: <S0's line>
    S2: <S2's line>
  both kept: diff <dir>/ll-s0/selfhost.nova.ll <dir>/ll-s2/selfhost.nova.ll
  ```

  `<dir>` is printed in full. `compare.rs` is unchanged.
- **The binaries:** `same_binaries(("S1", a), ("S2", b))` words its
  error "S2's binary differs from S1's: B bytes against S1's A, first
  at offset N". The golden test passes `"S0"` and `"S1"`, so its words
  are as today, and the clang probe's check of the offset holds.
- **The directory** is removed on success, kept on a failure.

## 5. The `.ll` differential: recorded, not added

Phase plan decision 3's first item, "S0 and S1 emit identical `.ll`" on
`tests/runtime/`, `examples/` and N, is inside 5.5a's corpus and runs on
every pull request on all three systems (§2 item 7). 5.6a adds no check
for it; ADR 0060 and the phase plan's note record where it runs.

The new test's `.ll` comparison is no second corpus gate: N's root
alone, on Linux, through `build`, from S1 and S2 (N compiled by
`clang`; the differential's N is the Cranelift S1). It places a binary
mismatch in the front end or the toolchain.

## 6. CI (decisions 2, 6, 7)

### 6.1 The step

After "S1 builds the golden suite (Linux)", before "N's own tests on the
debug runtime", with a comment citing this spec and ADR 0060 (5.5b's,
`ci.yml:191-199`, is the model):

```yaml
      - name: The bootstrap, S1 == S2 == S3 (Linux)
        if: runner.os == 'Linux'
        env:
          NOVA_REQUIRE_CLANG: "1"
          NOVA_CLANG: clang-18
        run: |
          cc --version
          ld --version
          cargo test --locked --workspace --all-features --no-fail-fast --test selfhost -- --ignored --nocapture --exact the_bootstrap_holds_s1_s2_s3
```

The job's `NOVA_EMBED_RUNTIME=1` and opt-level 1 hold here, so cargo
reuses the build and S0 carries the release runtime. **Linux only**
(decision 6): N's `build` refuses Windows (ADR 0059 §4), and macOS
linking has never run (ADR 0059's Consequences).

**A new step, not a name on 5.5b's line** (not asked): decision 2
places the chain in a step after the golden step; CI times it by
itself, as the gate asks (§14); it does not share the 4-vCPU runner
with the golden walk's four threads; and, needing no Cranelift S1, it
rebuilds nothing the golden step built. The cost: the CI step test reads
two steps (§6.2).

**The time effect of each** (projections, §8):
- **a new step:** +52 to 61 s on the job;
- **a name on 5.5b's line:** the chain runs beside the golden walk.
  That step's 31 s becomes 52 to 61 s if the two overlap fully, 83 to
  92 s if they do not: +21 to +61 s.

Either way the Selfhost job stays under the Test job (9:48 to 12:28):
**a pull request's wall time does not change.**

### 6.2 The CI step test

`the_ci_step_names_every_clang_test`, its name kept, holds both steps:
- each is in the Selfhost job; its command is its `run:` line or, after
  `run: |`, the block's `cargo test` line, with `--test selfhost`, `--`
  and `--ignored` before `--exact`;
- each name after `--exact` is a test marked `#[ignore`; no name is on
  both steps; the two steps' names together are exactly the tests
  ignored as "needs clang".

### 6.3 `cc` and `ld` (decision 7)

The step prints both versions, unpinned: the runner image fixes them
within a job. ADR 0060 records the condition. The runner's versions are
not known here.

## 7. Testing

Rules: no internet; fixed temporary names; `$NOVA_HOME` fresh; MSRV
1.78 (no `is_none_or`, `LazyLock`, `std::path::absolute`); no new crate,
`Cargo.lock` unchanged; `clang` installed by CI, never by a test.

- **`the_bootstrap_holds_s1_s2_s3`** (§3), in the new step.
- **`the_bootstrap_reports_name_the_stage`,** in the main step on all
  three systems (not asked; no `clang`, under 0.1 s): `ll_difference` on
  short `.ll` texts differing in a block, on a `define` line and outside
  any `define`; `same_binaries` on two small files one byte apart.
- **`the_ci_step_names_every_clang_test`,** extended (§6.2). The plan's
  mutants: the new name dropped, misspelt or on both steps; the new step
  without `--ignored`, outside the Selfhost job, or without its `cargo
  test` line.
- **The gate's mutants,** each to be caught: a stage compared with
  itself; the `.ll` comparisons skipped; a front-end run with a real
  `clang` (no `.ll` left); S2 linked with another runtime than S1's; S3
  built as `selfhosx` (one byte, §2 item 4), run once by hand on the
  prototype, its report recorded.
- **`git status` clean after a local run:** S0 builds N in place
  (unverified: the research built a copy).

## 8. CI time

- **Measured locally** (`chain-1.txt`, §2 item 3): S1 14.01 to 14.73
  s, S2 14.88 to 14.92 s, S3 14.46 to 15.07 s, in order; `cmp` 0.015 s.
  The front ends (1.47 to 2.06 s) were not run beside a build.
- **Projected, not measured:** the three builds, plus the front ends'
  contention and cargo's start: **about 43 to 45 s locally, 52 to 61 s
  on CI** (×1.2 to 1.35).
- **A margin,** since projections have missed both ways (5.4: +1 to 4
  s projected, +15 to 18 s measured; 5.5b's step: 46 to 57 s, 30.3 s):
  **plan for up to 75 s,** which the projection stays under. No line;
  the time is recorded.
- **The job:** about 4:11 to 4:35 on Linux, from 3:19 to 3:34.
- **The prototype on CI before the plan** (recommended, as for 5.5a and
  5.5b): with the user's word for that push, asked at that step, to
  `claude/loving-archimedes-frtf5q-56-ci` with a draft pull request,
  measurement only, closed unmerged: the new step, the golden step and
  `selfhost.rs` on all three systems; past 75 s or a line, ask.

## 9. The cut, for the plan

One pull request, split into tasks, test first (not asked):
1. **The prototype:** the CI step test extended, red until the step
   exists; the report test, red until `ll_difference` and the named
   `same_binaries` exist; `the_bootstrap_holds_s1_s2_s3`, red by hand on
   a wrong basename, then green; the step. Measured locally, then on CI.
2. **The measurements:** CI's figures; locally, stage times, sizes,
   digests and the `.keys()` count.
3. **The records** (§11).

## 10. Crates and files

`crates/nova-cli/tests/selfhost.rs` (the new test, `ll_difference`,
`same_binaries`' names, `runtime_in(home)` under `golden_runtime()`,
the report test, the CI step test);
`.github/workflows/ci.yml`; §11's files. S0's and N's sources, the
emitter, the runtime and `compare.rs` are unchanged, so ADR 0058 §16's
rule is not engaged. No new crate, no manifest change.

## 11. Records

- **ADR 0060, the bootstrap gate and its tolerances**
  (`docs/adr/0060-the-bootstrap-gate-and-its-tolerances.md`):
  - the gate (§3, §6); the conditions: one `-o` basename, clang
    18.1.3 (asserted), one runtime archive, one `cc`/`ld` in a job;
  - what varies the binary (the basename's `STT_FILE`, the runtime's
    bytes, the link driver) and what does not (timestamps, paths,
    `TMPDIR`, cwd, locale, the environment; the build-id, a content
    hash); so no byte tolerance, and phase plan decision 3's build-id
    clause (`phase-5-plan.md:243`) is not needed, `strip` alone not
    enough;
  - the Cranelift S1 is not used in the chain (decision 3): its code is
    Cranelift's, so it is another binary, though its `.ll` is S0's and
    the S2 it builds is S1 `--release` (§2 item 2);
    the search of N for `Map` iteration; the `.ll` differential (§5);
  - the figures, as of the tree (the digests and the runtime's
    `0.4.0-…` directory change at the bump); CI's, pending;
  - departures: phase plan decision 7's "CI job on `ubuntu-latest`" is
    a step of the Selfhost job on `ubuntu-24.04`; decision 3's
    tolerances are in ADR 0060, which the boundary ADR will cite.
- **`selfhost/README.md`,** 100 to 200 lines: what N is (Nova's
  compiler in Nova, source to `.ll`, then `clang` and `cc`; not shipped,
  no user's tool, phase plan decision 5); its commands, `tokens`, `ast`,
  `resolve`, `hir`, `mir [--ll]`, `build` and `check`, with their usage
  lines (`selfhost/src/main.nova:121-140`, `driver.nova`); S1, S2 and S3
  by hand, one `-o` basename, the release runtime through
  `NOVA_RUNTIME_LIB`, then `cmp`; how the differential runs (`cargo
  test --workspace --test selfhost`, the clang tests with `--ignored
  --exact`, N's `@test`s through `the_selfhost_tests_pass`); the rule
  that S0 and N change together in one pull request (phase plan
  decision 6, ADR 0058 §16); pointers to ADRs 0048 to 0060.
- **`CHANGELOG.md`,** Unreleased, Added: the bootstrap step and test,
  ADR 0060, `selfhost/README.md`.
- **Dated notes:** `docs/phase-5-plan.md` §5.6 (decision 1's split,
  5.6a's gate, the departures); `nova-spec/00-MASTER-SPEC.md`'s Phase 5
  gate (`:753`: stage2 == stage1 byte for byte, no tolerance, four
  conditions, ADR 0060).
- **Later pull requests** (decision 1): `v0.5.0`, with decision 8's
  notes; then the boundary ADR, in ADR 0047's shape, citing ADR 0060.

## 12. Risks

Ranked.

1. **CI time:** 52 to 61 s projected, where projections have missed 4
   to 5 times; the prototype comes first; past 75 s the user is asked.
2. **`cc` or `ld` changing between S1's build and S2's:** not within one
   job, and the chain runs in one step; the log shows the versions.
3. **Disk on the runner:** about 66 MB kept (three binaries, three
   `.ll`s), plus a build's transient `.ll` and object (not measured).
4. **A front-end run beside each build:** contention not measured.
5. **A later change that makes N's `.ll` nondeterministic** (a seeded
   `Map` iterated into output), which one chain may pass by chance:
   guarded by the `.ll` differential, §3.4 and ADR 0060's search.
6. **The release's version bump** changes the runtime cache's name and,
   likely, the runtime's bytes: digests recorded as of the tree only.

## 13. Not in 5.6a

- The release, its version bump, decision 8's notes; the boundary ADR.
- macOS and Windows (decision 6); a nightly job; pinning `cc` or `ld`.
- An extra rebuild in another environment; a `Map` lint (decision 5).
- Shipping `selfhost/` or an S2 binary (decision 8).
- Speed work: interned literals, unboxed `Option` and `Result` (phase
  plan decision 8); the chain is `clang`-bound.

## 14. Gate

- On `ubuntu-24.04`, in CI, on every pull request, the Selfhost job's
  new step runs `the_bootstrap_holds_s1_s2_s3`: S1 built by S0
  `--release`, then S2 by S1, then S3 by S2, in order, one `-o`
  basename; N's `.ll` equal from S0, S1 and S2; S1 == S2 == S3, byte
  for byte.
- The step's time is recorded, and `selfhost.rs`'s main step is within
  80, 100 and 110 s; or the user has ruled.
- ADR 0060 records the conditions, what varies the binary and what does
  not, and the search of N.
- `selfhost/README.md` exists, with §11's contents.
- The full suite, fmt, clippy (with `--test selfhost`), the MSRV job,
  `nova fmt --check` over `selfhost/`, `release_golden_suite`, and `git
  diff` on `Cargo.lock` are clean.

## Amendment (2026-11-22)

Dated, with the plan (`docs/superpowers/plans/2026-11-22-phase-5-6a-the-bootstrap-gate.md`),
after the prototype and its measurement on CI. Where this section and the
text above differ, this section holds; ADR 0060 records each point.

1. **The `.keys()` count** (§2 item 6). The research counted 14 sites;
   there are 17 lines, six of them in `fixes.nova`'s `did_you_mean`, as
   §2 already found. Each is sorted before use, given to `closest()` (a
   total order) or re-inserted into a `Map`; ADR 0060 §5 lists each.
2. **The debug runtime** (§3.6, "not measured"). S1 == S2 == S3 holds on
   the debug runtime beside S0, as a checkout runs the gate without
   `NOVA_RUNTIME_LIB`: 14,288,496 bytes each, N's `.ll` the same 12,760,111
   bytes. That size is this checkout's, with Rust 1.97's debug runtime,
   which holds its build paths: the same tree at another path gives
   another size, and the chain holds there too. It takes about 66 s
   (66.96 s on the prototype; 64.24 and 62.85 s in the plan's replay),
   not 45: on the debug runtime N's own front end takes 10 to 11 s.
3. **The local time** (§8, first two items). The test took 45.0 to 45.8
   s as CI runs it (`NOVA_EMBED_RUNTIME=1`, `opt-level = 1`) on the
   prototype, at the top of the 43 to 45 s projected, and 44.14 and
   44.16 s in the plan's replay. S2's and S3's builds took 15.4 to 16.1 s,
   against the research's 14.5 to 15.1 s in order: the front-end run
   beside each build, on a second core, explains the difference (§12,
   risk 4).
4. **S0's front end** (§2 item 3: 1.47 s) is that at `opt-level = 0`
   (1.42 to 1.71 s beside a build, in the prototype's two runs; ADR 0060
   §9), but 0.57 to 0.72 s at `opt-level = 1`, as CI's Selfhost job
   builds S0.
5. **§7's list grows, and so does §6.2's.**
   - **The comparisons are a function,** `bootstrap_failures`, which
     `the_bootstrap_reports_name_the_stage` runs on files laid out as the
     gate lays them out, one file made to differ at a time: its
     comparisons fail, and no other. On a correct tree a comparison that
     is skipped, loosened or made against the wrong stage changes nothing
     the gate can see, so §7's two reports alone let such mutants pass.
   - **Three cases for the plan's mutants:** the report test reaches
     `ll_difference`'s fallback place (one `.ll` goes on with a line that
     starts `== ` where the other ends: "line 13 of its .ll"), and a file
     that `ll_difference` or `same_binaries` cannot read (`cannot read
     PATH: …`); and the CI step test holds each clang step to
     `NOVA_REQUIRE_CLANG: "1"`, so that a runner without `clang` fails the
     step rather than skip its tests.
   - **Two checks after the plan's fact-check:** the CI step test holds
     each clang step to `if: runner.os == 'Linux'`, and the bootstrap
     step's command to `--nocapture` and its block to `cc --version` and
     `ld --version`; and the gate checks the runtime it hands on (item
     8).
   - **The review's checks,** from the review of the branch: the CI step
     test also holds each clang step to `NOVA_CLANG: clang-18` in its own
     `env:`, and refuses a `continue-on-error:` or `shell:` key in either
     step (among the step's own keys, so that the Test job's advisory
     step, `continue-on-error` on purpose, is not read), and `--skip`, or
     any flag after `--exact`, in its command; and the gate removes a
     caller's `CC` from every stage's command, so that every stage links
     with `cc`, whose version the step prints (ADR 0060 §2).
   - **The plan's mutants:** the prototype's 23, all killed, and 51 more
     for every check the diff adds that they miss, 74, all run in one run
     in the plan's replay (53 caught and 21 accepted); then 7 for the
     review's checks (C10 to C15, G5). All 81 run in one run at the
     branch's last commit: 60 caught and 21 accepted, each with its
     reason (ADR 0060 §8): 13 change output only, 4 are redundant with
     another check, 3 are equivalent, and 1 is reached only when a stage
     fails.
6. **CI's figures** (§8, last item; §14). The prototype was pushed to
   `claude/loving-archimedes-frtf5q-56-ci` with draft Sakeerin/nova#148,
   measurement only, closed unmerged. Run 45003631229, on `a453c61e`: 19
   of 19 jobs green. That run measured `a453c61e` only: the three cases,
   the two checks and the review's checks of item 5, added since, ran on
   this machine only, on Linux, and CI has run none of them on any
   system.
   - **The bootstrap step:** 27.58 s wall, the test `27.44 s in all`:
     S1 by S0 8.69 s, S2 by S1 9.45 s, S3 by S2 9.23 s, the front-end
     runs beside them 0.43, 1.29 and 1.25 s, the comparisons 0.07 s. It
     printed `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0` and `GNU ld (GNU
     Binutils for Ubuntu) 2.42`, `Ubuntu clang version 18.1.3
     (1ubuntu1)`, and S1 == S2 == S3, 9,539,736 bytes each, N's `.ll` the
     same from S0, S1 and S2, 12,760,111 bytes. §8 projected 52 to 61 s;
     it is under the 75 s margin by far.
   - **`selfhost.rs`'s main step** (`finished in`): 58.19 s on Linux (71
     passed, 3 ignored), 30.03 s on macOS (69, 3) and 91.31 s on Windows
     (65, 3), margins 21.81, 69.97 and 18.69 s against 80, 100 and 110 s.
   - **The golden step** 30.30 s; MSRV (1.78) passed; the Test job's main
     step `2838 passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9` on
     macOS and `2815, 0, 9` on Windows; the advisory step that runs the
     ignored GC tests exited 101 on Linux and macOS, as on `main`, not
     counted. The Selfhost jobs took 4:01, 1:35 and 5:32 (the longest
     job, the Test job on Windows, 7:44), so a pull request's wall time
     does not change.
   - **`main` at `83ba8210`,** run 44962982900: `selfhost.rs` 58.13 s on
     Linux, 83.52 s on macOS and 78.89 s on Windows (70, 68 and 64
     passed, 2 ignored each); the golden step 31 s; the Selfhost jobs
     3:19, 3:37 and 4:36.
7. **The gate's size and digest are as of the tree and the runtime**
   (§2 item 1, §12 risk 6). On CI S1 == S2 == S3 at 9,539,736 bytes; here
   at 9,354,680 (SHA-256 `877a8ee8…`). S0 on CI carries a runtime built
   there (`0.4.0-5549317a`), a different file from this machine's
   (`0.4.0-be7e8b48`), and the runtime's bytes vary the binary (§2 item
   4); N's `.ll` is one file on both. The gate holds on each machine,
   under one archive; its size and digest belong to the tree and to the
   runtime S0 carries, and differ between this machine and CI. CI's
   figures are recorded, not expected: CI builds its runtime with the
   current stable, so both can change, and no test pins a size or a
   digest.
8. **The open decisions, as settled** (ADR 0060 §12; each cheap to
   change):
   - the comparisons are a function, `bootstrap_failures`, tested in the
     main step (item 5);
   - a stage is its build and its front-end run, and either failing
     stops the chain; the later stages are not built;
   - S0's build is held to `built OUT` too, as N's is;
   - every build runs with its cwd at the gate's directory, and N by
     its path under the repository; after a local run `git status` is
     clean, which settles §7's "unverified";
   - `NOVA_LLC` and `NOVA_HOME` are set on N's runs too, though N reads
     neither; `NOVA_CLANG` is set on every build to the `clang` probed;
   - `bootstrap: not run (no clang)` goes to stderr, as the other clang
     tests' lines do;
   - the success report gives each stage's build time and its front-end
     run's, then the comparisons' time and the total;
   - `same_binaries` takes (name, path) pairs, so callers pass
     `.as_path()`;
   - `runtime_in`'s panic no longer names the golden walk, since two
     tests call it;
   - `ll_difference` has a fallback place, the line of the first
     differing byte, for a line that starts `== ` where the other `.ll`
     ends (item 5);
   - where S0 carries a runtime (`nova version`: `runtime: embedded`, as
     on CI) and `NOVA_RUNTIME_LIB` is not set, the gate fails unless the
     runtime it hands to S2 and S3 lies under its own `home/runtime/`; a
     plain checkout's `nova` carries none, and there the gate links the
     debug runtime beside S0 unchecked.
9. **§9's cut, as the plan makes it** (the plan's decision 12). Item 1
   is two tasks, test first: the comparisons and their report test,
   then the gate with its CI step and the CI step test. The gate passes
   from its tests commit, since S0 and N predate 5.6a; its red is shown
   by hand on a wrong basename (S3 built as `selfhosx`) and by the
   plan's mutants. Item 2, the measurements, is no task: CI was measured
   from the prototype (item 6), and the local figures again at the
   plan's last commit. The records are the last task.
