# ADR 0060 — The bootstrap gate and its tolerances

## Status

Accepted (2026-11-22). Branch `claude/loving-archimedes-frtf5q-56`
(`docs/superpowers/specs/2026-11-22-phase-5-6a-the-bootstrap-gate-design.md`),
written as 5.6a's prototype was built. The figures in §9 are local.
CI's figures are §10's, from the prototype's draft pull request
Sakeerin/nova#148 (measurement only, closed unmerged); no line was
passed.

This ADR records the gate's tolerances, which phase plan decision 3
leaves to "the boundary ADR". That ADR closes Phase 5 after the release
(the user's decision 1) and cites this one.

## Context

- **The master spec's Phase 5 gate** (`nova-spec/00-MASTER-SPEC.md`):
  "CI builds stage2 == stage1 byte-for-byte (or close — may differ in
  non-deterministic codegen, document tolerances)".
- **Phase plan decision 3** (`docs/phase-5-plan.md`) asks for three
  things, on Linux, in CI, with one pinned `clang`:
  1. S0 and S1 emit identical `.ll`;
  2. S1 == S2;
  3. S2 == S3.

  Its tolerances: one intermediate name, a pinned `clang`, no build-id,
  and Linux only.
- **Decision 7** asks for a CI job on every pull request.
- **Decision 9** asks for determinism by construction: a seeded `Map`
  is for lookups only, which 5.6's spec checks by a search of N.
- **The names:**
  - **S0:** the Rust compiler.
  - **N:** the Nova compiler in `selfhost/`.
  - **S1** = S0 `--release`(N), **S2** = S1(N), **S3** = S2(N).
- **The research of 2026-11-22**, on `83ba8210`, found S1 == S2 == S3
  already, over 12 builds, with one `-o` basename. It also probed what
  varies the binary (§3).
- **The user's decisions of 2026-11-22** are in the spec's header:
  - S1 is S0 `--release`'s (decision 3);
  - the gate is an ignored test in `selfhost.rs`, in a step of the
    Selfhost job's Linux leg (decisions 2 and 4);
  - one chain, its conditions recorded (decision 5);
  - Linux only (decision 6);
  - `cc` and `ld` printed, not pinned (decision 7);
  - the literal chain, built in order (the approval).

## Decision

### 1. The gate

On `ubuntu-24.04`, on every pull request, the Selfhost job's step "The
bootstrap, S1 == S2 == S3 (Linux)" runs `the_bootstrap_holds_s1_s2_s3`
(`crates/nova-cli/tests/selfhost.rs`). The test does four things.

1. **It builds the chain, strictly in order.** Each stage goes into a
   fixed directory of its own under `nova-selfhost-bootstrap/`:
   - S0 `nova build --release N -o s1/selfhost`, with a fresh
     `NOVA_HOME` (`home/`);
   - S1 `build N -o s2/selfhost --release --std STD`;
   - S2 `build N -o s3/selfhost --release --std STD`.

   N is `selfhost/src/main.nova` in place, and STD is `std/`. S0 links
   `NOVA_RUNTIME_LIB` if the environment sets it. Otherwise it links its
   embedded runtime, which it unpacks into
   `home/runtime/<version>-<crc32>/`, or else the runtime beside it.
   After S1's build, `runtime_in(home)` finds that same file in the same
   order, and S2's and S3's builds link it through `NOVA_RUNTIME_LIB`.
   When S0 carries a runtime (`nova version` prints `runtime: embedded`,
   as on CI) and `NOVA_RUNTIME_LIB` is not set, the test also checks that
   this file lies under `home/runtime/`, not beside S0.
2. **It writes each stage's `.ll` of N** in a front-end-only run, beside
   that stage's build and only there. The run is the same command into
   `ll-s0/`, `ll-s1/` or `ll-s2/`, with `NOVA_CLANG` (and S0's
   `NOVA_LLC`) set to `no-such-tool`, which is never created. It must
   exit 1 and leave `selfhost.nova.ll`. A build that succeeds removes its
   own `.ll`, so this run is the only way to keep one. It needs no
   flag, no wrapper and no change to either driver.
