# ADR 0047 — Phase 4's boundary

## Status

Accepted (2026-10-22). Branch `claude/loving-archimedes-frtf5q-48`
(`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
§6). The gate's measurement, the order of the release and `nova-bundler`'s
place are the user's decisions of 2026-10-22, listed below.

## Context

`nova-spec/00-MASTER-SPEC.md` §7 calls a phase done when:
1. its crates compile and pass CI;
2. its gate criteria are met with reproducible commands;
3. its new public surface is documented;
4. `CHANGELOG.md` is updated;
5. an ADR records each decision that deviates from the spec;
6. a `v0.{phase}.0` milestone is tagged.

Phase 4, "Frontend / WASM", makes its promises in three places:
- **the master spec's §3:** the Phase 4 list of nine numbered entries,
  cited below as "position N", and the Phase 4 gate:
  "`examples/06-counter-spa` and `examples/07-fullstack-blog` work in
  browser, pass Lighthouse 95+";
- **`docs/phase-4-plan.md`:** its §3 decisions, cited as "decision N", and
  sub-phases 4.0 to 4.8, each with its own gate;
- **`nova-spec/60-EXAMPLES.md` §6 and §7,** which give each example's
  gate, and **`nova-spec/30-FRONTEND.md`,** which describes more than the
  gate needs; its rows below are cited as "30-FRONTEND §N".

Earlier ADRs already narrowed Phase 4:
- 0038: the scope, what the gate's two examples need, and the list of
  what Phase 4 leaves out;
- 0039 and 0040: the WASM backend, its runtime in Nova, and its collector
  and executor;
- 0041: `std/dom` and the boundary with JS;
- 0042 and 0043: `std/ui`, and the component syntax with the bundle's last
  pieces;
- 0044: the router and `nova dev`;
- 0045: SSR, hydration and SSG;
- 0046: the blog, and what std gained for it.

Phase 4 was built in eight pull requests, #117 to #124, one for each of
4.0 to 4.7, merged from 2026-10-15 to 2026-10-22. No release carries it
yet: `v0.3.2`, released on 2026-10-14, is the last.

The user decided on 2026-10-22, and approved with the 4.8 spec:
1. **Lighthouse measures the mobile preset,** with its default simulated
   throttling, three runs a page; a page passes when the median of its
   three Performance scores is 95 or more.
2. **It measures the public pages,** 06's `/` and 07's `/`, `/posts/1` and
   `/login`, in a new step of CI's `browser` job,
   `tests/browser/lighthouse.mjs`, on the Chromium the job installs for
   Playwright, with `lighthouse@13.5.0` pinned.
3. **`nova build --release` is fixed for a real `wasm-opt`,** and CI tests
   it with binaryen itself.
4. **07 gets no size limit;** its size stays printed. A KB is 1,024 bytes,
   as the size tests measure, and the records say KiB.
5. **Phase 4 is complete before the tag,** as Phase 2 was (ADR 0025): the
   4.8 pull request, with the gate, the fix, the documentation and this
   ADR; then the release pull request, with the version; then a records
   pull request, with the publishes' run IDs. This gate needs nothing
   published.
6. **The user opens both examples from `main`,** by their READMEs, after
   the 4.8 pull request merges and before the tag; after the release, the
   published `nova` is tried again in place of the runbook's step 6.2.
7. **`nova-bundler` stays a stub** (below).
8. **The release also** runs `nova build --target wasm` in `gate.sh`; gives
   the root README and `nova-cli`'s README a "Web" section; and publishes
   the VS Code extension at 0.4.0 to both stores.

## Decision

### The inventory

"Position N" is the master spec's Phase 4 list. "4.N" is a sub-phase of
`docs/phase-4-plan.md`, and "decision N" one of its §3 decisions.
"60-EXAMPLES §N" and "30-FRONTEND §N" are sections of those files in
`nova-spec/`.

| Promise | Source | Status | Record |
|---|---|---|---|
| A WASM backend, on `wasm-encoder` and `walrus` | position 1; decisions 1 to 3 | shipped differently: on `wasm-encoder` 0.243.0 alone, with its runtime written in Nova (`std/rt`) and a collector between turns | ADRs 0038, 0039, 0040 |
| A bundler, `crates/nova-bundler`: bundle, tree-shake, code-split | position 2; decision 8 | shipped differently: the bundle is `nova-cli`'s, tree shaking is monomorphization from `main`, there is no code splitting, and `nova-bundler` is a stub | ADR 0043; decided here |
| `std/dom`, bindings generated from `web-sys` descriptors | position 3 | shipped differently: a written subset over `extern "js"`, imported with `import dom` | ADRs 0038, 0041 |
| `std/dom` on std's documentation site | the master spec's §7, item 3 | left out: the site documents native std; the READMEs point at its source | decided here |
| `std/ui`: signals, effects, memos, components | position 4; decision 7 | shipped; a component is written with `component` and `view { … }` (RFC 0001) | ADRs 0042, 0043 |
| `std/ui/html`, element builders | position 5 | shipped, imported with `import ui::html` | ADR 0042 |
| `std/ui/router`, a client router | position 6 | shipped: `Router` by the hash, `HistoryRouter` by the history API, `link` | ADR 0044 |
| `nova dev`, a dev server with HMR | position 7; decision 9 | shipped differently: a full build in a child process, and a reload over Server-Sent Events, with an overlay for the build's errors | ADRs 0038, 0044 |
| SSR: render to a string natively, hydrate in WASM | position 8; decision 10 | shipped differently: `nova build --ssr` builds the project's own server, which renders with `render_hydratable`; `hydrate` adopts the server's nodes, and builds the view again on a mismatch | ADR 0045 |
| SSG: compile-time route enumeration | position 9 | shipped differently: `nova build --ssg` runs the built server once, which writes a page for each route with no parameter | ADR 0045 |
| The gate: both examples "work in browser" | the master spec's Phase 4 gate | met in Chromium, in CI's `browser` job, and by the user's own check, from `main`; both recorded | "The gate's record" |
| The gate: "pass Lighthouse 95+" | the master spec's Phase 4 gate; 60-EXAMPLES §7; 30-FRONTEND §11; decision 12 | met where 4.8 was built and in the 4.8 pull request's CI run, every median 100; both recorded | decided here; "The gate's record" |
| 06: `nova dev` opens a browser, and the counter responds to clicks | 60-EXAMPLES §6 | met: `nova dev --open` opens one, and CI's `browser` job clicks the counter under `nova dev` | ADR 0044 |
| 06: under 50 KiB gzipped | 60-EXAMPLES §6; 30-FRONTEND §11; decision 12 | met: 21,070 bytes by `flate2` at level 9 where 4.8 was built, by a test that runs on all three systems | ADR 0043; "The gate's record" |
| Hello world under 30 KiB gzipped | 30-FRONTEND §11; decision 12 | met: 5,006 bytes by `flate2` at level 9 where 4.8 was built, by a test that runs on all three systems | ADR 0039; "The gate's record" |
| 07: pages that work, a login, the admin's create, edit and delete, sessions in cookies, CSRF tokens, no XSS | 60-EXAMPLES §7; decision 11 | shipped | ADR 0046 |
| 07: SQLite, through `std/db` | 60-EXAMPLES §7 | replaced: JSON files, each written whole, then renamed into place | ADRs 0038, 0046 |
| 07: hydration for the comments | 60-EXAMPLES §7 | shipped differently: every page hydrates whole; no islands | ADR 0046 |
| 07: `routes/` and `components/` | 60-EXAMPLES §7 | shipped differently: a flat `src/` | ADR 0046 |
| A CI job: Lighthouse, the gzipped sizes, Playwright | decision 12 | shipped: Playwright and Lighthouse in the `browser` job, the sizes in the `test` job on all three systems | "The gate's record" |
| The user opens both examples in their own browser | decision 12 | done, from `main`, after the 4.8 pull request merged: both work, as the user reported; recorded | "The gate's record" |
| std per target, and std modules a program imports | decisions 5 and 6 | shipped: `dom`, `ui`, `ui::html`, `ui::router` and, natively, `ui::ssr` | ADRs 0040, 0041, 0042, 0044, 0045 |
| Async on the browser's event loop, with no `block_on` on WASM | decision 4 | shipped differently: `block_on` works, and holds the page while it waits | ADR 0040 |
| `wasm-opt` under `--release`, never required | decision 2 | shipped in 4.4; fixed here for binaryen's own `wasm-opt` | ADR 0043; "Found outside this boundary" |
| 4.0: the WASM target and the first programs under Node | 4.0 | shipped; its gate, as amended, met on all three systems | ADR 0039 |
| 4.1: the language on WASM | 4.1 | shipped; its gate met: 94 of `tests/runtime/`'s 134 programs, and `examples/03-producer-consumer` | ADR 0040 |
| 4.2: `std/dom` and the shim | 4.2 | shipped; its gate met under the fake DOM and in Chromium | ADR 0041 |
| 4.3: `std/ui`, reactivity and element builders | 4.3 | shipped; its gate met under the fake DOM, in Chromium, and on both targets | ADR 0042 |
| 4.4: component syntax and `06-counter-spa` | 4.4 | shipped; its gate met | ADR 0043 |
| 4.5: the router and `nova dev` | 4.5 | shipped; its gate met in Chromium | ADR 0044 |
| 4.6: SSR, hydration and SSG | 4.6 | shipped; its gate met in Chromium | ADR 0045 |
| 4.7: `examples/07-fullstack-blog` | 4.7 | shipped; its gate met in Chromium, and over HTTP on all three systems | ADR 0046 |
| 4.8: the CI job's Lighthouse run, and the size checks | 4.8 | shipped | this ADR |
| 4.8: the user opens both examples | 4.8 | done: both work, as the user reported; recorded | "The gate's record" |
| 4.8: the records | 4.8 | done: the READMEs' "Web" sections, the dated notes, `CHANGELOG.md` and this ADR | this ADR |
| 4.8: `v0.4.0`, by the runbook | 4.8 | done: the release pull request, #126, then the tag, the GitHub release, crates.io, both stores and the Release gate, each on the user's word, and the user's check of the published `nova`; recorded | the user's decision 5; "The gate's record" |
| 4.8: `nova-codegen-wasm` and `nova-bundler` published if `nova-cli` depends on them | 4.8 | decided: `nova-codegen-wasm` is in the publish list since 4.0, and first reaches crates.io with `v0.4.0`; nothing depends on `nova-bundler`, which stays unpublished | ADR 0039; decided here |
| Examples 08 to 10 | 30-FRONTEND §12 | left out | ADR 0038; backlog |
| Code splitting with `lazy()` | position 2; 30-FRONTEND §7.3 | left out | ADR 0038; backlog |
| JS interop for programs: npm resolution, `@js_import`, `@js_export` | 30-FRONTEND §9 | left out: only std declares JS functions | ADRs 0038, 0041; backlog |
| Source maps | 30-FRONTEND §8 | left out | ADRs 0038, 0044; backlog |
| `axum` for the dev server | 30-FRONTEND §8 | left out: `nova dev` is written on `std::net` | ADRs 0038, 0044 |
| Reference counting on WASM, or the WASM GC proposal | `nova-spec/13-RUNTIME.md` §8 | left out: mark-sweep between turns | ADRs 0038, 0040 |
| `wasm-bindgen-test`, and visual regression snapshots | 30-FRONTEND §13 | left out: a fake DOM in `cargo test`, and Chromium in CI | ADRs 0038, 0041; backlog |
| The LSP rows marked Phase 4: inlay hints, code lens, call hierarchy | `nova-spec/40-TOOLING.md` §3.1 | left out | ADR 0038; backlog |
| Todo MVC under 100 KB, and the counter's time to interactive under 100 ms | 30-FRONTEND §11 | left out: there is no Todo MVC, and no measure of time to interactive | ADR 0038 (Todo MVC); backlog |

**Status words:**
- *Shipped:* a `v0.4.0` user can rely on it, within the limits its record
  states; *shipped differently* when its record changed how.
- *Replaced:* something else does its job, as its record says.
- *Left out:* nothing implements it.
- *Met:* a gate's checks passed, as the row and its record state them.
  *Done* says the same of a step that is not a feature.
- *To come:* a step of the release, which the release pull request or the
  records pull request adds to this ADR.
- *Backlog:* where a deferred item goes.
- *Decided here:* a section below makes the decision.

### Deviations no earlier record decides

#### The bundler: decided, `nova-cli`'s; `nova-bundler` stays a stub

Position 2 names a crate, `crates/nova-bundler`, that bundles,
tree-shakes and splits code. The phase plan's §2 found it a one-line stub,
`publish = false`, beside `nova-codegen-wasm`, and its decision 8 set the
bundle's output. The 4.0 spec kept the crate "unpublished and empty until
4.3"; 4.3 moved the bundler's pieces to 4.4, and ADR 0043 built them in
`nova-cli`, in `crates/nova-cli/src/bundle.rs`: the project's page,
`assets/` from `public/`, `manifest.json` and `wasm-opt`. No record since
rules on the crate. Of position 2's three parts:
- **the bundle** is `nova build --target wasm`'s, in `nova-cli`;
- **tree shaking** is monomorphization from `main`: MIR holds only the
  functions a program reaches (the plan's §2 item 2), and `std/rt`'s too
  (ADR 0039, decision 2);
- **code splitting** with `lazy()` is left out (ADR 0038).

The phase plan's 4.8 entry publishes `nova-bundler` if `nova-cli` depends
on it, and nothing does.

**Decision (the user, 2026-10-22, decision 7 above):** position 2 is
shipped differently, in `nova-cli`, as Phase 3's test runner is
`nova-cli`'s while `nova-test` stays a stub (ADR 0037). `nova-bundler`
stays a one-line stub, `publish = false`, and is not published; `v0.4.0`
publishes the same 20 crates `publish-crates.sh` lists. Moving the bundle
into the crate goes to the backlog.

#### `std/dom` on std's site: decided, a pointer to its source

The master spec's §7 asks for documentation of the new public surface.
std's documentation site, which `nova doc --std` writes and each release
attaches, documents the modules `std/lib.rs` lists in `ALL`, among them
`ui`, `ui/html`, `ui/router` and `ui/ssr`. `std/dom` is not in `ALL`: only
a WASM build compiles it, and the site is native std's (4.2's spec §3.2,
as `std/lib.rs` says). Its 67 public items are documented in
`std/dom/lib.nova` alone.

**Decision (the 4.8 spec's §12, decision 3):** the "Web" sections of the
root README and of `nova-cli`'s README name `std/dom`, say it is not on
the site, and point at `std/dom/lib.nova`. `std/ui` and its modules, which
the site documents, are the surface a program is meant to use. `std/dom`
on the site goes to the backlog.

#### "Lighthouse 95+": decided, a median of 95 or more over three mobile runs

The master spec's gate reads "pass Lighthouse 95+". `60-EXAMPLES.md` §7
asks 07 for "Lighthouse Performance > 95", and `30-FRONTEND.md` §11 for a
"Lighthouse Performance score: > 95"; the phase plan's decision 12 fails
the job "below 95 for Performance". The specs name no page, preset, number
of runs or build, and two of them say more than 95 where the gate and the
plan say 95 or more.

**Decision (made here, for the user's review with this ADR, on the user's
decisions 1 and 2 above):** the gate's "Lighthouse 95+" is
`tests/browser/lighthouse.mjs` in CI's `browser` job:
- Lighthouse 13.5.0, its mobile preset with the default simulated
  throttling, the Performance category alone;
- three runs on each public page, 06's `/` and 07's `/`, `/posts/1` and
  `/login`; `/admin` needs the admin's session, and is left out;
- debug builds, the ones the size tests measure and the other browser
  steps drive (the 4.8 spec's §12, decision 1);
- a page passes when the median of its three scores is 95 or more, and the
  step fails when a run fails.

The two specs' "> 95" is read as the gate's 95 or more. Every median
measured 100 (below), which meets either reading.

### Rows whose record already explains them

These rows have no section above:
- **The blog's JSON store:** the user decided on a JSON file in place of
  SQLite on 2026-10-14 (ADR 0038), and ADR 0046 built it, with the users in
  a file of their own; `60-EXAMPLES.md` §7's note of 2026-10-21 says so.
- **`walrus` and `axum`:** ADR 0038 left both out, ADR 0039 builds the
  backend on `wasm-encoder` alone, and ADR 0044 writes `nova dev` on
  `std::net`. No crate depends on either.
- **`std/dom`'s written subset:** ADR 0038 left generated bindings out,
  and ADR 0041 reaches JS through `extern "js"` declarations only std may
  write.
- **`nova dev`'s reload:** the user chose live reload over HMR on
  2026-10-14 (ADR 0038), and ADR 0044 records what departs from
  `30-FRONTEND.md` §8.
- **`block_on` on WASM:** the user's decision of 2026-10-15 (ADR 0040).
- **SSR and SSG:** ADR 0045 records what departs from `30-FRONTEND.md`
  §7.4 and the master spec's positions 8 and 9.
- **07's flat `src/` and whole-page hydration:** the user's decisions of
  2026-10-21 (ADR 0046).
- **06's page:** its `index.html` loads `./app.js` and calls `init()`, and
  its source uses `import ui`, `value()` and `mount("#app", Counter)`, as
  `60-EXAMPLES.md` §6's note of 2026-10-17 records (ADR 0043).
- **`nova-codegen-wasm` on crates.io:** ADR 0039 publishes it from the next
  release; the 4.0 spec's §8 put it into the publish list, the 20th crate.
- **The size checks:** hello world's limit of 30 KB and 06's of 50 KB are
  KiB, 30,720 and 51,200 bytes, as their tests have measured them since
  4.0 and 4.4, and 07 has no limit; both are the user's decision 4 above.

### The gate's record

**What runs.** CI's `browser` job, on `ubuntu-latest`, in this order:
- **in Chromium, through Playwright** (`playwright@1.56.1`): `counter.mjs`
  clicks 4.2's and 4.3's counters; `spa.mjs` clicks 06's; `dev.mjs` runs
  06 under `nova dev`, edits it and shows the overlay; `router.mjs` and
  `ssr.mjs` drive 4.5's and 4.6's test apps; and `blog.mjs` drives 07:
  every page hydrates in place, the forms, the admin's create, edit and
  delete, scripts that stay text, and a form without its token refused;
- **the release builds:** with binaryen's `wasm-opt` 132 on `PATH` for
  this step alone, 06 and 07 are built with `--release`; each `app.wasm`
  must be smaller than the debug build's, and `spa.mjs` and `blog.mjs`
  drive both builds;
- **the rules' test:** `lighthouse.test.mjs` tests the rules
  `lighthouse.mjs` applies, from `lighthouse-rules.mjs`, under
  `node --test`, without Lighthouse: the four pages, three runs each, the
  median of three, and a page failed by a median under 95, a failed run, a
  run with no score, or other than three runs; and `lighthouse-run.mjs`'s
  `run`, which builds the examples, given a child that exits before it
  reads its stdin;
- **Lighthouse** (`lighthouse.mjs`), as decided above, once the rules'
  test has passed.

The rules' test and Lighthouse run after a failure of the release builds
too, though not after an earlier step's, so that neither the release
builds' result nor Lighthouse's hides the other.

The `test` job runs the size tests on Linux, macOS and Windows:
`hello_world_is_under_30_kb_gzipped` and
`the_counter_spa_is_under_50_kb_gzipped` in
`crates/nova-cli/tests/wasm_tests.rs`, which gzip `app.wasm` and `app.js`
each at level 9 and add the two.

**Measured where 4.8 was built,** on 2026-10-22: the `browser` job's steps
as the job then had them, run in order on a development container, Ubuntu
24.04 on x86_64, with Node 22.22.0 and Playwright's Chromium 141, every
step passing. The job then ran Lighthouse before the release builds, and
had no step for the rules' test yet, so the Lighthouse lines come before
the release builds' here. Lines are shortened where marked `…`, the two
`blog:` lines and the scratch paths of the last two:

```text
counter: three clicks, and the count reads 3
counter: three clicks, and the count reads 3
counter-spa: +, +, +, -, and the count reads 2
dev: counts at build 1792684328264.1, reloads into .2 on an edit, into .3 after a missed one, shows the overlay for the type error of .4 and keeps it over a reload, and counts again at .5
router: a link to /posts/2 without a reload, back home, a reload at /posts/2 that stays at build 1792684333287.1, /nowhere not found, and a query read
ssr: / hydrates in place (36 nodes kept + 1 joined) and counts, a link to /about without a reload, /posts/2 in place (26 + 1), /nowhere 404 in place (28 + 1), and SSG's /about/ arrives rendered and hydrates in place (28 + 1); the client is 41688 bytes gzipped
blog: every page hydrates in place (nodes kept: / 37, /posts/1 48, /login 24, /nowhere 15, /posts/99 15, /admin 53, /admin/new 29, /admin/posts/2/edit 30), … the client is 49472 bytes gzipped
06 /: scores 100, 100, 100, median 100; medians FCP 679 ms, LCP 1054 ms, TBT 0 ms, CLS 0.000, SI 679 ms; benchmark index 1796, 2397, 2228
07 /: scores 100, 100, 100, median 100; medians FCP 903 ms, LCP 1803 ms, TBT 0 ms, CLS 0.000, SI 903 ms; benchmark index 2401, 2515, 2463
07 /posts/1: scores 100, 100, 100, median 100; medians FCP 901 ms, LCP 1652 ms, TBT 6 ms, CLS 0.000, SI 901 ms; benchmark index 2122, 2486, 2258
07 /login: scores 100, 100, 100, median 100; medians FCP 901 ms, LCP 1651 ms, TBT 0 ms, CLS 0.001, SI 901 ms; benchmark index 2342, 2062, 2325
lighthouse: 06's / and 07's /, /posts/1 and /login, 3 runs each, mobile, every median 95 or more (06 / 100, 07 / 100, 07 /posts/1 100, 07 /login 100); Lighthouse 13.5.0, HeadlessChrome/141.0.0.0, Node 22.22.0
wasm-opt version 132 (version_132)
counter-spa: +, +, +, -, and the count reads 2; the client is 11974 bytes gzipped, from a release build that wasm-opt optimized
blog: every page hydrates in place (nodes kept: / 37, /posts/1 48, /login 24, /nowhere 15, /posts/99 15, /admin 53, /admin/new 29, /admin/posts/2/edit 30), … the client is 27510 bytes gzipped, from a release build that wasm-opt optimized
…/counter-spa-release/app.wasm: 16525 bytes, from 40491 in the debug build
…/blog-release/fullstack-blog/dist/app.wasm: 57179 bytes, from 116579 in the debug build
```

**Measured again** while the plan was checked, on the same container: of
the Lighthouse step's 94 runs that gave a score, 93 scored 100 and one, a
run of 07's `/`, scored 93; every page's median was 100, as above. Runs
vary; the median of three takes one low run in its stride.

**The sizes,** `app.wasm` and `app.js` gzipped at level 9 by zlib, as
`blog.mjs`, and `spa.mjs` under `--release`, measure them, and added, in
bytes, with `app.wasm`'s raw size after it; "release" is `--release` with
binaryen 132's `wasm-opt` on `PATH`. The size tests compress with
`flate2`, whose output is a little larger: there, hello world's debug
build is 5,006 bytes and 06's 21,070.

| | debug | release | limit |
|---|---|---|---|
| hello world | 4,959 (4,120) | 3,658 (1,332) | 30 KiB, 30,720 |
| 06's client | 20,997 (40,491) | 11,974 (16,525) | 50 KiB, 51,200 |
| 07's client | 49,472 (116,579) | 27,510 (57,179) | none; printed |

**The 4.8 pull request's CI run,**
https://github.com/Sakeerin/nova/actions/runs/39891767287, on the pull
request's head `09548e1`, on 2026-10-23: all 24 checks passed, with
`release.yml`'s run 39891767288, whose "Publish the release" is skipped
on a pull request. All nine of its runs of `gate.sh` printed `gate: the
WASM build prints Hello, Nova! under Node` and `gate: passed`: the
`install` and `package` jobs' on Linux, macOS and Windows, and the
release's smoke test on three of its four targets, `x86_64-apple-darwin`'s
being skipped by design. The `browser` job (job 120212941991) passed
every step, the rules' test with `# tests 9`, `# pass 9`, `# fail 0`.
Its release builds' step and its Lighthouse step printed these lines,
shortened where marked `…`, the release build's `blog:` line and the
runner's paths:

```text
wasm-opt version 132 (version_132)
built …/counter-spa-release
counter-spa: +, +, +, -, and the count reads 2; the client is 11974 bytes gzipped, from a release build that wasm-opt optimized
blog: every page hydrates in place (nodes kept: / 37, /posts/1 48, /login 24, /nowhere 15, /posts/99 15, /admin 53, /admin/new 29, /admin/posts/2/edit 30), … the client is 27510 bytes gzipped, from a release build that wasm-opt optimized
…/counter-spa-release/app.wasm: 16525 bytes, from 40491 in the debug build
…/blog-release/fullstack-blog/dist/app.wasm: 57179 bytes, from 116579 in the debug build
06 /: scores 100, 100, 100, median 100; medians FCP 679 ms, LCP 1054 ms, TBT 0 ms, CLS 0.000, SI 679 ms; benchmark index 2438, 2457, 2434
07 /: scores 100, 100, 100, median 100; medians FCP 902 ms, LCP 1652 ms, TBT 0 ms, CLS 0.000, SI 902 ms; benchmark index 2456, 2390, 2440
07 /posts/1: scores 100, 100, 100, median 100; medians FCP 901 ms, LCP 1652 ms, TBT 0 ms, CLS 0.000, SI 901 ms; benchmark index 2395, 2435, 2445
07 /login: scores 100, 100, 100, median 100; medians FCP 902 ms, LCP 1052 ms, TBT 0 ms, CLS 0.001, SI 902 ms; benchmark index 2445, 2450, 2458
lighthouse: 06's / and 07's /, /posts/1 and /login, 3 runs each, mobile, every median 95 or more (06 / 100, 07 / 100, 07 /posts/1 100, 07 /login 100); Lighthouse 13.5.0, HeadlessChrome/141.0.0.0, Node 22.23.3
```

Every median is 100, as where 4.8 was built, and the release builds'
sizes are the table's above, to the byte.

**The user's check,** on 2026-10-23, after the 4.8 pull request merged:
the user installed `nova` from `main` (`1afbaf8`) with `cargo install
--locked --path crates/nova-cli`, ran both examples by their READMEs on
their own machine, and opened each in their own browser. They reported
both working, in their words "06 และ 07 เสร็จแล้วครับ" ("06 and 07 are
done"), with two screenshots:
- 06 at `http://127.0.0.1:3000/`, under `nova dev`: the page titled
  "Counter", reading `Count: 2`, with its `+` and `-` buttons;
- 07's Admin page, logged in, its header reading "Admin" and "Log out
  admin": "Write a new post", and the three seeded posts, "Sessions,
  tokens and passwords" (2026-10-21), "How the forms work" (2026-10-20)
  and "Hello from a Nova server" (2026-10-19), each with Edit and Delete.

Their system was not stated.

**The publishes,** from the tag `v0.4.0`, on `eab7da8`, which the user
made and pushed on 2026-10-24, each on the user's word:
- **The Release workflow,**
  https://github.com/Sakeerin/nova/actions/runs/40063515984, passed on
  2026-10-24. Its GitHub release, "Nova 0.4.0", is neither a draft nor a
  prerelease, and its notes are `CHANGELOG.md`'s `[0.4.0]`. It has eight
  assets and `SHA256SUMS`, each named for `0.4.0`: `install.ps1`,
  `install.sh`, the archives for `aarch64-apple-darwin`,
  `x86_64-apple-darwin`, `x86_64-pc-windows-msvc` and
  `x86_64-unknown-linux-gnu`, `nova-std-docs-0.4.0.tar.gz` and
  `nova-vscode-0.4.0.vsix`. All eight were downloaded and checked:
  `sha256sum --check SHA256SUMS` printed `OK` for each, and each sum
  equals the digest GitHub's API gives the asset. `releases/latest`
  redirects to `…/releases/tag/v0.4.0`.
- **"Publish to crates.io",**
  https://github.com/Sakeerin/nova/actions/runs/40074121811, from the
  tag, passed on 2026-10-24. Its package check printed `package-gate: 20
  packages, 1693099 bytes`, `gate: the WASM build prints Hello, Nova!
  under Node` and `gate: passed`. Its owner check printed
  `nova-codegen-wasm: not on crates.io`, and for each of the 19 others
  `on crates.io, owned by` the repository's owner. No line said `rate
  limited`: each crate was published at its first attempt, and the
  summary lists all 20 `published`, none skipped. The index's last line
  for each of the 20 names `"vers":"0.4.0"`; `nova-codegen-wasm` is on
  crates.io for the first time.
- **The extension,** 0.4.0, in both stores on 2026-10-24: the release's
  `nova-vscode-0.4.0.vsix`, its line of `SHA256SUMS` checked `OK`, went
  to the Marketplace and to Open VSX, where `ovsx` 1.3.0 printed
  `Published sakeerin.vscode-nova v0.4.0`. The user's screenshots of the
  two listings:
  - **Open VSX's:** Nova Language Tools by `sakeerin`, version 0.4.0,
    with the version alias `latest`, and the README;
  - **the Marketplace's:** version 0.4.0, last updated 10/24/2026, the
    unique identifier `sakeerin.vscode-nova`, the README, and the links
    Issues, Repository, Homepage, License and Changelog.
- **"Release gate",**
  https://github.com/Sakeerin/nova/actions/runs/40107204392, version
  `0.4.0`, on `main` at `eab7da8`: all six jobs passed, "From crates.io"
  and "From the install script", each on Ubuntu, macOS and Windows, each
  running `gate.sh --test --node`. On Ubuntu, "From crates.io" printed
  `Installed package nova-cli v0.4.0`, `nova 0.4.0`, `gate: the WASM
  build prints Hello, Nova! under Node`, `gate: passed`, `packages-gate:
  passed` and `doc-gate: passed`; "From the install script" printed
  `nova: checked nova-0.4.0-x86_64-unknown-linux-gnu.tar.gz against
  SHA256SUMS`, the same Node line and `gate: passed`. Neither log has a
  skip line.
- **The user's check of the published `nova`,** on 2026-10-25, in place
  of the runbook's step 6.2 (its note of 2026-10-22), on Windows, in Git
  Bash: `cargo install nova-cli --locked` built `nova-cli` 0.4.0 from
  crates.io in 3m 54s, replacing their 0.3.2 build from the checkout, and
  `nova --version` printed `nova 0.4.0`. In 06, `nova dev` printed
  `rebuilt in 186 ms` and `nova dev: serving http://127.0.0.1:3000/`,
  and the browser's "Counter" page read `Count: 2`. In 07, `nova build
  --ssr` printed `built dist` and
  `built target\debug\fullstack-blog-server.exe`. They reported both
  done, in their words "เรียบร้อยครับ" ("all done"), with three
  screenshots; none shows the blog running this time. Step 6.2's step 5,
  publishing `shapes`, did not run again.

### The backlog

The backlog is unscheduled. Each item is built when a program or a user
needs it, and the list promises no date, no phase and no order.

- **Left out of Phase 4 by ADR 0038:** examples 08 to 10; code splitting
  with `lazy()`; npm resolution for `@js_import`, and user-level
  `@js_import` and `@js_export` (ADR 0041 too); source maps, for a build
  and under `nova dev` (ADR 0044 too); HMR that keeps state across a
  change (ADR 0044 too); bindings generated from `web-sys` descriptors;
  the WASM GC proposal; `wasm-bindgen-test`, and visual regression
  snapshots; the LSP rows `40-TOOLING.md` §3.1 marks Phase 4 (inlay hints,
  code lens and call hierarchy); SQLite, and a `std/db`.
- **`std/ui` and its views** (ADRs 0042, 0043, 0045, 0046): a keyed
  `for_each` (0042, 0045); `untrack`, context, resources and error
  boundaries (0042); `if` and `for` in a view, fragments, and boolean
  properties such as `checked` (0043); checking attributes during
  hydration, partial hydration or islands, and streaming HTML with
  asynchronous data in `render` (0045); refusing `javascript:` URLs,
  preventing a submit, and `fetch` in shared components (0046).
- **The language** (ADRs 0043, 0046): named arguments outside `view`
  (0043); handlers that await, and async file access from a handler
  (0046); nested module directories (0046, after ADR 0003).
- **The router** (ADR 0044): nested routers and layouts; guards, lazy
  routes and a base path for history mode; scroll restoration; an accessor
  for the current location.
- **`nova dev`** (ADRs 0044, 0045, 0046): a development mode for SSR, which
  serves a server-rendered program (0044, 0045, 0046); an overlay for a
  runtime panic; HTTPS, and access from other machines (`--host`);
  incremental builds, in place of a full build in a child process (0044).
- **SSR and SSG** (ADR 0045): pre-rendering routes with parameters, and a
  list of paths a program gives; a `nova serve` command; `nova check` of
  `src/server.nova` as a second root.
- **`std/http`, `std/fs` and serving** (ADR 0046; the 4.8 spec's §11):
  chunked bodies, `HEAD`, compression and environment variables (0046);
  caching headers in `Dist` (4.8); `Secure` cookies by default, HTTPS, and
  a limit on login attempts (0046); a sync before a rename, and file
  modes, in `std/fs`, which the blog's README works around (0046).
- **The blog** (ADR 0046): more than one admin through its pages;
  Markdown; images; search; pages of posts.
- **Decided here:** `std/dom` on std's documentation site; the bundle in
  `crates/nova-bundler`; a measure of the counter's time to interactive,
  which `30-FRONTEND.md` §11 sets under 100 ms.

Taken from earlier backlogs:
- **ADR 0037's "Phase 4":** `nova build --target wasm`, `nova bundle` and
  `nova dev` are built; the LSP rows are left out by ADR 0038, and are on
  this backlog.
- **ADR 0025's "a router beyond exact paths and `GET`":** built in part,
  by `Server::fallback` (ADR 0045) and routes by method with `:name`
  patterns and `405` (ADR 0046). `use_middleware`, the `Handler` and
  `Middleware` aliases, and the HTTP client stay on ADR 0025's backlog.

### Found outside this boundary

Met while closing Phase 4. The first is fixed here; the others are
recorded, not decided.
- **`nova build --release` failed with a real `wasm-opt`.** 4.4 passed it
  `-Oz` alone, but the backend writes `memory.copy` and `memory.fill`,
  which binaryen refuses unless bulk memory is enabled ("memory.fill
  operations require bulk memory"): with binaryen's `wasm-opt` on `PATH`,
  every release build for WASM failed. 4.4's test ran a stand-in
  `wasm-opt` (its spec's §9.3), since neither its host nor CI had one. 4.8
  passes `--enable-bulk-memory` too (`WASM_OPT_FLAGS` in
  `crates/nova-cli/src/bundle.rs`), the stand-in's test expects it,
  `nova-codegen-wasm`'s tests validate every module with the MVP and bulk
  memory alone, so a module that needs another feature fails there, and
  CI builds and drives both examples with binaryen 132.
- **The phase plan's §5 asks for dated notes in `30-FRONTEND.md`,
  `14-CODEGEN.md` and `13-RUNTIME.md` as each sub-phase lands.**
  `30-FRONTEND.md` has them for 4.0 to 4.8, but `14-CODEGEN.md`'s last is
  4.2's and `13-RUNTIME.md`'s 4.3's: 4.5's router state, a second
  collector root natively and `std/rt`'s state word 112 on WASM, is
  recorded in ADR 0044 and `30-FRONTEND.md` §6's note, not in
  `13-RUNTIME.md`.
- **The 4.8 spec says `v0.3.2` published 20 crates** (its §2, and its
  §12, decision 7). It published 19 (ADR 0037's note of 2026-10-14);
  `nova-codegen-wasm`, the 20th in `publish-crates.sh` since 4.0, was not
  on crates.io on 2026-10-22, and first reaches it with `v0.4.0`, as the
  runbook's note of 2026-10-14 in its step 3 says.

## Consequences

- The master spec's §7 Definition of Done, item by item:
  1. the crates compile and pass CI: the 4.8 pull request's run, recorded
     in "The gate's record";
  2. the gate is met with reproducible commands: the `browser` job's
     steps and the size tests, measured where 4.8 was built and in the
     4.8 pull request's CI run, with the user's own check, all recorded in
     "The gate's record";
  3. the new public surface is documented: the "Web" sections of the root
     README and `nova-cli`'s, std's documentation site for `std/ui` and its
     modules, `std/dom`'s own source, the examples' READMEs, and the dated
     notes of `40-TOOLING.md` and `30-FRONTEND.md`;
  4. `CHANGELOG.md`'s `[0.4.0]` holds Phase 4, made from its Unreleased
     section by the release pull request, with the publishes' runs;
  5. ADRs 0038 to 0046 and this one record the deviations;
  6. the tag `v0.4.0`, on `eab7da8`, is the release's, and its publishes
     are recorded in "The gate's record".
- What a `v0.4.0` user can rely on is the "shipped" rows, within the
  limits their records state.
- The backlog promises nothing.
- Dated notes point here from:
  - the master spec's Phase 4 gate;
  - `nova-spec/40-TOOLING.md` §1.1 and `nova-spec/30-FRONTEND.md` §7, on
    `--release` and `wasm-opt`;
  - `nova-spec/60-EXAMPLES.md` §6 and §7, with each example's Lighthouse
    results;
  - `docs/phase-4-plan.md`'s 4.8 entry;
  - `docs/releasing.md`'s step 6.2, which `v0.4.0` replaces.
- The guides are corrected in place instead, because they describe the
  code as it is now: `agent.md`'s rows for `nova-codegen-wasm` and
  `nova-bundler`, `skill.md`'s WASM and frontend rows, and the examples'
  READMEs. Of the runbook's lines that were wrong for `v0.4.0`, the version
  `main` has and the 21 lines the bump changes in `Cargo.toml` are
  corrected in place, and the upload's time and step 6.2 get dated notes.
- `CHANGELOG.md`'s Unreleased section gains the Lighthouse step, `gate.sh`'s
  WASM build, the "Web" documentation, and the fix to `--release`.
- The release pull request added to this ADR the 4.8 pull request's CI
  run and the user's check; the records pull request added the publishes'
  runs and the published `nova`'s check.

**Tagged as `v0.4.0`, Phase 4 is complete within this boundary.**

## References

- Spec: `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
- `nova-spec/00-MASTER-SPEC.md` §3 and §7; `docs/phase-4-plan.md`;
  `nova-spec/60-EXAMPLES.md` §6 and §7; `nova-spec/30-FRONTEND.md`
- `docs/releasing.md`
- ADRs 0025, 0037, and 0038 to 0046
- Pull requests #117 to #124, and the release's, #126
- The workflow runs: Release 40063515984; Publish to crates.io
  40074121811; Release gate 40107204392
- `tests/browser/lighthouse.mjs`, `lighthouse-rules.mjs`,
  `lighthouse-run.mjs` and `lighthouse.test.mjs`,
  `.github/workflows/ci.yml`'s `browser` job, `.github/scripts/gate.sh`,
  and `crates/nova-cli/tests/wasm_tests.rs`
