# Phase 4.8, "The gate and `v0.4.0`": design

- **Date:** 2026-10-22.
- **Status:** a draft, for the user's approval.
- **The user's decisions, 2026-10-22:**
  1. **Lighthouse measures the mobile preset,** with its default simulated
     throttling, three runs a page; a page passes when the median of its
     three Performance scores is 95 or more.
  2. **It measures the public pages,** 06's `/` and 07's `/`, `/posts/1`
     and `/login`, in a new step of CI's `browser` job,
     `tests/browser/lighthouse.mjs`, on the Chromium the job already
     installs for Playwright, with `lighthouse@13.5.0` pinned.
  3. **`nova build --release` is fixed for a real `wasm-opt`:** it passes
     `--enable-bulk-memory`, since the backend writes `memory.fill` and
     `memory.copy`, and CI tests it with binaryen itself.
  4. **07 gets no size limit;** its size stays printed. **A KB is 1,024
     bytes,** as the size tests measure, and the records say KiB.
  5. **Phase 4 is complete before the tag** (the `v0.2.0` model): a 4.8
     pull request (the gate, the fix, the documentation and ADR 0047),
     then the release pull request (the version), then a records pull
     request (the run IDs), since this gate needs nothing published.
  6. **The user opens both examples from `main`,** after the 4.8 pull
     request merges and before the tag, by their READMEs; after the
     release, the published `nova` is tried again in place of the
     runbook's step 6.2.
  7. **`nova-bundler` stays a stub.** ADR 0047 records the master spec's
     bundler as shipped in `nova-cli`, as `nova-test`'s case was; nothing
     depends on the crate, so it is not published.
  8. **The release also:**
     - runs `nova build --target wasm` in `gate.sh`, so a packaged and an
       installed `nova` are tested on WASM;
     - gives the root README and `nova-cli`'s README a "Web" section;
     - publishes the VS Code extension at 0.4.0 to both stores, with a
       real CHANGELOG entry.
- **Branch:** `claude/loving-archimedes-frtf5q-48`, from `main` at
  `24e7d7f`, as 4.7 used `-47`.