3. **It compares, in order:**
   - N's `.ll` from S1, then from S2, against S0's;
   - then S2 against S1, and S3 against S2, byte for byte.

   Every comparison runs. The test fails with every one that fails
   (`bootstrap_failures`).
4. **It reports.**
   - On success it prints the sizes, `clang`'s version, the runtime's
     path and each stage's time. Then it removes the directory.
   - On a failure it keeps the directory, and says that it does.
   - A stage that fails stops the chain, with the stage, the command,
     the status and the stderr.
   - A `.ll` mismatch names the line of the `.ll`, the `define` and the
     block (`ll_difference`, through `compare::first_difference`), and
     prints the `diff` command for the two kept files.
   - A binary mismatch names both stages, both sizes and the first
     differing offset (`same_binaries`).

### 2. The conditions, and no tolerance on the bytes

S1 == S2 == S3 holds byte for byte, with nothing tolerated, under four
conditions:

1. **One `-o` basename across the stages.** The test uses `selfhost`.
   The `.ll`'s name, `selfhost.nova.ll`, reaches the binary as its
   `STT_FILE` symbol (§3).
2. **One `clang`, 18.1.3.** The job's install step asserts the version
   (ADR 0048), and the test prints it.
3. **One runtime archive, the same bytes,** for all three links. The
   test passes S1's runtime on to S2 and S3.