- **Inputs:**
  - `docs/phase-4-plan.md`: decision 12 (the gate's measurement), §4's 4.8
    entry, §5, risk 4;
  - `nova-spec/00-MASTER-SPEC.md` §7 (when a phase is done) and its Phase 4
    list and gate;
  - ADR 0037, the Phase 3 boundary, the model for ADR 0047; ADRs 0038 to
    0046, which leave Lighthouse, the size checks and a backlog to 4.8;
  - `docs/releasing.md`, the runbook.

## 1. What 4.8 delivers

- **The gate's measurement in CI** (§3): Lighthouse on both examples, and
  the size checks as they stand.
- **A working `--release` for WASM** (§4), tested with binaryen in CI.
- **The documentation Phase 4 owes** (§5): a "Web" section in the two
  READMEs, and stale lines corrected.
- **ADR 0047, which closes Phase 4** (§6).
- **The release `v0.4.0`** (§7), by the runbook, every publish and the tag
  on the user's word.

## 2. The starting point

Read on 2026-10-22, at `24e7d7f`.

- **No Lighthouse step exists.** The `browser` job (`ci.yml`) runs
  `counter.mjs`, `spa.mjs`, `dev.mjs`, `router.mjs`, `ssr.mjs` and
  `blog.mjs`, which already drive 06 and 07 as decision 12 asks.
- **The size tests** (`wasm_tests.rs`) gzip `app.wasm` and `app.js` apart,
  with `flate2` at level 9, add the two, and assert hello world under
  `30*1024` and 06 under `50*1024`, in the `test` job on all three
  systems. Today: hello world 4,959 bytes, 06 20,997 (21,070 with the
  test's own build), 07's client 49,472, printed by `blog.mjs`.
- **Lighthouse, measured here** with `lighthouse@13.5.0` on Playwright's
  Chromium 141: every page scored 100, in all three runs, mobile and
  desktop. With the CPU slowed 16 times, `/posts/1` still scored 99 to
  100. Median mobile LCP 1.05 to 1.8 s, TBT at most 37 ms, CLS at most
  0.001. About 11 s a run.
- **`--release` for WASM** runs only `wasm-opt -Oz`, when `wasm-opt` is on
  `PATH` (`bundle.rs`); without it the output is a debug build's. With
  binaryen's real `wasm-opt` it fails: "memory.fill operations require
  bulk memory". 4.4 tested a stand-in. With `--enable-bulk-memory`, 06's
  client drops to 11,974 bytes and 07's to 27,510; both still work.
- **Neither `Dist` nor the tests' static servers compress or set caching
  headers.** Lighthouse 13 scores neither, and the pages score 100
  without them.
- **The release:** 20 crates are published, in `publish-crates.sh`'s
  order; `nova-codegen-wasm` joined them in 4.0 through `nova-driver`.
  `nova-bundler` is a one-line stub, `publish = false`, that nothing
  depends on. Both names are free on crates.io. The version lives in the
  root `Cargo.toml` (its version and 20 pins), the extension's
  `package.json` and `package-lock.json`, and `Cargo.lock`.
- **The documentation:** the READMEs say nothing of WASM, `nova dev`,
  `--ssr` or `--ssg`; `40-TOOLING.md`'s 2026-10-14 note still says `build
  --target wasm` refuses `--release`; `docs/releasing.md` has stale lines
  (an `old=0.2.0`, 20 changed lines where there are 21, timings for 19 new
  crates); std's documentation site leaves out `std/dom`, by design, since
  it documents native std.

## 3. The Lighthouse gate

- **`tests/browser/lighthouse.mjs`,** run as `node
  tests/browser/lighthouse.mjs target/debug/nova <work dir> <npm dir>`,
  after `blog.mjs`:
  1. builds 06 with `nova build --target wasm` and serves its `dist/` from
     a small static server on `127.0.0.1:0`, as `counter.mjs` does;
  2. builds 07 with `nova build --ssr`, sets the admin's password with a
     low `--iterations`, and starts its server on port 0, as `blog.mjs`
     does;
  3. runs Lighthouse's CLI three times on each of the four pages: mobile,
     the default, with `--only-categories=performance`, `--output=json`,
     and `--chrome-flags="--headless=new --no-sandbox"`, `CHROME_PATH` set
     to Playwright's Chromium;
  4. prints a line per page with its three scores and their median, and
     fails when a median is below 95, or a run fails;
  5. prints one summary line beginning `lighthouse:`, and stops both
     servers whatever happens.
- **CI** installs `lighthouse@13.5.0` beside `playwright@1.56.1`, in the
  same `npm install --no-save` prefix, and runs the script in a new step
  after the blog's. Lighthouse 13 needs Node 22.19 or later; the job's
  Node 22 must be at least that.
- **Debug builds are measured,** the builds the size tests measure and the
  other browser steps drive.
- **The size checks stand as they are.** 07's client size stays printed
  by `blog.mjs`, with no limit.

## 4. `--release` and binaryen

- `wasm_opt` in `crates/nova-cli/src/bundle.rs` runs `wasm-opt -Oz
  --enable-bulk-memory app.wasm -o app.wasm`. The prototype checks whether
  the backend needs any other feature enabled, and the flag list is what
  the backend emits.
- The stand-in `wasm-opt` test in `wasm_tests.rs` expects the new
  arguments.
- **CI's `browser` job tests it with binaryen itself:** it installs
  `binaryen@132.0.0` from npm into the same prefix, puts its `wasm-opt` on
  `PATH` for one step, builds 06 and 07 with `--release`, checks that
  `wasm-opt` ran and the clients shrank, and drives both through
  `spa.mjs` and `blog.mjs` on those builds, printing their sizes. The
  scripts gain whatever option they need to build with `--release`.

## 5. Documentation

- **A "Web" section** in the root `README.md` and in `nova-cli`'s
  `README.md` (the one crates.io shows): `nova build --target wasm`,
  `nova dev`, `nova build --ssr` and `--ssg`, `std/ui` and its modules,
  `std/dom` (which std's site does not list, so the section points at its
  source), and the two examples. `nova-cli`'s `description` and keywords
  in `Cargo.toml` mention the web.
- **Stale lines corrected:** `40-TOOLING.md`'s note on `--release` (a
  dated note that 4.4 and now 4.8 changed it), `docs/releasing.md`'s three,
  and any line in `agent.md` that names a Phase 4 command wrongly.

## 6. ADR 0047, Phase 4's boundary

In ADR 0037's form:
- **Context:** the master spec's §7, its Phase 4 list and gate, the phase
  plan's sub-phases, and ADRs 0038 to 0046.
- **The inventory:** each master-spec Phase 4 item and each phase-plan
  sub-phase, shipped, shipped differently, or left out, with its record.
- **The deviations it decides:** the bundler in `nova-cli` (`nova-bundler`
  a stub); `std/dom` off std's site; the blog's JSON store; and any other
  the inventory finds that no earlier ADR decides.
- **The gate's record:** the CI run of the 4.8 pull request, with its
  Lighthouse lines and sizes; the user's own check, added by the release
  pull request; the publishes' runs, added by the records pull request.
- **The backlog:** everything ADRs 0038 to 0046 left out or deferred
  (compression, a development mode for SSR, nested modules, async
  handlers, `std/dom` on the site, and the rest), gathered in one list.
- **Consequences:** "Phase 4 is complete; `v0.4.0` is its release."

## 7. The release `v0.4.0`

By `docs/releasing.md`, in three pull requests:
1. **4.8's** (§3 to §6), merged on the user's word.
2. **The user's check:** both examples from `main`, by their READMEs;
   their word that both work.
3. **The release pull request:** the version 0.4.0 in the root
   `Cargo.toml` (its version and pins), `Cargo.lock`'s own packages, the
   extension's `package.json` and `package-lock.json`; CHANGELOG's
   Unreleased section made `0.4.0` with its date and a lead paragraph; the
   extension's CHANGELOG entry; ADR 0047 gains the user's check. **This
   changes `Cargo.lock`:** only the workspace's own packages' versions, as
   every release has; approving this spec approves that change, and
   nothing else in the lock.
4. **On the user's word, each:** the merge; the tag `v0.4.0`, which makes
   the GitHub release; the 20 crates, in the script's order; the extension
   on the Marketplace, then on Open VSX; the runbook's gates.
5. **The records pull request:** ADR 0047 and the CHANGELOG gain the
   publishes' run IDs and the published `nova`'s check.

## 8. Testing

- Tests never touch the internet; Lighthouse and binaryen run in CI only,
  installed by the job.
- `wasm_tests.rs`'s stand-in `wasm-opt` test checks the new arguments; the
  full suite, fmt, clippy, `nova fmt`, MSRV and the lockfile as in every
  sub-phase.
- `lighthouse.mjs` runs locally in the prototype with Lighthouse installed
  in scratch, and in CI.
- The release builds with binaryen run locally in the prototype with
  binaryen installed in scratch, and in CI.
- `gate.sh`'s WASM build runs in CI's `install` and `package` jobs, as the
  script does today.

## 9. Records

- ADR 0047.
- Dated notes: `00-MASTER-SPEC.md` (Phase 4 complete), `40-TOOLING.md`
  (`--release` with `wasm-opt`), `60-EXAMPLES.md` §6 and §7 (Lighthouse),
  the phase plan's 4.8 amendment.
- CHANGELOG's Unreleased: Added (the Lighthouse gate), Fixed (`--release`
  with a real `wasm-opt`), Changed (the documentation, `gate.sh`).

## 10. Risks

1. **Lighthouse on a slower runner,** or a noisy one: the medians keep a
   wide margin here (100, and 99 with the CPU slowed 16 times); a failed
   run fails the step rather than being retried.
2. **Installing Lighthouse and binaryen** adds time to a job whose
   Playwright install already varies from 21 s to six minutes.
3. **`--no-sandbox`** is needed on the runner, as Playwright itself uses.
4. **The release changes `Cargo.lock`'s versions,** and a publish cannot
   be undone; the runbook's order, and the user's word at each step, hold.

## 11. Not in 4.8

- Compression and caching headers in `Dist`; the desktop preset; devtools
  throttling; a size limit for 07; Lighthouse in `cargo test`.
- `std/dom` on std's site; moving the bundler into `nova-bundler`.
- Phase 5's work.

## 12. Decisions made while writing this spec

1. **Debug builds are measured,** as the size tests and the browser steps
   use.
2. **Compression stays out,** since the pages score 100 without it (ADR
   0046 deferred it).
3. **`std/dom` is documented by a pointer in the READMEs,** and its place
   on std's site goes to the backlog.
4. **binaryen comes from npm,** `binaryen@132.0.0`, pinned, in the job's
   npm prefix.
5. **CI drives 06 and 07 on their `--release` builds too,** so the fix is
   tested end to end.
6. **CHANGELOG's version section is made in the release pull request,**
   and the user's check is added to ADR 0047 there.
7. **v0.4.0 publishes the same 20 crates** as v0.3.2;
   `nova-codegen-wasm` already among them.