4. **One `cc` and one `ld` within a job.** They are not pinned: the
   runner image fixes them for the job, and the step prints both
   versions (the user's decision 7). The chain runs in one step, so it
   runs under one of each.

So phase plan decision 3's build-id clause ("no build IDs:
`-Wl,--build-id=none`, or a comparison after `strip`") is not needed.
GNU ld's build-id is a hash of the content, and identical inputs give
identical ids (§3). The `strip` half would not hide a basename of
another length in any case: the build-id still differs after `strip`.

### 3. What varies the binary, and what does not

These are the research's probes, made on 2026-11-22 on `83ba8210`
(spec §2 item 4). In each probe S2 was rebuilt with one thing changed
and compared with the reference S2.

**What varies the binary:**

| Changed | Result |
|---|---|
| the basename, same length (`selfhosx`) | 1 byte differs, the `STT_FILE` `selfhosx.nova.ll`; the build-id is the same |
| the basename, shorter (`stage2`) | 367,460 bytes and the build-id differ; after `strip`, the 20 build-id bytes still differ; `strip -R .note.gnu.build-id` makes them equal |
| the runtime archive (the debug one) | another binary, 14,288,496 bytes |
| the link driver (`CC=clang-18`) | 5,586,894 bytes differ |

**What does not:**
- `SOURCE_DATE_EPOCH`, and builds made minutes apart;
- the runtime's path (the same file copied elsewhere);
- `CC=gcc`;
- `NOVA_CLANG=clang` (the same 18.1.3; no other version is installed);
- `env -i` with only `PATH`, `LANG`/`LC_ALL=C`, and a 3,000-byte variable
  with `HOME=/nonexistent`;
- N and std at longer paths, relative paths, another cwd and another
  `TMPDIR`, for both S2 and S0 `--release`;
- nine front-end runs, which gave one `.ll`.

**Why.**
- N's object has no debug section and names no path; its only file
  name is `STT_FILE`. The binary's `.debug_*` sections and paths come
  from the runtime archive (ring's C, `/rustc/<hash>`), so they are the
  same whenever the archive is.
- N reads only `NOVA_CLANG`, `CC`, `NOVA_RUNTIME_LIB` and
  `current_exe` from outside (`selfhost/src/driver.nova`).
- std's modules come from a fixed list (`STD_MODULES`,
  `selfhost/src/builtins.nova`). `read_dir` only tests membership.

The prototype repeated two of these probes in the test itself:
- the same-length basename, as a mutant (§8, G1): "S3's binary differs
  from S2's: 9354680 bytes against S2's 9354680, first at offset
  8970099", where `readelf -s` shows `selfhosx.nova.ll`;
- the debug runtime for S2 alone (§8, G4): 14,288,496 bytes against
  9,354,680.

### 4. The Cranelift S1 is not used in the chain

The Selfhost job's other steps build N with S0's default backend,
Cranelift (`build_selfhost`). That S1 is another binary (9,376,184
bytes, SHA-256 `c2e8a846…`): its machine code is Cranelift's, not
`clang`'s. Its `.ll` is S0's, and the S2 it builds is S1 `--release`,
byte for byte. So "S1 == S2" holds literally only for the `--release`
S1, and the chain starts there (the user's decision 3). The test never
calls `selfhost()`. Of what `build_selfhost` gives, it needs only a
runtime, and S0's own unpacking gives it that. So the step rebuilds
nothing that the golden step built.

### 5. The search of N for `Map` iteration

Decision 9 holds by construction, and the search re-checks it on the
tree as of `45a06d1d`.

- **std's `Map` has one way to iterate:** `keys()`. Its order is
  unspecified ("it can change from one run of a program to the next",
  `std/collections/lib.nova`).
- **N uses no `OrderedMap`,** and uses its two `Set`s only for
  membership (`insert`'s result): `loader.nova:368` and
  `typeck_traits.nova:1083`.
- **`grep -rn '\.keys()' selfhost/`** finds **17 lines**. Each falls in
  one of four classes, and none lets the map's order reach the output:

| Class | Lines |
|---|---|
| sorted before use (`sort_strings`) | `resolution_dump.nova:54, 316, 378, 381, 384` (5) |
| sorted and made unique (`sorted_unique`) | `typeck_body.nova:125` (1) |
| candidates for `closest()` | `fixes.nova:521, 527, 528, 530, 538, 545` (`did_you_mean`, through `change_to`), `typeck_pattern.nova:722`, `typeck_expr.nova:281`, `typeck_ty.nova:184` (both through `name_fixes`) (9) |
| re-inserted into a `Map` | `typeck.nova:879` (std's `impl_selves` into the program's), `typeck.nova:1027` (`copy_scope`) (2) |

- **`closest()` gives the same answer in any order**
  (`selfhost/src/diagnostic.nova`, `closest`), as checked for this ADR.
  It skips the name itself. It keeps the best candidate by the triple
  (far, distance, bytes): a name equal but for ASCII case comes first,
  then the least edit distance, then the least in byte order (`less`).
  That is a strict lexicographic minimum, so the result is the least
  candidate whatever the order of the candidates. Two candidates equal
  in all three are the same string.
- **A re-inserted map's order** is again unspecified. Its later reads
  are lookups, or the classes above: `copy_scope`'s scopes reach only
  `visible_locals` (sorted and unique) and `did_you_mean`.

The research counted "14 `.keys()` sites". Its own classes sum to 15
(5 sorted, 7 for `closest()`, 1 sorted and unique, 2 re-inserted). The
lines number 17, because `fixes.nova`'s `did_you_mean` holds six.
Neither count is a gate. A later change that iterates a `Map` into
output is guarded three ways:
- the `.ll` differential (§6), on every pull request, on three systems;
- this test's `.ll` comparison;
- this search, which the boundary ADR repeats.

No lint is added (the user's decision 5).

### 6. The `.ll` differential: recorded, not added

Decision 3's first item is "S0 and S1 emit identical `.ll` on
`tests/runtime/`, `examples/` and N". It is part of 5.5a's corpus,
which the Selfhost job's main step runs on Linux, macOS and Windows on
every pull request. That step compares S0's `check --dump-mir --ll`
against N's `mir --ll` on 778 `.ll`s, N's root among them (ADR 0058
§11), with the Cranelift S1. 5.6a adds no check for it.

The new test's `.ll` comparison is not a second corpus gate. It covers
N's root alone, on Linux, through `build`, from S1 and S2, which `clang`
compiled. Its job is to place a binary mismatch: equal `.ll`s and
differing binaries point at the toolchain or the link, and differing
`.ll`s point at the front end.

### 7. The CI step, and the test that holds it

The step comes after "S1 builds the golden suite (Linux)" and before
"N's own tests on the debug runtime". It runs under the job's
`NOVA_EMBED_RUNTIME=1` and `opt-level = 1`, so cargo reuses the build:

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

**Why a step of its own, not a name on 5.5b's line** (spec §6.1):
- CI times it by itself;
- it does not share the 4-vCPU runner with the golden walk's four
  threads;
- it needs no Cranelift S1, so it rebuilds nothing.

`--nocapture` shows the success report in the log.

**`the_ci_step_names_every_clang_test`** keeps its name and now holds
both clang steps:
- each step is in the Selfhost job;
- each step sets `NOVA_REQUIRE_CLANG: "1"`, so that a runner without
  `clang` fails it rather than skip its tests;
- each step carries `if: runner.os == 'Linux'`, since only that leg
  installs `clang-18`;
- the bootstrap step's command has `--nocapture`, and its block prints
  `cc --version` and `ld --version`, so that its log shows the report
  and the two tools (§2's fourth condition);
- each step's command is its `run:` line, or, after `run: |`, the
  block's `cargo test` line;
- the command has `--test selfhost`, `--` and `--ignored` before
  `--exact`;
- each name after `--exact` is a test marked `#[ignore`;
- no name is on both steps, or twice;
- the two steps' names together are exactly the tests `selfhost.rs`
  ignores as "needs clang".

### 8. The mutants

Each mutant was run once on the prototype, in the working tree, and
reverted; none was committed. The exact edits are in the plan's Task 4,
Step 8 (`docs/superpowers/plans/2026-11-22-phase-5-6a-the-bootstrap-gate.md`),
mutants 1 to 23. **All 23 were killed.**

**Killed by `the_bootstrap_reports_name_the_stage`** (the main step, no
`clang`):
- M1 to M4: each of the four comparisons skipped;
- M5: a stage compared with itself (S3 against S3);
- M6: `same_binaries`' `==` loosened to the sizes;
- M7: `ll_difference` passing on equal sizes;
- M8: both `.ll` comparisons gone;
- M9: `same_binaries`' offset one off;
- M10: `ll_difference` without the `(Mir` line, so with no place;
- M11: `ll_difference`'s place replaced by the line alone.

**Killed by `the_ci_step_names_every_clang_test`:**
- M12: the new name dropped;
- M13: the new name misspelt;
- M14: the new name on both steps;
- M15: `--ignored` dropped;
- M16: the step moved into the `vscode` job;
- M17: the step's `cargo test` line deleted;
- M18: the ignore reason changed;
- M19: the `#[ignore]` removed.

**Killed by `the_bootstrap_holds_s1_s2_s3`** (by hand, with `clang`):
- G1: S3 built as `selfhosx` (§3);
- G2: S2 built by S0 instead of S1. S0 refuses N's `--std` with exit 2,
  and the chain stops at S2.
- G3: S0's front-end run with the real `clang`. It exits 0 and leaves no
  `.ll`, and the chain stops at S1.
- G4: S2 linked with the debug runtime. Two binary comparisons fail.

**Two mutants are equivalent, argued and not run:**
- **S2 built by S0 with S0's own command line.** S0 `--release` gives S1
  by the gate's own claim, and both print `built OUT`. The stage's
  command form follows its index, not its builder, so G2 is the form
  that can be killed.
- **S3 built by S1.** S1 == S2.

**A case added for these mutants.** On a correct tree, a comparison
that is skipped or loosened changes nothing the gate test can see. So
the comparisons are a function, `bootstrap_failures`, and the report
test lays out the gate's files and makes one file differ at a time.
That comparison, and no other, must fail. This addition is not in the
spec's §7, which lists only the two reports.

**The plan's mutants.** The plan runs the 23 again with 51 more, all in
one run at its last commit, with S0 carrying the release runtime as on
CI: 74 in all, **53 caught and 21 accepted**, each with its reason. The
51 cover every check the diff adds that the 23 miss:
- **the two reports' words** (W1 to W9, B1 to B5): each part of
  `ll_difference`'s and `same_binaries`' reports, their fallback place
  and its line, and a file neither can read. All 14 are caught by
  `the_bootstrap_reports_name_the_stage`.
- **the CI step test and the step** (C1 to C9): the `run: |` block's
  first line taken as the command; the step without
  `NOVA_REQUIRE_CLANG`, without `--nocapture`, or without its `if:`; the
  golden step without its `if:`; and the step without `cc --version`,
  `ld --version` or both. All eight are caught by
  `the_ci_step_names_every_clang_test`. One is accepted: the check that
  no name is on both steps removed (redundant: such a name is counted
  twice, and the final equality fails).
- **the gate's own report** (GW1 to GW7, GS1 to GS5): its failure lines
  and its success lines. All 12 are accepted as output only: the
  verdict does not depend on the words, and G1 to G4 show the failure
  lines in full.
- **the gate's checks** (GB1 to GB5), all accepted: `built OUT` not
  checked (held elsewhere: S1's by the golden walk, N's against S0's by
  the clang test, S0's by `project.rs`); a front-end run's status, or
  its `.ll`, not checked (redundant: only a run stopped at `clang`
  leaves a `.ll`, and a `.ll` not left fails its comparison); a failed
  stage not stopping the chain (reached only on a failure); and S0's
  front-end run without `NOVA_LLC`, which is equivalent with an `llc`
  on `PATH` or without one. With one, S0 compiles the `.ll` with it,
  and `cc` refuses to link its non-PIE object; without one, S0 stops
  with "no LLVM toolchain found". Either way the run exits 1 and leaves
  the same `.ll`. The gate sets `NOVA_LLC`, so it depends on neither.
- **the runtime's handoff between stages** (H1 to H5): no runtime
  handed to S2's and S3's builds (N finds none, and the chain stops at
  S2) and the runtime taken from the front-end runs' home (the debug
  one, so S2 differs from S1) are caught by the comparisons; the
  runtime taken before S1's build, so that all three link the debug one
  beside S0, is caught by the gate's check that S2's and S3's runtime
  lies under `home/runtime/` when S0 carries one (§1). Two are
  accepted, both equivalent: no runtime handed to the front-end runs,
  and the runtime taken again after each stage.
- **`runtime_in`** (R1 to R3): the wrong directory under `home` is
  caught by the gate; `NOVA_RUNTIME_LIB` not read is caught by the gate
  run with `NOVA_RUNTIME_LIB` set; `home` ignored for the golden walk's
  fixed directory is caught twice: by the runtime check, and by the gate
  run with `TMPDIR` at a fresh, empty directory, where the golden walk's
  directory does not exist, so S2 takes the debug runtime and differs
  from S1.
- **the no-clang skip** (N1 to N3): the requirement dropped, or read the
  other way, is caught by the gate run with `NOVA_CLANG` a missing file:
  under `NOVA_REQUIRE_CLANG=1` it must fail, and without it pass. The
  `not run (no clang)` line's words are accepted as output only.

**Cases added for the plan's mutants.** Three checks would otherwise
have let mutants pass, so the prototype gained them before the plan:
the report test reaches `ll_difference`'s fallback place (one `.ll` goes
on with a line that starts `== ` where the other ends: "line 13 of its
.ll") and a file that `ll_difference` or `same_binaries` cannot read;
and the CI step test holds each clang step to `NOVA_REQUIRE_CLANG: "1"`.
The plan's fact-check added two more: the CI step test holds each clang
step to its `if:`, and the bootstrap step to `--nocapture`, `cc
--version` and `ld --version` (C4 to C9); and the gate checks the
runtime it hands to S2 and S3 (H4, R2).

### 9. The figures, as of the tree

These were measured on 5.6a's prototype on 2026-11-22: 4 cores,
`clang-18` 18.1.3, `cc` (Ubuntu 13.3.0-6ubuntu2~24.04.1), GNU ld 2.42.
"CI-style" means as the Selfhost job runs: `NOVA_EMBED_RUNTIME=1
CARGO_PROFILE_DEV_OPT_LEVEL=1`, with the runtime that S0 carries.

**The bytes:**

| Output | Bytes | SHA-256 |
|---|---:|---|
| S1, S2, S3 | 9,354,680 | `877a8ee8cd27ed4452316ec9387733303cfa7033d793d2aa9ccb6f3bc1c446e2` |
| N's `.ll` from S0, S1, S2 (406,253 lines) | 12,760,111 | `ae8ce14baa34386e7c3c37323c2ddce0d875c83478485def8777eece2bb382b7` |
| the runtime, `home/runtime/0.4.0-be7e8b48/libnova_runtime.a` | 28,790,180 | `7924727dc09fc8bba33ff56fb481e5cdfa6b5c6d65ce30394bf3ca68f4325065` |

They match the research's figures on `83ba8210`. The digests, and the
runtime's `0.4.0-…` directory, will change at the release's version
bump, so they hold as of this tree only.

**The test as CI runs it** (seconds; each stage's build, with its
front-end run beside it):

| Run | S1 by S0 | S0's `.ll` | S2 by S1 | S1's `.ll` | S3 by S2 | S2's `.ll` | compare | test | wall |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `C` 1 | 13.84 | 0.61 | 15.43 | 2.07 | 15.57 | 2.37 | 0.13 | 45.02 | 45.28 |
| `C` 2 | 14.30 | 0.69 | 15.66 | 2.17 | 15.69 | 2.28 | 0.11 | 45.77 | 45.97 |
| step 1 | 13.83 | 0.57 | 16.07 | 2.18 | 15.45 | 2.42 | 0.13 | 45.52 | 45.77 |
| step 2 | 14.10 | 0.72 | 15.95 | 2.54 | 15.44 | 2.22 | 0.11 | 45.60 | 45.86 |

The "step" rows ran the step's whole `run:` block, `cc --version` and
`ld --version` included, on a built tree.

**Other runs:**
- **At opt-level 0, with `NOVA_RUNTIME_LIB` naming the release runtime:**
  47.10 and 45.78 s in the test. S0's front end took 1.42 to 1.71 s
  there, in these two runs, against 0.57 to 0.72 s at opt-level 1 (the
  table above).
- **At opt-level 0 with the debug runtime beside S0,** as a checkout
  runs it without `NOVA_RUNTIME_LIB`: **S1 == S2 == S3 holds**, at
  14,288,496 bytes, in 66.96 s. That settles the spec's §3.6, which had
  not measured it. On the debug runtime N's own front end takes 10.4 to
  11.1 s.
- **Disk:** the test's directory peaks at about 95 MB on disk:
  - three binaries;
  - three `.ll`s;
  - the 28.8 MB runtime that S0 unpacks.

  `df` on `/` read 7,095 MiB free before the run and 7,025 MiB at its
  peak.
- **`selfhost.rs`'s main step as CI runs it,** before (`45a06d1d`) and
  after, interleaved:
  - before: 69.35 and 64.48 s, `70 passed; 2 ignored`;
  - after: 64.98 and 66.34 s, `71 passed; 3 ignored`.

  The two main-step tests cost nothing that can be measured; the spread
  is the machine's.
- **The golden step's line as CI runs it:** 39.18 s, 2 passed.

**Against the spec's projection** (§8: 43 to 45 s locally):
- The test took 45.0 to 45.8 s CI-style, at the top of the range.
- S2's and S3's builds took 15.4 to 16.1 s, against the research's
  14.5 to 15.1 s in order. The front-end run beside each build, on a
  second core, explains the difference, as risk 4 expected.
- The spec's projection for CI was 52 to 61 s (§8: its 43 to 45 s at
  ×1.2 to 1.35), under the 75 s margin. CI took 27.58 s (§10).

### 10. CI's figures

From the prototype's draft pull request Sakeerin/nova#148 (measurement
only, closed unmerged): run 45003631229 on `a453c61e`, the commit CI
measured, 19 of 19 jobs green. `main`'s run at `83ba8210` (5.5b's
merge) is 44962982900. Run 45003631229 ran `a453c61e` only: since then
the prototype has added three cases for the plan's mutants (two in the
report test, and the CI step test's check of `NOVA_REQUIRE_CLANG`); the
CI step test's checks of each clang step's `if:`, and of the bootstrap
step's `--nocapture`, `cc` and `ld`; and the gate's check that S2's and
S3's runtime is the one S0 unpacked under its home (§8). CI has run
none of them on any system.

**The bootstrap step,** "The bootstrap, S1 == S2 == S3 (Linux)": 27.58
s wall; the test `27.44 s in all`, `finished in 27.46s`, `1 passed`,
`73 filtered out`. Each stage's build, with the front-end run beside it
(seconds):

| Stage | Build | The builder's `.ll` beside it |
|---|---:|---:|
| S1 by S0 | 8.69 | 0.43 |
| S2 by S1 | 9.45 | 1.29 |
| S3 by S2 | 9.23 | 1.25 |

The comparisons took 0.07 s. The step printed:
- `cc (Ubuntu 13.3.0-6ubuntu2~24.04.1) 13.3.0` and `GNU ld (GNU Binutils
  for Ubuntu) 2.42`, the versions measured here (§9);
- `Ubuntu clang version 18.1.3 (1ubuntu1)`;
- S1 == S2 == S3, 9,539,736 bytes each, and N's `.ll` the same from S0,
  S1 and S2, 12,760,111 bytes, as here;
- the runtime, `home/runtime/0.4.0-5549317a/libnova_runtime.a` under
  the test's directory.

**The binaries' size differs from §9's 9,354,680 bytes** because CI's
embedded runtime differs: S0 on CI carries a runtime built there
(`0.4.0-5549317a`), a different file from this machine's
(`0.4.0-be7e8b48`, §9). The `.ll` is the same file on both machines,
and the runtime's bytes vary the binary (§3). So the gate holds "as of
the tree", on each machine with the runtime that S0 carries there:
S1 == S2 == S3 under one archive. The size and the digest are the
tree's and the runtime's, and they differ between this machine and CI.

27.58 s is below this machine's 45.0 to 45.8 s (§9), below the spec's
projection of 52 to 61 s for CI (spec §8), and well under its 75 s
margin.

**`selfhost.rs`'s main step** (`finished in`; passed, failed, ignored),
against the Selfhost job's lines (ADR 0057 §19). The margin is the line
minus the time:

| System | Line | Run 45003631229 | Counts | `main`, run 44962982900 |
|---|---:|---:|---|---:|
| Linux | 80 s | 58.19 s (margin 21.81) | `71 0 3` | 58.13 s, `70 0 2` |
| macOS | 100 s | 30.03 s (margin 69.97) | `69 0 3` | 83.52 s, `68 0 2` |
| Windows | 110 s | 91.31 s (margin 18.69) | `65 0 3` | 78.89 s, `64 0 2` |

Each system adds the report test to the main step, and the gate to the
ignored tests. `the_ci_step_names_every_clang_test` and
`the_bootstrap_reports_name_the_stage`, as `a453c61e` has them, passed
on all three. The times
move with the runners: macOS took 53 s less than on `main`'s run, and
Windows 12 s more. No line was passed.

**The other steps on Linux:**
- **S1 builds the golden suite:** 30.30 s wall (`finished in 30.18s`,
  `2 passed`), against `main`'s 31 s.
- **N's own tests on the debug runtime:** `1 passed` in 1.16 s.

**The whole Selfhost job:** 4:01 on Linux (`main`: 3:19), 1:35 on macOS
(3:37) and 5:32 on Windows (4:36). The longest job of the run was the
Test job on Windows, 7:44, so a pull request's wall time does not
change.

**The other jobs:**
- **MSRV (1.78):** passed, `cargo check --locked --workspace` with
  `rustc 1.78.0`.
- **The Test job's main step,** its `test result:` lines summed: `2838
  passed, 0 failed, 10 ignored` on Linux, `2838, 0, 9` on macOS and
  `2815, 0, 9` on Windows, as on `main`.
- **The advisory step that runs the ignored GC tests** exited 101 on
  Linux and macOS, as it does on `main`, and is not counted.

### 11. Departures

- **Phase plan decision 7:** "a CI job on `ubuntu-latest`" is a step of
  the Selfhost job on `ubuntu-24.04` (the user's decision 2). The job
  already has the build, the embedded runtime and `clang-18`.
- **Phase plan decision 3's tolerances** are recorded here, not in the
  boundary ADR, which will cite this one:
  - the intermediate name is one `-o` basename;
  - `clang` is one 18.1.3;
  - no build-id tolerance is needed (§2);
  - Linux only.

  This ADR adds two conditions: one runtime archive, and one `cc`/`ld`.
- **The master spec's gate** reads "or close …, document tolerances".
  There is no tolerance: stage2 == stage1 byte for byte, under §2's four
  conditions.

### 12. Decisions the spec does not settle

Each decision comes with its cost if it is wrong.

1. **The comparisons are a function, tested in the main step** (§8).
   Cost: none; the report test takes about 0.01 s.
2. **A stage is its build and its front-end run, and either failing
   stops the chain.** Cost: if only the `.ll` run failed, the later
   stages' binaries go unbuilt; the report names the failure.
3. **`NOVA_LLC` is set on N's front-end runs too.** N does not read it.
   Cost: none.
4. **Every build runs with cwd at the test's directory, N by its
   absolute path** (`crates/nova-cli/../../selfhost/src/main.nova`).
   The probes show that neither cwd nor the path reaches the binary.
   After a local run `git status` was clean, which settles the spec's
   "unverified".
5. **S0's build is held to `built OUT` too,** which S0 prints as N does.
   Cost: none.
6. **`NOVA_CLANG` is set on every build to the `clang` probed,** and
   `NOVA_HOME` on N's builds too, though N does not read it. Cost: none.
7. **`bootstrap: not run (no clang)` goes to stderr,** as the other clang
   tests' lines do. Cost: none.
8. **The report gives each stage's build time and its front-end run's,**
   then the comparisons' time and the total.
9. **`same_binaries` takes (name, path) pairs,** so callers pass
   `.as_path()`: a tuple does not coerce `&PathBuf`.
10. **`runtime_in`'s panic** no longer names "the golden walk", since
    two tests call it.
11. **`ll_difference` has a fallback place** for a line that starts
    `== ` where the other `.ll` ends. LLVM IR never has such a line;
    the report test reaches it with one (§8).
12. **The runtime handed to S2 and S3 is checked** to lie under
    `home/runtime/` when S0 carries a runtime and `NOVA_RUNTIME_LIB` is
    not set (§1), S0 asked by `nova version`. A checkout's plain `nova`
    carries none, so there the gate links the debug runtime beside S0
    unchecked, as before. Cost: one `nova version` run.

## Consequences

- CI holds S1 == S2 == S3 byte for byte, and N's `.ll` equal from S0,
  S1 and S2, on every pull request on Linux. This is phase plan
  decision 3's second and third items; the first has run since 5.5a.
- A mismatch is placed:
  - in the front end, when the `.ll`s differ;
  - in the toolchain or the link, when the `.ll`s are equal and the
    binaries differ.

  Either way the files are kept for a `diff` or a `cmp`.
- The Selfhost job's Linux leg gains about 45 s locally and 27.58 s on
  CI (§10). The job stays under the Test job, so a pull request's wall
  time does not change.
- The binaries' size and digest are recorded as of the tree and the
  runtime: this machine's and CI's differ, since each S0 carries its
  own runtime (§10).
- A later change to N that varies its output from run to run is caught
  by the `.ll` differential or by this gate, with §5's search as the
  record.
- macOS and Windows stay outside the gate (the user's decision 6). They
  go to the boundary ADR's backlog with an unpinned `cc` and `ld`.
- S0, N, std, the emitter, the runtime and `compare.rs` are unchanged,
  so ADR 0058 §16's rule is not engaged. No crate was added, and
  `Cargo.lock` is unchanged.
